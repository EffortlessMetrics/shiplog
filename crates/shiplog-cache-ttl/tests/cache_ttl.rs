use proptest::prelude::*;
use shiplog_cache_ttl::TtlCache;

// ── Basics ──────────────────────────────────────────────────────────

#[test]
fn new_cache_is_empty() {
    let cache: TtlCache<String, i32> = TtlCache::new(3600);
    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
    assert_eq!(cache.total_entries(), 0);
}

#[test]
fn with_default_ttl_creates_cache() {
    let cache: TtlCache<String, i32> = TtlCache::with_default_ttl();
    assert!(cache.is_empty());
}

#[test]
fn default_creates_cache() {
    let cache: TtlCache<String, i32> = TtlCache::default();
    assert!(cache.is_empty());
}

#[test]
fn put_and_get() {
    let mut cache = TtlCache::new(3600);
    cache.put("a", 1);
    assert_eq!(cache.get(&"a"), Some(&1));
    assert!(cache.contains_key(&"a"));
}

#[test]
fn get_nonexistent_returns_none() {
    let cache: TtlCache<&str, i32> = TtlCache::new(3600);
    assert_eq!(cache.get(&"x"), None);
    assert!(!cache.contains_key(&"x"));
}

#[test]
fn put_overwrites_value() {
    let mut cache = TtlCache::new(3600);
    cache.put("k", 1);
    cache.put("k", 2);
    assert_eq!(cache.get(&"k"), Some(&2));
    assert_eq!(cache.len(), 1);
}

// ── Expiration ──────────────────────────────────────────────────────

#[test]
fn expired_entry_not_visible() {
    let mut cache = TtlCache::new(-1); // already expired
    cache.put("k", 42);
    assert_eq!(cache.get(&"k"), None);
    assert!(!cache.contains_key(&"k"));
    assert_eq!(cache.len(), 0);
    assert_eq!(cache.total_entries(), 1);
}

#[test]
fn custom_ttl_expires_independently() {
    let mut cache = TtlCache::new(3600);
    cache.put_with_ttl("short", 1, -1); // already expired
    cache.put_with_ttl("long", 2, 3600);
    assert_eq!(cache.get(&"short"), None);
    assert_eq!(cache.get(&"long"), Some(&2));
}

#[test]
fn entry_expires_after_sleep() {
    let mut cache = TtlCache::new(1); // 1 second
    cache.put("k", 99);
    assert_eq!(cache.get(&"k"), Some(&99));
    std::thread::sleep(std::time::Duration::from_millis(1100));
    assert_eq!(cache.get(&"k"), None);
}

// ── Remove ──────────────────────────────────────────────────────────

#[test]
fn remove_returns_value() {
    let mut cache = TtlCache::new(3600);
    cache.put("a", 10);
    assert_eq!(cache.remove(&"a"), Some(10));
    assert!(!cache.contains_key(&"a"));
}

#[test]
fn remove_nonexistent_returns_none() {
    let mut cache: TtlCache<&str, i32> = TtlCache::new(3600);
    assert_eq!(cache.remove(&"x"), None);
}

#[test]
fn remove_expired_entry_still_returns_value() {
    let mut cache = TtlCache::new(-1);
    cache.put("k", 42);
    // remove works even if expired (it removes from internal map)
    assert_eq!(cache.remove(&"k"), Some(42));
}

// ── Cleanup ─────────────────────────────────────────────────────────

#[test]
fn cleanup_removes_expired() {
    let mut cache = TtlCache::new(3600);
    cache.put("live", 1);
    cache.put_with_ttl("dead", 2, -1);
    let cleaned = cache.cleanup_expired();
    assert_eq!(cleaned, 1);
    assert_eq!(cache.total_entries(), 1);
    assert!(cache.contains_key(&"live"));
}

#[test]
fn cleanup_on_empty() {
    let mut cache: TtlCache<&str, i32> = TtlCache::new(3600);
    assert_eq!(cache.cleanup_expired(), 0);
}

#[test]
fn cleanup_all_expired() {
    let mut cache = TtlCache::new(-1);
    cache.put("a", 1);
    cache.put("b", 2);
    cache.put("c", 3);
    let cleaned = cache.cleanup_expired();
    assert_eq!(cleaned, 3);
    assert_eq!(cache.total_entries(), 0);
}

// ── Clear ───────────────────────────────────────────────────────────

#[test]
fn clear_empties_everything() {
    let mut cache = TtlCache::new(3600);
    cache.put("a", 1);
    cache.put("b", 2);
    cache.clear();
    assert!(cache.is_empty());
    assert_eq!(cache.total_entries(), 0);
}

// ── set_default_ttl ─────────────────────────────────────────────────

#[test]
fn set_default_ttl_affects_new_inserts() {
    let mut cache = TtlCache::new(3600);
    cache.put("old", 1);
    cache.set_default_ttl(-1);
    cache.put("new", 2);
    assert_eq!(cache.get(&"old"), Some(&1));
    assert_eq!(cache.get(&"new"), None); // new entry already expired
}

// ── len vs total_entries ────────────────────────────────────────────

#[test]
fn len_excludes_expired_total_includes() {
    let mut cache = TtlCache::new(3600);
    cache.put("live", 1);
    cache.put_with_ttl("dead", 2, -1);
    assert_eq!(cache.len(), 1);
    assert_eq!(cache.total_entries(), 2);
}

// ── Property tests ──────────────────────────────────────────────────

proptest! {
    #[test]
    fn put_get_roundtrip(key in 0u32..100, val in 0i64..10_000) {
        let mut cache = TtlCache::new(3600);
        cache.put(key, val);
        prop_assert_eq!(cache.get(&key), Some(&val));
    }

    #[test]
    fn len_matches_live_entries(
        entries in prop::collection::hash_map(0u32..50, 0i32..100, 1..30)
    ) {
        let mut cache = TtlCache::new(3600);
        for (&k, &v) in &entries {
            cache.put(k, v);
        }
        prop_assert_eq!(cache.len(), entries.len());
    }

    #[test]
    fn cleanup_all_expired_removes_all(n in 1usize..20) {
        let mut cache = TtlCache::new(-1);
        for i in 0..n {
            cache.put(i, i as i32);
        }
        let cleaned = cache.cleanup_expired();
        prop_assert_eq!(cleaned, n);
        prop_assert_eq!(cache.total_entries(), 0);
    }

    #[test]
    fn remove_after_put_returns_value(key in 0u32..50, val in 0i32..1000) {
        let mut cache = TtlCache::new(3600);
        cache.put(key, val);
        prop_assert_eq!(cache.remove(&key), Some(val));
        prop_assert!(!cache.contains_key(&key));
    }
}
