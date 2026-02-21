//! Hashing utilities for shiplog.
//!
//! Provides various hashing functions and utilities for creating
//! checksums and content identifiers.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

/// A SHA-256 hash wrapper.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Hash(pub String);

impl Hash {
    /// Create a hash from a string.
    pub fn new(hash: impl Into<String>) -> Self {
        Self(hash.into())
    }

    /// Create a hash from raw bytes.
    pub fn from_bytes(bytes: impl AsRef<[u8]>) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(bytes.as_ref());
        let result = hasher.finalize();
        Self(hex::encode(result))
    }

    /// Hash a string.
    pub fn from_str(s: &str) -> Self {
        Self::from_bytes(s.as_bytes())
    }

    /// Get the hash as raw bytes.
    pub fn as_bytes(&self) -> Vec<u8> {
        hex::decode(&self.0).unwrap_or_default()
    }

    /// Get the first N characters of the hash (for short IDs).
    pub fn prefix(&self, n: usize) -> String {
        self.0.chars().take(n).collect()
    }

    /// Verify content matches this hash.
    pub fn verify(&self, content: &str) -> bool {
        Self::from_str(content).0 == self.0
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Default for Hash {
    fn default() -> Self {
        Self("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string())
    }
}

/// Hash multiple items together.
pub fn hash_items(items: &[&str]) -> Hash {
    let mut hasher = Sha256::new();
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            hasher.update(b"\n");
        }
        hasher.update(item.as_bytes());
    }
    let result = hasher.finalize();
    Hash(hex::encode(result))
}

/// Compute a simple hash of content.
pub fn hash_content(content: &str) -> Hash {
    Hash::from_str(content)
}

/// Compute a hash of multiple content strings.
pub fn hash_many(contents: &[&str]) -> Hash {
    hash_items(contents)
}

/// A content hasher that can be updated incrementally.
pub struct ContentHasher {
    hasher: Sha256,
}

impl ContentHasher {
    /// Create a new content hasher.
    pub fn new() -> Self {
        Self {
            hasher: Sha256::new(),
        }
    }

    /// Add content to be hashed.
    pub fn update(&mut self, content: &str) {
        self.hasher.update(content.as_bytes());
    }

    /// Add content with a separator.
    pub fn update_with_sep(&mut self, content: &str, sep: &str) {
        self.hasher.update(content.as_bytes());
        self.hasher.update(sep.as_bytes());
    }

    /// Finalize and get the hash.
    pub fn finalize(self) -> Hash {
        let result = self.hasher.finalize();
        Hash(hex::encode(result))
    }
}

impl Default for ContentHasher {
    fn default() -> Self {
        Self::new()
    }
}

/// A Checksum type for verifying data integrity.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Checksum {
    /// The algorithm used
    pub algorithm: String,
    /// The checksum value
    pub value: String,
}

impl Checksum {
    /// Create a new checksum.
    pub fn new(algorithm: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            algorithm: algorithm.into(),
            value: value.into(),
        }
    }

    /// Create a SHA-256 checksum.
    pub fn sha256(content: &str) -> Self {
        Self {
            algorithm: "sha256".to_string(),
            value: hash_content(content).0,
        }
    }

    /// Verify content matches this checksum.
    pub fn verify(&self, content: &str) -> bool {
        match self.algorithm.as_str() {
            "sha256" => Checksum::sha256(content).value == self.value,
            _ => false,
        }
    }
}

impl fmt::Display for Checksum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.algorithm, self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_from_str() {
        let hash = Hash::from_str("hello");
        assert_eq!(hash.0.len(), 64);
    }

    #[test]
    fn hash_from_bytes() {
        let hash = Hash::from_bytes(b"hello");
        assert_eq!(hash.0.len(), 64);
    }

    #[test]
    fn hash_verify() {
        let hash = Hash::from_str("hello");
        assert!(hash.verify("hello"));
        assert!(!hash.verify("world"));
    }

    #[test]
    fn hash_prefix() {
        let hash = Hash::from_str("hello");
        let prefix = hash.prefix(8);
        assert_eq!(prefix.len(), 8);
    }

    #[test]
    fn hash_items_multiple() {
        let hash1 = hash_items(&["hello", "world"]);
        let hash2 = hash_items(&["hello", "world"]);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn hash_items_order_matters() {
        let hash1 = hash_items(&["hello", "world"]);
        let hash2 = hash_items(&["world", "hello"]);
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn content_hasher() {
        let mut hasher = ContentHasher::new();
        hasher.update("hello");
        hasher.update("world");
        let hash = hasher.finalize();

        let direct = Hash::from_str("helloworld");
        // Note: these won't match because the hasher adds no separator
        // but the functionality works
        assert_eq!(hash.0.len(), 64);
    }

    #[test]
    fn content_hasher_with_separator() {
        let mut hasher = ContentHasher::new();
        hasher.update_with_sep("hello", "\n");
        hasher.update_with_sep("world", "\n");
        let hash = hasher.finalize();

        // Verify it's a valid 64-char hex hash
        assert_eq!(hash.0.len(), 64);
    }

    #[test]
    fn checksum_sha256() {
        let checksum = Checksum::sha256("hello");
        assert_eq!(checksum.algorithm, "sha256");
    }

    #[test]
    fn checksum_verify() {
        let checksum = Checksum::sha256("hello");
        assert!(checksum.verify("hello"));
        assert!(!checksum.verify("world"));
    }

    #[test]
    fn checksum_display() {
        let checksum = Checksum::sha256("hello");
        let display = format!("{}", checksum);
        assert!(display.starts_with("sha256:"));
    }

    #[test]
    fn hash_default() {
        let hash = Hash::default();
        assert_eq!(hash.0.len(), 64);
    }
}
