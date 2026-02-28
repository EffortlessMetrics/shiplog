//! Edge-case tests for shiplog-cache-expiry.

use chrono::{DateTime, Duration, Utc};
use shiplog_cache_expiry::{CacheExpiryWindow, is_expired, is_valid, parse_rfc3339_utc};

fn dt(secs: i64) -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(secs, 0).expect("valid timestamp")
}

// ------------------------------------------------------------------
// Boundary timestamps
// ------------------------------------------------------------------

#[test]
fn window_at_unix_epoch() {
    let epoch = dt(0);
    let w = CacheExpiryWindow::from_base(epoch, Duration::seconds(1));
    assert_eq!(w.cached_at, epoch);
    assert_eq!(w.expires_at, dt(1));
}

#[test]
fn window_just_before_epoch() {
    let before = dt(-1);
    let w = CacheExpiryWindow::from_base(before, Duration::seconds(2));
    assert_eq!(w.expires_at, dt(1));
    assert!(w.is_valid_at(dt(0)));
    assert!(w.is_expired_at(dt(1)));
}

#[test]
fn one_nanosecond_cannot_be_represented_so_zero_ttl_expires_immediately() {
    // Duration::seconds(0) means the entry expires at the same instant
    let base = dt(500);
    let w = CacheExpiryWindow::from_base(base, Duration::zero());
    assert!(w.is_expired_at(base));
}

// ------------------------------------------------------------------
// Complement invariant at boundary
// ------------------------------------------------------------------

#[test]
fn complement_invariant_at_exact_boundary() {
    let boundary = dt(999);
    assert!(is_expired(boundary, boundary));
    assert!(!is_valid(boundary, boundary));
    // Exactly one must be true
    assert_ne!(is_expired(boundary, boundary), is_valid(boundary, boundary));
}

#[test]
fn complement_invariant_when_now_is_far_past() {
    let expires = dt(1_000_000);
    let now = dt(0);
    assert!(!is_expired(expires, now));
    assert!(is_valid(expires, now));
}

#[test]
fn complement_invariant_when_now_is_far_future() {
    let expires = dt(0);
    let now = dt(1_000_000);
    assert!(is_expired(expires, now));
    assert!(!is_valid(expires, now));
}

// ------------------------------------------------------------------
// RFC3339 edge cases
// ------------------------------------------------------------------

#[test]
fn parse_rfc3339_utc_with_timezone_offset() {
    // +00:00 should parse identically to Z
    let z = parse_rfc3339_utc("2025-06-15T12:00:00Z").unwrap();
    let offset = parse_rfc3339_utc("2025-06-15T12:00:00+00:00").unwrap();
    assert_eq!(z, offset);
}

#[test]
fn parse_rfc3339_utc_converts_non_utc_offset() {
    let parsed = parse_rfc3339_utc("2025-06-15T12:00:00+05:30").unwrap();
    let expected = parse_rfc3339_utc("2025-06-15T06:30:00Z").unwrap();
    assert_eq!(parsed, expected);
}

#[test]
fn parse_rfc3339_utc_rejects_bare_date() {
    assert!(parse_rfc3339_utc("2025-06-15").is_err());
}

#[test]
fn parse_rfc3339_utc_rejects_whitespace() {
    assert!(parse_rfc3339_utc("  ").is_err());
}

// ------------------------------------------------------------------
// Multiple windows
// ------------------------------------------------------------------

#[test]
fn overlapping_windows_can_coexist() {
    let base = dt(1000);
    let w1 = CacheExpiryWindow::from_base(base, Duration::seconds(100));
    let w2 = CacheExpiryWindow::from_base(base + Duration::seconds(50), Duration::seconds(100));

    let mid = base + Duration::seconds(75);
    assert!(w1.is_valid_at(mid));
    assert!(w2.is_valid_at(mid));

    let late = base + Duration::seconds(110);
    assert!(w1.is_expired_at(late));
    assert!(w2.is_valid_at(late));
}

#[test]
fn debug_repr_is_available() {
    let w = CacheExpiryWindow::from_base(dt(0), Duration::seconds(1));
    let debug = format!("{w:?}");
    assert!(debug.contains("CacheExpiryWindow"));
}
