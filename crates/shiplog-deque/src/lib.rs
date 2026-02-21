//! Double-ended queue implementation for shiplog.
//!
//! This crate provides a double-ended queue (deque) implementation that allows
//! efficient insertion and removal from both ends.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// A double-ended queue that supports efficient operations at both ends.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deque<T> {
    inner: VecDeque<T>,
}

impl<T> Default for Deque<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Deque<T> {
    /// Create a new empty deque.
    pub fn new() -> Self {
        Self {
            inner: VecDeque::new(),
        }
    }

    /// Create a deque with a specific capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: VecDeque::with_capacity(capacity),
        }
    }

    /// Push an element to the front of the deque.
    pub fn push_front(&mut self, item: T) {
        self.inner.push_front(item);
    }

    /// Push an element to the back of the deque.
    pub fn push_back(&mut self, item: T) {
        self.inner.push_back(item);
    }

    /// Pop an element from the front of the deque.
    pub fn pop_front(&mut self) -> Option<T> {
        self.inner.pop_front()
    }

    /// Pop an element from the back of the deque.
    pub fn pop_back(&mut self) -> Option<T> {
        self.inner.pop_back()
    }

    /// Peek at the front element without removing it.
    pub fn front(&self) -> Option<&T> {
        self.inner.front()
    }

    /// Peek at the back element without removing it.
    pub fn back(&self) -> Option<&T> {
        self.inner.back()
    }

    /// Get the number of elements in the deque.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Check if the deque is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Clear all elements from the deque.
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Get an iterator over the deque.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.inner.iter()
    }

    /// Get a mutable iterator over the deque.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.inner.iter_mut()
    }

    /// Rotate the deque by n positions to the left.
    pub fn rotate_left(&mut self, n: usize) {
        self.inner.rotate_left(n);
    }

    /// Rotate the deque by n positions to the right.
    pub fn rotate_right(&mut self, n: usize) {
        self.inner.rotate_right(n);
    }

    /// Split the deque into two at the given index.
    pub fn split_at(&mut self, mid: usize) -> (Deque<T>, Deque<T>) {
        let left: VecDeque<T> = self.inner.drain(..mid).collect();
        let right = std::mem::take(&mut self.inner);
        (Deque { inner: left }, Deque { inner: right })
    }
}

/// Deque errors
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DequeError {
    Empty,
    IndexOutOfBounds,
}

impl std::fmt::Display for DequeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DequeError::Empty => write!(f, "Deque is empty"),
            DequeError::IndexOutOfBounds => write!(f, "Index out of bounds"),
        }
    }
}

impl std::error::Error for DequeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deque_push_pop_front() {
        let mut deque: Deque<i32> = Deque::new();

        deque.push_front(1);
        deque.push_front(2);
        deque.push_front(3);

        assert_eq!(deque.pop_front(), Some(3));
        assert_eq!(deque.pop_front(), Some(2));
        assert_eq!(deque.pop_front(), Some(1));
        assert_eq!(deque.pop_front(), None);
    }

    #[test]
    fn test_deque_push_pop_back() {
        let mut deque: Deque<i32> = Deque::new();

        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);

        assert_eq!(deque.pop_back(), Some(3));
        assert_eq!(deque.pop_back(), Some(2));
        assert_eq!(deque.pop_back(), Some(1));
        assert_eq!(deque.pop_back(), None);
    }

    #[test]
    fn test_deque_mixed_operations() {
        let mut deque: Deque<i32> = Deque::new();

        deque.push_back(1);
        deque.push_front(2);
        deque.push_back(3);
        deque.push_front(4);

        assert_eq!(deque.pop_front(), Some(4));
        assert_eq!(deque.pop_back(), Some(3));
        assert_eq!(deque.pop_front(), Some(2));
        assert_eq!(deque.pop_back(), Some(1));
    }

    #[test]
    fn test_deque_peek() {
        let mut deque: Deque<i32> = Deque::new();

        deque.push_back(1);
        deque.push_back(2);

        assert_eq!(deque.front(), Some(&1));
        assert_eq!(deque.back(), Some(&2));

        assert_eq!(deque.len(), 2);
    }

    #[test]
    fn test_deque_clear() {
        let mut deque: Deque<i32> = Deque::new();

        deque.push_back(1);
        deque.push_back(2);
        assert!(!deque.is_empty());

        deque.clear();
        assert!(deque.is_empty());
    }

    #[test]
    fn test_deque_rotate() {
        let mut deque: Deque<i32> = Deque::new();

        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        deque.push_back(4);

        deque.rotate_left(2);

        assert_eq!(deque.pop_front(), Some(3));
        assert_eq!(deque.pop_front(), Some(4));
        assert_eq!(deque.pop_front(), Some(1));
        assert_eq!(deque.pop_front(), Some(2));
    }

    #[test]
    fn test_deque_split_at() {
        let mut deque: Deque<i32> = Deque::new();

        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);
        deque.push_back(4);

        let (left, right) = deque.split_at(2);

        assert_eq!(left.len(), 2);
        assert_eq!(right.len(), 2);
    }

    #[test]
    fn test_deque_with_capacity() {
        let deque: Deque<i32> = Deque::with_capacity(100);
        assert!(deque.is_empty());

        let mut deque = deque;
        deque.push_back(42);
        assert_eq!(deque.len(), 1);
    }

    #[test]
    fn test_deque_iter() {
        let mut deque: Deque<i32> = Deque::new();

        deque.push_back(1);
        deque.push_back(2);
        deque.push_back(3);

        let sum: i32 = deque.iter().sum();
        assert_eq!(sum, 6);
    }

    #[test]
    fn test_deque_error_display() {
        assert_eq!(DequeError::Empty.to_string(), "Deque is empty");
        assert_eq!(
            DequeError::IndexOutOfBounds.to_string(),
            "Index out of bounds"
        );
    }
}
