//! Canonical cache-stat normalization contracts for shiplog caches.
//!
//! This crate has one responsibility:
//! converting raw storage counts into stable, non-negative cache stats.

/// Byte size used to compute whole-megabyte cache reports.
pub const BYTES_PER_MEGABYTE: u64 = 1024 * 1024;

/// Cache statistics exposed by shiplog caches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub valid_entries: usize,
    pub cache_size_mb: u64,
}

impl CacheStats {
    /// Build canonical stats from raw storage values.
    ///
    /// Rules:
    /// - negative values clamp to zero
    /// - expired entries clamp to total entries
    /// - valid entries are always `total - expired`
    /// - cache size is reported as floor(bytes / MiB)
    #[must_use]
    pub fn from_raw_counts(
        total_entries: i64,
        expired_entries: i64,
        cache_size_bytes: i64,
    ) -> Self {
        let total_raw = total_entries.max(0) as u64;
        let expired_raw = expired_entries.max(0) as u64;
        let clamped_expired_raw = expired_raw.min(total_raw);

        let total_entries = clamp_u64_to_usize(total_raw);
        let expired_entries = clamp_u64_to_usize(clamped_expired_raw).min(total_entries);
        let valid_entries = total_entries.saturating_sub(expired_entries);
        let cache_size_mb = (cache_size_bytes.max(0) as u64) / BYTES_PER_MEGABYTE;

        Self {
            total_entries,
            expired_entries,
            valid_entries,
            cache_size_mb,
        }
    }

    /// True when the cache has no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.total_entries == 0
    }
}

fn clamp_u64_to_usize(value: u64) -> usize {
    let max = usize::MAX as u64;
    value.min(max) as usize
}

#[cfg(test)]
mod tests {
    use super::{BYTES_PER_MEGABYTE, CacheStats};

    #[test]
    fn from_raw_counts_maps_normal_values() {
        let stats = CacheStats::from_raw_counts(9, 2, 5_242_880);
        assert_eq!(stats.total_entries, 9);
        assert_eq!(stats.expired_entries, 2);
        assert_eq!(stats.valid_entries, 7);
        assert_eq!(stats.cache_size_mb, 5);
    }

    #[test]
    fn from_raw_counts_clamps_negative_values() {
        let stats = CacheStats::from_raw_counts(-10, -3, -99);
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.expired_entries, 0);
        assert_eq!(stats.valid_entries, 0);
        assert_eq!(stats.cache_size_mb, 0);
    }

    #[test]
    fn from_raw_counts_clamps_expired_entries_to_total() {
        let stats = CacheStats::from_raw_counts(3, 99, 0);
        assert_eq!(stats.total_entries, 3);
        assert_eq!(stats.expired_entries, 3);
        assert_eq!(stats.valid_entries, 0);
    }

    #[test]
    fn from_raw_counts_rounds_down_megabytes() {
        let stats = CacheStats::from_raw_counts(1, 0, (BYTES_PER_MEGABYTE * 2 + 123) as i64);
        assert_eq!(stats.cache_size_mb, 2);
    }

    #[test]
    fn is_empty_is_driven_by_total_entries() {
        assert!(CacheStats::from_raw_counts(0, 0, 0).is_empty());
        assert!(!CacheStats::from_raw_counts(1, 0, 0).is_empty());
    }
}
