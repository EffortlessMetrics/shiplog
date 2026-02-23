#![no_main]

use chrono::{DateTime, Duration, Utc};
use libfuzzer_sys::fuzz_target;
use shiplog_cache_expiry::{CacheExpiryWindow, is_expired, is_valid, parse_rfc3339_utc};

fuzz_target!(|data: &[u8]| {
    let mut padded = [0u8; 32];
    let copy_len = data.len().min(padded.len());
    padded[..copy_len].copy_from_slice(&data[..copy_len]);

    let base_raw = i64::from_le_bytes(padded[0..8].try_into().unwrap());
    let ttl_raw = i64::from_le_bytes(padded[8..16].try_into().unwrap());
    let skew_raw = i64::from_le_bytes(padded[16..24].try_into().unwrap());
    let parse_slice = &padded[24..32];

    // Keep values in a range where timestamp arithmetic is stable.
    let base_secs = (base_raw % 4_102_444_800).abs();
    let base_secs = 2_051_222_400 - base_secs;
    let ttl_secs = ttl_raw.clamp(-31_536_000, 31_536_000);
    let skew_secs = skew_raw.clamp(-31_536_000, 31_536_000);

    if let Some(base) = DateTime::<Utc>::from_timestamp(base_secs, 0) {
        let window = CacheExpiryWindow::from_base(base, Duration::seconds(ttl_secs));
        let now = base + Duration::seconds(skew_secs);

        let expired = is_expired(window.expires_at, now);
        let valid = is_valid(window.expires_at, now);
        assert_eq!(expired, !valid);

        let cached_raw = window.cached_at_rfc3339();
        let expires_raw = window.expires_at_rfc3339();
        let parsed_cached = parse_rfc3339_utc(&cached_raw).unwrap();
        let parsed_expires = parse_rfc3339_utc(&expires_raw).unwrap();

        assert_eq!(parsed_cached, window.cached_at);
        assert_eq!(parsed_expires, window.expires_at);
    }

    if let Ok(raw) = std::str::from_utf8(parse_slice) {
        let _ = parse_rfc3339_utc(raw);
    }
});
