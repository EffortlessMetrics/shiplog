//! Integration tests for shiplog-cache-sqlite <-> cache-expiry wiring.

use chrono::Duration;
use rusqlite::Connection;
use shiplog_cache_expiry::{is_expired, is_valid, parse_rfc3339_utc};
use shiplog_cache_sqlite::ApiCache;

#[test]
fn stored_timestamps_follow_expiry_window_contract() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("cache.sqlite");

    let cache = ApiCache::open(&db_path).unwrap();
    cache
        .set_with_ttl(
            "contract-key",
            &serde_json::json!({"status":"ok"}),
            Duration::seconds(120),
        )
        .unwrap();
    drop(cache);

    let conn = Connection::open(&db_path).unwrap();
    let (cached_at_raw, expires_at_raw): (String, String) = conn
        .query_row(
            "SELECT cached_at, expires_at FROM cache_entries WHERE key = ?1",
            ["contract-key"],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();

    let cached_at = parse_rfc3339_utc(&cached_at_raw).unwrap();
    let expires_at = parse_rfc3339_utc(&expires_at_raw).unwrap();

    assert_eq!(expires_at - cached_at, Duration::seconds(120));
    assert!(is_valid(expires_at, cached_at));
    assert!(!is_expired(expires_at, cached_at));
}

#[test]
fn negative_ttl_entries_are_expired_immediately() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("cache.sqlite");

    let cache = ApiCache::open(&db_path).unwrap();
    cache
        .set_with_ttl(
            "expired-key",
            &serde_json::json!({"status":"expired"}),
            Duration::seconds(-5),
        )
        .unwrap();

    let retrieved: Option<serde_json::Value> = cache.get("expired-key").unwrap();
    assert!(retrieved.is_none());
    drop(cache);

    let conn = Connection::open(&db_path).unwrap();
    let (cached_at_raw, expires_at_raw): (String, String) = conn
        .query_row(
            "SELECT cached_at, expires_at FROM cache_entries WHERE key = ?1",
            ["expired-key"],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();

    let cached_at = parse_rfc3339_utc(&cached_at_raw).unwrap();
    let expires_at = parse_rfc3339_utc(&expires_at_raw).unwrap();

    assert!(is_expired(expires_at, cached_at));
    assert!(!is_valid(expires_at, cached_at));
}
