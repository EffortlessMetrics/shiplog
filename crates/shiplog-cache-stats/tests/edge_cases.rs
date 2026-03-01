//! Edge-case tests for shiplog-cache-stats.

use shiplog_cache_stats::{BYTES_PER_MEGABYTE, CacheStats};

#[test]
fn zero_total_with_nonzero_expired_clamps_to_zero() {
    let stats = CacheStats::from_raw_counts(0, 100, 0);
    assert_eq!(stats.total_entries, 0);
    assert_eq!(stats.expired_entries, 0);
    assert_eq!(stats.valid_entries, 0);
}

#[test]
fn all_entries_expired() {
    let stats = CacheStats::from_raw_counts(5, 5, 0);
    assert_eq!(stats.total_entries, 5);
    assert_eq!(stats.expired_entries, 5);
    assert_eq!(stats.valid_entries, 0);
}

#[test]
fn no_entries_expired() {
    let stats = CacheStats::from_raw_counts(5, 0, 0);
    assert_eq!(stats.total_entries, 5);
    assert_eq!(stats.expired_entries, 0);
    assert_eq!(stats.valid_entries, 5);
}

#[test]
fn one_byte_less_than_one_mb() {
    let stats = CacheStats::from_raw_counts(1, 0, (BYTES_PER_MEGABYTE - 1) as i64);
    assert_eq!(stats.cache_size_mb, 0);
}

#[test]
fn exactly_one_mb() {
    let stats = CacheStats::from_raw_counts(1, 0, BYTES_PER_MEGABYTE as i64);
    assert_eq!(stats.cache_size_mb, 1);
}

#[test]
fn one_byte_more_than_one_mb() {
    let stats = CacheStats::from_raw_counts(1, 0, (BYTES_PER_MEGABYTE + 1) as i64);
    assert_eq!(stats.cache_size_mb, 1);
}

#[test]
fn i64_max_bytes() {
    let stats = CacheStats::from_raw_counts(1, 0, i64::MAX);
    let expected_mb = (i64::MAX as u64) / BYTES_PER_MEGABYTE;
    assert_eq!(stats.cache_size_mb, expected_mb);
}

#[test]
fn i64_min_bytes_clamps_to_zero() {
    let stats = CacheStats::from_raw_counts(1, 0, i64::MIN);
    assert_eq!(stats.cache_size_mb, 0);
}

#[test]
fn i64_max_total_and_expired() {
    let stats = CacheStats::from_raw_counts(i64::MAX, i64::MAX, 0);
    assert_eq!(stats.expired_entries, stats.total_entries);
    assert_eq!(stats.valid_entries, 0);
}

#[test]
fn i64_min_total_and_expired() {
    let stats = CacheStats::from_raw_counts(i64::MIN, i64::MIN, 0);
    assert_eq!(stats.total_entries, 0);
    assert_eq!(stats.expired_entries, 0);
    assert_eq!(stats.valid_entries, 0);
}

#[test]
fn is_empty_when_all_zeros() {
    assert!(CacheStats::from_raw_counts(0, 0, 0).is_empty());
}

#[test]
fn not_empty_with_one_entry() {
    assert!(!CacheStats::from_raw_counts(1, 0, 0).is_empty());
}

#[test]
fn not_empty_with_only_expired_entries() {
    assert!(!CacheStats::from_raw_counts(1, 1, 0).is_empty());
}

#[test]
fn bytes_per_megabyte_constant_is_binary_mib() {
    assert_eq!(BYTES_PER_MEGABYTE, 1024 * 1024);
    assert_eq!(BYTES_PER_MEGABYTE, 1_048_576);
}

#[test]
fn copy_and_clone_semantics() {
    let s1 = CacheStats::from_raw_counts(10, 3, 2 * 1024 * 1024);
    let s2 = s1;
    let s3 = s1;
    assert_eq!(s1, s2);
    assert_eq!(s1, s3);
}

#[test]
fn debug_output_is_available() {
    let stats = CacheStats::from_raw_counts(5, 2, 1024 * 1024);
    let debug = format!("{stats:?}");
    assert!(debug.contains("CacheStats"));
}
