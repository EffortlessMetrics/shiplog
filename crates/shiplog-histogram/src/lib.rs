//! Histogram utilities for shiplog.
//!
//! This crate provides histogram functionality for metrics collection
//! including fixed bucket histograms and dynamic histogram implementations.

use serde::{Deserialize, Serialize};

/// A histogram with fixed buckets for metrics collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Histogram {
    name: String,
    buckets: Vec<Bucket>,
    sum: f64,
    count: u64,
    min: f64,
    max: f64,
}

/// A single histogram bucket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bucket {
    upper_bound: f64,
    count: u64,
}

impl Histogram {
    /// Create a new histogram with default exponential buckets
    pub fn new(name: impl Into<String>) -> Self {
        // Default bucket boundaries: 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0
        let buckets = vec![
            Bucket {
                upper_bound: 0.005,
                count: 0,
            },
            Bucket {
                upper_bound: 0.01,
                count: 0,
            },
            Bucket {
                upper_bound: 0.025,
                count: 0,
            },
            Bucket {
                upper_bound: 0.05,
                count: 0,
            },
            Bucket {
                upper_bound: 0.1,
                count: 0,
            },
            Bucket {
                upper_bound: 0.25,
                count: 0,
            },
            Bucket {
                upper_bound: 0.5,
                count: 0,
            },
            Bucket {
                upper_bound: 1.0,
                count: 0,
            },
            Bucket {
                upper_bound: 2.5,
                count: 0,
            },
            Bucket {
                upper_bound: 5.0,
                count: 0,
            },
            Bucket {
                upper_bound: 10.0,
                count: 0,
            },
            Bucket {
                upper_bound: f64::INFINITY,
                count: 0,
            },
        ];

        Self {
            name: name.into(),
            buckets,
            sum: 0.0,
            count: 0,
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
        }
    }

    /// Create a histogram with custom bucket boundaries
    pub fn with_buckets(name: impl Into<String>, bounds: Vec<f64>) -> Self {
        let mut buckets: Vec<Bucket> = bounds
            .into_iter()
            .map(|b| Bucket {
                upper_bound: b,
                count: 0,
            })
            .collect();

        // Ensure there's an infinity bucket
        if buckets
            .last()
            .map(|b| b.upper_bound.is_infinite())
            .unwrap_or(false)
        {
            // Already has infinity bucket
        } else {
            buckets.push(Bucket {
                upper_bound: f64::INFINITY,
                count: 0,
            });
        }

        Self {
            name: name.into(),
            buckets,
            sum: 0.0,
            count: 0,
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
        }
    }

    /// Record a value in the histogram
    pub fn record(&mut self, value: f64) {
        self.count += 1;
        self.sum += value;

        if value < self.min {
            self.min = value;
        }
        if value > self.max {
            self.max = value;
        }

        for bucket in &mut self.buckets {
            if value <= bucket.upper_bound {
                bucket.count += 1;
                break;
            }
        }
    }

    /// Get the histogram name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the total count of recorded values
    pub fn count(&self) -> u64 {
        self.count
    }

    /// Get the sum of all recorded values
    pub fn sum(&self) -> f64 {
        self.sum
    }

    /// Get the minimum recorded value
    pub fn min(&self) -> f64 {
        if self.count == 0 { 0.0 } else { self.min }
    }

    /// Get the maximum recorded value
    pub fn max(&self) -> f64 {
        if self.count == 0 { 0.0 } else { self.max }
    }

    /// Get the mean of all recorded values
    pub fn mean(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.sum / self.count as f64
        }
    }

    /// Get all buckets
    pub fn buckets(&self) -> &[Bucket] {
        &self.buckets
    }

    /// Get the bucket index for a given value
    pub fn bucket_index(&self, value: f64) -> usize {
        for (i, bucket) in self.buckets.iter().enumerate() {
            if value <= bucket.upper_bound {
                return i;
            }
        }
        self.buckets.len() - 1
    }

    /// Get the cumulative count at or below a threshold
    pub fn cumulative_below(&self, threshold: f64) -> u64 {
        let mut total = 0u64;
        for bucket in &self.buckets {
            if bucket.upper_bound > threshold {
                break;
            }
            total += bucket.count;
        }
        total
    }

    /// Reset the histogram
    pub fn reset(&mut self) {
        for bucket in &mut self.buckets {
            bucket.count = 0;
        }
        self.sum = 0.0;
        self.count = 0;
        self.min = f64::INFINITY;
        self.max = f64::NEG_INFINITY;
    }
}

/// A simple linear histogram with evenly spaced buckets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinearHistogram {
    name: String,
    min: f64,
    max: f64,
    bucket_count: usize,
    buckets: Vec<u64>,
    underflow: u64,
    overflow: u64,
    sum: f64,
    count: u64,
}

impl LinearHistogram {
    /// Create a new linear histogram
    pub fn new(name: impl Into<String>, min: f64, max: f64, bucket_count: usize) -> Self {
        Self {
            name: name.into(),
            min,
            max,
            bucket_count,
            buckets: vec![0; bucket_count],
            underflow: 0,
            overflow: 0,
            sum: 0.0,
            count: 0,
        }
    }

    /// Record a value
    pub fn record(&mut self, value: f64) {
        self.count += 1;
        self.sum += value;

        if value < self.min {
            self.underflow += 1;
        } else if value >= self.max {
            self.overflow += 1;
        } else {
            let range = self.max - self.min;
            let normalized = (value - self.min) / range;
            let bucket_index =
                ((normalized * self.bucket_count as f64) as usize).min(self.bucket_count - 1);
            self.buckets[bucket_index] += 1;
        }
    }

    /// Get the bucket values
    pub fn buckets(&self) -> &[u64] {
        &self.buckets
    }

    /// Get the underflow count
    pub fn underflow(&self) -> u64 {
        self.underflow
    }

    /// Get the overflow count
    pub fn overflow(&self) -> u64 {
        self.overflow
    }

    /// Get the total count
    pub fn count(&self) -> u64 {
        self.count
    }

    /// Get the mean
    pub fn mean(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.sum / self.count as f64
        }
    }

    /// Reset the histogram
    pub fn reset(&mut self) {
        for bucket in &mut self.buckets {
            *bucket = 0;
        }
        self.underflow = 0;
        self.overflow = 0;
        self.sum = 0.0;
        self.count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn histogram_basic() {
        let mut hist = Histogram::new("test");

        hist.record(0.001);
        hist.record(0.01);
        hist.record(0.1);
        hist.record(1.0);
        hist.record(10.0);

        assert_eq!(hist.count(), 5);
        assert_eq!(hist.sum(), 11.111);
    }

    #[test]
    fn histogram_min_max() {
        let mut hist = Histogram::new("test");

        hist.record(5.0);
        hist.record(1.0);
        hist.record(10.0);

        assert_eq!(hist.min(), 1.0);
        assert_eq!(hist.max(), 10.0);
    }

    #[test]
    fn histogram_mean() {
        let mut hist = Histogram::new("test");

        hist.record(10.0);
        hist.record(20.0);
        hist.record(30.0);

        assert_eq!(hist.mean(), 20.0);
    }

    #[test]
    fn histogram_buckets() {
        let mut hist = Histogram::new("test");

        hist.record(0.001);
        hist.record(0.004); // should go in 0.005 bucket
        hist.record(0.1);

        let buckets = hist.buckets();
        // Find bucket indices
        let idx_005 = buckets.iter().position(|b| b.upper_bound == 0.005).unwrap();

        // Check that values went into buckets
        assert!(buckets[idx_005].count >= 2);
        assert_eq!(hist.count(), 3);
    }

    #[test]
    fn histogram_cumulative() {
        let mut hist = Histogram::new("test");

        hist.record(0.001);
        hist.record(0.006);
        hist.record(0.1);

        let cum = hist.cumulative_below(0.025);
        assert!(cum >= 2);
    }

    #[test]
    fn histogram_reset() {
        let mut hist = Histogram::new("test");

        hist.record(10.0);
        hist.reset();

        assert_eq!(hist.count(), 0);
        assert_eq!(hist.sum(), 0.0);
    }

    #[test]
    fn histogram_custom_buckets() {
        let bounds = vec![1.0, 5.0, 10.0];
        let mut hist = Histogram::with_buckets("test", bounds);

        hist.record(0.5);
        hist.record(3.0);
        hist.record(7.0);
        hist.record(15.0);

        assert_eq!(hist.count(), 4);
    }

    #[test]
    fn linear_histogram() {
        let mut hist = LinearHistogram::new("test", 0.0, 100.0, 10);

        hist.record(5.0);
        hist.record(25.0);
        hist.record(55.0);
        hist.record(95.0);

        assert_eq!(hist.count(), 4);
        assert_eq!(hist.mean(), 45.0);
    }

    #[test]
    fn linear_histogram_overflow() {
        let mut hist = LinearHistogram::new("test", 0.0, 100.0, 10);

        hist.record(50.0);
        hist.record(-10.0); // underflow
        hist.record(150.0); // overflow

        assert_eq!(hist.underflow(), 1);
        assert_eq!(hist.overflow(), 1);
    }

    #[test]
    fn linear_histogram_reset() {
        let mut hist = LinearHistogram::new("test", 0.0, 100.0, 5);

        hist.record(50.0);
        hist.reset();

        assert_eq!(hist.count(), 0);
    }
}
