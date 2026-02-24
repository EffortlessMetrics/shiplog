//! Property tests for shiplog-cache-expiry.

use chrono::{DateTime, Duration, Utc};
use proptest::prelude::*;
use shiplog_cache_expiry::{CacheExpiryWindow, is_expired, is_valid, parse_rfc3339_utc};

fn strategy_timestamp() -> impl Strategy<Value = DateTime<Utc>> {
    // Keep timestamp range bounded to avoid arithmetic overflow in generated cases.
    (-4_102_444_800i64..=4_102_444_800i64)
        .prop_map(|secs| DateTime::<Utc>::from_timestamp(secs, 0).expect("valid timestamp"))
}

proptest! {
    #[test]
    fn prop_window_preserves_ttl_delta(
        base in strategy_timestamp(),
        ttl_secs in -31_536_000i64..=31_536_000i64,
    ) {
        let ttl = Duration::seconds(ttl_secs);
        let window = CacheExpiryWindow::from_base(base, ttl);
        prop_assert_eq!(window.expires_at - window.cached_at, ttl);
    }

    #[test]
    fn prop_validity_and_expiry_are_complements(
        expires_at in strategy_timestamp(),
        now in strategy_timestamp(),
    ) {
        prop_assert_eq!(is_expired(expires_at, now), !is_valid(expires_at, now));
    }

    #[test]
    fn prop_rfc3339_round_trip_is_lossless(
        base in strategy_timestamp(),
        ttl_secs in -86_400i64..=86_400i64,
    ) {
        let window = CacheExpiryWindow::from_base(base, Duration::seconds(ttl_secs));
        let cached = window.cached_at_rfc3339();
        let expires = window.expires_at_rfc3339();

        let parsed_cached = parse_rfc3339_utc(&cached).expect("cached_at should parse");
        let parsed_expires = parse_rfc3339_utc(&expires).expect("expires_at should parse");

        prop_assert_eq!(parsed_cached, window.cached_at);
        prop_assert_eq!(parsed_expires, window.expires_at);
    }
}
