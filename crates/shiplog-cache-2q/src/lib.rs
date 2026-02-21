//! 2Q (Two-Queue) cache implementation.
//!
//! This module provides a 2Q cache algorithm which uses two queues:
//! - A FIFO queue for first-time insertions (in queue)
//! - An LRU queue for frequently accessed items (out queue)
//!
//! The 2Q algorithm is efficient for workloads with a "temporal locality"
//! where items accessed once are unlikely to be accessed again.

use std::collections::{HashMap, VecDeque};
use std::hash::Hash;

/// 2Q Cache entry.
#[derive(Debug, Clone)]
struct CacheEntry<V> {
    value: V,
    /// Whether this entry is in the "out" (frequent) queue
    in_out_queue: bool,
}

/// 2Q Cache implementation.
///
/// Uses two queues: one for new entries (FIFO) and one for frequently
/// accessed entries (LRU). This provides better hit rates than simple LRU
/// for workloads with temporal locality.
#[derive(Debug)]
pub struct TwoQCache<K, V> {
    capacity: usize,
    /// Incoming queue - FIFO for new entries
    in_queue: VecDeque<K>,
    /// Outgoing queue - LRU for frequently accessed entries
    out_queue: VecDeque<K>,
    /// Map from key to value and queue location
    entries: HashMap<K, CacheEntry<V>>,
}

impl<K, V> TwoQCache<K, V>
where
    K: Hash + Eq + Clone,
{
    /// Create a new 2Q cache with the specified capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            in_queue: VecDeque::new(),
            out_queue: VecDeque::new(),
            entries: HashMap::new(),
        }
    }

    /// Get a value from the cache.
    pub fn get(&mut self, key: &K) -> Option<&V> {
        if let Some(entry) = self.entries.get_mut(key) {
            if entry.in_out_queue {
                // Move to end of out queue (most recently used)
                self.out_queue.retain(|k| k != key);
                self.out_queue.push_back(key.clone());
            } else {
                // Promote from in-queue to out-queue
                self.in_queue.retain(|k| k != key);
                self.out_queue.push_back(key.clone());
                entry.in_out_queue = true;
            }
            Some(&entry.value)
        } else {
            None
        }
    }

    /// Insert a value into the cache.
    pub fn put(&mut self, key: K, value: V) -> Option<V> {
        // Check if key already exists
        if let Some(entry) = self.entries.get_mut(&key) {
            // Update existing entry
            if entry.in_out_queue {
                // Move to end of out queue
                self.out_queue.retain(|k| k != &key);
                self.out_queue.push_back(key.clone());
            }
            return Some(std::mem::replace(&mut entry.value, value));
        }

        // Evict if at capacity
        if self.entries.len() >= self.capacity {
            self.evict();
        }

        // Add to in queue
        self.in_queue.push_back(key.clone());
        self.entries.insert(
            key,
            CacheEntry {
                value,
                in_out_queue: false,
            },
        );

        None
    }

    /// Check if the cache contains a key.
    pub fn contains_key(&self, key: &K) -> bool {
        self.entries.contains_key(key)
    }

    /// Remove a key from the cache.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        if let Some(entry) = self.entries.remove(key) {
            if entry.in_out_queue {
                self.out_queue.retain(|k| k != key);
            } else {
                self.in_queue.retain(|k| k != key);
            }
            Some(entry.value)
        } else {
            None
        }
    }

    /// Clear all items from the cache.
    pub fn clear(&mut self) {
        self.in_queue.clear();
        self.out_queue.clear();
        self.entries.clear();
    }

    /// Get the number of items in the cache.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get the capacity of the cache.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Evict the least recently used item.
    fn evict(&mut self) {
        // First try to evict from in queue (FIFO)
        if let Some(key) = self.in_queue.pop_front() {
            self.entries.remove(&key);
            return;
        }

        // If in queue is empty, evict from out queue (LRU)
        if let Some(key) = self.out_queue.pop_front() {
            self.entries.remove(&key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_2q_cache_basic_operations() {
        let mut cache = TwoQCache::new(3);

        // Initially empty
        assert!(cache.is_empty());

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
    fn test_2q_cache_promotion() {
        let mut cache = TwoQCache::new(3);

        // Fill the in queue
        cache.put("a", 1);
        cache.put("b", 2);

        // Access "a" to promote it to out queue
        cache.get(&"a");

        // Add more items to trigger eviction
        cache.put("c", 3);
        cache.put("d", 4);

        // "b" should be evicted (oldest in in-queue)
        // "a" should still be in cache (promoted to out-queue)
        assert!(cache.contains_key(&"a"));
        assert!(!cache.contains_key(&"b"));
        assert!(cache.contains_key(&"c"));
        assert!(cache.contains_key(&"d"));
    }

    #[test]
    fn test_2q_cache_update_existing() {
        let mut cache = TwoQCache::new(3);

        cache.put("a", 1);
        cache.put("a", 2);

        assert_eq!(cache.get(&"a"), Some(&2));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_2q_cache_remove() {
        let mut cache = TwoQCache::new(3);

        cache.put("a", 1);
        cache.put("b", 2);

        let removed = cache.remove(&"a");
        assert_eq!(removed, Some(1));
        assert!(!cache.contains_key(&"a"));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_2q_cache_clear() {
        let mut cache = TwoQCache::new(3);

        cache.put("a", 1);
        cache.put("b", 2);
        cache.clear();

        assert!(cache.is_empty());
    }

    #[test]
    fn test_2q_cache_at_capacity() {
        let mut cache = TwoQCache::new(2);

        cache.put("a", 1);
        cache.put("b", 2);
        assert_eq!(cache.len(), 2);

        // Adding more items should trigger eviction
        cache.put("c", 3);
        assert_eq!(cache.len(), 2);

        // Oldest should be evicted
        assert!(!cache.contains_key(&"a"));
    }

    #[test]
    fn test_2q_cache_out_queue_lru() {
        let mut cache = TwoQCache::new(3);

        // Fill and access to promote to out queue
        cache.put("a", 1);
        cache.put("b", 2);
        cache.put("c", 3);

        // Access in order to build out queue
        cache.get(&"a");
        cache.get(&"b");
        cache.get(&"c");

        // Fill in queue
        cache.put("d", 4);
        cache.put("e", 5);

        // Oldest in out queue ("a") should be evicted
        assert!(!cache.contains_key(&"a"));
        assert!(cache.contains_key(&"b"));
    }
}
