//! Backup and restore functionality for shiplog data.
//!
//! Provides utilities for creating backups of shiplog data,
//! managing backup metadata, and restoring from backups.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Backup metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub description: Option<String>,
    pub size_bytes: u64,
    pub file_count: usize,
    pub checksum: String,
}

/// A backup archive containing shiplog data.
pub struct BackupArchive {
    pub metadata: BackupMetadata,
    path: PathBuf,
}

impl BackupArchive {
    /// Create a new backup archive.
    pub fn new(
        id: impl Into<String>,
        description: Option<String>,
        path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            metadata: BackupMetadata {
                id: id.into(),
                created_at: Utc::now(),
                description,
                size_bytes: 0,
                file_count: 0,
                checksum: String::new(),
            },
            path: path.into(),
        }
    }

    /// Get the backup path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Set the metadata after backup is created.
    pub fn set_metadata(&mut self, size_bytes: u64, file_count: usize, checksum: impl Into<String>) {
        self.metadata.size_bytes = size_bytes;
        self.metadata.file_count = file_count;
        self.metadata.checksum = checksum.into();
    }
}

/// Backup manager for creating and restoring backups.
pub struct BackupManager {
    backup_dir: PathBuf,
}

impl BackupManager {
    /// Create a new backup manager with the given backup directory.
    pub fn new(backup_dir: impl Into<PathBuf>) -> Self {
        Self {
            backup_dir: backup_dir.into(),
        }
    }

    /// Get the backup directory path.
    pub fn backup_dir(&self) -> &Path {
        &self.backup_dir
    }

    /// Create a backup of the given source directory.
    pub fn create_backup(
        &self,
        source_dir: &Path,
        backup_name: &str,
    ) -> anyhow::Result<BackupArchive> {
        // Ensure backup directory exists
        fs::create_dir_all(&self.backup_dir)?;

        let backup_id = format!(
            "{}_{}",
            backup_name,
            Utc::now().format("%Y%m%d_%H%M%S")
        );
        let backup_path = self.backup_dir.join(format!("{}.zip", backup_id));

        let mut archive = BackupArchive::new(backup_id, None, &backup_path);

        // Create zip archive
        let file = File::create(&backup_path)?;
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        let mut file_count = 0usize;
        let mut total_size = 0u64;

        // Walk the source directory
        for entry in walkdir::WalkDir::new(source_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() {
                let relative_path = path
                    .strip_prefix(source_dir)
                    .map_err(|e| anyhow::anyhow!("Failed to strip prefix: {}", e))?;

                zip.start_file(relative_path.to_string_lossy(), options)?;

                let mut f = File::open(path)?;
                let mut buffer = Vec::new();
                f.read_to_end(&mut buffer)?;

                zip.write_all(&buffer)?;
                total_size += buffer.len() as u64;
                file_count += 1;
            }
        }

        zip.finish()?;

        // Calculate checksum
        let checksum = calculate_checksum(&backup_path)?;

        archive.set_metadata(total_size, file_count, checksum);

        // Save metadata
        let metadata_path = self
            .backup_dir
            .join(format!("{}.meta.json", archive.metadata.id));
        let metadata_json = serde_json::to_string_pretty(&archive.metadata)?;
        fs::write(metadata_path, metadata_json)?;

        Ok(archive)
    }

    /// Restore a backup to the target directory.
    pub fn restore_backup(&self, backup_id: &str, target_dir: &Path) -> anyhow::Result<()> {
        let backup_path = self.backup_dir.join(format!("{}.zip", backup_id));

        if !backup_path.exists() {
            anyhow::bail!("Backup not found: {}", backup_id);
        }

        // Ensure target directory exists
        fs::create_dir_all(target_dir)?;

        // Extract zip archive
        let file = File::open(&backup_path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = target_dir.join(file.name());

            if file.name().ends_with('/') {
                fs::create_dir_all(&outpath)?;
            } else {
                if let Some(parent) = outpath.parent() {
                    fs::create_dir_all(parent)?;
                }
                let mut outfile = File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }
        }

        Ok(())
    }

    /// List all backups.
    pub fn list_backups(&self) -> anyhow::Result<Vec<BackupMetadata>> {
        let mut backups = Vec::new();

        if !self.backup_dir.exists() {
            return Ok(backups);
        }

        for entry in fs::read_dir(&self.backup_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map_or(false, |e| e == "json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(metadata) = serde_json::from_str::<BackupMetadata>(&content) {
                        backups.push(metadata);
                    }
                }
            }
        }

        // Sort by creation date, newest first
        backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(backups)
    }

    /// Delete a backup.
    pub fn delete_backup(&self, backup_id: &str) -> anyhow::Result<()> {
        let backup_path = self.backup_dir.join(format!("{}.zip", backup_id));
        let metadata_path = self.backup_dir.join(format!("{}.meta.json", backup_id));

        if backup_path.exists() {
            fs::remove_file(backup_path)?;
        }

        if metadata_path.exists() {
            fs::remove_file(metadata_path)?;
        }

        Ok(())
    }

    /// Get a specific backup metadata.
    pub fn get_backup(&self, backup_id: &str) -> anyhow::Result<BackupMetadata> {
        let metadata_path = self
            .backup_dir
            .join(format!("{}.meta.json", backup_id));

        if !metadata_path.exists() {
            anyhow::bail!("Backup not found: {}", backup_id);
        }

        let content = fs::read_to_string(metadata_path)?;
        let metadata = serde_json::from_str(&content)?;

        Ok(metadata)
    }
}

/// Calculate SHA-256 checksum of a file.
fn calculate_checksum(path: &Path) -> anyhow::Result<String> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(&buffer);
    let result = hasher.finalize();

    Ok(hex::encode(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_backup_metadata() {
        let metadata = BackupMetadata {
            id: "test-backup-001".to_string(),
            created_at: Utc::now(),
            description: Some("Test backup".to_string()),
            size_bytes: 1024,
            file_count: 5,
            checksum: "abc123".to_string(),
        };

        assert_eq!(metadata.id, "test-backup-001");
        assert!(metadata.description.is_some());
        assert_eq!(metadata.size_bytes, 1024);
    }

    #[test]
    fn test_backup_archive_creation() {
        let archive = BackupArchive::new(
            "backup-001",
            Some("My backup".to_string()),
            "/tmp/backup.zip",
        );

        assert_eq!(archive.metadata.id, "backup-001");
        assert_eq!(archive.metadata.description, Some("My backup".to_string()));
        assert_eq!(archive.path(), Path::new("/tmp/backup.zip"));
    }

    #[test]
    fn test_backup_manager_new() {
        let manager = BackupManager::new("/backups");
        assert_eq!(manager.backup_dir(), Path::new("/backups"));
    }

    #[test]
    fn test_backup_manager_with_temp_dir() {
        let temp_dir = TempDir::new().unwrap();
        let manager = BackupManager::new(temp_dir.path());
        
        // List backups on empty directory should work
        let backups = manager.list_backups().unwrap();
        assert!(backups.is_empty());
    }

    #[test]
    fn test_backup_create_and_restore() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");

        // Create source files
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(source_dir.join("test.txt"), "Hello, World!").unwrap();
        fs::write(source_dir.join("data.json"), r#"{"key": "value"}"#).unwrap();

        // Create backup
        let manager = BackupManager::new(&backup_dir);
        let backup = manager.create_backup(&source_dir, "test").unwrap();

        // Verify backup was created
        assert!(backup.path().exists());
        assert!(backup.metadata.file_count >= 2);

        // Restore backup
        manager.restore_backup(&backup.metadata.id, &target_dir).unwrap();

        // Verify restored files
        assert!(target_dir.join("test.txt").exists());
        assert!(target_dir.join("data.json").exists());

        let content = fs::read_to_string(target_dir.join("test.txt")).unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[test]
    fn test_delete_backup() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");
        let source_dir = temp_dir.path().join("source");

        // Create source file
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(source_dir.join("test.txt"), "test").unwrap();

        // Create and delete backup
        let manager = BackupManager::new(&backup_dir);
        let backup = manager.create_backup(&source_dir, "test").unwrap();
        let backup_id = backup.metadata.id.clone();

        // Verify backup exists
        assert!(backup_dir.join(format!("{}.zip", backup_id)).exists());

        // Delete backup
        manager.delete_backup(&backup_id).unwrap();

        // Verify backup is deleted
        assert!(!backup_dir.join(format!("{}.zip", backup_id)).exists());
        assert!(!backup_dir.join(format!("{}.meta.json", backup_id)).exists());
    }
}
