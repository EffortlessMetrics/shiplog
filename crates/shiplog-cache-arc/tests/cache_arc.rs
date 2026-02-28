use proptest::prelude::*;
use shiplog_cache_arc::*;
use std::sync::Arc;
use std::thread;

// ── ArcCache basics ─────────────────────────────────────────────────

#[test]
fn new_cache_is_empty() {
    let cache: ArcCache<String, i32> = ArcCache::new();
    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
}

#[test]
fn default_matches_new() {
    let a: ArcCache<String, i32> = ArcCache::default();
    assert!(a.is_empty());
}

#[test]
fn put_and_get() {
    let cache = ArcCache::new();
    cache.put("a", 1);
    assert_eq!(cache.get(&"a"), Some(1));
    assert!(cache.contains_key(&"a"));
}

#[test]
fn put_returns_previous_value() {
    let cache = ArcCache::new();
    assert_eq!(cache.put("k", 1), None);
    assert_eq!(cache.put("k", 2), Some(1));
    assert_eq!(cache.get(&"k"), Some(2));
}

#[test]
fn get_nonexistent_returns_none() {
    let cache: ArcCache<&str, i32> = ArcCache::new();
    assert_eq!(cache.get(&"missing"), None);
}

#[test]
fn remove_existing() {
    let cache = ArcCache::new();
    cache.put("a", 10);
    assert_eq!(cache.remove(&"a"), Some(10));
    assert!(!cache.contains_key(&"a"));
    assert!(cache.is_empty());
}

#[test]
fn remove_nonexistent() {
    let cache: ArcCache<&str, i32> = ArcCache::new();
    assert_eq!(cache.remove(&"x"), None);
}

#[test]
fn clear_removes_all() {
    let cache = ArcCache::new();
    cache.put("a", 1);
    cache.put("b", 2);
    cache.clear();
    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
}

// ── BoundedArcCache ─────────────────────────────────────────────────

#[test]
fn bounded_respects_capacity() {
    let cache = BoundedArcCache::new(2);
    cache.put("a", 1);
    cache.put("b", 2);
    assert_eq!(cache.len(), 2);
    cache.put("c", 3);
    assert!(cache.len() <= 2);
    assert_eq!(cache.capacity(), 2);
}

#[test]
fn bounded_get_and_contains() {
    let cache = BoundedArcCache::new(10);
    cache.put("x", 42);
    assert_eq!(cache.get(&"x"), Some(42));
    assert!(cache.contains_key(&"x"));
    assert!(!cache.contains_key(&"y"));
}

#[test]
fn bounded_clear() {
    let cache = BoundedArcCache::new(10);
    cache.put("a", 1);
    cache.clear();
    assert!(cache.is_empty());
}

// ── Concurrency ─────────────────────────────────────────────────────

#[test]
fn concurrent_reads_and_writes() {
    let cache = Arc::new(ArcCache::new());
    let mut handles = Vec::new();

    // Writers
    for i in 0..10 {
        let c = Arc::clone(&cache);
        handles.push(thread::spawn(move || {
            for j in 0..50 {
                c.put(i * 100 + j, j);
            }
        }));
    }

    // Readers
    for _ in 0..5 {
        let c = Arc::clone(&cache);
        handles.push(thread::spawn(move || {
            for k in 0..500 {
                let _ = c.get(&k);
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
}

#[test]
fn concurrent_bounded_cache() {
    let cache = Arc::new(BoundedArcCache::new(50));
    let mut handles = Vec::new();

    for i in 0..8 {
        let c = Arc::clone(&cache);
        handles.push(thread::spawn(move || {
            for j in 0..20 {
                c.put(i * 100 + j, j);
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
    assert!(cache.len() <= 50);
}

// ── Property tests ──────────────────────────────────────────────────

proptest! {
    #[test]
    fn put_get_roundtrip(key in 0u32..200, val in 0i64..10_000) {
        let cache = ArcCache::new();
        cache.put(key, val);
        prop_assert_eq!(cache.get(&key), Some(val));
    }

    #[test]
    fn len_tracks_unique_keys(entries in prop::collection::hash_map(0u32..50, 0i32..100, 1..30)) {
        let cache = ArcCache::new();
        for (&k, &v) in &entries {
            cache.put(k, v);
        }
        prop_assert_eq!(cache.len(), entries.len());
    }

    #[test]
    fn bounded_never_exceeds_capacity(
        cap in 1usize..20,
        entries in prop::collection::vec((0u32..100, 0i32..100), 1..50)
    ) {
        let cache = BoundedArcCache::new(cap);
        for (k, v) in &entries {
            cache.put(*k, *v);
        }
        prop_assert!(cache.len() <= cap);
    }
}
