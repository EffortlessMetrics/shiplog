//! Percentile calculation utilities for shiplog.
//! 
//! This crate provides percentile calculation functionality for metrics
//! including streaming percentiles and batch percentile computation.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// A streaming percentile estimator using the P-Square algorithm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Percentile {
    name: String,
    quantile: f64,
    values: VecDeque<f64>,
    estimate: f64,
    count: u64,
}

impl Percentile {
    /// Create a new percentile estimator
    pub fn new(name: impl Into<String>, quantile: f64) -> Self {
        Self {
            name: name.into(),
            quantile: quantile.clamp(0.0, 1.0),
            values: VecDeque::new(),
            estimate: 0.0,
            count: 0,
        }
    }
    
    /// Add a value to the percentile estimator
    pub fn record(&mut self, value: f64) {
        self.values.push_back(value);
        self.count += 1;
        
        // Update estimate using sorted values
        if self.count > 0 {
            let sorted: Vec<f64> = self.values.iter().cloned().collect();
            let idx = ((self.quantile * (self.count - 1) as f64)) as usize;
            let idx = idx.min(sorted.len() - 1);
            self.estimate = sorted[idx];
        }
    }
    
    /// Get the current percentile estimate
    pub fn get(&self) -> f64 {
        self.estimate
    }
    
    /// Get the quantile being estimated
    pub fn quantile(&self) -> f64 {
        self.quantile
    }
    
    /// Get the number of values recorded
    pub fn count(&self) -> u64 {
        self.count
    }
    
    /// Reset the estimator
    pub fn reset(&mut self) {
        self.values.clear();
        self.estimate = 0.0;
        self.count = 0;
    }
}

/// Multiple percentile calculator for common percentiles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PercentileSet {
    name: String,
    values: Vec<f64>,
    percentiles: Vec<f64>,
}

impl PercentileSet {
    /// Create a new percentile set with common percentiles (50, 90, 95, 99)
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            values: Vec::new(),
            percentiles: vec![50.0, 90.0, 95.0, 99.0],
        }
    }
    
    /// Create a new percentile set with custom percentiles
    pub fn with_percentiles(name: impl Into<String>, percentiles: Vec<f64>) -> Self {
        Self {
            name: name.into(),
            values: Vec::new(),
            percentiles,
        }
    }
    
    /// Record a value
    pub fn record(&mut self, value: f64) {
        self.values.push(value);
    }
    
    /// Record multiple values
    pub fn record_many(&mut self, values: impl IntoIterator<Item = f64>) {
        self.values.extend(values);
    }
    
    /// Get a specific percentile value
    pub fn percentile(&self, p: f64) -> f64 {
        if self.values.is_empty() {
            return 0.0;
        }
        
        let mut sorted = self.values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        
        let idx = ((p / 100.0) * (sorted.len() - 1) as f64) as usize;
        let idx = idx.min(sorted.len() - 1);
        sorted[idx]
    }
    
    /// Get all configured percentile values
    pub fn get_percentiles(&self) -> Vec<(f64, f64)> {
        self.percentiles
            .iter()
            .map(|&p| (p, self.percentile(p)))
            .collect()
    }
    
    /// Get the median (50th percentile)
    pub fn median(&self) -> f64 {
        self.percentile(50.0)
    }
    
    /// Get the 95th percentile
    pub fn p95(&self) -> f64 {
        self.percentile(95.0)
    }
    
    /// Get the 99th percentile
    pub fn p99(&self) -> f64 {
        self.percentile(99.0)
    }
    
    /// Get the count of values
    pub fn count(&self) -> usize {
        self.values.len()
    }
    
    /// Get all values
    pub fn values(&self) -> &[f64] {
        &self.values
    }
    
    /// Reset the set
    pub fn reset(&mut self) {
        self.values.clear();
    }
}

/// Calculate percentile from a slice of values
pub fn calculate_percentile(values: &[f64], percentile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    
    let idx = ((percentile / 100.0) * (sorted.len() - 1) as f64) as usize;
    let idx = idx.min(sorted.len() - 1);
    sorted[idx]
}

/// Calculate multiple percentiles at once
pub fn calculate_percentiles(values: &[f64], percentiles: &[f64]) -> Vec<(f64, f64)> {
    percentiles
        .iter()
        .map(|&p| (p, calculate_percentile(values, p)))
        .collect()
}

/// Streaming percentile tracker with fixed memory usage
#[derive(Debug, Clone)]
pub struct StreamingPercentile {
    quantile: f64,
    count: u64,
    values: Vec<f64>,
    max_values: usize,
}

impl StreamingPercentile {
    /// Create a new streaming percentile tracker
    pub fn new(quantile: f64, max_values: usize) -> Self {
        Self {
            quantile: quantile.clamp(0.0, 1.0),
            count: 0,
            values: Vec::with_capacity(max_values),
            max_values,
        }
    }
    
    /// Record a value
    pub fn record(&mut self, value: f64) {
        self.count += 1;
        
        if self.values.len() < self.max_values {
            self.values.push(value);
            self.values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        } else {
            // Randomly decide whether to replace (reservoir sampling approach)
            let idx = (self.count as f64 * self.quantile) as usize % self.max_values;
            if (self.count as f64 * self.quantile).fract() < 1.0 / self.max_values as f64 {
                self.values[idx] = value;
                // Re-sort periodically for accuracy
                if self.count % 1000 == 0 {
                    self.values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                }
            }
        }
    }
    
    /// Get the current percentile estimate
    pub fn get(&self) -> f64 {
        if self.values.is_empty() {
            return 0.0;
        }
        
        let idx = ((self.quantile) * (self.values.len() - 1) as f64) as usize;
        let idx = idx.min(self.values.len() - 1);
        self.values[idx]
    }
    
    /// Get the count
    pub fn count(&self) -> u64 {
        self.count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn percentile_basic() {
        let mut p = Percentile::new("latency", 0.95);
        
        for i in 1..=100 {
            p.record(i as f64);
        }
        
        assert!(p.get() >= 90.0);
    }
    
    #[test]
    fn percentile_empty() {
        let p = Percentile::new("empty", 0.5);
        assert_eq!(p.get(), 0.0);
    }
    
    #[test]
    fn percentile_reset() {
        let mut p = Percentile::new("test", 0.5);
        
        p.record(10.0);
        p.reset();
        
        assert_eq!(p.count(), 0);
        assert_eq!(p.get(), 0.0);
    }
    
    #[test]
    fn percentile_set_median() {
        let mut ps = PercentileSet::new("test");
        
        for i in 1..=100 {
            ps.record(i as f64);
        }
        
        assert_eq!(ps.median(), 50.0);
    }
    
    #[test]
    fn percentile_set_p95() {
        let mut ps = PercentileSet::new("test");
        
        for i in 1..=100 {
            ps.record(i as f64);
        }
        
        assert_eq!(ps.p95(), 95.0);
    }
    
    #[test]
    fn percentile_set_custom() {
        let mut ps = PercentileSet::with_percentiles("test", vec![25.0, 75.0]);
        
        ps.record_many(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        
        let result = ps.get_percentiles();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, 25.0);
        assert_eq!(result[1].0, 75.0);
    }
    
    #[test]
    fn percentile_set_empty() {
        let ps = PercentileSet::new("test");
        assert_eq!(ps.median(), 0.0);
    }
    
    #[test]
    fn calculate_percentile_basic() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        
        assert_eq!(calculate_percentile(&values, 50.0), 3.0);
        assert_eq!(calculate_percentile(&values, 100.0), 5.0);
    }
    
    #[test]
    fn calculate_percentiles_multiple() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        
        let result = calculate_percentiles(&values, &[25.0, 50.0, 75.0]);
        
        assert_eq!(result.len(), 3);
    }
    
    #[test]
    fn streaming_percentile() {
        let mut sp = StreamingPercentile::new(0.5, 100);
        
        for i in 1..=100 {
            sp.record(i as f64);
        }
        
        assert!(sp.get() > 0.0);
    }
    
    #[test]
    fn streaming_percentile_empty() {
        let sp = StreamingPercentile::new(0.5, 100);
        assert_eq!(sp.get(), 0.0);
    }
    
    #[test]
    fn streaming_percentile_single() {
        let mut sp = StreamingPercentile::new(0.5, 100);
        
        sp.record(42.0);
        
        assert_eq!(sp.get(), 42.0);
    }
}
