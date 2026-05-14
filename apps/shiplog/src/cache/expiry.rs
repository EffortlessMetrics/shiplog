//! Canonical cache-expiry helpers for shiplog cache implementations.
//!
//! This crate has a single responsibility:
//! - represent cache timestamp windows (`cached_at`, `expires_at`)
//! - provide canonical validity semantics (`expires_at > now`)

use chrono::{DateTime, Duration, ParseError, Utc};

/// A cache timestamp window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheExpiryWindow {
    pub cached_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl CacheExpiryWindow {
    /// Build a window from the current UTC time and a TTL.
    #[must_use]
    pub fn from_now(ttl: Duration) -> Self {
        Self::from_base(Utc::now(), ttl)
    }

    /// Build a window from an explicit base timestamp and a TTL.
    #[must_use]
    pub fn from_base(cached_at: DateTime<Utc>, ttl: Duration) -> Self {
        Self {
            cached_at,
            expires_at: cached_at + ttl,
        }
    }

    /// Whether this window is expired at `now`.
    #[must_use]
    pub fn is_expired_at(&self, now: DateTime<Utc>) -> bool {
        is_expired(self.expires_at, now)
    }

    /// Whether this window is valid at `now`.
    #[must_use]
    pub fn is_valid_at(&self, now: DateTime<Utc>) -> bool {
        is_valid(self.expires_at, now)
    }

    /// Cached-at timestamp encoded as RFC3339.
    #[must_use]
    pub fn cached_at_rfc3339(&self) -> String {
        self.cached_at.to_rfc3339()
    }

    /// Expires-at timestamp encoded as RFC3339.
    #[must_use]
    pub fn expires_at_rfc3339(&self) -> String {
        self.expires_at.to_rfc3339()
    }
}

/// Current UTC timestamp encoded as RFC3339.
#[must_use]
pub fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

/// Canonical expiry predicate: an entry is expired when `expires_at <= now`.
#[must_use]
pub fn is_expired(expires_at: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    expires_at <= now
}

/// Canonical validity predicate: an entry is valid when `expires_at > now`.
#[must_use]
pub fn is_valid(expires_at: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    expires_at > now
}

/// Parse an RFC3339 timestamp into UTC.
pub fn parse_rfc3339_utc(raw: &str) -> Result<DateTime<Utc>, ParseError> {
    DateTime::parse_from_rfc3339(raw).map(|dt| dt.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dt(secs: i64) -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(secs, 0).expect("valid timestamp")
    }

    #[test]
    fn window_from_base_keeps_exact_ttl_delta() {
        let cached = dt(1_700_000_000);
        let window = CacheExpiryWindow::from_base(cached, Duration::seconds(90));
        assert_eq!(window.expires_at - window.cached_at, Duration::seconds(90));
    }

    #[test]
    fn validity_and_expiry_follow_strict_gt_contract() {
        let cached = dt(1_700_000_000);
        let window = CacheExpiryWindow::from_base(cached, Duration::seconds(30));
        let at_expiry = cached + Duration::seconds(30);

        assert!(window.is_valid_at(cached));
        assert!(!window.is_valid_at(at_expiry));
        assert!(window.is_expired_at(at_expiry));
    }

    #[test]
    fn rfc3339_round_trip_preserves_timestamp() {
        let cached = dt(1_700_000_000);
        let window = CacheExpiryWindow::from_base(cached, Duration::seconds(60));

        let parsed_cached = parse_rfc3339_utc(&window.cached_at_rfc3339()).unwrap();
        let parsed_expires = parse_rfc3339_utc(&window.expires_at_rfc3339()).unwrap();

        assert_eq!(parsed_cached, window.cached_at);
        assert_eq!(parsed_expires, window.expires_at);
    }
}
