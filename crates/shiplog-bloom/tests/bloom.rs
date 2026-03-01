use proptest::prelude::*;
use shiplog_bloom::{BloomBuilder, BloomConfig, BloomFilter};

// ── Property tests ──────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn inserted_items_are_always_found(values in proptest::collection::vec(any::<i64>(), 1..100)) {
        let config = BloomConfig::new(values.len().max(1), 0.01);
        let mut filter = BloomFilter::with_config(&config);

        for v in &values {
            filter.insert(v);
        }
        for v in &values {
            prop_assert!(filter.contains(v), "inserted value {v} not found");
        }
    }

    #[test]
    fn inserted_strings_are_always_found(values in proptest::collection::vec("[a-z]{1,20}", 1..50)) {
        let config = BloomConfig::new(values.len().max(1), 0.01);
        let mut filter = BloomFilter::with_config(&config);

        for v in &values {
            filter.insert(v);
        }
        for v in &values {
            prop_assert!(filter.contains(v), "inserted string not found");
        }
    }

    #[test]
    fn bits_and_hash_count_are_positive(items in 1..10_000usize, rate in 0.001..0.5f64) {
        let config = BloomConfig::new(items, rate);
        prop_assert!(config.optimal_bits() > 0);
        prop_assert!(config.optimal_hash_count() > 0);
    }

    #[test]
    fn false_positive_rate_stays_below_one(values in proptest::collection::vec(any::<i32>(), 1..200)) {
        let mut filter: BloomFilter<i32> = BloomFilter::with_size(1000, 5);
        for v in &values {
            filter.insert(v);
        }
        let rate = filter.false_positive_rate();
        prop_assert!((0.0..=1.0).contains(&rate), "rate out of range: {rate}");
    }
}

// ── Edge cases ──────────────────────────────────────────────────────────────

#[test]
fn empty_filter_contains_nothing() {
    let filter: BloomFilter<String> = BloomFilter::new();
    assert!(!filter.contains(&"anything".to_string()));
    assert!(!filter.contains(&String::new()));
}

#[test]
fn single_element_insert_and_lookup() {
    let mut filter: BloomFilter<u64> = BloomFilter::new();
    filter.insert(&42);
    assert!(filter.contains(&42));
}

#[test]
fn clear_resets_all_bits() {
    let mut filter: BloomFilter<i32> = BloomFilter::with_size(64, 3);
    for i in 0..20 {
        filter.insert(&i);
    }
    filter.clear();
    // After clearing, the estimated FP rate should be 0
    assert!((filter.false_positive_rate() - 0.0).abs() < f64::EPSILON);
}

#[test]
fn with_size_respects_parameters() {
    let filter: BloomFilter<&str> = BloomFilter::with_size(256, 5);
    assert_eq!(filter.bits(), 256);
    assert_eq!(filter.hash_count(), 5);
}

#[test]
fn default_creates_valid_filter() {
    let filter: BloomFilter<i32> = BloomFilter::default();
    assert!(filter.bits() > 0);
    assert!(filter.hash_count() > 0);
}

#[test]
fn builder_produces_consistent_config() {
    let config = BloomBuilder::new()
        .expected_items(500)
        .false_positive_rate(0.05)
        .build_config();
    assert_eq!(config.expected_items, 500);
    assert!((config.false_positive_rate - 0.05).abs() < f64::EPSILON);

    let filter: BloomFilter<i32> = BloomFilter::with_config(&config);
    assert!(filter.bits() > 0);
}

#[test]
fn builder_default_matches_bloombuilder_new() {
    let a = BloomBuilder::default().build_config();
    let b = BloomBuilder::new().build_config();
    assert_eq!(a.expected_items, b.expected_items);
    assert!((a.false_positive_rate - b.false_positive_rate).abs() < f64::EPSILON);
}

// ── Stress test ─────────────────────────────────────────────────────────────

#[test]
fn stress_many_inserts_bounded_false_positives() {
    let n = 1000;
    let config = BloomConfig::new(n, 0.01);
    let mut filter: BloomFilter<i32> = BloomFilter::with_config(&config);

    for i in 0..n as i32 {
        filter.insert(&i);
    }

    // All inserted items must be found
    for i in 0..n as i32 {
        assert!(filter.contains(&i), "missing inserted value {i}");
    }

    // Count false positives in a separate range
    let fp_count = (n as i32..n as i32 * 10)
        .filter(|v| filter.contains(v))
        .count();
    let fp_rate = fp_count as f64 / (n as f64 * 9.0);
    // Allow generous margin (5×) over target FP rate for statistical variance
    assert!(
        fp_rate < 0.05,
        "false positive rate {fp_rate} exceeds tolerance"
    );
}
