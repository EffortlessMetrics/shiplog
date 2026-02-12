//! Local SQLite cache for GitHub API responses.
//!
//! This module provides durable caching to reduce API calls and speed up
//! repeated runs. Cache entries are keyed by URL and include TTL support.

use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::path::Path;

/// Cache for GitHub API responses.
///
/// Stores JSON responses in SQLite with configurable TTL.
/// Thread-safe via internal connection management.
#[derive(Debug)]
pub struct ApiCache {
    conn: Connection,
    default_ttl: Duration,
}

impl Clone for ApiCache {
    fn clone(&self) -> Self {
        // Open a new connection to the same database
        // This is a best-effort clone for the derive macro
        let path = self.conn.path().unwrap_or(":memory:");
        let conn = Connection::open(path).expect("Failed to clone cache connection");
        Self {
            conn,
            default_ttl: self.default_ttl,
        }
    }
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
        })
    }

    /// Set the default TTL for cache entries.
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = ttl;
        self
    }

    /// Get a cached value if it exists and hasn't expired.
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let now = Utc::now();

        let row: Option<(String,)> = self
            .conn
            .query_row(
                "SELECT data FROM cache_entries 
                 WHERE key = ?1 AND expires_at > ?2",
                params![key, now.to_rfc3339()],
                |row| Ok((row.get(0)?,)),
            )
            .optional()?;

        match row {
            Some((data,)) => {
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
        let now = Utc::now();
        let expires = now + ttl;
        let data = serde_json::to_string(value)
            .with_context(|| format!("serialize value for key: {key}"))?;

        self.conn.execute(
            "INSERT OR REPLACE INTO cache_entries (key, data, cached_at, expires_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![key, data, now.to_rfc3339(), expires.to_rfc3339()],
        )?;

        Ok(())
    }

    /// Check if a key exists and hasn't expired.
    pub fn contains(&self, key: &str) -> Result<bool> {
        let now = Utc::now();

        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM cache_entries 
                 WHERE key = ?1 AND expires_at > ?2",
            params![key, now.to_rfc3339()],
            |row| row.get(0),
        )?;

        Ok(count > 0)
    }

    /// Remove expired entries from the cache.
    pub fn cleanup_expired(&self) -> Result<usize> {
        let now = Utc::now();

        let deleted = self.conn.execute(
            "DELETE FROM cache_entries WHERE expires_at <= ?1",
            params![now.to_rfc3339()],
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
        let now = Utc::now();

        let total: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM cache_entries", [], |row| row.get(0))?;

        let expired: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM cache_entries WHERE expires_at <= ?1",
            params![now.to_rfc3339()],
            |row| row.get(0),
        )?;

        Ok(CacheStats {
            total_entries: total as usize,
            expired_entries: expired as usize,
            valid_entries: (total - expired) as usize,
        })
    }
}

/// Cache statistics.
#[derive(Debug, Clone, Copy)]
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub valid_entries: usize,
}

/// Cache key builder for GitHub API requests.
pub struct CacheKey;

impl CacheKey {
    /// Create a key for a search query.
    pub fn search(query: &str, page: u32, per_page: u32) -> String {
        format!(
            "search:{}:page{}:per{}",
            Self::hash_query(query),
            page,
            per_page
        )
    }

    /// Create a key for PR details.
    pub fn pr_details(pr_api_url: &str) -> String {
        format!("pr:details:{}", pr_api_url)
    }

    /// Create a key for PR reviews.
    pub fn pr_reviews(pr_api_url: &str, page: u32) -> String {
        format!("pr:reviews:{}:page{}", pr_api_url, page)
    }

    fn hash_query(query: &str) -> String {
        // Use a simple hash for the query to keep keys reasonably sized
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        query.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

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
    fn cache_expiration() {
        let cache = ApiCache::open_in_memory()
            .unwrap()
            .with_ttl(Duration::seconds(-1)); // Already expired

        let data = TestData {
            name: "expired".to_string(),
            count: 0,
        };

        cache.set("expired_key", &data).unwrap();

        // Should not be retrievable (expired)
        let result: Option<TestData> = cache.get("expired_key").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn cache_contains_check() {
        let cache = ApiCache::open_in_memory().unwrap();

        let data = TestData {
            name: "test".to_string(),
            count: 1,
        };

        assert!(!cache.contains("key").unwrap());

        cache.set("key", &data).unwrap();

        assert!(cache.contains("key").unwrap());
    }

    #[test]
    fn cache_cleanup() {
        let cache = ApiCache::open_in_memory()
            .unwrap()
            .with_ttl(Duration::seconds(-1));

        let data = TestData {
            name: "old".to_string(),
            count: 1,
        };

        cache.set("old1", &data).unwrap();
        cache.set("old2", &data).unwrap();

        let stats_before = cache.stats().unwrap();
        assert_eq!(stats_before.total_entries, 2);
        assert_eq!(stats_before.expired_entries, 2);

        let deleted = cache.cleanup_expired().unwrap();
        assert_eq!(deleted, 2);

        let stats_after = cache.stats().unwrap();
        assert_eq!(stats_after.total_entries, 0);
    }

    #[test]
    fn cache_key_builder() {
        let search_key = CacheKey::search("is:pr author:user", 1, 100);
        assert!(search_key.starts_with("search:"));
        assert!(search_key.contains(":page1:per100"));

        let pr_key = CacheKey::pr_details("https://api.github.com/repos/owner/repo/pulls/42");
        assert_eq!(
            pr_key,
            "pr:details:https://api.github.com/repos/owner/repo/pulls/42"
        );

        let reviews_key =
            CacheKey::pr_reviews("https://api.github.com/repos/owner/repo/pulls/42", 2);
        assert_eq!(
            reviews_key,
            "pr:reviews:https://api.github.com/repos/owner/repo/pulls/42:page2"
        );
    }

    #[test]
    fn cache_stats() {
        let cache = ApiCache::open_in_memory().unwrap();

        let data = TestData {
            name: "stats".to_string(),
            count: 100,
        };

        let stats_empty = cache.stats().unwrap();
        assert_eq!(stats_empty.total_entries, 0);
        assert_eq!(stats_empty.valid_entries, 0);

        cache.set("key1", &data).unwrap();
        cache.set("key2", &data).unwrap();

        let stats_full = cache.stats().unwrap();
        assert_eq!(stats_full.total_entries, 2);
        assert_eq!(stats_full.valid_entries, 2);
        assert_eq!(stats_full.expired_entries, 0);
    }
}
