//! LRU (Least Recently Used) cache implementation for shiplog.
//!
//! This crate provides an in-memory LRU cache implementation that evicts
//! the least recently used items when the capacity is reached.

use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

/// An LRU (Least Recently Used) cache with a fixed capacity.
#[derive(Debug)]
pub struct LruCache<K, V> {
    capacity: usize,
    map: HashMap<K, V>,
    order: Vec<K>,
}

impl<K, V> LruCache<K, V>
where
    K: Eq + Hash + Clone,
{
    /// Create a new LRU cache with the given capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            map: HashMap::new(),
            order: Vec::new(),
        }
    }

    /// Get a value from the cache, marking it as recently used.
    pub fn get(&mut self, key: &K) -> Option<&V> {
        if self.map.contains_key(key) {
            // Move key to the end (most recently used)
            self.order.retain(|k| k != key);
            self.order.push(key.clone());
            self.map.get(key)
        } else {
            None
        }
    }

    /// Insert a key-value pair into the cache.
    /// If the key already exists, it will be updated and moved to front.
    /// If the cache is full, the least recently used item will be evicted.
    pub fn insert(&mut self, key: K, value: V) {
        // If key exists, update and move to front
        if self.map.contains_key(&key) {
            self.map.insert(key.clone(), value);
            self.order.retain(|k| k != &key);
            self.order.push(key);
            return;
        }

        // If cache is full, evict LRU item
        if self.map.len() >= self.capacity
            && let Some(lru_key) = self.order.first().cloned()
        {
            self.map.remove(&lru_key);
            self.order.remove(0);
        }

        // Insert new item
        self.map.insert(key.clone(), value);
        self.order.push(key);
    }

    /// Remove a key from the cache.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        if let Some(value) = self.map.remove(key) {
            self.order.retain(|k| k != key);
            Some(value)
        } else {
            None
        }
    }

    /// Check if the cache contains a key.
    pub fn contains(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    /// Get the number of items in the cache.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Check if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Get the capacity of the cache.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Clear all items from the cache.
    pub fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lru_cache_new() {
        let cache: LruCache<i32, &str> = LruCache::new(3);
        assert_eq!(cache.capacity(), 3);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_lru_cache_insert_and_get() {
        let mut cache = LruCache::new(3);
        cache.insert(1, "one");
        cache.insert(2, "two");

        assert_eq!(cache.get(&1), Some(&"one"));
        assert_eq!(cache.get(&2), Some(&"two"));
    }

    #[test]
    fn test_lru_cache_eviction() {
        let mut cache = LruCache::new(3);
        cache.insert(1, "one");
        cache.insert(2, "two");
        cache.insert(3, "three");

        // This should evict key 1 (least recently used) when we add the 4th item
        cache.insert(4, "four");

        // Key 1 should be evicted now (oldest)
        assert!(!cache.contains(&1));
        // Keys 2, 3, 4 should still be there
        assert!(cache.contains(&2));
        assert!(cache.contains(&3));
        assert!(cache.contains(&4));
    }

    #[test]
    fn test_lru_cache_lru_eviction_order() {
        let mut cache = LruCache::new(2);
        cache.insert(1, "one");
        cache.insert(2, "two");

        // Access key 1, making key 2 the LRU
        cache.get(&1);

        // This should evict key 2
        cache.insert(3, "three");

        assert!(cache.contains(&1));
        assert!(!cache.contains(&2));
        assert!(cache.contains(&3));
    }

    #[test]
    fn test_lru_cache_update_existing() {
        let mut cache = LruCache::new(3);
        cache.insert(1, "one");
        cache.insert(1, "updated");

        assert_eq!(cache.get(&1), Some(&"updated"));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_lru_cache_remove() {
        let mut cache = LruCache::new(3);
        cache.insert(1, "one");

        assert_eq!(cache.remove(&1), Some("one"));
        assert!(!cache.contains(&1));
        assert!(cache.is_empty());
    }

    #[test]
    fn test_lru_cache_clear() {
        let mut cache = LruCache::new(3);
        cache.insert(1, "one");
        cache.insert(2, "two");

        cache.clear();

        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_lru_cache_get_marks_as_used() {
        let mut cache = LruCache::new(2);
        cache.insert(1, "one");
        cache.insert(2, "two");

        // Access key 1, making it recently used
        cache.get(&1);

        // Insert new item - should evict key 2
        cache.insert(3, "three");

        assert!(cache.contains(&1));
        assert!(!cache.contains(&2));
        assert!(cache.contains(&3));
    }
}
