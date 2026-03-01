use chrono::Duration;
use proptest::prelude::*;
use shiplog_circuitbreaker::*;

// ── State machine: Closed → Open → HalfOpen → Closed ───────────────

#[test]
fn initial_state_is_closed() {
    let mut cb = CircuitBreaker::new(CircuitBreakerConfig::new(3, 2, Duration::seconds(30)));
    assert!(cb.is_available("svc"));
    assert_eq!(cb.get_state_enum("svc"), CircuitState::Closed);
}

#[test]
fn closed_to_open_on_failure_threshold() {
    let mut cb = CircuitBreaker::new(CircuitBreakerConfig::new(3, 2, Duration::seconds(30)));
    cb.record_failure("svc");
    cb.record_failure("svc");
    assert_eq!(cb.get_state_enum("svc"), CircuitState::Closed);
    cb.record_failure("svc");
    assert_eq!(cb.get_state_enum("svc"), CircuitState::Open);
    assert!(!cb.is_available("svc"));
}

#[test]
fn open_to_half_open_after_timeout() {
    let mut cb = CircuitBreaker::new(CircuitBreakerConfig::new(2, 2, Duration::milliseconds(50)));
    cb.record_failure("svc");
    cb.record_failure("svc");
    assert!(!cb.is_available("svc"));

    std::thread::sleep(std::time::Duration::from_millis(80));

    assert!(cb.is_available("svc"));
    assert_eq!(cb.get_state_enum("svc"), CircuitState::HalfOpen);
}

#[test]
fn half_open_to_closed_on_success_threshold() {
    let mut cb = CircuitBreaker::new(CircuitBreakerConfig::new(2, 2, Duration::milliseconds(50)));
    cb.record_failure("svc");
    cb.record_failure("svc");

    std::thread::sleep(std::time::Duration::from_millis(80));
    cb.is_available("svc"); // trigger transition to HalfOpen

    cb.record_success("svc");
    assert_eq!(cb.get_state_enum("svc"), CircuitState::HalfOpen);
    cb.record_success("svc");
    assert_eq!(cb.get_state_enum("svc"), CircuitState::Closed);
}

#[test]
fn half_open_to_open_on_failure() {
    let mut cb = CircuitBreaker::new(CircuitBreakerConfig::new(2, 3, Duration::milliseconds(50)));
    cb.record_failure("svc");
    cb.record_failure("svc");

    std::thread::sleep(std::time::Duration::from_millis(80));
    cb.is_available("svc");
    assert_eq!(cb.get_state_enum("svc"), CircuitState::HalfOpen);

    cb.record_failure("svc");
    assert_eq!(cb.get_state_enum("svc"), CircuitState::Open);
}

// ── Success resets failure count when closed ─────────────────────────

#[test]
fn success_resets_failure_count_while_closed() {
    let mut cb = CircuitBreaker::new(CircuitBreakerConfig::new(3, 1, Duration::seconds(30)));
    cb.record_failure("svc");
    cb.record_failure("svc");
    cb.record_success("svc");
    // failure count reset; one more failure shouldn't open
    cb.record_failure("svc");
    assert_eq!(cb.get_state_enum("svc"), CircuitState::Closed);
}

// ── Independent circuits ────────────────────────────────────────────

#[test]
fn circuits_are_independent() {
    let mut cb = CircuitBreaker::new(CircuitBreakerConfig::new(1, 1, Duration::seconds(30)));
    cb.record_failure("a");
    assert!(!cb.is_available("a"));
    assert!(cb.is_available("b"));
}

// ── Reset / remove / clear ──────────────────────────────────────────

#[test]
fn reset_returns_to_closed() {
    let mut cb = CircuitBreaker::new(CircuitBreakerConfig::new(1, 1, Duration::seconds(30)));
    cb.record_failure("svc");
    assert!(!cb.is_available("svc"));
    cb.reset("svc");
    assert!(cb.is_available("svc"));
}

#[test]
fn remove_deletes_circuit() {
    let mut cb = CircuitBreaker::new(CircuitBreakerConfig::new(1, 1, Duration::seconds(30)));
    cb.record_failure("svc");
    assert_eq!(cb.len(), 1);
    cb.remove("svc");
    assert_eq!(cb.len(), 0);
    assert_eq!(cb.get_state_enum("svc"), CircuitState::Closed);
}

#[test]
fn clear_removes_all_circuits() {
    let mut cb = CircuitBreaker::new(CircuitBreakerConfig::new(1, 1, Duration::seconds(30)));
    cb.record_failure("a");
    cb.record_failure("b");
    assert_eq!(cb.len(), 2);
    cb.clear();
    assert!(cb.is_empty());
}

// ── get_state_info ──────────────────────────────────────────────────

#[test]
fn get_state_info_returns_none_for_unknown() {
    let cb = CircuitBreaker::new(CircuitBreakerConfig::strict());
    assert!(cb.get_state_info("unknown").is_none());
}

#[test]
fn get_state_info_returns_details() {
    let mut cb = CircuitBreaker::new(CircuitBreakerConfig::strict());
    cb.record_failure("svc");
    let info = cb.get_state_info("svc").unwrap();
    assert_eq!(info.failure_count, 1);
    assert!(info.last_failure.is_some());
}

// ── Preset configs ──────────────────────────────────────────────────

#[test]
fn strict_and_lenient_configs() {
    let s = CircuitBreakerConfig::strict();
    assert_eq!(s.failure_threshold, 3);
    assert_eq!(s.success_threshold, 2);

    let l = CircuitBreakerConfig::lenient();
    assert_eq!(l.failure_threshold, 10);
    assert_eq!(l.success_threshold, 5);
}

// ── Error type ──────────────────────────────────────────────────────

#[test]
fn error_display() {
    let e = CircuitBreakerError::new("open", "svc");
    let s = format!("{e}");
    assert!(s.contains("svc"));
    assert!(s.contains("open"));
}

// ── Edge cases ──────────────────────────────────────────────────────

#[test]
fn threshold_one_opens_immediately() {
    let mut cb = CircuitBreaker::new(CircuitBreakerConfig::new(1, 1, Duration::seconds(30)));
    cb.record_failure("svc");
    assert_eq!(cb.get_state_enum("svc"), CircuitState::Open);
}

#[test]
fn success_in_open_state_is_noop() {
    let mut cb = CircuitBreaker::new(CircuitBreakerConfig::new(1, 1, Duration::seconds(30)));
    cb.record_failure("svc");
    cb.record_success("svc"); // should not crash or change state
    assert_eq!(cb.get_state_enum("svc"), CircuitState::Open);
}

// ── Property tests ──────────────────────────────────────────────────

proptest! {
    #[test]
    fn failures_below_threshold_stay_closed(
        threshold in 2u32..50,
        failures in 0u32..50,
    ) {
        let mut cb = CircuitBreaker::new(
            CircuitBreakerConfig::new(threshold, 1, Duration::seconds(600)),
        );
        for _ in 0..failures.min(threshold - 1) {
            cb.record_failure("svc");
        }
        if failures < threshold {
            prop_assert_eq!(cb.get_state_enum("svc"), CircuitState::Closed);
        }
    }

    #[test]
    fn exact_threshold_opens(threshold in 1u32..50) {
        let mut cb = CircuitBreaker::new(
            CircuitBreakerConfig::new(threshold, 1, Duration::seconds(600)),
        );
        for _ in 0..threshold {
            cb.record_failure("svc");
        }
        prop_assert_eq!(cb.get_state_enum("svc"), CircuitState::Open);
    }
}
