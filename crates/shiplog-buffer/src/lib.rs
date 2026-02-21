//! Buffer utilities for shiplog.
//!
//! This crate provides buffer utilities for managing collections of items.

use std::collections::VecDeque;

/// Configuration for buffer behavior
#[derive(Debug, Clone)]
pub struct BufferConfig {
    pub capacity: usize,
    pub strategy: BufferStrategy,
    pub name: String,
}

/// Buffer overflow strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferStrategy {
    DropOldest,
    DropNewest,
    Block,
}

impl Default for BufferConfig {
    fn default() -> Self {
        Self {
            capacity: 100,
            strategy: BufferStrategy::DropOldest,
            name: "buffer".to_string(),
        }
    }
}

/// Builder for creating buffer configurations
#[derive(Debug)]
pub struct BufferBuilder {
    config: BufferConfig,
}

impl BufferBuilder {
    pub fn new() -> Self {
        Self {
            config: BufferConfig::default(),
        }
    }

    pub fn capacity(mut self, capacity: usize) -> Self {
        self.config.capacity = capacity;
        self
    }

    pub fn strategy(mut self, strategy: BufferStrategy) -> Self {
        self.config.strategy = strategy;
        self
    }

    pub fn name(mut self, name: &str) -> Self {
        self.config.name = name.to_string();
        self
    }

    pub fn build(self) -> BufferConfig {
        self.config
    }
}

impl Default for BufferBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// A fixed-capacity buffer with different overflow strategies
pub struct Buffer<T> {
    data: VecDeque<T>,
    capacity: usize,
    strategy: BufferStrategy,
    name: String,
}

impl<T> Buffer<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(capacity),
            capacity,
            strategy: BufferStrategy::DropOldest,
            name: "buffer".to_string(),
        }
    }

    pub fn with_config(config: &BufferConfig) -> Self {
        Self {
            data: VecDeque::with_capacity(config.capacity),
            capacity: config.capacity,
            strategy: config.strategy,
            name: config.name.clone(),
        }
    }

    pub fn push(&mut self, item: T) -> Option<T> {
        if self.data.len() >= self.capacity {
            match self.strategy {
                BufferStrategy::DropOldest => {
                    self.data.pop_front();
                }
                BufferStrategy::DropNewest => {
                    // Don't add the new item
                    return Some(item);
                }
                BufferStrategy::Block => {
                    // In a real implementation, this would block
                    // For now, drop oldest
                    self.data.pop_front();
                }
            }
        }
        self.data.push_back(item);
        None
    }

    pub fn pop(&mut self) -> Option<T> {
        self.data.pop_front()
    }

    pub fn front(&self) -> Option<&T> {
        self.data.front()
    }

    pub fn back(&self) -> Option<&T> {
        self.data.back()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn is_full(&self) -> bool {
        self.data.len() >= self.capacity
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Circular buffer for efficient FIFO operations
pub struct CircularBuffer<T> {
    buffer: Vec<Option<T>>,
    head: usize,
    tail: usize,
    count: usize,
    capacity: usize,
}

impl<T: Clone> CircularBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![None; capacity],
            head: 0,
            tail: 0,
            count: 0,
            capacity,
        }
    }

    pub fn push(&mut self, item: T) -> Option<T> {
        if self.count == self.capacity {
            // Buffer is full, return old item
            let old = self.buffer[self.tail].take();
            self.tail = (self.tail + 1) % self.capacity;
            self.count -= 1;

            self.buffer[self.head] = Some(item);
            self.head = (self.head + 1) % self.capacity;
            self.count += 1;

            old
        } else {
            self.buffer[self.head] = Some(item);
            self.head = (self.head + 1) % self.capacity;
            self.count += 1;
            None
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.count == 0 {
            None
        } else {
            let item = self.buffer[self.tail].take();
            self.tail = (self.tail + 1) % self.capacity;
            self.count -= 1;
            item
        }
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn is_full(&self) -> bool {
        self.count == self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_config_default() {
        let config = BufferConfig::default();
        assert_eq!(config.capacity, 100);
        assert_eq!(config.strategy, BufferStrategy::DropOldest);
        assert_eq!(config.name, "buffer");
    }

    #[test]
    fn test_buffer_builder() {
        let config = BufferBuilder::new()
            .capacity(200)
            .strategy(BufferStrategy::DropNewest)
            .name("test-buffer")
            .build();

        assert_eq!(config.capacity, 200);
        assert_eq!(config.strategy, BufferStrategy::DropNewest);
        assert_eq!(config.name, "test-buffer");
    }

    #[test]
    fn test_buffer_push_pop() {
        let mut buffer: Buffer<i32> = Buffer::new(3);

        assert!(buffer.push(1).is_none());
        assert!(buffer.push(2).is_none());
        assert!(buffer.push(3).is_none());

        assert_eq!(buffer.len(), 3);
        assert!(buffer.is_full());

        assert_eq!(buffer.pop(), Some(1));
        assert_eq!(buffer.pop(), Some(2));
        assert_eq!(buffer.pop(), Some(3));
        assert_eq!(buffer.pop(), None);
    }

    #[test]
    fn test_buffer_drop_oldest() {
        let mut buffer: Buffer<i32> = Buffer::new(3);

        buffer.push(1);
        buffer.push(2);
        buffer.push(3);
        buffer.push(4); // Should drop 1

        assert_eq!(buffer.pop(), Some(2));
        assert_eq!(buffer.pop(), Some(3));
        assert_eq!(buffer.pop(), Some(4));
    }

    #[test]
    fn test_buffer_drop_newest() {
        let buffer = BufferBuilder::new()
            .capacity(3)
            .strategy(BufferStrategy::DropNewest)
            .build();

        let mut buf: Buffer<i32> = Buffer::with_config(&buffer);

        buf.push(1);
        buf.push(2);
        buf.push(3);
        buf.push(4); // Should drop 4

        assert_eq!(buf.pop(), Some(1));
        assert_eq!(buf.pop(), Some(2));
        assert_eq!(buf.pop(), Some(3));
    }

    #[test]
    fn test_circular_buffer() {
        let mut buffer: CircularBuffer<i32> = CircularBuffer::new(3);

        assert!(buffer.push(1).is_none());
        assert!(buffer.push(2).is_none());
        assert!(buffer.push(3).is_none());
        assert!(buffer.is_full());

        // This should return 1 (the oldest)
        assert_eq!(buffer.push(4), Some(1));

        assert_eq!(buffer.pop(), Some(2));
        assert_eq!(buffer.pop(), Some(3));
        assert_eq!(buffer.pop(), Some(4));
        assert!(buffer.is_empty());
    }
}
