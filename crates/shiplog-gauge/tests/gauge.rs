use proptest::prelude::*;
use shiplog_gauge::{AverageGauge, ClampedGauge, Gauge, GaugeRegistry};

// ── Property tests ──────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_gauge_set_returns_last_value(values in proptest::collection::vec(-1e12_f64..1e12, 1..100)) {
        let mut g = Gauge::new("p");
        for &v in &values {
            g.set(v);
        }
        prop_assert_eq!(g.value(), *values.last().unwrap());
    }

    #[test]
    fn prop_min_le_value_le_max(values in proptest::collection::vec(-1e6_f64..1e6, 1..200)) {
        let mut g = Gauge::new("p");
        for &v in &values {
            g.set(v);
        }
        prop_assert!(g.min() <= g.value());
        prop_assert!(g.value() <= g.max());
    }

    #[test]
    fn prop_min_max_track_extremes(values in proptest::collection::vec(-1e6_f64..1e6, 1..200)) {
        let mut g = Gauge::new("p");
        for &v in &values {
            g.set(v);
        }
        let expected_min = values.iter().cloned().reduce(f64::min).unwrap();
        let expected_max = values.iter().cloned().reduce(f64::max).unwrap();
        prop_assert_eq!(g.min(), expected_min);
        prop_assert_eq!(g.max(), expected_max);
    }

    #[test]
    fn prop_inc_dec_is_additive(base in -1e6_f64..1e6, inc in 0.0_f64..1e6, dec in 0.0_f64..1e6) {
        let mut g = Gauge::new("p");
        g.set(base);
        g.inc(inc);
        g.dec(dec);
        let expected = base + inc - dec;
        prop_assert!((g.value() - expected).abs() < 1e-6);
    }

    #[test]
    fn prop_clamped_gauge_always_within_bounds(
        min_val in -1000.0_f64..0.0,
        max_val in 1.0_f64..1000.0,
        values in proptest::collection::vec(-1e6_f64..1e6, 1..100),
    ) {
        let mut cg = ClampedGauge::new("p", min_val, max_val);
        for &v in &values {
            cg.set(v);
            prop_assert!(cg.value() >= min_val);
            prop_assert!(cg.value() <= max_val);
        }
    }

    #[test]
    fn prop_average_gauge_average_within_range(values in proptest::collection::vec(-1e6_f64..1e6, 1..200)) {
        let mut ag = AverageGauge::new("p");
        for &v in &values {
            ag.record(v);
        }
        let min = values.iter().cloned().reduce(f64::min).unwrap();
        let max = values.iter().cloned().reduce(f64::max).unwrap();
        prop_assert!(ag.average() >= min);
        prop_assert!(ag.average() <= max);
    }
}

// ── Known-answer tests ──────────────────────────────────────────────────────

#[test]
fn known_answer_gauge_inc_dec() {
    let mut g = Gauge::new("temp");
    g.set(100.0);
    g.inc(25.0);
    g.dec(10.0);
    assert_eq!(g.value(), 115.0);
    assert_eq!(g.min(), 100.0);
    assert_eq!(g.max(), 125.0);
}

#[test]
fn known_answer_clamped_gauge() {
    let mut cg = ClampedGauge::new("pct", 0.0, 100.0);
    cg.set(50.0);
    assert_eq!(cg.value(), 50.0);
    cg.set(200.0);
    assert_eq!(cg.value(), 100.0);
    cg.set(-50.0);
    assert_eq!(cg.value(), 0.0);
}

#[test]
fn known_answer_average_gauge() {
    let mut ag = AverageGauge::new("r");
    ag.record(10.0);
    ag.record(20.0);
    ag.record(30.0);
    assert!((ag.average() - 20.0).abs() < f64::EPSILON);
    assert_eq!(ag.value(), 30.0);
    assert_eq!(ag.count(), 3);
}

// ── Edge cases ──────────────────────────────────────────────────────────────

#[test]
fn edge_gauge_fresh_min_max() {
    let g = Gauge::new("fresh");
    // No values set yet
    assert_eq!(g.value(), 0.0);
    assert_eq!(g.min(), 0.0);
    assert_eq!(g.max(), 0.0);
}

#[test]
fn edge_gauge_single_value() {
    let mut g = Gauge::new("single");
    g.set(42.0);
    assert_eq!(g.min(), 42.0);
    assert_eq!(g.max(), 42.0);
}

#[test]
fn edge_gauge_all_same_values() {
    let mut g = Gauge::new("same");
    for _ in 0..100 {
        g.set(7.0);
    }
    assert_eq!(g.min(), 7.0);
    assert_eq!(g.max(), 7.0);
    assert_eq!(g.value(), 7.0);
}

#[test]
fn edge_gauge_extreme_values() {
    let mut g = Gauge::new("extreme");
    g.set(f64::MIN);
    g.set(f64::MAX);
    assert_eq!(g.min(), f64::MIN);
    assert_eq!(g.max(), f64::MAX);
}

#[test]
fn edge_gauge_reset_clears_min_max() {
    let mut g = Gauge::new("reset");
    g.set(50.0);
    g.set(100.0);
    g.reset();
    assert_eq!(g.value(), 0.0);
    assert_eq!(g.min(), 0.0);
    assert_eq!(g.max(), 0.0);
}

#[test]
fn edge_clamped_gauge_equal_bounds() {
    let mut cg = ClampedGauge::new("eq", 5.0, 5.0);
    cg.set(10.0);
    assert_eq!(cg.value(), 5.0);
    cg.set(-10.0);
    assert_eq!(cg.value(), 5.0);
}

#[test]
fn edge_average_gauge_empty() {
    let ag = AverageGauge::new("empty");
    assert_eq!(ag.average(), 0.0);
    assert_eq!(ag.value(), 0.0);
    assert_eq!(ag.count(), 0);
}

#[test]
fn edge_average_gauge_single_value() {
    let mut ag = AverageGauge::new("single");
    ag.record(42.0);
    assert_eq!(ag.average(), 42.0);
    assert_eq!(ag.value(), 42.0);
}

#[test]
fn edge_average_gauge_clear() {
    let mut ag = AverageGauge::new("clear");
    ag.record(1.0);
    ag.record(2.0);
    ag.clear();
    assert_eq!(ag.count(), 0);
    assert_eq!(ag.average(), 0.0);
}

// ── Registry tests ──────────────────────────────────────────────────────────

#[test]
fn registry_auto_creates_on_set() {
    let mut reg = GaugeRegistry::new();
    reg.set("new_gauge", 42.0);
    assert_eq!(reg.get("new_gauge").unwrap().value(), 42.0);
}

#[test]
fn registry_auto_creates_on_inc() {
    let mut reg = GaugeRegistry::new();
    reg.inc("g", 5.0);
    assert_eq!(reg.get("g").unwrap().value(), 5.0);
}

#[test]
fn registry_missing_returns_none() {
    let reg = GaugeRegistry::new();
    assert!(reg.get("nonexistent").is_none());
}

#[test]
fn registry_clear_removes_all() {
    let mut reg = GaugeRegistry::new();
    reg.set("a", 1.0);
    reg.set("b", 2.0);
    reg.clear();
    assert!(reg.get("a").is_none());
    assert!(reg.get("b").is_none());
}

#[test]
fn registry_values_snapshot() {
    let mut reg = GaugeRegistry::new();
    reg.set("x", 10.0);
    reg.set("y", 20.0);
    let vals = reg.values();
    assert_eq!(vals.len(), 2);
    assert_eq!(vals["x"], 10.0);
    assert_eq!(vals["y"], 20.0);
}

// ── Snapshot tests ──────────────────────────────────────────────────────────

#[test]
fn snapshot_captures_current_state() {
    let mut g = Gauge::new("snap");
    g.set(10.0);
    g.set(50.0);
    g.set(30.0);
    let snap = g.snapshot();
    assert_eq!(snap.name, "snap");
    assert_eq!(snap.value, 30.0);
    assert_eq!(snap.min, 10.0);
    assert_eq!(snap.max, 50.0);
    assert!(snap.timestamp > 0);
}

// ── Serde round-trip ────────────────────────────────────────────────────────

#[test]
fn gauge_serde_round_trip() {
    let mut g = Gauge::new("serde_test");
    g.set(42.0);
    let json = serde_json::to_string(&g).unwrap();
    let g2: Gauge = serde_json::from_str(&json).unwrap();
    assert_eq!(g2.value(), 42.0);
    assert_eq!(g2.name(), "serde_test");
}

#[test]
fn clamped_gauge_serde_round_trip() {
    let mut cg = ClampedGauge::new("serde_clamped", 0.0, 100.0);
    cg.set(75.0);
    let json = serde_json::to_string(&cg).unwrap();
    let cg2: ClampedGauge = serde_json::from_str(&json).unwrap();
    assert_eq!(cg2.value(), 75.0);
}
