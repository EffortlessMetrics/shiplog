//! Snapshot tests for shiplog-cache-expiry serializable output.

use chrono::{DateTime, Duration, Utc};
use shiplog_cache_expiry::CacheExpiryWindow;

fn dt(secs: i64) -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(secs, 0).expect("valid timestamp")
}

#[test]
fn snapshot_rfc3339_output_at_epoch() {
    let w = CacheExpiryWindow::from_base(dt(0), Duration::seconds(3600));
    insta::assert_snapshot!("epoch_cached_at", w.cached_at_rfc3339());
    insta::assert_snapshot!("epoch_expires_at", w.expires_at_rfc3339());
}

#[test]
fn snapshot_rfc3339_output_at_known_timestamp() {
    let w = CacheExpiryWindow::from_base(dt(1_700_000_000), Duration::hours(24));
    insta::assert_snapshot!("known_cached_at", w.cached_at_rfc3339());
    insta::assert_snapshot!("known_expires_at", w.expires_at_rfc3339());
}

#[test]
fn snapshot_debug_repr() {
    let w = CacheExpiryWindow::from_base(dt(1_700_000_000), Duration::seconds(90));
    insta::assert_snapshot!("debug_repr", format!("{w:?}"));
}
