use proptest::prelude::*;
use shiplog_rate::*;

// ── TokenBucket correctness ─────────────────────────────────────────

#[test]
fn token_bucket_consumes_up_to_max() {
    let config = RateLimitConfig::new(5.0, 0.0); // no refill
    let mut bucket = TokenBucket::new(config);

    for _ in 0..5 {
        assert!(bucket.try_consume("k"));
    }
    assert!(!bucket.try_consume("k"));
}

#[test]
fn token_bucket_independent_keys() {
    let config = RateLimitConfig::new(2.0, 0.0);
    let mut bucket = TokenBucket::new(config);

    assert!(bucket.try_consume("a"));
    assert!(bucket.try_consume("a"));
    assert!(!bucket.try_consume("a"));

    // "b" should still have full budget
    assert!(bucket.try_consume("b"));
    assert!(bucket.try_consume("b"));
    assert!(!bucket.try_consume("b"));
}

#[test]
fn token_bucket_available_tokens_for_unknown_key() {
    let config = RateLimitConfig::new(7.0, 1.0);
    let bucket = TokenBucket::new(config);
    // Never-seen key reports max_tokens
    assert!((bucket.available_tokens("new") - 7.0).abs() < f64::EPSILON);
}

#[test]
fn token_bucket_remove_resets_key() {
    let config = RateLimitConfig::new(1.0, 0.0);
    let mut bucket = TokenBucket::new(config);

    assert!(bucket.try_consume("k"));
    assert!(!bucket.try_consume("k"));

    bucket.remove("k");
    assert!(bucket.try_consume("k"));
}

#[test]
fn token_bucket_clear_removes_all() {
    let config = RateLimitConfig::new(1.0, 0.0);
    let mut bucket = TokenBucket::new(config);

    bucket.try_consume("a");
    bucket.try_consume("b");
    bucket.clear();

    assert!(bucket.try_consume("a"));
    assert!(bucket.try_consume("b"));
}

#[test]
fn token_bucket_is_rate_limited() {
    let config = RateLimitConfig::new(1.0, 0.0);
    let mut bucket = TokenBucket::new(config);

    assert!(!bucket.is_rate_limited("k"));
    bucket.try_consume("k");
    assert!(bucket.is_rate_limited("k"));
}

// ── SlidingWindow correctness ───────────────────────────────────────

#[test]
fn sliding_window_enforces_limit() {
    let mut sw = SlidingWindow::new(3, chrono::Duration::seconds(60));

    assert!(sw.try_record("k"));
    assert!(sw.try_record("k"));
    assert!(sw.try_record("k"));
    assert!(!sw.try_record("k"));
}

#[test]
fn sliding_window_independent_keys() {
    let mut sw = SlidingWindow::new(1, chrono::Duration::seconds(60));

    assert!(sw.try_record("a"));
    assert!(!sw.try_record("a"));
    assert!(sw.try_record("b"));
}

#[test]
fn sliding_window_current_requests_unknown_key() {
    let sw = SlidingWindow::new(10, chrono::Duration::seconds(60));
    assert_eq!(sw.current_requests("missing"), 0);
}

#[test]
fn sliding_window_is_rate_limited() {
    let mut sw = SlidingWindow::new(1, chrono::Duration::seconds(60));
    assert!(!sw.is_rate_limited("k"));
    sw.try_record("k");
    assert!(sw.is_rate_limited("k"));
}

#[test]
fn sliding_window_remove_resets() {
    let mut sw = SlidingWindow::new(1, chrono::Duration::seconds(60));
    sw.try_record("k");
    assert!(!sw.try_record("k"));
    sw.remove("k");
    assert!(sw.try_record("k"));
}

#[test]
fn sliding_window_clear() {
    let mut sw = SlidingWindow::new(1, chrono::Duration::seconds(60));
    sw.try_record("a");
    sw.try_record("b");
    sw.clear();
    assert!(sw.try_record("a"));
    assert!(sw.try_record("b"));
}

// ── RateLimitConfig convenience constructors ────────────────────────

#[test]
fn per_second_config() {
    let c = RateLimitConfig::per_second(10);
    assert!((c.max_tokens - 10.0).abs() < f64::EPSILON);
    assert!((c.refill_rate - 10.0).abs() < f64::EPSILON);
}

#[test]
fn per_minute_config() {
    let c = RateLimitConfig::per_minute(60);
    assert!((c.max_tokens - 60.0).abs() < f64::EPSILON);
    assert!((c.refill_rate - 1.0).abs() < f64::EPSILON);
}

#[test]
fn per_hour_config() {
    let c = RateLimitConfig::per_hour(3600);
    assert!((c.max_tokens - 3600.0).abs() < f64::EPSILON);
    assert!((c.refill_rate - 1.0).abs() < f64::EPSILON);
}

#[test]
fn github_api_limit_values() {
    let c = github_api_limit();
    assert!((c.max_tokens - 5000.0).abs() < f64::EPSILON);
}

#[test]
fn standard_api_limit_values() {
    let c = standard_api_limit();
    assert!((c.max_tokens - 100.0).abs() < f64::EPSILON);
}

// ── Edge cases ──────────────────────────────────────────────────────

#[test]
fn token_bucket_zero_max_tokens_always_rejects() {
    let config = RateLimitConfig::new(0.0, 0.0);
    let mut bucket = TokenBucket::new(config);
    assert!(!bucket.try_consume("k"));
}

#[test]
fn sliding_window_zero_max_requests_always_rejects() {
    let mut sw = SlidingWindow::new(0, chrono::Duration::seconds(60));
    assert!(!sw.try_record("k"));
}

#[test]
fn token_bucket_fractional_token_config() {
    let config = RateLimitConfig::new(0.5, 0.0);
    let mut bucket = TokenBucket::new(config);
    // 0.5 tokens < 1.0 required → rejected immediately
    assert!(!bucket.try_consume("k"));
}

// ── Property tests ──────────────────────────────────────────────────

proptest! {
    #[test]
    fn token_bucket_never_exceeds_capacity(max_tokens in 1u32..200, num_consumes in 1usize..400) {
        let config = RateLimitConfig::new(max_tokens as f64, 0.0);
        let mut bucket = TokenBucket::new(config);
        let mut accepted = 0usize;
        for _ in 0..num_consumes {
            if bucket.try_consume("k") {
                accepted += 1;
            }
        }
        prop_assert!(accepted <= max_tokens as usize);
    }

    #[test]
    fn sliding_window_never_exceeds_limit(max_req in 1usize..100, attempts in 1usize..200) {
        let mut sw = SlidingWindow::new(max_req, chrono::Duration::seconds(600));
        let mut accepted = 0usize;
        for _ in 0..attempts {
            if sw.try_record("k") {
                accepted += 1;
            }
        }
        prop_assert!(accepted <= max_req);
    }

    #[test]
    fn per_second_refill_matches_max(rps in 1u32..10000) {
        let c = RateLimitConfig::per_second(rps);
        prop_assert!((c.max_tokens - c.refill_rate).abs() < f64::EPSILON);
    }
}
