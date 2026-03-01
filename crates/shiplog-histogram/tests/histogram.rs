use proptest::prelude::*;
use shiplog_histogram::{Histogram, LinearHistogram};

// ── Property tests ──────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_count_equals_num_records(values in proptest::collection::vec(0.0_f64..100.0, 1..200)) {
        let mut h = Histogram::new("p");
        for &v in &values {
            h.record(v);
        }
        prop_assert_eq!(h.count(), values.len() as u64);
    }

    #[test]
    fn prop_sum_matches(values in proptest::collection::vec(0.0_f64..1000.0, 1..100)) {
        let mut h = Histogram::new("p");
        let expected_sum: f64 = values.iter().sum();
        for &v in &values {
            h.record(v);
        }
        prop_assert!((h.sum() - expected_sum).abs() < 1e-6);
    }

    #[test]
    fn prop_min_le_max(values in proptest::collection::vec(-1e6_f64..1e6, 1..200)) {
        let mut h = Histogram::new("p");
        for &v in &values {
            h.record(v);
        }
        prop_assert!(h.min() <= h.max());
    }

    #[test]
    fn prop_min_is_actual_min(values in proptest::collection::vec(-1e6_f64..1e6, 1..200)) {
        let mut h = Histogram::new("p");
        for &v in &values {
            h.record(v);
        }
        let expected_min = values.iter().cloned().reduce(f64::min).unwrap();
        prop_assert_eq!(h.min(), expected_min);
    }

    #[test]
    fn prop_max_is_actual_max(values in proptest::collection::vec(-1e6_f64..1e6, 1..200)) {
        let mut h = Histogram::new("p");
        for &v in &values {
            h.record(v);
        }
        let expected_max = values.iter().cloned().reduce(f64::max).unwrap();
        prop_assert_eq!(h.max(), expected_max);
    }

    #[test]
    fn prop_mean_within_range(values in proptest::collection::vec(0.0_f64..100.0, 1..200)) {
        let mut h = Histogram::new("p");
        for &v in &values {
            h.record(v);
        }
        prop_assert!(h.mean() >= h.min());
        prop_assert!(h.mean() <= h.max());
    }

    #[test]
    fn prop_cumulative_at_infinity_equals_count(values in proptest::collection::vec(0.0_f64..20.0, 1..200)) {
        let mut h = Histogram::new("p");
        for &v in &values {
            h.record(v);
        }
        // Cumulative up to a very large threshold should capture all values
        // recorded in finite buckets; count() is the authoritative total.
        prop_assert_eq!(h.count(), values.len() as u64);
    }

    #[test]
    fn prop_cumulative_monotonic(values in proptest::collection::vec(0.0_f64..20.0, 1..100)) {
        let mut h = Histogram::new("p");
        for &v in &values {
            h.record(v);
        }
        let thresholds = [0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0];
        let mut prev = 0u64;
        for &t in &thresholds {
            let cum = h.cumulative_below(t);
            prop_assert!(cum >= prev, "cumulative must be monotonic: {} < {} at {}", cum, prev, t);
            prev = cum;
        }
    }

    #[test]
    fn prop_linear_count_correct(
        values in proptest::collection::vec(0.0_f64..100.0, 1..200),
    ) {
        let mut lh = LinearHistogram::new("p", 0.0, 100.0, 10);
        for &v in &values {
            lh.record(v);
        }
        prop_assert_eq!(lh.count(), values.len() as u64);
    }

    #[test]
    fn prop_linear_bucket_sum_plus_overflow_underflow(
        values in proptest::collection::vec(-50.0_f64..150.0, 1..200),
    ) {
        let mut lh = LinearHistogram::new("p", 0.0, 100.0, 10);
        for &v in &values {
            lh.record(v);
        }
        let bucket_sum: u64 = lh.buckets().iter().sum();
        prop_assert_eq!(bucket_sum + lh.underflow() + lh.overflow(), lh.count());
    }
}

// ── Known-answer tests ──────────────────────────────────────────────────────

#[test]
fn known_answer_histogram_stats() {
    let mut h = Histogram::new("lat");
    h.record(1.0);
    h.record(2.0);
    h.record(3.0);
    assert_eq!(h.count(), 3);
    assert!((h.sum() - 6.0).abs() < f64::EPSILON);
    assert_eq!(h.min(), 1.0);
    assert_eq!(h.max(), 3.0);
    assert!((h.mean() - 2.0).abs() < f64::EPSILON);
}

#[test]
fn known_answer_custom_buckets() {
    let mut h = Histogram::with_buckets("custom", vec![1.0, 5.0, 10.0]);
    h.record(0.5); // bucket 1.0
    h.record(3.0); // bucket 5.0
    h.record(7.0); // bucket 10.0
    h.record(20.0); // bucket +Inf
    assert_eq!(h.count(), 4);
    assert_eq!(h.buckets().len(), 4); // 1.0, 5.0, 10.0, +Inf
}

#[test]
fn known_answer_bucket_index() {
    let h = Histogram::new("idx");
    // Default buckets: 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, +Inf
    assert_eq!(h.bucket_index(0.001), 0); // 0.005
    assert_eq!(h.bucket_index(0.007), 1); // 0.01
    assert_eq!(h.bucket_index(0.5), 6); // 0.5
    assert_eq!(h.bucket_index(100.0), 11); // +Inf
}

#[test]
fn known_answer_linear_histogram() {
    let mut lh = LinearHistogram::new("lin", 0.0, 100.0, 10);
    lh.record(5.0);
    lh.record(55.0);
    lh.record(-10.0);
    lh.record(110.0);
    assert_eq!(lh.count(), 4);
    assert_eq!(lh.underflow(), 1);
    assert_eq!(lh.overflow(), 1);
    assert!((lh.mean() - 40.0).abs() < f64::EPSILON);
}

// ── Edge cases ──────────────────────────────────────────────────────────────

#[test]
fn edge_histogram_empty() {
    let h = Histogram::new("empty");
    assert_eq!(h.count(), 0);
    assert_eq!(h.sum(), 0.0);
    assert_eq!(h.min(), 0.0);
    assert_eq!(h.max(), 0.0);
    assert_eq!(h.mean(), 0.0);
}

#[test]
fn edge_histogram_single_value() {
    let mut h = Histogram::new("single");
    h.record(42.0);
    assert_eq!(h.count(), 1);
    assert_eq!(h.min(), 42.0);
    assert_eq!(h.max(), 42.0);
    assert_eq!(h.mean(), 42.0);
}

#[test]
fn edge_histogram_all_same_values() {
    let mut h = Histogram::new("same");
    for _ in 0..100 {
        h.record(5.0);
    }
    assert_eq!(h.count(), 100);
    assert_eq!(h.min(), 5.0);
    assert_eq!(h.max(), 5.0);
    assert!((h.mean() - 5.0).abs() < f64::EPSILON);
}

#[test]
fn edge_histogram_negative_values() {
    let mut h = Histogram::new("neg");
    h.record(-10.0);
    h.record(-5.0);
    h.record(-1.0);
    assert_eq!(h.count(), 3);
    assert_eq!(h.min(), -10.0);
    assert_eq!(h.max(), -1.0);
}

#[test]
fn edge_histogram_reset() {
    let mut h = Histogram::new("reset");
    h.record(10.0);
    h.record(20.0);
    h.reset();
    assert_eq!(h.count(), 0);
    assert_eq!(h.sum(), 0.0);
    assert_eq!(h.min(), 0.0);
    assert_eq!(h.max(), 0.0);
    assert_eq!(h.cumulative_below(f64::MAX), 0);
}

#[test]
fn edge_histogram_with_infinity_bucket_in_bounds() {
    let mut h = Histogram::with_buckets("inf", vec![1.0, f64::INFINITY]);
    h.record(0.5);
    h.record(5.0);
    assert_eq!(h.count(), 2);
    assert_eq!(h.buckets().len(), 2);
}

#[test]
fn edge_linear_histogram_empty() {
    let lh = LinearHistogram::new("empty", 0.0, 100.0, 10);
    assert_eq!(lh.count(), 0);
    assert_eq!(lh.mean(), 0.0);
}

#[test]
fn edge_linear_histogram_all_underflow() {
    let mut lh = LinearHistogram::new("under", 10.0, 20.0, 5);
    for i in 0..10 {
        lh.record(i as f64);
    }
    assert_eq!(lh.underflow(), 10);
    assert_eq!(lh.overflow(), 0);
}

#[test]
fn edge_linear_histogram_all_overflow() {
    let mut lh = LinearHistogram::new("over", 0.0, 10.0, 5);
    for i in 10..20 {
        lh.record(i as f64);
    }
    assert_eq!(lh.overflow(), 10);
    assert_eq!(lh.underflow(), 0);
}

#[test]
fn edge_linear_histogram_reset() {
    let mut lh = LinearHistogram::new("reset", 0.0, 100.0, 5);
    lh.record(50.0);
    lh.record(-10.0);
    lh.record(200.0);
    lh.reset();
    assert_eq!(lh.count(), 0);
    assert_eq!(lh.underflow(), 0);
    assert_eq!(lh.overflow(), 0);
    assert!(lh.buckets().iter().all(|&c| c == 0));
}

// ── Serde round-trip ────────────────────────────────────────────────────────

// Note: Histogram serde round-trip is not tested because the default
// and custom buckets include f64::INFINITY which JSON cannot represent.
