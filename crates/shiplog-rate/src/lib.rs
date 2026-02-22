//! Rate limiting utilities for shiplog.
//!
//! This crate provides rate limiting utilities including token bucket
//! and sliding window rate limiters.

use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;

/// Rate limiter state for a single key.
#[derive(Debug, Clone)]
pub struct RateLimitState {
    pub tokens: f64,
    pub last_update: DateTime<Utc>,
}

/// Configuration for rate limiting.
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum number of tokens in the bucket.
    pub max_tokens: f64,
    /// Number of tokens added per second.
    pub refill_rate: f64,
}

impl RateLimitConfig {
    /// Create a new rate limit configuration.
    pub fn new(max_tokens: f64, refill_rate: f64) -> Self {
        Self {
            max_tokens,
            refill_rate,
        }
    }

    /// Create a configuration for requests per second.
    pub fn per_second(max_per_second: u32) -> Self {
        Self {
            max_tokens: max_per_second as f64,
            refill_rate: max_per_second as f64,
        }
    }

    /// Create a configuration for requests per minute.
    pub fn per_minute(max_per_minute: u32) -> Self {
        Self {
            max_tokens: max_per_minute as f64,
            refill_rate: max_per_minute as f64 / 60.0,
        }
    }

    /// Create a configuration for requests per hour.
    pub fn per_hour(max_per_hour: u32) -> Self {
        Self {
            max_tokens: max_per_hour as f64,
            refill_rate: max_per_hour as f64 / 3600.0,
        }
    }
}

/// Token bucket rate limiter.
#[derive(Debug)]
pub struct TokenBucket {
    config: RateLimitConfig,
    states: HashMap<String, RateLimitState>,
}

impl TokenBucket {
    /// Create a new token bucket rate limiter.
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            states: HashMap::new(),
        }
    }

    /// Try to consume a token for the given key.
    /// Returns true if the token was consumed, false if rate limited.
    pub fn try_consume(&mut self, key: &str) -> bool {
        let now = Utc::now();
        let state = self
            .states
            .entry(key.to_string())
            .or_insert(RateLimitState {
                tokens: self.config.max_tokens,
                last_update: now,
            });

        // Calculate time elapsed and refill tokens
        let elapsed = (now - state.last_update).num_milliseconds() as f64 / 1000.0;
        state.tokens =
            (state.tokens + elapsed * self.config.refill_rate).min(self.config.max_tokens);
        state.last_update = now;

        if state.tokens >= 1.0 {
            state.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Get the number of available tokens for a key.
    pub fn available_tokens(&self, key: &str) -> f64 {
        if let Some(state) = self.states.get(key) {
            let now = Utc::now();
            let elapsed = (now - state.last_update).num_milliseconds() as f64 / 1000.0;
            (state.tokens + elapsed * self.config.refill_rate).min(self.config.max_tokens)
        } else {
            self.config.max_tokens
        }
    }

    /// Check if the key is rate limited (has 0 tokens).
    pub fn is_rate_limited(&self, key: &str) -> bool {
        self.available_tokens(key) < 1.0
    }

    /// Remove a key's state (for cleanup).
    pub fn remove(&mut self, key: &str) {
        self.states.remove(key);
    }

    /// Clear all states.
    pub fn clear(&mut self) {
        self.states.clear();
    }
}

/// Simple sliding window rate limiter.
#[derive(Debug)]
pub struct SlidingWindow {
    max_requests: usize,
    window_size: Duration,
    requests: HashMap<String, Vec<DateTime<Utc>>>,
}

impl SlidingWindow {
    /// Create a new sliding window rate limiter.
    pub fn new(max_requests: usize, window_size: Duration) -> Self {
        Self {
            max_requests,
            window_size,
            requests: HashMap::new(),
        }
    }

    /// Try to record a request.
    /// Returns true if the request is allowed, false if rate limited.
    pub fn try_record(&mut self, key: &str) -> bool {
        let now = Utc::now();
        let window_start = now - self.window_size;

        let key_requests = self.requests.entry(key.to_string()).or_default();

        // Remove old requests outside the window
        key_requests.retain(|t| *t > window_start);

        if key_requests.len() < self.max_requests {
            key_requests.push(now);
            true
        } else {
            false
        }
    }

    /// Get the number of requests in the current window.
    pub fn current_requests(&self, key: &str) -> usize {
        let now = Utc::now();
        let window_start = now - self.window_size;

        self.requests
            .get(key)
            .map(|reqs| reqs.iter().filter(|t| **t > window_start).count())
            .unwrap_or(0)
    }

    /// Check if the key is rate limited.
    pub fn is_rate_limited(&self, key: &str) -> bool {
        self.current_requests(key) >= self.max_requests
    }

    /// Remove a key's state.
    pub fn remove(&mut self, key: &str) {
        self.requests.remove(key);
    }

    /// Clear all states.
    pub fn clear(&mut self) {
        self.requests.clear();
    }
}

/// Create a rate limit config for GitHub API (5000 requests/hour).
pub fn github_api_limit() -> RateLimitConfig {
    RateLimitConfig::per_hour(5000)
}

/// Create a rate limit config for typical web API (100 requests/minute).
pub fn standard_api_limit() -> RateLimitConfig {
    RateLimitConfig::per_minute(100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket_try_consume() {
        let config = RateLimitConfig::per_second(10);
        let mut bucket = TokenBucket::new(config);

        // First request should succeed
        assert!(bucket.try_consume("user1"));

        // Immediate second request should also succeed (we have 10 tokens)
        assert!(bucket.try_consume("user1"));
    }

    #[test]
    fn test_token_bucket_exhausted() {
        let config = RateLimitConfig::new(1.0, 0.0); // No refill
        let mut bucket = TokenBucket::new(config);

        assert!(bucket.try_consume("user1"));
        assert!(!bucket.try_consume("user1"));
    }

    #[test]
    fn test_token_bucket_available_tokens() {
        let config = RateLimitConfig::per_second(10);
        let bucket = TokenBucket::new(config);

        let tokens = bucket.available_tokens("user1");
        assert!(tokens >= 10.0);
    }

    #[test]
    fn test_sliding_window_try_record() {
        let config = SlidingWindow::new(5, Duration::seconds(60));
        let mut window = config;

        for _ in 0..5 {
            assert!(window.try_record("user1"));
        }

        // Sixth request should be rate limited
        assert!(!window.try_record("user1"));
    }

    #[test]
    fn test_sliding_window_current_requests() {
        let mut window = SlidingWindow::new(10, Duration::seconds(60));

        window.try_record("user1");
        window.try_record("user1");

        assert_eq!(window.current_requests("user1"), 2);
    }

    #[test]
    fn test_rate_limit_configs() {
        let github = github_api_limit();
        assert_eq!(github.max_tokens, 5000.0);

        let standard = standard_api_limit();
        assert_eq!(standard.max_tokens, 100.0);
    }
}
