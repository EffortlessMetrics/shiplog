//! Priority queue implementation for shiplog.
//!
//! This crate provides a priority queue implementation for handling
//! items with different priority levels.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// A priority queue item with an associated priority.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PriorityItem<T> {
    /// The item data.
    pub item: T,
    /// The priority value (higher = more important).
    pub priority: i32,
    /// Insertion order for FIFO tie-breaking.
    pub seq: usize,
}

impl<T> PriorityItem<T> {
    /// Create a new priority item.
    pub fn new(item: T, priority: i32) -> Self {
        Self {
            item,
            priority,
            seq: 0,
        }
    }

    /// Create with custom sequence number.
    pub fn with_seq(item: T, priority: i32, seq: usize) -> Self {
        Self { item, priority, seq }
    }
}

/// A priority queue implementation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityQueue<T> {
    data: Vec<PriorityItem<T>>,
    max_size: usize,
    seq_counter: usize,
}

impl<T: Clone> Default for PriorityQueue<T> {
    fn default() -> Self {
        Self::new(usize::MAX)
    }
}

impl<T: Clone> PriorityQueue<T> {
    /// Create a new priority queue with unlimited size.
    pub fn new(max_size: usize) -> Self {
        Self {
            data: Vec::new(),
            max_size,
            seq_counter: 0,
        }
    }

    /// Get the number of elements.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Check if full.
    pub fn is_full(&self) -> bool {
        self.data.len() >= self.max_size
    }

    /// Get the maximum size.
    pub fn max_size(&self) -> usize {
        self.max_size
    }

    /// Push an item with the given priority.
    /// Returns the pushed item or None if queue is full.
    pub fn push(&mut self, item: T, priority: i32) -> Option<PriorityItem<T>> {
        if self.data.len() >= self.max_size {
            return None;
        }

        let p_item = PriorityItem {
            item,
            priority,
            seq: self.seq_counter,
        };
        self.seq_counter += 1;

        let idx = self.data.len();
        self.data.push(p_item.clone());
        self.bubble_up(idx);

        Some(p_item)
    }

    /// Pop the highest priority item.
    pub fn pop(&mut self) -> Option<PriorityItem<T>> {
        if self.data.is_empty() {
            return None;
        }

        let result = self.data.first().cloned();
        
        if self.data.len() == 1 {
            self.data.pop();
        } else {
            let last = self.data.pop().unwrap();
            self.data[0] = last;
            self.bubble_down(0);
        }

        result
    }

    /// Peek at the highest priority item without removing.
    pub fn peek(&self) -> Option<&PriorityItem<T>> {
        self.data.first()
    }

    /// Peek at the highest priority item mutably.
    pub fn peek_mut(&mut self) -> Option<&mut PriorityItem<T>> {
        self.data.first_mut()
    }

    /// Clear all items from the queue.
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Get an iterator over the items.
    pub fn iter(&self) -> impl Iterator<Item = &PriorityItem<T>> {
        self.data.iter()
    }

    fn bubble_up(&mut self, mut idx: usize) {
        while idx > 0 {
            let parent = (idx - 1) / 2;
            
            if self.cmp(idx, parent) != Ordering::Greater {
                break;
            }
            
            self.data.swap(idx, parent);
            idx = parent;
        }
    }

    fn bubble_down(&mut self, mut idx: usize) {
        let n = self.data.len();
        
        loop {
            let left = 2 * idx + 1;
            let right = 2 * idx + 2;
            let mut largest = idx;
            
            if left < n && self.cmp(left, largest) == Ordering::Greater {
                largest = left;
            }
            
            if right < n && self.cmp(right, largest) == Ordering::Greater {
                largest = right;
            }
            
            if largest == idx {
                break;
            }
            
            self.data.swap(idx, largest);
            idx = largest;
        }
    }

    fn cmp(&self, i: usize, j: usize) -> Ordering {
        // Higher priority first, then earlier insertion (lower seq) first
        match self.data[i].priority.cmp(&self.data[j].priority) {
            Ordering::Equal => self.data[j].seq.cmp(&self.data[i].seq),
            other => other,
        }
    }
}

/// Priority queue errors
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PriorityQueueError {
    Empty,
    Full,
}

impl std::fmt::Display for PriorityQueueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PriorityQueueError::Empty => write!(f, "Priority queue is empty"),
            PriorityQueueError::Full => write!(f, "Priority queue is full"),
        }
    }
}

impl std::error::Error for PriorityQueueError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_queue_push_pop() {
        let mut pq: PriorityQueue<&str> = PriorityQueue::new(10);
        
        pq.push("low", 1).unwrap();
        pq.push("high", 10).unwrap();
        pq.push("medium", 5).unwrap();
        
        assert_eq!(pq.len(), 3);
        
        let item = pq.pop().unwrap();
        assert_eq!(item.item, "high");
        assert_eq!(item.priority, 10);
    }

    #[test]
    fn test_priority_queue_ordering() {
        let mut pq: PriorityQueue<&str> = PriorityQueue::new(10);
        
        pq.push("a", 1).unwrap();
        pq.push("b", 2).unwrap();
        pq.push("c", 3).unwrap();
        
        assert_eq!(pq.pop().unwrap().item, "c"); // Highest priority
        assert_eq!(pq.pop().unwrap().item, "b");
        assert_eq!(pq.pop().unwrap().item, "a");
    }

    #[test]
    fn test_priority_queue_same_priority_fifo() {
        let mut pq: PriorityQueue<&str> = PriorityQueue::new(10);
        
        pq.push("first", 5).unwrap();
        pq.push("second", 5).unwrap();
        pq.push("third", 5).unwrap();
        
        assert_eq!(pq.pop().unwrap().item, "first");
        assert_eq!(pq.pop().unwrap().item, "second");
        assert_eq!(pq.pop().unwrap().item, "third");
    }

    #[test]
    fn test_priority_queue_peek() {
        let mut pq: PriorityQueue<&str> = PriorityQueue::new(10);
        
        pq.push("low", 1).unwrap();
        pq.push("high", 10).unwrap();
        
        let peeked = pq.peek().unwrap();
        assert_eq!(peeked.item, "high");
        
        assert_eq!(pq.len(), 2);
    }

    #[test]
    fn test_priority_queue_peek_mut() {
        let mut pq: PriorityQueue<i32> = PriorityQueue::new(10);
        
        pq.push(10, 1).unwrap();
        pq.push(20, 5).unwrap();
        
        // Note: peek_mut allows mutation but doesn't re-heapify
        // The highest priority item (20) is still at the front
        assert_eq!(pq.peek().unwrap().item, 20);
    }

    #[test]
    fn test_priority_queue_empty() {
        let mut pq: PriorityQueue<i32> = PriorityQueue::new(10);
        
        assert!(pq.is_empty());
        assert_eq!(pq.pop(), None);
        assert_eq!(pq.peek(), None);
    }

    #[test]
    fn test_priority_queue_full() {
        let mut pq: PriorityQueue<i32> = PriorityQueue::new(2);
        
        pq.push(1, 1).unwrap();
        pq.push(2, 2).unwrap();
        
        assert!(pq.is_full());
        
        let result = pq.push(3, 3);
        assert!(result.is_none());
    }

    #[test]
    fn test_priority_queue_clear() {
        let mut pq: PriorityQueue<i32> = PriorityQueue::new(10);
        
        pq.push(1, 1).unwrap();
        pq.push(2, 2).unwrap();
        
        assert!(!pq.is_empty());
        
        pq.clear();
        assert!(pq.is_empty());
    }

    #[test]
    fn test_priority_queue_max_size() {
        let pq: PriorityQueue<i32> = PriorityQueue::new(100);
        
        assert_eq!(pq.max_size(), 100);
    }

    #[test]
    fn test_priority_queue_iter() {
        let mut pq: PriorityQueue<i32> = PriorityQueue::new(10);
        
        pq.push(1, 1).unwrap();
        pq.push(2, 2).unwrap();
        
        let sum_priority: i32 = pq.iter().map(|p| p.priority).sum();
        assert_eq!(sum_priority, 3);
    }

    #[test]
    fn test_priority_queue_unlimited() {
        let mut pq: PriorityQueue<i32> = PriorityQueue::new(usize::MAX);
        
        for i in 0..1000 {
            pq.push(i, i as i32).unwrap();
        }
        
        assert_eq!(pq.len(), 1000);
    }

    #[test]
    fn test_priority_item_new() {
        let item = PriorityItem::new("test", 5);
        
        assert_eq!(item.item, "test");
        assert_eq!(item.priority, 5);
    }

    #[test]
    fn test_priority_queue_error_display() {
        assert_eq!(PriorityQueueError::Empty.to_string(), "Priority queue is empty");
        assert_eq!(PriorityQueueError::Full.to_string(), "Priority queue is full");
    }
}
