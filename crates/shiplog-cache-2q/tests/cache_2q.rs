use proptest::prelude::*;
use shiplog_cache_2q::TwoQCache;

// ── Basics ──────────────────────────────────────────────────────────

#[test]
fn new_cache_is_empty() {
    let cache: TwoQCache<String, i32> = TwoQCache::new(10);
    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
    assert_eq!(cache.capacity(), 10);
}

#[test]
fn put_and_get() {
    let mut cache = TwoQCache::new(5);
    assert_eq!(cache.put("a", 1), None);
    assert_eq!(cache.get(&"a"), Some(&1));
    assert!(cache.contains_key(&"a"));
}

#[test]
fn get_nonexistent_returns_none() {
    let mut cache: TwoQCache<&str, i32> = TwoQCache::new(5);
    assert_eq!(cache.get(&"x"), None);
}

#[test]
fn put_returns_old_value() {
    let mut cache = TwoQCache::new(5);
    assert_eq!(cache.put("k", 1), None);
    assert_eq!(cache.put("k", 2), Some(1));
    assert_eq!(cache.get(&"k"), Some(&2));
    assert_eq!(cache.len(), 1);
}

// ── Remove ──────────────────────────────────────────────────────────

#[test]
fn remove_from_in_queue() {
    let mut cache = TwoQCache::new(5);
    cache.put("a", 1);
    assert_eq!(cache.remove(&"a"), Some(1));
    assert!(!cache.contains_key(&"a"));
    assert!(cache.is_empty());
}

#[test]
fn remove_from_out_queue() {
    let mut cache = TwoQCache::new(5);
    cache.put("a", 1);
    cache.get(&"a"); // promote to out queue
    assert_eq!(cache.remove(&"a"), Some(1));
    assert!(!cache.contains_key(&"a"));
}

#[test]
fn remove_nonexistent() {
    let mut cache: TwoQCache<&str, i32> = TwoQCache::new(5);
    assert_eq!(cache.remove(&"x"), None);
}

// ── Clear ───────────────────────────────────────────────────────────

#[test]
fn clear_empties_cache() {
    let mut cache = TwoQCache::new(5);
    cache.put("a", 1);
    cache.put("b", 2);
    cache.get(&"a"); // promote a
    cache.clear();
    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
}

// ── Eviction: in-queue FIFO ─────────────────────────────────────────

#[test]
fn eviction_removes_oldest_in_queue_first() {
    let mut cache = TwoQCache::new(3);
    cache.put("a", 1);
    cache.put("b", 2);
    cache.put("c", 3);
    // a is oldest in in-queue
    cache.put("d", 4);
    assert!(!cache.contains_key(&"a"));
    assert!(cache.contains_key(&"b"));
    assert!(cache.contains_key(&"c"));
    assert!(cache.contains_key(&"d"));
}

// ── Promotion ───────────────────────────────────────────────────────

#[test]
fn access_promotes_from_in_to_out() {
    let mut cache = TwoQCache::new(3);
    cache.put("a", 1);
    cache.put("b", 2);
    // Access a => promote to out queue
    cache.get(&"a");
    cache.put("c", 3);
    // Now at capacity. Next put evicts from in-queue first (b)
    cache.put("d", 4);
    assert!(cache.contains_key(&"a"), "promoted item should survive");
    assert!(!cache.contains_key(&"b"), "oldest in-queue item evicted");
}

#[test]
fn out_queue_lru_eviction_when_in_queue_empty() {
    let mut cache = TwoQCache::new(3);
    cache.put("a", 1);
    cache.put("b", 2);
    cache.put("c", 3);
    // Promote all to out queue
    cache.get(&"a");
    cache.get(&"b");
    cache.get(&"c");
    // In-queue is empty. Next inserts go to in-queue; evictions from in-queue first.
    cache.put("d", 4); // evicts oldest in-queue... but in-queue just has "d" after evict
    // Actually at capacity -> evict from in-queue (empty) -> evict from out-queue (LRU = "a")
    assert!(
        !cache.contains_key(&"a"),
        "LRU in out-queue should be evicted"
    );
}

// ── Capacity 1 edge case ────────────────────────────────────────────

#[test]
fn capacity_one() {
    let mut cache = TwoQCache::new(1);
    cache.put("a", 1);
    assert_eq!(cache.get(&"a"), Some(&1));
    cache.put("b", 2);
    assert!(!cache.contains_key(&"a"));
    assert_eq!(cache.get(&"b"), Some(&2));
}

// ── Property tests ──────────────────────────────────────────────────

proptest! {
    #[test]
    fn never_exceeds_capacity(
        cap in 1usize..20,
        ops in prop::collection::vec((0u32..30, 0i32..100), 1..60)
    ) {
        let mut cache = TwoQCache::new(cap);
        for (k, v) in &ops {
            cache.put(*k, *v);
        }
        prop_assert!(cache.len() <= cap, "len {} > cap {}", cache.len(), cap);
    }

    #[test]
    fn put_get_roundtrip(key in 0u32..50, val in 0i32..1000) {
        let mut cache = TwoQCache::new(100);
        cache.put(key, val);
        prop_assert_eq!(cache.get(&key), Some(&val));
    }

    #[test]
    fn remove_makes_key_absent(key in 0u32..50, val in 0i32..1000) {
        let mut cache = TwoQCache::new(100);
        cache.put(key, val);
        cache.remove(&key);
        prop_assert!(!cache.contains_key(&key));
    }

    #[test]
    fn update_preserves_length(key in 0u32..10, v1 in 0i32..100, v2 in 0i32..100) {
        let mut cache = TwoQCache::new(100);
        cache.put(key, v1);
        let len_before = cache.len();
        cache.put(key, v2);
        prop_assert_eq!(cache.len(), len_before);
    }
}
