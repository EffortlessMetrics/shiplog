use proptest::prelude::*;
use shiplog_cache_lru::LruCache;

// ── Basics ──────────────────────────────────────────────────────────

#[test]
fn new_cache_is_empty() {
    let cache: LruCache<String, i32> = LruCache::new(10);
    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
    assert_eq!(cache.capacity(), 10);
}

#[test]
fn put_and_get() {
    let mut cache = LruCache::new(5);
    assert_eq!(cache.put("a", 1), None);
    assert_eq!(cache.get(&"a"), Some(&1));
    assert!(cache.contains_key(&"a"));
}

#[test]
fn get_nonexistent_returns_none() {
    let mut cache: LruCache<&str, i32> = LruCache::new(5);
    assert_eq!(cache.get(&"x"), None);
}

#[test]
fn put_returns_old_value_on_update() {
    let mut cache = LruCache::new(5);
    assert_eq!(cache.put("k", 1), None);
    assert_eq!(cache.put("k", 2), Some(1));
    assert_eq!(cache.get(&"k"), Some(&2));
    assert_eq!(cache.len(), 1);
}

// ── Remove ──────────────────────────────────────────────────────────

#[test]
fn remove_existing_key() {
    let mut cache = LruCache::new(5);
    cache.put("a", 10);
    assert_eq!(cache.remove(&"a"), Some(10));
    assert!(!cache.contains_key(&"a"));
    assert!(cache.is_empty());
}

#[test]
fn remove_nonexistent_key() {
    let mut cache: LruCache<&str, i32> = LruCache::new(5);
    assert_eq!(cache.remove(&"x"), None);
}

// ── Clear ───────────────────────────────────────────────────────────

#[test]
fn clear_empties_cache() {
    let mut cache = LruCache::new(5);
    cache.put("a", 1);
    cache.put("b", 2);
    cache.clear();
    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
}

// ── Eviction: LRU ordering ─────────────────────────────────────────

#[test]
fn evicts_least_recently_used() {
    let mut cache = LruCache::new(3);
    cache.put("a", 1);
    cache.put("b", 2);
    cache.put("c", 3);
    // a is LRU
    cache.put("d", 4);
    assert!(!cache.contains_key(&"a"), "LRU item should be evicted");
    assert!(cache.contains_key(&"b"));
    assert!(cache.contains_key(&"c"));
    assert!(cache.contains_key(&"d"));
}

#[test]
fn get_refreshes_access_order() {
    let mut cache = LruCache::new(3);
    cache.put("a", 1);
    cache.put("b", 2);
    cache.put("c", 3);
    // Touch a => now b is LRU
    cache.get(&"a");
    cache.put("d", 4);
    assert!(cache.contains_key(&"a"), "refreshed item should survive");
    assert!(
        !cache.contains_key(&"b"),
        "LRU after refresh should be evicted"
    );
}

#[test]
fn put_update_refreshes_access_order() {
    let mut cache = LruCache::new(3);
    cache.put("a", 1);
    cache.put("b", 2);
    cache.put("c", 3);
    // Update a => refreshes its position, b becomes LRU
    cache.put("a", 10);
    cache.put("d", 4);
    assert!(cache.contains_key(&"a"));
    assert!(!cache.contains_key(&"b"));
}

// ── Capacity 1 ──────────────────────────────────────────────────────

#[test]
fn capacity_one() {
    let mut cache = LruCache::new(1);
    cache.put("a", 1);
    assert_eq!(cache.get(&"a"), Some(&1));
    cache.put("b", 2);
    assert!(!cache.contains_key(&"a"));
    assert_eq!(cache.get(&"b"), Some(&2));
    assert_eq!(cache.len(), 1);
}

// ── Sequential eviction ─────────────────────────────────────────────

#[test]
fn sequential_eviction_order() {
    let mut cache = LruCache::new(3);
    // Insert a, b, c
    cache.put("a", 1);
    cache.put("b", 2);
    cache.put("c", 3);

    // Insert d => evicts a
    cache.put("d", 4);
    assert!(!cache.contains_key(&"a"));

    // Insert e => evicts b
    cache.put("e", 5);
    assert!(!cache.contains_key(&"b"));

    // Insert f => evicts c
    cache.put("f", 6);
    assert!(!cache.contains_key(&"c"));

    // d, e, f remain
    assert!(cache.contains_key(&"d"));
    assert!(cache.contains_key(&"e"));
    assert!(cache.contains_key(&"f"));
}

// ── Property tests ──────────────────────────────────────────────────

proptest! {
    #[test]
    fn never_exceeds_capacity(
        cap in 1usize..20,
        ops in prop::collection::vec((0u32..30, 0i32..100), 1..60)
    ) {
        let mut cache = LruCache::new(cap);
        for (k, v) in &ops {
            cache.put(*k, *v);
        }
        prop_assert!(cache.len() <= cap, "len {} > cap {}", cache.len(), cap);
    }

    #[test]
    fn put_get_roundtrip(key in 0u32..50, val in 0i32..1000) {
        let mut cache = LruCache::new(100);
        cache.put(key, val);
        prop_assert_eq!(cache.get(&key), Some(&val));
    }

    #[test]
    fn remove_makes_key_absent(key in 0u32..50, val in 0i32..1000) {
        let mut cache = LruCache::new(100);
        cache.put(key, val);
        cache.remove(&key);
        prop_assert!(!cache.contains_key(&key));
    }

    #[test]
    fn last_put_wins(key in 0u32..10, v1 in 0i32..100, v2 in 0i32..100) {
        let mut cache = LruCache::new(100);
        cache.put(key, v1);
        cache.put(key, v2);
        prop_assert_eq!(cache.get(&key), Some(&v2));
        prop_assert_eq!(cache.len(), 1);
    }

    #[test]
    fn most_recent_n_survive(
        cap in 2usize..10,
        keys in prop::collection::vec(0u32..1000, 10..40)
    ) {
        let mut cache = LruCache::new(cap);
        for &k in &keys {
            cache.put(k, 0);
        }
        // Deduplicate while preserving last occurrence order
        let mut seen = std::collections::HashSet::new();
        let mut unique_last: Vec<u32> = Vec::new();
        for &k in keys.iter().rev() {
            if seen.insert(k) {
                unique_last.push(k);
            }
        }
        unique_last.reverse();
        // The last `cap` unique keys should be in the cache
        let expected: Vec<u32> = unique_last.iter().rev().take(cap).copied().collect();
        for k in &expected {
            prop_assert!(cache.contains_key(k), "key {} should be in cache", k);
        }
    }
}
