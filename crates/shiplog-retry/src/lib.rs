//! Retry/backoff utilities for shiplog operations.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Retry strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum RetryStrategy {
    /// Fixed delay between retries
    #[default]
    Fixed,
    /// Exponential backoff
    Exponential,
    /// Linear backoff
    Linear,
}

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retries
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    /// Initial delay in milliseconds
    #[serde(default = "default_initial_delay")]
    pub initial_delay_ms: u64,
    /// Maximum delay in milliseconds
    #[serde(default = "default_max_delay")]
    pub max_delay_ms: u64,
    /// Retry strategy
    #[serde(default)]
    pub strategy: RetryStrategy,
    /// Multiplier for backoff
    #[serde(default = "default_multiplier")]
    pub multiplier: f64,
}

fn default_max_retries() -> u32 {
    3
}

fn default_initial_delay() -> u64 {
    100
}

fn default_max_delay() -> u64 {
    30000
}

fn default_multiplier() -> f64 {
    2.0
}

/// Retry result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryResult<T> {
    pub result: Option<T>,
    pub attempts: u32,
    pub success: bool,
}

/// Retry utilities
pub struct Retry;

impl Retry {
    /// Calculate the delay for a given attempt
    pub fn calculate_delay(attempt: u32, config: &RetryConfig) -> Duration {
        let delay_ms = match config.strategy {
            RetryStrategy::Fixed => config.initial_delay_ms,
            RetryStrategy::Exponential => {
                let delay = config.initial_delay_ms as f64 * config.multiplier.powi(attempt as i32);
                delay.min(config.max_delay_ms as f64) as u64
            }
            RetryStrategy::Linear => {
                let delay = config.initial_delay_ms + ((attempt as u64) * config.initial_delay_ms);
                delay.min(config.max_delay_ms)
            }
        };

        Duration::from_millis(delay_ms)
    }

    /// Execute a synchronous operation with retry
    pub fn execute<T, E, F>(config: &RetryConfig, mut operation: F) -> RetryResult<T>
    where
        F: FnMut() -> Result<T, E>,
        E: std::fmt::Debug,
    {
        let mut attempts = 0;

        loop {
            attempts += 1;

            match operation() {
                Ok(result) => {
                    return RetryResult {
                        result: Some(result),
                        attempts,
                        success: true,
                    };
                }
                Err(e) if attempts >= config.max_retries => {
                    return RetryResult {
                        result: None,
                        attempts,
                        success: false,
                    };
                }
                Err(_) => {
                    if attempts < config.max_retries {
                        let delay = Self::calculate_delay(attempts, config);
                        std::thread::sleep(delay);
                    }
                }
            }
        }
    }

    /// Create a default retry config
    pub fn default_config() -> RetryConfig {
        RetryConfig {
            max_retries: 3,
            initial_delay_ms: 100,
            max_delay_ms: 30000,
            strategy: RetryStrategy::Exponential,
            multiplier: 2.0,
        }
    }
}

/// Async retry utilities
pub mod async_retry {
    use super::*;

    /// Execute an async operation with retry
    pub async fn execute_async<T, E, F, Fut>(
        config: &RetryConfig,
        mut operation: F,
    ) -> RetryResult<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Debug,
    {
        let mut attempts = 0;

        loop {
            attempts += 1;

            match operation().await {
                Ok(result) => {
                    return RetryResult {
                        result: Some(result),
                        attempts,
                        success: true,
                    };
                }
                Err(e) if attempts >= config.max_retries => {
                    return RetryResult {
                        result: None,
                        attempts,
                        success: false,
                    };
                }
                Err(_) => {
                    if attempts < config.max_retries {
                        let delay = Retry::calculate_delay(attempts, config);
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_config_default() {
        let config = RetryConfig {
            max_retries: 3,
            initial_delay_ms: 100,
            max_delay_ms: 30000,
            strategy: RetryStrategy::Exponential,
            multiplier: 2.0,
        };
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.strategy, RetryStrategy::Exponential);
    }

    #[test]
    fn retry_execute_success() {
        let config = Retry::default_config();

        let result = Retry::execute(&config, || Ok::<_, ()>("success".to_string()));

        assert!(result.success);
        assert_eq!(result.attempts, 1);
        assert_eq!(result.result.unwrap(), "success");
    }

    #[test]
    fn retry_execute_eventual_success() {
        let config = RetryConfig {
            max_retries: 3,
            initial_delay_ms: 1,
            max_delay_ms: 10,
            strategy: RetryStrategy::Fixed,
            multiplier: 2.0,
        };

        let mut call_count = 0;

        let result = Retry::execute(&config, || {
            call_count += 1;
            if call_count < 2 {
                Err::<String, _>(())
            } else {
                Ok("success".to_string())
            }
        });

        assert!(result.success);
        assert_eq!(result.attempts, 2);
    }

    #[test]
    fn retry_execute_failure() {
        let config = RetryConfig {
            max_retries: 3,
            initial_delay_ms: 1,
            max_delay_ms: 10,
            strategy: RetryStrategy::Fixed,
            multiplier: 2.0,
        };

        let result = Retry::execute(&config, || Err::<String, _>(()));

        assert!(!result.success);
        assert_eq!(result.attempts, 3);
        assert!(result.result.is_none());
    }

    #[test]
    fn calculate_delay_fixed() {
        let config = RetryConfig {
            max_retries: 3,
            initial_delay_ms: 100,
            max_delay_ms: 30000,
            strategy: RetryStrategy::Fixed,
            multiplier: 2.0,
        };

        let delay = Retry::calculate_delay(0, &config);
        assert_eq!(delay, Duration::from_millis(100));
    }

    #[test]
    fn calculate_delay_exponential() {
        let config = RetryConfig {
            max_retries: 3,
            initial_delay_ms: 100,
            max_delay_ms: 30000,
            strategy: RetryStrategy::Exponential,
            multiplier: 2.0,
        };

        let delay1 = Retry::calculate_delay(0, &config);
        let delay2 = Retry::calculate_delay(1, &config);
        let delay3 = Retry::calculate_delay(2, &config);

        assert_eq!(delay1, Duration::from_millis(100)); // 100 * 2^0
        assert_eq!(delay2, Duration::from_millis(200)); // 100 * 2^1
        assert_eq!(delay3, Duration::from_millis(400)); // 100 * 2^2
    }

    #[test]
    fn calculate_delay_max() {
        let config = RetryConfig {
            max_retries: 10,
            initial_delay_ms: 100,
            max_delay_ms: 500,
            strategy: RetryStrategy::Exponential,
            multiplier: 2.0,
        };

        let delay = Retry::calculate_delay(10, &config);
        assert_eq!(delay, Duration::from_millis(500)); // capped at max_delay
    }
}
