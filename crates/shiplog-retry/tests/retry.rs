use proptest::prelude::*;
use shiplog_retry::*;
use std::time::Duration;

// ── calculate_delay ─────────────────────────────────────────────────

#[test]
fn fixed_delay_is_constant() {
    let c = RetryConfig {
        max_retries: 5,
        initial_delay_ms: 200,
        max_delay_ms: 10000,
        strategy: RetryStrategy::Fixed,
        multiplier: 2.0,
    };
    for attempt in 0..5 {
        assert_eq!(
            Retry::calculate_delay(attempt, &c),
            Duration::from_millis(200)
        );
    }
}

#[test]
fn exponential_doubles() {
    let c = RetryConfig {
        max_retries: 5,
        initial_delay_ms: 100,
        max_delay_ms: 100_000,
        strategy: RetryStrategy::Exponential,
        multiplier: 2.0,
    };
    assert_eq!(Retry::calculate_delay(0, &c), Duration::from_millis(100));
    assert_eq!(Retry::calculate_delay(1, &c), Duration::from_millis(200));
    assert_eq!(Retry::calculate_delay(2, &c), Duration::from_millis(400));
    assert_eq!(Retry::calculate_delay(3, &c), Duration::from_millis(800));
}

#[test]
fn linear_increases_linearly() {
    let c = RetryConfig {
        max_retries: 5,
        initial_delay_ms: 100,
        max_delay_ms: 100_000,
        strategy: RetryStrategy::Linear,
        multiplier: 2.0,
    };
    assert_eq!(Retry::calculate_delay(0, &c), Duration::from_millis(100));
    assert_eq!(Retry::calculate_delay(1, &c), Duration::from_millis(200));
    assert_eq!(Retry::calculate_delay(2, &c), Duration::from_millis(300));
}

#[test]
fn exponential_capped_at_max() {
    let c = RetryConfig {
        max_retries: 20,
        initial_delay_ms: 100,
        max_delay_ms: 500,
        strategy: RetryStrategy::Exponential,
        multiplier: 2.0,
    };
    for attempt in 5..20 {
        assert_eq!(
            Retry::calculate_delay(attempt, &c),
            Duration::from_millis(500)
        );
    }
}

#[test]
fn linear_capped_at_max() {
    let c = RetryConfig {
        max_retries: 20,
        initial_delay_ms: 100,
        max_delay_ms: 300,
        strategy: RetryStrategy::Linear,
        multiplier: 2.0,
    };
    assert_eq!(Retry::calculate_delay(10, &c), Duration::from_millis(300));
}

// ── Retry::execute ──────────────────────────────────────────────────

#[test]
fn immediate_success() {
    let c = Retry::default_config();
    let r = Retry::execute(&c, || Ok::<_, ()>(42));
    assert!(r.success);
    assert_eq!(r.attempts, 1);
    assert_eq!(r.result, Some(42));
}

#[test]
fn eventual_success_on_second_try() {
    let c = RetryConfig {
        max_retries: 5,
        initial_delay_ms: 1,
        max_delay_ms: 10,
        strategy: RetryStrategy::Fixed,
        multiplier: 1.0,
    };
    let mut count = 0;
    let r = Retry::execute(&c, || {
        count += 1;
        if count < 3 {
            Err::<&str, _>("fail")
        } else {
            Ok("ok")
        }
    });
    assert!(r.success);
    assert_eq!(r.attempts, 3);
    assert_eq!(r.result, Some("ok"));
}

#[test]
fn all_retries_exhausted() {
    let c = RetryConfig {
        max_retries: 3,
        initial_delay_ms: 1,
        max_delay_ms: 10,
        strategy: RetryStrategy::Fixed,
        multiplier: 1.0,
    };
    let r = Retry::execute(&c, || Err::<i32, _>("nope"));
    assert!(!r.success);
    assert_eq!(r.attempts, 3);
    assert!(r.result.is_none());
}

#[test]
fn max_retries_one_means_no_retry() {
    let c = RetryConfig {
        max_retries: 1,
        initial_delay_ms: 1,
        max_delay_ms: 10,
        strategy: RetryStrategy::Fixed,
        multiplier: 1.0,
    };
    let r = Retry::execute(&c, || Err::<i32, _>("fail"));
    assert!(!r.success);
    assert_eq!(r.attempts, 1);
}

// ── RetryStrategy default ───────────────────────────────────────────

#[test]
fn default_strategy_is_fixed() {
    assert_eq!(RetryStrategy::default(), RetryStrategy::Fixed);
}

// ── default_config ──────────────────────────────────────────────────

#[test]
fn default_config_values() {
    let c = Retry::default_config();
    assert_eq!(c.max_retries, 3);
    assert_eq!(c.initial_delay_ms, 100);
    assert_eq!(c.max_delay_ms, 30000);
    assert_eq!(c.strategy, RetryStrategy::Exponential);
    assert!((c.multiplier - 2.0).abs() < f64::EPSILON);
}

// ── Serde round-trip ────────────────────────────────────────────────

#[test]
fn config_serde_roundtrip() {
    let c = Retry::default_config();
    let json = serde_json::to_string(&c).unwrap();
    let c2: RetryConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(c2.max_retries, c.max_retries);
    assert_eq!(c2.strategy, c.strategy);
}

#[test]
fn config_deserializes_with_defaults() {
    let json = r#"{}"#;
    let c: RetryConfig = serde_json::from_str(json).unwrap();
    assert_eq!(c.max_retries, 3);
    assert_eq!(c.initial_delay_ms, 100);
    assert_eq!(c.max_delay_ms, 30000);
    assert_eq!(c.strategy, RetryStrategy::Fixed);
}

// ── Edge cases ──────────────────────────────────────────────────────

#[test]
fn zero_initial_delay() {
    let c = RetryConfig {
        max_retries: 3,
        initial_delay_ms: 0,
        max_delay_ms: 0,
        strategy: RetryStrategy::Exponential,
        multiplier: 2.0,
    };
    assert_eq!(Retry::calculate_delay(5, &c), Duration::ZERO);
}

// ── Property tests ──────────────────────────────────────────────────

proptest! {
    #[test]
    fn fixed_delay_independent_of_attempt(
        initial in 0u64..10_000,
        attempt in 0u32..100,
    ) {
        let c = RetryConfig {
            max_retries: 100,
            initial_delay_ms: initial,
            max_delay_ms: u64::MAX,
            strategy: RetryStrategy::Fixed,
            multiplier: 2.0,
        };
        prop_assert_eq!(
            Retry::calculate_delay(attempt, &c),
            Duration::from_millis(initial)
        );
    }

    #[test]
    fn exponential_delay_monotonically_increases(
        initial in 1u64..1000,
        max_d in 1_000_000u64..10_000_000,
    ) {
        let c = RetryConfig {
            max_retries: 10,
            initial_delay_ms: initial,
            max_delay_ms: max_d,
            strategy: RetryStrategy::Exponential,
            multiplier: 2.0,
        };
        let mut prev = Duration::ZERO;
        for attempt in 0..5 {
            let d = Retry::calculate_delay(attempt, &c);
            prop_assert!(d >= prev);
            prev = d;
        }
    }

    #[test]
    fn delay_never_exceeds_max(
        initial in 1u64..1000,
        max_d in 1u64..10_000,
        attempt in 0u32..50,
    ) {
        let c = RetryConfig {
            max_retries: 100,
            initial_delay_ms: initial,
            max_delay_ms: max_d,
            strategy: RetryStrategy::Exponential,
            multiplier: 2.0,
        };
        prop_assert!(Retry::calculate_delay(attempt, &c) <= Duration::from_millis(max_d));
    }

    #[test]
    fn linear_delay_never_exceeds_max(
        initial in 1u64..1000,
        max_d in 1u64..10_000,
        attempt in 0u32..50,
    ) {
        let c = RetryConfig {
            max_retries: 100,
            initial_delay_ms: initial,
            max_delay_ms: max_d,
            strategy: RetryStrategy::Linear,
            multiplier: 2.0,
        };
        prop_assert!(Retry::calculate_delay(attempt, &c) <= Duration::from_millis(max_d));
    }
}
