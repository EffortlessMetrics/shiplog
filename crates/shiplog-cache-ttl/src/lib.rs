//! TTL (Time-To-Live) based cache implementation.
//!
//! This module provides an in-memory cache with time-based expiration.
//! Each cache entry has a TTL after which it automatically expires.

use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::hash::Hash;

/// Cache entry with expiration time.
#[derive(Debug, Clone)]
struct CacheEntry<V> {
    value: V,
    expires_at: DateTime<Utc>,
}

/// TTL Cache with time-based expiration.
///
/// Entries automatically expire after their TTL elapses.
#[derive(Debug)]
pub struct TtlCache<K, V> {
    entries: HashMap<K, CacheEntry<V>>,
    default_ttl: Duration,
}

impl<K, V> TtlCache<K, V>
where
    K: Hash + Eq + Clone,
{
    /// Create a new TTL cache with the specified default TTL.
    pub fn new(ttl_secs: i64) -> Self {
        Self {
            entries: HashMap::new(),
            default_ttl: Duration::seconds(ttl_secs),
        }
    }

    /// Create a new TTL cache with a default TTL of 1 hour.
    pub fn with_default_ttl() -> Self {
        Self::new(3600)
    }

    /// Get a value from the cache if it exists and hasn't expired.
    pub fn get(&self, key: &K) -> Option<&V> {
        let now = Utc::now();
        self.entries
            .get(key)
            .filter(|entry| entry.expires_at > now)
            .map(|entry| &entry.value)
    }

    /// Insert a value into the cache with the default TTL.
    pub fn put(&mut self, key: K, value: V) {
        let expires_at = Utc::now() + self.default_ttl;
        self.entries.insert(key, CacheEntry { value, expires_at });
    }

    /// Insert a value with a custom TTL (in seconds).
    pub fn put_with_ttl(&mut self, key: K, value: V, ttl_secs: i64) {
        let expires_at = Utc::now() + Duration::seconds(ttl_secs);
        self.entries.insert(key, CacheEntry { value, expires_at });
    }

    /// Check if the cache contains a non-expired key.
    pub fn contains_key(&self, key: &K) -> bool {
        self.get(key).is_some()
    }

    /// Remove a key from the cache.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.entries.remove(key).map(|e| e.value)
    }

    /// Clear all expired entries.
    pub fn cleanup_expired(&mut self) -> usize {
        let now = Utc::now();
        let before = self.entries.len();
        self.entries.retain(|_, entry| entry.expires_at > now);
        before - self.entries.len()
    }

    /// Clear all entries from the cache.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Get the number of valid (non-expired) entries.
    pub fn len(&self) -> usize {
        let now = Utc::now();
        self.entries
            .values()
            .filter(|entry| entry.expires_at > now)
            .count()
    }

    /// Check if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the total number of entries (including expired).
    pub fn total_entries(&self) -> usize {
        self.entries.len()
    }

    /// Set the default TTL for new entries.
    pub fn set_default_ttl(&mut self, ttl_secs: i64) {
        self.default_ttl = Duration::seconds(ttl_secs);
    }
}

impl<K, V> Default for TtlCache<K, V>
where
    K: Hash + Eq + Clone,
{
    fn default() -> Self {
        Self::with_default_ttl()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ttl_cache_basic_operations() {
        let mut cache = TtlCache::new(3600);

        // Initially empty
        assert!(cache.is_empty());

        // Insert items
        cache.put("a", 1);
        cache.put("b", 2);

        assert_eq!(cache.len(), 2);
        assert!(cache.contains_key(&"a"));

        // Get existing item
        assert_eq!(cache.get(&"a"), Some(&1));

        // Get non-existent item
        assert_eq!(cache.get(&"c"), None);
    }

    #[test]
    fn test_ttl_cache_expiration() {
        let mut cache = TtlCache::new(1); // 1 second TTL

        cache.put("a", 1);

        // Should be retrievable immediately
        assert_eq!(cache.get(&"a"), Some(&1));

        // Wait for expiration
        std::thread::sleep(std::time::Duration::from_millis(1100));

        // Should not be retrievable after expiration
        assert_eq!(cache.get(&"a"), None);
    }

    #[test]
    fn test_ttl_cache_custom_ttl() {
        let mut cache = TtlCache::new(3600); // default 1 hour

        cache.put_with_ttl("a", 1, 1); // 1 second TTL

        assert_eq!(cache.get(&"a"), Some(&1));

        std::thread::sleep(std::time::Duration::from_millis(1100));

        assert_eq!(cache.get(&"a"), None);
    }

    #[test]
    fn test_ttl_cache_cleanup() {
        let mut cache = TtlCache::new(-1); // already expired

        cache.put("a", 1);
        cache.put("b", 2);

        // All entries should be expired
        assert_eq!(cache.len(), 0);
        assert_eq!(cache.total_entries(), 2);

        // Cleanup should remove expired entries
        let cleaned = cache.cleanup_expired();
        assert_eq!(cleaned, 2);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_ttl_cache_remove() {
        let mut cache = TtlCache::new(3600);

        cache.put("a", 1);
        let removed = cache.remove(&"a");

        assert_eq!(removed, Some(1));
        assert!(!cache.contains_key(&"a"));
    }

    #[test]
    fn test_ttl_cache_clear() {
        let mut cache = TtlCache::new(3600);

        cache.put("a", 1);
        cache.put("b", 2);
        cache.clear();

        assert!(cache.is_empty());
    }

    #[test]
    fn test_ttl_cache_update_existing() {
        let mut cache = TtlCache::new(3600);

        cache.put("a", 1);
        cache.put("a", 2);

        assert_eq!(cache.get(&"a"), Some(&2));
        assert_eq!(cache.len(), 1);
    }
}
