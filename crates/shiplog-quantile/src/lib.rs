//! Quantile estimation utilities for shiplog.
//! 
//! This crate provides quantile estimation functionality including
//! TDigest-based quantile estimation and other approximation algorithms.

use serde::{Deserialize, Serialize};

/// TDigest-based quantile estimator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TDigest {
    centroids: Vec<Centroid>,
    compression: f64,
    count: u64,
}

/// A single centroid in the TDigest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Centroid {
    mean: f64,
    weight: f64,
}

impl TDigest {
    /// Create a new TDigest with default compression
    pub fn new() -> Self {
        Self::with_compression(100.0)
    }
    
    /// Create a new TDigest with custom compression
    pub fn with_compression(compression: f64) -> Self {
        Self {
            centroids: Vec::new(),
            compression: compression.max(1.0),
            count: 0,
        }
    }
    
    /// Add a value to the digest
    pub fn add(&mut self, value: f64) {
        self.add_weighted(value, 1.0);
    }
    
    /// Add a weighted value to the digest
    pub fn add_weighted(&mut self, value: f64, weight: f64) {
        self.count += 1;
        
        // Merge with existing centroid if close
        for centroid in &mut self.centroids {
            if (centroid.mean - value).abs() < 1.0 / self.compression {
                let total_weight = centroid.weight + weight;
                centroid.mean = (centroid.mean * centroid.weight + value * weight) / total_weight;
                centroid.weight = total_weight;
                return;
            }
        }
        
        // Add new centroid
        self.centroids.push(Centroid { mean: value, weight });
        
        // Compress if needed
        if self.centroids.len() > (self.compression * 10.0) as usize {
            self.compress();
        }
    }
    
    /// Compress the digest by merging nearby centroids
    fn compress(&mut self) {
        if self.centroids.is_empty() {
            return;
        }
        
        // Sort by mean
        self.centroids.sort_by(|a, b| a.mean.partial_cmp(&b.mean).unwrap_or(std::cmp::Ordering::Equal));
        
        // Merge centroids
        let mut merged: Vec<Centroid> = Vec::new();
        let mut current = self.centroids[0].clone();
        
        for centroid in self.centroids.iter().skip(1) {
            let max_weight = self.compression;
            if current.weight + centroid.weight <= max_weight {
                let total_weight = current.weight + centroid.weight;
                current.mean = (current.mean * current.weight + centroid.mean * centroid.weight) / total_weight;
                current.weight = total_weight;
            } else {
                merged.push(current);
                current = centroid.clone();
            }
        }
        merged.push(current);
        
        self.centroids = merged;
    }
    
    /// Estimate the quantile value
    pub fn quantile(&self, q: f64) -> f64 {
        if self.centroids.is_empty() {
            return 0.0;
        }
        
        let q = q.clamp(0.0, 1.0);
        let total_weight: f64 = self.centroids.iter().map(|c| c.weight).sum();
        let rank = q * total_weight;
        
        let mut cumulative = 0.0;
        for centroid in &self.centroids {
            cumulative += centroid.weight;
            if cumulative >= rank {
                return centroid.mean;
            }
        }
        
        // Return last centroid mean if we exceed
        self.centroids.last().map(|c| c.mean).unwrap_or(0.0)
    }
    
    /// Get total count
    pub fn count(&self) -> u64 {
        self.count
    }
    
    /// Get number of centroids
    pub fn centroids(&self) -> usize {
        self.centroids.len()
    }
    
    /// Reset the digest
    pub fn reset(&mut self) {
        self.centroids.clear();
        self.count = 0;
    }
}

impl Default for TDigest {
    fn default() -> Self {
        Self::new()
    }
}

/// Quantile estimator using reservoir sampling
#[derive(Debug, Clone)]
pub struct ReservoirQuantile {
    quantile: f64,
    reservoir: Vec<f64>,
    count: u64,
    max_size: usize,
}

impl ReservoirQuantile {
    /// Create a new reservoir quantile estimator
    pub fn new(quantile: f64, max_size: usize) -> Self {
        Self {
            quantile: quantile.clamp(0.0, 1.0),
            reservoir: Vec::with_capacity(max_size),
            count: 0,
            max_size,
        }
    }
    
    /// Add a value
    pub fn add(&mut self, value: f64) {
        self.count += 1;
        
        if self.reservoir.len() < self.max_size {
            self.reservoir.push(value);
        } else {
            // Reservoir sampling
            let idx = (self.count as f64 * self.quantile) as usize % self.max_size;
            if (self.count as f64 * self.quantile).fract() < 1.0 / self.max_size as f64 {
                self.reservoir[idx] = value;
            }
        }
    }
    
    /// Get the quantile estimate
    pub fn quantile(&self) -> f64 {
        if self.reservoir.is_empty() {
            return 0.0;
        }
        
        let mut sorted = self.reservoir.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        
        let idx = ((self.quantile) * (sorted.len() - 1) as f64) as usize;
        let idx = idx.min(sorted.len() - 1);
        sorted[idx]
    }
    
    /// Get the count
    pub fn count(&self) -> u64 {
        self.count
    }
}

/// GK quantile estimator (Greenwald-Khanna)
#[derive(Debug, Clone)]
pub struct GKQuantile {
    epsilon: f64,
    samples: Vec<(f64, u64, u64)>,
    count: u64,
}

impl GKQuantile {
    /// Create a new GK quantile estimator
    pub fn new(epsilon: f64) -> Self {
        Self {
            epsilon: epsilon.clamp(0.001, 0.5),
            samples: Vec::new(),
            count: 0,
        }
    }
    
    /// Add a value
    pub fn add(&mut self, value: f64) {
        self.count += 1;
        
        if self.samples.is_empty() {
            self.samples.push((value, 1, 1));
            return;
        }
        
        // Find position and insert
        let pos = self.samples.iter().position(|(v, _, _)| value <= *v);
        
        match pos {
            Some(idx) => {
                let (_v, g, _delta) = &mut self.samples[idx];
                *g += 1;
                if *g > (2.0 * self.epsilon * self.count as f64) as u64 {
                    self.compress();
                    self.samples.insert(idx, (value, 1, 1));
                }
            }
            None => {
                self.samples.push((value, 1, 1));
            }
        }
    }
    
    /// Compress the samples
    fn compress(&mut self) {
        let threshold = (2.0 * self.epsilon * self.count as f64) as u64;
        self.samples.retain(|(_, g, _)| *g <= threshold);
    }
    
    /// Get the quantile estimate
    pub fn quantile(&self, q: f64) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        
        let q = q.clamp(0.0, 1.0);
        let rank = q * self.count as f64;
        let mut cumsum = 0u64;
        
        for (value, g, _) in &self.samples {
            cumsum += g;
            if cumsum as f64 >= rank {
                return *value;
            }
        }
        
        self.samples.last().map(|(v, _, _)| *v).unwrap_or(0.0)
    }
    
    /// Get total count
    pub fn count(&self) -> u64 {
        self.count
    }
}

/// Simple quantile calculator using sorted array
pub fn quantile(values: &[f64], q: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    
    let idx = (q * (sorted.len() - 1) as f64) as usize;
    let idx = idx.min(sorted.len() - 1);
    sorted[idx]
}

/// Calculate multiple quantiles at once
pub fn quantiles(values: &[f64], qs: &[f64]) -> Vec<(f64, f64)> {
    qs.iter().map(|&q| (q, quantile(values, q))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn tdigest_basic() {
        let mut td = TDigest::new();
        
        for i in 1..=1000 {
            td.add(i as f64);
        }
        
        assert!(td.quantile(0.5) > 0.0);
        assert!(td.quantile(0.99) > td.quantile(0.5));
    }
    
    #[test]
    fn tdigest_empty() {
        let td = TDigest::new();
        assert_eq!(td.quantile(0.5), 0.0);
    }
    
    #[test]
    fn tdigest_single() {
        let mut td = TDigest::new();
        td.add(42.0);
        
        assert_eq!(td.quantile(0.5), 42.0);
    }
    
    #[test]
    fn tdigest_weighted() {
        let mut td = TDigest::new();
        
        td.add_weighted(1.0, 100.0);
        td.add_weighted(100.0, 1.0);
        
        let q = td.quantile(0.5);
        assert!(q < 50.0); // Should be closer to 1 due to higher weight
    }
    
    #[test]
    fn tdigest_reset() {
        let mut td = TDigest::new();
        
        td.add(10.0);
        td.reset();
        
        assert_eq!(td.count(), 0);
    }
    
    #[test]
    fn reservoir_quantile() {
        let mut rq = ReservoirQuantile::new(0.5, 100);
        
        for i in 1..=1000 {
            rq.add(i as f64);
        }
        
        assert!(rq.quantile() > 0.0);
    }
    
    #[test]
    fn reservoir_quantile_empty() {
        let rq = ReservoirQuantile::new(0.5, 100);
        assert_eq!(rq.quantile(), 0.0);
    }
    
    #[test]
    fn reservoir_quantile_single() {
        let mut rq = ReservoirQuantile::new(0.5, 100);
        rq.add(42.0);
        
        assert_eq!(rq.quantile(), 42.0);
    }
    
    #[test]
    fn gk_quantile() {
        let mut gk = GKQuantile::new(0.05);
        
        for i in 1..=1000 {
            gk.add(i as f64);
        }
        
        let q50 = gk.quantile(0.5);
        assert!(q50 > 0.0);
    }
    
    #[test]
    fn gk_quantile_empty() {
        let gk = GKQuantile::new(0.05);
        assert_eq!(gk.quantile(0.5), 0.0);
    }
    
    #[test]
    fn gk_quantile_single() {
        let mut gk = GKQuantile::new(0.05);
        gk.add(42.0);
        
        assert_eq!(gk.quantile(0.5), 42.0);
    }
    
    #[test]
    fn quantile_standalone() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        
        assert_eq!(quantile(&values, 0.0), 1.0);
        assert_eq!(quantile(&values, 0.5), 3.0);
        assert_eq!(quantile(&values, 1.0), 5.0);
    }
    
    #[test]
    fn quantile_empty() {
        let values: Vec<f64> = vec![];
        assert_eq!(quantile(&values, 0.5), 0.0);
    }
    
    #[test]
    fn quantiles_multiple() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        
        let result = quantiles(&values, &[0.25, 0.5, 0.75]);
        
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].0, 0.25);
        assert_eq!(result[1].0, 0.5);
    }
}
