//! Timeout utilities for shiplog.
//!
//! This crate provides timeout utilities for async operations in shiplog.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

/// Configuration for timeout operations
#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    pub default_timeout_ms: u64,
    pub retry_on_timeout: bool,
    pub max_retries: u32,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            default_timeout_ms: 5000,
            retry_on_timeout: false,
            max_retries: 3,
        }
    }
}

/// Builder for timeout configuration
#[derive(Debug)]
pub struct TimeoutBuilder {
    config: TimeoutConfig,
}

impl TimeoutBuilder {
    pub fn new() -> Self {
        Self {
            config: TimeoutConfig::default(),
        }
    }

    pub fn default_timeout(mut self, ms: u64) -> Self {
        self.config.default_timeout_ms = ms;
        self
    }

    pub fn retry_on_timeout(mut self, retry: bool) -> Self {
        self.config.retry_on_timeout = retry;
        self
    }

    pub fn max_retries(mut self, retries: u32) -> Self {
        self.config.max_retries = retries;
        self
    }

    pub fn build(self) -> TimeoutConfig {
        self.config
    }
}

impl Default for TimeoutBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Timeout error type
#[derive(Debug, Clone)]
pub struct TimeoutError {
    pub elapsed_ms: u64,
    pub message: String,
}

impl TimeoutError {
    pub fn new(elapsed_ms: u64, message: &str) -> Self {
        Self {
            elapsed_ms,
            message: message.to_string(),
        }
    }
}

impl std::fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Timeout after {}ms: {}", self.elapsed_ms, self.message)
    }
}

impl std::error::Error for TimeoutError {}

/// Result type for timeout operations
#[derive(Debug)]
pub enum TimeoutResult<T> {
    Ok(T, u64),
    Err(TimeoutError),
}

impl<T> TimeoutResult<T> {
    pub fn ok(value: T, elapsed_ms: u64) -> Self {
        Self::Ok(value, elapsed_ms)
    }

    pub fn err(elapsed_ms: u64, message: &str) -> Self {
        Self::Err(TimeoutError::new(elapsed_ms, message))
    }

    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Ok(_, _))
    }

    pub fn is_timeout(&self) -> bool {
        matches!(self, Self::Err(_))
    }

    pub fn into_result(self) -> Result<T, TimeoutError> {
        match self {
            Self::Ok(value, _) => Ok(value),
            Self::Err(e) => Err(e),
        }
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
    type Output = Result<F::Output, TimeoutError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        match Pin::new(&mut this.future).poll(cx) {
            Poll::Ready(output) => Poll::Ready(Ok(output)),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Extension trait for adding timeout to futures
pub trait TimeoutExt: Future + Sized {
    fn with_timeout(self) -> Timeout<Self>;
}

impl<F: Future + Unpin> TimeoutExt for F {
    fn with_timeout(self) -> Timeout<Self> {
        Timeout::new(self)
    }
}

/// Timeout wrapper for tokio operations
pub struct TokioTimeout;

impl TokioTimeout {
    /// Run a future with timeout
    pub async fn run<F, T, E>(future: F, timeout: Duration) -> Result<T, TimeoutError>
    where
        F: Future<Output = Result<T, E>>,
        E: std::fmt::Debug,
    {
        tokio::time::timeout(timeout, future)
            .await
            .map_err(|_| TimeoutError::new(timeout.as_millis() as u64, "Operation timed out"))?
            .map_err(|e| TimeoutError::new(0, &format!("{:?}", e)))
    }

    /// Run a future with a default timeout from config
    pub async fn run_with_config<F, T, E>(
        future: F,
        config: &TimeoutConfig,
    ) -> Result<T, TimeoutError>
    where
        F: Future<Output = Result<T, E>>,
        E: std::fmt::Debug,
    {
        let timeout = Duration::from_millis(config.default_timeout_ms);
        Self::run(future, timeout).await
    }
}

/// Timer state for tracking timeouts
#[derive(Debug)]
pub struct Timer {
    duration: Duration,
    started: Option<Instant>,
}

impl Timer {
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            started: None,
        }
    }

    pub fn start(&mut self) {
        self.started = Some(Instant::now());
    }

    pub fn is_expired(&self) -> bool {
        if let Some(started) = self.started {
            started.elapsed() >= self.duration
        } else {
            false
        }
    }

    pub fn remaining(&self) -> Option<Duration> {
        if let Some(started) = self.started {
            let elapsed = started.elapsed();
            if elapsed >= self.duration {
                None
            } else {
                Some(self.duration - elapsed)
            }
        } else {
            Some(self.duration)
        }
    }

    pub fn elapsed(&self) -> Option<Duration> {
        self.started.map(|s| s.elapsed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeout_config_default() {
        let config = TimeoutConfig::default();
        assert_eq!(config.default_timeout_ms, 5000);
        assert!(!config.retry_on_timeout);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_timeout_builder() {
        let config = TimeoutBuilder::new()
            .default_timeout(1000)
            .retry_on_timeout(true)
            .max_retries(5)
            .build();

        assert_eq!(config.default_timeout_ms, 1000);
        assert!(config.retry_on_timeout);
        assert_eq!(config.max_retries, 5);
    }

    #[test]
    fn test_timeout_result() {
        let ok_result = TimeoutResult::ok(42, 100);
        assert!(ok_result.is_ok());
        assert!(!ok_result.is_timeout());

        let err_result = TimeoutResult::<i32>::err(5000, "timeout");
        assert!(!err_result.is_ok());
        assert!(err_result.is_timeout());
    }

    #[test]
    fn test_timer() {
        let mut timer = Timer::new(Duration::from_secs(1));

        assert!(!timer.is_expired());
        assert!(timer.remaining().is_some());

        timer.start();
        assert!(!timer.is_expired());
        assert!(timer.elapsed().is_some());
    }
}
