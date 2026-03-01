//! Backward-compatible public façade for shiplog cache APIs.
//!
//! The SQLite implementation now lives in [`shiplog_cache_sqlite`]; this crate
//! re-exports the storage API and cache-key/stat helpers used by downstream
//! consumers.

pub use shiplog_cache_sqlite::{ApiCache, CacheKey, CacheStats};
