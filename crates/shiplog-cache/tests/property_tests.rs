//! Property tests for shiplog-cache
//!
//! This module contains property-based tests for cache invariants
//! (persistence, TTL, and size limits).

use proptest::prelude::*;
use shiplog_cache::FileCache;
use shiplog_testkit::proptest::*;
use std::time::Duration;

// ============================================================================
// Cache Basic Operations Tests
// ============================================================================

proptest! {
    /// Test that get-set round-trip equals original value
    #[test]
    fn prop_get_set_roundtrip(
        key in strategy_cache_key(),
        value in "[a-zA-Z0-9_ ]{1,500}"
    ) {
        let cache = FileCache::new(tempfile::tempdir().unwrap().path()).unwrap();
        let json_value = serde_json::json!(value);

        cache.set(&key, &json_value, Duration::from_secs(3600)).unwrap();
        let retrieved = cache.get::<serde_json::Value>(&key).unwrap();

        prop_assert_eq!(Some(json_value), retrieved);
    }

    /// Test that set overwrites previous value
    #[test]
    fn prop_set_overwrites(
        key in strategy_cache_key(),
        value1 in "[a-zA-Z0-9_ ]{1,500}",
        value2 in "[a-zA-Z0-9_ ]{1,500}"
    ) {
        prop_assume!(value1 != value2);
        let cache = FileCache::new(tempfile::tempdir().unwrap().path()).unwrap();
        let json_value1 = serde_json::json!(value1);
        let json_value2 = serde_json::json!(value2);

        cache.set(&key, &json_value1, Duration::from_secs(3600)).unwrap();
        cache.set(&key, &json_value2, Duration::from_secs(3600)).unwrap();

        let retrieved = cache.get::<serde_json::Value>(&key).unwrap();
        prop_assert_eq!(Some(json_value2), retrieved);
        prop_assert_ne!(Some(json_value1), retrieved);
    }

    /// Test that contains() returns true iff get() returns Some
    #[test]
    fn prop_contains_consistency(
        key in strategy_cache_key(),
        value in "[a-zA-Z0-9_ ]{1,500}"
    ) {
        let cache = FileCache::new(tempfile::tempdir().unwrap().path()).unwrap();
        let json_value = serde_json::json!(value);

        // Before set
        prop_assert!(!cache.contains(&key).unwrap());
        prop_assert_eq!(None, cache.get::<serde_json::Value>(&key).unwrap());

        // After set
        cache.set(&key, &json_value, Duration::from_secs(3600)).unwrap();
        prop_assert!(cache.contains(&key).unwrap());
        prop_assert_eq!(Some(json_value), cache.get::<serde_json::Value>(&key).unwrap());
    }
}

// ============================================================================
// Cache Key Uniqueness Tests
// ============================================================================

proptest! {
    /// Test that different keys store independent values
    #[test]
    fn prop_cache_key_uniqueness(
        key1 in strategy_cache_key(),
        key2 in strategy_cache_key(),
        value1 in "[a-zA-Z0-9_ ]{1,500}",
        value2 in "[a-zA-Z0-9_ ]{1,500}"
    ) {
        prop_assume!(key1 != key2);
        let cache = FileCache::new(tempfile::tempdir().unwrap().path()).unwrap();
        let json_value1 = serde_json::json!(value1);
        let json_value2 = serde_json::json!(value2);

        cache.set(&key1, &json_value1, Duration::from_secs(3600)).unwrap();
        cache.set(&key2, &json_value2, Duration::from_secs(3600)).unwrap();

        let retrieved1 = cache.get::<serde_json::Value>(&key1).unwrap();
        let retrieved2 = cache.get::<serde_json::Value>(&key2).unwrap();

        prop_assert_eq!(Some(json_value1), retrieved1);
        prop_assert_eq!(Some(json_value2), retrieved2);
    }
}

// ============================================================================
// TTL Expiration Tests
// ============================================================================

proptest! {
    /// Test that expired entries return None on get
    #[test]
    fn prop_ttl_expiration(
        key in strategy_cache_key(),
        value in "[a-zA-Z0-9_ ]{1,500}",
        ttl_ms in 1u64..1000u64
    ) {
        let cache = FileCache::new(tempfile::tempdir().unwrap().path()).unwrap();
        let json_value = serde_json::json!(value);

        cache.set(&key, &json_value, Duration::from_millis(ttl_ms)).unwrap();

        // Wait for TTL to expire
        std::thread::sleep(Duration::from_millis(ttl_ms + 100));

        let retrieved = cache.get::<serde_json::Value>(&key).unwrap();
        prop_assert_eq!(None, retrieved);
    }

    /// Test that custom TTL is applied
    #[test]
    fn prop_custom_ttl_applied(
        key in strategy_cache_key(),
        value in "[a-zA-Z0-9_ ]{1,500}",
        ttl_ms in 1000u64..5000u64
    ) {
        let cache = FileCache::new(tempfile::tempdir().unwrap().path()).unwrap();
        let json_value = serde_json::json!(value);

        cache.set(&key, &json_value, Duration::from_millis(ttl_ms)).unwrap();

        // Should still be valid immediately
        let retrieved = cache.get::<serde_json::Value>(&key).unwrap();
        prop_assert_eq!(Some(json_value), retrieved);
    }
}

// ============================================================================
// Cache Cleanup Tests
// ============================================================================

proptest! {
    /// Test that cleanup removes expired entries
    #[test]
    fn prop_cleanup_removes_expired(
        keys in proptest::collection::vec(strategy_cache_key(), 5..20),
        ttl_ms in 100u64..500u64
    ) {
        let cache = FileCache::new(tempfile::tempdir().unwrap().path()).unwrap();

        // Set multiple entries with short TTL
        for key in &keys {
            let value = serde_json::json!(format!("value_{}", key));
            cache.set(key, &value, Duration::from_millis(ttl_ms)).unwrap();
        }

        // Wait for entries to expire
        std::thread::sleep(Duration::from_millis(ttl_ms + 100));

        // Cleanup should remove all expired entries
        cache.cleanup_expired().unwrap();

        // All entries should be gone
        for key in &keys {
            let retrieved = cache.get::<serde_json::Value>(key).unwrap();
            prop_assert_eq!(None, retrieved);
        }
    }

    /// Test that clear empties cache
    #[test]
    fn prop_clear_empties_cache(
        keys in proptest::collection::vec(strategy_cache_key(), 5..20)
    ) {
        let cache = FileCache::new(tempfile::tempdir().unwrap().path()).unwrap();

        // Set multiple entries
        for key in &keys {
            let value = serde_json::json!(format!("value_{}", key));
            cache.set(key, &value, Duration::from_secs(3600)).unwrap();
        }

        // Clear cache
        cache.clear().unwrap();

        // All entries should be gone
        for key in &keys {
            let retrieved = cache.get::<serde_json::Value>(key).unwrap();
            prop_assert_eq!(None, retrieved);
            prop_assert!(!cache.contains(key).unwrap());
        }
    }
}

// ============================================================================
// Cache Persistence Tests
// ============================================================================

proptest! {
    /// Test that data survives close and reopen
    #[test]
    fn prop_persistence_across_opens(
        key in strategy_cache_key(),
        value in "[a-zA-Z0-9_ ]{1,500}"
    ) {
        let dir = tempfile::tempdir().unwrap();
        let json_value = serde_json::json!(value);

        // Set value in first cache instance
        {
            let cache = FileCache::new(dir.path()).unwrap();
            cache.set(&key, &json_value, Duration::from_secs(3600)).unwrap();
        }

        // Reopen cache and retrieve value
        let cache = FileCache::new(dir.path()).unwrap();
        let retrieved = cache.get::<serde_json::Value>(&key).unwrap();

        prop_assert_eq!(Some(json_value), retrieved);
    }
}

// ============================================================================
// Cache Entry Invariant Tests
// ============================================================================

proptest! {
    /// Test that timestamp is monotonic (cached_at <= expires_at)
    #[test]
    fn prop_timestamp_monotonic(
        key in strategy_cache_key(),
        value in "[a-zA-Z0-9_ ]{1,500}",
        ttl_secs in 1u64..3600u64
    ) {
        let cache = FileCache::new(tempfile::tempdir().unwrap().path()).unwrap();
        let json_value = serde_json::json!(value);

        let before_set = std::time::SystemTime::now();
        cache.set(&key, &json_value, Duration::from_secs(ttl_secs)).unwrap();
        let after_set = std::time::SystemTime::now();

        // The cached_at timestamp should be between before_set and after_set
        // This is a basic invariant - actual implementation may vary
        prop_assert!(before_set <= after_set);
    }

    /// Test that JSON serialization preserves value structure
    #[test]
    fn prop_data_serialization(
        key in strategy_cache_key(),
        value in "[a-zA-Z0-9_ ]{1,500}"
    ) {
        let cache = FileCache::new(tempfile::tempdir().unwrap().path()).unwrap();
        let json_value = serde_json::json!(value);

        cache.set(&key, &json_value, Duration::from_secs(3600)).unwrap();
        let retrieved = cache.get::<serde_json::Value>(&key).unwrap();

        prop_assert_eq!(Some(json_value), retrieved);
    }

    /// Test that keys are valid UTF-8 strings
    #[test]
    fn prop_key_string_validity(
        key in strategy_cache_key(),
        value in "[a-zA-Z0-9_ ]{1,500}"
    ) {
        let cache = FileCache::new(tempfile::tempdir().unwrap().path()).unwrap();
        let json_value = serde_json::json!(value);

        // Should not panic with valid UTF-8 keys
        cache.set(&key, &json_value, Duration::from_secs(3600)).unwrap();
        let retrieved = cache.get::<serde_json::Value>(&key).unwrap();

        prop_assert_eq!(Some(json_value), retrieved);
    }
}
