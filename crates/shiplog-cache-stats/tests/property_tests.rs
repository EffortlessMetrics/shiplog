//! Property tests for shiplog-cache-stats.

use proptest::prelude::*;
use shiplog_cache_stats::{BYTES_PER_MEGABYTE, CacheStats};

proptest! {
    #[test]
    fn prop_normalized_stats_keep_invariants(
        total in any::<i64>(),
        expired in any::<i64>(),
        bytes in any::<i64>(),
    ) {
        let stats = CacheStats::from_raw_counts(total, expired, bytes);

        prop_assert!(stats.expired_entries <= stats.total_entries);
        prop_assert_eq!(stats.valid_entries, stats.total_entries - stats.expired_entries);
        prop_assert!(stats.cache_size_mb <= (i64::MAX as u64 / BYTES_PER_MEGABYTE));
    }

    #[test]
    fn prop_negative_byte_sizes_always_clamp_to_zero(
        total in any::<i64>(),
        expired in any::<i64>(),
        bytes in i64::MIN..0i64,
    ) {
        let stats = CacheStats::from_raw_counts(total, expired, bytes);
        prop_assert_eq!(stats.cache_size_mb, 0);
    }

    #[test]
    fn prop_megabytes_are_floor_division_for_non_negative_bytes(
        total in any::<i64>(),
        expired in any::<i64>(),
        bytes in 0i64..i64::MAX,
    ) {
        let stats = CacheStats::from_raw_counts(total, expired, bytes);
        let expected_mb = (bytes as u64) / BYTES_PER_MEGABYTE;
        prop_assert_eq!(stats.cache_size_mb, expected_mb);
    }
}
