//! Metering utilities for shiplog.
//! 
//! This crate provides utilities for measuring rates, throughput, and timing.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Meter data for tracking rates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeterData {
    pub name: String,
    pub count: u64,
    pub start_time: i64,
    pub last_update: i64,
}

impl MeterData {
    /// Create a new meter
    pub fn new(name: impl Into<String>) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            name: name.into(),
            count: 0,
            start_time: now,
            last_update: now,
        }
    }

    /// Record an event
    pub fn record(&mut self) {
        self.count += 1;
        self.last_update = chrono::Utc::now().timestamp();
    }

    /// Record multiple events
    pub fn record_many(&mut self, count: u64) {
        self.count += count;
        self.last_update = chrono::Utc::now().timestamp();
    }

    /// Get the rate per second
    pub fn rate_per_second(&self) -> f64 {
        let elapsed = self.last_update - self.start_time;
        if elapsed > 0 {
            self.count as f64 / elapsed as f64
        } else {
            0.0
        }
    }

    /// Get the rate per minute
    pub fn rate_per_minute(&self) -> f64 {
        self.rate_per_second() * 60.0
    }

    /// Get the rate per hour
    pub fn rate_per_hour(&self) -> f64 {
        self.rate_per_minute() * 60.0
    }

    /// Reset the meter
    pub fn reset(&mut self) {
        let now = chrono::Utc::now().timestamp();
        self.count = 0;
        self.start_time = now;
        self.last_update = now;
    }
}

/// Meter registry for managing multiple meters
#[derive(Debug, Default)]
pub struct MeterRegistry {
    meters: HashMap<String, MeterData>,
}

impl MeterRegistry {
    /// Create a new meter registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Get or create a meter
    pub fn get_or_create(&mut self, name: impl Into<String> + Clone) -> &mut MeterData {
        let key = name.clone().into();
        self.meters.entry(key).or_insert_with(|| MeterData::new(name))
    }

    /// Record an event on a meter
    pub fn record(&mut self, name: &str) {
        self.get_or_create(name).record();
    }

    /// Record multiple events on a meter
    pub fn record_many(&mut self, name: &str, count: u64) {
        self.get_or_create(name).record_many(count);
    }

    /// Get a meter by name
    pub fn get(&self, name: &str) -> Option<&MeterData> {
        self.meters.get(name)
    }

    /// Get all meters
    pub fn get_all(&self) -> &HashMap<String, MeterData> {
        &self.meters
    }

    /// Get all rates as a map
    pub fn get_rates(&self) -> HashMap<String, f64> {
        self.meters
            .iter()
            .map(|(name, meter)| (name.clone(), meter.rate_per_second()))
            .collect()
    }

    /// Clear all meters
    pub fn clear(&mut self) {
        self.meters.clear();
    }
}

/// Timing context for measuring duration
pub struct TimingContext {
    start: Instant,
    name: String,
}

impl TimingContext {
    /// Create a new timing context
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            start: Instant::now(),
            name: name.into(),
        }
    }

    /// Get the elapsed duration
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    /// Get elapsed in milliseconds
    pub fn elapsed_millis(&self) -> u128 {
        self.start.elapsed().as_millis()
    }

    /// Get elapsed in microseconds
    pub fn elapsed_micros(&self) -> u128 {
        self.start.elapsed().as_micros()
    }

    /// Get elapsed in seconds
    pub fn elapsed_secs(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }
}

impl Drop for TimingContext {
    fn drop(&mut self) {
        // Timing context dropped - could emit to a collector in future
    }
}

/// Timing recorder for collecting timing data
#[derive(Debug, Default)]
pub struct TimingRecorder {
    timings: HashMap<String, Vec<f64>>,
}

impl TimingRecorder {
    /// Create a new timing recorder
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a timing
    pub fn record(&mut self, name: impl Into<String>, duration_secs: f64) {
        self.timings
            .entry(name.into())
            .or_insert_with(Vec::new)
            .push(duration_secs);
    }

    /// Get timing statistics
    pub fn stats(&self, name: &str) -> Option<TimingStats> {
        self.timings.get(name).map(|values| {
            let count = values.len() as u64;
            let sum: f64 = values.iter().sum();
            let mean = if count > 0 { sum / count as f64 } else { 0.0 };

            let mut sorted = values.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

            let min = sorted.first().copied();
            let max = sorted.last().copied();
            let p50 = if count > 0 {
                sorted[(count as usize) / 2]
            } else {
                0.0
            };
            let p95 = if count > 0 {
                sorted[(count as usize * 95) / 100]
            } else {
                0.0
            };
            let p99 = if count > 0 {
                sorted[(count as usize * 99) / 100]
            } else {
                0.0
            };

            TimingStats {
                count,
                sum,
                mean,
                min,
                max,
                p50,
                p95,
                p99,
            }
        })
    }

    /// Get all timing names
    pub fn names(&self) -> Vec<&String> {
        self.timings.keys().collect()
    }

    /// Clear all timings
    pub fn clear(&mut self) {
        self.timings.clear();
    }
}

/// Timing statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingStats {
    pub count: u64,
    pub sum: f64,
    pub mean: f64,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meter_data() {
        let mut meter = MeterData::new("test");
        
        meter.record();
        meter.record();
        meter.record_many(5);
        
        assert_eq!(meter.count, 7);
    }

    #[test]
    fn meter_registry() {
        let mut registry = MeterRegistry::new();
        
        registry.record("api_calls");
        registry.record("api_calls");
        registry.record_many("api_calls", 5);
        
        let meter = registry.get("api_calls").unwrap();
        assert_eq!(meter.count, 7);
    }

    #[test]
    fn timing_context() {
        let _ctx = TimingContext::new("operation");
        // Just test it compiles
    }

    #[test]
    fn timing_recorder() {
        let mut recorder = TimingRecorder::new();
        
        recorder.record("query", 0.1);
        recorder.record("query", 0.2);
        recorder.record("query", 0.3);
        
        let stats = recorder.stats("query").unwrap();
        
        assert_eq!(stats.count, 3);
        assert!((stats.sum - 0.6).abs() < 0.001);
    }

    #[test]
    fn timing_stats_percentiles() {
        let mut recorder = TimingRecorder::new();
        
        for i in 1..=100 {
            recorder.record("latency", i as f64);
        }
        
        let stats = recorder.stats("latency").unwrap();
        
        assert_eq!(stats.count, 100);
        assert!(stats.p50 > 0.0);
        assert!(stats.p95 > stats.p50);
        assert!(stats.p99 >= stats.p95);
    }

    #[test]
    fn timing_recorder_names() {
        let mut recorder = TimingRecorder::new();
        
        recorder.record("op1", 0.1);
        recorder.record("op2", 0.2);
        
        let names = recorder.names();
        assert_eq!(names.len(), 2);
    }
}
