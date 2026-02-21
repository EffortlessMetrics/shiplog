//! Gauge utilities for shiplog.
//!
//! This crate provides specialized gauge functionality for tracking
//! point-in-time values like temperature, memory usage, queue depth, etc.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A gauge represents a point-in-time value that can go up or down
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gauge {
    name: String,
    value: f64,
    min: f64,
    max: f64,
    #[serde(skip)]
    start_time: i64,
}

impl Gauge {
    /// Create a new gauge with default bounds
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: 0.0,
            min: f64::MAX,
            max: f64::MIN,
            start_time: chrono::Utc::now().timestamp(),
        }
    }

    /// Create a new gauge with custom bounds
    pub fn with_bounds(name: impl Into<String>, min: f64, max: f64) -> Self {
        Self {
            name: name.into(),
            value: 0.0,
            min,
            max,
            start_time: chrono::Utc::now().timestamp(),
        }
    }

    /// Get the gauge name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the current value
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Set the gauge value
    pub fn set(&mut self, value: f64) {
        self.value = value;
        if value < self.min {
            self.min = value;
        }
        if value > self.max {
            self.max = value;
        }
    }

    /// Increment the gauge value
    pub fn inc(&mut self, amount: f64) {
        self.set(self.value + amount);
    }

    /// Decrement the gauge value
    pub fn dec(&mut self, amount: f64) {
        self.set(self.value - amount);
    }

    /// Get the minimum value seen
    pub fn min(&self) -> f64 {
        if self.min == f64::MAX {
            self.value
        } else {
            self.min
        }
    }

    /// Get the maximum value seen
    pub fn max(&self) -> f64 {
        if self.max == f64::MIN {
            self.value
        } else {
            self.max
        }
    }

    /// Reset the gauge
    pub fn reset(&mut self) {
        self.value = 0.0;
        self.min = f64::MAX;
        self.max = f64::MIN;
        self.start_time = chrono::Utc::now().timestamp();
    }
}

/// Gauge that clamps values within a range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClampedGauge {
    name: String,
    value: f64,
    min: f64,
    max: f64,
}

impl ClampedGauge {
    /// Create a new clamped gauge
    pub fn new(name: impl Into<String>, min: f64, max: f64) -> Self {
        Self {
            name: name.into(),
            value: 0.0,
            min,
            max,
        }
    }

    /// Set the value, clamping to bounds
    pub fn set(&mut self, value: f64) {
        self.value = value.clamp(self.min, self.max);
    }

    /// Get the current value
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Get the minimum bound
    pub fn min(&self) -> f64 {
        self.min
    }

    /// Get the maximum bound
    pub fn max(&self) -> f64 {
        self.max
    }

    /// Reset to zero
    pub fn reset(&mut self) {
        self.value = 0.0;
    }
}

/// Gauge that tracks the average of all values set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AverageGauge {
    name: String,
    values: Vec<f64>,
}

impl AverageGauge {
    /// Create a new average gauge
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            values: Vec::new(),
        }
    }

    /// Record a value
    pub fn record(&mut self, value: f64) {
        self.values.push(value);
    }

    /// Get the current average
    pub fn average(&self) -> f64 {
        if self.values.is_empty() {
            0.0
        } else {
            self.values.iter().sum::<f64>() / self.values.len() as f64
        }
    }

    /// Get the current value (most recent)
    pub fn value(&self) -> f64 {
        self.values.last().copied().unwrap_or(0.0)
    }

    /// Get the count of recorded values
    pub fn count(&self) -> usize {
        self.values.len()
    }

    /// Clear all recorded values
    pub fn clear(&mut self) {
        self.values.clear();
    }
}

/// Gauge registry for managing multiple gauges
#[derive(Debug, Default)]
pub struct GaugeRegistry {
    gauges: HashMap<String, Gauge>,
}

impl GaugeRegistry {
    /// Create a new gauge registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Get or create a gauge
    pub fn get_or_create(&mut self, name: impl Into<String> + Clone) -> &mut Gauge {
        let key = name.clone().into();
        self.gauges.entry(key).or_insert_with(|| Gauge::new(name))
    }

    /// Set a gauge value
    pub fn set(&mut self, name: &str, value: f64) {
        self.get_or_create(name).set(value);
    }

    /// Increment a gauge
    pub fn inc(&mut self, name: &str, amount: f64) {
        self.get_or_create(name).inc(amount);
    }

    /// Decrement a gauge
    pub fn dec(&mut self, name: &str, amount: f64) {
        self.get_or_create(name).dec(amount);
    }

    /// Get a gauge by name
    pub fn get(&self, name: &str) -> Option<&Gauge> {
        self.gauges.get(name)
    }

    /// Get all gauges
    pub fn get_all(&self) -> &HashMap<String, Gauge> {
        &self.gauges
    }

    /// Get all current values
    pub fn values(&self) -> HashMap<String, f64> {
        self.gauges
            .iter()
            .map(|(name, gauge)| (name.clone(), gauge.value()))
            .collect()
    }

    /// Clear all gauges
    pub fn clear(&mut self) {
        self.gauges.clear();
    }
}

/// Gauge snapshot for reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GaugeSnapshot {
    pub name: String,
    pub value: f64,
    pub min: f64,
    pub max: f64,
    pub timestamp: i64,
}

impl Gauge {
    /// Create a snapshot of the current gauge state
    pub fn snapshot(&self) -> GaugeSnapshot {
        GaugeSnapshot {
            name: self.name.clone(),
            value: self.value,
            min: self.min(),
            max: self.max(),
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gauge_basic() {
        let mut gauge = Gauge::new("temperature");

        gauge.set(72.5);
        assert_eq!(gauge.value(), 72.5);

        gauge.set(73.0);
        assert_eq!(gauge.value(), 73.0);
    }

    #[test]
    fn gauge_min_max() {
        let mut gauge = Gauge::new("temperature");

        gauge.set(50.0);
        gauge.set(100.0);
        gauge.set(75.0);

        assert_eq!(gauge.min(), 50.0);
        assert_eq!(gauge.max(), 100.0);
    }

    #[test]
    fn gauge_inc_dec() {
        let mut gauge = Gauge::new("value");

        gauge.set(10.0);
        gauge.inc(5.0);
        assert_eq!(gauge.value(), 15.0);

        gauge.dec(3.0);
        assert_eq!(gauge.value(), 12.0);
    }

    #[test]
    fn gauge_reset() {
        let mut gauge = Gauge::new("test");

        gauge.set(100.0);
        gauge.reset();

        assert_eq!(gauge.value(), 0.0);
    }

    #[test]
    fn clamped_gauge() {
        let mut gauge = ClampedGauge::new("percent", 0.0, 100.0);

        gauge.set(50.0);
        assert_eq!(gauge.value(), 50.0);

        gauge.set(150.0); // clamped
        assert_eq!(gauge.value(), 100.0);

        gauge.set(-50.0); // clamped
        assert_eq!(gauge.value(), 0.0);
    }

    #[test]
    fn average_gauge() {
        let mut gauge = AverageGauge::new("readings");

        gauge.record(10.0);
        gauge.record(20.0);
        gauge.record(30.0);

        assert_eq!(gauge.average(), 20.0);
        assert_eq!(gauge.value(), 30.0);
        assert_eq!(gauge.count(), 3);
    }

    #[test]
    fn gauge_registry() {
        let mut registry = GaugeRegistry::new();

        registry.set("temperature", 72.0);
        registry.set("humidity", 65.0);

        assert_eq!(registry.get("temperature").unwrap().value(), 72.0);
        assert_eq!(registry.get("humidity").unwrap().value(), 65.0);
    }

    #[test]
    fn gauge_registry_inc_dec() {
        let mut registry = GaugeRegistry::new();

        registry.set("queue_depth", 10.0);
        registry.inc("queue_depth", 5.0);
        assert_eq!(registry.get("queue_depth").unwrap().value(), 15.0);

        registry.dec("queue_depth", 3.0);
        assert_eq!(registry.get("queue_depth").unwrap().value(), 12.0);
    }

    #[test]
    fn gauge_registry_values() {
        let mut registry = GaugeRegistry::new();

        registry.set("a", 1.0);
        registry.set("b", 2.0);

        let values = registry.values();
        assert_eq!(values.get("a"), Some(&1.0));
        assert_eq!(values.get("b"), Some(&2.0));
    }

    #[test]
    fn gauge_snapshot() {
        let mut gauge = Gauge::new("test");

        gauge.set(50.0);
        gauge.set(100.0);

        let snap = gauge.snapshot();

        assert_eq!(snap.name, "test");
        assert_eq!(snap.value, 100.0);
        assert_eq!(snap.min, 50.0);
        assert_eq!(snap.max, 100.0);
    }
}
