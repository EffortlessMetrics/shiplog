//! Compression utilities for shiplog data.

use flate2::Compression;
use flate2::write::{GzDecoder, GzEncoder};
use serde::{Deserialize, Serialize};
use std::io::Write;

/// Compression algorithm
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum CompressionAlgorithm {
    #[default]
    Gzip,
    Snappy,
    None,
}

/// Compression configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    /// Compression algorithm to use
    #[serde(default)]
    pub algorithm: CompressionAlgorithm,
    /// Compression level (0-9 for gzip)
    #[serde(default = "default_compression_level")]
    pub level: u32,
}

fn default_compression_level() -> u32 {
    6
}

/// Compressor for encoding data
pub struct Compressor {
    config: CompressionConfig,
}

impl Compressor {
    /// Create a new compressor
    pub fn new(config: CompressionConfig) -> Self {
        Self { config }
    }

    /// Compress data using gzip
    pub fn compress_gzip(&self, data: &[u8]) -> anyhow::Result<Vec<u8>> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::new(self.config.level));
        encoder.write_all(data)?;
        encoder
            .finish()
            .map_err(|e| anyhow::anyhow!("Compression failed: {}", e))
    }

    /// Decompress gzip data
    pub fn decompress_gzip(&self, data: &[u8]) -> anyhow::Result<Vec<u8>> {
        let mut decoder = GzDecoder::new(Vec::new());
        decoder.write_all(data)?;
        decoder
            .finish()
            .map_err(|e| anyhow::anyhow!("Decompression failed: {}", e))
    }

    /// Compress data using configured algorithm
    pub fn compress(&self, data: &[u8]) -> anyhow::Result<Vec<u8>> {
        match self.config.algorithm {
            CompressionAlgorithm::Gzip => self.compress_gzip(data),
            CompressionAlgorithm::Snappy => self.compress_snappy(data),
            CompressionAlgorithm::None => Ok(data.to_vec()),
        }
    }

    /// Decompress data using configured algorithm
    pub fn decompress(&self, data: &[u8]) -> anyhow::Result<Vec<u8>> {
        match self.config.algorithm {
            CompressionAlgorithm::Gzip => self.decompress_gzip(data),
            CompressionAlgorithm::Snappy => self.decompress_snappy(data),
            CompressionAlgorithm::None => Ok(data.to_vec()),
        }
    }

    /// Compress using Snappy
    fn compress_snappy(&self, data: &[u8]) -> anyhow::Result<Vec<u8>> {
        let compressed = snap::raw::Encoder::new()
            .compress_vec(data)
            .map_err(|e| anyhow::anyhow!("Snappy compression failed: {}", e))?;
        Ok(compressed)
    }

    /// Decompress Snappy data
    fn decompress_snappy(&self, data: &[u8]) -> anyhow::Result<Vec<u8>> {
        let decompressed = snap::raw::Decoder::new()
            .decompress_vec(data)
            .map_err(|e| anyhow::anyhow!("Snappy decompression failed: {}", e))?;
        Ok(decompressed)
    }
}

/// Compression statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompressionStats {
    pub original_size: usize,
    pub compressed_size: usize,
}

impl CompressionStats {
    /// Calculate compression ratio
    pub fn ratio(&self) -> f64 {
        if self.original_size == 0 {
            return 1.0;
        }
        self.compressed_size as f64 / self.original_size as f64
    }

    /// Calculate space savings percentage
    pub fn savings_percent(&self) -> f64 {
        (1.0 - self.ratio()) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compression_config_default() {
        let config = CompressionConfig {
            algorithm: CompressionAlgorithm::Gzip,
            level: 6,
        };
        assert_eq!(config.algorithm, CompressionAlgorithm::Gzip);
        assert_eq!(config.level, 6);
    }

    #[test]
    fn gzip_compression() -> anyhow::Result<()> {
        let config = CompressionConfig {
            algorithm: CompressionAlgorithm::Gzip,
            level: 6,
        };

        let compressor = Compressor::new(config);
        // Use larger data to ensure compression is effective
        let original =
            b"Hello, this is test data for compression! It contains multiple bytes. ".repeat(10);

        let compressed = compressor.compress(&original)?;
        let decompressed = compressor.decompress(&compressed)?;

        assert_eq!(original.as_slice(), decompressed.as_slice());

        // Verify compression actually reduced size (for larger data)
        assert!(compressed.len() < original.len());

        Ok(())
    }

    #[test]
    fn snappy_compression() -> anyhow::Result<()> {
        let config = CompressionConfig {
            algorithm: CompressionAlgorithm::Snappy,
            level: 6,
        };

        let compressor = Compressor::new(config);
        let original = b"Hello, this is test data for Snappy compression!";

        let compressed = compressor.compress(original)?;
        let decompressed = compressor.decompress(&compressed)?;

        assert_eq!(original.as_slice(), decompressed.as_slice());

        Ok(())
    }

    #[test]
    fn no_compression() -> anyhow::Result<()> {
        let config = CompressionConfig {
            algorithm: CompressionAlgorithm::None,
            level: 6,
        };

        let compressor = Compressor::new(config);
        let original = b"Test data";

        let compressed = compressor.compress(original)?;
        let decompressed = compressor.decompress(&compressed)?;

        assert_eq!(original.as_slice(), compressed);
        assert_eq!(original.as_slice(), decompressed);

        Ok(())
    }

    #[test]
    fn compression_stats() {
        let stats = CompressionStats {
            original_size: 100,
            compressed_size: 50,
        };

        assert_eq!(stats.ratio(), 0.5);
        assert_eq!(stats.savings_percent(), 50.0);
    }
}
