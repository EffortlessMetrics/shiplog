//! Integration tests for shiplog-cache-expiry.

use chrono::{DateTime, Duration, Utc};
use shiplog_cache_expiry::{
    CacheExpiryWindow, is_expired, is_valid, now_rfc3339, parse_rfc3339_utc,
};

fn dt(secs: i64) -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(secs, 0).expect("valid timestamp")
}

// ------------------------------------------------------------------
// CacheExpiryWindow construction
// ------------------------------------------------------------------

#[test]
fn from_base_stores_exact_timestamps() {
    let base = dt(1_700_000_000);
    let w = CacheExpiryWindow::from_base(base, Duration::seconds(120));
    assert_eq!(w.cached_at, base);
    assert_eq!(w.expires_at, base + Duration::seconds(120));
}

#[test]
fn from_now_creates_window_in_the_future() {
    let before = Utc::now();
    let w = CacheExpiryWindow::from_now(Duration::hours(1));
    let after = Utc::now();

    assert!(w.cached_at >= before);
    assert!(w.cached_at <= after);
    assert!(w.expires_at > w.cached_at);
}

#[test]
fn zero_ttl_means_immediately_expired() {
    let base = dt(1_700_000_000);
    let w = CacheExpiryWindow::from_base(base, Duration::zero());
    assert!(w.is_expired_at(base));
    assert!(!w.is_valid_at(base));
}

#[test]
fn negative_ttl_creates_already_expired_window() {
    let base = dt(1_700_000_000);
    let w = CacheExpiryWindow::from_base(base, Duration::seconds(-60));
    assert!(w.expires_at < w.cached_at);
    assert!(w.is_expired_at(base));
}

// ------------------------------------------------------------------
// Validity and expiry boundary behaviour
// ------------------------------------------------------------------

#[test]
fn valid_one_second_before_expiry() {
    let base = dt(1_700_000_000);
    let w = CacheExpiryWindow::from_base(base, Duration::seconds(100));
    let just_before = base + Duration::seconds(99);
    assert!(w.is_valid_at(just_before));
    assert!(!w.is_expired_at(just_before));
}

#[test]
fn expired_exactly_at_expiry_boundary() {
    let base = dt(1_700_000_000);
    let w = CacheExpiryWindow::from_base(base, Duration::seconds(100));
    let at_expiry = base + Duration::seconds(100);
    assert!(w.is_expired_at(at_expiry));
    assert!(!w.is_valid_at(at_expiry));
}

#[test]
fn expired_after_expiry() {
    let base = dt(1_700_000_000);
    let w = CacheExpiryWindow::from_base(base, Duration::seconds(100));
    let after = base + Duration::seconds(101);
    assert!(w.is_expired_at(after));
}

#[test]
fn valid_at_cached_time() {
    let base = dt(1_700_000_000);
    let w = CacheExpiryWindow::from_base(base, Duration::seconds(10));
    assert!(w.is_valid_at(base));
}

// ------------------------------------------------------------------
// Free functions
// ------------------------------------------------------------------

#[test]
fn is_expired_and_is_valid_are_complements() {
    let t1 = dt(100);
    let t2 = dt(200);
    assert_ne!(is_expired(t1, t2), is_valid(t1, t2));
}

#[test]
fn is_expired_when_equal_timestamps() {
    let t = dt(500);
    assert!(is_expired(t, t));
    assert!(!is_valid(t, t));
}

// ------------------------------------------------------------------
// RFC3339 round-trip
// ------------------------------------------------------------------

#[test]
fn rfc3339_round_trip_at_epoch() {
    let epoch = dt(0);
    let w = CacheExpiryWindow::from_base(epoch, Duration::seconds(1));
    let parsed = parse_rfc3339_utc(&w.cached_at_rfc3339()).unwrap();
    assert_eq!(parsed, epoch);
}

#[test]
fn rfc3339_round_trip_at_large_future_timestamp() {
    let far_future = dt(4_000_000_000);
    let w = CacheExpiryWindow::from_base(far_future, Duration::hours(1));
    let parsed = parse_rfc3339_utc(&w.expires_at_rfc3339()).unwrap();
    assert_eq!(parsed, far_future + Duration::hours(1));
}

#[test]
fn rfc3339_round_trip_at_negative_timestamp() {
    let before_epoch = dt(-86400);
    let w = CacheExpiryWindow::from_base(before_epoch, Duration::seconds(60));
    let parsed_cached = parse_rfc3339_utc(&w.cached_at_rfc3339()).unwrap();
    let parsed_expires = parse_rfc3339_utc(&w.expires_at_rfc3339()).unwrap();
    assert_eq!(parsed_cached, before_epoch);
    assert_eq!(parsed_expires, before_epoch + Duration::seconds(60));
}

#[test]
fn parse_rfc3339_utc_rejects_invalid_input() {
    assert!(parse_rfc3339_utc("not-a-date").is_err());
    assert!(parse_rfc3339_utc("").is_err());
    assert!(parse_rfc3339_utc("2025-13-01T00:00:00Z").is_err());
}

#[test]
fn now_rfc3339_produces_parseable_timestamp() {
    let raw = now_rfc3339();
    let parsed = parse_rfc3339_utc(&raw);
    assert!(parsed.is_ok(), "now_rfc3339 should produce valid RFC3339");
}

// ------------------------------------------------------------------
// Edge cases: very large TTLs
// ------------------------------------------------------------------

#[test]
fn large_ttl_does_not_panic() {
    let base = dt(0);
    let w = CacheExpiryWindow::from_base(base, Duration::days(365 * 100));
    assert!(w.expires_at > base);
    assert!(w.is_valid_at(base));
}

#[test]
fn window_copy_semantics() {
    let base = dt(1_700_000_000);
    let w1 = CacheExpiryWindow::from_base(base, Duration::seconds(60));
    let w2 = w1;
    assert_eq!(w1, w2);
}
