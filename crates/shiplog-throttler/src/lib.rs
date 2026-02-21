//! Rate limiting and throttling utilities for shiplog.
//!
//! This crate provides throttling implementations for controlling
//! the rate of operations.

use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;

/// Throttler configuration.
#[derive(Debug, Clone)]
pub struct ThrottlerConfig {
    /// Maximum number of requests allowed per window.
    pub max_requests: u32,
    /// Time window duration.
    pub window: Duration,
}

impl ThrottlerConfig {
    /// Create a new throttler configuration.
    pub fn new(max_requests: u32, window: Duration) -> Self {
        Self {
            max_requests,
            window,
        }
    }

    /// Create a strict configuration (10 requests per minute).
    pub fn strict() -> Self {
        Self {
            max_requests: 10,
            window: Duration::minutes(1),
        }
    }

    /// Create a lenient configuration (100 requests per minute).
    pub fn lenient() -> Self {
        Self {
            max_requests: 100,
            window: Duration::minutes(1),
        }
    }
}

impl Default for ThrottlerConfig {
    fn default() -> Self {
        Self::lenient()
    }
}

/// State for a throttler entry.
#[derive(Debug, Clone)]
pub struct ThrottlerState {
    pub request_count: u32,
    pub window_start: DateTime<Utc>,
}

/// Throttler for rate limiting requests.
#[derive(Debug)]
pub struct Throttler {
    config: ThrottlerConfig,
    states: HashMap<String, ThrottlerState>,
}

impl Throttler {
    /// Create a new throttler with the given configuration.
    pub fn new(config: ThrottlerConfig) -> Self {
        Self {
            config,
            states: HashMap::new(),
        }
    }

    /// Check if a request is allowed and record it if so.
    pub fn try_acquire(&mut self, key: &str) -> bool {
        let now = Utc::now();
        let window = self.config.window;
        let max_requests = self.config.max_requests;

        let state = self
            .states
            .entry(key.to_string())
            .or_insert_with(|| ThrottlerState {
                request_count: 0,
                window_start: now,
            });

        // Check if window has expired
        if now - state.window_start >= window {
            // Reset the window
            state.request_count = 1;
            state.window_start = now;
            return true;
        }

        // Check if we've exceeded the limit
        if state.request_count >= max_requests {
            return false;
        }

        // Increment the counter
        state.request_count += 1;
        true
    }

    /// Get the current state for a key.
    pub fn get_state(&self, key: &str) -> Option<&ThrottlerState> {
        self.states.get(key)
    }

    /// Get the number of remaining requests for a key.
    pub fn remaining(&self, key: &str) -> u32 {
        if let Some(state) = self.states.get(key) {
            let now = Utc::now();
            if now - state.window_start < self.config.window {
                return self.config.max_requests.saturating_sub(state.request_count);
            }
        }
        self.config.max_requests
    }

    /// Reset the throttler for a key.
    pub fn reset(&mut self, key: &str) {
        self.states.remove(key);
    }

    /// Clear all throttler states.
    pub fn clear(&mut self) {
        self.states.clear();
    }

    /// Get the number of tracked keys.
    pub fn len(&self) -> usize {
        self.states.len()
    }

    /// Check if there are no tracked keys.
    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }
}

/// Result type for throttler operations.
pub type ThrottlerResult<T> = Result<T, ThrottlerError>;

/// Error type for throttler operations.
#[derive(Debug, Clone)]
pub struct ThrottlerError {
    pub message: String,
    pub key: String,
}

impl ThrottlerError {
    pub fn new(message: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            key: key.into(),
        }
    }
}

impl std::fmt::Display for ThrottlerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Throttler error for '{}': {}", self.key, self.message)
    }
}

impl std::error::Error for ThrottlerError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_throttler_initial_state() {
        let config = ThrottlerConfig::strict();
        let throttler = Throttler::new(config);

        assert!(throttler.is_empty());
    }

    #[test]
    fn test_throttler_allows_requests_within_limit() {
        let config = ThrottlerConfig::new(3, Duration::minutes(1));
        let mut throttler = Throttler::new(config);

        // First 3 requests should be allowed
        assert!(throttler.try_acquire("user1"));
        assert!(throttler.try_acquire("user1"));
        assert!(throttler.try_acquire("user1"));

        // 4th request should be denied
        assert!(!throttler.try_acquire("user1"));
    }

    #[test]
    fn test_throttler_separate_keys() {
        let config = ThrottlerConfig::new(2, Duration::minutes(1));
        let mut throttler = Throttler::new(config);

        // Each key should have its own limit
        assert!(throttler.try_acquire("user1"));
        assert!(throttler.try_acquire("user1"));
        assert!(!throttler.try_acquire("user1"));

        assert!(throttler.try_acquire("user2"));
        assert!(throttler.try_acquire("user2"));
        assert!(!throttler.try_acquire("user2"));
    }

    #[test]
    fn test_throttler_remaining() {
        let config = ThrottlerConfig::new(3, Duration::minutes(1));
        let mut throttler = Throttler::new(config);

        assert_eq!(throttler.remaining("user1"), 3);

        throttler.try_acquire("user1");
        assert_eq!(throttler.remaining("user1"), 2);

        throttler.try_acquire("user1");
        assert_eq!(throttler.remaining("user1"), 1);

        throttler.try_acquire("user1");
        assert_eq!(throttler.remaining("user1"), 0);
    }

    #[test]
    fn test_throttler_reset() {
        let config = ThrottlerConfig::new(2, Duration::minutes(1));
        let mut throttler = Throttler::new(config);

        throttler.try_acquire("user1");
        throttler.try_acquire("user1");
        assert!(!throttler.try_acquire("user1"));

        throttler.reset("user1");

        assert!(throttler.try_acquire("user1"));
        assert_eq!(throttler.remaining("user1"), 1);
    }

    #[test]
    fn test_throttler_config() {
        let strict = ThrottlerConfig::strict();
        assert_eq!(strict.max_requests, 10);

        let lenient = ThrottlerConfig::lenient();
        assert_eq!(lenient.max_requests, 100);
    }

    #[test]
    fn test_throttler_window_reset() {
        let config = ThrottlerConfig::new(1, Duration::milliseconds(50));
        let mut throttler = Throttler::new(config);

        assert!(throttler.try_acquire("user1"));
        assert!(!throttler.try_acquire("user1"));

        // Wait for window to expire
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Should be allowed again
        assert!(throttler.try_acquire("user1"));
    }
}
