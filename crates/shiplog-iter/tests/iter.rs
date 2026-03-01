use proptest::prelude::*;
use shiplog_iter::*;

// ── group_by ──────────────────────────────────────────────────────────

#[test]
fn group_by_empty_iterator() {
    let items: Vec<i32> = vec![];
    let groups = group_by(items.into_iter(), |x| x % 2);
    assert!(groups.is_empty());
}

#[test]
fn group_by_single_element() {
    let groups = group_by(vec![42].into_iter(), |x| x % 2);
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[&0], vec![42]);
}

#[test]
fn group_by_all_same_key() {
    let items = vec![2, 4, 6, 8];
    let groups = group_by(items.into_iter(), |x| x % 2);
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[&0], vec![2, 4, 6, 8]);
}

#[test]
fn group_by_string_keys() {
    let items = vec!["apple", "avocado", "banana", "blueberry"];
    let groups = group_by(items.into_iter(), |s| s.chars().next().unwrap());
    assert_eq!(groups[&'a'], vec!["apple", "avocado"]);
    assert_eq!(groups[&'b'], vec!["banana", "blueberry"]);
}

// ── flatten_opt ───────────────────────────────────────────────────────

#[test]
fn flatten_opt_all_none() {
    let items: Vec<Option<i32>> = vec![None, None, None];
    let result: Vec<_> = flatten_opt(items.into_iter()).collect();
    assert!(result.is_empty());
}

#[test]
fn flatten_opt_all_some() {
    let items = vec![Some(1), Some(2), Some(3)];
    let result: Vec<_> = flatten_opt(items.into_iter()).collect();
    assert_eq!(result, vec![1, 2, 3]);
}

#[test]
fn flatten_opt_empty() {
    let items: Vec<Option<i32>> = vec![];
    let result: Vec<_> = flatten_opt(items.into_iter()).collect();
    assert!(result.is_empty());
}

// ── chunk ─────────────────────────────────────────────────────────────

#[test]
fn chunk_empty() {
    let items: Vec<i32> = vec![];
    let chunks = chunk(items.into_iter(), 3);
    assert!(chunks.is_empty());
}

#[test]
fn chunk_exact_division() {
    let items = vec![1, 2, 3, 4, 5, 6];
    let chunks = chunk(items.into_iter(), 2);
    assert_eq!(chunks, vec![vec![1, 2], vec![3, 4], vec![5, 6]]);
}

#[test]
fn chunk_size_one() {
    let items = vec![10, 20, 30];
    let chunks = chunk(items.into_iter(), 1);
    assert_eq!(chunks, vec![vec![10], vec![20], vec![30]]);
}

#[test]
fn chunk_size_larger_than_input() {
    let items = vec![1, 2];
    let chunks = chunk(items.into_iter(), 10);
    assert_eq!(chunks, vec![vec![1, 2]]);
}

// ── window ────────────────────────────────────────────────────────────

#[test]
fn window_empty_iterator() {
    let items: Vec<i32> = vec![];
    let windows: Vec<_> = window(items.into_iter(), 3).collect();
    assert!(windows.is_empty());
}

#[test]
fn window_fewer_elements_than_size() {
    let items = vec![1, 2];
    let windows: Vec<_> = window(items.into_iter(), 5).collect();
    assert!(windows.is_empty());
}

#[test]
fn window_exact_size() {
    let items = vec![1, 2, 3];
    let windows: Vec<_> = window(items.into_iter(), 3).collect();
    assert_eq!(windows, vec![vec![1, 2, 3]]);
}

#[test]
fn window_size_one() {
    let items = vec![10, 20, 30];
    let windows: Vec<_> = window(items.into_iter(), 1).collect();
    assert_eq!(windows, vec![vec![10], vec![20], vec![30]]);
}

#[test]
fn window_preserves_order() {
    let items = vec![5, 4, 3, 2, 1];
    let windows: Vec<_> = window(items.into_iter(), 2).collect();
    assert_eq!(
        windows,
        vec![vec![5, 4], vec![4, 3], vec![3, 2], vec![2, 1]]
    );
}

// ── partition ─────────────────────────────────────────────────────────

#[test]
fn partition_empty() {
    let items: Vec<i32> = vec![];
    let (yes, no) = partition(items.into_iter(), |_| true);
    assert!(yes.is_empty());
    assert!(no.is_empty());
}

#[test]
fn partition_all_true() {
    let items = vec![1, 2, 3];
    let (yes, no) = partition(items.into_iter(), |_| true);
    assert_eq!(yes, vec![1, 2, 3]);
    assert!(no.is_empty());
}

#[test]
fn partition_all_false() {
    let items = vec![1, 2, 3];
    let (yes, no) = partition(items.into_iter(), |_| false);
    assert!(yes.is_empty());
    assert_eq!(no, vec![1, 2, 3]);
}

// ── unique ────────────────────────────────────────────────────────────

#[test]
fn unique_empty() {
    let items: Vec<i32> = vec![];
    let result: Vec<_> = unique(items.into_iter(), |x| *x).collect();
    assert!(result.is_empty());
}

#[test]
fn unique_all_same() {
    let items = vec![5, 5, 5, 5];
    let result: Vec<_> = unique(items.into_iter(), |x| *x).collect();
    assert_eq!(result, vec![5]);
}

#[test]
fn unique_preserves_first_occurrence() {
    let items = vec![1, 2, 1, 3, 2, 4];
    let result: Vec<_> = unique(items.into_iter(), |x| *x).collect();
    assert_eq!(result, vec![1, 2, 3, 4]);
}

#[test]
fn unique_already_unique() {
    let items = vec![1, 2, 3, 4, 5];
    let result: Vec<_> = unique(items.into_iter(), |x| *x).collect();
    assert_eq!(result, vec![1, 2, 3, 4, 5]);
}

// ── count ─────────────────────────────────────────────────────────────

#[test]
fn count_empty() {
    let items: Vec<i32> = vec![];
    assert_eq!(count(items.into_iter()), 0);
}

#[test]
fn count_single() {
    assert_eq!(count(vec![42].into_iter()), 1);
}

// ── config types ──────────────────────────────────────────────────────

#[test]
fn iter_config_clone() {
    let config = IterConfig {
        batch_size: 50,
        parallel: true,
        buffer_size: 500,
    };
    let cloned = config.clone();
    assert_eq!(cloned.batch_size, 50);
    assert!(cloned.parallel);
    assert_eq!(cloned.buffer_size, 500);
}

#[test]
fn batch_config_default() {
    let config = BatchConfig::default();
    assert_eq!(config.size, 100);
    assert_eq!(config.overlap, 0);
}

// ── proptest ──────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn chunk_total_elements_preserved(ref data in proptest::collection::vec(any::<i32>(), 0..100), size in 1usize..20) {
        let total: usize = chunk(data.clone().into_iter(), size)
            .iter()
            .map(|c| c.len())
            .sum();
        prop_assert_eq!(total, data.len());
    }

    #[test]
    fn chunk_max_size_respected(ref data in proptest::collection::vec(any::<i32>(), 0..100), size in 1usize..20) {
        for c in chunk(data.clone().into_iter(), size) {
            prop_assert!(c.len() <= size);
        }
    }

    #[test]
    fn partition_covers_all_elements(ref data in proptest::collection::vec(any::<i32>(), 0..100)) {
        let (yes, no) = partition(data.clone().into_iter(), |x| x % 2 == 0);
        prop_assert_eq!(yes.len() + no.len(), data.len());
    }

    #[test]
    fn unique_output_no_duplicates(ref data in proptest::collection::vec(0i32..20, 0..100)) {
        let result: Vec<_> = unique(data.clone().into_iter(), |x| *x).collect();
        let mut seen = std::collections::HashSet::new();
        for item in &result {
            prop_assert!(seen.insert(*item), "duplicate found: {}", item);
        }
    }

    #[test]
    fn unique_subset_of_input(ref data in proptest::collection::vec(any::<i32>(), 0..100)) {
        let result: Vec<_> = unique(data.clone().into_iter(), |x| *x).collect();
        prop_assert!(result.len() <= data.len());
    }

    #[test]
    fn group_by_all_items_accounted(ref data in proptest::collection::vec(0i32..50, 0..100)) {
        let groups = group_by(data.clone().into_iter(), |x| x % 3);
        let total: usize = groups.values().map(|v| v.len()).sum();
        prop_assert_eq!(total, data.len());
    }

    #[test]
    fn window_count_correct(n in 1usize..50, size in 1usize..10) {
        let data: Vec<i32> = (0..n as i32).collect();
        let windows: Vec<_> = window(data.into_iter(), size).collect();
        if n >= size {
            prop_assert_eq!(windows.len(), n - size + 1);
        } else {
            prop_assert!(windows.is_empty());
        }
    }

    #[test]
    fn flatten_opt_preserves_some_values(ref data in proptest::collection::vec(any::<Option<i32>>(), 0..100)) {
        let result: Vec<_> = flatten_opt(data.clone().into_iter()).collect();
        let expected_count = data.iter().filter(|x| x.is_some()).count();
        prop_assert_eq!(result.len(), expected_count);
    }
}
