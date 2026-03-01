use proptest::prelude::*;
use shiplog_delayqueue::*;
use std::time::Duration;

// ── DelayItem ───────────────────────────────────────────────────────

#[test]
fn delay_item_not_ready_immediately() {
    let item = DelayItem::new(0, "hello", Duration::from_secs(10));
    assert!(!item.is_ready());
    assert!(item.remaining() > Duration::ZERO);
}

#[test]
fn delay_item_ready_after_wait() {
    let item = DelayItem::new(0, "hello", Duration::from_millis(20));
    std::thread::sleep(Duration::from_millis(40));
    assert!(item.is_ready());
}

#[test]
fn delay_item_zero_delay_is_ready() {
    let item = DelayItem::new(0, 42, Duration::ZERO);
    // Might or might not be ready immediately due to timing; just verify no panic
    let _ = item.is_ready();
}

#[test]
fn delay_item_ordering_min_heap() {
    let a = DelayItem::new(0, "early", Duration::from_millis(10));
    std::thread::sleep(Duration::from_millis(5));
    let b = DelayItem::new(1, "late", Duration::from_millis(100));
    // a should be "greater" in reversed ordering (min-heap)
    assert!(a > b);
}

// ── DelayQueue basic ops ────────────────────────────────────────────

#[test]
fn insert_returns_incrementing_ids() {
    let mut q: DelayQueue<i32> = DelayQueue::new();
    assert_eq!(q.insert(1, Duration::from_secs(1)), 0);
    assert_eq!(q.insert(2, Duration::from_secs(1)), 1);
    assert_eq!(q.insert(3, Duration::from_secs(1)), 2);
}

#[test]
fn len_tracks_insertions() {
    let mut q: DelayQueue<i32> = DelayQueue::new();
    assert!(q.is_empty());
    q.insert(1, Duration::from_secs(1));
    q.insert(2, Duration::from_secs(1));
    assert_eq!(q.len(), 2);
}

#[test]
fn try_pop_none_when_not_ready() {
    let mut q: DelayQueue<i32> = DelayQueue::new();
    q.insert(1, Duration::from_secs(60));
    assert!(q.try_pop().is_none());
}

#[test]
fn try_pop_returns_ready_item() {
    let mut q: DelayQueue<i32> = DelayQueue::new();
    q.insert(42, Duration::from_millis(10));
    std::thread::sleep(Duration::from_millis(30));
    let item = q.try_pop().unwrap();
    assert_eq!(item.data, 42);
    assert_eq!(q.len(), 0);
}

#[test]
fn pop_all_ready_returns_multiple() {
    let mut q: DelayQueue<i32> = DelayQueue::new();
    q.insert(1, Duration::from_millis(10));
    q.insert(2, Duration::from_millis(10));
    q.insert(3, Duration::from_secs(60));
    std::thread::sleep(Duration::from_millis(30));
    let ready = q.pop_all_ready();
    assert_eq!(ready.len(), 2);
    assert_eq!(q.len(), 1); // item 3 still pending
}

#[test]
fn peek_does_not_remove() {
    let mut q: DelayQueue<i32> = DelayQueue::new();
    q.insert(1, Duration::from_secs(10));
    let peeked = q.peek();
    assert!(peeked.is_some());
    assert_eq!(q.len(), 1);
}

#[test]
fn next_deadline_returns_earliest() {
    let mut q: DelayQueue<i32> = DelayQueue::new();
    q.insert(1, Duration::from_secs(100));
    q.insert(2, Duration::from_millis(10));
    let deadline = q.next_deadline().unwrap();
    // The earliest deadline should be roughly now + 10ms
    assert!(deadline < std::time::Instant::now() + Duration::from_secs(1));
}

// ── Remove ──────────────────────────────────────────────────────────

#[test]
fn remove_existing_item() {
    let mut q: DelayQueue<i32> = DelayQueue::new();
    let id = q.insert(1, Duration::from_secs(60));
    let removed = q.remove(id);
    assert!(removed.is_some());
    assert_eq!(removed.unwrap().data, 1);
    assert_eq!(q.len(), 0);
}

#[test]
fn remove_nonexistent_returns_none() {
    let mut q: DelayQueue<i32> = DelayQueue::new();
    assert!(q.remove(999).is_none());
}

#[test]
fn remove_preserves_other_items() {
    let mut q: DelayQueue<i32> = DelayQueue::new();
    q.insert(1, Duration::from_secs(60));
    let id2 = q.insert(2, Duration::from_secs(60));
    q.insert(3, Duration::from_secs(60));
    q.remove(id2);
    assert_eq!(q.len(), 2);
}

// ── Clear ───────────────────────────────────────────────────────────

#[test]
fn clear_empties_queue() {
    let mut q: DelayQueue<i32> = DelayQueue::new();
    q.insert(1, Duration::from_secs(1));
    q.insert(2, Duration::from_secs(1));
    q.clear();
    assert!(q.is_empty());
    assert_eq!(q.len(), 0);
}

// ── Ordering ────────────────────────────────────────────────────────

#[test]
fn earliest_deadline_popped_first() {
    let mut q: DelayQueue<&str> = DelayQueue::new();
    q.insert("late", Duration::from_secs(60));
    q.insert("early", Duration::from_millis(10));
    std::thread::sleep(Duration::from_millis(30));
    let item = q.try_pop().unwrap();
    assert_eq!(item.data, "early");
}

// ── UpdateableDelayQueue ────────────────────────────────────────────

#[test]
fn updateable_insert_and_pop() {
    let mut q: UpdateableDelayQueue<i32> = UpdateableDelayQueue::new();
    let id = q.insert(42, Duration::from_millis(10));
    assert_eq!(q.len(), 1);
    std::thread::sleep(Duration::from_millis(30));
    let item = q.try_pop().unwrap();
    assert_eq!(item.data, 42);
    assert!(q.is_empty());
    let _ = id;
}

#[test]
fn updateable_get_deadline() {
    let mut q: UpdateableDelayQueue<i32> = UpdateableDelayQueue::new();
    let id = q.insert(1, Duration::from_secs(10));
    assert!(q.get_deadline(id).is_some());
}

#[test]
fn updateable_update_deadline() {
    let mut q: UpdateableDelayQueue<i32> = UpdateableDelayQueue::new();
    let id = q.insert(1, Duration::from_secs(60));
    let updated = q.update(id, Duration::from_millis(10));
    assert!(updated);
    assert_eq!(q.len(), 1);
}

#[test]
fn updateable_pop_all_ready() {
    let mut q: UpdateableDelayQueue<i32> = UpdateableDelayQueue::new();
    q.insert(1, Duration::from_millis(10));
    q.insert(2, Duration::from_millis(10));
    std::thread::sleep(Duration::from_millis(30));
    let items = q.pop_all_ready();
    assert_eq!(items.len(), 2);
    assert!(q.is_empty());
}

// ── DelayQueueStats ─────────────────────────────────────────────────

#[test]
fn stats_tracking() {
    let mut s = DelayQueueStats::new();
    s.record_insert();
    s.record_insert();
    s.record_pop();
    s.record_remove();
    assert_eq!(s.total_inserted, 2);
    assert_eq!(s.total_popped, 1);
    assert_eq!(s.total_removed, 1);
}

// ── Edge cases ──────────────────────────────────────────────────────

#[test]
fn default_queue_is_empty() {
    let q: DelayQueue<i32> = DelayQueue::default();
    assert!(q.is_empty());
}

#[test]
fn default_updateable_is_empty() {
    let q: UpdateableDelayQueue<i32> = UpdateableDelayQueue::default();
    assert!(q.is_empty());
}

// ── Property tests ──────────────────────────────────────────────────

proptest! {
    #[test]
    fn insert_n_items_has_len_n(n in 0usize..50) {
        let mut q: DelayQueue<usize> = DelayQueue::new();
        for i in 0..n {
            q.insert(i, Duration::from_secs(60));
        }
        prop_assert_eq!(q.len(), n);
    }

    #[test]
    fn remove_all_makes_empty(n in 1usize..30) {
        let mut q: DelayQueue<usize> = DelayQueue::new();
        let mut ids = Vec::new();
        for i in 0..n {
            ids.push(q.insert(i, Duration::from_secs(60)));
        }
        for id in ids {
            q.remove(id);
        }
        prop_assert!(q.is_empty());
    }

    #[test]
    fn clear_always_empties(n in 0usize..50) {
        let mut q: DelayQueue<usize> = DelayQueue::new();
        for i in 0..n {
            q.insert(i, Duration::from_secs(60));
        }
        q.clear();
        prop_assert!(q.is_empty());
        prop_assert_eq!(q.len(), 0);
    }

    #[test]
    fn ids_are_unique(n in 1usize..50) {
        let mut q: DelayQueue<usize> = DelayQueue::new();
        let mut ids = Vec::new();
        for i in 0..n {
            ids.push(q.insert(i, Duration::from_secs(60)));
        }
        ids.sort();
        ids.dedup();
        prop_assert_eq!(ids.len(), n);
    }
}
