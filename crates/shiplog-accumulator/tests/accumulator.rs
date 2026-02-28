use proptest::prelude::*;
use shiplog_accumulator::{Accumulator, GroupAccumulator, SumAccumulator};

// ── Accumulator correctness ────────────────────────────────────────

#[test]
fn accumulator_empty() {
    let acc: Accumulator<i32> = Accumulator::new();
    assert!(acc.is_empty());
    assert_eq!(acc.len(), 0);
    assert!(acc.values().is_empty());
}

#[test]
fn accumulator_add_items() {
    let mut acc = Accumulator::new();
    acc.add(10);
    acc.add(20);
    acc.add(30);
    assert_eq!(acc.len(), 3);
    assert_eq!(acc.values(), &[10, 20, 30]);
}

#[test]
fn accumulator_clear() {
    let mut acc = Accumulator::new();
    acc.add(1);
    acc.add(2);
    acc.clear();
    assert!(acc.is_empty());
    assert_eq!(acc.len(), 0);
    assert!(acc.values().is_empty());
}

#[test]
fn accumulator_clear_then_add() {
    let mut acc = Accumulator::new();
    acc.add(1);
    acc.clear();
    acc.add(2);
    assert_eq!(acc.len(), 1);
    assert_eq!(acc.values(), &[2]);
}

#[test]
fn accumulator_single_item() {
    let mut acc = Accumulator::new();
    acc.add("hello");
    assert_eq!(acc.len(), 1);
    assert_eq!(acc.values(), &["hello"]);
}

#[test]
fn accumulator_default() {
    let acc: Accumulator<i32> = Accumulator::default();
    assert!(acc.is_empty());
}

// ── SumAccumulator correctness ─────────────────────────────────────

#[test]
fn sum_accumulator_empty() {
    let acc: SumAccumulator<i32> = SumAccumulator::new();
    assert_eq!(acc.sum(), 0);
    assert_eq!(acc.count(), 0);
    assert!(acc.average().is_none());
}

#[test]
fn sum_accumulator_basic() {
    let mut acc = SumAccumulator::new();
    acc.add(10);
    acc.add(20);
    acc.add(30);
    assert_eq!(acc.sum(), 60);
    assert_eq!(acc.count(), 3);
    assert_eq!(acc.average(), Some(20));
}

#[test]
fn sum_accumulator_single() {
    let mut acc = SumAccumulator::new();
    acc.add(42i32);
    assert_eq!(acc.sum(), 42);
    assert_eq!(acc.count(), 1);
    assert_eq!(acc.average(), Some(42));
}

#[test]
fn sum_accumulator_zeros() {
    let mut acc = SumAccumulator::new();
    acc.add(0i32);
    acc.add(0);
    assert_eq!(acc.sum(), 0);
    assert_eq!(acc.count(), 2);
    assert_eq!(acc.average(), Some(0));
}

#[test]
fn sum_accumulator_default() {
    let acc: SumAccumulator<i32> = SumAccumulator::default();
    assert_eq!(acc.sum(), 0);
}

// ── GroupAccumulator correctness ───────────────────────────────────

#[test]
fn group_accumulator_empty() {
    let acc: GroupAccumulator<String, i32> = GroupAccumulator::new();
    assert!(acc.is_empty());
    assert_eq!(acc.len(), 0);
}

#[test]
fn group_accumulator_basic() {
    let mut acc = GroupAccumulator::new();
    acc.insert("a", 1);
    acc.insert("a", 2);
    acc.insert("b", 3);
    assert_eq!(acc.len(), 2);
    assert_eq!(acc.get(&"a"), Some(&vec![1, 2]));
    assert_eq!(acc.get(&"b"), Some(&vec![3]));
}

#[test]
fn group_accumulator_missing_key() {
    let acc: GroupAccumulator<&str, i32> = GroupAccumulator::new();
    assert!(acc.get(&"missing").is_none());
}

#[test]
fn group_accumulator_single_group() {
    let mut acc = GroupAccumulator::new();
    acc.insert("only", 1);
    acc.insert("only", 2);
    acc.insert("only", 3);
    assert_eq!(acc.len(), 1);
    assert_eq!(acc.get(&"only"), Some(&vec![1, 2, 3]));
}

#[test]
fn group_accumulator_keys() {
    let mut acc = GroupAccumulator::new();
    acc.insert("x", 1);
    acc.insert("y", 2);
    acc.insert("z", 3);
    let keys: std::collections::HashSet<_> = acc.keys().collect();
    assert!(keys.contains(&"x"));
    assert!(keys.contains(&"y"));
    assert!(keys.contains(&"z"));
}

#[test]
fn group_accumulator_default() {
    let acc: GroupAccumulator<String, i32> = GroupAccumulator::default();
    assert!(acc.is_empty());
}

// ── Composition: accumulate then summarize ─────────────────────────

#[test]
fn composition_accumulate_then_sum() {
    let mut acc = Accumulator::new();
    for i in 1..=10 {
        acc.add(i);
    }
    let sum: i32 = acc.values().iter().sum();
    assert_eq!(sum, 55);
}

#[test]
fn composition_group_then_count() {
    let mut acc = GroupAccumulator::new();
    let items = vec!["a", "b", "a", "c", "b", "a"];
    for item in items {
        acc.insert(item, 1);
    }
    assert_eq!(acc.get(&"a").unwrap().len(), 3);
    assert_eq!(acc.get(&"b").unwrap().len(), 2);
    assert_eq!(acc.get(&"c").unwrap().len(), 1);
}

// ── Property tests ─────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_accumulator_len_matches_adds(items in prop::collection::vec(any::<i32>(), 0..200)) {
        let mut acc = Accumulator::new();
        for &item in &items {
            acc.add(item);
        }
        prop_assert_eq!(acc.len(), items.len());
        prop_assert_eq!(acc.values(), items.as_slice());
    }

    #[test]
    fn prop_accumulator_clear_resets(items in prop::collection::vec(any::<i32>(), 1..100)) {
        let mut acc = Accumulator::new();
        for item in items {
            acc.add(item);
        }
        acc.clear();
        prop_assert!(acc.is_empty());
        prop_assert_eq!(acc.len(), 0);
    }

    #[test]
    fn prop_sum_accumulator_sum_correct(items in prop::collection::vec(-100i32..100, 0..100)) {
        let mut acc = SumAccumulator::new();
        for &item in &items {
            acc.add(item);
        }
        let expected: i32 = items.iter().sum();
        prop_assert_eq!(acc.sum(), expected);
        prop_assert_eq!(acc.count(), items.len());
    }

    #[test]
    fn prop_sum_accumulator_count_matches(items in prop::collection::vec(-100i32..100, 0..200)) {
        let mut acc = SumAccumulator::new();
        for &item in &items {
            acc.add(item);
        }
        prop_assert_eq!(acc.count(), items.len());
    }

    #[test]
    fn prop_group_accumulator_total_values(
        items in prop::collection::vec((0u32..5, any::<i32>()), 0..100)
    ) {
        let mut acc = GroupAccumulator::new();
        for (k, v) in &items {
            acc.insert(*k, *v);
        }
        let total: usize = (0..5u32)
            .filter_map(|k| acc.get(&k))
            .map(|v| v.len())
            .sum();
        prop_assert_eq!(total, items.len());
    }
}
