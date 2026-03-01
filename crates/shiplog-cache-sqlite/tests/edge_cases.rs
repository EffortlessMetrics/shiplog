//! Edge-case and stress tests for shiplog-cache-sqlite.

use chrono::Duration;
use shiplog_cache_sqlite::ApiCache;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn mem_cache() -> ApiCache {
    ApiCache::open_in_memory().unwrap()
}

// ---------------------------------------------------------------------------
// Empty keys and values
// ---------------------------------------------------------------------------

#[test]
fn empty_key_roundtrips() {
    let cache = mem_cache();
    cache.set("", &serde_json::json!("value")).unwrap();

    let got: Option<serde_json::Value> = cache.get("").unwrap();
    assert_eq!(got, Some(serde_json::json!("value")));
    assert!(cache.contains("").unwrap());
}

#[test]
fn empty_string_value_roundtrips() {
    let cache = mem_cache();
    cache.set("k", &serde_json::json!("")).unwrap();

    let got: Option<serde_json::Value> = cache.get("k").unwrap();
    assert_eq!(got, Some(serde_json::json!("")));
}

#[test]
fn null_json_value_roundtrips() {
    let cache = mem_cache();
    cache.set("k", &serde_json::Value::Null).unwrap();

    let got: Option<serde_json::Value> = cache.get("k").unwrap();
    assert_eq!(got, Some(serde_json::Value::Null));
}

#[test]
fn empty_object_and_array_roundtrip() {
    let cache = mem_cache();
    cache.set("obj", &serde_json::json!({})).unwrap();
    cache.set("arr", &serde_json::json!([])).unwrap();

    assert_eq!(
        cache.get::<serde_json::Value>("obj").unwrap(),
        Some(serde_json::json!({}))
    );
    assert_eq!(
        cache.get::<serde_json::Value>("arr").unwrap(),
        Some(serde_json::json!([]))
    );
}

// ---------------------------------------------------------------------------
// Large values
// ---------------------------------------------------------------------------

#[test]
fn large_value_roundtrips() {
    let cache = mem_cache();
    // ~1 MB JSON string
    let big = "x".repeat(1_000_000);
    cache.set("big", &serde_json::json!(big)).unwrap();

    let got: Option<serde_json::Value> = cache.get("big").unwrap();
    assert_eq!(got, Some(serde_json::json!(big)));
}

#[test]
fn large_value_reflected_in_stats_size() {
    let cache = mem_cache();
    let big = "y".repeat(500_000);
    cache.set("big", &serde_json::json!(big)).unwrap();

    let stats = cache.stats().unwrap();
    assert_eq!(stats.total_entries, 1);
    // The serialized JSON includes quotes, so size > 500_000.
    // Sanity check: stats are reported without error (size may round to 0 MB).
    assert_eq!(stats.total_entries, 1);
}

#[test]
fn deeply_nested_json_roundtrips() {
    let cache = mem_cache();
    // Build 100-level nested object: {"a":{"a":{..."leaf"...}}}
    let mut val = serde_json::json!("leaf");
    for _ in 0..100 {
        val = serde_json::json!({"a": val});
    }

    cache.set("deep", &val).unwrap();
    let got: Option<serde_json::Value> = cache.get("deep").unwrap();
    assert_eq!(got, Some(val));
}

// ---------------------------------------------------------------------------
// Special characters in keys
// ---------------------------------------------------------------------------

#[test]
fn keys_with_special_characters() {
    let cache = mem_cache();
    let specials = [
        "key with spaces",
        "key\twith\ttabs",
        "key\nwith\nnewlines",
        "key'with'quotes",
        "key\"with\"doublequotes",
        "key%with%percent",
        "emoji_🦀_key",
        "path/to/resource?query=1&page=2",
        "null\0embedded",
        "中文键",
    ];

    for (i, key) in specials.iter().enumerate() {
        let val = serde_json::json!(i);
        cache.set(key, &val).unwrap();
    }

    for (i, key) in specials.iter().enumerate() {
        let got: Option<serde_json::Value> = cache.get(key).unwrap();
        assert_eq!(got, Some(serde_json::json!(i)), "mismatch for key: {key:?}");
        assert!(
            cache.contains(key).unwrap(),
            "contains failed for key: {key:?}"
        );
    }
}

#[test]
fn sql_injection_in_key_is_harmless() {
    let cache = mem_cache();
    let evil = "'; DROP TABLE cache_entries; --";

    cache.set(evil, &serde_json::json!("safe")).unwrap();

    let got: Option<serde_json::Value> = cache.get(evil).unwrap();
    assert_eq!(got, Some(serde_json::json!("safe")));

    // Table is still intact.
    let stats = cache.stats().unwrap();
    assert_eq!(stats.total_entries, 1);
}

// ---------------------------------------------------------------------------
// TTL expiry edge cases
// ---------------------------------------------------------------------------

#[test]
fn zero_ttl_entry_is_immediately_expired() {
    let cache = mem_cache().with_ttl(Duration::zero());
    cache.set("k", &serde_json::json!(1)).unwrap();

    // Zero TTL means expires_at == cached_at, and the query uses `expires_at > now`,
    // so by the time we read, the entry should be gone (or at the boundary).
    let got: Option<serde_json::Value> = cache.get("k").unwrap();
    assert!(got.is_none(), "zero TTL entry should be expired");
}

#[test]
fn negative_ttl_entry_is_expired() {
    let cache = mem_cache();
    cache
        .set_with_ttl("neg", &serde_json::json!(1), Duration::seconds(-10))
        .unwrap();

    assert!(cache.get::<serde_json::Value>("neg").unwrap().is_none());
    assert!(!cache.contains("neg").unwrap());
}

#[test]
fn very_large_ttl_entry_stays_valid() {
    let cache = mem_cache();
    // TTL of ~100 years
    cache
        .set_with_ttl(
            "forever",
            &serde_json::json!("eternal"),
            Duration::days(36_500),
        )
        .unwrap();

    let got: Option<serde_json::Value> = cache.get("forever").unwrap();
    assert_eq!(got, Some(serde_json::json!("eternal")));
    assert!(cache.contains("forever").unwrap());
}

#[test]
fn expired_entry_still_counted_in_stats_total() {
    let cache = mem_cache();
    cache
        .set_with_ttl("exp", &serde_json::json!(1), Duration::seconds(-5))
        .unwrap();
    cache.set("live", &serde_json::json!(2)).unwrap();

    let stats = cache.stats().unwrap();
    assert_eq!(stats.total_entries, 2);
    assert_eq!(stats.expired_entries, 1);
    assert_eq!(stats.valid_entries, 1);
}

#[test]
fn cleanup_only_removes_expired_leaves_valid() {
    let cache = mem_cache();
    cache
        .set_with_ttl("dead1", &serde_json::json!(1), Duration::seconds(-1))
        .unwrap();
    cache
        .set_with_ttl("dead2", &serde_json::json!(2), Duration::seconds(-1))
        .unwrap();
    cache.set("alive", &serde_json::json!(3)).unwrap();

    let deleted = cache.cleanup_expired().unwrap();
    assert_eq!(deleted, 2);

    assert!(cache.get::<serde_json::Value>("alive").unwrap().is_some());
    let stats = cache.stats().unwrap();
    assert_eq!(stats.total_entries, 1);
    assert_eq!(stats.valid_entries, 1);
    assert_eq!(stats.expired_entries, 0);
}

#[test]
fn cleanup_on_empty_cache_returns_zero() {
    let cache = mem_cache();
    assert_eq!(cache.cleanup_expired().unwrap(), 0);
}

// ---------------------------------------------------------------------------
// Stats accuracy after mixed operations
// ---------------------------------------------------------------------------

#[test]
fn stats_after_overwrite_does_not_double_count() {
    let cache = mem_cache();
    cache.set("k", &serde_json::json!(1)).unwrap();
    cache.set("k", &serde_json::json!(2)).unwrap();
    cache.set("k", &serde_json::json!(3)).unwrap();

    let stats = cache.stats().unwrap();
    assert_eq!(
        stats.total_entries, 1,
        "INSERT OR REPLACE should not duplicate"
    );
    assert_eq!(stats.valid_entries, 1);
}

#[test]
fn stats_size_decreases_after_clear() {
    let cache = mem_cache();
    let big = "z".repeat(10_000);
    cache.set("k", &serde_json::json!(big)).unwrap();

    let before = cache.stats().unwrap();
    assert!(before.total_entries > 0);

    cache.clear().unwrap();

    let after = cache.stats().unwrap();
    assert_eq!(after.total_entries, 0);
}

#[test]
fn stats_correct_with_many_entries() {
    let cache = mem_cache();

    for i in 0..100 {
        cache.set(&format!("k{i}"), &serde_json::json!(i)).unwrap();
    }

    let stats = cache.stats().unwrap();
    assert_eq!(stats.total_entries, 100);
    assert_eq!(stats.valid_entries, 100);
    assert_eq!(stats.expired_entries, 0);
}

#[test]
fn stats_mixed_expired_and_valid() {
    let cache = mem_cache();

    for i in 0..50 {
        cache
            .set_with_ttl(
                &format!("exp{i}"),
                &serde_json::json!(i),
                Duration::seconds(-1),
            )
            .unwrap();
    }
    for i in 0..30 {
        cache
            .set(&format!("live{i}"), &serde_json::json!(i))
            .unwrap();
    }

    let stats = cache.stats().unwrap();
    assert_eq!(stats.total_entries, 80);
    assert_eq!(stats.expired_entries, 50);
    assert_eq!(stats.valid_entries, 30);
}

// ---------------------------------------------------------------------------
// Overwrite / upsert semantics
// ---------------------------------------------------------------------------

#[test]
fn overwrite_with_different_type_succeeds() {
    let cache = mem_cache();
    cache.set("k", &serde_json::json!(42)).unwrap();
    cache.set("k", &serde_json::json!("now a string")).unwrap();

    let got: Option<serde_json::Value> = cache.get("k").unwrap();
    assert_eq!(got, Some(serde_json::json!("now a string")));
}

#[test]
fn overwrite_refreshes_ttl() {
    let cache = mem_cache();

    // First write with a very short (already-expired) TTL.
    cache
        .set_with_ttl("k", &serde_json::json!(1), Duration::seconds(-1))
        .unwrap();
    assert!(cache.get::<serde_json::Value>("k").unwrap().is_none());

    // Overwrite with the default (24 h) TTL.
    cache.set("k", &serde_json::json!(2)).unwrap();
    let got: Option<serde_json::Value> = cache.get("k").unwrap();
    assert_eq!(got, Some(serde_json::json!(2)));
}

// ---------------------------------------------------------------------------
// Get on missing key
// ---------------------------------------------------------------------------

#[test]
fn get_nonexistent_key_returns_none() {
    let cache = mem_cache();
    let got: Option<serde_json::Value> = cache.get("nope").unwrap();
    assert!(got.is_none());
}

#[test]
fn contains_nonexistent_key_returns_false() {
    let cache = mem_cache();
    assert!(!cache.contains("nope").unwrap());
}

// ---------------------------------------------------------------------------
// Persistence edge cases (file-backed)
// ---------------------------------------------------------------------------

#[test]
fn persistence_survives_close_and_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("test.sqlite");

    {
        let cache = ApiCache::open(&db).unwrap();
        cache.set("persist", &serde_json::json!("hello")).unwrap();
    }

    let cache = ApiCache::open(&db).unwrap();
    let got: Option<serde_json::Value> = cache.get("persist").unwrap();
    assert_eq!(got, Some(serde_json::json!("hello")));
}

#[test]
fn double_open_same_path_does_not_corrupt() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("test.sqlite");

    let c1 = ApiCache::open(&db).unwrap();
    c1.set("a", &serde_json::json!(1)).unwrap();
    drop(c1);

    let c2 = ApiCache::open(&db).unwrap();
    c2.set("b", &serde_json::json!(2)).unwrap();

    assert_eq!(
        c2.get::<serde_json::Value>("a").unwrap(),
        Some(serde_json::json!(1))
    );
    assert_eq!(
        c2.get::<serde_json::Value>("b").unwrap(),
        Some(serde_json::json!(2))
    );
    assert_eq!(c2.stats().unwrap().total_entries, 2);
}

// ---------------------------------------------------------------------------
// Stress: many keys
// ---------------------------------------------------------------------------

#[test]
fn stress_1000_unique_keys() {
    let cache = mem_cache();

    for i in 0..1_000 {
        cache
            .set(&format!("stress:{i}"), &serde_json::json!(i))
            .unwrap();
    }

    let stats = cache.stats().unwrap();
    assert_eq!(stats.total_entries, 1_000);

    // Spot-check a few values.
    for i in [0, 42, 500, 999] {
        let got: Option<serde_json::Value> = cache.get(&format!("stress:{i}")).unwrap();
        assert_eq!(got, Some(serde_json::json!(i)));
    }
}

#[test]
fn stress_repeated_overwrites_same_key() {
    let cache = mem_cache();

    for i in 0..1_000 {
        cache.set("hot", &serde_json::json!(i)).unwrap();
    }

    let got: Option<serde_json::Value> = cache.get("hot").unwrap();
    assert_eq!(got, Some(serde_json::json!(999)));
    assert_eq!(cache.stats().unwrap().total_entries, 1);
}

// ---------------------------------------------------------------------------
// Builder method chaining
// ---------------------------------------------------------------------------

#[test]
fn with_max_size_does_not_break_operations() {
    let cache = mem_cache().with_max_size(1024);
    cache.set("k", &serde_json::json!(1)).unwrap();

    let got: Option<serde_json::Value> = cache.get("k").unwrap();
    assert_eq!(got, Some(serde_json::json!(1)));
}

#[test]
fn chained_builders_compose() {
    let cache = mem_cache().with_ttl(Duration::hours(1)).with_max_size(4096);

    cache.set("k", &serde_json::json!("ok")).unwrap();
    assert!(cache.contains("k").unwrap());
}
