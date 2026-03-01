use proptest::prelude::*;
use shiplog_enumerator::*;

// ── EnumerationConfig ─────────────────────────────────────────────────

#[test]
fn config_default_values() {
    let config = EnumerationConfig::default();
    assert_eq!(config.start_index, 0);
    assert_eq!(config.step, 1);
}

#[test]
fn config_clone() {
    let config = EnumerationConfig {
        start_index: 5,
        step: 3,
    };
    let cloned = config.clone();
    assert_eq!(cloned.start_index, 5);
    assert_eq!(cloned.step, 3);
}

// ── EnumerationBuilder ────────────────────────────────────────────────

#[test]
fn builder_default() {
    let builder = EnumerationBuilder::default();
    let config = builder.build();
    assert_eq!(config.start_index, 0);
    assert_eq!(config.step, 1);
}

#[test]
fn builder_chaining() {
    let config = EnumerationBuilder::new().start_index(100).step(10).build();
    assert_eq!(config.start_index, 100);
    assert_eq!(config.step, 10);
}

// ── Enumerate iterator ────────────────────────────────────────────────

#[test]
fn enumerate_empty() {
    let items: Vec<i32> = vec![];
    let result: Vec<_> = Enumerate::new(items.into_iter()).collect();
    assert!(result.is_empty());
}

#[test]
fn enumerate_from_zero() {
    let items = vec!['a', 'b', 'c'];
    let result: Vec<_> = Enumerate::new(items.into_iter()).collect();
    assert_eq!(result, vec![(0, 'a'), (1, 'b'), (2, 'c')]);
}

#[test]
fn enumerate_with_start_offset() {
    let items = vec!['x', 'y'];
    let result: Vec<_> = Enumerate::with_start(items.into_iter(), 42).collect();
    assert_eq!(result, vec![(42, 'x'), (43, 'y')]);
}

#[test]
fn enumerate_with_config_step() {
    let config = EnumerationConfig {
        start_index: 0,
        step: 5,
    };
    let items = vec!["a", "b", "c"];
    let result: Vec<_> = Enumerate::with_config(items.into_iter(), &config).collect();
    assert_eq!(result, vec![(0, "a"), (5, "b"), (10, "c")]);
}

#[test]
fn enumerate_with_config_start_and_step() {
    let config = EnumerationConfig {
        start_index: 100,
        step: 50,
    };
    let items = vec![1, 2, 3];
    let result: Vec<_> = Enumerate::with_config(items.into_iter(), &config).collect();
    assert_eq!(result, vec![(100, 1), (150, 2), (200, 3)]);
}

// ── LabeledEnumerate ──────────────────────────────────────────────────

#[test]
fn labeled_enumerate_empty() {
    let items: Vec<i32> = vec![];
    let labels = vec!["first".to_string()];
    let result: Vec<_> = LabeledEnumerate::new(items.into_iter(), labels).collect();
    assert!(result.is_empty());
}

#[test]
fn labeled_enumerate_exact_labels() {
    let items = vec![10, 20];
    let labels = vec!["one".to_string(), "two".to_string()];
    let result: Vec<_> = LabeledEnumerate::new(items.into_iter(), labels).collect();
    assert_eq!(
        result,
        vec![("one".to_string(), 10), ("two".to_string(), 20)]
    );
}

#[test]
fn labeled_enumerate_fewer_labels_than_items() {
    let items = vec![1, 2, 3];
    let labels = vec!["first".to_string()];
    let result: Vec<_> = LabeledEnumerate::new(items.into_iter(), labels).collect();
    assert_eq!(result[0].0, "first");
    // Beyond labels, fallback to "item_N"
    assert_eq!(result[1].0, "item_1");
    assert_eq!(result[2].0, "item_2");
}

#[test]
fn labeled_enumerate_with_generator() {
    let items = vec![100, 200, 300];
    let result: Vec<_> =
        LabeledEnumerate::with_generator(items.into_iter(), |i| format!("step_{}", i)).collect();
    assert_eq!(result[0].0, "step_0");
    assert_eq!(result[1].0, "step_1");
    assert_eq!(result[2].0, "step_2");
}

// ── EnumerateExt trait ────────────────────────────────────────────────

#[test]
fn ext_enumerate_items() {
    let result: Vec<_> = vec![10, 20, 30].into_iter().enumerate_items().collect();
    assert_eq!(result, vec![(0, 10), (1, 20), (2, 30)]);
}

#[test]
fn ext_enumerate_from() {
    let result: Vec<_> = vec!['a', 'b'].into_iter().enumerate_from(5).collect();
    assert_eq!(result, vec![(5, 'a'), (6, 'b')]);
}

#[test]
fn ext_enumerate_with_config() {
    let config = EnumerationBuilder::new().start_index(0).step(3).build();
    let result: Vec<_> = vec![10, 20, 30]
        .into_iter()
        .enumerate_with_config(&config)
        .collect();
    assert_eq!(result, vec![(0, 10), (3, 20), (6, 30)]);
}

#[test]
fn ext_enumerate_labeled() {
    let labels = vec!["x".to_string(), "y".to_string(), "z".to_string()];
    let result: Vec<_> = vec![1, 2, 3]
        .into_iter()
        .enumerate_labeled(labels)
        .collect();
    assert_eq!(result[0], ("x".to_string(), 1));
    assert_eq!(result[2], ("z".to_string(), 3));
}

// ── proptest ──────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn enumerate_items_count_matches(n in 0usize..200) {
        let items: Vec<usize> = (0..n).collect();
        let result: Vec<_> = items.into_iter().enumerate_items().collect();
        prop_assert_eq!(result.len(), n);
    }

    #[test]
    fn enumerate_indices_are_sequential(ref data in proptest::collection::vec(any::<i32>(), 1..50)) {
        let result: Vec<_> = data.clone().into_iter().enumerate_items().collect();
        for (i, (idx, _)) in result.iter().enumerate() {
            prop_assert_eq!(*idx, i);
        }
    }

    #[test]
    fn enumerate_from_starts_at_given(start in 0usize..1000, n in 1usize..50) {
        let items: Vec<usize> = (0..n).collect();
        let result: Vec<_> = items.into_iter().enumerate_from(start).collect();
        prop_assert_eq!(result[0].0, start);
        prop_assert_eq!(result.last().unwrap().0, start + n - 1);
    }

    #[test]
    fn enumerate_step_indices_are_arithmetic(start in 0usize..100, step in 1usize..20, n in 1usize..30) {
        let config = EnumerationConfig { start_index: start, step };
        let items: Vec<usize> = (0..n).collect();
        let result: Vec<_> = Enumerate::with_config(items.into_iter(), &config).collect();
        for (i, (idx, _)) in result.iter().enumerate() {
            prop_assert_eq!(*idx, start + i * step);
        }
    }

    #[test]
    fn enumerate_preserves_values(ref data in proptest::collection::vec(any::<i32>(), 0..100)) {
        let result: Vec<_> = data.clone().into_iter().enumerate_items().collect();
        let values: Vec<_> = result.into_iter().map(|(_, v)| v).collect();
        prop_assert_eq!(values, data.clone());
    }
}
