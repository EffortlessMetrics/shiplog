//! Source freshness receipts for ingest runs.
//!
//! Each `SourceFreshness` entry attributes the cache/fetch state of one
//! source contribution within an ingest run. Adapters populate freshness
//! locally (they are the only layer that knows whether they served data
//! from cache or fetched it live); the CLI/report layer merges these
//! adapter receipts with skipped/unavailable source decisions to build
//! the final per-source freshness rollup in `intake.report.{md,json}`.
//!
//! The taxonomy is intentionally narrow in v1: `Fresh`, `Cached`, `Skipped`,
//! `Unavailable`, and `Stale` (see the `FreshnessStatus` enum below).
//! `Stale` is emitted only when an adapter receives a proven expired cache row
//! through `shiplog::cache`'s `CacheLookup::Stale(_)` result.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Per-source freshness receipt produced by an ingest adapter.
///
/// One entry per source contribution. For aggregated ingestors that
/// produce multiple source contributions in a single `ingest()` call,
/// emit one [`SourceFreshness`] per logical source.
///
/// `cache_hits` and `cache_misses` are aggregate counts across all API
/// calls the adapter made for this source during the run; they let
/// callers tell partial-cache from all-cache from all-fresh without
/// re-deriving the signal.
///
/// # Examples
///
/// ```
/// use shiplog_schema::freshness::{FreshnessStatus, SourceFreshness};
/// use chrono::Utc;
///
/// let receipt = SourceFreshness {
///     source: "github".into(),
///     status: FreshnessStatus::Cached,
///     cache_hits: 5,
///     cache_misses: 0,
///     fetched_at: Some(Utc::now()),
///     reason: None,
/// };
/// assert!(matches!(receipt.status, FreshnessStatus::Cached));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceFreshness {
    /// Source identifier (e.g. `"github"`, `"manual"`, `"json_import"`).
    pub source: String,
    /// Freshness classification for this source's contribution.
    pub status: FreshnessStatus,
    /// Number of API/storage lookups that returned a valid cached value.
    pub cache_hits: u64,
    /// Number of API/storage lookups that missed the cache and triggered
    /// a fresh fetch (or a read from current local input).
    pub cache_misses: u64,
    /// When the adapter completed its work for this source, when known.
    pub fetched_at: Option<DateTime<Utc>>,
    /// Free-form reason. Required for [`FreshnessStatus::Skipped`] and
    /// [`FreshnessStatus::Unavailable`]; optional otherwise.
    pub reason: Option<String>,
}

/// Per-source freshness classification.
///
/// `Stale` is emitted only from a proven stale cache lookup. Adapters must not
/// infer it from a miss, because a miss can mean no row, an expired row that was
/// purged, or a different cache key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FreshnessStatus {
    /// Data was fetched live or read from current local input this run.
    /// At least one cache miss occurred (or the adapter has no cache).
    Fresh,
    /// All data for this source was served from a valid (unexpired)
    /// cache entry. No fresh fetch happened.
    Cached,
    /// The cache served data past its TTL from a proven stale row.
    Stale,
    /// The source was intentionally not attempted this run (e.g.
    /// configuration omitted it, or credentials were absent and the
    /// source was elided rather than failing the run).
    Skipped,
    /// The source was attempted or configured but produced no usable
    /// result (e.g. transient failure, partial-fetch error).
    Unavailable,
}

impl FreshnessStatus {
    /// Stable lower-case string label for rendering in `intake.report.md`.
    pub fn as_label(&self) -> &'static str {
        match self {
            FreshnessStatus::Fresh => "fresh",
            FreshnessStatus::Cached => "cached",
            FreshnessStatus::Stale => "stale",
            FreshnessStatus::Skipped => "skipped",
            FreshnessStatus::Unavailable => "unavailable",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_label_is_snake_case() {
        assert_eq!(FreshnessStatus::Fresh.as_label(), "fresh");
        assert_eq!(FreshnessStatus::Cached.as_label(), "cached");
        assert_eq!(FreshnessStatus::Stale.as_label(), "stale");
        assert_eq!(FreshnessStatus::Skipped.as_label(), "skipped");
        assert_eq!(FreshnessStatus::Unavailable.as_label(), "unavailable");
    }

    #[test]
    fn source_freshness_round_trips_through_json() -> serde_json::Result<()> {
        let receipt = SourceFreshness {
            source: "github".into(),
            status: FreshnessStatus::Cached,
            cache_hits: 7,
            cache_misses: 0,
            fetched_at: None,
            reason: None,
        };
        let json = serde_json::to_string(&receipt)?;
        let back: SourceFreshness = serde_json::from_str(&json)?;
        assert_eq!(back, receipt);
        Ok(())
    }

    #[test]
    fn status_serialises_as_snake_case() -> serde_json::Result<()> {
        let json = serde_json::to_string(&FreshnessStatus::Unavailable)?;
        assert_eq!(json, "\"unavailable\"");
        Ok(())
    }
}
