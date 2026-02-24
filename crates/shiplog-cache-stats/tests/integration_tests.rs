//! Integration tests for shiplog-cache-stats.

use shiplog_cache_stats::{BYTES_PER_MEGABYTE, CacheStats};

#[test]
fn normalizes_sqlite_style_aggregate_rows() {
    let stats = CacheStats::from_raw_counts(
        27,
        4,
        (7 * BYTES_PER_MEGABYTE + BYTES_PER_MEGABYTE / 2) as i64,
    );

    assert_eq!(stats.total_entries, 27);
    assert_eq!(stats.expired_entries, 4);
    assert_eq!(stats.valid_entries, 23);
    assert_eq!(stats.cache_size_mb, 7);
}

#[test]
fn clamps_invalid_storage_values() {
    let stats = CacheStats::from_raw_counts(-1, 999, -10);

    assert_eq!(stats.total_entries, 0);
    assert_eq!(stats.expired_entries, 0);
    assert_eq!(stats.valid_entries, 0);
    assert_eq!(stats.cache_size_mb, 0);
}

#[test]
fn expired_entries_cannot_exceed_total_entries() {
    let stats = CacheStats::from_raw_counts(2, 100, 0);

    assert_eq!(stats.total_entries, 2);
    assert_eq!(stats.expired_entries, 2);
    assert_eq!(stats.valid_entries, 0);
}

#[test]
fn extreme_values_do_not_break_invariants() {
    let stats = CacheStats::from_raw_counts(i64::MAX, i64::MAX, i64::MAX);

    assert!(stats.expired_entries <= stats.total_entries);
    assert_eq!(
        stats.valid_entries,
        stats.total_entries - stats.expired_entries
    );
}
