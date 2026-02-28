use proptest::prelude::*;
use shiplog_aggregator::*;

// ── Aggregator basics ───────────────────────────────────────────────

#[test]
fn new_aggregator_is_empty() {
    let agg: Aggregator<i32> = Aggregator::new();
    assert!(agg.is_empty());
    assert_eq!(agg.len(), 0);
    assert_eq!(agg.count(), 0);
}

#[test]
fn default_matches_new() {
    let a: Aggregator<i32> = Aggregator::default();
    let b: Aggregator<i32> = Aggregator::new();
    assert_eq!(a.len(), b.len());
}

#[test]
fn add_many_extends_items() {
    let mut agg = Aggregator::new();
    agg.add_many(vec![1, 2, 3]);
    assert_eq!(agg.len(), 3);
    assert_eq!(agg.items(), &[1, 2, 3]);
}

#[test]
fn clear_empties_aggregator() {
    let mut agg = Aggregator::new();
    agg.add_many(vec![10, 20, 30]);
    agg.clear();
    assert!(agg.is_empty());
}

// ── Sum ─────────────────────────────────────────────────────────────

#[test]
fn sum_empty_is_none() {
    let agg: Aggregator<i32> = Aggregator::new();
    assert_eq!(agg.sum(), None);
}

#[test]
fn sum_single_element() {
    let mut agg = Aggregator::new();
    agg.add(42);
    assert_eq!(agg.sum(), Some(42));
}

proptest! {
    #[test]
    fn sum_matches_iter_sum(vals in prop::collection::vec(-1000i64..1000, 1..50)) {
        let mut agg = Aggregator::new();
        agg.add_many(vals.clone());
        let expected: i64 = vals.iter().sum();
        prop_assert_eq!(agg.sum(), Some(expected));
    }
}

// ── Min / Max ───────────────────────────────────────────────────────

#[test]
fn min_max_empty_is_none() {
    let agg: Aggregator<i32> = Aggregator::new();
    assert_eq!(agg.min(), None);
    assert_eq!(agg.max(), None);
}

proptest! {
    #[test]
    fn min_max_matches_iter(vals in prop::collection::vec(-1000i32..1000, 1..50)) {
        let mut agg = Aggregator::new();
        agg.add_many(vals.clone());
        prop_assert_eq!(agg.min(), vals.iter().copied().min());
        prop_assert_eq!(agg.max(), vals.iter().copied().max());
    }
}

// ── Average ─────────────────────────────────────────────────────────

#[test]
fn average_empty_is_none() {
    let agg: Aggregator<i32> = Aggregator::new();
    assert_eq!(agg.average(), None);
}

proptest! {
    #[test]
    fn average_within_min_max(vals in prop::collection::vec(1i32..1000, 1..50)) {
        let mut agg = Aggregator::new();
        agg.add_many(vals.clone());
        let avg = agg.average().unwrap();
        let min_val: f64 = *vals.iter().min().unwrap() as f64;
        let max_val: f64 = *vals.iter().max().unwrap() as f64;
        prop_assert!(avg >= min_val && avg <= max_val,
            "avg={avg} not in [{min_val}, {max_val}]");
    }
}

// ── KeyedAggregator ─────────────────────────────────────────────────

#[test]
fn keyed_new_is_empty() {
    let ka: KeyedAggregator<String, i32> = KeyedAggregator::new();
    assert!(ka.is_empty());
    assert_eq!(ka.len(), 0);
}

#[test]
fn keyed_default_matches_new() {
    let a: KeyedAggregator<String, i32> = KeyedAggregator::default();
    let b: KeyedAggregator<String, i32> = KeyedAggregator::new();
    assert_eq!(a.len(), b.len());
}

#[test]
fn keyed_insert_groups_by_key() {
    let mut ka = KeyedAggregator::new();
    ka.insert("a", 1);
    ka.insert("a", 2);
    ka.insert("b", 3);
    assert_eq!(ka.len(), 2);
    assert_eq!(ka.get(&"a").map(|v| v.len()), Some(2));
    assert_eq!(ka.get(&"b").map(|v| v.len()), Some(1));
    assert_eq!(ka.get(&"c"), None);
}

#[test]
fn keyed_totals_counts_per_key() {
    let mut ka = KeyedAggregator::new();
    ka.insert("x", 10);
    ka.insert("x", 20);
    ka.insert("y", 30);
    let t = ka.totals();
    assert_eq!(t[&"x"], 2);
    assert_eq!(t[&"y"], 1);
}

#[test]
fn keyed_clear_empties() {
    let mut ka = KeyedAggregator::new();
    ka.insert("a", 1);
    ka.clear();
    assert!(ka.is_empty());
}

proptest! {
    #[test]
    fn keyed_keys_count_equals_len(
        entries in prop::collection::vec(
            (0u8..5, 0i32..100), 1..30
        )
    ) {
        let mut ka = KeyedAggregator::new();
        for (k, v) in &entries {
            ka.insert(*k, *v);
        }
        let unique_keys: std::collections::HashSet<_> = entries.iter().map(|(k, _)| k).collect();
        prop_assert_eq!(ka.len(), unique_keys.len());
        prop_assert_eq!(ka.keys().count(), unique_keys.len());
    }
}
