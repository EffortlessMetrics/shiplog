//! Archive/compress old packets.
//!
//! This crate provides functionality for archiving and compressing old shipping packets.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Archive format
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ArchiveFormat {
    Zip,
    Gzip,
}

/// Archive configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveConfig {
    pub format: ArchiveFormat,
    pub retention_days: u32,
    pub compression_level: Option<u32>,
}

impl Default for ArchiveConfig {
    fn default() -> Self {
        Self {
            format: ArchiveFormat::Zip,
            retention_days: 90,
            compression_level: Some(6),
        }
    }
}

/// Archive entry metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveEntry {
    pub original_path: PathBuf,
    pub archived_at: DateTime<Utc>,
    pub size_original: u64,
    pub size_compressed: u64,
    pub checksum: String,
}

/// Archive manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveManifest {
    pub created_at: DateTime<Utc>,
    pub entries: Vec<ArchiveEntry>,
    pub total_original_size: u64,
    pub total_compressed_size: u64,
}

impl ArchiveManifest {
    pub fn new() -> Self {
        Self {
            created_at: Utc::now(),
            entries: Vec::new(),
            total_original_size: 0,
            total_compressed_size: 0,
        }
    }

    pub fn add_entry(&mut self, entry: ArchiveEntry) {
        self.total_original_size += entry.size_original;
        self.total_compressed_size += entry.size_compressed;
        self.entries.push(entry);
    }

    /// Calculate compression ratio
    pub fn compression_ratio(&self) -> f64 {
        if self.total_original_size > 0 {
            (self.total_compressed_size as f64 / self.total_original_size as f64) * 100.0
        } else {
            0.0
        }
    }
}

impl Default for ArchiveManifest {
    fn default() -> Self {
        Self::new()
    }
}

/// Archive status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ArchiveStatus {
    Pending,
    InProgress,
    Completed,
    Failed(String),
}

/// Archive state
pub struct ArchiveState {
    pub config: ArchiveConfig,
    pub status: ArchiveStatus,
    pub manifest: ArchiveManifest,
}

impl ArchiveState {
    pub fn new(config: ArchiveConfig) -> Self {
        Self {
            config,
            status: ArchiveStatus::Pending,
            manifest: ArchiveManifest::new(),
        }
    }

    /// Start archiving
    pub fn start(&mut self) {
        self.status = ArchiveStatus::InProgress;
    }

    /// Complete archiving
    pub fn complete(&mut self, manifest: ArchiveManifest) {
        self.manifest = manifest;
        self.status = ArchiveStatus::Completed;
    }

    /// Fail archiving
    pub fn fail(&mut self, error: String) {
        self.status = ArchiveStatus::Failed(error);
    }

    /// Check if packet should be archived based on retention
    pub fn should_archive(&self, created_at: &DateTime<Utc>) -> bool {
        let age_days = Utc::now().signed_duration_since(*created_at).num_days();
        age_days >= self.config.retention_days as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_archive_config_default() {
        let config = ArchiveConfig::default();

        assert_eq!(config.format, ArchiveFormat::Zip);
        assert_eq!(config.retention_days, 90);
    }

    #[test]
    fn test_archive_manifest() {
        let mut manifest = ArchiveManifest::new();

        manifest.add_entry(ArchiveEntry {
            original_path: PathBuf::from("packet1.json"),
            archived_at: Utc::now(),
            size_original: 1000,
            size_compressed: 400,
            checksum: "abc123".to_string(),
        });

        assert_eq!(manifest.entries.len(), 1);
        assert_eq!(manifest.total_original_size, 1000);
        assert_eq!(manifest.total_compressed_size, 400);
    }

    #[test]
    fn test_compression_ratio() {
        let mut manifest = ArchiveManifest::new();

        manifest.add_entry(ArchiveEntry {
            original_path: PathBuf::from("packet1.json"),
            archived_at: Utc::now(),
            size_original: 1000,
            size_compressed: 500,
            checksum: "abc123".to_string(),
        });

        assert_eq!(manifest.compression_ratio(), 50.0);
    }

    #[test]
    fn test_archive_state_creation() {
        let config = ArchiveConfig::default();
        let state = ArchiveState::new(config);

        assert_eq!(state.status, ArchiveStatus::Pending);
    }

    #[test]
    fn test_should_archive() {
        let config = ArchiveConfig {
            format: ArchiveFormat::Zip,
            retention_days: 30,
            compression_level: Some(6),
        };
        let state = ArchiveState::new(config);

        // Created 60 days ago should be archived
        let old_date = Utc::now() - chrono::Duration::days(60);
        assert!(state.should_archive(&old_date));

        // Created 10 days ago should not be archived
        let recent_date = Utc::now() - chrono::Duration::days(10);
        assert!(!state.should_archive(&recent_date));
    }

    #[test]
    fn test_archive_state_transitions() {
        let config = ArchiveConfig::default();
        let mut state = ArchiveState::new(config);

        state.start();
        assert!(matches!(state.status, ArchiveStatus::InProgress));

        let manifest = ArchiveManifest::new();
        state.complete(manifest);
        assert!(matches!(state.status, ArchiveStatus::Completed));

        let config2 = ArchiveConfig::default();
        let mut state2 = ArchiveState::new(config2);
        state2.fail("Disk full".to_string());
        assert!(matches!(state2.status, ArchiveStatus::Failed(msg) if msg == "Disk full"));
    }
}
