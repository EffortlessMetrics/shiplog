use proptest::prelude::*;
use shiplog_deque::{Deque, DequeError};

// ── Property tests ──────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn push_back_pop_front_is_fifo(values in proptest::collection::vec(any::<i32>(), 0..100)) {
        let mut deque = Deque::new();
        for v in &values {
            deque.push_back(*v);
        }
        for v in &values {
            prop_assert_eq!(deque.pop_front(), Some(*v));
        }
        prop_assert_eq!(deque.pop_front(), None);
    }

    #[test]
    fn push_front_pop_back_is_fifo(values in proptest::collection::vec(any::<i32>(), 0..100)) {
        let mut deque = Deque::new();
        for v in &values {
            deque.push_front(*v);
        }
        for v in &values {
            prop_assert_eq!(deque.pop_back(), Some(*v));
        }
        prop_assert_eq!(deque.pop_back(), None);
    }

    #[test]
    fn push_back_pop_back_is_lifo(values in proptest::collection::vec(any::<i32>(), 0..100)) {
        let mut deque = Deque::new();
        for v in &values {
            deque.push_back(*v);
        }
        for v in values.iter().rev() {
            prop_assert_eq!(deque.pop_back(), Some(*v));
        }
    }

    #[test]
    fn len_tracks_operations(values in proptest::collection::vec(any::<i32>(), 0..100)) {
        let mut deque = Deque::new();
        for (i, v) in values.iter().enumerate() {
            deque.push_back(*v);
            prop_assert_eq!(deque.len(), i + 1);
        }
    }

    #[test]
    fn rotate_left_then_right_is_identity(values in proptest::collection::vec(any::<i32>(), 1..50)) {
        let mut deque = Deque::new();
        for v in &values {
            deque.push_back(*v);
        }
        let n = values.len() / 2;
        deque.rotate_left(n);
        deque.rotate_right(n);
        for v in &values {
            prop_assert_eq!(deque.pop_front(), Some(*v));
        }
    }

    #[test]
    fn iter_collects_all_elements(values in proptest::collection::vec(any::<i32>(), 0..100)) {
        let mut deque = Deque::new();
        for v in &values {
            deque.push_back(*v);
        }
        let collected: Vec<i32> = deque.iter().copied().collect();
        prop_assert_eq!(collected, values);
    }
}

// ── Edge cases ──────────────────────────────────────────────────────────────

#[test]
fn empty_deque() {
    let mut deque: Deque<i32> = Deque::new();
    assert!(deque.is_empty());
    assert_eq!(deque.len(), 0);
    assert_eq!(deque.front(), None);
    assert_eq!(deque.back(), None);
    assert_eq!(deque.pop_front(), None);
    assert_eq!(deque.pop_back(), None);
}

#[test]
fn single_element_front() {
    let mut deque = Deque::new();
    deque.push_front(42);
    assert_eq!(deque.front(), Some(&42));
    assert_eq!(deque.back(), Some(&42));
    assert_eq!(deque.len(), 1);
}

#[test]
fn single_element_back() {
    let mut deque = Deque::new();
    deque.push_back(42);
    assert_eq!(deque.front(), Some(&42));
    assert_eq!(deque.back(), Some(&42));
}

#[test]
fn clear_empties_deque() {
    let mut deque = Deque::new();
    deque.push_back(1);
    deque.push_back(2);
    deque.clear();
    assert!(deque.is_empty());
    assert_eq!(deque.front(), None);
}

#[test]
fn with_capacity() {
    let deque: Deque<i32> = Deque::with_capacity(100);
    assert!(deque.is_empty());
}

#[test]
fn default_creates_empty() {
    let deque: Deque<i32> = Deque::default();
    assert!(deque.is_empty());
}

#[test]
fn split_at_zero() {
    let mut deque = Deque::new();
    deque.push_back(1);
    deque.push_back(2);
    let (left, right) = deque.split_at(0);
    assert_eq!(left.len(), 0);
    assert_eq!(right.len(), 2);
}

#[test]
fn split_at_end() {
    let mut deque = Deque::new();
    deque.push_back(1);
    deque.push_back(2);
    let (left, right) = deque.split_at(2);
    assert_eq!(left.len(), 2);
    assert_eq!(right.len(), 0);
}

#[test]
fn iter_mut_modifies_elements() {
    let mut deque = Deque::new();
    deque.push_back(1);
    deque.push_back(2);
    deque.push_back(3);
    for v in deque.iter_mut() {
        *v *= 10;
    }
    assert_eq!(deque.pop_front(), Some(10));
    assert_eq!(deque.pop_front(), Some(20));
    assert_eq!(deque.pop_front(), Some(30));
}

#[test]
fn error_display() {
    assert_eq!(DequeError::Empty.to_string(), "Deque is empty");
    assert_eq!(
        DequeError::IndexOutOfBounds.to_string(),
        "Index out of bounds"
    );
}

// ── Stress test ─────────────────────────────────────────────────────────────

#[test]
fn stress_alternating_ends() {
    let mut deque = Deque::new();
    for i in 0..5000 {
        if i % 2 == 0 {
            deque.push_front(i);
        } else {
            deque.push_back(i);
        }
    }
    assert_eq!(deque.len(), 5000);
    for _ in 0..2500 {
        deque.pop_front();
        deque.pop_back();
    }
    assert!(deque.is_empty());
}
