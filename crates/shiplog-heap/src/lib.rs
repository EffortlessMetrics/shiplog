//! Binary heap utilities for shiplog.
//!
//! This crate provides binary heap implementations for efficient
//! priority queue operations and heap sort.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// A binary heap implementation (max-heap by default).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryHeap<T> {
    data: Vec<T>,
}

/// Ordering mode for the heap
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeapOrder {
    Min,
    Max,
}

impl<T: Ord + Clone> Default for BinaryHeap<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Ord + Clone> BinaryHeap<T> {
    /// Create a new empty binary heap.
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    /// Create a new binary heap with a specific ordering.
    pub fn with_order(order: HeapOrder) -> BinaryHeapWithOrder<T> {
        BinaryHeapWithOrder {
            data: Vec::new(),
            order,
        }
    }

    /// Create a binary heap from a vector.
    pub fn from_vec(vec: Vec<T>) -> Self {
        let mut heap = Self { data: vec };
        heap.heapify();
        heap
    }

    /// Get the number of elements in the heap.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if the heap is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get the top element without removing it.
    pub fn peek(&self) -> Option<&T> {
        self.data.first()
    }

    /// Insert an element into the heap.
    pub fn push(&mut self, item: T) {
        self.data.push(item);
        self.bubble_up(self.data.len() - 1);
    }

    /// Remove and return the top element.
    pub fn pop(&mut self) -> Option<T> {
        if self.data.is_empty() {
            return None;
        }
        
        let top = self.data.first().cloned();
        let last = self.data.pop();
        
        if !self.data.is_empty() {
            self.data[0] = last.unwrap();
            self.bubble_down(0);
        }
        
        top
    }

    /// Convert the heap to a sorted vector.
    pub fn into_sorted_vec(mut self) -> Vec<T> {
        let mut result = Vec::with_capacity(self.data.len());
        
        while let Some(item) = self.pop() {
            result.push(item);
        }
        
        result
    }

    /// Get an iterator over the heap elements.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.data.iter()
    }

    fn heapify(&mut self) {
        let n = self.data.len();
        for i in (0..n / 2).rev() {
            self.bubble_down(i);
        }
    }

    fn bubble_up(&mut self, mut idx: usize) {
        while idx > 0 {
            let parent = (idx - 1) / 2;
            if self.data[idx] <= self.data[parent] {
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
            
            if left < n && self.data[left] > self.data[largest] {
                largest = left;
            }
            
            if right < n && self.data[right] > self.data[largest] {
                largest = right;
            }
            
            if largest == idx {
                break;
            }
            
            self.data.swap(idx, largest);
            idx = largest;
        }
    }
}

/// A binary heap with configurable ordering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryHeapWithOrder<T> {
    data: Vec<T>,
    order: HeapOrder,
}

impl<T: Ord + Clone> BinaryHeapWithOrder<T> {
    /// Create a new empty binary heap with specified ordering.
    pub fn new(order: HeapOrder) -> Self {
        Self {
            data: Vec::new(),
            order,
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

    /// Peek at top element.
    pub fn peek(&self) -> Option<&T> {
        self.data.first()
    }

    /// Insert an element.
    pub fn push(&mut self, item: T) {
        self.data.push(item);
        self.bubble_up(self.data.len() - 1);
    }

    /// Pop top element.
    pub fn pop(&mut self) -> Option<T> {
        if self.data.is_empty() {
            return None;
        }

        let top = self.data.first().cloned();
        let last = self.data.pop();

        if !self.data.is_empty() {
            self.data[0] = last.unwrap();
            self.bubble_down(0);
        }

        top
    }

    fn bubble_up(&mut self, mut idx: usize) {
        loop {
            let parent = (idx.saturating_sub(1)) / 2;
            if idx == 0 || self.cmp(&self.data[idx], &self.data[parent]) != Ordering::Greater {
                break;
            }
            self.data.swap(idx, parent);
            idx = parent;
        }
    }

    fn bubble_down(&mut self, mut idx: usize) {
        loop {
            let left = 2 * idx + 1;
            let right = 2 * idx + 2;
            let mut smallest_or_largest = idx;

            if left < self.data.len() && self.cmp(&self.data[left], &self.data[smallest_or_largest]) == Ordering::Greater {
                smallest_or_largest = left;
            }

            if right < self.data.len() && self.cmp(&self.data[right], &self.data[smallest_or_largest]) == Ordering::Greater {
                smallest_or_largest = right;
            }

            if smallest_or_largest == idx {
                break;
            }

            self.data.swap(idx, smallest_or_largest);
            idx = smallest_or_largest;
        }
    }

    fn cmp(&self, a: &T, b: &T) -> Ordering {
        match self.order {
            HeapOrder::Max => a.cmp(b),  // Larger is "greater" for max-heap
            HeapOrder::Min => b.cmp(a),  // Smaller is "greater" for min-heap
        }
    }
}

/// Heap errors
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HeapError {
    Empty,
}

impl std::fmt::Display for HeapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HeapError::Empty => write!(f, "Heap is empty"),
        }
    }
}

impl std::error::Error for HeapError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_heap_push_pop() {
        let mut heap: BinaryHeap<i32> = BinaryHeap::new();
        
        heap.push(3);
        heap.push(1);
        heap.push(4);
        heap.push(1);
        heap.push(5);
        
        assert_eq!(heap.len(), 5);
        assert_eq!(heap.pop(), Some(5)); // Max heap returns largest
    }

    #[test]
    fn test_binary_heap_from_vec() {
        let heap = BinaryHeap::from_vec(vec![3, 1, 4, 1, 5, 9, 2, 6]);
        
        assert_eq!(heap.len(), 8);
    }

    #[test]
    fn test_binary_heap_peek() {
        let mut heap: BinaryHeap<i32> = BinaryHeap::new();
        
        heap.push(3);
        heap.push(1);
        heap.push(4);
        
        assert_eq!(heap.peek(), Some(&4));
    }

    #[test]
    fn test_binary_heap_pop_all() {
        let mut heap = BinaryHeap::from_vec(vec![3, 1, 4, 1, 5]);
        
        let mut results = Vec::new();
        while let Some(item) = heap.pop() {
            results.push(item);
        }
        
        // Should be in descending order (max heap)
        assert_eq!(results, vec![5, 4, 3, 1, 1]);
    }

    #[test]
    fn test_binary_heap_into_sorted() {
        let heap = BinaryHeap::from_vec(vec![3, 1, 4, 1, 5]);
        
        let sorted = heap.into_sorted_vec();
        
        // Sorted in descending order (max heap)
        assert_eq!(sorted, vec![5, 4, 3, 1, 1]);
    }

    #[test]
    fn test_binary_heap_empty() {
        let mut heap: BinaryHeap<i32> = BinaryHeap::new();
        
        assert!(heap.is_empty());
        assert_eq!(heap.pop(), None);
        assert_eq!(heap.peek(), None);
    }

    #[test]
    fn test_binary_heap_with_order_min() {
        let mut heap = BinaryHeapWithOrder::new(HeapOrder::Min);
        
        heap.push(3);
        heap.push(1);
        heap.push(4);
        
        assert_eq!(heap.pop(), Some(1)); // Min heap returns smallest first
    }

    #[test]
    fn test_binary_heap_with_order_max() {
        let mut heap = BinaryHeapWithOrder::new(HeapOrder::Max);
        
        heap.push(3);
        heap.push(1);
        heap.push(4);
        
        assert_eq!(heap.pop(), Some(4)); // Max heap returns largest first
    }

    #[test]
    fn test_binary_heap_with_order_empty() {
        let mut heap: BinaryHeapWithOrder<i32> = BinaryHeapWithOrder::new(HeapOrder::Min);
        
        assert!(heap.is_empty());
        assert_eq!(heap.pop(), None);
    }

    #[test]
    fn test_binary_heap_iter() {
        let mut heap: BinaryHeap<i32> = BinaryHeap::new();
        
        heap.push(3);
        heap.push(1);
        heap.push(4);
        
        let sum: i32 = heap.iter().sum();
        assert_eq!(sum, 8);
    }

    #[test]
    fn test_heap_error_display() {
        assert_eq!(HeapError::Empty.to_string(), "Heap is empty");
    }

    #[test]
    fn test_binary_heap_with_order_multiple_pops() {
        let mut heap = BinaryHeapWithOrder::new(HeapOrder::Min);
        
        heap.push(5);
        heap.push(3);
        heap.push(7);
        heap.push(1);
        
        assert_eq!(heap.pop(), Some(1));
        assert_eq!(heap.pop(), Some(3));
        assert_eq!(heap.pop(), Some(5));
        assert_eq!(heap.pop(), Some(7));
        assert_eq!(heap.pop(), None);
    }
}
