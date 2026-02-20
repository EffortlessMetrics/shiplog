//! Storage backend abstractions for shiplog.
//!
//! Provides traits and implementations for various storage backends
//! including file system, in-memory, and cloud storage.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Storage backend types supported by shiplog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageBackend {
    FileSystem,
    InMemory,
    Cloud,
}

impl fmt::Display for StorageBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageBackend::FileSystem => write!(f, "filesystem"),
            StorageBackend::InMemory => write!(f, "memory"),
            StorageBackend::Cloud => write!(f, "cloud"),
        }
    }
}

/// Error type for storage operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageError {
    pub message: String,
    pub backend: StorageBackend,
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "storage error [{}]: {}", self.backend, self.message)
    }
}

impl std::error::Error for StorageError {}

/// Result type for storage operations.
pub type StorageResult<T> = Result<T, StorageError>;

/// Storage key for accessing stored data.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StorageKey(pub String);

impl StorageKey {
    /// Create a new storage key from a path.
    pub fn from_path(path: &str) -> Self {
        Self(path.to_string())
    }
}

impl fmt::Display for StorageKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Trait for storage backends.
pub trait Storage: Send + Sync {
    /// Get a value from storage.
    fn get(&self, key: &StorageKey) -> StorageResult<Option<Vec<u8>>>;

    /// Set a value in storage.
    fn set(&mut self, key: &StorageKey, value: Vec<u8>) -> StorageResult<()>;

    /// Delete a value from storage.
    fn delete(&mut self, key: &StorageKey) -> StorageResult<()>;

    /// Check if a key exists.
    fn exists(&self, key: &StorageKey) -> StorageResult<bool>;

    /// List all keys with a given prefix.
    fn list(&self, prefix: &StorageKey) -> StorageResult<Vec<StorageKey>>;
}

/// In-memory storage implementation for testing.
pub struct InMemoryStorage {
    data: std::collections::HashMap<String, Vec<u8>>,
}

impl InMemoryStorage {
    /// Create a new in-memory storage.
    pub fn new() -> Self {
        Self {
            data: std::collections::HashMap::new(),
        }
    }
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl Storage for InMemoryStorage {
    fn get(&self, key: &StorageKey) -> StorageResult<Option<Vec<u8>>> {
        Ok(self.data.get(&key.0).cloned())
    }

    fn set(&mut self, key: &StorageKey, value: Vec<u8>) -> StorageResult<()> {
        self.data.insert(key.0.clone(), value);
        Ok(())
    }

    fn delete(&mut self, key: &StorageKey) -> StorageResult<()> {
        self.data.remove(&key.0);
        Ok(())
    }

    fn exists(&self, key: &StorageKey) -> StorageResult<bool> {
        Ok(self.data.contains_key(&key.0))
    }

    fn list(&self, prefix: &StorageKey) -> StorageResult<Vec<StorageKey>> {
        let keys: Vec<StorageKey> = self
            .data
            .keys()
            .filter(|k| k.starts_with(&prefix.0))
            .map(|k| StorageKey(k.clone()))
            .collect();
        Ok(keys)
    }
}

/// File system storage configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSystemConfig {
    pub root_path: String,
}

impl FileSystemConfig {
    /// Create a new file system config.
    pub fn new(root_path: impl Into<String>) -> Self {
        Self {
            root_path: root_path.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_key_display() {
        let key = StorageKey::from_path("events/2024/01/15");
        assert_eq!(format!("{}", key), "events/2024/01/15");
    }

    #[test]
    fn storage_backend_display() {
        assert_eq!(format!("{}", StorageBackend::FileSystem), "filesystem");
        assert_eq!(format!("{}", StorageBackend::InMemory), "memory");
    }

    #[test]
    fn in_memory_storage_basic() {
        let mut storage = InMemoryStorage::new();
        let key = StorageKey::from_path("test/key");

        // Should not exist initially
        assert!(!storage.exists(&key).unwrap());

        // Set a value
        storage.set(&key, b"test value".to_vec()).unwrap();

        // Should exist now
        assert!(storage.exists(&key).unwrap());

        // Get the value
        let value = storage.get(&key).unwrap();
        assert_eq!(value, Some(b"test value".to_vec()));
    }

    #[test]
    fn in_memory_storage_delete() {
        let mut storage = InMemoryStorage::new();
        let key = StorageKey::from_path("test/key");

        storage.set(&key, b"test".to_vec()).unwrap();
        assert!(storage.exists(&key).unwrap());

        storage.delete(&key).unwrap();
        assert!(!storage.exists(&key).unwrap());
    }

    #[test]
    fn in_memory_storage_list() {
        let mut storage = InMemoryStorage::new();

        storage.set(&StorageKey::from_path("events/1"), b"a".to_vec()).unwrap();
        storage.set(&StorageKey::from_path("events/2"), b"b".to_vec()).unwrap();
        storage.set(&StorageKey::from_path("other/1"), b"c".to_vec()).unwrap();

        let prefix = StorageKey::from_path("events/");
        let keys = storage.list(&prefix).unwrap();
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn in_memory_storage_get_nonexistent() {
        let storage = InMemoryStorage::new();
        let key = StorageKey::from_path("nonexistent");

        let result = storage.get(&key).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn file_system_config() {
        let config = FileSystemConfig::new("/data/shiplog");
        assert_eq!(config.root_path, "/data/shiplog");
    }
}
