use proptest::prelude::*;
use shiplog_ring::{RingBuffer, RingBuilder, RingConfig};

// ── Property tests ──────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn push_pop_fifo_order(values in proptest::collection::vec(any::<i32>(), 1..100)) {
        let cap = values.len() + 10;
        let mut ring = RingBuffer::new(cap);
        for v in &values {
            ring.push(*v);
        }
        for v in &values {
            prop_assert_eq!(ring.pop(), Some(*v));
        }
        prop_assert_eq!(ring.pop(), None);
    }

    #[test]
    fn len_never_exceeds_capacity(
        cap in 1..50usize,
        values in proptest::collection::vec(any::<i32>(), 0..200),
    ) {
        let mut ring = RingBuffer::new(cap);
        for v in &values {
            ring.push(*v);
            prop_assert!(ring.len() <= cap);
        }
    }

    #[test]
    fn to_vec_length_matches_len(
        cap in 1..50usize,
        values in proptest::collection::vec(any::<i32>(), 0..100),
    ) {
        let mut ring = RingBuffer::new(cap);
        for v in &values {
            ring.push(*v);
        }
        prop_assert_eq!(ring.to_vec().len(), ring.len());
    }

    #[test]
    fn overflow_returns_oldest_item(
        cap in 1..20usize,
        values in proptest::collection::vec(any::<i32>(), 0..100),
    ) {
        let mut ring = RingBuffer::new(cap);
        let mut expected_queue: std::collections::VecDeque<i32> = std::collections::VecDeque::new();
        for v in &values {
            let overwritten = ring.push(*v);
            if expected_queue.len() == cap {
                let oldest = expected_queue.pop_front();
                prop_assert_eq!(overwritten, oldest);
            } else {
                prop_assert_eq!(overwritten, None);
            }
            expected_queue.push_back(*v);
            if expected_queue.len() > cap {
                expected_queue.pop_front();
            }
        }
    }

    #[test]
    fn available_space_invariant(cap in 1..50usize, pushes in 0..100usize) {
        let mut ring: RingBuffer<i32> = RingBuffer::new(cap);
        for i in 0..pushes {
            ring.push(i as i32);
        }
        prop_assert_eq!(ring.len() + ring.available_space(), ring.capacity());
    }
}

// ── Edge cases ──────────────────────────────────────────────────────────────

#[test]
fn empty_buffer() {
    let ring: RingBuffer<i32> = RingBuffer::new(5);
    assert!(ring.is_empty());
    assert!(!ring.is_full());
    assert_eq!(ring.len(), 0);
    assert_eq!(ring.capacity(), 5);
    assert_eq!(ring.available_space(), 5);
    assert_eq!(ring.front(), None);
    assert_eq!(ring.back(), None);
    assert!(ring.to_vec().is_empty());
}

#[test]
fn pop_from_empty() {
    let mut ring: RingBuffer<i32> = RingBuffer::new(3);
    assert_eq!(ring.pop(), None);
}

#[test]
fn single_element() {
    let mut ring = RingBuffer::new(5);
    ring.push(42);
    assert_eq!(ring.front(), Some(&42));
    assert_eq!(ring.back(), Some(&42));
    assert_eq!(ring.len(), 1);
    assert_eq!(ring.pop(), Some(42));
    assert!(ring.is_empty());
}

#[test]
fn capacity_one() {
    let mut ring = RingBuffer::new(1);
    assert!(ring.push(1).is_none());
    assert!(ring.is_full());
    assert_eq!(ring.push(2), Some(1));
    assert_eq!(ring.front(), Some(&2));
    assert_eq!(ring.pop(), Some(2));
    assert!(ring.is_empty());
}

#[test]
fn from_vec_overflow() {
    let ring = RingBuffer::from_vec(3, vec![1, 2, 3, 4, 5]);
    assert_eq!(ring.len(), 3);
    assert_eq!(ring.to_vec(), vec![3, 4, 5]);
}

#[test]
fn clear_resets_state() {
    let mut ring = RingBuffer::new(5);
    ring.push(1);
    ring.push(2);
    ring.push(3);
    ring.clear();
    assert!(ring.is_empty());
    assert_eq!(ring.len(), 0);
    assert_eq!(ring.front(), None);
    assert_eq!(ring.back(), None);
    assert_eq!(ring.available_space(), 5);
}

#[test]
fn wrap_around_preserves_order() {
    let mut ring = RingBuffer::new(3);
    ring.push(1);
    ring.push(2);
    ring.push(3);
    ring.pop(); // remove 1
    ring.pop(); // remove 2
    ring.push(4);
    ring.push(5);
    assert_eq!(ring.to_vec(), vec![3, 4, 5]);
}

#[test]
fn default_has_capacity_64() {
    let ring: RingBuffer<i32> = RingBuffer::default();
    assert_eq!(ring.capacity(), 64);
}

#[test]
fn builder_produces_correct_capacity() {
    let ring: RingBuffer<i32> = RingBuilder::new().capacity(10).build();
    assert_eq!(ring.capacity(), 10);
}

#[test]
fn config_default_values() {
    let config = RingConfig::default();
    assert_eq!(config.capacity, 64);
    assert!(config.overwrite);
}

#[test]
fn builder_default() {
    let ring: RingBuffer<i32> = RingBuilder::default().build();
    assert_eq!(ring.capacity(), 64);
}

// ── Stress test ─────────────────────────────────────────────────────────────

#[test]
fn stress_many_push_pop_cycles() {
    let mut ring = RingBuffer::new(10);
    for cycle in 0..100 {
        for j in 0..10 {
            ring.push(cycle * 10 + j);
        }
        assert!(ring.is_full());
        for _ in 0..5 {
            ring.pop();
        }
        assert_eq!(ring.len(), 5);
    }
}
