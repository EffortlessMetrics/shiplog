use chrono::Duration;
use proptest::prelude::*;
use shiplog_ttl::*;

// ── TtlEntry ────────────────────────────────────────────────────────

#[test]
fn entry_with_negative_ttl_is_expired() {
    let entry = TtlEntry::new(42, Duration::seconds(-10));
    assert!(entry.is_expired());
    assert!(entry.remaining_ttl().is_none());
}

#[test]
fn entry_with_large_ttl_is_not_expired() {
    let entry = TtlEntry::new("data", Duration::hours(24));
    assert!(!entry.is_expired());
    assert!(entry.remaining_ttl().is_some());
}

#[test]
fn entry_remaining_ttl_is_positive_for_fresh() {
    let entry = TtlEntry::new((), Duration::seconds(100));
    let remaining = entry.remaining_ttl().unwrap();
    assert!(remaining > Duration::zero());
    assert!(remaining <= Duration::seconds(100));
}

// ── TtlCache basics ────────────────────────────────────────────────

#[test]
fn new_cache_is_empty() {
    let cache: TtlCache<String, i32> = TtlCache::new(Duration::seconds(60));
    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
}

#[test]
fn insert_and_get() {
    let mut cache = TtlCache::new(Duration::hours(1));
    cache.insert("k", 100);
    assert_eq!(cache.get(&"k"), Some(&100));
    assert!(cache.contains(&"k"));
    assert!(!cache.contains(&"other"));
}

#[test]
fn insert_overwrites_value() {
    let mut cache = TtlCache::new(Duration::hours(1));
    cache.insert("k", 1);
    cache.insert("k", 2);
    assert_eq!(cache.get(&"k"), Some(&2));
}

#[test]
fn expired_entry_not_visible() {
    let mut cache = TtlCache::new(Duration::milliseconds(1));
    cache.insert("k", 99);
    std::thread::sleep(std::time::Duration::from_millis(10));
    assert!(cache.get(&"k").is_none());
    assert!(!cache.contains(&"k"));
}

#[test]
fn insert_with_custom_ttl() {
    let mut cache = TtlCache::new(Duration::hours(1));
    cache.insert_with_ttl("short", 1, Duration::milliseconds(1));
    cache.insert_with_ttl("long", 2, Duration::hours(2));
    std::thread::sleep(std::time::Duration::from_millis(10));
    assert!(cache.get(&"short").is_none());
    assert_eq!(cache.get(&"long"), Some(&2));
}

// ── Remove ──────────────────────────────────────────────────────────

#[test]
fn remove_returns_value() {
    let mut cache = TtlCache::new(Duration::hours(1));
    cache.insert("k", 42);
    assert_eq!(cache.remove(&"k"), Some(42));
    assert!(cache.is_empty());
}

#[test]
fn remove_nonexistent_returns_none() {
    let mut cache: TtlCache<&str, i32> = TtlCache::new(Duration::hours(1));
    assert_eq!(cache.remove(&"missing"), None);
}

// ── Cleanup ─────────────────────────────────────────────────────────

#[test]
fn cleanup_removes_only_expired() {
    let mut cache = TtlCache::new(Duration::hours(1));
    cache.insert("live", 1);
    cache.insert_with_ttl("dead", 2, Duration::milliseconds(1));
    std::thread::sleep(std::time::Duration::from_millis(10));
    let cleaned = cache.cleanup();
    assert_eq!(cleaned, 1);
    assert_eq!(cache.len(), 1);
    assert!(cache.contains(&"live"));
}

#[test]
fn cleanup_on_empty_cache() {
    let mut cache: TtlCache<&str, i32> = TtlCache::new(Duration::hours(1));
    assert_eq!(cache.cleanup(), 0);
}

// ── Clear ───────────────────────────────────────────────────────────

#[test]
fn clear_empties_everything() {
    let mut cache = TtlCache::new(Duration::hours(1));
    cache.insert("a", 1);
    cache.insert("b", 2);
    cache.clear();
    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
}

// ── Helper functions ────────────────────────────────────────────────

#[test]
fn ttl_helper_conversions() {
    assert_eq!(ttl_from_secs(120), Duration::seconds(120));
    assert_eq!(ttl_from_mins(3), Duration::minutes(3));
    assert_eq!(ttl_from_hours(5), Duration::hours(5));
    assert_eq!(ttl_from_days(7), Duration::days(7));
}

#[test]
fn ttl_helper_zero() {
    assert_eq!(ttl_from_secs(0), Duration::zero());
}

// ── Property tests ──────────────────────────────────────────────────

proptest! {
    #[test]
    fn insert_get_roundtrip(key in 0u32..100, val in 0i64..10000) {
        let mut cache = TtlCache::new(Duration::hours(1));
        cache.insert(key, val);
        prop_assert_eq!(cache.get(&key), Some(&val));
    }

    #[test]
    fn len_matches_inserts(entries in prop::collection::hash_map(0u32..50, 0i32..100, 1..30)) {
        let mut cache = TtlCache::new(Duration::hours(1));
        for (&k, &v) in &entries {
            cache.insert(k, v);
        }
        prop_assert_eq!(cache.len(), entries.len());
    }

    #[test]
    fn cleanup_with_all_expired_removes_all(n in 1usize..20) {
        let mut cache = TtlCache::new(Duration::seconds(-1));
        for i in 0..n {
            cache.insert(i, i);
        }
        let cleaned = cache.cleanup();
        prop_assert_eq!(cleaned, n);
        prop_assert!(cache.is_empty());
    }

    #[test]
    fn ttl_secs_identity(secs in 0i64..100_000) {
        prop_assert_eq!(ttl_from_secs(secs), Duration::seconds(secs));
    }
}
