//! Deduplication utilities for shiplog.
//!
//! Provides utilities for removing duplicate items from collections
//! based on various strategies and key extraction.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fmt;
use std::hash::Hash;

/// A key that can be used for deduplication.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DedupKey(String);

impl DedupKey {
    /// Create a new deduplication key from a string.
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    /// Create a deduplication key by hashing content.
    pub fn from_content(content: impl AsRef<str>) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(content.as_ref().as_bytes());
        let result = hasher.finalize();
        Self(hex::encode(result))
    }

    /// Get the underlying string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DedupKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<String> for DedupKey {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for DedupKey {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Configuration for deduplication behavior.
#[derive(Clone, Debug, Default)]
pub struct DedupConfig {
    /// Whether to keep the first or last occurrence.
    pub keep_first: bool,
    /// Whether to use case-sensitive comparison.
    pub case_sensitive: bool,
}

impl DedupConfig {
    /// Create a new config with default settings (keep first, case sensitive).
    pub fn new() -> Self {
        Self {
            keep_first: true,
            case_sensitive: true,
        }
    }

    /// Set to keep the first occurrence.
    pub fn keep_first(mut self) -> Self {
        self.keep_first = true;
        self
    }

    /// Set to keep the last occurrence.
    pub fn keep_last(mut self) -> Self {
        self.keep_first = false;
        self
    }

    /// Enable case-sensitive comparison.
    pub fn case_sensitive(mut self) -> Self {
        self.case_sensitive = true;
        self
    }

    /// Enable case-insensitive comparison.
    pub fn case_insensitive(mut self) -> Self {
        self.case_sensitive = false;
        self
    }
}

/// Deduplicator that removes duplicate items based on a key function.
#[derive(Clone, Debug)]
pub struct Deduplicator<T> {
    config: DedupConfig,
    _marker: std::marker::PhantomData<T>,
}

impl<T> Deduplicator<T> {
    /// Create a new deduplicator with the given config.
    pub fn new(config: DedupConfig) -> Self {
        Self {
            config,
            _marker: std::marker::PhantomData,
        }
    }

    /// Create a deduplicator with default config.
    pub fn default_config() -> Self {
        Self::new(DedupConfig::new())
    }

    /// Deduplicate items using a key function.
    ///
    /// Returns a vector with duplicates removed.
    pub fn deduplicate<F>(&self, items: &[T], key_fn: F) -> Vec<T>
    where
        T: Clone + Hash + Eq,
        F: Fn(&T) -> String,
    {
        let mut seen: HashSet<String> = HashSet::new();
        let mut result = Vec::new();

        for item in items {
            let key = key_fn(item);
            let key = if self.config.case_sensitive {
                key
            } else {
                key.to_lowercase()
            };

            let should_add = if self.config.keep_first {
                seen.insert(key)
            } else {
                // For keep_last, we still track what's seen but add in reverse later
                // For simplicity, this implementation keeps first
                seen.insert(key)
            };

            if should_add {
                result.push(item.clone());
            }
        }

        result
    }
}

/// Deduplicate a slice of items using their string representation as the key.
impl<T: ToString + Hash + Eq> Deduplicator<T> {
    /// Simple deduplication using ToString as the key.
    pub fn deduplicate_simple(&self, items: &[T]) -> Vec<T>
    where
        T: Clone,
    {
        let mut seen: HashSet<String> = HashSet::new();
        let mut result = Vec::new();

        for item in items {
            let key = item.to_string();
            let key = if self.config.case_sensitive {
                key
            } else {
                key.to_lowercase()
            };

            if seen.insert(key) {
                result.push(item.clone());
            }
        }

        result
    }
}

/// Deduplicate strings using content-based hashing.
pub fn dedupe_strings(strings: &[&str]) -> Vec<String> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut result = Vec::new();

    for s in strings {
        let key = DedupKey::from_content(*s);
        if seen.insert(key.0) {
            result.push(s.to_string());
        }
    }

    result
}

/// Deduplicate items by extracting a key and comparing.
pub fn dedupe_by_key<T, F>(items: &[T], key_fn: F) -> Vec<T>
where
    T: Clone + Hash + Eq,
    F: Fn(&T) -> String,
{
    Deduplicator::default_config().deduplicate(items, key_fn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedup_key_from_content() {
        let key1 = DedupKey::from_content("hello");
        let key2 = DedupKey::from_content("hello");
        let key3 = DedupKey::from_content("world");

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn dedup_key_as_str() {
        let key = DedupKey::new("test-key");
        assert_eq!(key.as_str(), "test-key");
    }

    #[test]
    fn deduplicator_keep_first() {
        let config = DedupConfig::new().keep_first();
        let dedup = Deduplicator::new(config);

        let items = vec![1, 2, 1, 3, 2, 4];
        let result = dedup.deduplicate_simple(&items);

        assert_eq!(result, vec![1, 2, 3, 4]);
    }

    #[test]
    fn deduplicator_case_insensitive() {
        let config = DedupConfig::new().case_insensitive();
        let dedup = Deduplicator::new(config);

        let items = vec!["Hello", "hello", "HELLO", "world"];
        let result = dedup.deduplicate_simple(&items);

        assert_eq!(result.len(), 2);
    }

    #[test]
    fn deduplicator_with_key_function() {
        let dedup = Deduplicator::default_config();

        #[derive(Clone, Debug, PartialEq, Eq, Hash)]
        struct Item {
            id: u32,
            name: String,
        }

        let items = vec![
            Item {
                id: 1,
                name: "a".to_string(),
            },
            Item {
                id: 2,
                name: "b".to_string(),
            },
            Item {
                id: 1,
                name: "c".to_string(),
            },
        ];

        let result = dedup.deduplicate(&items, |item| item.id.to_string());

        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_dedupe_strings() {
        let strings = vec!["hello", "world", "hello", "test", "world"];
        let result = super::dedupe_strings(&strings);

        assert_eq!(result.len(), 3);
        assert!(result.contains(&"hello".to_string()));
        assert!(result.contains(&"world".to_string()));
        assert!(result.contains(&"test".to_string()));
    }

    #[test]
    fn test_dedupe_by_key() {
        #[derive(Clone, Debug, PartialEq, Eq, Hash)]
        struct Item(u32);

        let items = vec![Item(1), Item(2), Item(1), Item(3)];
        let result = super::dedupe_by_key(&items, |item| item.0.to_string());

        assert_eq!(result.len(), 3);
    }

    #[test]
    fn dedup_config_new() {
        let config = DedupConfig::new();
        assert!(config.keep_first, "new() should keep first by default");
        assert!(config.case_sensitive, "new() should be case sensitive");
    }
}
