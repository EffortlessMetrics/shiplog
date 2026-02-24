#![no_main]

use libfuzzer_sys::fuzz_target;
use shiplog_cache_stats::{BYTES_PER_MEGABYTE, CacheStats};

fuzz_target!(|data: &[u8]| {
    let mut padded = [0u8; 24];
    let copy_len = data.len().min(padded.len());
    padded[..copy_len].copy_from_slice(&data[..copy_len]);

    let total_entries = i64::from_le_bytes(padded[0..8].try_into().unwrap());
    let expired_entries = i64::from_le_bytes(padded[8..16].try_into().unwrap());
    let cache_size_bytes = i64::from_le_bytes(padded[16..24].try_into().unwrap());

    let stats = CacheStats::from_raw_counts(total_entries, expired_entries, cache_size_bytes);

    assert!(stats.expired_entries <= stats.total_entries);
    assert_eq!(stats.valid_entries, stats.total_entries - stats.expired_entries);

    if cache_size_bytes < 0 {
        assert_eq!(stats.cache_size_mb, 0);
    } else {
        let expected_mb = (cache_size_bytes as u64) / BYTES_PER_MEGABYTE;
        assert_eq!(stats.cache_size_mb, expected_mb);
    }
});
