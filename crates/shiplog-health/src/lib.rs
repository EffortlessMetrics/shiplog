//! Health check utilities for shiplog.
//!
//! This crate provides health check utilities for the shiplog ecosystem,
//! allowing applications to expose health status and readiness probes.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Health status of a component
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// Component is healthy
    #[default]
    Healthy,
    /// Component is degraded but working
    Degraded,
    /// Component is unhealthy
    Unhealthy,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "healthy"),
            HealthStatus::Degraded => write!(f, "degraded"),
            HealthStatus::Unhealthy => write!(f, "unhealthy"),
        }
    }
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// Name of the health check
    pub name: String,
    /// Status of the check
    pub status: HealthStatus,
    /// Optional message describing the status
    pub message: Option<String>,
    /// Timestamp of the check
    pub timestamp: i64,
}

impl HealthCheckResult {
    /// Creates a new health check result
    pub fn new(name: impl Into<String>, status: HealthStatus) -> Self {
        Self {
            name: name.into(),
            status,
            message: None,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    /// Creates a healthy result
    pub fn healthy(name: impl Into<String>) -> Self {
        Self::new(name, HealthStatus::Healthy)
    }

    /// Creates a degraded result
    pub fn degraded(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: HealthStatus::Degraded,
            message: Some(message.into()),
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    /// Creates an unhealthy result
    pub fn unhealthy(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: HealthStatus::Unhealthy,
            message: Some(message.into()),
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}

/// A health check function type
pub type HealthCheckFn = Box<dyn Fn() -> HealthCheckResult + Send + Sync>;

/// Creates a health check function from a closure
pub fn make_health_check<F>(name: impl Into<String>, check_fn: F) -> (String, HealthCheckFn)
where
    F: Fn() -> HealthCheckResult + Send + Sync + 'static,
{
    (name.into(), Box::new(check_fn))
}

/// Health check registry
pub struct HealthRegistry {
    checks: RwLock<Vec<(String, HealthCheckFn)>>,
}

impl HealthRegistry {
    /// Creates a new health registry
    pub fn new() -> Self {
        Self {
            checks: RwLock::new(Vec::new()),
        }
    }

    /// Registers a health check
    pub async fn register(&self, name: impl Into<String>, check_fn: HealthCheckFn) {
        let mut checks = self.checks.write().await;
        checks.push((name.into(), check_fn));
    }

    /// Registers a simple health check
    pub async fn register_simple<F>(&self, name: impl Into<String>, check_fn: F)
    where
        F: Fn() -> HealthCheckResult + Send + Sync + 'static,
    {
        self.register(name, Box::new(check_fn)).await;
    }

    /// Performs all health checks
    pub async fn check_all(&self) -> Vec<HealthCheckResult> {
        let checks = self.checks.read().await;
        checks.iter().map(|(_, check)| check()).collect()
    }

    /// Returns the overall health status
    pub async fn overall_status(&self) -> HealthStatus {
        let results = self.check_all().await;

        if results.iter().any(|r| r.status == HealthStatus::Unhealthy) {
            HealthStatus::Unhealthy
        } else if results.iter().any(|r| r.status == HealthStatus::Degraded) {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        }
    }

    /// Returns the number of registered checks
    pub async fn len(&self) -> usize {
        self.checks.read().await.len()
    }

    /// Returns whether there are any registered checks
    pub async fn is_empty(&self) -> bool {
        self.checks.read().await.is_empty()
    }
}

impl Default for HealthRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Health monitor that tracks health over time
pub struct HealthMonitor {
    status: RwLock<HealthStatus>,
    last_check: RwLock<Option<Instant>>,
    check_interval: Duration,
}

impl HealthMonitor {
    /// Creates a new health monitor
    pub fn new(check_interval: Duration) -> Self {
        Self {
            status: RwLock::new(HealthStatus::Healthy),
            last_check: RwLock::new(None),
            check_interval,
        }
    }

    /// Updates the current health status
    pub async fn set_status(&self, status: HealthStatus) {
        *self.status.write().await = status;
        *self.last_check.write().await = Some(Instant::now());
    }

    /// Gets the current health status
    pub async fn get_status(&self) -> HealthStatus {
        *self.status.read().await
    }

    /// Gets the time since last check
    pub async fn time_since_last_check(&self) -> Option<Duration> {
        self.last_check.read().await.map(|t| t.elapsed())
    }

    /// Gets the check interval
    pub fn check_interval(&self) -> Duration {
        self.check_interval
    }
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new(Duration::from_secs(30))
    }
}

/// A readiness check that can be used for Kubernetes readiness probes
pub struct ReadinessCheck {
    ready: tokio::sync::Mutex<bool>,
}

impl ReadinessCheck {
    /// Creates a new readiness check
    pub fn new() -> Self {
        Self {
            ready: tokio::sync::Mutex::new(false),
        }
    }

    /// Marks the component as ready
    pub async fn set_ready(&self) {
        *self.ready.lock().await = true;
    }

    /// Marks the component as not ready
    pub async fn set_not_ready(&self) {
        *self.ready.lock().await = false;
    }

    /// Checks if the component is ready
    pub async fn is_ready(&self) -> bool {
        *self.ready.lock().await
    }
}

impl Default for ReadinessCheck {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_values() {
        assert_eq!(HealthStatus::Healthy, HealthStatus::Healthy);
        assert_eq!(HealthStatus::Degraded, HealthStatus::Degraded);
        assert_eq!(HealthStatus::Unhealthy, HealthStatus::Unhealthy);
    }

    #[test]
    fn test_health_status_display() {
        assert_eq!(format!("{}", HealthStatus::Healthy), "healthy");
        assert_eq!(format!("{}", HealthStatus::Degraded), "degraded");
        assert_eq!(format!("{}", HealthStatus::Unhealthy), "unhealthy");
    }

    #[test]
    fn test_health_check_result_creation() {
        let result = HealthCheckResult::healthy("test");
        assert_eq!(result.name, "test");
        assert_eq!(result.status, HealthStatus::Healthy);
        assert!(result.message.is_none());
    }

    #[test]
    fn test_health_check_result_with_message() {
        let result = HealthCheckResult::degraded("test", "Something is slow");
        assert_eq!(result.status, HealthStatus::Degraded);
        assert_eq!(result.message, Some("Something is slow".to_string()));
    }

    #[test]
    fn test_health_check_result_unhealthy() {
        let result = HealthCheckResult::unhealthy("test", "Something is broken");
        assert_eq!(result.status, HealthStatus::Unhealthy);
        assert_eq!(result.message, Some("Something is broken".to_string()));
    }

    #[tokio::test]
    async fn test_health_registry() {
        let registry = HealthRegistry::new();

        assert!(registry.is_empty().await);
        assert_eq!(registry.len().await, 0);

        // Register a simple health check
        registry
            .register_simple("test_check", || HealthCheckResult::healthy("test_check"))
            .await;

        assert!(!registry.is_empty().await);
        assert_eq!(registry.len().await, 1);

        // Check all
        let results = registry.check_all().await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "test_check");
    }

    #[tokio::test]
    async fn test_health_registry_overall_status() {
        let registry = HealthRegistry::new();

        // Register a healthy check
        registry
            .register_simple("healthy", || HealthCheckResult::healthy("healthy"))
            .await;

        assert_eq!(registry.overall_status().await, HealthStatus::Healthy);

        // Add a degraded check
        registry
            .register_simple("degraded", || {
                HealthCheckResult::degraded("degraded", "slow")
            })
            .await;

        assert_eq!(registry.overall_status().await, HealthStatus::Degraded);

        // Add an unhealthy check
        registry
            .register_simple("unhealthy", || {
                HealthCheckResult::unhealthy("unhealthy", "broken")
            })
            .await;

        assert_eq!(registry.overall_status().await, HealthStatus::Unhealthy);
    }

    #[tokio::test]
    async fn test_health_monitor() {
        let monitor = HealthMonitor::new(Duration::from_secs(60));

        assert_eq!(monitor.get_status().await, HealthStatus::Healthy);

        monitor.set_status(HealthStatus::Degraded).await;
        assert_eq!(monitor.get_status().await, HealthStatus::Degraded);

        assert!(monitor.time_since_last_check().await.is_some());
        assert_eq!(monitor.check_interval(), Duration::from_secs(60));
    }

    #[tokio::test]
    async fn test_readiness_check() {
        let check = ReadinessCheck::new();

        assert!(!check.is_ready().await);

        check.set_ready().await;
        assert!(check.is_ready().await);

        check.set_not_ready().await;
        assert!(!check.is_ready().await);
    }
}
