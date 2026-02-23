//! Circuit breaker pattern implementation for shiplog.
//!
//! This crate provides a circuit breaker implementation for handling
//! fault tolerance and preventing cascade failures.

use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;

/// Circuit breaker state.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CircuitState {
    /// Circuit is closed, requests pass through normally.
    #[default]
    Closed,
    /// Circuit is open, requests are rejected.
    Open,
    /// Circuit is half-open, testing if service recovered.
    HalfOpen,
}

/// Circuit breaker configuration.
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening the circuit.
    pub failure_threshold: u32,
    /// Number of successes needed to close the circuit from half-open.
    pub success_threshold: u32,
    /// Duration to wait before transitioning from open to half-open.
    pub timeout: Duration,
}

impl CircuitBreakerConfig {
    /// Create a new configuration.
    pub fn new(failure_threshold: u32, success_threshold: u32, timeout: Duration) -> Self {
        Self {
            failure_threshold,
            success_threshold,
            timeout,
        }
    }

    /// Create a strict configuration (opens quickly).
    pub fn strict() -> Self {
        Self {
            failure_threshold: 3,
            success_threshold: 2,
            timeout: Duration::seconds(30),
        }
    }

    /// Create a lenient configuration (more tolerant).
    pub fn lenient() -> Self {
        Self {
            failure_threshold: 10,
            success_threshold: 5,
            timeout: Duration::minutes(1),
        }
    }
}

/// Circuit breaker state for a single circuit.
#[derive(Debug, Clone)]
pub struct CircuitBreakerState {
    pub state: CircuitState,
    pub failure_count: u32,
    pub success_count: u32,
    pub last_failure: Option<DateTime<Utc>>,
    pub last_state_change: DateTime<Utc>,
}

impl Default for CircuitBreakerState {
    fn default() -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            last_failure: None,
            last_state_change: Utc::now(),
        }
    }
}

/// Circuit breaker for handling fault tolerance.
#[derive(Debug)]
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    circuits: HashMap<String, CircuitBreakerState>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with the given configuration.
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            circuits: HashMap::new(),
        }
    }

    /// Get or create the state for a circuit.
    fn get_state(&mut self, name: &str) -> &mut CircuitBreakerState {
        self.circuits.entry(name.to_string()).or_default()
    }

    /// Check if a circuit allows requests.
    pub fn is_available(&mut self, name: &str) -> bool {
        let timeout = self.config.timeout;

        let state = self.get_state(name);

        // Update state based on timeout
        if state.state == CircuitState::Open {
            let now = Utc::now();
            if now - state.last_state_change >= timeout {
                state.state = CircuitState::HalfOpen;
                state.success_count = 0;
                state.last_state_change = now;
            }
        }

        match state.state {
            CircuitState::Closed => true,
            CircuitState::Open => false,
            CircuitState::HalfOpen => true,
        }
    }

    /// Record a successful request.
    pub fn record_success(&mut self, name: &str) {
        let success_threshold = self.config.success_threshold;

        let state = self.get_state(name);

        match state.state {
            CircuitState::Closed => {
                state.failure_count = 0;
            }
            CircuitState::HalfOpen => {
                state.success_count += 1;
                if state.success_count >= success_threshold {
                    state.state = CircuitState::Closed;
                    state.failure_count = 0;
                    state.last_state_change = Utc::now();
                }
            }
            CircuitState::Open => {}
        }
    }

    /// Record a failed request.
    pub fn record_failure(&mut self, name: &str) {
        let failure_threshold = self.config.failure_threshold;

        let state = self.get_state(name);
        state.failure_count += 1;
        state.last_failure = Some(Utc::now());

        match state.state {
            CircuitState::Closed => {
                if state.failure_count >= failure_threshold {
                    state.state = CircuitState::Open;
                    state.last_state_change = Utc::now();
                }
            }
            CircuitState::HalfOpen => {
                state.state = CircuitState::Open;
                state.last_state_change = Utc::now();
            }
            CircuitState::Open => {
                state.last_state_change = Utc::now();
            }
        }
    }

    /// Get the current state of a circuit.
    pub fn get_state_info(&self, name: &str) -> Option<&CircuitBreakerState> {
        self.circuits.get(name)
    }

    /// Get the current state enum of a circuit.
    pub fn get_state_enum(&self, name: &str) -> CircuitState {
        self.circuits
            .get(name)
            .map(|s| s.state.clone())
            .unwrap_or(CircuitState::Closed)
    }

    /// Reset a circuit to closed state.
    pub fn reset(&mut self, name: &str) {
        if let Some(state) = self.circuits.get_mut(name) {
            *state = CircuitBreakerState::default();
        }
    }

    /// Remove a circuit completely.
    pub fn remove(&mut self, name: &str) {
        self.circuits.remove(name);
    }

    /// Clear all circuits.
    pub fn clear(&mut self) {
        self.circuits.clear();
    }

    /// Get the number of circuits.
    pub fn len(&self) -> usize {
        self.circuits.len()
    }

    /// Check if there are no circuits.
    pub fn is_empty(&self) -> bool {
        self.circuits.is_empty()
    }
}

/// Result type for circuit breaker operations.
pub type CircuitBreakerResult<T> = Result<T, CircuitBreakerError>;

/// Error type for circuit breaker operations.
#[derive(Debug, Clone)]
pub struct CircuitBreakerError {
    pub message: String,
    pub circuit_name: String,
}

impl CircuitBreakerError {
    pub fn new(message: impl Into<String>, circuit_name: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            circuit_name: circuit_name.into(),
        }
    }
}

impl std::fmt::Display for CircuitBreakerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Circuit '{}' error: {}", self.circuit_name, self.message)
    }
}

impl std::error::Error for CircuitBreakerError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_initial_state() {
        let config = CircuitBreakerConfig::strict();
        let mut breaker = CircuitBreaker::new(config);

        assert!(breaker.is_available("test"));
    }

    #[test]
    fn test_circuit_opens_after_failures() {
        let config = CircuitBreakerConfig::new(3, 2, Duration::seconds(1));
        let mut breaker = CircuitBreaker::new(config);

        breaker.record_failure("test");
        breaker.record_failure("test");
        breaker.record_failure("test");

        assert!(!breaker.is_available("test"));
        assert_eq!(breaker.get_state_enum("test"), CircuitState::Open);
    }

    #[test]
    fn test_circuit_closes_after_successes() {
        let config = CircuitBreakerConfig::new(2, 2, Duration::milliseconds(50));
        let mut breaker = CircuitBreaker::new(config);

        breaker.record_failure("test");
        breaker.record_failure("test");
        assert!(!breaker.is_available("test"));

        std::thread::sleep(std::time::Duration::from_millis(100));

        assert!(breaker.is_available("test"));

        breaker.record_success("test");
        breaker.record_success("test");

        assert!(breaker.is_available("test"));
    }

    #[test]
    fn test_circuit_half_open() {
        let config = CircuitBreakerConfig::new(2, 2, Duration::milliseconds(50));
        let mut breaker = CircuitBreaker::new(config);

        breaker.record_failure("test");
        breaker.record_failure("test");

        std::thread::sleep(std::time::Duration::from_millis(100));

        assert!(breaker.is_available("test"));
        assert_eq!(breaker.get_state_enum("test"), CircuitState::HalfOpen);
    }

    #[test]
    fn test_circuit_resets() {
        let config = CircuitBreakerConfig::strict();
        let mut breaker = CircuitBreaker::new(config);

        breaker.record_failure("test");
        breaker.record_failure("test");
        breaker.record_failure("test");

        breaker.reset("test");

        assert!(breaker.is_available("test"));
    }

    #[test]
    fn test_circuit_config() {
        let strict = CircuitBreakerConfig::strict();
        assert_eq!(strict.failure_threshold, 3);

        let lenient = CircuitBreakerConfig::lenient();
        assert_eq!(lenient.failure_threshold, 10);
    }
}
