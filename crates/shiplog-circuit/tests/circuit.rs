use chrono::Duration;
use proptest::prelude::*;
use shiplog_circuit::*;

// ── Full state machine cycle ────────────────────────────────────────

#[test]
fn full_cycle_closed_open_halfopen_closed() {
    let mut cb = CircuitBreaker::new(CircuitBreakerConfig::new(2, 2, Duration::milliseconds(50)));

    // Closed → Open
    assert_eq!(cb.get_state_enum("svc"), CircuitState::Closed);
    cb.record_failure("svc");
    cb.record_failure("svc");
    assert_eq!(cb.get_state_enum("svc"), CircuitState::Open);
    assert!(!cb.is_available("svc"));

    // Open → HalfOpen (after timeout)
    std::thread::sleep(std::time::Duration::from_millis(80));
    assert!(cb.is_available("svc"));
    assert_eq!(cb.get_state_enum("svc"), CircuitState::HalfOpen);

    // HalfOpen → Closed (after success threshold)
    cb.record_success("svc");
    cb.record_success("svc");
    assert_eq!(cb.get_state_enum("svc"), CircuitState::Closed);
}

#[test]
fn half_open_failure_reverts_to_open() {
    let mut cb = CircuitBreaker::new(CircuitBreakerConfig::new(1, 3, Duration::milliseconds(50)));

    cb.record_failure("svc");
    assert_eq!(cb.get_state_enum("svc"), CircuitState::Open);

    std::thread::sleep(std::time::Duration::from_millis(80));
    cb.is_available("svc");
    assert_eq!(cb.get_state_enum("svc"), CircuitState::HalfOpen);

    cb.record_failure("svc");
    assert_eq!(cb.get_state_enum("svc"), CircuitState::Open);
}

// ── Initial state ───────────────────────────────────────────────────

#[test]
fn default_state_is_closed() {
    let mut cb = CircuitBreaker::new(CircuitBreakerConfig::strict());
    assert!(cb.is_available("x"));
    assert_eq!(cb.get_state_enum("x"), CircuitState::Closed);
}

#[test]
fn unknown_circuit_is_closed() {
    let cb = CircuitBreaker::new(CircuitBreakerConfig::strict());
    assert_eq!(cb.get_state_enum("nonexistent"), CircuitState::Closed);
}

// ── Success in closed resets failure count ───────────────────────────

#[test]
fn success_resets_failure_counter() {
    let mut cb = CircuitBreaker::new(CircuitBreakerConfig::new(3, 1, Duration::seconds(30)));
    cb.record_failure("svc");
    cb.record_failure("svc");
    cb.record_success("svc"); // resets
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
fn reset_restores_closed() {
    let mut cb = CircuitBreaker::new(CircuitBreakerConfig::new(1, 1, Duration::seconds(30)));
    cb.record_failure("svc");
    cb.reset("svc");
    assert!(cb.is_available("svc"));
    assert_eq!(cb.get_state_enum("svc"), CircuitState::Closed);
}

#[test]
fn remove_deletes_circuit() {
    let mut cb = CircuitBreaker::new(CircuitBreakerConfig::new(1, 1, Duration::seconds(30)));
    cb.record_failure("svc");
    cb.remove("svc");
    assert!(cb.is_empty());
}

#[test]
fn clear_empties_all() {
    let mut cb = CircuitBreaker::new(CircuitBreakerConfig::new(1, 1, Duration::seconds(30)));
    cb.record_failure("a");
    cb.record_failure("b");
    cb.clear();
    assert!(cb.is_empty());
}

// ── get_state_info ──────────────────────────────────────────────────

#[test]
fn get_state_info_none_for_unknown() {
    let cb = CircuitBreaker::new(CircuitBreakerConfig::strict());
    assert!(cb.get_state_info("x").is_none());
}

#[test]
fn get_state_info_tracks_failure_count() {
    let mut cb = CircuitBreaker::new(CircuitBreakerConfig::strict());
    cb.record_failure("svc");
    cb.record_failure("svc");
    let info = cb.get_state_info("svc").unwrap();
    assert_eq!(info.failure_count, 2);
    assert!(info.last_failure.is_some());
}

// ── Preset configs ──────────────────────────────────────────────────

#[test]
fn preset_configs() {
    let s = CircuitBreakerConfig::strict();
    assert_eq!(s.failure_threshold, 3);
    let l = CircuitBreakerConfig::lenient();
    assert_eq!(l.failure_threshold, 10);
}

// ── Error type ──────────────────────────────────────────────────────

#[test]
fn circuit_error_display() {
    let e = CircuitError::new("msg", "svc");
    let s = format!("{e}");
    assert!(s.contains("svc"));
    assert!(s.contains("msg"));
}

// ── Edge cases ──────────────────────────────────────────────────────

#[test]
fn threshold_one_opens_immediately() {
    let mut cb = CircuitBreaker::new(CircuitBreakerConfig::new(1, 1, Duration::seconds(30)));
    cb.record_failure("svc");
    assert_eq!(cb.get_state_enum("svc"), CircuitState::Open);
}

#[test]
fn success_in_open_noop() {
    let mut cb = CircuitBreaker::new(CircuitBreakerConfig::new(1, 1, Duration::seconds(30)));
    cb.record_failure("svc");
    cb.record_success("svc");
    assert_eq!(cb.get_state_enum("svc"), CircuitState::Open);
}

#[test]
fn rapid_failures_keep_circuit_open() {
    let mut cb = CircuitBreaker::new(CircuitBreakerConfig::new(2, 1, Duration::seconds(30)));
    for _ in 0..100 {
        cb.record_failure("svc");
    }
    assert_eq!(cb.get_state_enum("svc"), CircuitState::Open);
}

// ── Property tests ──────────────────────────────────────────────────

proptest! {
    #[test]
    fn fewer_failures_than_threshold_stay_closed(
        threshold in 2u32..50,
        failures in 0u32..50,
    ) {
        let mut cb = CircuitBreaker::new(
            CircuitBreakerConfig::new(threshold, 1, Duration::seconds(600)),
        );
        let actual_failures = failures.min(threshold - 1);
        for _ in 0..actual_failures {
            cb.record_failure("svc");
        }
        if failures < threshold {
            prop_assert_eq!(cb.get_state_enum("svc"), CircuitState::Closed);
        }
    }

    #[test]
    fn exact_threshold_failures_opens(threshold in 1u32..50) {
        let mut cb = CircuitBreaker::new(
            CircuitBreakerConfig::new(threshold, 1, Duration::seconds(600)),
        );
        for _ in 0..threshold {
            cb.record_failure("svc");
        }
        prop_assert_eq!(cb.get_state_enum("svc"), CircuitState::Open);
    }

    #[test]
    fn len_matches_distinct_keys(n in 1usize..20) {
        let mut cb = CircuitBreaker::new(CircuitBreakerConfig::strict());
        for i in 0..n {
            cb.record_failure(&format!("svc-{i}"));
        }
        prop_assert_eq!(cb.len(), n);
    }
}
