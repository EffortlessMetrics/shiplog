use proptest::prelude::*;
use shiplog_percentile::{
    Percentile, PercentileSet, StreamingPercentile, calculate_percentile, calculate_percentiles,
};

// ── Property tests ──────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_percentile_0_is_min(values in proptest::collection::vec(0.0_f64..1000.0, 2..200)) {
        let result = calculate_percentile(&values, 0.0);
        let min = values.iter().cloned().reduce(f64::min).unwrap();
        prop_assert_eq!(result, min);
    }

    #[test]
    fn prop_percentile_100_is_max(values in proptest::collection::vec(0.0_f64..1000.0, 2..200)) {
        let result = calculate_percentile(&values, 100.0);
        let max = values.iter().cloned().reduce(f64::max).unwrap();
        prop_assert_eq!(result, max);
    }

    #[test]
    fn prop_percentile_monotonic(values in proptest::collection::vec(0.0_f64..1000.0, 5..200)) {
        let p25 = calculate_percentile(&values, 25.0);
        let p50 = calculate_percentile(&values, 50.0);
        let p75 = calculate_percentile(&values, 75.0);
        let p100 = calculate_percentile(&values, 100.0);
        prop_assert!(p25 <= p50, "p25 {} > p50 {}", p25, p50);
        prop_assert!(p50 <= p75, "p50 {} > p75 {}", p50, p75);
        prop_assert!(p75 <= p100, "p75 {} > p100 {}", p75, p100);
    }

    #[test]
    fn prop_percentile_within_data_range(
        values in proptest::collection::vec(0.0_f64..1000.0, 1..200),
        p in 0.0_f64..100.0,
    ) {
        let result = calculate_percentile(&values, p);
        let min = values.iter().cloned().reduce(f64::min).unwrap();
        let max = values.iter().cloned().reduce(f64::max).unwrap();
        prop_assert!(result >= min);
        prop_assert!(result <= max);
    }

    #[test]
    fn prop_percentile_set_count(values in proptest::collection::vec(0.0_f64..1000.0, 1..200)) {
        let mut ps = PercentileSet::new("p");
        ps.record_many(values.iter().cloned());
        prop_assert_eq!(ps.count(), values.len());
    }

    #[test]
    fn prop_percentile_set_median_within_range(values in proptest::collection::vec(0.0_f64..1000.0, 1..200)) {
        let mut ps = PercentileSet::new("p");
        ps.record_many(values.iter().cloned());
        let min = values.iter().cloned().reduce(f64::min).unwrap();
        let max = values.iter().cloned().reduce(f64::max).unwrap();
        let median = ps.median();
        prop_assert!(median >= min);
        prop_assert!(median <= max);
    }

    #[test]
    fn prop_percentile_set_p95_ge_median(values in proptest::collection::vec(0.0_f64..1000.0, 5..200)) {
        let mut ps = PercentileSet::new("p");
        ps.record_many(values.iter().cloned());
        prop_assert!(ps.p95() >= ps.median());
    }

    #[test]
    fn prop_percentile_set_p99_ge_p95(values in proptest::collection::vec(0.0_f64..1000.0, 5..200)) {
        let mut ps = PercentileSet::new("p");
        ps.record_many(values.iter().cloned());
        prop_assert!(ps.p99() >= ps.p95());
    }

    #[test]
    fn prop_all_same_values_all_percentiles_equal(val in 0.0_f64..1000.0, n in 2usize..100) {
        let values: Vec<f64> = vec![val; n];
        let p25 = calculate_percentile(&values, 25.0);
        let p50 = calculate_percentile(&values, 50.0);
        let p75 = calculate_percentile(&values, 75.0);
        prop_assert_eq!(p25, val);
        prop_assert_eq!(p50, val);
        prop_assert_eq!(p75, val);
    }

    #[test]
    fn prop_streaming_percentile_count(values in proptest::collection::vec(0.0_f64..1000.0, 1..200)) {
        let mut sp = StreamingPercentile::new(0.5, 1000);
        for &v in &values {
            sp.record(v);
        }
        prop_assert_eq!(sp.count(), values.len() as u64);
    }
}

// ── Known-answer tests ──────────────────────────────────────────────────────

#[test]
fn known_answer_percentile_1_to_100() {
    let values: Vec<f64> = (1..=100).map(|i| i as f64).collect();
    assert_eq!(calculate_percentile(&values, 0.0), 1.0);
    assert_eq!(calculate_percentile(&values, 50.0), 50.0);
    assert_eq!(calculate_percentile(&values, 100.0), 100.0);
}

#[test]
fn known_answer_percentile_5_values() {
    let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    assert_eq!(calculate_percentile(&values, 0.0), 1.0);
    assert_eq!(calculate_percentile(&values, 25.0), 2.0);
    assert_eq!(calculate_percentile(&values, 50.0), 3.0);
    assert_eq!(calculate_percentile(&values, 75.0), 4.0);
    assert_eq!(calculate_percentile(&values, 100.0), 5.0);
}

#[test]
fn known_answer_percentile_set() {
    let mut ps = PercentileSet::new("lat");
    for i in 1..=100 {
        ps.record(i as f64);
    }
    assert_eq!(ps.median(), 50.0);
    assert_eq!(ps.p95(), 95.0);
    assert_eq!(ps.p99(), 99.0);
}

#[test]
fn known_answer_calculate_percentiles_multiple() {
    let values = vec![10.0, 20.0, 30.0, 40.0, 50.0];
    let result = calculate_percentiles(&values, &[0.0, 50.0, 100.0]);
    assert_eq!(result.len(), 3);
    assert_eq!(result[0], (0.0, 10.0));
    assert_eq!(result[1], (50.0, 30.0));
    assert_eq!(result[2], (100.0, 50.0));
}

#[test]
fn known_answer_percentile_struct() {
    let mut p = Percentile::new("p95", 0.95);
    for i in 1..=100 {
        p.record(i as f64);
    }
    assert_eq!(p.count(), 100);
    assert_eq!(p.quantile(), 0.95);
    // Should be near the 95th value
    assert!(p.get() >= 90.0);
}

// ── Edge cases ──────────────────────────────────────────────────────────────

#[test]
fn edge_percentile_empty_slice() {
    assert_eq!(calculate_percentile(&[], 50.0), 0.0);
}

#[test]
fn edge_percentile_single_value() {
    assert_eq!(calculate_percentile(&[42.0], 0.0), 42.0);
    assert_eq!(calculate_percentile(&[42.0], 50.0), 42.0);
    assert_eq!(calculate_percentile(&[42.0], 100.0), 42.0);
}

#[test]
fn edge_percentile_two_values() {
    let values = vec![10.0, 20.0];
    assert_eq!(calculate_percentile(&values, 0.0), 10.0);
    assert_eq!(calculate_percentile(&values, 100.0), 20.0);
}

#[test]
fn edge_percentile_all_same() {
    let values = vec![7.0; 100];
    assert_eq!(calculate_percentile(&values, 25.0), 7.0);
    assert_eq!(calculate_percentile(&values, 50.0), 7.0);
    assert_eq!(calculate_percentile(&values, 99.0), 7.0);
}

#[test]
fn edge_percentile_extreme_outlier() {
    let mut values: Vec<f64> = vec![1.0; 99];
    values.push(1_000_000.0);
    // p100 should be the outlier
    assert_eq!(calculate_percentile(&values, 100.0), 1_000_000.0);
    // p99 with 100 values: index = (99/100) * 99 = 98 → 1.0 (floor-based)
    assert_eq!(calculate_percentile(&values, 99.0), 1.0);
}

#[test]
fn edge_percentile_struct_empty() {
    let p = Percentile::new("empty", 0.5);
    assert_eq!(p.get(), 0.0);
    assert_eq!(p.count(), 0);
}

#[test]
fn edge_percentile_struct_reset() {
    let mut p = Percentile::new("reset", 0.5);
    p.record(1.0);
    p.record(2.0);
    p.reset();
    assert_eq!(p.count(), 0);
    assert_eq!(p.get(), 0.0);
}

#[test]
fn edge_percentile_set_empty() {
    let ps = PercentileSet::new("empty");
    assert_eq!(ps.median(), 0.0);
    assert_eq!(ps.p95(), 0.0);
    assert_eq!(ps.p99(), 0.0);
    assert_eq!(ps.count(), 0);
}

#[test]
fn edge_percentile_set_reset() {
    let mut ps = PercentileSet::new("reset");
    ps.record(1.0);
    ps.record(2.0);
    ps.reset();
    assert_eq!(ps.count(), 0);
    assert_eq!(ps.median(), 0.0);
}

#[test]
fn edge_percentile_set_single_value() {
    let mut ps = PercentileSet::new("single");
    ps.record(42.0);
    assert_eq!(ps.median(), 42.0);
    assert_eq!(ps.p95(), 42.0);
    assert_eq!(ps.p99(), 42.0);
}

#[test]
fn edge_streaming_percentile_empty() {
    let sp = StreamingPercentile::new(0.5, 100);
    assert_eq!(sp.get(), 0.0);
    assert_eq!(sp.count(), 0);
}

#[test]
fn edge_streaming_percentile_single() {
    let mut sp = StreamingPercentile::new(0.5, 100);
    sp.record(42.0);
    assert_eq!(sp.get(), 42.0);
}

#[test]
fn edge_percentile_set_custom_percentiles() {
    let mut ps = PercentileSet::with_percentiles("custom", vec![10.0, 90.0]);
    for i in 1..=100 {
        ps.record(i as f64);
    }
    let results = ps.get_percentiles();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].0, 10.0);
    assert_eq!(results[1].0, 90.0);
    // p10 should be around 10, p90 around 90
    assert!(results[0].1 >= 1.0 && results[0].1 <= 20.0);
    assert!(results[1].1 >= 80.0 && results[1].1 <= 100.0);
}

#[test]
fn edge_percentile_struct_quantile_clamped() {
    // quantile > 1.0 should be clamped to 1.0
    let p = Percentile::new("clamp", 2.0);
    assert_eq!(p.quantile(), 1.0);

    // quantile < 0.0 should be clamped to 0.0
    let p2 = Percentile::new("clamp_neg", -1.0);
    assert_eq!(p2.quantile(), 0.0);
}
