//! Integration tests for the shiplog-cache facade crate.

use chrono::Duration;
use shiplog_cache::{ApiCache, CacheKey, CacheStats};

// ------------------------------------------------------------------
// Re-export smoke tests
// ------------------------------------------------------------------

#[test]
fn reexported_cache_key_builds_search_key() {
    let key = CacheKey::search("is:pr author:alice", 1, 100);
    assert!(key.starts_with("search:"));
    assert!(key.ends_with(":page1:per100"));
}

#[test]
fn reexported_cache_key_builds_pr_details() {
    let url = "https://api.github.com/repos/o/r/pulls/1";
    let key = CacheKey::pr_details(url);
    assert_eq!(key, format!("pr:details:{url}"));
}

#[test]
fn reexported_cache_key_builds_pr_reviews() {
    let url = "https://api.github.com/repos/o/r/pulls/5";
    let key = CacheKey::pr_reviews(url, 3);
    assert_eq!(key, format!("pr:reviews:{url}:page3"));
}

#[test]
fn reexported_cache_key_builds_mr_notes() {
    let key = CacheKey::mr_notes(99, 7, 2);
    assert_eq!(key, "gitlab:mr:notes:project99:mr7:page2");
}

#[test]
fn reexported_cache_stats_normalizes_raw_counts() {
    let stats = CacheStats::from_raw_counts(10, 3, 5 * 1024 * 1024);
    assert_eq!(stats.total_entries, 10);
    assert_eq!(stats.expired_entries, 3);
    assert_eq!(stats.valid_entries, 7);
    assert_eq!(stats.cache_size_mb, 5);
}

#[test]
fn reexported_cache_stats_clamps_negative_values() {
    let stats = CacheStats::from_raw_counts(-5, -1, -100);
    assert_eq!(stats.total_entries, 0);
    assert_eq!(stats.expired_entries, 0);
    assert_eq!(stats.valid_entries, 0);
    assert_eq!(stats.cache_size_mb, 0);
}

// ------------------------------------------------------------------
// ApiCache integration tests
// ------------------------------------------------------------------

#[test]
fn api_cache_set_get_round_trip() {
    let cache = ApiCache::open_in_memory().unwrap();
    cache.set("k1", &serde_json::json!({"a": 1})).unwrap();
    let val: Option<serde_json::Value> = cache.get("k1").unwrap();
    assert_eq!(val.unwrap()["a"], 1);
}

#[test]
fn api_cache_missing_key_returns_none() {
    let cache = ApiCache::open_in_memory().unwrap();
    let val: Option<serde_json::Value> = cache.get("nonexistent").unwrap();
    assert!(val.is_none());
}

#[test]
fn api_cache_overwrite_replaces_value() {
    let cache = ApiCache::open_in_memory().unwrap();
    cache.set("k", &"first").unwrap();
    cache.set("k", &"second").unwrap();
    let val: Option<String> = cache.get("k").unwrap();
    assert_eq!(val.unwrap(), "second");
}

#[test]
fn api_cache_contains_reflects_presence() {
    let cache = ApiCache::open_in_memory().unwrap();
    assert!(!cache.contains("k").unwrap());
    cache.set("k", &42u32).unwrap();
    assert!(cache.contains("k").unwrap());
}

#[test]
fn api_cache_clear_removes_all_entries() {
    let cache = ApiCache::open_in_memory().unwrap();
    cache.set("a", &1u32).unwrap();
    cache.set("b", &2u32).unwrap();
    cache.clear().unwrap();
    let stats = cache.stats().unwrap();
    assert!(stats.is_empty());
}

#[test]
fn api_cache_negative_ttl_creates_already_expired_entry() {
    let cache = ApiCache::open_in_memory().unwrap();
    cache
        .set_with_ttl("k", &"val", Duration::seconds(-10))
        .unwrap();
    let val: Option<String> = cache.get("k").unwrap();
    assert!(val.is_none(), "entry with negative TTL should be expired");
    assert!(!cache.contains("k").unwrap());
}

#[test]
fn api_cache_cleanup_expired_returns_count() {
    let cache = ApiCache::open_in_memory().unwrap();
    cache
        .set_with_ttl("expired1", &1u32, Duration::seconds(-1))
        .unwrap();
    cache
        .set_with_ttl("expired2", &2u32, Duration::seconds(-1))
        .unwrap();
    cache.set("valid", &3u32).unwrap();

    let deleted = cache.cleanup_expired().unwrap();
    assert_eq!(deleted, 2);

    let stats = cache.stats().unwrap();
    assert_eq!(stats.total_entries, 1);
    assert_eq!(stats.valid_entries, 1);
}

#[test]
fn api_cache_stats_tracks_expired_entries() {
    let cache = ApiCache::open_in_memory().unwrap();
    cache
        .set_with_ttl("old", &"x", Duration::seconds(-1))
        .unwrap();
    cache.set("fresh", &"y").unwrap();

    let stats = cache.stats().unwrap();
    assert_eq!(stats.total_entries, 2);
    assert_eq!(stats.expired_entries, 1);
    assert_eq!(stats.valid_entries, 1);
}

#[test]
fn api_cache_with_ttl_builder_is_chainable() {
    let cache = ApiCache::open_in_memory()
        .unwrap()
        .with_ttl(Duration::hours(48))
        .with_max_size(10 * 1024 * 1024);

    cache.set("k", &"v").unwrap();
    let val: Option<String> = cache.get("k").unwrap();
    assert_eq!(val.unwrap(), "v");
}

#[test]
fn api_cache_open_on_disk() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.db");

    {
        let cache = ApiCache::open(&path).unwrap();
        cache.set("k", &"persisted").unwrap();
    }

    {
        let cache = ApiCache::open(&path).unwrap();
        let val: Option<String> = cache.get("k").unwrap();
        assert_eq!(val.unwrap(), "persisted");
    }
}

// ------------------------------------------------------------------
// Edge cases
// ------------------------------------------------------------------

#[test]
fn api_cache_empty_string_key() {
    let cache = ApiCache::open_in_memory().unwrap();
    cache.set("", &"empty-key").unwrap();
    let val: Option<String> = cache.get("").unwrap();
    assert_eq!(val.unwrap(), "empty-key");
}

#[test]
fn api_cache_unicode_key_and_value() {
    let cache = ApiCache::open_in_memory().unwrap();
    let key = "日本語キー";
    let value = "こんにちは世界 🌍";
    cache.set(key, &value).unwrap();
    let val: Option<String> = cache.get(key).unwrap();
    assert_eq!(val.unwrap(), value);
}

#[test]
fn api_cache_large_value() {
    let cache = ApiCache::open_in_memory().unwrap();
    let big = "x".repeat(1_000_000);
    cache.set("big", &big).unwrap();
    let val: Option<String> = cache.get("big").unwrap();
    assert_eq!(val.unwrap().len(), 1_000_000);
}

#[test]
fn api_cache_stats_size_reflects_stored_data() {
    let cache = ApiCache::open_in_memory().unwrap();
    cache.set("k", &"some data payload").unwrap();
    let stats = cache.stats().unwrap();
    assert!(stats.cache_size_mb == 0, "small data rounds down to 0 MB");
    assert_eq!(stats.total_entries, 1);
}

// ------------------------------------------------------------------
// Snapshot tests for stats
// ------------------------------------------------------------------

#[test]
fn snapshot_cache_stats_from_reexport() {
    let stats = CacheStats::from_raw_counts(15, 5, 10 * 1024 * 1024 + 512);
    insta::assert_snapshot!("reexported_stats", format!("{stats:?}"));
}

// ------------------------------------------------------------------
// Property tests
// ------------------------------------------------------------------

mod prop {
    use proptest::prelude::*;
    use shiplog_cache::CacheStats;

    proptest! {
        #[test]
        fn prop_facade_stats_valid_plus_expired_eq_total(
            total in 0i64..10_000,
            expired in 0i64..10_000,
            bytes in 0i64..i64::MAX / 2,
        ) {
            let stats = CacheStats::from_raw_counts(total, expired, bytes);
            prop_assert_eq!(stats.valid_entries + stats.expired_entries, stats.total_entries);
        }

        #[test]
        fn prop_facade_stats_expired_le_total(
            total in any::<i64>(),
            expired in any::<i64>(),
            bytes in any::<i64>(),
        ) {
            let stats = CacheStats::from_raw_counts(total, expired, bytes);
            prop_assert!(stats.expired_entries <= stats.total_entries);
        }
    }
}
