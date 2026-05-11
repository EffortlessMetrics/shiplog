//! SQLite-backed implementation of `ApiCache` for shiplog API responses.

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::path::Path;

use crate::expiry::{CacheExpiryWindow, now_rfc3339};
use crate::stats::CacheStats;

/// Cache for API responses backed by a local SQLite database.
///
/// This is the public facade for the cache-internals seam tracked as
/// `cpf-0005` in
/// [`policy/clippy-protected-fields.toml`](../../../../policy/clippy-protected-fields.toml).
/// External callers must use the public methods on this type; adding a
/// `raw_connection` (or similar) accessor would re-introduce the failure
/// mode the seam guards against (custom SQL queries silently breaking on
/// a future schema migration).
///
/// The protected raw fields live on the private `ApiCacheInner`
/// struct that this type wraps. The split exists so that
/// `clippy::disallowed_fields` can target the *public* paths
/// (`ApiCache::*`) when the lint activates in a follow-up PR, while
/// internal methods continue to access their working state via the
/// private inner type without tripping the lint.
#[derive(Debug)]
pub struct ApiCache {
    inner: ApiCacheInner,
}

/// Private storage for [`ApiCache`]'s raw fields.
///
/// The `conn`, `default_ttl`, and `max_size_bytes` fields are the
/// cache-internals seam (`cpf-0005`); they live here, not on
/// `ApiCache`, so that the `clippy::disallowed_fields` activation in a
/// follow-up PR can target `ApiCache::*` without flagging this type's
/// own impl methods. External code cannot name `ApiCacheInner` (it is
/// not re-exported), so no caller outside this module can reach the
/// raw state.
#[derive(Debug)]
struct ApiCacheInner {
    conn: Connection,
    default_ttl: Duration,
    #[allow(dead_code)]
    max_size_bytes: Option<u64>,
}

/// Detailed cache inspection data for CLI and diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheInspection {
    pub stats: CacheStats,
    pub oldest_cached_at: Option<String>,
    pub newest_cached_at: Option<String>,
}

impl ApiCache {
    /// Open or create cache at the given path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path).context("open cache database")?;

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
            inner: ApiCacheInner {
                conn,
                default_ttl: Duration::hours(24),
                max_size_bytes: None,
            },
        })
    }

    /// Open an existing cache in read-only mode without initializing schema.
    pub fn open_read_only(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .context("open cache database read-only")?;

        Ok(Self {
            inner: ApiCacheInner {
                conn,
                default_ttl: Duration::hours(24),
                max_size_bytes: None,
            },
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
            inner: ApiCacheInner {
                conn,
                default_ttl: Duration::hours(24),
                max_size_bytes: None,
            },
        })
    }

    /// Set the default TTL for cache entries.
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.inner.default_ttl = ttl;
        self
    }

    /// Create a cache with a maximum size limit.
    pub fn with_max_size(mut self, max_size_bytes: u64) -> Self {
        self.inner.max_size_bytes = Some(max_size_bytes);
        self
    }

    /// Get a cached value if it exists and hasn't expired.
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let now = now_rfc3339();

        let row: Option<String> = self
            .inner
            .conn
            .query_row(
                "SELECT data FROM cache_entries WHERE key = ?1 AND expires_at > ?2",
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
        self.set_with_ttl(key, value, self.inner.default_ttl)
    }

    /// Store a value with a custom TTL.
    pub fn set_with_ttl<T: Serialize>(&self, key: &str, value: &T, ttl: Duration) -> Result<()> {
        let window = CacheExpiryWindow::from_now(ttl);
        let data = serde_json::to_string(value)
            .with_context(|| format!("serialize value for key: {key}"))?;

        self.inner.conn.execute(
            "INSERT OR REPLACE INTO cache_entries (key, data, cached_at, expires_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                key,
                data,
                window.cached_at_rfc3339(),
                window.expires_at_rfc3339(),
            ],
        )?;

        Ok(())
    }

    /// Check if a key exists and hasn't expired.
    pub fn contains(&self, key: &str) -> Result<bool> {
        let now = now_rfc3339();

        let count: i64 = self.inner.conn.query_row(
            "SELECT COUNT(*) FROM cache_entries WHERE key = ?1 AND expires_at > ?2",
            params![key, now],
            |row| row.get(0),
        )?;

        Ok(count > 0)
    }

    /// Remove expired entries from the cache.
    pub fn cleanup_expired(&self) -> Result<usize> {
        let now = now_rfc3339();

        let deleted = self.inner.conn.execute(
            "DELETE FROM cache_entries WHERE expires_at <= ?1",
            params![now],
        )?;

        Ok(deleted)
    }

    /// Count entries cached before the given cutoff.
    pub fn count_older_than(&self, cutoff: DateTime<Utc>) -> Result<usize> {
        let cutoff = cutoff.to_rfc3339();
        let count: i64 = self.inner.conn.query_row(
            "SELECT COUNT(*) FROM cache_entries WHERE cached_at < ?1",
            params![cutoff],
            |row| row.get(0),
        )?;
        Ok(count.max(0) as usize)
    }

    /// Remove entries cached before the given cutoff.
    pub fn cleanup_older_than(&self, cutoff: DateTime<Utc>) -> Result<usize> {
        let cutoff = cutoff.to_rfc3339();
        let deleted = self.inner.conn.execute(
            "DELETE FROM cache_entries WHERE cached_at < ?1",
            params![cutoff],
        )?;
        Ok(deleted)
    }

    /// Clear all entries from the cache.
    pub fn clear(&self) -> Result<()> {
        self.inner.conn.execute("DELETE FROM cache_entries", [])?;
        Ok(())
    }

    /// Get cache statistics.
    pub fn stats(&self) -> Result<CacheStats> {
        let now = now_rfc3339();

        let total: i64 =
            self.inner
                .conn
                .query_row("SELECT COUNT(*) FROM cache_entries", [], |row| row.get(0))?;

        let expired: i64 = self.inner.conn.query_row(
            "SELECT COUNT(*) FROM cache_entries WHERE expires_at <= ?1",
            params![now],
            |row| row.get(0),
        )?;

        let size_bytes: i64 = self.inner.conn.query_row(
            "SELECT SUM(LENGTH(data)) FROM cache_entries",
            [],
            |row| Ok(row.get::<_, Option<i64>>(0).unwrap_or(Some(0)).unwrap_or(0)),
        )?;

        Ok(CacheStats::from_raw_counts(total, expired, size_bytes))
    }

    /// Inspect cache statistics and entry timestamp bounds.
    pub fn inspect(&self) -> Result<CacheInspection> {
        let stats = self.stats()?;
        let oldest_cached_at =
            self.inner
                .conn
                .query_row("SELECT MIN(cached_at) FROM cache_entries", [], |row| {
                    row.get::<_, Option<String>>(0)
                })?;
        let newest_cached_at =
            self.inner
                .conn
                .query_row("SELECT MAX(cached_at) FROM cache_entries", [], |row| {
                    row.get::<_, Option<String>>(0)
                })?;
        Ok(CacheInspection {
            stats,
            oldest_cached_at,
            newest_cached_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CacheKey;

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

        let result: Option<TestData> = cache.get("key1").unwrap();
        assert!(result.is_none());

        cache.set("key1", &data).unwrap();

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

        cache.set("key1", &data).unwrap();

        let result: Option<TestData> = cache.get("key1").unwrap();
        assert_eq!(result, Some(data.clone()));

        std::thread::sleep(std::time::Duration::from_millis(1100));

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

        cache.set("key1", &data).unwrap();
        cache.set("key2", &data).unwrap();

        let stats = cache.stats().unwrap();
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.valid_entries, 2);
        assert_eq!(stats.expired_entries, 0);
    }

    #[test]
    fn cache_inspect_reports_timestamp_bounds() {
        let cache = ApiCache::open_in_memory().unwrap();

        cache.set("key1", &"one").unwrap();
        cache.set("key2", &"two").unwrap();

        let inspection = cache.inspect().unwrap();
        assert_eq!(inspection.stats.total_entries, 2);
        assert!(inspection.oldest_cached_at.is_some());
        assert!(inspection.newest_cached_at.is_some());
    }

    #[test]
    fn cache_cleanup() {
        let cache = ApiCache::open_in_memory().unwrap();

        let data = TestData {
            name: "test".to_string(),
            count: 42,
        };

        cache
            .set_with_ttl("key1", &data, Duration::seconds(-1))
            .unwrap();

        let deleted = cache.cleanup_expired().unwrap();
        assert_eq!(deleted, 1);

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

        cache.set("key1", &data).unwrap();
        cache.set("key2", &data).unwrap();

        cache.clear().unwrap();

        let stats = cache.stats().unwrap();
        assert_eq!(stats.total_entries, 0);
    }

    #[test]
    fn cache_cleanup_older_than_removes_matching_entries() {
        let cache = ApiCache::open_in_memory().unwrap();

        cache.set("old1", &"one").unwrap();
        cache.set("old2", &"two").unwrap();

        let cutoff = Utc::now() + Duration::seconds(1);
        assert_eq!(cache.count_older_than(cutoff).unwrap(), 2);
        assert_eq!(cache.cleanup_older_than(cutoff).unwrap(), 2);
        assert!(cache.stats().unwrap().is_empty());
    }

    #[test]
    fn cache_contains() {
        let cache = ApiCache::open_in_memory().unwrap();

        let data = TestData {
            name: "test".to_string(),
            count: 42,
        };

        assert!(!cache.contains("key1").unwrap());

        cache.set("key1", &data).unwrap();
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
