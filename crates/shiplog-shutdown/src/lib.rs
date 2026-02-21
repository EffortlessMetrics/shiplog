//! Graceful shutdown utilities for shiplog.
//!
//! This crate provides graceful shutdown utilities for the shiplog ecosystem,
//! allowing applications to gracefully shut down components in order.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::timeout;
use std::sync::atomic::{AtomicBool, Ordering};

/// Shutdown reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownReason {
    /// Normal shutdown requested
    Shutdown,
    /// Force shutdown (no graceful cleanup)
    Force,
    /// Timeout during graceful shutdown
    Timeout,
}

impl Default for ShutdownReason {
    fn default() -> Self {
        Self::Shutdown
    }
}

/// Graceful shutdown coordinator
#[derive(Clone)]
pub struct ShutdownCoordinator {
    shutdown_tx: broadcast::Sender<ShutdownReason>,
    shutdown_triggered: Arc<AtomicBool>,
}

impl ShutdownCoordinator {
    /// Creates a new shutdown coordinator
    pub fn new() -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self {
            shutdown_tx,
            shutdown_triggered: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Creates a new shutdown coordinator with capacity
    pub fn with_capacity(capacity: usize) -> Self {
        let (shutdown_tx, _) = broadcast::channel(capacity);
        Self {
            shutdown_tx,
            shutdown_triggered: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Returns a receiver for shutdown signals
    pub fn subscribe(&self) -> ShutdownReceiver {
        ShutdownReceiver {
            rx: self.shutdown_tx.subscribe(),
        }
    }

    /// Initiates a graceful shutdown
    pub fn shutdown(&self) {
        self.shutdown_triggered.store(true, Ordering::SeqCst);
        let _ = self.shutdown_tx.send(ShutdownReason::Shutdown);
    }

    /// Initiates a force shutdown
    pub fn force_shutdown(&self) {
        self.shutdown_triggered.store(true, Ordering::SeqCst);
        let _ = self.shutdown_tx.send(ShutdownReason::Force);
    }

    /// Initiates shutdown with timeout
    pub async fn shutdown_with_timeout(&self, duration: Duration) {
        self.shutdown();
        tokio::time::sleep(duration).await;
        self.force_shutdown();
    }

    /// Checks if shutdown has been initiated
    pub fn is_shutdown_initiated(&self) -> bool {
        self.shutdown_triggered.load(Ordering::SeqCst)
    }
}

impl Default for ShutdownCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

/// Receiver for shutdown signals
pub struct ShutdownReceiver {
    rx: broadcast::Receiver<ShutdownReason>,
}

impl ShutdownReceiver {
    /// Waits for shutdown signal
    pub async fn wait_for_shutdown(&mut self) -> ShutdownReason {
        if let Ok(reason) = self.rx.recv().await {
            reason
        } else {
            ShutdownReason::Shutdown
        }
    }

    /// Tries to receive a shutdown signal
    pub fn try_recv(&mut self) -> Option<ShutdownReason> {
        self.rx.try_recv().ok()
    }

    /// Waits for shutdown with timeout
    pub async fn wait_for_shutdown_timeout(&mut self, duration: Duration) -> ShutdownReason {
        timeout(duration, self.wait_for_shutdown())
            .await
            .unwrap_or(ShutdownReason::Timeout)
    }
}

/// A guard that automatically triggers shutdown when dropped
pub struct ShutdownGuard {
    coordinator: ShutdownCoordinator,
}

impl ShutdownGuard {
    /// Creates a new shutdown guard
    pub fn new(coordinator: ShutdownCoordinator) -> Self {
        Self { coordinator }
    }

    /// Triggers shutdown manually
    pub fn trigger(&self) {
        self.coordinator.shutdown();
    }
}

impl Drop for ShutdownGuard {
    fn drop(&mut self) {
        self.coordinator.shutdown();
    }
}

/// Creates a new shutdown channel
pub fn create_shutdown_channel() -> (ShutdownCoordinator, ShutdownReceiver) {
    let coordinator = ShutdownCoordinator::new();
    let receiver = coordinator.subscribe();
    (coordinator, receiver)
}

/// Applies graceful shutdown to a future with a timeout
pub async fn graceful_shutdown<F, T, E>(
    future: F,
    timeout_duration: Duration,
) -> Result<T, ShutdownReason>
where
    F: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Debug,
{
    match timeout(timeout_duration, future).await {
        Ok(result) => result.map_err(|_| ShutdownReason::Shutdown),
        Err(_) => Err(ShutdownReason::Timeout),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shutdown_coordinator_creation() {
        let coordinator = ShutdownCoordinator::new();
        assert!(!coordinator.is_shutdown_initiated());
    }

    #[test]
    fn test_shutdown_coordinator_with_capacity() {
        let coordinator = ShutdownCoordinator::with_capacity(10);
        assert!(!coordinator.is_shutdown_initiated());
    }

    #[test]
    fn test_shutdown_guard_creation() {
        let coordinator = ShutdownCoordinator::new();
        let _guard = ShutdownGuard::new(coordinator.clone());
        // Guard is created but hasn't triggered shutdown yet
        assert!(!coordinator.is_shutdown_initiated());
    }

    #[test]
    fn test_create_shutdown_channel() {
        let (coordinator, mut receiver) = create_shutdown_channel();
        assert!(!coordinator.is_shutdown_initiated());
        
        coordinator.shutdown();
        assert!(coordinator.is_shutdown_initiated());
        
        // Receiver should get the signal
        let reason = receiver.try_recv();
        assert_eq!(reason, Some(ShutdownReason::Shutdown));
    }

    #[test]
    fn test_shutdown_reason_values() {
        assert_eq!(ShutdownReason::Shutdown, ShutdownReason::Shutdown);
        assert_eq!(ShutdownReason::Force, ShutdownReason::Force);
        assert_eq!(ShutdownReason::Timeout, ShutdownReason::Timeout);
    }

    #[tokio::test]
    async fn test_shutdown_receiver_wait() {
        let (coordinator, mut receiver) = create_shutdown_channel();
        
        // Start a task that waits for shutdown
        let handle = tokio::spawn(async move {
            receiver.wait_for_shutdown().await
        });
        
        // Trigger shutdown after a small delay
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            coordinator.shutdown();
        });
        
        let reason = handle.await.unwrap();
        assert_eq!(reason, ShutdownReason::Shutdown);
    }

    #[test]
    fn test_shutdown_guard_drop() {
        let coordinator = ShutdownCoordinator::new();
        
        {
            let _guard = ShutdownGuard::new(coordinator.clone());
            // Guard drops here, triggering shutdown
        }
        
        assert!(coordinator.is_shutdown_initiated());
    }

    #[test]
    fn test_shutdown_coordinator_clone() {
        let coordinator = ShutdownCoordinator::new();
        let _clone = coordinator.clone();
        // Should work fine
    }

    #[tokio::test]
    async fn test_graceful_shutdown_success() {
        async fn do_work() -> Result<i32, &'static str> {
            Ok(42)
        }

        let result = graceful_shutdown(do_work(), Duration::from_secs(1)).await;
        assert_eq!(result, Ok(42));
    }

    #[tokio::test]
    async fn test_graceful_shutdown_timeout() {
        async fn do_work() -> Result<i32, &'static str> {
            tokio::time::sleep(Duration::from_secs(10)).await;
            Ok(42)
        }

        let result = graceful_shutdown(do_work(), Duration::from_millis(50)).await;
        assert_eq!(result, Err(ShutdownReason::Timeout));
    }
}
