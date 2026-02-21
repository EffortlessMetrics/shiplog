//! Sink utilities for streaming data in shiplog.
//!
//! This crate provides sink implementations for consuming and processing streaming data.

use std::collections::VecDeque;

/// A sink that collects items into a buffer
pub struct BufferSink<T> {
    buffer: VecDeque<T>,
    capacity: usize,
}

impl<T> BufferSink<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, item: T) -> Option<T> {
        if self.buffer.len() >= self.capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back(item);
        None
    }

    pub fn drain(&mut self) -> Vec<T> {
        self.buffer.drain(..).collect()
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

impl<T> Default for BufferSink<T> {
    fn default() -> Self {
        Self::new(100)
    }
}

/// A sink that transforms items as they pass through
pub struct TransformSink<T, F>
where
    F: Fn(T) -> T,
{
    transform: F,
    output: Vec<T>,
}

impl<T, F> TransformSink<T, F>
where
    F: Fn(T) -> T,
{
    pub fn new(transform: F) -> Self {
        Self {
            transform,
            output: Vec::new(),
        }
    }

    pub fn push(&mut self, item: T) {
        let transformed = (self.transform)(item);
        self.output.push(transformed);
    }

    pub fn drain(&mut self) -> Vec<T> {
        std::mem::take(&mut self.output)
    }

    pub fn is_empty(&self) -> bool {
        self.output.is_empty()
    }
}

/// A sink that filters items based on a predicate
pub struct FilterSink<T, F>
where
    F: Fn(&T) -> bool,
{
    filter: F,
    output: Vec<T>,
}

impl<T, F> FilterSink<T, F>
where
    F: Fn(&T) -> bool,
{
    pub fn new(filter: F) -> Self {
        Self {
            filter,
            output: Vec::new(),
        }
    }

    pub fn push(&mut self, item: T) {
        if (self.filter)(&item) {
            self.output.push(item);
        }
    }

    pub fn drain(&mut self) -> Vec<T> {
        std::mem::take(&mut self.output)
    }

    pub fn is_empty(&self) -> bool {
        self.output.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_sink_basic() {
        let mut sink: BufferSink<i32> = BufferSink::new(3);

        sink.push(1);
        sink.push(2);
        sink.push(3);

        assert_eq!(sink.len(), 3);

        sink.push(4); // Should evict 1
        assert_eq!(sink.len(), 3);

        let drained = sink.drain();
        assert_eq!(drained, vec![2, 3, 4]);
    }

    #[test]
    fn test_buffer_sink_empty() {
        let sink: BufferSink<i32> = BufferSink::new(3);
        assert!(sink.is_empty());
    }

    #[test]
    fn test_buffer_sink_clear() {
        let mut sink: BufferSink<i32> = BufferSink::new(3);
        sink.push(1);
        sink.push(2);

        sink.clear();

        assert!(sink.is_empty());
        assert_eq!(sink.len(), 0);
    }

    #[test]
    fn test_transform_sink() {
        let mut sink = TransformSink::new(|x: i32| x * 2);

        sink.push(1);
        sink.push(2);
        sink.push(3);

        let drained = sink.drain();
        assert_eq!(drained, vec![2, 4, 6]);
    }

    #[test]
    fn test_transform_sink_multiple_drains() {
        let mut sink = TransformSink::new(|x: i32| x + 1);

        sink.push(1);
        assert_eq!(sink.drain(), vec![2]);

        sink.push(2);
        assert_eq!(sink.drain(), vec![3]);
    }

    #[test]
    fn test_filter_sink() {
        let mut sink = FilterSink::new(|x: &i32| *x % 2 == 0);

        sink.push(1);
        sink.push(2);
        sink.push(3);
        sink.push(4);

        let drained = sink.drain();
        assert_eq!(drained, vec![2, 4]);
    }

    #[test]
    fn test_filter_sink_none_match() {
        let mut sink = FilterSink::new(|x: &i32| *x > 10);

        sink.push(1);
        sink.push(2);
        sink.push(3);

        assert!(sink.is_empty());
    }

    #[test]
    fn test_filter_sink_all_match() {
        let mut sink = FilterSink::new(|_: &i32| true);

        sink.push(1);
        sink.push(2);
        sink.push(3);

        let drained = sink.drain();
        assert_eq!(drained, vec![1, 2, 3]);
    }
}
