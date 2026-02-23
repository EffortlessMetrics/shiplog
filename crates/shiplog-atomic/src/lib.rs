//! Atomic counter utilities for shiplog.
//!
//! This crate provides atomic counter primitives for lock-free synchronization.

use std::sync::atomic::{
    AtomicBool, AtomicI64 as StdAtomicI64, AtomicU64 as StdAtomicU64, AtomicUsize, Ordering,
};

/// A simple atomic counter using `AtomicUsize`.
#[derive(Debug)]
pub struct Counter {
    inner: AtomicUsize,
}

impl Counter {
    /// Create a new counter with the given initial value.
    pub fn new(initial: usize) -> Self {
        Self {
            inner: AtomicUsize::new(initial),
        }
    }

    /// Get the current value.
    pub fn get(&self) -> usize {
        self.inner.load(Ordering::SeqCst)
    }

    /// Increment the counter by one.
    ///
    /// Returns the new value after incrementing.
    pub fn increment(&self) -> usize {
        self.inner.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Decrement the counter by one.
    ///
    /// Returns the new value after decrementing.
    pub fn decrement(&self) -> usize {
        self.inner.fetch_sub(1, Ordering::SeqCst) - 1
    }

    /// Add a value to the counter.
    ///
    /// Returns the new value after adding.
    pub fn add(&self, value: usize) -> usize {
        self.inner.fetch_add(value, Ordering::SeqCst) + value
    }

    /// Subtract a value from the counter.
    ///
    /// Returns the new value after subtracting.
    pub fn sub(&self, value: usize) -> usize {
        self.inner.fetch_sub(value, Ordering::SeqCst) - value
    }

    /// Swap the value with a new one.
    ///
    /// Returns the old value.
    pub fn swap(&self, value: usize) -> usize {
        self.inner.swap(value, Ordering::SeqCst)
    }

    /// Compare and swap the value.
    ///
    /// If the current value equals `current`, it is replaced with `new`.
    /// Returns the old value.
    pub fn compare_and_swap(&self, current: usize, new: usize) -> usize {
        self.inner
            .compare_exchange(current, new, Ordering::SeqCst, Ordering::SeqCst)
            .unwrap_or_else(|e| e)
    }

    /// Reset the counter to zero.
    pub fn reset(&self) {
        self.inner.store(0, Ordering::SeqCst);
    }
}

impl Default for Counter {
    fn default() -> Self {
        Self::new(0)
    }
}

/// An atomic 64-bit unsigned integer wrapper.
#[derive(Debug)]
pub struct AtomicU64 {
    inner: StdAtomicU64,
}

impl AtomicU64 {
    /// Create a new atomic unsigned 64-bit integer.
    pub fn new(value: u64) -> Self {
        Self {
            inner: StdAtomicU64::new(value),
        }
    }

    /// Get the current value.
    pub fn get(&self) -> u64 {
        self.inner.load(Ordering::SeqCst)
    }

    /// Set a new value.
    pub fn set(&self, value: u64) {
        self.inner.store(value, Ordering::SeqCst);
    }

    /// Increment and return the new value.
    pub fn increment(&self) -> u64 {
        self.inner.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Decrement and return the new value.
    pub fn decrement(&self) -> u64 {
        self.inner.fetch_sub(1, Ordering::SeqCst) - 1
    }

    /// Add a value and return the new value.
    pub fn add(&self, value: u64) -> u64 {
        self.inner.fetch_add(value, Ordering::SeqCst) + value
    }
}

impl Default for AtomicU64 {
    fn default() -> Self {
        Self::new(0)
    }
}

/// An atomic 64-bit signed integer wrapper.
#[derive(Debug)]
pub struct AtomicI64 {
    inner: StdAtomicI64,
}

impl AtomicI64 {
    /// Create a new atomic signed 64-bit integer.
    pub fn new(value: i64) -> Self {
        Self {
            inner: StdAtomicI64::new(value),
        }
    }

    /// Get the current value.
    pub fn get(&self) -> i64 {
        self.inner.load(Ordering::SeqCst)
    }

    /// Set a new value.
    pub fn set(&self, value: i64) {
        self.inner.store(value, Ordering::SeqCst);
    }

    /// Increment and return the new value.
    pub fn increment(&self) -> i64 {
        self.inner.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Decrement and return the new value.
    pub fn decrement(&self) -> i64 {
        self.inner.fetch_sub(1, Ordering::SeqCst) - 1
    }

    /// Add a value and return the new value.
    pub fn add(&self, value: i64) -> i64 {
        self.inner.fetch_add(value, Ordering::SeqCst) + value
    }
}

impl Default for AtomicI64 {
    fn default() -> Self {
        Self::new(0)
    }
}

/// An atomic boolean flag.
#[derive(Debug)]
pub struct AtomicFlag {
    inner: AtomicBool,
}

impl AtomicFlag {
    /// Create a new atomic flag with the given initial value.
    pub fn new(value: bool) -> Self {
        Self {
            inner: AtomicBool::new(value),
        }
    }

    /// Get the current value.
    pub fn get(&self) -> bool {
        self.inner.load(Ordering::SeqCst)
    }

    /// Set a new value.
    pub fn set(&self, value: bool) {
        self.inner.store(value, Ordering::SeqCst);
    }

    /// Set the flag to true.
    ///
    /// Returns the old value.
    pub fn set_true(&self) -> bool {
        self.inner.swap(true, Ordering::SeqCst)
    }

    /// Set the flag to false.
    ///
    /// Returns the old value.
    pub fn set_false(&self) -> bool {
        self.inner.swap(false, Ordering::SeqCst)
    }

    /// Compare and set the flag.
    ///
    /// If the current value equals `current`, it is replaced with `new`.
    /// Returns true if successful.
    pub fn compare_and_set(&self, current: bool, new: bool) -> bool {
        self.inner
            .compare_exchange(current, new, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }
}

impl Default for AtomicFlag {
    fn default() -> Self {
        Self::new(false)
    }
}

/// A sequence generator using atomic counter.
#[derive(Debug)]
pub struct Sequence {
    counter: Counter,
}

impl Sequence {
    /// Create a new sequence with the given starting value.
    pub fn new(start: usize) -> Self {
        Self {
            counter: Counter::new(start),
        }
    }

    /// Get the next value in the sequence.
    pub fn next(&self) -> usize {
        self.counter.increment()
    }

    /// Get the current value without incrementing.
    pub fn current(&self) -> usize {
        self.counter.get()
    }

    /// Reset the sequence to the initial value.
    pub fn reset(&self, start: usize) {
        self.counter.swap(start);
    }
}

impl Default for Sequence {
    fn default() -> Self {
        Self::new(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_counter_new() {
        let counter = Counter::new(5);
        assert_eq!(counter.get(), 5);
    }

    #[test]
    fn test_counter_increment() {
        let counter = Counter::new(0);
        assert_eq!(counter.increment(), 1);
        assert_eq!(counter.increment(), 2);
        assert_eq!(counter.get(), 2);
    }

    #[test]
    fn test_counter_decrement() {
        let counter = Counter::new(5);
        assert_eq!(counter.decrement(), 4);
        assert_eq!(counter.get(), 4);
    }

    #[test]
    fn test_counter_add() {
        let counter = Counter::new(10);
        assert_eq!(counter.add(5), 15);
        assert_eq!(counter.get(), 15);
    }

    #[test]
    fn test_counter_sub() {
        let counter = Counter::new(10);
        assert_eq!(counter.sub(3), 7);
        assert_eq!(counter.get(), 7);
    }

    #[test]
    fn test_counter_swap() {
        let counter = Counter::new(5);
        let old = counter.swap(10);
        assert_eq!(old, 5);
        assert_eq!(counter.get(), 10);
    }

    #[test]
    fn test_counter_compare_and_swap() {
        let counter = Counter::new(5);

        // Successful CAS - returns old value
        let old = counter.compare_and_swap(5, 10);
        assert_eq!(old, 5);
        assert_eq!(counter.get(), 10);

        // Failed CAS - returns current value
        let old = counter.compare_and_swap(5, 20);
        assert_eq!(old, 10); // Still 10, not changed
    }

    #[test]
    fn test_counter_reset() {
        let counter = Counter::new(100);
        counter.reset();
        assert_eq!(counter.get(), 0);
    }

    #[test]
    fn test_counter_concurrent() {
        let counter = Arc::new(Counter::new(0));
        let mut handles = Vec::new();

        for _ in 0..10 {
            let counter = Arc::clone(&counter);
            let handle = thread::spawn(move || {
                for _ in 0..1000 {
                    counter.increment();
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(counter.get(), 10000);
    }

    #[test]
    fn test_atomic_u64() {
        let atomic = AtomicU64::new(42);
        assert_eq!(atomic.get(), 42);

        atomic.set(100);
        assert_eq!(atomic.get(), 100);

        assert_eq!(atomic.increment(), 101);
        assert_eq!(atomic.decrement(), 100);
    }

    #[test]
    fn test_atomic_i64() {
        let atomic = AtomicI64::new(-10);
        assert_eq!(atomic.get(), -10);

        atomic.set(50);
        assert_eq!(atomic.get(), 50);

        assert_eq!(atomic.increment(), 51);
        assert_eq!(atomic.decrement(), 50);

        assert_eq!(atomic.add(-5), 45);
    }

    #[test]
    fn test_atomic_flag() {
        let flag = AtomicFlag::new(false);
        assert!(!flag.get());

        flag.set(true);
        assert!(flag.get());

        let old = flag.set_true();
        assert!(old);
        assert!(flag.get());

        let old = flag.set_false();
        assert!(old);
        assert!(!flag.get());
    }

    #[test]
    fn test_atomic_flag_compare_and_set() {
        let flag = AtomicFlag::new(false);

        assert!(flag.compare_and_set(false, true));
        assert!(!flag.compare_and_set(false, true)); // Already true

        assert!(flag.compare_and_set(true, false));
    }

    #[test]
    fn test_sequence() {
        let seq = Sequence::new(1);

        // next() returns the value after incrementing
        assert_eq!(seq.next(), 2); // First call: starts at 1, returns 2
        assert_eq!(seq.current(), 2);
        assert_eq!(seq.next(), 3); // Second call: now at 2, returns 3

        seq.reset(100);
        assert_eq!(seq.current(), 100);
    }
}
