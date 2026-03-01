use proptest::prelude::*;
use shiplog_priorityqueue::{PriorityItem, PriorityQueue, PriorityQueueError};

// ── Property tests ──────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn pop_returns_highest_priority_first(
        items in proptest::collection::vec((any::<i32>(), -100..100i32), 1..50),
    ) {
        let mut pq = PriorityQueue::new(1000);
        for (val, pri) in &items {
            pq.push(*val, *pri);
        }
        let mut prev_priority = i32::MAX;
        while let Some(item) = pq.pop() {
            prop_assert!(item.priority <= prev_priority,
                "out of order: {} after {}", item.priority, prev_priority);
            prev_priority = item.priority;
        }
    }

    #[test]
    fn len_matches_push_count(count in 0..100usize) {
        let mut pq = PriorityQueue::new(1000);
        for i in 0..count {
            pq.push(i as i32, i as i32);
        }
        prop_assert_eq!(pq.len(), count);
    }

    #[test]
    fn peek_returns_same_as_first_pop(
        items in proptest::collection::vec((any::<i32>(), -50..50i32), 1..50),
    ) {
        let mut pq = PriorityQueue::new(1000);
        for (val, pri) in &items {
            pq.push(*val, *pri);
        }
        let peeked = pq.peek().cloned();
        let popped = pq.pop();
        prop_assert_eq!(peeked, popped);
    }

    #[test]
    fn fifo_among_equal_priorities(values in proptest::collection::vec(any::<i32>(), 1..50)) {
        let mut pq = PriorityQueue::new(1000);
        for v in &values {
            pq.push(*v, 5);
        }
        for v in &values {
            let item = pq.pop().unwrap();
            prop_assert_eq!(item.item, *v);
        }
    }
}

// ── Edge cases ──────────────────────────────────────────────────────────────

#[test]
fn empty_queue() {
    let mut pq: PriorityQueue<i32> = PriorityQueue::new(10);
    assert!(pq.is_empty());
    assert_eq!(pq.len(), 0);
    assert_eq!(pq.pop(), None);
    assert_eq!(pq.peek(), None);
}

#[test]
fn single_element() {
    let mut pq = PriorityQueue::new(10);
    let pushed = pq.push(42, 5).unwrap();
    assert_eq!(pushed.item, 42);
    assert_eq!(pushed.priority, 5);
    assert_eq!(pq.len(), 1);

    let popped = pq.pop().unwrap();
    assert_eq!(popped.item, 42);
    assert!(pq.is_empty());
}

#[test]
fn full_queue_returns_none() {
    let mut pq = PriorityQueue::new(2);
    pq.push(1, 1).unwrap();
    pq.push(2, 2).unwrap();
    assert!(pq.is_full());
    assert!(pq.push(3, 3).is_none());
}

#[test]
fn clear_empties_queue() {
    let mut pq = PriorityQueue::new(10);
    pq.push(1, 1);
    pq.push(2, 2);
    pq.clear();
    assert!(pq.is_empty());
    assert_eq!(pq.len(), 0);
}

#[test]
fn default_has_unlimited_size() {
    let pq: PriorityQueue<i32> = PriorityQueue::default();
    assert_eq!(pq.max_size(), usize::MAX);
}

#[test]
fn peek_does_not_remove() {
    let mut pq = PriorityQueue::new(10);
    pq.push(42, 1);
    assert_eq!(pq.peek().unwrap().item, 42);
    assert_eq!(pq.len(), 1);
}

#[test]
fn peek_mut_allows_modification() {
    let mut pq = PriorityQueue::new(10);
    pq.push(42, 1);
    pq.peek_mut().unwrap().item = 99;
    assert_eq!(pq.peek().unwrap().item, 99);
}

#[test]
fn priority_item_new() {
    let item = PriorityItem::new("test", 5);
    assert_eq!(item.item, "test");
    assert_eq!(item.priority, 5);
    assert_eq!(item.seq, 0);
}

#[test]
fn priority_item_with_seq() {
    let item = PriorityItem::with_seq("test", 5, 42);
    assert_eq!(item.seq, 42);
}

#[test]
fn iter_visits_all() {
    let mut pq = PriorityQueue::new(10);
    pq.push(1, 10);
    pq.push(2, 20);
    pq.push(3, 30);
    let total: i32 = pq.iter().map(|p| p.item).sum();
    assert_eq!(total, 6);
}

#[test]
fn negative_priorities() {
    let mut pq = PriorityQueue::new(10);
    pq.push("a", -10);
    pq.push("b", -1);
    pq.push("c", -100);
    assert_eq!(pq.pop().unwrap().item, "b"); // -1 is highest
    assert_eq!(pq.pop().unwrap().item, "a"); // -10
    assert_eq!(pq.pop().unwrap().item, "c"); // -100
}

#[test]
fn error_display() {
    assert_eq!(
        PriorityQueueError::Empty.to_string(),
        "Priority queue is empty"
    );
    assert_eq!(
        PriorityQueueError::Full.to_string(),
        "Priority queue is full"
    );
}

// ── Stress test ─────────────────────────────────────────────────────────────

#[test]
fn stress_large_queue_ordering() {
    let mut pq = PriorityQueue::new(10_000);
    for i in 0..5000 {
        pq.push(i, i % 100);
    }
    assert_eq!(pq.len(), 5000);

    let mut prev = i32::MAX;
    while let Some(item) = pq.pop() {
        assert!(item.priority <= prev);
        prev = item.priority;
    }
}

#[test]
fn stress_push_pop_interleaved() {
    let mut pq = PriorityQueue::new(10_000);
    for i in 0..1000 {
        pq.push(i, i);
        if i % 3 == 0 {
            pq.pop();
        }
    }
    let mut prev = i32::MAX;
    while let Some(item) = pq.pop() {
        assert!(item.priority <= prev);
        prev = item.priority;
    }
}
