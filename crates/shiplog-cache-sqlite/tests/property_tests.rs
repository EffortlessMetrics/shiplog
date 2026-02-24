//! Property tests for shiplog-cache-sqlite.

use chrono::Duration;
use proptest::prelude::*;
use shiplog_cache_sqlite::ApiCache;
use shiplog_testkit::proptest::*;

proptest! {
    #[test]
    fn prop_get_set_roundtrip(
        key in strategy_cache_key(),
        value in "[a-zA-Z0-9_ ]{1,500}"
    ) {
        let cache = ApiCache::open_in_memory().unwrap();
        let json_value = serde_json::json!(value);

        cache.set(&key, &json_value).unwrap();
        let retrieved = cache.get::<serde_json::Value>(&key).unwrap();

        prop_assert_eq!(Some(json_value), retrieved);
    }

    #[test]
    fn prop_set_overwrites(
        key in strategy_cache_key(),
        value1 in "[a-zA-Z0-9_ ]{1,500}",
        value2 in "[a-zA-Z0-9_ ]{1,500}"
    ) {
        prop_assume!(value1 != value2);
        let cache = ApiCache::open_in_memory().unwrap();
        let json_value1 = serde_json::json!(value1);
        let json_value2 = serde_json::json!(value2);

        cache.set(&key, &json_value1).unwrap();
        cache.set(&key, &json_value2).unwrap();

        let retrieved = cache.get::<serde_json::Value>(&key).unwrap();
        prop_assert_eq!(Some(json_value2), retrieved);
    }

    #[test]
    fn prop_contains_consistency(
        key in strategy_cache_key(),
        value in "[a-zA-Z0-9_ ]{1,500}"
    ) {
        let cache = ApiCache::open_in_memory().unwrap();
        let json_value = serde_json::json!(value);

        prop_assert!(!cache.contains(&key).unwrap());
        prop_assert_eq!(None, cache.get::<serde_json::Value>(&key).unwrap());

        cache.set(&key, &json_value).unwrap();
        prop_assert!(cache.contains(&key).unwrap());
        prop_assert_eq!(Some(json_value), cache.get::<serde_json::Value>(&key).unwrap());
    }
}

proptest! {
    #[test]
    fn prop_expired_entries_not_returned(
        key in strategy_cache_key(),
        value in "[a-zA-Z0-9_ ]{1,500}"
    ) {
        let cache = ApiCache::open_in_memory().unwrap().with_ttl(Duration::seconds(-1));
        let json_value = serde_json::json!(value);

        cache.set(&key, &json_value).unwrap();
        let retrieved = cache.get::<serde_json::Value>(&key).unwrap();

        prop_assert_eq!(None, retrieved);
    }

    #[test]
    fn prop_custom_ttl_applied(
        key in strategy_cache_key(),
        value in "[a-zA-Z0-9_ ]{1,500}"
    ) {
        let cache = ApiCache::open_in_memory().unwrap();
        let json_value = serde_json::json!(value);

        cache.set_with_ttl(&key, &json_value, Duration::seconds(1)).unwrap();
        let retrieved = cache.get::<serde_json::Value>(&key).unwrap();

        prop_assert_eq!(Some(json_value), retrieved);
    }
}

proptest! {
    #[test]
    fn prop_cleanup_removes_expired(
        keys in proptest::collection::vec(strategy_cache_key(), 1..20)
    ) {
        let cache = ApiCache::open_in_memory().unwrap().with_ttl(Duration::seconds(-1));

        for key in &keys {
            let value = serde_json::json!(format!("value_{key}"));
            cache.set(key, &value).unwrap();
        }

        let deleted = cache.cleanup_expired().unwrap();
        prop_assert_eq!(deleted, keys.len());

        for key in &keys {
            prop_assert_eq!(None, cache.get::<serde_json::Value>(key).unwrap());
        }
    }

    #[test]
    fn prop_clear_empties_cache(
        keys in proptest::collection::vec(strategy_cache_key(), 1..20)
    ) {
        let cache = ApiCache::open_in_memory().unwrap();

        for key in &keys {
            let value = serde_json::json!(format!("value_{key}"));
            cache.set(key, &value).unwrap();
        }

        cache.clear().unwrap();

        for key in &keys {
            prop_assert_eq!(None, cache.get::<serde_json::Value>(key).unwrap());
            prop_assert!(!cache.contains(key).unwrap());
        }
    }
}

proptest! {
    #[test]
    fn prop_persistence_across_opens(
        key in strategy_cache_key(),
        value in "[a-zA-Z0-9_ ]{1,500}"
    ) {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("cache.sqlite");
        let json_value = serde_json::json!(value);

        {
            let cache = ApiCache::open(&db_path).unwrap();
            cache.set(&key, &json_value).unwrap();
        }

        let cache = ApiCache::open(&db_path).unwrap();
        let retrieved = cache.get::<serde_json::Value>(&key).unwrap();

        prop_assert_eq!(Some(json_value), retrieved);
    }
}
