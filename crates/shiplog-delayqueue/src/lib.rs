//! Delayed queue implementation for shiplog.
//!
//! This crate provides delayed queue data structures for managing
//! items that should be processed after a specified delay.

use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;
use std::time::{Duration, Instant};

/// An entry in the delay queue
#[derive(Debug, Clone, Eq)]
pub struct DelayItem<T: Eq> {
    pub id: u64,
    pub data: T,
    pub deadline: Instant,
}

impl<T: Eq> DelayItem<T> {
    pub fn new(id: u64, data: T, delay: Duration) -> Self {
        Self {
            id,
            data,
            deadline: Instant::now() + delay,
        }
    }

    pub fn is_ready(&self) -> bool {
        Instant::now() >= self.deadline
    }

    pub fn remaining(&self) -> Duration {
        self.deadline.saturating_duration_since(Instant::now())
    }
}

/// Ordering for the min-heap (earliest deadline first)
impl<T: Eq> Ord for DelayItem<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse order for min-heap (earliest deadline at top)
        other.deadline.cmp(&self.deadline)
    }
}

impl<T: Eq> PartialOrd for DelayItem<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Eq> PartialEq for DelayItem<T> {
    fn eq(&self, other: &Self) -> bool {
        self.deadline == other.deadline
    }
}

/// A delayed queue that returns items when their delay has passed
pub struct DelayQueue<T: Eq> {
    heap: BinaryHeap<DelayItem<T>>,
    next_id: u64,
    len: usize,
}

impl<T: Eq> DelayQueue<T> {
    pub fn new() -> Self {
        Self {
            heap: BinaryHeap::new(),
            next_id: 0,
            len: 0,
        }
    }

    /// Insert an item with a delay
    pub fn insert(&mut self, data: T, delay: Duration) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        
        let item = DelayItem::new(id, data, delay);
        self.heap.push(item);
        self.len += 1;
        
        id
    }

    /// Try to pop an item that's ready (deadline passed)
    pub fn try_pop(&mut self) -> Option<DelayItem<T>> {
        if let Some(item) = self.heap.peek() {
            if item.is_ready() {
                self.len -= 1;
                return self.heap.pop();
            }
        }
        None
    }

    /// Pop all ready items
    pub fn pop_all_ready(&mut self) -> Vec<DelayItem<T>> {
        let mut ready = Vec::new();
        
        while let Some(item) = self.heap.peek() {
            if item.is_ready() {
                self.len -= 1;
                ready.push(self.heap.pop().unwrap());
            } else {
                break;
            }
        }
        
        ready
    }

    /// Peek at the earliest deadline without removing
    pub fn peek(&self) -> Option<&DelayItem<T>> {
        self.heap.peek()
    }

    /// Get the next deadline
    pub fn next_deadline(&self) -> Option<Instant> {
        self.heap.peek().map(|item| item.deadline)
    }

    /// Get time until next item is ready
    pub fn time_until_ready(&self) -> Option<Duration> {
        self.heap.peek().map(|item| {
            let elapsed = item.deadline.elapsed();
            if elapsed.is_zero() {
                Duration::ZERO
            } else {
                elapsed
            }
        })
    }

    /// Remove a specific item by id
    pub fn remove(&mut self, id: u64) -> Option<DelayItem<T>> {
        // Note: BinaryHeap doesn't support efficient removal by id
        // This is O(n) - for frequent removals, consider a different structure
        let mut found = None;
        let mut temp = BinaryHeap::new();
        
        while let Some(item) = self.heap.pop() {
            if item.id == id {
                found = Some(item);
                self.len -= 1;
                break;
            } else {
                temp.push(item);
            }
        }
        
        // Put back remaining items
        while let Some(item) = temp.pop() {
            self.heap.push(item);
        }
        
        found
    }

    /// Get the number of items in the queue
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Clear all items from the queue
    pub fn clear(&mut self) {
        self.heap.clear();
        self.len = 0;
    }
}

impl<T: Eq> Default for DelayQueue<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// A delay queue that supports updating item deadlines
pub struct UpdateableDelayQueue<T: Eq> {
    queue: DelayQueue<T>,
    index: HashMap<u64, Instant>,
}

impl<T: Eq> UpdateableDelayQueue<T> {
    pub fn new() -> Self {
        Self {
            queue: DelayQueue::new(),
            index: HashMap::new(),
        }
    }

    /// Insert an item with delay and track it
    pub fn insert(&mut self, data: T, delay: Duration) -> u64 {
        let deadline = Instant::now() + delay;
        let id = self.queue.insert(data, delay);
        self.index.insert(id, deadline);
        id
    }

    /// Update the deadline for an existing item
    pub fn update(&mut self, id: u64, new_delay: Duration) -> bool {
        // Remove old item
        if let Some(old_item) = self.queue.remove(id) {
            let new_deadline = Instant::now() + new_delay;
            self.index.insert(id, new_deadline);
            // Re-insert with new delay
            self.queue.insert(old_item.data, new_delay);
            true
        } else {
            false
        }
    }

    /// Try to pop a ready item
    pub fn try_pop(&mut self) -> Option<DelayItem<T>> {
        if let Some(item) = self.queue.try_pop() {
            self.index.remove(&item.id);
            Some(item)
        } else {
            None
        }
    }

    /// Pop all ready items
    pub fn pop_all_ready(&mut self) -> Vec<DelayItem<T>> {
        let items = self.queue.pop_all_ready();
        for item in &items {
            self.index.remove(&item.id);
        }
        items
    }

    /// Get the deadline for an item
    pub fn get_deadline(&self, id: u64) -> Option<Instant> {
        self.index.get(&id).copied()
    }

    /// Get queue length
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

impl<T: Eq> Default for UpdateableDelayQueue<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics for delay queue
#[derive(Debug, Default, Clone)]
pub struct DelayQueueStats {
    pub total_inserted: u64,
    pub total_popped: u64,
    pub total_removed: u64,
}

impl DelayQueueStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_insert(&mut self) {
        self.total_inserted += 1;
    }

    pub fn record_pop(&mut self) {
        self.total_popped += 1;
    }

    pub fn record_remove(&mut self) {
        self.total_removed += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_delay_item() {
        let item = DelayItem::new(1, "test", Duration::from_millis(100));
        
        assert_eq!(item.id, 1);
        assert!(!item.is_ready());
    }

    #[test]
    fn test_delay_queue_insert() {
        let mut queue: DelayQueue<i32> = DelayQueue::new();
        
        let id1 = queue.insert(1, Duration::from_millis(100));
        let id2 = queue.insert(2, Duration::from_millis(50));
        
        assert_eq!(id1, 0);
        assert_eq!(id2, 1);
        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn test_delay_queue_pop_not_ready() {
        let mut queue: DelayQueue<i32> = DelayQueue::new();
        
        queue.insert(1, Duration::from_millis(100));
        
        let item = queue.try_pop();
        assert!(item.is_none());
    }

    #[test]
    fn test_delay_queue_pop_ready() {
        let mut queue: DelayQueue<i32> = DelayQueue::new();
        
        queue.insert(1, Duration::from_millis(10));
        
        thread::sleep(Duration::from_millis(20));
        
        let item = queue.try_pop();
        assert!(item.is_some());
        assert_eq!(item.unwrap().data, 1);
    }

    #[test]
    fn test_delay_queue_pop_all_ready() {
        let mut queue: DelayQueue<i32> = DelayQueue::new();
        
        queue.insert(1, Duration::from_millis(10));
        queue.insert(2, Duration::from_millis(10));
        
        thread::sleep(Duration::from_millis(20));
        
        let items = queue.pop_all_ready();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_delay_queue_peek() {
        let mut queue: DelayQueue<i32> = DelayQueue::new();
        
        queue.insert(1, Duration::from_millis(100));
        
        let peeked = queue.peek();
        assert!(peeked.is_some());
        assert_eq!(peeked.unwrap().data, 1);
        
        assert_eq!(queue.len(), 1); // Peek doesn't remove
    }

    #[test]
    fn test_delay_queue_remove() {
        let mut queue: DelayQueue<i32> = DelayQueue::new();
        
        let id = queue.insert(1, Duration::from_millis(100));
        
        let removed = queue.remove(id);
        assert!(removed.is_some());
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn test_delay_queue_ordering() {
        let mut queue: DelayQueue<i32> = DelayQueue::new();
        
        queue.insert(1, Duration::from_millis(100));
        queue.insert(2, Duration::from_millis(50));
        
        let deadline = queue.next_deadline().unwrap();
        
        thread::sleep(Duration::from_millis(60));
        
        let item = queue.try_pop();
        assert!(item.is_some());
        assert_eq!(item.unwrap().data, 2); // 50ms delay should come first
    }

    #[test]
    fn test_updateable_delay_queue() {
        let mut queue: UpdateableDelayQueue<i32> = UpdateableDelayQueue::new();
        
        let id = queue.insert(1, Duration::from_millis(100));
        
        // Update the deadline
        let updated = queue.update(id, Duration::from_millis(200));
        assert!(updated);
        
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn test_updateable_delay_queue_get_deadline() {
        let mut queue: UpdateableDelayQueue<i32> = UpdateableDelayQueue::new();
        
        let id = queue.insert(1, Duration::from_millis(100));
        
        let deadline = queue.get_deadline(id);
        assert!(deadline.is_some());
    }

    #[test]
    fn test_delay_queue_stats() {
        let mut stats = DelayQueueStats::new();
        
        stats.record_insert();
        stats.record_insert();
        stats.record_pop();
        stats.record_remove();
        
        assert_eq!(stats.total_inserted, 2);
        assert_eq!(stats.total_popped, 1);
        assert_eq!(stats.total_removed, 1);
    }

    #[test]
    fn test_delay_queue_clear() {
        let mut queue: DelayQueue<i32> = DelayQueue::new();
        
        queue.insert(1, Duration::from_millis(100));
        queue.insert(2, Duration::from_millis(100));
        
        queue.clear();
        
        assert!(queue.is_empty());
    }
}
