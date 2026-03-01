use proptest::prelude::*;
use shiplog_heap::{BinaryHeap, BinaryHeapWithOrder, HeapError, HeapOrder};

// ── Property tests ──────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn pop_returns_elements_in_descending_order(values in proptest::collection::vec(any::<i32>(), 1..100)) {
        let heap = BinaryHeap::from_vec(values);
        let sorted = heap.into_sorted_vec();
        for window in sorted.windows(2) {
            prop_assert!(window[0] >= window[1], "not descending: {} < {}", window[0], window[1]);
        }
    }

    #[test]
    fn len_matches_insert_count(values in proptest::collection::vec(any::<i32>(), 0..100)) {
        let mut heap = BinaryHeap::new();
        for v in &values {
            heap.push(*v);
        }
        prop_assert_eq!(heap.len(), values.len());
    }

    #[test]
    fn peek_returns_max_element(values in proptest::collection::vec(any::<i32>(), 1..100)) {
        let heap = BinaryHeap::from_vec(values.clone());
        let max = values.iter().max().unwrap();
        prop_assert_eq!(heap.peek(), Some(max));
    }

    #[test]
    fn min_heap_pop_returns_ascending(values in proptest::collection::vec(any::<i32>(), 1..100)) {
        let mut heap = BinaryHeapWithOrder::new(HeapOrder::Min);
        for v in &values {
            heap.push(*v);
        }
        let mut sorted = Vec::new();
        while let Some(v) = heap.pop() {
            sorted.push(v);
        }
        for window in sorted.windows(2) {
            prop_assert!(window[0] <= window[1], "not ascending: {} > {}", window[0], window[1]);
        }
    }

    #[test]
    fn max_heap_with_order_pop_returns_descending(values in proptest::collection::vec(any::<i32>(), 1..100)) {
        let mut heap = BinaryHeapWithOrder::new(HeapOrder::Max);
        for v in &values {
            heap.push(*v);
        }
        let mut sorted = Vec::new();
        while let Some(v) = heap.pop() {
            sorted.push(v);
        }
        for window in sorted.windows(2) {
            prop_assert!(window[0] >= window[1], "not descending: {} < {}", window[0], window[1]);
        }
    }

    #[test]
    fn from_vec_preserves_all_elements(values in proptest::collection::vec(any::<i32>(), 0..100)) {
        let heap = BinaryHeap::from_vec(values.clone());
        let mut sorted = heap.into_sorted_vec();
        sorted.sort();
        let mut expected = values;
        expected.sort();
        prop_assert_eq!(sorted, expected);
    }
}

// ── Edge cases ──────────────────────────────────────────────────────────────

#[test]
fn empty_heap() {
    let mut heap: BinaryHeap<i32> = BinaryHeap::new();
    assert!(heap.is_empty());
    assert_eq!(heap.len(), 0);
    assert_eq!(heap.pop(), None);
    assert_eq!(heap.peek(), None);
}

#[test]
fn single_element() {
    let mut heap = BinaryHeap::new();
    heap.push(42);
    assert_eq!(heap.len(), 1);
    assert_eq!(heap.peek(), Some(&42));
    assert_eq!(heap.pop(), Some(42));
    assert!(heap.is_empty());
}

#[test]
fn default_creates_empty_heap() {
    let heap: BinaryHeap<i32> = BinaryHeap::default();
    assert!(heap.is_empty());
}

#[test]
fn from_vec_empty() {
    let heap = BinaryHeap::from_vec(Vec::<i32>::new());
    assert!(heap.is_empty());
}

#[test]
fn from_vec_single() {
    let heap = BinaryHeap::from_vec(vec![99]);
    assert_eq!(heap.len(), 1);
    assert_eq!(heap.peek(), Some(&99));
}

#[test]
fn into_sorted_vec_empty() {
    let heap: BinaryHeap<i32> = BinaryHeap::new();
    assert!(heap.into_sorted_vec().is_empty());
}

#[test]
fn iter_visits_all_elements() {
    let heap = BinaryHeap::from_vec(vec![3, 1, 4, 1, 5]);
    let sum: i32 = heap.iter().sum();
    assert_eq!(sum, 14);
}

#[test]
fn with_order_empty_min_heap() {
    let mut heap: BinaryHeapWithOrder<i32> = BinaryHeapWithOrder::new(HeapOrder::Min);
    assert!(heap.is_empty());
    assert_eq!(heap.pop(), None);
    assert_eq!(heap.peek(), None);
}

#[test]
fn with_order_single_element() {
    let mut heap = BinaryHeapWithOrder::new(HeapOrder::Min);
    heap.push(42);
    assert_eq!(heap.peek(), Some(&42));
    assert_eq!(heap.pop(), Some(42));
    assert!(heap.is_empty());
}

#[test]
fn heap_error_display() {
    assert_eq!(HeapError::Empty.to_string(), "Heap is empty");
}

#[test]
fn duplicate_elements_preserved() {
    let heap = BinaryHeap::from_vec(vec![5, 5, 5, 5]);
    let sorted = heap.into_sorted_vec();
    assert_eq!(sorted, vec![5, 5, 5, 5]);
}

// ── Stress test ─────────────────────────────────────────────────────────────

#[test]
fn stress_large_heap_sort() {
    let values: Vec<i32> = (0..5000).rev().collect();
    let heap = BinaryHeap::from_vec(values);
    let sorted = heap.into_sorted_vec();
    assert_eq!(sorted.len(), 5000);
    for window in sorted.windows(2) {
        assert!(window[0] >= window[1]);
    }
}

#[test]
fn stress_interleaved_push_pop() {
    let mut heap = BinaryHeap::new();
    for i in 0..1000 {
        heap.push(i);
        if i % 3 == 0 {
            heap.pop();
        }
    }
    let mut prev = i32::MAX;
    while let Some(v) = heap.pop() {
        assert!(v <= prev);
        prev = v;
    }
}
