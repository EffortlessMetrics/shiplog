//! RAII guard utilities for shiplog.
//!
//! This crate provides RAII guard utilities for automatic resource management.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// A guard that calls a cleanup function when dropped
pub struct Guard<F: FnOnce()> {
    cleanup: Option<F>,
}

impl<F: FnOnce()> Guard<F> {
    /// Create a new guard with a cleanup function
    pub fn new(cleanup: F) -> Self {
        Self {
            cleanup: Some(cleanup),
        }
    }

    /// Consume the guard without running cleanup
    pub fn disarm(mut self) {
        self.cleanup = None;
    }
}

impl<F: FnOnce()> Drop for Guard<F> {
    fn drop(&mut self) {
        if let Some(cleanup) = self.cleanup.take() {
            cleanup();
        }
    }
}

/// A boolean guard that ensures cleanup runs exactly once
pub struct OnceGuard {
    done: AtomicBool,
}

impl OnceGuard {
    /// Create a new once guard
    pub fn new() -> Self {
        Self {
            done: AtomicBool::new(false),
        }
    }

    /// Check if the guard has been triggered
    pub fn is_done(&self) -> bool {
        self.done.load(Ordering::SeqCst)
    }

    /// Try to trigger the guard, returns true if successful
    pub fn try_trigger(&self) -> bool {
        !self.done.swap(true, Ordering::SeqCst)
    }
}

impl Default for OnceGuard {
    fn default() -> Self {
        Self::new()
    }
}

/// A scope guard that manages a counter
pub struct CounterGuard {
    counter: &'static AtomicUsize,
}

impl CounterGuard {
    /// Create a new counter guard that increments the counter
    pub fn new(counter: &'static AtomicUsize) -> Self {
        counter.fetch_add(1, Ordering::SeqCst);
        Self { counter }
    }
}

impl Drop for CounterGuard {
    fn drop(&mut self) {
        let current = self.counter.load(Ordering::SeqCst);
        if current > 0 {
            self.counter.fetch_sub(1, Ordering::SeqCst);
        }
    }
}

/// A mutex guard wrapper that provides additional functionality
pub struct MutexGuard<'a, T> {
    guard: MutexGuardInner<'a, T>,
}

enum MutexGuardInner<'a, T> {
    Some(std::sync::MutexGuard<'a, T>),
    None,
}

impl<'a, T> MutexGuard<'a, T> {
    /// Create a new mutex guard
    pub fn new(guard: std::sync::MutexGuard<'a, T>) -> Self {
        Self {
            guard: MutexGuardInner::Some(guard),
        }
    }

    /// Get a reference to the guarded data
    pub fn get(&self) -> Option<&T> {
        match &self.guard {
            MutexGuardInner::Some(g) => Some(g.deref()),
            MutexGuardInner::None => None,
        }
    }

    /// Release the guard early
    pub fn release(mut self) {
        self.guard = MutexGuardInner::None;
    }
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        // MutexGuard is automatically dropped here
    }
}

use std::ops::Deref;

/// Configuration for guards
#[derive(Debug, Clone, Default)]
pub struct GuardConfig {
    pub panic_on_drop: bool,
    pub log_on_drop: bool,
}

/// Builder for guard configuration
#[derive(Debug)]
pub struct GuardBuilder {
    config: GuardConfig,
}

impl GuardBuilder {
    pub fn new() -> Self {
        Self {
            config: GuardConfig::default(),
        }
    }

    pub fn panic_on_drop(mut self, panic: bool) -> Self {
        self.config.panic_on_drop = panic;
        self
    }

    pub fn log_on_drop(mut self, log: bool) -> Self {
        self.config.log_on_drop = log;
        self
    }

    pub fn build(self) -> GuardConfig {
        self.config
    }
}

impl Default for GuardBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guard_basic() {
        static DROPPED: AtomicBool = AtomicBool::new(false);

        {
            let _guard = Guard::new(|| {
                DROPPED.store(true, Ordering::SeqCst);
            });
            assert!(!DROPPED.load(Ordering::SeqCst));
        }

        assert!(DROPPED.load(Ordering::SeqCst));
    }

    #[test]
    fn test_guard_disarm() {
        static DROPPED: AtomicBool = AtomicBool::new(false);

        {
            let guard = Guard::new(|| {
                DROPPED.store(true, Ordering::SeqCst);
            });
            guard.disarm();
            assert!(!DROPPED.load(Ordering::SeqCst));
        }

        // Should not be called after disarm
        assert!(!DROPPED.load(Ordering::SeqCst));
    }

    #[test]
    fn test_once_guard() {
        let guard = OnceGuard::new();

        assert!(!guard.is_done());
        assert!(guard.try_trigger());
        assert!(guard.is_done());

        // Second try should fail
        assert!(!guard.try_trigger());
    }

    #[test]
    fn test_counter_guard() {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);

        assert_eq!(COUNTER.load(Ordering::SeqCst), 0);

        {
            let _guard = CounterGuard::new(&COUNTER);
            assert_eq!(COUNTER.load(Ordering::SeqCst), 1);

            {
                let _guard2 = CounterGuard::new(&COUNTER);
                assert_eq!(COUNTER.load(Ordering::SeqCst), 2);
            }

            assert_eq!(COUNTER.load(Ordering::SeqCst), 1);
        }

        assert_eq!(COUNTER.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_guard_config_default() {
        let config = GuardConfig::default();
        assert!(!config.panic_on_drop);
        assert!(!config.log_on_drop);
    }

    #[test]
    fn test_guard_builder() {
        let config = GuardBuilder::new()
            .panic_on_drop(true)
            .log_on_drop(true)
            .build();

        assert!(config.panic_on_drop);
        assert!(config.log_on_drop);
    }
}
