use proptest::prelude::*;
use shiplog_timeout::*;
use std::time::Duration;

// ── TimeoutConfig defaults ──────────────────────────────────────────

#[test]
fn default_config_values() {
    let c = TimeoutConfig::default();
    assert_eq!(c.default_timeout_ms, 5000);
    assert!(!c.retry_on_timeout);
    assert_eq!(c.max_retries, 3);
}

// ── TimeoutBuilder ──────────────────────────────────────────────────

#[test]
fn builder_sets_all_fields() {
    let c = TimeoutBuilder::new()
        .default_timeout(1234)
        .retry_on_timeout(true)
        .max_retries(7)
        .build();
    assert_eq!(c.default_timeout_ms, 1234);
    assert!(c.retry_on_timeout);
    assert_eq!(c.max_retries, 7);
}

#[test]
fn builder_default_is_same_as_config_default() {
    let a = TimeoutBuilder::default().build();
    let b = TimeoutConfig::default();
    assert_eq!(a.default_timeout_ms, b.default_timeout_ms);
    assert_eq!(a.retry_on_timeout, b.retry_on_timeout);
    assert_eq!(a.max_retries, b.max_retries);
}

// ── TimeoutResult ───────────────────────────────────────────────────

#[test]
fn timeout_result_ok() {
    let r = TimeoutResult::ok(42, 100);
    assert!(r.is_ok());
    assert!(!r.is_timeout());
    assert_eq!(r.into_result().unwrap(), 42);
}

#[test]
fn timeout_result_err() {
    let r = TimeoutResult::<i32>::err(5000, "timed out");
    assert!(!r.is_ok());
    assert!(r.is_timeout());
    let e = r.into_result().unwrap_err();
    assert_eq!(e.elapsed_ms, 5000);
    assert!(e.message.contains("timed out"));
}

// ── TimeoutError ────────────────────────────────────────────────────

#[test]
fn timeout_error_display() {
    let e = TimeoutError::new(1500, "slow op");
    let s = format!("{e}");
    assert!(s.contains("1500ms"));
    assert!(s.contains("slow op"));
}

#[test]
fn timeout_error_is_std_error() {
    let e = TimeoutError::new(0, "test");
    let _: &dyn std::error::Error = &e;
}

// ── Timer ───────────────────────────────────────────────────────────

#[test]
fn timer_not_started_is_not_expired() {
    let t = Timer::new(Duration::from_millis(1));
    assert!(!t.is_expired());
}

#[test]
fn timer_not_started_remaining_is_full() {
    let t = Timer::new(Duration::from_secs(5));
    assert_eq!(t.remaining(), Some(Duration::from_secs(5)));
}

#[test]
fn timer_not_started_elapsed_is_none() {
    let t = Timer::new(Duration::from_secs(1));
    assert!(t.elapsed().is_none());
}

#[test]
fn timer_starts_and_tracks_elapsed() {
    let mut t = Timer::new(Duration::from_secs(60));
    t.start();
    assert!(t.elapsed().is_some());
    assert!(!t.is_expired());
}

#[test]
fn timer_expires() {
    let mut t = Timer::new(Duration::from_millis(20));
    t.start();
    std::thread::sleep(Duration::from_millis(40));
    assert!(t.is_expired());
    assert!(t.remaining().is_none());
}

// ── Edge cases ──────────────────────────────────────────────────────

#[test]
fn timer_zero_duration_expires_immediately() {
    let mut t = Timer::new(Duration::ZERO);
    t.start();
    assert!(t.is_expired());
}

#[test]
fn builder_zero_timeout() {
    let c = TimeoutBuilder::new().default_timeout(0).build();
    assert_eq!(c.default_timeout_ms, 0);
}

#[test]
fn builder_zero_retries() {
    let c = TimeoutBuilder::new().max_retries(0).build();
    assert_eq!(c.max_retries, 0);
}

// ── Property tests ──────────────────────────────────────────────────

proptest! {
    #[test]
    fn builder_roundtrip(ms in 0u64..100_000, retries in 0u32..100, retry in proptest::bool::ANY) {
        let c = TimeoutBuilder::new()
            .default_timeout(ms)
            .max_retries(retries)
            .retry_on_timeout(retry)
            .build();
        prop_assert_eq!(c.default_timeout_ms, ms);
        prop_assert_eq!(c.max_retries, retries);
        prop_assert_eq!(c.retry_on_timeout, retry);
    }

    #[test]
    fn timeout_result_ok_always_succeeds(val in any::<i64>(), elapsed in 0u64..100_000) {
        let r = TimeoutResult::ok(val, elapsed);
        prop_assert!(r.is_ok());
        prop_assert!(!r.is_timeout());
    }

    #[test]
    fn timer_remaining_not_started_equals_duration(ms in 1u64..100_000) {
        let d = Duration::from_millis(ms);
        let t = Timer::new(d);
        prop_assert_eq!(t.remaining(), Some(d));
    }
}
