//! LRU (Least Recently Used) cache implementation.
//!
//! This module provides an in-memory LRU cache with a fixed capacity.
//! When the cache is full, the least recently used item is evicted.

use std::collections::HashMap;
use std::hash::Hash;

/// LRU Cache with fixed capacity.
///
/// Maintains items in order of access, evicting the least recently
/// used item when capacity is exceeded.
#[derive(Debug)]
pub struct LruCache<K, V> {
    capacity: usize,
    items: HashMap<K, V>,
    access_order: Vec<K>,
}

impl<K, V> LruCache<K, V>
where
    K: Hash + Eq + Clone,
{
    /// Create a new LRU cache with the specified capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            items: HashMap::new(),
            access_order: Vec::new(),
        }
    }

    /// Get a value from the cache, updating its access order.
    pub fn get(&mut self, key: &K) -> Option<&V> {
        if let Some(pos) = self.access_order.iter().position(|k| k == key) {
            // Move to end (most recently used)
            let k = self.access_order.remove(pos);
            self.access_order.push(k);
            self.items.get(key)
        } else {
            None
        }
    }

    /// Insert a value into the cache.
    /// If the key already exists, its value is updated and access order is refreshed.
    /// If the cache is full, the least recently used item is evicted.
    pub fn put(&mut self, key: K, value: V) -> Option<V> {
        // Check if key exists
        if let Some(pos) = self.access_order.iter().position(|k| k == &key) {
            // Update existing item
            self.access_order.remove(pos);
            self.access_order.push(key.clone());
            return self.items.insert(key, value);
        }

        // Evict if at capacity
        if self.items.len() >= self.capacity
            && let Some(lru_key) = self.access_order.first().cloned()
        {
            self.items.remove(&lru_key);
            self.access_order.remove(0);
        }

        // Insert new item
        self.access_order.push(key.clone());
        self.items.insert(key, value)
    }

    /// Check if the cache contains a key.
    pub fn contains_key(&self, key: &K) -> bool {
        self.items.contains_key(key)
    }

    /// Remove a key from the cache.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        if let Some(pos) = self.access_order.iter().position(|k| k == key) {
            self.access_order.remove(pos);
            self.items.remove(key)
        } else {
            None
        }
    }

    /// Clear all items from the cache.
    pub fn clear(&mut self) {
        self.items.clear();
        self.access_order.clear();
    }

    /// Get the number of items in the cache.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get the capacity of the cache.
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lru_cache_basic_operations() {
        let mut cache = LruCache::new(3);

        // Initially empty
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);

        // Insert items
        cache.put("a", 1);
        cache.put("b", 2);
        cache.put("c", 3);

        assert_eq!(cache.len(), 3);
        assert!(cache.contains_key(&"a"));

        // Get existing item
        assert_eq!(cache.get(&"a"), Some(&1));

        // Get non-existent item
        assert_eq!(cache.get(&"d"), None);
    }

    #[test]
    fn test_lru_cache_eviction() {
        let mut cache = LruCache::new(3);

        // Fill the cache
        cache.put("a", 1);
        cache.put("b", 2);
        cache.put("c", 3);

        // Access "a" to make it most recently used
        cache.get(&"a");

        // Add another item - should evict "b" (least recently used)
        cache.put("d", 4);

        assert!(cache.contains_key(&"a"));
        assert!(!cache.contains_key(&"b"));
        assert!(cache.contains_key(&"c"));
        assert!(cache.contains_key(&"d"));
    }

    #[test]
    fn test_lru_cache_update_existing() {
        let mut cache = LruCache::new(3);

        cache.put("a", 1);
        cache.put("a", 2);

        assert_eq!(cache.get(&"a"), Some(&2));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_lru_cache_remove() {
        let mut cache = LruCache::new(3);

        cache.put("a", 1);
        cache.put("b", 2);

        let removed = cache.remove(&"a");
        assert_eq!(removed, Some(1));
        assert!(!cache.contains_key(&"a"));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_lru_cache_clear() {
        let mut cache = LruCache::new(3);

        cache.put("a", 1);
        cache.put("b", 2);
        cache.clear();

        assert!(cache.is_empty());
    }

    #[test]
    fn test_lru_cache_at_capacity() {
        let mut cache = LruCache::new(2);

        cache.put("a", 1);
        cache.put("b", 2);
        assert_eq!(cache.len(), 2);

        // Adding more items should trigger eviction
        cache.put("c", 3);
        assert_eq!(cache.len(), 2);

        // The oldest should be evicted
        assert!(!cache.contains_key(&"a"));
        assert!(cache.contains_key(&"b"));
        assert!(cache.contains_key(&"c"));
    }
}
