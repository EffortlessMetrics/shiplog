//! Local SQLite cache for GitHub API responses.
//!
//! This module provides durable caching to reduce API calls and speed up
//! repeated runs. Cache entries are keyed by URL and include TTL support.

use anyhow::{Context, Result};
use chrono::Duration;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use serde::de::DeserializeOwned;
use shiplog_cache_expiry::{CacheExpiryWindow, now_rfc3339};
use std::path::Path;

/// Cache for GitHub API responses.
///
/// Stores JSON responses in SQLite with configurable TTL.
/// Thread-safe via internal connection management.
#[derive(Debug)]
pub struct ApiCache {
    conn: Connection,
    default_ttl: Duration,
    max_size_bytes: Option<u64>,
}

impl ApiCache {
    /// Open or create cache at the given path.
    ///
    /// If the database doesn't exist, it will be created with the schema.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path).context("open cache database")?;

        // Create tables if they don't exist
        conn.execute(
            "CREATE TABLE IF NOT EXISTS cache_entries (
                key TEXT PRIMARY KEY,
                data TEXT NOT NULL,
                cached_at TEXT NOT NULL,
                expires_at TEXT NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_expires ON cache_entries(expires_at)",
            [],
        )?;

        Ok(Self {
            conn,
            default_ttl: Duration::hours(24),
            max_size_bytes: None,
        })
    }

    /// Create an in-memory cache (for testing).
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().context("open in-memory cache")?;

        conn.execute(
            "CREATE TABLE cache_entries (
                key TEXT PRIMARY KEY,
                data TEXT NOT NULL,
                cached_at TEXT NOT NULL,
                expires_at TEXT NOT NULL
            )",
            [],
        )?;

        Ok(Self {
            conn,
            default_ttl: Duration::hours(24),
            max_size_bytes: None,
        })
    }

    /// Set the default TTL for cache entries.
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = ttl;
        self
    }

    /// Create a cache with a maximum size limit.
    pub fn with_max_size(mut self, max_size_bytes: u64) -> Self {
        self.max_size_bytes = Some(max_size_bytes);
        self
    }

    /// Get a cached value if it exists and hasn't expired.
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let now = now_rfc3339();

        let row: Option<String> = self
            .conn
            .query_row(
                "SELECT data FROM cache_entries
                 WHERE key = ?1 AND expires_at > ?2",
                params![key, now],
                |row| row.get(0),
            )
            .optional()?;

        match row {
            Some(data) => {
                let value: T = serde_json::from_str(&data)
                    .with_context(|| format!("deserialize cached value for key: {key}"))?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    /// Store a value in the cache.
    pub fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        self.set_with_ttl(key, value, self.default_ttl)
    }

    /// Store a value with a custom TTL.
    pub fn set_with_ttl<T: Serialize>(&self, key: &str, value: &T, ttl: Duration) -> Result<()> {
        let window = CacheExpiryWindow::from_now(ttl);
        let data = serde_json::to_string(value)
            .with_context(|| format!("serialize value for key: {key}"))?;

        self.conn.execute(
            "INSERT OR REPLACE INTO cache_entries (key, data, cached_at, expires_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                key,
                data,
                window.cached_at_rfc3339(),
                window.expires_at_rfc3339()
            ],
        )?;

        Ok(())
    }

    /// Check if a key exists and hasn't expired.
    pub fn contains(&self, key: &str) -> Result<bool> {
        let now = now_rfc3339();

        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM cache_entries 
                 WHERE key = ?1 AND expires_at > ?2",
            params![key, now],
            |row| row.get(0),
        )?;

        Ok(count > 0)
    }

    /// Remove expired entries from the cache.
    pub fn cleanup_expired(&self) -> Result<usize> {
        let now = now_rfc3339();

        let deleted = self.conn.execute(
            "DELETE FROM cache_entries WHERE expires_at <= ?1",
            params![now],
        )?;

        Ok(deleted)
    }

    /// Clear all entries from the cache.
    pub fn clear(&self) -> Result<()> {
        self.conn.execute("DELETE FROM cache_entries", [])?;
        Ok(())
    }

    /// Get cache statistics.
    pub fn stats(&self) -> Result<CacheStats> {
        let now = now_rfc3339();

        let total: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM cache_entries", [], |row| row.get(0))?;

        let expired: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM cache_entries WHERE expires_at <= ?1",
            params![now],
            |row| row.get(0),
        )?;

        // Calculate cache size in bytes
        let size_bytes: i64 =
            self.conn
                .query_row("SELECT SUM(LENGTH(data)) FROM cache_entries", [], |row| {
                    Ok(row.get::<_, Option<i64>>(0).unwrap_or(Some(0)).unwrap_or(0))
                })?;

        Ok(CacheStats::from_raw_counts(total, expired, size_bytes))
    }
}

pub use shiplog_cache_key::CacheKey;
pub use shiplog_cache_stats::CacheStats;

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Clone)]
    struct TestData {
        name: String,
        count: u32,
    }

    #[test]
    fn cache_basic_operations() {
        let cache = ApiCache::open_in_memory().unwrap();

        let data = TestData {
            name: "test".to_string(),
            count: 42,
        };

        // Initially not in cache
        let result: Option<TestData> = cache.get("key1").unwrap();
        assert!(result.is_none());

        // Store in cache
        cache.set("key1", &data).unwrap();

        // Now should be retrievable
        let result: Option<TestData> = cache.get("key1").unwrap();
        assert_eq!(result, Some(data));
    }

    #[test]
    fn cache_ttl_expiration() {
        let cache = ApiCache::open_in_memory()
            .unwrap()
            .with_ttl(Duration::seconds(1));

        let data = TestData {
            name: "test".to_string(),
            count: 42,
        };

        // Store with 1 second TTL
        cache.set("key1", &data).unwrap();

        // Should be retrievable immediately
        let result: Option<TestData> = cache.get("key1").unwrap();
        assert_eq!(result, Some(data));

        // Wait for expiration
        std::thread::sleep(std::time::Duration::from_millis(1100));

        // Should not be retrievable after expiration
        let result: Option<TestData> = cache.get("key1").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn cache_stats() {
        let cache = ApiCache::open_in_memory().unwrap();

        let data = TestData {
            name: "test".to_string(),
            count: 42,
        };

        // Add some entries
        cache.set("key1", &data).unwrap();
        cache.set("key2", &data).unwrap();

        // Get stats
        let stats = cache.stats().unwrap();

        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.valid_entries, 2);
        assert_eq!(stats.expired_entries, 0);
    }

    #[test]
    fn cache_cleanup() {
        let cache = ApiCache::open_in_memory().unwrap();

        let data = TestData {
            name: "test".to_string(),
            count: 42,
        };

        // Add an entry with expired TTL
        cache
            .set_with_ttl("key1", &data, Duration::seconds(-1))
            .unwrap();

        // Cleanup should remove it
        let deleted = cache.cleanup_expired().unwrap();
        assert_eq!(deleted, 1);

        // Stats should show 0 expired entries
        let stats = cache.stats().unwrap();
        assert_eq!(stats.expired_entries, 0);
    }

    #[test]
    fn cache_clear() {
        let cache = ApiCache::open_in_memory().unwrap();

        let data = TestData {
            name: "test".to_string(),
            count: 42,
        };

        // Add some entries
        cache.set("key1", &data).unwrap();
        cache.set("key2", &data).unwrap();

        // Clear all entries
        cache.clear().unwrap();

        // Stats should show 0 total entries
        let stats = cache.stats().unwrap();
        assert_eq!(stats.total_entries, 0);
    }

    #[test]
    fn cache_contains() {
        let cache = ApiCache::open_in_memory().unwrap();

        let data = TestData {
            name: "test".to_string(),
            count: 42,
        };

        // Initially not in cache
        assert!(!cache.contains("key1").unwrap());

        // Store in cache
        cache.set("key1", &data).unwrap();

        // Now should be in cache
        assert!(cache.contains("key1").unwrap());
    }

    #[test]
    fn cache_key_reexport_matches_contract() {
        let details = CacheKey::pr_details("https://api.github.com/repos/o/r/pulls/1");
        let reviews = CacheKey::pr_reviews("https://api.github.com/repos/o/r/pulls/1", 2);
        let notes = CacheKey::mr_notes(12, 34, 1);

        assert_eq!(
            details,
            "pr:details:https://api.github.com/repos/o/r/pulls/1"
        );
        assert_eq!(
            reviews,
            "pr:reviews:https://api.github.com/repos/o/r/pulls/1:page2"
        );
        assert_eq!(notes, "gitlab:mr:notes:project12:mr34:page1");
    }

    #[test]
    fn cache_stats_reexport_matches_contract() {
        let stats = CacheStats::from_raw_counts(5, 2, 2 * 1024 * 1024 + 77);
        assert_eq!(stats.total_entries, 5);
        assert_eq!(stats.expired_entries, 2);
        assert_eq!(stats.valid_entries, 3);
        assert_eq!(stats.cache_size_mb, 2);
    }
}
