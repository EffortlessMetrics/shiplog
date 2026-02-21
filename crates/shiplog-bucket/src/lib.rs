//! Token bucket implementation for shiplog.
//!
//! This crate provides a token bucket implementation for rate limiting.

use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;

/// Token bucket configuration.
#[derive(Debug, Clone)]
pub struct TokenBucketConfig {
    /// Maximum number of tokens in the bucket.
    pub capacity: u64,
    /// Number of tokens added per refill interval.
    pub refill_rate: u64,
    /// Interval between refills.
    pub refill_interval: Duration,
}

impl TokenBucketConfig {
    /// Create a new token bucket configuration.
    pub fn new(capacity: u64, refill_rate: u64, refill_interval: Duration) -> Self {
        Self {
            capacity,
            refill_rate,
            refill_interval,
        }
    }

    /// Create a strict configuration (10 tokens, refills 1 per second).
    pub fn strict() -> Self {
        Self {
            capacity: 10,
            refill_rate: 1,
            refill_interval: Duration::seconds(1),
        }
    }

    /// Create a lenient configuration (100 tokens, refills 10 per second).
    pub fn lenient() -> Self {
        Self {
            capacity: 100,
            refill_rate: 10,
            refill_interval: Duration::seconds(1),
        }
    }
}

impl Default for TokenBucketConfig {
    fn default() -> Self {
        Self::lenient()
    }
}

/// Token bucket state.
#[derive(Debug, Clone)]
pub struct TokenBucketState {
    pub tokens: u64,
    pub last_refill: DateTime<Utc>,
}

/// Token bucket for rate limiting.
#[derive(Debug)]
pub struct TokenBucket {
    config: TokenBucketConfig,
    buckets: HashMap<String, TokenBucketState>,
}

impl TokenBucket {
    /// Create a new token bucket with the given configuration.
    pub fn new(config: TokenBucketConfig) -> Self {
        Self {
            config,
            buckets: HashMap::new(),
        }
    }

    /// Try to consume tokens from the bucket.
    pub fn try_consume(&mut self, key: &str, tokens: u64) -> bool {
        let now = Utc::now();
        
        let state = self.buckets.entry(key.to_string()).or_insert_with(|| {
            TokenBucketState {
                tokens: self.config.capacity,
                last_refill: now,
            }
        });

        // Refill tokens based on time elapsed
        let elapsed = now - state.last_refill;
        if elapsed >= self.config.refill_interval {
            let elapsed_ms = elapsed.num_milliseconds();
            let interval_ms = self.config.refill_interval.num_milliseconds();
            let intervals = (elapsed_ms / interval_ms) as u64;
            state.tokens = std::cmp::min(
                state.tokens + intervals * self.config.refill_rate,
                self.config.capacity,
            );
            state.last_refill = now;
        }

        // Try to consume tokens
        if state.tokens >= tokens {
            state.tokens -= tokens;
            true
        } else {
            false
        }
    }

    /// Get the current number of tokens for a key.
    pub fn available_tokens(&self, key: &str) -> u64 {
        if let Some(state) = self.buckets.get(key) {
            let now = Utc::now();
            let elapsed = now - state.last_refill;
            if elapsed >= self.config.refill_interval {
                let elapsed_ms = elapsed.num_milliseconds();
                let interval_ms = self.config.refill_interval.num_milliseconds();
                let intervals = (elapsed_ms / interval_ms) as u64;
                return std::cmp::min(
                    state.tokens + intervals * self.config.refill_rate,
                    self.config.capacity,
                );
            }
            state.tokens
        } else {
            self.config.capacity
        }
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

/// Result type for token bucket operations.
pub type TokenBucketResult<T> = Result<T, TokenBucketError>;

/// Error type for token bucket operations.
#[derive(Debug, Clone)]
pub struct TokenBucketError {
    pub message: String,
    pub key: String,
}

impl TokenBucketError {
    pub fn new(message: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            key: key.into(),
        }
    }
}

impl std::fmt::Display for TokenBucketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TokenBucket error for '{}': {}", self.key, self.message)
    }
}

impl std::error::Error for TokenBucketError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket_initial_state() {
        let config = TokenBucketConfig::strict();
        let bucket = TokenBucket::new(config);

        assert!(bucket.is_empty());
    }

    #[test]
    fn test_token_bucket_consume() {
        let config = TokenBucketConfig::new(5, 1, Duration::seconds(1));
        let mut bucket = TokenBucket::new(config);

        // Should be able to consume up to capacity
        assert!(bucket.try_consume("user1", 1));
        assert!(bucket.try_consume("user1", 2));
        assert!(bucket.try_consume("user1", 2));
        
        // Should fail when bucket is empty
        assert!(!bucket.try_consume("user1", 1));
    }

    #[test]
    fn test_token_bucket_separate_keys() {
        let config = TokenBucketConfig::new(2, 1, Duration::seconds(1));
        let mut bucket = TokenBucket::new(config);

        // Each key should have its own bucket
        assert!(bucket.try_consume("user1", 1));
        assert!(bucket.try_consume("user1", 1));
        assert!(!bucket.try_consume("user1", 1));

        assert!(bucket.try_consume("user2", 1));
        assert!(bucket.try_consume("user2", 1));
        assert!(!bucket.try_consume("user2", 1));
    }

    #[test]
    fn test_token_bucket_available_tokens() {
        let config = TokenBucketConfig::new(5, 1, Duration::seconds(1));
        let mut bucket = TokenBucket::new(config);

        assert_eq!(bucket.available_tokens("user1"), 5);

        bucket.try_consume("user1", 2);
        assert_eq!(bucket.available_tokens("user1"), 3);
    }

    #[test]
    fn test_token_bucket_reset() {
        let config = TokenBucketConfig::new(2, 1, Duration::seconds(1));
        let mut bucket = TokenBucket::new(config);

        bucket.try_consume("user1", 2);
        assert!(!bucket.try_consume("user1", 1));

        bucket.reset("user1");

        assert!(bucket.try_consume("user1", 1));
    }

    #[test]
    fn test_token_bucket_config() {
        let strict = TokenBucketConfig::strict();
        assert_eq!(strict.capacity, 10);

        let lenient = TokenBucketConfig::lenient();
        assert_eq!(lenient.capacity, 100);
    }

    #[test]
    fn test_token_bucket_refill() {
        let config = TokenBucketConfig::new(3, 2, Duration::milliseconds(50));
        let mut bucket = TokenBucket::new(config);

        // Consume all tokens
        assert!(bucket.try_consume("user1", 3));
        assert_eq!(bucket.available_tokens("user1"), 0);

        // Wait for refill
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Should have 2 tokens now (2 refills * 2 tokens each = 4, but max is 3)
        assert!(bucket.try_consume("user1", 2));
    }
}
