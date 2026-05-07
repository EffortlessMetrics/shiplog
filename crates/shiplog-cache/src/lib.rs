//! Public facade for shiplog cache APIs.
//!
//! Cache key construction, expiry semantics, statistics normalization, and the
//! SQLite-backed API cache live here as module-level implementation seams.

pub mod expiry;

mod key;
mod sqlite;
mod stats;

pub use key::CacheKey;
pub use sqlite::{ApiCache, CacheInspection};
pub use stats::{BYTES_PER_MEGABYTE, CacheStats};
