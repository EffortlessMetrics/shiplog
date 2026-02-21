//! Leaky bucket implementation for shiplog.
//!
//! This crate provides a leaky bucket implementation for rate limiting.

use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;

/// Leaky bucket configuration.
#[derive(Debug, Clone)]
pub struct LeakyBucketConfig {
    /// Maximum capacity of the bucket.
    pub capacity: u64,
    /// Rate at which the bucket leaks (items per interval).
    pub leak_rate: u64,
    /// Interval for leaking.
    pub leak_interval: Duration,
}

impl LeakyBucketConfig {
    /// Create a new leaky bucket configuration.
    pub fn new(capacity: u64, leak_rate: u64, leak_interval: Duration) -> Self {
        Self {
            capacity,
            leak_rate,
            leak_interval,
        }
    }

    /// Create a strict configuration (10 capacity, leaks 1 per second).
    pub fn strict() -> Self {
        Self {
            capacity: 10,
            leak_rate: 1,
            leak_interval: Duration::seconds(1),
        }
    }

    /// Create a lenient configuration (100 capacity, leaks 10 per second).
    pub fn lenient() -> Self {
        Self {
            capacity: 100,
            leak_rate: 10,
            leak_interval: Duration::seconds(1),
        }
    }
}

impl Default for LeakyBucketConfig {
    fn default() -> Self {
        Self::lenient()
    }
}

/// Leaky bucket state.
#[derive(Debug, Clone)]
pub struct LeakyBucketState {
    pub current_level: u64,
    pub last_leak: DateTime<Utc>,
}

/// Leaky bucket for rate limiting.
#[derive(Debug)]
pub struct LeakyBucket {
    config: LeakyBucketConfig,
    buckets: HashMap<String, LeakyBucketState>,
}

impl LeakyBucket {
    /// Create a new leaky bucket with the given configuration.
    pub fn new(config: LeakyBucketConfig) -> Self {
        Self {
            config,
            buckets: HashMap::new(),
        }
    }

    /// Try to add an item to the bucket.
    pub fn try_add(&mut self, key: &str) -> bool {
        let now = Utc::now();
        
        let state = self.buckets.entry(key.to_string()).or_insert_with(|| {
            LeakyBucketState {
                current_level: 0,
                last_leak: now,
            }
        });

        // Leak based on time elapsed
        let elapsed = now - state.last_leak;
        if elapsed >= self.config.leak_interval {
            let elapsed_ms = elapsed.num_milliseconds();
            let interval_ms = self.config.leak_interval.num_milliseconds();
            let intervals = (elapsed_ms / interval_ms) as u64;
            state.current_level = state.current_level.saturating_sub(intervals * self.config.leak_rate);
            state.last_leak = now;
        }

        // Try to add item
        if state.current_level < self.config.capacity {
            state.current_level += 1;
            true
        } else {
            false
        }
    }

    /// Get the current level of the bucket.
    pub fn current_level(&self, key: &str) -> u64 {
        if let Some(state) = self.buckets.get(key) {
            let now = Utc::now();
            let elapsed = now - state.last_leak;
            if elapsed >= self.config.leak_interval {
                let elapsed_ms = elapsed.num_milliseconds();
                let interval_ms = self.config.leak_interval.num_milliseconds();
                let intervals = (elapsed_ms / interval_ms) as u64;
                return state.current_level.saturating_sub(intervals * self.config.leak_rate);
            }
            state.current_level
        } else {
            0
        }
    }

    /// Get the remaining capacity of the bucket.
    pub fn remaining_capacity(&self, key: &str) -> u64 {
        self.config.capacity.saturating_sub(self.current_level(key))
    }

    /// Reset the bucket for a key.
    pub fn reset(&mut self, key: &str) {
        self.buckets.remove(key);
    }

    /// Clear all buckets.
    pub fn clear(&mut self) {
        self.buckets.clear();
    }

    /// Get the number of tracked keys.
    pub fn len(&self) -> usize {
        self.buckets.len()
    }

    /// Check if there are no tracked keys.
    pub fn is_empty(&self) -> bool {
        self.buckets.is_empty()
    }
}

/// Result type for leaky bucket operations.
pub type LeakyBucketResult<T> = Result<T, LeakyBucketError>;

/// Error type for leaky bucket operations.
#[derive(Debug, Clone)]
pub struct LeakyBucketError {
    pub message: String,
    pub key: String,
}

impl LeakyBucketError {
    pub fn new(message: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            key: key.into(),
        }
    }
}

impl std::fmt::Display for LeakyBucketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LeakyBucket error for '{}': {}", self.key, self.message)
    }
}

impl std::error::Error for LeakyBucketError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_leaky_bucket_initial_state() {
        let config = LeakyBucketConfig::strict();
        let bucket = LeakyBucket::new(config);

        assert!(bucket.is_empty());
    }

    #[test]
    fn test_leaky_bucket_add() {
        let config = LeakyBucketConfig::new(5, 1, Duration::seconds(1));
        let mut bucket = LeakyBucket::new(config);

        // Should be able to add up to capacity
        assert!(bucket.try_add("user1"));
        assert!(bucket.try_add("user1"));
        assert!(bucket.try_add("user1"));
        assert!(bucket.try_add("user1"));
        assert!(bucket.try_add("user1"));
        
        // Should fail when bucket is full
        assert!(!bucket.try_add("user1"));
    }

    #[test]
    fn test_leaky_bucket_separate_keys() {
        let config = LeakyBucketConfig::new(2, 1, Duration::seconds(1));
        let mut bucket = LeakyBucket::new(config);

        // Each key should have its own bucket
        assert!(bucket.try_add("user1"));
        assert!(bucket.try_add("user1"));
        assert!(!bucket.try_add("user1"));

        assert!(bucket.try_add("user2"));
        assert!(bucket.try_add("user2"));
        assert!(!bucket.try_add("user2"));
    }

    #[test]
    fn test_leaky_bucket_current_level() {
        let config = LeakyBucketConfig::new(5, 1, Duration::seconds(1));
        let mut bucket = LeakyBucket::new(config);

        assert_eq!(bucket.current_level("user1"), 0);

        bucket.try_add("user1");
        bucket.try_add("user1");
        assert_eq!(bucket.current_level("user1"), 2);
    }

    #[test]
    fn test_leaky_bucket_remaining_capacity() {
        let config = LeakyBucketConfig::new(5, 1, Duration::seconds(1));
        let mut bucket = LeakyBucket::new(config);

        assert_eq!(bucket.remaining_capacity("user1"), 5);

        bucket.try_add("user1");
        bucket.try_add("user1");
        assert_eq!(bucket.remaining_capacity("user1"), 3);
    }

    #[test]
    fn test_leaky_bucket_reset() {
        let config = LeakyBucketConfig::new(2, 1, Duration::seconds(1));
        let mut bucket = LeakyBucket::new(config);

        bucket.try_add("user1");
        bucket.try_add("user1");
        assert!(!bucket.try_add("user1"));

        bucket.reset("user1");

        assert!(bucket.try_add("user1"));
    }

    #[test]
    fn test_leaky_bucket_config() {
        let strict = LeakyBucketConfig::strict();
        assert_eq!(strict.capacity, 10);

        let lenient = LeakyBucketConfig::lenient();
        assert_eq!(lenient.capacity, 100);
    }

    #[test]
    fn test_leaky_bucket_leak() {
        let config = LeakyBucketConfig::new(5, 2, Duration::milliseconds(50));
        let mut bucket = LeakyBucket::new(config);

        // Fill the bucket
        for _ in 0..5 {
            assert!(bucket.try_add("user1"));
        }
        assert_eq!(bucket.current_level("user1"), 5);

        // Wait for leak
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Should have leaked 4 tokens (2 per 50ms * 2 intervals = 4)
        assert_eq!(bucket.current_level("user1"), 1);
    }
}
