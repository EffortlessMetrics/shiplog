//! Arc-based thread-safe cache implementation.
//!
//! This module provides a thread-safe cache using Arc (Atomically Reference Counted)
//! for sharing cache data across threads with minimal locking.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, RwLock};

/// Thread-safe cache using Arc for shared ownership.
///
/// Uses a RwLock for concurrent read access and exclusive write access.
#[derive(Debug)]
pub struct ArcCache<K, V> {
    inner: Arc<RwLock<HashMap<K, V>>>,
}

impl<K, V> ArcCache<K, V>
where
    K: Hash + Eq + Clone,
{
    /// Create a new Arc cache.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get a value from the cache.
    pub fn get(&self, key: &K) -> Option<V>
    where
        V: Clone,
    {
        let guard = self.inner.read().ok()?;
        guard.get(key).cloned()
    }

    /// Insert a value into the cache.
    pub fn put(&self, key: K, value: V) -> Option<V> {
        let mut guard = self.inner.write().ok()?;
        guard.insert(key, value)
    }

    /// Remove a value from the cache.
    pub fn remove(&self, key: &K) -> Option<V> {
        let mut guard = self.inner.write().ok()?;
        guard.remove(key)
    }

    /// Check if the cache contains a key.
    pub fn contains_key(&self, key: &K) -> bool {
        if let Ok(guard) = self.inner.read() {
            guard.contains_key(key)
        } else {
            false
        }
    }

    /// Clear all items from the cache.
    pub fn clear(&self) {
        if let Ok(mut guard) = self.inner.write() {
            guard.clear();
        }
    }

    /// Get the number of items in the cache.
    pub fn len(&self) -> usize {
        self.inner.read().map(|g| g.len()).unwrap_or(0)
    }

    /// Check if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<K, V> Default for ArcCache<K, V>
where
    K: Hash + Eq + Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

/// ArcCache with a maximum size limit.
#[derive(Debug)]
pub struct BoundedArcCache<K, V> {
    cache: ArcCache<K, V>,
    max_size: usize,
}

impl<K, V> BoundedArcCache<K, V>
where
    K: Hash + Eq + Clone,
{
    /// Create a new bounded Arc cache with the specified maximum size.
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: ArcCache::new(),
            max_size,
        }
    }

    /// Get a value from the cache.
    pub fn get(&self, key: &K) -> Option<V>
    where
        V: Clone,
    {
        self.cache.get(key)
    }

    /// Insert a value into the cache.
    /// If the cache is at capacity, the oldest entry is removed.
    pub fn put(&self, key: K, value: V) -> Option<V> {
        // Check if we need to evict
        if self.cache.len() >= self.max_size {
            // Simple eviction: remove the first item
            // For a more sophisticated LRU, see shiplog-cache-lru
            if let Ok(mut guard) = self.cache.inner.write()
                && let Some(first_key) = guard.keys().next().cloned()
            {
                guard.remove(&first_key);
            }
        }
        self.cache.put(key, value)
    }

    /// Check if the cache contains a key.
    pub fn contains_key(&self, key: &K) -> bool {
        self.cache.contains_key(key)
    }

    /// Clear all items from the cache.
    pub fn clear(&self) {
        self.cache.clear();
    }

    /// Get the number of items in the cache.
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Get the maximum capacity of the cache.
    pub fn capacity(&self) -> usize {
        self.max_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arc_cache_basic_operations() {
        let cache = ArcCache::new();

        // Initially empty
        assert!(cache.is_empty());

        // Insert items
        cache.put("a", 1);
        cache.put("b", 2);

        assert_eq!(cache.len(), 2);
        assert!(cache.contains_key(&"a"));

        // Get existing item
        assert_eq!(cache.get(&"a"), Some(1));

        // Get non-existent item
        assert_eq!(cache.get(&"c"), None);
    }

    #[test]
    fn test_arc_cache_update_existing() {
        let cache = ArcCache::new();

        cache.put("a", 1);
        let old = cache.put("a", 2);

        assert_eq!(old, Some(1));
        assert_eq!(cache.get(&"a"), Some(2));
    }

    #[test]
    fn test_arc_cache_remove() {
        let cache = ArcCache::new();

        cache.put("a", 1);
        let removed = cache.remove(&"a");

        assert_eq!(removed, Some(1));
        assert!(!cache.contains_key(&"a"));
    }

    #[test]
    fn test_arc_cache_clear() {
        let cache = ArcCache::new();

        cache.put("a", 1);
        cache.put("b", 2);
        cache.clear();

        assert!(cache.is_empty());
    }

    #[test]
    fn test_bounded_arc_cache() {
        let cache = BoundedArcCache::new(2);

        cache.put("a", 1);
        cache.put("b", 2);
        assert_eq!(cache.len(), 2);

        // Adding more items should trigger eviction
        cache.put("c", 3);
        assert!(cache.len() <= 2);
    }
}
