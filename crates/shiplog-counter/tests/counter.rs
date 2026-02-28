use proptest::prelude::*;
use shiplog_counter::{BoundedCounter, Counter, CounterRegistry, DeltaCounter, WrappingCounter};

// ── Property tests ──────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_counter_inc_monotonic(n in 1u64..10_000) {
        let mut c = Counter::new("p");
        for _ in 0..n {
            c.inc();
        }
        prop_assert_eq!(c.value(), n);
    }

    #[test]
    fn prop_counter_inc_by_additive(amounts in proptest::collection::vec(0u64..1000, 1..50)) {
        let mut c = Counter::new("p");
        let expected: u64 = amounts.iter().sum();
        for &a in &amounts {
            c.inc_by(a);
        }
        prop_assert_eq!(c.value(), expected);
    }

    #[test]
    fn prop_wrapping_counter_always_le_max(max_val in 1u64..1000, n in 1u64..5000) {
        let mut wc = WrappingCounter::new("p", max_val);
        for _ in 0..n {
            wc.inc();
        }
        prop_assert!(wc.value() <= max_val);
    }

    #[test]
    fn prop_bounded_counter_never_exceeds_max(max_val in 1u64..1000, n in 1u64..5000) {
        let mut bc = BoundedCounter::new("p", max_val);
        for _ in 0..n {
            bc.inc();
        }
        prop_assert!(bc.value() <= max_val);
    }

    #[test]
    fn prop_bounded_counter_try_inc_returns_false_at_max(max_val in 1u64..100) {
        let mut bc = BoundedCounter::new("p", max_val);
        for _ in 0..max_val {
            prop_assert!(bc.try_inc());
        }
        prop_assert!(!bc.try_inc());
        prop_assert!(bc.is_maxed());
    }

    #[test]
    fn prop_delta_counter_snapshot_consistency(
        amounts in proptest::collection::vec(1u64..100, 1..20),
    ) {
        let mut dc = DeltaCounter::new("p");
        let mut total_snapped = 0u64;
        for &a in &amounts {
            dc.inc_by(a);
            total_snapped += dc.snapshot();
        }
        prop_assert_eq!(total_snapped, dc.value());
        prop_assert_eq!(dc.delta(), 0);
    }

    #[test]
    fn prop_registry_total_is_sum(entries in proptest::collection::vec((1u64..100,), 1..20)) {
        let mut reg = CounterRegistry::new();
        let mut expected_total = 0u64;
        for (i, (amount,)) in entries.iter().enumerate() {
            let name = format!("c_{}", i);
            reg.inc_by(&name, *amount);
            expected_total += amount;
        }
        prop_assert_eq!(reg.total(), expected_total);
    }
}

// ── Known-answer tests ──────────────────────────────────────────────────────

#[test]
fn known_answer_counter() {
    let mut c = Counter::new("requests");
    c.inc();
    c.inc();
    c.inc_by(8);
    assert_eq!(c.value(), 10);
    assert_eq!(c.name(), "requests");
}

#[test]
fn known_answer_wrapping_counter() {
    let mut wc = WrappingCounter::new("wrap", 3);
    wc.inc(); // 1
    wc.inc(); // 2
    wc.inc(); // 3
    assert_eq!(wc.value(), 3);
    wc.inc(); // wraps to 0
    assert_eq!(wc.value(), 0);
    wc.inc(); // 1
    assert_eq!(wc.value(), 1);
}

#[test]
fn known_answer_bounded_counter() {
    let mut bc = BoundedCounter::new("bounded", 5);
    for _ in 0..10 {
        bc.inc();
    }
    assert_eq!(bc.value(), 5);
    assert!(bc.is_maxed());
}

#[test]
fn known_answer_delta_counter() {
    let mut dc = DeltaCounter::new("delta");
    dc.inc_by(10);
    assert_eq!(dc.delta(), 10);
    let snap1 = dc.snapshot();
    assert_eq!(snap1, 10);
    assert_eq!(dc.delta(), 0);

    dc.inc_by(5);
    assert_eq!(dc.delta(), 5);
    let snap2 = dc.snapshot();
    assert_eq!(snap2, 5);
    assert_eq!(dc.value(), 15);
}

#[test]
fn known_answer_registry_total() {
    let mut reg = CounterRegistry::new();
    reg.inc_by("a", 10);
    reg.inc_by("b", 20);
    reg.inc_by("c", 30);
    assert_eq!(reg.total(), 60);
}

// ── Edge cases ──────────────────────────────────────────────────────────────

#[test]
fn edge_counter_fresh() {
    let c = Counter::new("fresh");
    assert_eq!(c.value(), 0);
    assert!(c.elapsed() >= 0);
}

#[test]
fn edge_counter_reset() {
    let mut c = Counter::new("reset");
    c.inc_by(100);
    c.reset();
    assert_eq!(c.value(), 0);
}

#[test]
fn edge_counter_inc_by_zero() {
    let mut c = Counter::new("zero");
    c.inc_by(0);
    assert_eq!(c.value(), 0);
}

#[test]
fn edge_wrapping_counter_max_1() {
    let mut wc = WrappingCounter::new("tiny", 1);
    wc.inc(); // 1
    assert_eq!(wc.value(), 1);
    wc.inc(); // wraps to 0
    assert_eq!(wc.value(), 0);
}

#[test]
fn edge_wrapping_counter_reset() {
    let mut wc = WrappingCounter::new("reset", 10);
    wc.inc();
    wc.inc();
    wc.reset();
    assert_eq!(wc.value(), 0);
}

#[test]
fn edge_bounded_counter_max_0() {
    let mut bc = BoundedCounter::new("zero_max", 0);
    assert!(!bc.try_inc());
    assert!(bc.is_maxed());
    assert_eq!(bc.value(), 0);
}

#[test]
fn edge_bounded_counter_reset() {
    let mut bc = BoundedCounter::new("reset", 10);
    for _ in 0..10 {
        bc.inc();
    }
    assert!(bc.is_maxed());
    bc.reset();
    assert_eq!(bc.value(), 0);
    assert!(!bc.is_maxed());
}

#[test]
fn edge_delta_counter_no_changes() {
    let dc = DeltaCounter::new("empty");
    assert_eq!(dc.delta(), 0);
    assert_eq!(dc.value(), 0);
}

#[test]
fn edge_delta_counter_reset() {
    let mut dc = DeltaCounter::new("reset");
    dc.inc_by(50);
    dc.snapshot();
    dc.inc_by(10);
    dc.reset();
    assert_eq!(dc.value(), 0);
    assert_eq!(dc.delta(), 0);
}

// ── Registry edge cases ─────────────────────────────────────────────────────

#[test]
fn registry_empty() {
    let reg = CounterRegistry::new();
    assert_eq!(reg.total(), 0);
    assert!(reg.get("x").is_none());
}

#[test]
fn registry_clear() {
    let mut reg = CounterRegistry::new();
    reg.inc("a");
    reg.inc("b");
    reg.clear();
    assert_eq!(reg.total(), 0);
    assert!(reg.get("a").is_none());
}

#[test]
fn registry_auto_creates_on_inc() {
    let mut reg = CounterRegistry::new();
    reg.inc("new_counter");
    assert_eq!(reg.get("new_counter").unwrap().value(), 1);
}

#[test]
fn registry_multiple_increments_same_key() {
    let mut reg = CounterRegistry::new();
    for _ in 0..100 {
        reg.inc("hits");
    }
    assert_eq!(reg.get("hits").unwrap().value(), 100);
}

// ── Serde round-trip ────────────────────────────────────────────────────────

#[test]
fn counter_serde_round_trip() {
    let mut c = Counter::new("serde");
    c.inc_by(42);
    let json = serde_json::to_string(&c).unwrap();
    let c2: Counter = serde_json::from_str(&json).unwrap();
    assert_eq!(c2.value(), 42);
    assert_eq!(c2.name(), "serde");
}

#[test]
fn wrapping_counter_serde_round_trip() {
    let mut wc = WrappingCounter::new("serde_wrap", 100);
    wc.inc();
    wc.inc();
    let json = serde_json::to_string(&wc).unwrap();
    let wc2: WrappingCounter = serde_json::from_str(&json).unwrap();
    assert_eq!(wc2.value(), 2);
    assert_eq!(wc2.max(), 100);
}

#[test]
fn bounded_counter_serde_round_trip() {
    let mut bc = BoundedCounter::new("serde_bounded", 50);
    bc.inc();
    let json = serde_json::to_string(&bc).unwrap();
    let bc2: BoundedCounter = serde_json::from_str(&json).unwrap();
    assert_eq!(bc2.value(), 1);
    assert_eq!(bc2.max(), 50);
}
