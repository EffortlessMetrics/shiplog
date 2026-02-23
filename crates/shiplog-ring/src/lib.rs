//! Ring buffer implementation for shiplog.
//!
//! This crate provides a Ring Buffer (Circular Buffer) implementation
//! for efficient FIFO operations with fixed capacity.

/// A Ring Buffer (Circular Buffer) implementation
#[derive(Debug)]
pub struct RingBuffer<T> {
    buffer: Vec<Option<T>>,
    head: usize,
    tail: usize,
    count: usize,
    capacity: usize,
}

impl<T: Clone> RingBuffer<T> {
    /// Creates a new Ring Buffer with the specified capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![None; capacity],
            head: 0,
            tail: 0,
            count: 0,
            capacity,
        }
    }

    /// Creates a new Ring Buffer with the specified capacity and fills it with initial values
    pub fn from_vec(capacity: usize, data: Vec<T>) -> Self {
        let mut buffer = Self::new(capacity);

        for item in data {
            let _ = buffer.push(item);
        }

        buffer
    }

    /// Pushes an item into the buffer
    /// Returns the overwritten item if the buffer was full
    pub fn push(&mut self, item: T) -> Option<T> {
        let overwritten = if self.count == self.capacity {
            // Buffer is full, overwrite the tail (oldest)
            let old = self.buffer[self.tail].take();
            self.tail = (self.tail + 1) % self.capacity;
            old
        } else {
            self.count += 1;
            None
        };

        self.buffer[self.head] = Some(item);
        self.head = (self.head + 1) % self.capacity;

        overwritten
    }

    /// Pops an item from the buffer
    pub fn pop(&mut self) -> Option<T> {
        if self.count == 0 {
            return None;
        }

        self.count -= 1;
        let item = self.buffer[self.tail].take();
        self.tail = (self.tail + 1) % self.capacity;

        item
    }

    /// Peeks at the front item without removing it
    pub fn front(&self) -> Option<&T> {
        self.buffer[self.tail].as_ref()
    }

    /// Peeks at the back item without removing it
    pub fn back(&self) -> Option<&T> {
        let back_idx = if self.head == 0 {
            self.capacity - 1
        } else {
            self.head - 1
        };

        self.buffer[back_idx].as_ref()
    }

    /// Returns the number of items in the buffer
    pub fn len(&self) -> usize {
        self.count
    }

    /// Returns true if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Returns true if the buffer is full
    pub fn is_full(&self) -> bool {
        self.count == self.capacity
    }

    /// Returns the capacity of the buffer
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the remaining free space in the buffer
    pub fn available_space(&self) -> usize {
        self.capacity - self.count
    }

    /// Clears all items from the buffer
    pub fn clear(&mut self) {
        for item in &mut self.buffer {
            *item = None;
        }
        self.head = 0;
        self.tail = 0;
        self.count = 0;
    }

    /// Returns all items as a vector (in order from oldest to newest)
    pub fn to_vec(&self) -> Vec<T> {
        let mut result = Vec::with_capacity(self.count);

        if self.count == 0 {
            return result;
        }

        let mut idx = self.tail;
        for _ in 0..self.count {
            if let Some(ref item) = self.buffer[idx] {
                result.push(item.clone());
            }
            idx = (idx + 1) % self.capacity;
        }

        result
    }
}

impl<T: Clone> Default for RingBuffer<T> {
    fn default() -> Self {
        Self::new(64)
    }
}

/// Configuration for ring buffer behavior
#[derive(Debug, Clone)]
pub struct RingConfig {
    pub capacity: usize,
    pub overwrite: bool,
}

impl Default for RingConfig {
    fn default() -> Self {
        Self {
            capacity: 64,
            overwrite: true,
        }
    }
}

/// Builder for ring buffer configurations
#[derive(Debug)]
pub struct RingBuilder {
    config: RingConfig,
}

impl RingBuilder {
    pub fn new() -> Self {
        Self {
            config: RingConfig::default(),
        }
    }

    pub fn capacity(mut self, capacity: usize) -> Self {
        self.config.capacity = capacity;
        self
    }

    pub fn overwrite(mut self, overwrite: bool) -> Self {
        self.config.overwrite = overwrite;
        self
    }

    pub fn build_config(self) -> RingConfig {
        self.config
    }

    pub fn build<T: Clone>(self) -> RingBuffer<T> {
        RingBuffer::new(self.config.capacity)
    }
}

impl Default for RingBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer_new() {
        let buffer: RingBuffer<i32> = RingBuffer::new(5);
        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);
        assert_eq!(buffer.capacity(), 5);
    }

    #[test]
    fn test_ring_buffer_push_pop() {
        let mut buffer: RingBuffer<i32> = RingBuffer::new(3);

        assert!(buffer.push(1).is_none());
        assert!(buffer.push(2).is_none());
        assert!(buffer.push(3).is_none());

        assert!(buffer.is_full());

        assert_eq!(buffer.pop(), Some(1));
        assert_eq!(buffer.pop(), Some(2));
        assert_eq!(buffer.pop(), Some(3));

        assert!(buffer.is_empty());
    }

    #[test]
    fn test_ring_buffer_overflow() {
        let mut buffer: RingBuffer<i32> = RingBuffer::new(3);

        buffer.push(1);
        buffer.push(2);
        buffer.push(3);

        // This should overwrite the oldest (1)
        let overwritten = buffer.push(4);
        assert_eq!(overwritten, Some(1));

        // Order should be 2, 3, 4
        assert_eq!(buffer.pop(), Some(2));
        assert_eq!(buffer.pop(), Some(3));
        assert_eq!(buffer.pop(), Some(4));
    }

    #[test]
    fn test_ring_buffer_front_back() {
        let mut buffer: RingBuffer<i32> = RingBuffer::new(3);

        buffer.push(1);
        buffer.push(2);

        assert_eq!(buffer.front(), Some(&1));
        assert_eq!(buffer.back(), Some(&2));
    }

    #[test]
    fn test_ring_buffer_clear() {
        let mut buffer: RingBuffer<i32> = RingBuffer::new(3);

        buffer.push(1);
        buffer.push(2);
        buffer.clear();

        assert!(buffer.is_empty());
        assert_eq!(buffer.front(), None);
    }

    #[test]
    fn test_ring_buffer_to_vec() {
        let mut buffer: RingBuffer<i32> = RingBuffer::new(5);

        buffer.push(1);
        buffer.push(2);
        buffer.push(3);

        assert_eq!(buffer.to_vec(), vec![1, 2, 3]);
    }

    #[test]
    fn test_ring_buffer_from_vec() {
        let buffer = RingBuffer::from_vec(5, vec![1, 2, 3]);

        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer.to_vec(), vec![1, 2, 3]);
    }

    #[test]
    fn test_ring_buffer_wrap_around() {
        let mut buffer: RingBuffer<i32> = RingBuffer::new(3);

        // Fill the buffer
        buffer.push(1);
        buffer.push(2);
        buffer.push(3);

        // Pop one (removes 1)
        buffer.pop();

        // Add two more - should wrap around
        buffer.push(4);
        buffer.push(5);

        // Now we should have: 3, 4, 5
        assert_eq!(buffer.to_vec(), vec![3, 4, 5]);
    }

    #[test]
    fn test_ring_buffer_available_space() {
        let mut buffer: RingBuffer<i32> = RingBuffer::new(3);

        assert_eq!(buffer.available_space(), 3);

        buffer.push(1);
        assert_eq!(buffer.available_space(), 2);

        buffer.push(2);
        buffer.push(3);
        assert_eq!(buffer.available_space(), 0);

        buffer.pop();
        assert_eq!(buffer.available_space(), 1);
    }

    #[test]
    fn test_ring_builder() {
        let buffer: RingBuffer<i32> = RingBuilder::new().capacity(10).overwrite(true).build();

        assert_eq!(buffer.capacity(), 10);
    }

    #[test]
    fn test_ring_config_default() {
        let config = RingConfig::default();

        assert_eq!(config.capacity, 64);
        assert!(config.overwrite);
    }
}
