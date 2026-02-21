//! Counter utilities for shiplog.
//! 
//! This crate provides specialized counter functionality including
//! atomic counters, increment-only counters, and counter aggregation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A simple counter that can be incremented and reset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Counter {
    name: String,
    value: u64,
    #[serde(skip)]
    start_time: i64,
}

impl Counter {
    /// Create a new counter
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: 0,
            start_time: chrono::Utc::now().timestamp(),
        }
    }

    /// Get the counter name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the current value
    pub fn value(&self) -> u64 {
        self.value
    }

    /// Increment the counter by 1
    pub fn inc(&mut self) {
        self.value += 1;
    }

    /// Increment the counter by a specific amount
    pub fn inc_by(&mut self, amount: u64) {
        self.value += amount;
    }

    /// Reset the counter to zero
    pub fn reset(&mut self) {
        self.value = 0;
        self.start_time = chrono::Utc::now().timestamp();
    }

    /// Get the time since the counter was created/reset
    pub fn elapsed(&self) -> i64 {
        chrono::Utc::now().timestamp() - self.start_time
    }
}

/// Counter that wraps around at a maximum value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WrappingCounter {
    name: String,
    value: u64,
    max: u64,
}

impl WrappingCounter {
    /// Create a new wrapping counter with the given maximum value
    pub fn new(name: impl Into<String>, max: u64) -> Self {
        Self {
            name: name.into(),
            value: 0,
            max,
        }
    }

    /// Increment the counter, wrapping if necessary
    pub fn inc(&mut self) {
        self.value = self.value.wrapping_add(1);
        if self.value > self.max {
            self.value = 0;
        }
    }

    /// Get the current value
    pub fn value(&self) -> u64 {
        self.value
    }

    /// Get the maximum value
    pub fn max(&self) -> u64 {
        self.max
    }

    /// Reset to zero
    pub fn reset(&mut self) {
        self.value = 0;
    }
}

/// Counter that can only be incremented up to a maximum value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundedCounter {
    name: String,
    value: u64,
    max: u64,
}

impl BoundedCounter {
    /// Create a new bounded counter
    pub fn new(name: impl Into<String>, max: u64) -> Self {
        Self {
            name: name.into(),
            value: 0,
            max,
        }
    }

    /// Try to increment the counter
    /// Returns true if successful, false if already at max
    pub fn try_inc(&mut self) -> bool {
        if self.value < self.max {
            self.value += 1;
            true
        } else {
            false
        }
    }

    /// Force increment, clamping at max
    pub fn inc(&mut self) {
        if self.value < self.max {
            self.value += 1;
        }
    }

    /// Get the current value
    pub fn value(&self) -> u64 {
        self.value
    }

    /// Get the maximum value
    pub fn max(&self) -> u64 {
        self.max
    }

    /// Check if at maximum
    pub fn is_maxed(&self) -> bool {
        self.value >= self.max
    }

    /// Reset to zero
    pub fn reset(&mut self) {
        self.value = 0;
    }
}

/// Delta counter - tracks the change in value since last snapshot
#[derive(Debug, Clone)]
pub struct DeltaCounter {
    name: String,
    current: u64,
    last_snapshot: u64,
}

impl DeltaCounter {
    /// Create a new delta counter
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            current: 0,
            last_snapshot: 0,
        }
    }

    /// Increment the counter
    pub fn inc(&mut self) {
        self.current += 1;
    }

    /// Increment by a specific amount
    pub fn inc_by(&mut self, amount: u64) {
        self.current += amount;
    }

    /// Get the delta since last snapshot
    pub fn delta(&self) -> u64 {
        self.current.saturating_sub(self.last_snapshot)
    }

    /// Take a snapshot and return the delta
    pub fn snapshot(&mut self) -> u64 {
        let delta = self.delta();
        self.last_snapshot = self.current;
        delta
    }

    /// Get current value
    pub fn value(&self) -> u64 {
        self.current
    }

    /// Reset both current and snapshot
    pub fn reset(&mut self) {
        self.current = 0;
        self.last_snapshot = 0;
    }
}

/// Counter registry for managing multiple counters
#[derive(Debug, Default)]
pub struct CounterRegistry {
    counters: HashMap<String, Counter>,
}

impl CounterRegistry {
    /// Create a new counter registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Get or create a counter
    pub fn get_or_create(&mut self, name: impl Into<String> + Clone) -> &mut Counter {
        let key = name.clone().into();
        self.counters.entry(key).or_insert_with(|| Counter::new(name))
    }

    /// Increment a counter
    pub fn inc(&mut self, name: &str) {
        self.get_or_create(name).inc();
    }

    /// Increment a counter by a specific amount
    pub fn inc_by(&mut self, name: &str, amount: u64) {
        self.get_or_create(name).inc_by(amount);
    }

    /// Get a counter by name
    pub fn get(&self, name: &str) -> Option<&Counter> {
        self.counters.get(name)
    }

    /// Get all counters
    pub fn get_all(&self) -> &HashMap<String, Counter> {
        &self.counters
    }

    /// Get total count across all counters
    pub fn total(&self) -> u64 {
        self.counters.values().map(|c| c.value()).sum()
    }

    /// Clear all counters
    pub fn clear(&mut self) {
        self.counters.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counter_basic() {
        let mut counter = Counter::new("requests");
        
        counter.inc();
        counter.inc();
        counter.inc_by(5);
        
        assert_eq!(counter.value(), 7);
    }

    #[test]
    fn counter_reset() {
        let mut counter = Counter::new("test");
        
        counter.inc_by(10);
        assert_eq!(counter.value(), 10);
        
        counter.reset();
        assert_eq!(counter.value(), 0);
    }

    #[test]
    fn counter_elapsed() {
        let counter = Counter::new("test");
        let elapsed = counter.elapsed();
        assert!(elapsed >= 0);
    }

    #[test]
    fn wrapping_counter() {
        let mut counter = WrappingCounter::new("wrap", 5);
        
        for _ in 0..5 {
            counter.inc();
        }
        assert_eq!(counter.value(), 5);
        
        counter.inc(); // wraps to 0
        assert_eq!(counter.value(), 0);
    }

    #[test]
    fn bounded_counter() {
        let mut counter = BoundedCounter::new("bounded", 3);
        
        assert!(counter.try_inc());
        assert!(counter.try_inc());
        assert!(counter.try_inc());
        assert!(!counter.try_inc()); // at max
        assert!(counter.is_maxed());
    }

    #[test]
    fn delta_counter() {
        let mut counter = DeltaCounter::new("delta");
        
        counter.inc_by(10);
        assert_eq!(counter.delta(), 10);
        
        let snap = counter.snapshot();
        assert_eq!(snap, 10);
        assert_eq!(counter.delta(), 0); // delta reset after snapshot
    }

    #[test]
    fn counter_registry() {
        let mut registry = CounterRegistry::new();
        
        registry.inc("requests");
        registry.inc("requests");
        registry.inc_by("requests", 5);
        registry.inc("errors");
        
        assert_eq!(registry.get("requests").unwrap().value(), 7);
        assert_eq!(registry.get("errors").unwrap().value(), 1);
    }

    #[test]
    fn counter_registry_total() {
        let mut registry = CounterRegistry::new();
        
        registry.inc_by("a", 10);
        registry.inc_by("b", 20);
        registry.inc_by("c", 30);
        
        assert_eq!(registry.total(), 60);
    }
}
