use shiplog_compressor::{CompressionAlgorithm, CompressionConfig, CompressionStats, Compressor};

// ── CompressionAlgorithm ──────────────────────────────────────────

#[test]
fn algorithm_default_is_gzip() {
    assert_eq!(CompressionAlgorithm::default(), CompressionAlgorithm::Gzip);
}

#[test]
fn algorithm_eq() {
    assert_eq!(CompressionAlgorithm::Gzip, CompressionAlgorithm::Gzip);
    assert_eq!(CompressionAlgorithm::Snappy, CompressionAlgorithm::Snappy);
    assert_eq!(CompressionAlgorithm::None, CompressionAlgorithm::None);
    assert_ne!(CompressionAlgorithm::Gzip, CompressionAlgorithm::Snappy);
}

#[test]
fn algorithm_serde_roundtrip() {
    for algo in [
        CompressionAlgorithm::Gzip,
        CompressionAlgorithm::Snappy,
        CompressionAlgorithm::None,
    ] {
        let json = serde_json::to_string(&algo).unwrap();
        let de: CompressionAlgorithm = serde_json::from_str(&json).unwrap();
        assert_eq!(de, algo);
    }
}

// ── CompressionConfig ─────────────────────────────────────────────

#[test]
fn config_serde_roundtrip() {
    let cfg = CompressionConfig {
        algorithm: CompressionAlgorithm::Snappy,
        level: 3,
    };
    let json = serde_json::to_string(&cfg).unwrap();
    let de: CompressionConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(de.algorithm, CompressionAlgorithm::Snappy);
    assert_eq!(de.level, 3);
}

#[test]
fn config_serde_defaults() {
    let json = r#"{}"#;
    let cfg: CompressionConfig = serde_json::from_str(json).unwrap();
    assert_eq!(cfg.algorithm, CompressionAlgorithm::Gzip);
    assert_eq!(cfg.level, 6);
}

// ── Gzip roundtrip ────────────────────────────────────────────────

#[test]
fn gzip_roundtrip_basic() {
    let c = make_compressor(CompressionAlgorithm::Gzip, 6);
    let data = b"Hello, gzip world!";
    let compressed = c.compress_gzip(data).unwrap();
    let decompressed = c.decompress_gzip(&compressed).unwrap();
    assert_eq!(decompressed, data);
}

#[test]
fn gzip_roundtrip_empty() {
    let c = make_compressor(CompressionAlgorithm::Gzip, 6);
    let compressed = c.compress_gzip(b"").unwrap();
    let decompressed = c.decompress_gzip(&compressed).unwrap();
    assert!(decompressed.is_empty());
}

#[test]
fn gzip_roundtrip_large() {
    let c = make_compressor(CompressionAlgorithm::Gzip, 6);
    let data = vec![0xABu8; 100_000];
    let compressed = c.compress_gzip(&data).unwrap();
    let decompressed = c.decompress_gzip(&compressed).unwrap();
    assert_eq!(decompressed, data);
    assert!(compressed.len() < data.len());
}

#[test]
fn gzip_roundtrip_binary() {
    let c = make_compressor(CompressionAlgorithm::Gzip, 6);
    let data: Vec<u8> = (0..=255).collect();
    let compressed = c.compress_gzip(&data).unwrap();
    let decompressed = c.decompress_gzip(&compressed).unwrap();
    assert_eq!(decompressed, data);
}

#[test]
fn gzip_compression_levels() {
    let data = b"test data for compression level testing ".repeat(100);
    for level in [0, 1, 6, 9] {
        let c = make_compressor(CompressionAlgorithm::Gzip, level);
        let compressed = c.compress_gzip(&data).unwrap();
        let decompressed = c.decompress_gzip(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }
}

// ── Snappy roundtrip ──────────────────────────────────────────────

#[test]
fn snappy_roundtrip_basic() {
    let c = make_compressor(CompressionAlgorithm::Snappy, 0);
    let data = b"Hello, snappy world!";
    let compressed = c.compress(data).unwrap();
    let decompressed = c.decompress(&compressed).unwrap();
    assert_eq!(decompressed, data);
}

#[test]
fn snappy_roundtrip_empty() {
    let c = make_compressor(CompressionAlgorithm::Snappy, 0);
    let compressed = c.compress(b"").unwrap();
    let decompressed = c.decompress(&compressed).unwrap();
    assert!(decompressed.is_empty());
}

#[test]
fn snappy_roundtrip_large() {
    let c = make_compressor(CompressionAlgorithm::Snappy, 0);
    let data = vec![0xCDu8; 100_000];
    let compressed = c.compress(&data).unwrap();
    let decompressed = c.decompress(&compressed).unwrap();
    assert_eq!(decompressed, data);
}

// ── None algorithm ────────────────────────────────────────────────

#[test]
fn none_is_identity() {
    let c = make_compressor(CompressionAlgorithm::None, 0);
    let data = b"passthrough";
    let compressed = c.compress(data).unwrap();
    assert_eq!(compressed, data);
    let decompressed = c.decompress(&compressed).unwrap();
    assert_eq!(decompressed, data);
}

#[test]
fn none_empty_data() {
    let c = make_compressor(CompressionAlgorithm::None, 0);
    let compressed = c.compress(b"").unwrap();
    assert!(compressed.is_empty());
}

// ── compress/decompress dispatch ──────────────────────────────────

#[test]
fn dispatch_gzip() {
    let c = make_compressor(CompressionAlgorithm::Gzip, 6);
    let data = b"dispatch gzip test data";
    let compressed = c.compress(data).unwrap();
    let decompressed = c.decompress(&compressed).unwrap();
    assert_eq!(decompressed, data);
}

#[test]
fn dispatch_snappy() {
    let c = make_compressor(CompressionAlgorithm::Snappy, 0);
    let data = b"dispatch snappy test data";
    let compressed = c.compress(data).unwrap();
    let decompressed = c.decompress(&compressed).unwrap();
    assert_eq!(decompressed, data);
}

#[test]
fn dispatch_none() {
    let c = make_compressor(CompressionAlgorithm::None, 0);
    let data = b"dispatch none";
    let out = c.compress(data).unwrap();
    assert_eq!(out, data);
}

// ── Error handling ────────────────────────────────────────────────

#[test]
fn decompress_gzip_invalid_data() {
    let c = make_compressor(CompressionAlgorithm::Gzip, 6);
    let result = c.decompress_gzip(b"not valid gzip");
    assert!(result.is_err());
}

#[test]
fn decompress_snappy_invalid_data() {
    let c = make_compressor(CompressionAlgorithm::Snappy, 0);
    let result = c.decompress(b"\xff\xff\xff\xff\xff");
    assert!(result.is_err());
}

// ── CompressionStats ──────────────────────────────────────────────

#[test]
fn stats_default() {
    let s = CompressionStats::default();
    assert_eq!(s.original_size, 0);
    assert_eq!(s.compressed_size, 0);
}

#[test]
fn stats_ratio_half() {
    let s = CompressionStats {
        original_size: 100,
        compressed_size: 50,
    };
    assert!((s.ratio() - 0.5).abs() < f64::EPSILON);
}

#[test]
fn stats_ratio_zero_original() {
    let s = CompressionStats {
        original_size: 0,
        compressed_size: 0,
    };
    assert!((s.ratio() - 1.0).abs() < f64::EPSILON);
}

#[test]
fn stats_savings_percent() {
    let s = CompressionStats {
        original_size: 100,
        compressed_size: 25,
    };
    assert!((s.savings_percent() - 75.0).abs() < f64::EPSILON);
}

#[test]
fn stats_savings_no_savings() {
    let s = CompressionStats {
        original_size: 100,
        compressed_size: 100,
    };
    assert!((s.savings_percent() - 0.0).abs() < f64::EPSILON);
}

#[test]
fn stats_serde_roundtrip() {
    let s = CompressionStats {
        original_size: 500,
        compressed_size: 200,
    };
    let json = serde_json::to_string(&s).unwrap();
    let de: CompressionStats = serde_json::from_str(&json).unwrap();
    assert_eq!(de.original_size, 500);
    assert_eq!(de.compressed_size, 200);
}

// ── Property tests ────────────────────────────────────────────────

mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn gzip_roundtrip_any_data(data in prop::collection::vec(any::<u8>(), 0..2000)) {
            let c = make_compressor(CompressionAlgorithm::Gzip, 6);
            let compressed = c.compress(&data).unwrap();
            let decompressed = c.decompress(&compressed).unwrap();
            prop_assert_eq!(decompressed, data);
        }

        #[test]
        fn snappy_roundtrip_any_data(data in prop::collection::vec(any::<u8>(), 0..2000)) {
            let c = make_compressor(CompressionAlgorithm::Snappy, 0);
            let compressed = c.compress(&data).unwrap();
            let decompressed = c.decompress(&compressed).unwrap();
            prop_assert_eq!(decompressed, data);
        }

        #[test]
        fn none_roundtrip_any_data(data in prop::collection::vec(any::<u8>(), 0..2000)) {
            let c = make_compressor(CompressionAlgorithm::None, 0);
            let compressed = c.compress(&data).unwrap();
            prop_assert_eq!(compressed.clone(), data.clone());
            let decompressed = c.decompress(&compressed).unwrap();
            prop_assert_eq!(decompressed, data);
        }

        #[test]
        fn stats_ratio_in_range(orig in 1usize..100_000, comp in 0usize..100_000) {
            let s = CompressionStats { original_size: orig, compressed_size: comp };
            prop_assert!(s.ratio() >= 0.0);
        }

        #[test]
        fn stats_savings_in_range(orig in 1usize..100_000, comp in 0usize..100_000) {
            let s = CompressionStats { original_size: orig, compressed_size: comp };
            let savings = s.savings_percent();
            prop_assert!(savings <= 100.0);
        }

        #[test]
        fn gzip_level_roundtrip(data in prop::collection::vec(any::<u8>(), 0..500), level in 0u32..=9) {
            let c = make_compressor(CompressionAlgorithm::Gzip, level);
            let compressed = c.compress(&data).unwrap();
            let decompressed = c.decompress(&compressed).unwrap();
            prop_assert_eq!(decompressed, data);
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────

fn make_compressor(algorithm: CompressionAlgorithm, level: u32) -> Compressor {
    Compressor::new(CompressionConfig { algorithm, level })
}
