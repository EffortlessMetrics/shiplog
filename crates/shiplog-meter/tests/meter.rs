use proptest::prelude::*;
use shiplog_meter::{MeterData, MeterRegistry, TimingContext, TimingRecorder};

// ── Property tests ──────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_meter_count_monotonic(events in 1u64..500) {
        let mut m = MeterData::new("p");
        for _ in 0..events {
            m.record();
        }
        prop_assert_eq!(m.count, events);
    }

    #[test]
    fn prop_meter_record_many(batches in proptest::collection::vec(1u64..100, 1..20)) {
        let mut m = MeterData::new("p");
        let expected: u64 = batches.iter().sum();
        for &b in &batches {
            m.record_many(b);
        }
        prop_assert_eq!(m.count, expected);
    }

    #[test]
    fn prop_rate_per_minute_is_60x_per_second(count in 1u64..1000) {
        let mut m = MeterData::new("p");
        m.record_many(count);
        let rps = m.rate_per_second();
        let rpm = m.rate_per_minute();
        // rate_per_minute = rate_per_second * 60
        prop_assert!((rpm - rps * 60.0).abs() < 1e-6);
    }

    #[test]
    fn prop_rate_per_hour_is_60x_per_minute(count in 1u64..1000) {
        let mut m = MeterData::new("p");
        m.record_many(count);
        let rpm = m.rate_per_minute();
        let rph = m.rate_per_hour();
        prop_assert!((rph - rpm * 60.0).abs() < 1e-6);
    }

    #[test]
    fn prop_timing_recorder_count(
        values in proptest::collection::vec(0.0_f64..100.0, 1..100),
    ) {
        let mut rec = TimingRecorder::new();
        for &v in &values {
            rec.record("op", v);
        }
        let stats = rec.stats("op").unwrap();
        prop_assert_eq!(stats.count, values.len() as u64);
    }

    #[test]
    fn prop_timing_stats_sum(
        values in proptest::collection::vec(0.0_f64..100.0, 1..100),
    ) {
        let mut rec = TimingRecorder::new();
        let expected_sum: f64 = values.iter().sum();
        for &v in &values {
            rec.record("op", v);
        }
        let stats = rec.stats("op").unwrap();
        prop_assert!((stats.sum - expected_sum).abs() < 1e-6);
    }

    #[test]
    fn prop_timing_p50_le_p95_le_p99(
        values in proptest::collection::vec(0.0_f64..1000.0, 2..200),
    ) {
        let mut rec = TimingRecorder::new();
        for &v in &values {
            rec.record("op", v);
        }
        let stats = rec.stats("op").unwrap();
        prop_assert!(stats.p50 <= stats.p95, "p50 {} > p95 {}", stats.p50, stats.p95);
        prop_assert!(stats.p95 <= stats.p99, "p95 {} > p99 {}", stats.p95, stats.p99);
    }
}

// ── Known-answer tests ──────────────────────────────────────────────────────

#[test]
fn known_answer_meter_count() {
    let mut m = MeterData::new("api_calls");
    m.record();
    m.record();
    m.record_many(8);
    assert_eq!(m.count, 10);
}

#[test]
fn known_answer_timing_recorder_stats() {
    let mut rec = TimingRecorder::new();
    for i in 1..=100 {
        rec.record("latency", i as f64);
    }
    let stats = rec.stats("latency").unwrap();
    assert_eq!(stats.count, 100);
    assert!((stats.sum - 5050.0).abs() < f64::EPSILON);
    assert!((stats.mean - 50.5).abs() < f64::EPSILON);
    assert_eq!(stats.min, Some(1.0));
    assert_eq!(stats.max, Some(100.0));
}

// ── Edge cases ──────────────────────────────────────────────────────────────

#[test]
fn edge_meter_fresh_rate() {
    let m = MeterData::new("fresh");
    assert_eq!(m.count, 0);
    // rate should be 0 when elapsed is 0
    assert_eq!(m.rate_per_second(), 0.0);
}

#[test]
fn edge_meter_reset() {
    let mut m = MeterData::new("reset_test");
    m.record_many(100);
    m.reset();
    assert_eq!(m.count, 0);
}

#[test]
fn edge_timing_context_elapsed() {
    let ctx = TimingContext::new("test_op");
    // elapsed should be non-negative
    assert!(ctx.elapsed_secs() >= 0.0);
    assert!(ctx.elapsed_millis() < 1000); // should be fast
}

#[test]
fn edge_timing_recorder_empty_name() {
    let rec = TimingRecorder::new();
    assert!(rec.stats("nonexistent").is_none());
}

#[test]
fn edge_timing_recorder_single_value() {
    let mut rec = TimingRecorder::new();
    rec.record("op", 42.0);
    let stats = rec.stats("op").unwrap();
    assert_eq!(stats.count, 1);
    assert_eq!(stats.mean, 42.0);
    assert_eq!(stats.min, Some(42.0));
    assert_eq!(stats.max, Some(42.0));
}

#[test]
fn edge_timing_recorder_clear() {
    let mut rec = TimingRecorder::new();
    rec.record("op1", 1.0);
    rec.record("op2", 2.0);
    rec.clear();
    assert!(rec.stats("op1").is_none());
    assert!(rec.stats("op2").is_none());
    assert!(rec.names().is_empty());
}

// ── Registry tests ──────────────────────────────────────────────────────────

#[test]
fn registry_auto_creates() {
    let mut reg = MeterRegistry::new();
    reg.record("new_meter");
    assert_eq!(reg.get("new_meter").unwrap().count, 1);
}

#[test]
fn registry_missing_returns_none() {
    let reg = MeterRegistry::new();
    assert!(reg.get("nope").is_none());
}

#[test]
fn registry_clear() {
    let mut reg = MeterRegistry::new();
    reg.record("a");
    reg.record("b");
    reg.clear();
    assert!(reg.get("a").is_none());
}

#[test]
fn registry_record_many() {
    let mut reg = MeterRegistry::new();
    reg.record_many("batch", 50);
    assert_eq!(reg.get("batch").unwrap().count, 50);
}

#[test]
fn registry_get_rates() {
    let mut reg = MeterRegistry::new();
    reg.record("x");
    let rates = reg.get_rates();
    assert!(rates.contains_key("x"));
}

#[test]
fn registry_multiple_names() {
    let mut reg = MeterRegistry::new();
    reg.record("a");
    reg.record("b");
    reg.record("c");
    assert_eq!(reg.get_all().len(), 3);
}

// ── Serde round-trip ────────────────────────────────────────────────────────

#[test]
fn meter_data_serde_round_trip() {
    let mut m = MeterData::new("serde_test");
    m.record_many(42);
    let json = serde_json::to_string(&m).unwrap();
    let m2: MeterData = serde_json::from_str(&json).unwrap();
    assert_eq!(m2.count, 42);
    assert_eq!(m2.name, "serde_test");
}
