use proptest::prelude::*;
use shiplog_lru::LruCache;

// ── Known-answer tests ──────────────────────────────────────────────

#[test]
fn insert_and_retrieve() {
    let mut cache = LruCache::new(3);
    cache.insert("a", 1);
    cache.insert("b", 2);
    cache.insert("c", 3);
    assert_eq!(cache.get(&"a"), Some(&1));
    assert_eq!(cache.get(&"b"), Some(&2));
    assert_eq!(cache.get(&"c"), Some(&3));
}

#[test]
fn eviction_drops_lru() {
    let mut cache = LruCache::new(2);
    cache.insert(1, "a");
    cache.insert(2, "b");
    cache.insert(3, "c"); // evicts 1
    assert!(!cache.contains(&1));
    assert!(cache.contains(&2));
    assert!(cache.contains(&3));
}

#[test]
fn get_promotes_to_mru() {
    let mut cache = LruCache::new(2);
    cache.insert(1, "a");
    cache.insert(2, "b");
    cache.get(&1); // promotes 1, so 2 becomes LRU
    cache.insert(3, "c"); // evicts 2
    assert!(cache.contains(&1));
    assert!(!cache.contains(&2));
    assert!(cache.contains(&3));
}

#[test]
fn update_existing_key() {
    let mut cache = LruCache::new(3);
    cache.insert(1, "old");
    cache.insert(1, "new");
    assert_eq!(cache.get(&1), Some(&"new"));
    assert_eq!(cache.len(), 1);
}

#[test]
fn remove_returns_value() {
    let mut cache = LruCache::new(3);
    cache.insert(1, 100);
    assert_eq!(cache.remove(&1), Some(100));
    assert!(!cache.contains(&1));
    assert_eq!(cache.len(), 0);
}

#[test]
fn remove_nonexistent_returns_none() {
    let mut cache: LruCache<i32, i32> = LruCache::new(3);
    assert_eq!(cache.remove(&99), None);
}

#[test]
fn clear_empties_cache() {
    let mut cache = LruCache::new(5);
    for i in 0..5 {
        cache.insert(i, i * 10);
    }
    cache.clear();
    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
    for i in 0..5 {
        assert!(!cache.contains(&i));
    }
}

// ── Edge cases ──────────────────────────────────────────────────────

#[test]
fn capacity_one() {
    let mut cache = LruCache::new(1);
    cache.insert("a", 1);
    assert_eq!(cache.get(&"a"), Some(&1));
    cache.insert("b", 2); // evicts "a"
    assert!(!cache.contains(&"a"));
    assert_eq!(cache.get(&"b"), Some(&2));
}

#[test]
fn get_nonexistent_returns_none() {
    let mut cache: LruCache<i32, i32> = LruCache::new(3);
    assert_eq!(cache.get(&42), None);
}

#[test]
fn contains_does_not_promote() {
    let mut cache = LruCache::new(2);
    cache.insert(1, "a");
    cache.insert(2, "b");
    // contains is read-only, doesn't change order
    assert!(cache.contains(&1));
    cache.insert(3, "c"); // should evict 1 since contains doesn't promote
    assert!(!cache.contains(&1));
}

#[test]
fn insert_at_capacity_repeatedly() {
    let mut cache = LruCache::new(2);
    for i in 0..100 {
        cache.insert(i, i);
    }
    assert_eq!(cache.len(), 2);
    assert!(cache.contains(&98));
    assert!(cache.contains(&99));
}

#[test]
fn empty_cache_properties() {
    let cache: LruCache<String, String> = LruCache::new(10);
    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
    assert_eq!(cache.capacity(), 10);
}

#[test]
fn update_promotes_to_mru() {
    let mut cache = LruCache::new(2);
    cache.insert(1, "a");
    cache.insert(2, "b");
    cache.insert(1, "a_updated"); // update key 1, promoting it
    cache.insert(3, "c"); // should evict 2 (the LRU)
    assert!(cache.contains(&1));
    assert!(!cache.contains(&2));
    assert!(cache.contains(&3));
}

// ── Property tests ──────────────────────────────────────────────────

proptest! {
    #[test]
    fn len_never_exceeds_capacity(
        cap in 1usize..20,
        inserts in prop::collection::vec(0u32..100, 0..50)
    ) {
        let mut cache = LruCache::new(cap);
        for &k in &inserts {
            cache.insert(k, k);
            prop_assert!(cache.len() <= cap);
        }
    }

    #[test]
    fn last_inserted_always_present(
        cap in 1usize..20,
        keys in prop::collection::vec(0u32..100, 1..30)
    ) {
        let mut cache = LruCache::new(cap);
        for &k in &keys {
            cache.insert(k, k);
        }
        let last = keys.last().unwrap();
        prop_assert!(cache.contains(last));
    }

    #[test]
    fn insert_then_get_returns_value(
        cap in 1usize..20, key in 0u32..100, val in 0u32..1000
    ) {
        let mut cache = LruCache::new(cap);
        cache.insert(key, val);
        prop_assert_eq!(cache.get(&key), Some(&val));
    }

    #[test]
    fn remove_then_not_contains(
        cap in 1usize..20,
        keys in prop::collection::vec(0u32..50, 1..20),
        remove_idx in 0usize..20
    ) {
        let mut cache = LruCache::new(cap);
        for &k in &keys {
            cache.insert(k, k);
        }
        if let Some(&key) = keys.get(remove_idx % keys.len()) {
            cache.remove(&key);
            prop_assert!(!cache.contains(&key));
        }
    }

    #[test]
    fn clear_makes_empty(
        cap in 1usize..20,
        keys in prop::collection::vec(0u32..50, 0..20)
    ) {
        let mut cache = LruCache::new(cap);
        for &k in &keys {
            cache.insert(k, k);
        }
        cache.clear();
        prop_assert!(cache.is_empty());
        prop_assert_eq!(cache.len(), 0);
    }

    #[test]
    fn capacity_unchanged_after_operations(
        cap in 1usize..50,
        ops in prop::collection::vec(0u32..100, 0..30)
    ) {
        let mut cache = LruCache::new(cap);
        for &k in &ops {
            cache.insert(k, k);
        }
        prop_assert_eq!(cache.capacity(), cap);
    }
}
