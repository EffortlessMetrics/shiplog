//! TTL (time-to-live) cache utilities for shiplog.
//!
//! This crate provides utilities for managing time-to-live entries in caches,
//! including expiration checking and TTL-aware data structures.

use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::hash::Hash;

/// A value with an associated expiration time.
#[derive(Debug, Clone)]
pub struct TtlEntry<T> {
    pub value: T,
    pub expires_at: DateTime<Utc>,
}

impl<T> TtlEntry<T> {
    /// Create a new TTL entry with the given duration from now.
    pub fn new(value: T, ttl: Duration) -> Self {
        Self {
            value,
            expires_at: Utc::now() + ttl,
        }
    }

    /// Check if the entry has expired.
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Get the remaining time-to-live.
    pub fn remaining_ttl(&self) -> Option<Duration> {
        let remaining = self.expires_at - Utc::now();
        if remaining <= Duration::zero() {
            None
        } else {
            Some(remaining)
        }
    }
}

/// A simple in-memory TTL cache.
#[derive(Debug)]
pub struct TtlCache<K, V> {
    entries: HashMap<K, TtlEntry<V>>,
    default_ttl: Duration,
}

impl<K, V> TtlCache<K, V>
where
    K: Eq + Hash + Clone,
{
    /// Create a new TTL cache with the specified default TTL.
    pub fn new(default_ttl: Duration) -> Self {
        Self {
            entries: HashMap::new(),
            default_ttl,
        }
    }

    /// Insert a value with the default TTL.
    pub fn insert(&mut self, key: K, value: V) {
        let entry = TtlEntry::new(value, self.default_ttl);
        self.entries.insert(key, entry);
    }

    /// Insert a value with a custom TTL.
    pub fn insert_with_ttl(&mut self, key: K, value: V, ttl: Duration) {
        let entry = TtlEntry::new(value, ttl);
        self.entries.insert(key, entry);
    }

    /// Get a value if it exists and hasn't expired.
    pub fn get(&self, key: &K) -> Option<&V> {
        self.entries.get(key).and_then(|entry| {
            if entry.is_expired() {
                None
            } else {
                Some(&entry.value)
            }
        })
    }

    /// Check if a key exists and hasn't expired.
    pub fn contains(&self, key: &K) -> bool {
        self.get(key).is_some()
    }

    /// Remove a key from the cache.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.entries.remove(key).map(|e| e.value)
    }

    /// Remove all expired entries.
    pub fn cleanup(&mut self) -> usize {
        let before = self.entries.len();
        self.entries.retain(|_, entry| !entry.is_expired());
        before - self.entries.len()
    }

    /// Get the number of entries (including expired ones).
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// Calculate the TTL duration from seconds.
pub fn ttl_from_secs(seconds: i64) -> Duration {
    Duration::seconds(seconds)
}

/// Calculate the TTL duration from minutes.
pub fn ttl_from_mins(minutes: i64) -> Duration {
    Duration::minutes(minutes)
}

/// Calculate the TTL duration from hours.
pub fn ttl_from_hours(hours: i64) -> Duration {
    Duration::hours(hours)
}

/// Calculate the TTL duration from days.
pub fn ttl_from_days(days: i64) -> Duration {
    Duration::days(days)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ttl_entry_is_expired() {
        let entry = TtlEntry::new("test", Duration::seconds(-1));
        assert!(entry.is_expired());
    }

    #[test]
    fn test_ttl_entry_not_expired() {
        let entry = TtlEntry::new("test", Duration::seconds(3600));
        assert!(!entry.is_expired());
    }

    #[test]
    fn test_ttl_entry_remaining_ttl() {
        let entry = TtlEntry::new("test", Duration::seconds(3600));
        assert!(entry.remaining_ttl().is_some());
        
        let expired_entry = TtlEntry::new("test", Duration::seconds(-1));
        assert!(expired_entry.remaining_ttl().is_none());
    }

    #[test]
    fn test_ttl_cache_insert_and_get() {
        let mut cache = TtlCache::new(Duration::seconds(3600));
        cache.insert("key1", "value1");
        
        assert_eq!(cache.get(&"key1"), Some(&"value1"));
        assert!(!cache.contains(&"key2"));
    }

    #[test]
    fn test_ttl_cache_expired_entry() {
        let mut cache = TtlCache::new(Duration::milliseconds(1));
        cache.insert("key1", "value1");
        
        // Wait for expiration
        std::thread::sleep(std::time::Duration::from_millis(10));
        
        assert!(cache.get(&"key1").is_none());
    }

    #[test]
    fn test_ttl_cache_remove() {
        let mut cache = TtlCache::new(Duration::seconds(3600));
        cache.insert("key1", "value1");
        
        assert_eq!(cache.remove(&"key1"), Some("value1"));
        assert!(cache.is_empty());
    }

    #[test]
    fn test_ttl_cache_cleanup() {
        let mut cache = TtlCache::new(Duration::seconds(-1));
        cache.insert("key1", "value1");
        cache.insert("key2", "value2");
        
        let cleaned = cache.cleanup();
        assert_eq!(cleaned, 2);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_ttl_helpers() {
        assert_eq!(ttl_from_secs(60), Duration::seconds(60));
        assert_eq!(ttl_from_mins(5), Duration::minutes(5));
        assert_eq!(ttl_from_hours(2), Duration::hours(2));
        assert_eq!(ttl_from_days(1), Duration::days(1));
    }
}
