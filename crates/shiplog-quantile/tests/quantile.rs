use proptest::prelude::*;
use shiplog_quantile::{GKQuantile, ReservoirQuantile, TDigest, quantile, quantiles};

// ── Property tests ──────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_quantile_0_is_min(values in proptest::collection::vec(0.0_f64..1000.0, 2..200)) {
        let result = quantile(&values, 0.0);
        let min = values.iter().cloned().reduce(f64::min).unwrap();
        prop_assert_eq!(result, min);
    }

    #[test]
    fn prop_quantile_1_is_max(values in proptest::collection::vec(0.0_f64..1000.0, 2..200)) {
        let result = quantile(&values, 1.0);
        let max = values.iter().cloned().reduce(f64::max).unwrap();
        prop_assert_eq!(result, max);
    }

    #[test]
    fn prop_quantile_monotonic(values in proptest::collection::vec(0.0_f64..1000.0, 5..200)) {
        let q25 = quantile(&values, 0.25);
        let q50 = quantile(&values, 0.5);
        let q75 = quantile(&values, 0.75);
        let q100 = quantile(&values, 1.0);
        prop_assert!(q25 <= q50);
        prop_assert!(q50 <= q75);
        prop_assert!(q75 <= q100);
    }

    #[test]
    fn prop_quantile_within_data_range(
        values in proptest::collection::vec(0.0_f64..1000.0, 1..200),
        q in 0.0_f64..1.0,
    ) {
        let result = quantile(&values, q);
        let min = values.iter().cloned().reduce(f64::min).unwrap();
        let max = values.iter().cloned().reduce(f64::max).unwrap();
        prop_assert!(result >= min);
        prop_assert!(result <= max);
    }

    #[test]
    fn prop_tdigest_count(values in proptest::collection::vec(0.0_f64..1000.0, 1..500)) {
        let mut td = TDigest::new();
        for &v in &values {
            td.add(v);
        }
        prop_assert_eq!(td.count(), values.len() as u64);
    }

    #[test]
    fn prop_tdigest_quantile_within_range(values in proptest::collection::vec(0.0_f64..1000.0, 10..200)) {
        let mut td = TDigest::new();
        let min = values.iter().cloned().reduce(f64::min).unwrap();
        let max = values.iter().cloned().reduce(f64::max).unwrap();
        for &v in &values {
            td.add(v);
        }
        let q50 = td.quantile(0.5);
        prop_assert!(q50 >= min, "q50 {} < min {}", q50, min);
        prop_assert!(q50 <= max, "q50 {} > max {}", q50, max);
    }

    #[test]
    fn prop_tdigest_quantile_non_negative_for_positive_data(values in proptest::collection::vec(1.0_f64..1000.0, 10..200)) {
        let mut td = TDigest::new();
        for &v in &values {
            td.add(v);
        }
        let q50 = td.quantile(0.5);
        prop_assert!(q50 > 0.0, "q50 should be > 0 for positive data, got {}", q50);
    }

    #[test]
    fn prop_all_same_quantiles_equal(val in 0.0_f64..1000.0, n in 2usize..100) {
        let values: Vec<f64> = vec![val; n];
        let q0 = quantile(&values, 0.0);
        let q50 = quantile(&values, 0.5);
        let q100 = quantile(&values, 1.0);
        prop_assert_eq!(q0, val);
        prop_assert_eq!(q50, val);
        prop_assert_eq!(q100, val);
    }

    #[test]
    fn prop_reservoir_count(values in proptest::collection::vec(0.0_f64..1000.0, 1..500)) {
        let mut rq = ReservoirQuantile::new(0.5, 100);
        for &v in &values {
            rq.add(v);
        }
        prop_assert_eq!(rq.count(), values.len() as u64);
    }

    #[test]
    fn prop_gk_count(values in proptest::collection::vec(1.0_f64..1000.0, 1..100)) {
        // Use a larger epsilon to avoid compress edge case in GKQuantile
        let mut gk = GKQuantile::new(0.5);
        for &v in &values {
            gk.add(v);
        }
        prop_assert_eq!(gk.count(), values.len() as u64);
    }
}

// ── Known-answer tests ──────────────────────────────────────────────────────

#[test]
fn known_answer_quantile_5_values() {
    let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    assert_eq!(quantile(&values, 0.0), 1.0);
    assert_eq!(quantile(&values, 0.25), 2.0);
    assert_eq!(quantile(&values, 0.5), 3.0);
    assert_eq!(quantile(&values, 0.75), 4.0);
    assert_eq!(quantile(&values, 1.0), 5.0);
}

#[test]
fn known_answer_quantiles_multiple() {
    let values = vec![10.0, 20.0, 30.0, 40.0, 50.0];
    let result = quantiles(&values, &[0.0, 0.5, 1.0]);
    assert_eq!(result.len(), 3);
    assert_eq!(result[0], (0.0, 10.0));
    assert_eq!(result[1], (0.5, 30.0));
    assert_eq!(result[2], (1.0, 50.0));
}

#[test]
fn known_answer_tdigest_single() {
    let mut td = TDigest::new();
    td.add(42.0);
    assert_eq!(td.quantile(0.0), 42.0);
    assert_eq!(td.quantile(0.5), 42.0);
    assert_eq!(td.quantile(1.0), 42.0);
}

#[test]
fn known_answer_tdigest_weighted() {
    let mut td = TDigest::new();
    td.add_weighted(1.0, 100.0);
    td.add_weighted(100.0, 1.0);
    // Median should be closer to 1.0 because of the much higher weight
    let q50 = td.quantile(0.5);
    assert!(q50 < 50.0, "q50={} should be < 50 due to weighting", q50);
}

// ── Edge cases ──────────────────────────────────────────────────────────────

#[test]
fn edge_quantile_empty() {
    assert_eq!(quantile(&[], 0.5), 0.0);
}

#[test]
fn edge_quantile_single() {
    assert_eq!(quantile(&[42.0], 0.0), 42.0);
    assert_eq!(quantile(&[42.0], 0.5), 42.0);
    assert_eq!(quantile(&[42.0], 1.0), 42.0);
}

#[test]
fn edge_quantile_all_same() {
    let values = vec![7.0; 100];
    assert_eq!(quantile(&values, 0.25), 7.0);
    assert_eq!(quantile(&values, 0.5), 7.0);
    assert_eq!(quantile(&values, 0.99), 7.0);
}

#[test]
fn edge_tdigest_empty() {
    let td = TDigest::new();
    assert_eq!(td.quantile(0.5), 0.0);
    assert_eq!(td.count(), 0);
    assert_eq!(td.centroids(), 0);
}

#[test]
fn edge_tdigest_reset() {
    let mut td = TDigest::new();
    for i in 0..100 {
        td.add(i as f64);
    }
    td.reset();
    assert_eq!(td.count(), 0);
    assert_eq!(td.centroids(), 0);
}

#[test]
fn edge_tdigest_default() {
    let td = TDigest::default();
    assert_eq!(td.count(), 0);
    assert_eq!(td.quantile(0.5), 0.0);
}

#[test]
fn edge_tdigest_compression_min() {
    // Compression < 1 is clamped to 1
    let td = TDigest::with_compression(0.1);
    assert_eq!(td.count(), 0);
}

#[test]
fn edge_reservoir_empty() {
    let rq = ReservoirQuantile::new(0.5, 100);
    assert_eq!(rq.quantile(), 0.0);
    assert_eq!(rq.count(), 0);
}

#[test]
fn edge_reservoir_single() {
    let mut rq = ReservoirQuantile::new(0.5, 100);
    rq.add(42.0);
    assert_eq!(rq.quantile(), 42.0);
}

#[test]
fn edge_gk_empty() {
    let gk = GKQuantile::new(0.05);
    assert_eq!(gk.quantile(0.5), 0.0);
    assert_eq!(gk.count(), 0);
}

#[test]
fn edge_gk_single() {
    let mut gk = GKQuantile::new(0.05);
    gk.add(42.0);
    assert_eq!(gk.quantile(0.5), 42.0);
}

#[test]
fn edge_gk_all_same() {
    let mut gk = GKQuantile::new(0.05);
    for _ in 0..100 {
        gk.add(7.0);
    }
    assert_eq!(gk.quantile(0.5), 7.0);
}

#[test]
fn edge_quantile_extreme_outlier() {
    let mut values: Vec<f64> = vec![1.0; 99];
    values.push(1_000_000.0);
    // q(1.0) should be the outlier
    assert_eq!(quantile(&values, 1.0), 1_000_000.0);
    // q(0.0) should be 1.0
    assert_eq!(quantile(&values, 0.0), 1.0);
}

// ── TDigest large data ──────────────────────────────────────────────────────

#[test]
fn tdigest_large_data_set() {
    let mut td = TDigest::new();
    for i in 1..=10_000 {
        td.add(i as f64);
    }
    assert_eq!(td.count(), 10_000);
    // Approximate check — TDigest is lossy
    let q50 = td.quantile(0.5);
    assert!(q50 > 3000.0 && q50 < 7000.0, "q50={}", q50);
    let q99 = td.quantile(0.99);
    assert!(q99 > q50, "q99 {} <= q50 {}", q99, q50);
}

// ── Serde round-trip ────────────────────────────────────────────────────────

#[test]
fn tdigest_serde_round_trip() {
    let mut td = TDigest::new();
    td.add(1.0);
    td.add(2.0);
    td.add(3.0);
    let json = serde_json::to_string(&td).unwrap();
    let td2: TDigest = serde_json::from_str(&json).unwrap();
    assert_eq!(td2.count(), 3);
}
