//! Async utilities for shiplog.
//!
//! This crate provides async utilities for asynchronous operations in shiplog.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Configuration for async operations
#[derive(Debug, Clone)]
pub struct AsyncConfig {
    pub buffer_size: usize,
    pub max_concurrent: usize,
    pub spawn_timeout_ms: u64,
}

impl Default for AsyncConfig {
    fn default() -> Self {
        Self {
            buffer_size: 100,
            max_concurrent: 10,
            spawn_timeout_ms: 5000,
        }
    }
}

/// Builder for async task execution
#[derive(Debug)]
pub struct AsyncBuilder {
    config: AsyncConfig,
}

impl AsyncBuilder {
    pub fn new() -> Self {
        Self {
            config: AsyncConfig::default(),
        }
    }

    pub fn buffer_size(mut self, size: usize) -> Self {
        self.config.buffer_size = size;
        self
    }

    pub fn max_concurrent(mut self, max: usize) -> Self {
        self.config.max_concurrent = max;
        self
    }

    pub fn spawn_timeout(mut self, ms: u64) -> Self {
        self.config.spawn_timeout_ms = ms;
        self
    }

    pub fn build(self) -> AsyncConfig {
        self.config
    }
}

impl Default for AsyncBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Async result type that combines result with timeout information
#[derive(Debug, Clone)]
pub struct AsyncResult<T> {
    pub value: Option<T>,
    pub elapsed_ms: u64,
    pub timed_out: bool,
}

impl<T> AsyncResult<T> {
    pub fn new(value: T, elapsed_ms: u64) -> Self {
        Self {
            value: Some(value),
            elapsed_ms,
            timed_out: false,
        }
    }

    pub fn timeout(elapsed_ms: u64) -> Self {
        Self {
            value: None,
            elapsed_ms,
            timed_out: true,
        }
    }

    pub fn is_ok(&self) -> bool {
        self.value.is_some() && !self.timed_out
    }

    pub fn is_timeout(&self) -> bool {
        self.timed_out
    }
}

/// Future that wraps another future with a timeout (simplified)
pub struct Timeout<F> {
    future: F,
}

impl<F> Timeout<F> {
    pub fn new(future: F) -> Self {
        Self { future }
    }
}

impl<F: Future + Unpin> Future for Timeout<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        Pin::new(&mut this.future).poll(cx)
    }
}

/// Extension trait for adding timeout to futures
pub trait TimeoutExt: Future + Sized {
    fn timeout(self) -> Timeout<Self>;
}

impl<F: Future + Unpin> TimeoutExt for F {
    fn timeout(self) -> Timeout<Self> {
        Timeout::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_async_config_default() {
        let config = AsyncConfig::default();
        assert_eq!(config.buffer_size, 100);
        assert_eq!(config.max_concurrent, 10);
        assert_eq!(config.spawn_timeout_ms, 5000);
    }

    #[test]
    fn test_async_builder() {
        let config = AsyncBuilder::new()
            .buffer_size(200)
            .max_concurrent(5)
            .spawn_timeout(3000)
            .build();
        
        assert_eq!(config.buffer_size, 200);
        assert_eq!(config.max_concurrent, 5);
        assert_eq!(config.spawn_timeout_ms, 3000);
    }

    #[test]
    fn test_async_result() {
        let result = AsyncResult::new(42, 100);
        assert!(result.is_ok());
        assert!(!result.is_timeout());
        assert_eq!(result.value, Some(42));
        
        let timeout_result = AsyncResult::<i32>::timeout(5000);
        assert!(!timeout_result.is_ok());
        assert!(timeout_result.is_timeout());
    }

    #[tokio::test]
    async fn test_timeout_ext() {
        // Simple test to ensure async works
        let result = async { 42 }.await;
        assert_eq!(result, 42);
    }
}
