//! Summary statistics utilities for shiplog.
//! 
//! This crate provides summary statistics functionality including
//! descriptive statistics, running statistics, and statistical moments.

use serde::{Deserialize, Serialize};

/// A summary of statistical values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    count: u64,
    sum: f64,
    min: f64,
    max: f64,
    mean: f64,
    variance: f64,
    std_dev: f64,
}

impl Summary {
    /// Create a new empty summary
    pub fn new() -> Self {
        Self {
            count: 0,
            sum: 0.0,
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
            mean: 0.0,
            variance: 0.0,
            std_dev: 0.0,
        }
    }
    
    /// Add a value to the summary
    pub fn add(&mut self, value: f64) {
        let n = self.count as f64;
        self.count += 1;
        
        // Update min/max
        if value < self.min {
            self.min = value;
        }
        if value > self.max {
            self.max = value;
        }
        
        // Update sum
        self.sum += value;
        
        // Update mean using Welford's algorithm
        let new_n = self.count as f64;
        let delta = value - self.mean;
        self.mean += delta / new_n;
        
        // Update variance using Welford's algorithm
        let delta2 = value - self.mean;
        self.variance = (n * self.variance + delta * delta2) / new_n;
        
        // Calculate standard deviation
        self.std_dev = self.variance.sqrt();
    }
    
    /// Get the count
    pub fn count(&self) -> u64 {
        self.count
    }
    
    /// Get the sum
    pub fn sum(&self) -> f64 {
        self.sum
    }
    
    /// Get the minimum
    pub fn min(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.min
        }
    }
    
    /// Get the maximum
    pub fn max(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.max
        }
    }
    
    /// Get the mean
    pub fn mean(&self) -> f64 {
        self.mean
    }
    
    /// Get the variance
    pub fn variance(&self) -> f64 {
        self.variance
    }
    
    /// Get the standard deviation
    pub fn std_dev(&self) -> f64 {
        self.std_dev
    }
    
    /// Reset the summary
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

impl Default for Summary {
    fn default() -> Self {
        Self::new()
    }
}

/// Running statistics calculator
#[derive(Debug, Clone)]
pub struct RunningStats {
    count: u64,
    mean: f64,
    m2: f64, // Sum of squares of differences from the mean
    min: f64,
    max: f64,
}

impl RunningStats {
    /// Create new running stats
    pub fn new() -> Self {
        Self {
            count: 0,
            mean: 0.0,
            m2: 0.0,
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
        }
    }
    
    /// Push a value
    pub fn push(&mut self, value: f64) {
        self.count += 1;
        
        // Update min/max
        if value < self.min {
            self.min = value;
        }
        if value > self.max {
            self.max = value;
        }
        
        // Update running statistics
        let delta = value - self.mean;
        self.mean += delta / self.count as f64;
        let delta2 = value - self.mean;
        self.m2 += delta * delta2;
    }
    
    /// Get the count
    pub fn count(&self) -> u64 {
        self.count
    }
    
    /// Get the mean
    pub fn mean(&self) -> f64 {
        self.mean
    }
    
    /// Get the variance
    pub fn variance(&self) -> f64 {
        if self.count < 2 {
            0.0
        } else {
            self.m2 / (self.count - 1) as f64
        }
    }
    
    /// Get the standard deviation
    pub fn std_dev(&self) -> f64 {
        self.variance().sqrt()
    }
    
    /// Get the min
    pub fn min(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.min
        }
    }
    
    /// Get the max
    pub fn max(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.max
        }
    }
}

impl Default for RunningStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Descriptive statistics calculator
#[derive(Debug, Clone, Default)]
pub struct DescriptiveStats {
    values: Vec<f64>,
}

impl DescriptiveStats {
    /// Create new descriptive stats
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }
    
    /// Add a value
    pub fn add(&mut self, value: f64) {
        self.values.push(value);
    }
    
    /// Add multiple values
    pub fn extend(&mut self, values: impl IntoIterator<Item = f64>) {
        self.values.extend(values);
    }
    
    /// Get the count
    pub fn count(&self) -> usize {
        self.values.len()
    }
    
    /// Get the sum
    pub fn sum(&self) -> f64 {
        self.values.iter().sum()
    }
    
    /// Get the mean
    pub fn mean(&self) -> f64 {
        if self.values.is_empty() {
            0.0
        } else {
            self.sum() / self.values.len() as f64
        }
    }
    
    /// Get the median
    pub fn median(&self) -> f64 {
        if self.values.is_empty() {
            return 0.0;
        }
        
        let mut sorted = self.values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        
        let mid = sorted.len() / 2;
        if sorted.len() % 2 == 0 {
            (sorted[mid - 1] + sorted[mid]) / 2.0
        } else {
            sorted[mid]
        }
    }
    
    /// Get the min
    pub fn min(&self) -> f64 {
        self.values.iter().cloned().fold(f64::INFINITY, f64::min)
    }
    
    /// Get the max
    pub fn max(&self) -> f64 {
        self.values.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
    }
    
    /// Get the variance (population)
    pub fn variance(&self) -> f64 {
        if self.values.is_empty() {
            return 0.0;
        }
        
        let mean = self.mean();
        let sum_sq: f64 = self.values.iter().map(|v| (v - mean).powi(2)).sum();
        sum_sq / self.values.len() as f64
    }
    
    /// Get the variance (sample)
    pub fn sample_variance(&self) -> f64 {
        if self.values.len() < 2 {
            return 0.0;
        }
        
        let mean = self.mean();
        let sum_sq: f64 = self.values.iter().map(|v| (v - mean).powi(2)).sum();
        sum_sq / (self.values.len() - 1) as f64
    }
    
    /// Get the standard deviation (population)
    pub fn std_dev(&self) -> f64 {
        self.variance().sqrt()
    }
    
    /// Get the standard deviation (sample)
    pub fn sample_std_dev(&self) -> f64 {
        self.sample_variance().sqrt()
    }
    
    /// Get all values
    pub fn values(&self) -> &[f64] {
        &self.values
    }
}

/// Calculate mean of values
pub fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

/// Calculate variance of values
pub fn variance(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    
    let m = mean(values);
    values.iter().map(|v| (v - m).powi(2)).sum::<f64>() / values.len() as f64
}

/// Calculate standard deviation of values
pub fn std_dev(values: &[f64]) -> f64 {
    variance(values).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn summary_basic() {
        let mut s = Summary::new();
        
        s.add(1.0);
        s.add(2.0);
        s.add(3.0);
        
        assert_eq!(s.count(), 3);
        assert_eq!(s.sum(), 6.0);
        assert_eq!(s.mean(), 2.0);
        assert_eq!(s.min(), 1.0);
        assert_eq!(s.max(), 3.0);
    }
    
    #[test]
    fn summary_empty() {
        let s = Summary::new();
        
        assert_eq!(s.count(), 0);
        assert_eq!(s.sum(), 0.0);
        assert_eq!(s.mean(), 0.0);
    }
    
    #[test]
    fn summary_single() {
        let mut s = Summary::new();
        s.add(42.0);
        
        assert_eq!(s.mean(), 42.0);
        assert_eq!(s.min(), 42.0);
        assert_eq!(s.max(), 42.0);
    }
    
    #[test]
    fn summary_reset() {
        let mut s = Summary::new();
        
        s.add(10.0);
        s.reset();
        
        assert_eq!(s.count(), 0);
    }
    
    #[test]
    fn running_stats() {
        let mut rs = RunningStats::new();
        
        rs.push(1.0);
        rs.push(2.0);
        rs.push(3.0);
        rs.push(4.0);
        rs.push(5.0);
        
        assert_eq!(rs.count(), 5);
        assert_eq!(rs.mean(), 3.0);
        assert_eq!(rs.min(), 1.0);
        assert_eq!(rs.max(), 5.0);
    }
    
    #[test]
    fn running_stats_empty() {
        let rs = RunningStats::new();
        
        assert_eq!(rs.mean(), 0.0);
        assert_eq!(rs.variance(), 0.0);
    }
    
    #[test]
    fn running_stats_single() {
        let mut rs = RunningStats::new();
        rs.push(42.0);
        
        assert_eq!(rs.mean(), 42.0);
    }
    
    #[test]
    fn descriptive_stats() {
        let mut ds = DescriptiveStats::new();
        
        ds.add(1.0);
        ds.add(2.0);
        ds.add(3.0);
        ds.add(4.0);
        ds.add(5.0);
        
        assert_eq!(ds.count(), 5);
        assert_eq!(ds.mean(), 3.0);
        assert_eq!(ds.median(), 3.0);
        assert_eq!(ds.min(), 1.0);
        assert_eq!(ds.max(), 5.0);
    }
    
    #[test]
    fn descriptive_stats_extend() {
        let mut ds = DescriptiveStats::new();
        ds.extend(vec![1.0, 2.0, 3.0]);
        
        assert_eq!(ds.count(), 3);
    }
    
    #[test]
    fn descriptive_stats_empty() {
        let ds = DescriptiveStats::new();
        
        assert_eq!(ds.mean(), 0.0);
        assert_eq!(ds.median(), 0.0);
    }
    
    #[test]
    fn descriptive_stats_variance() {
        let mut ds = DescriptiveStats::new();
        ds.extend(vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]);
        
        let var = ds.variance();
        assert!(var > 0.0);
    }
    
    #[test]
    fn mean_standalone() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(mean(&values), 3.0);
    }
    
    #[test]
    fn mean_empty() {
        let values: Vec<f64> = vec![];
        assert_eq!(mean(&values), 0.0);
    }
    
    #[test]
    fn variance_standalone() {
        let values = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let var = variance(&values);
        assert!(var > 0.0);
    }
    
    #[test]
    fn std_dev_standalone() {
        let values = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let sd = std_dev(&values);
        assert!(sd > 0.0);
    }
}
