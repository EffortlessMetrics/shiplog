use chrono::Duration;
use shiplog_throttler::*;

#[test]
fn throttler_allows_within_limit() {
    let config = ThrottlerConfig::new(3, Duration::minutes(1));
    let mut t = Throttler::new(config);
    assert!(t.try_acquire("k1"));
    assert!(t.try_acquire("k1"));
    assert!(t.try_acquire("k1"));
    assert!(!t.try_acquire("k1"));
}

#[test]
fn throttler_separate_keys() {
    let config = ThrottlerConfig::new(1, Duration::minutes(1));
    let mut t = Throttler::new(config);
    assert!(t.try_acquire("a"));
    assert!(!t.try_acquire("a"));
    assert!(t.try_acquire("b"));
    assert!(!t.try_acquire("b"));
}

#[test]
fn throttler_remaining() {
    let config = ThrottlerConfig::new(5, Duration::minutes(1));
    let mut t = Throttler::new(config);
    assert_eq!(t.remaining("k"), 5);
    t.try_acquire("k");
    assert_eq!(t.remaining("k"), 4);
}

#[test]
fn throttler_reset() {
    let config = ThrottlerConfig::new(1, Duration::minutes(1));
    let mut t = Throttler::new(config);
    t.try_acquire("k");
    assert!(!t.try_acquire("k"));
    t.reset("k");
    assert!(t.try_acquire("k"));
}

#[test]
fn throttler_clear() {
    let config = ThrottlerConfig::new(1, Duration::minutes(1));
    let mut t = Throttler::new(config);
    t.try_acquire("a");
    t.try_acquire("b");
    assert_eq!(t.len(), 2);
    t.clear();
    assert!(t.is_empty());
}

#[test]
fn throttler_presets() {
    let strict = ThrottlerConfig::strict();
    assert_eq!(strict.max_requests, 10);
    let lenient = ThrottlerConfig::lenient();
    assert_eq!(lenient.max_requests, 100);
}

#[test]
fn throttler_error_display() {
    let err = ThrottlerError::new("rate limited", "user1");
    assert!(err.to_string().contains("user1"));
    assert!(err.to_string().contains("rate limited"));
}

#[test]
fn throttler_window_expiry() {
    let config = ThrottlerConfig::new(1, Duration::milliseconds(50));
    let mut t = Throttler::new(config);
    assert!(t.try_acquire("k"));
    assert!(!t.try_acquire("k"));
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert!(t.try_acquire("k"));
}
