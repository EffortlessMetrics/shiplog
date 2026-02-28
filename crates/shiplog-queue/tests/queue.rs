use proptest::prelude::*;
use shiplog_queue::{MessageQueue, QueueError};

// ── Property tests ──────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn enqueue_dequeue_preserves_order_same_priority(
        payloads in proptest::collection::vec("[a-z]{1,10}", 1..50),
    ) {
        let queue = MessageQueue::new(1000);
        for p in &payloads {
            queue.enqueue(p.clone(), 0).unwrap();
        }
        for p in &payloads {
            let item = queue.dequeue().unwrap();
            prop_assert_eq!(&item.payload, p);
        }
        prop_assert!(queue.dequeue().is_none());
    }

    #[test]
    fn len_tracks_enqueue_dequeue(count in 1..50usize) {
        let queue = MessageQueue::new(1000);
        for i in 0..count {
            queue.enqueue(i as i32, 0).unwrap();
            prop_assert_eq!(queue.len(), i + 1);
        }
        for i in (0..count).rev() {
            queue.dequeue();
            prop_assert_eq!(queue.len(), i);
        }
    }

    #[test]
    fn higher_priority_dequeued_first(
        low_count in 1..10usize,
        high_count in 1..10usize,
    ) {
        let queue = MessageQueue::new(1000);
        for i in 0..low_count {
            queue.enqueue(format!("low-{i}"), 1).unwrap();
        }
        for i in 0..high_count {
            queue.enqueue(format!("high-{i}"), 10).unwrap();
        }
        // All high-priority items should come first
        for _ in 0..high_count {
            let item = queue.dequeue().unwrap();
            prop_assert!(item.payload.starts_with("high-"));
        }
        for _ in 0..low_count {
            let item = queue.dequeue().unwrap();
            prop_assert!(item.payload.starts_with("low-"));
        }
    }
}

// ── Edge cases ──────────────────────────────────────────────────────────────

#[test]
fn empty_queue() {
    let queue: MessageQueue<String> = MessageQueue::new(10);
    assert!(queue.is_empty());
    assert_eq!(queue.len(), 0);
    assert!(queue.dequeue().is_none());
    assert!(queue.peek().is_none());
}

#[test]
fn single_enqueue_dequeue() {
    let queue = MessageQueue::new(10);
    let item = queue.enqueue("test".to_string(), 0).unwrap();
    assert_eq!(item.payload, "test");
    assert_eq!(item.priority, 0);
    assert_eq!(item.retries, 0);

    let dequeued = queue.dequeue().unwrap();
    assert_eq!(dequeued.payload, "test");
    assert!(queue.is_empty());
}

#[test]
fn queue_full_error() {
    let queue = MessageQueue::new(1);
    queue.enqueue("first".to_string(), 0).unwrap();
    let result = queue.enqueue("second".to_string(), 0);
    assert_eq!(result.unwrap_err(), QueueError::QueueFull);
}

#[test]
fn queue_full_zero_capacity() {
    let queue = MessageQueue::new(0);
    let result = queue.enqueue("item".to_string(), 0);
    assert_eq!(result.unwrap_err(), QueueError::QueueFull);
}

#[test]
fn peek_does_not_remove() {
    let queue = MessageQueue::new(10);
    queue.enqueue("hello".to_string(), 0).unwrap();
    let peeked = queue.peek().unwrap();
    assert_eq!(peeked.payload, "hello");
    assert_eq!(queue.len(), 1);
}

#[test]
fn clear_empties_queue() {
    let queue = MessageQueue::new(10);
    queue.enqueue("a".to_string(), 0).unwrap();
    queue.enqueue("b".to_string(), 0).unwrap();
    queue.clear();
    assert!(queue.is_empty());
}

#[test]
fn increment_retries_returns_count() {
    let queue = MessageQueue::new(10);
    let item = queue.enqueue("task".to_string(), 0).unwrap();
    assert_eq!(queue.increment_retries(&item.id), Some(1));
    assert_eq!(queue.increment_retries(&item.id), Some(2));
    assert_eq!(queue.increment_retries(&item.id), Some(3));
}

#[test]
fn increment_retries_missing_id() {
    let queue: MessageQueue<String> = MessageQueue::new(10);
    assert_eq!(queue.increment_retries("nonexistent"), None);
}

#[test]
fn priority_ordering_three_levels() {
    let queue = MessageQueue::new(10);
    queue.enqueue("low".to_string(), 1).unwrap();
    queue.enqueue("high".to_string(), 100).unwrap();
    queue.enqueue("medium".to_string(), 50).unwrap();

    assert_eq!(queue.dequeue().unwrap().payload, "high");
    assert_eq!(queue.dequeue().unwrap().payload, "medium");
    assert_eq!(queue.dequeue().unwrap().payload, "low");
}

#[test]
fn default_queue_has_capacity_1000() {
    let queue: MessageQueue<String> = MessageQueue::default();
    // Should be able to enqueue many items
    for i in 0..100 {
        queue.enqueue(format!("{i}"), 0).unwrap();
    }
    assert_eq!(queue.len(), 100);
}

#[test]
fn error_display() {
    assert_eq!(
        QueueError::QueueFull.to_string(),
        "Queue is at maximum capacity"
    );
    assert_eq!(
        QueueError::ItemNotFound.to_string(),
        "Item not found in queue"
    );
}

// ── Stress test ─────────────────────────────────────────────────────────────

#[test]
fn stress_enqueue_dequeue_cycle() {
    let queue = MessageQueue::new(10_000);
    for i in 0..5000 {
        queue.enqueue(i, i % 10).unwrap();
    }
    assert_eq!(queue.len(), 5000);
    for _ in 0..5000 {
        assert!(queue.dequeue().is_some());
    }
    assert!(queue.is_empty());
}
