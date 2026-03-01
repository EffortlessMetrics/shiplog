//! Snapshot tests for shiplog-cache-stats.

use shiplog_cache_stats::CacheStats;

#[test]
fn snapshot_normal_stats_debug() {
    let stats = CacheStats::from_raw_counts(25, 8, 10 * 1024 * 1024 + 999);
    insta::assert_snapshot!("normal_stats_debug", format!("{stats:?}"));
}

#[test]
fn snapshot_empty_stats_debug() {
    let stats = CacheStats::from_raw_counts(0, 0, 0);
    insta::assert_snapshot!("empty_stats_debug", format!("{stats:?}"));
}

#[test]
fn snapshot_clamped_stats_debug() {
    let stats = CacheStats::from_raw_counts(-5, 999, -100);
    insta::assert_snapshot!("clamped_stats_debug", format!("{stats:?}"));
}
