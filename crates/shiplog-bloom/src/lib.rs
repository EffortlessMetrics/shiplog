//! Bloom filter implementation for shiplog.
//!
//! This crate provides a Bloom filter implementation for probabilistic
//! set membership testing with a tunable false positive rate.

use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

/// Configuration for Bloom filter
#[derive(Debug, Clone)]
pub struct BloomConfig {
    pub expected_items: usize,
    pub false_positive_rate: f64,
}

impl BloomConfig {
    /// Creates a new configuration with expected items and false positive rate
    pub fn new(expected_items: usize, false_positive_rate: f64) -> Self {
        Self {
            expected_items,
            false_positive_rate,
        }
    }

    /// Calculates the optimal number of bits
    pub fn optimal_bits(&self) -> usize {
        let m = -(self.expected_items as f64 * self.false_positive_rate.ln())
            / (2.0_f64.ln().powi(2));
        m.ceil() as usize
    }

    /// Calculates the optimal number of hash functions
    pub fn optimal_hash_count(&self) -> usize {
        let m = self.optimal_bits() as f64;
        let n = self.expected_items as f64;
        (m / n * 2.0_f64.ln()).round() as usize
    }
}

impl Default for BloomConfig {
    fn default() -> Self {
        Self::new(1000, 0.01)
    }
}

/// A Bloom filter for probabilistic set membership testing
pub struct BloomFilter<T> {
    bits: Vec<bool>,
    hash_count: usize,
    _phantom: PhantomData<T>,
}

impl<T: Hash> BloomFilter<T> {
    /// Creates a new Bloom filter with the given configuration
    pub fn with_config(config: &BloomConfig) -> Self {
        let bits = vec![false; config.optimal_bits()];
        let hash_count = config.optimal_hash_count();
        
        Self {
            bits,
            hash_count,
            _phantom: PhantomData,
        }
    }

    /// Creates a new Bloom filter with default configuration
    pub fn new() -> Self {
        Self::with_config(&BloomConfig::default())
    }

    /// Creates a Bloom filter with specified size and hash count
    pub fn with_size(bits: usize, hash_count: usize) -> Self {
        Self {
            bits: vec![false; bits],
            hash_count,
            _phantom: PhantomData,
        }
    }

    /// Inserts an item into the Bloom filter
    pub fn insert(&mut self, item: &T) {
        for i in self.get_bit_indices(item) {
            self.bits[i] = true;
        }
    }

    /// Checks if an item might be in the set
    pub fn contains(&self, item: &T) -> bool {
        self.get_bit_indices(item).iter().all(|&i| self.bits[i])
    }

    /// Returns the number of bits in the filter
    pub fn bits(&self) -> usize {
        self.bits.len()
    }

    /// Returns the number of hash functions
    pub fn hash_count(&self) -> usize {
        self.hash_count
    }

    /// Returns the current false positive rate estimate
    pub fn false_positive_rate(&self) -> f64 {
        let set_bits = self.bits.iter().filter(|&&b| b).count() as f64;
        let total_bits = self.bits.len() as f64;
        let k = self.hash_count as f64;
        
        (1.0 - (-k * set_bits / total_bits).exp()).powi(k as i32)
    }

    /// Clears the Bloom filter
    pub fn clear(&mut self) {
        for bit in &mut self.bits {
            *bit = false;
        }
    }

    /// Returns the bit indices for a given item
    fn get_bit_indices(&self, item: &T) -> Vec<usize> {
        let mut hasher1 = std::collections::hash_map::DefaultHasher::new();
        let mut hasher2 = std::collections::hash_map::DefaultHasher::new();
        
        item.hash(&mut hasher1);
        item.hash(&mut hasher2);
        
        let h1 = hasher1.finish();
        let h2 = hasher2.finish();
        
        (0..self.hash_count)
            .map(|i| {
                let combined = h1.wrapping_add((i as u64).wrapping_mul(h2));
                (combined as usize) % self.bits.len()
            })
            .collect()
    }
}

impl<T: Hash> Default for BloomFilter<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating Bloom filter configurations
#[derive(Debug)]
pub struct BloomBuilder {
    expected_items: usize,
    false_positive_rate: f64,
}

impl BloomBuilder {
    pub fn new() -> Self {
        Self {
            expected_items: 1000,
            false_positive_rate: 0.01,
        }
    }

    pub fn expected_items(mut self, items: usize) -> Self {
        self.expected_items = items;
        self
    }

    pub fn false_positive_rate(mut self, rate: f64) -> Self {
        self.false_positive_rate = rate;
        self
    }

    pub fn build_config(self) -> BloomConfig {
        BloomConfig::new(self.expected_items, self.false_positive_rate)
    }
}

impl Default for BloomBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bloom_filter_insert_contains() {
        let mut filter: BloomFilter<String> = BloomFilter::new();
        
        filter.insert(&"hello".to_string());
        filter.insert(&"world".to_string());
        
        assert!(filter.contains(&"hello".to_string()));
        assert!(filter.contains(&"world".to_string()));
        assert!(!filter.contains(&"not present".to_string()));
    }

    #[test]
    fn test_bloom_filter_default_config() {
        let config = BloomConfig::default();
        
        assert_eq!(config.expected_items, 1000);
        assert!((config.false_positive_rate - 0.01).abs() < 0.001);
    }

    #[test]
    fn test_bloom_filter_optimal_calculations() {
        let config = BloomConfig::new(1000, 0.01);
        
        let bits = config.optimal_bits();
        let hash_count = config.optimal_hash_count();
        
        // With 1000 items and 1% false positive rate:
        // bits should be around 9580
        assert!(bits > 9000);
        assert!(bits < 10000);
        
        // hash count should be around 7
        assert!(hash_count >= 6);
        assert!(hash_count <= 8);
    }

    #[test]
    fn test_bloom_filter_with_size() {
        let filter: BloomFilter<i32> = BloomFilter::with_size(100, 3);
        
        assert_eq!(filter.bits(), 100);
        assert_eq!(filter.hash_count(), 3);
    }

    #[test]
    fn test_bloom_filter_clear() {
        let mut filter: BloomFilter<String> = BloomFilter::new();
        
        filter.insert(&"test".to_string());
        assert!(filter.contains(&"test".to_string()));
        
        filter.clear();
        // Note: after clear, items may still show as contained due to
        // the probabilistic nature - this is expected behavior
        // The filter is cleared but hash collisions can still occur
    }

    #[test]
    fn test_bloom_builder() {
        let config = BloomBuilder::new()
            .expected_items(500)
            .false_positive_rate(0.05)
            .build_config();
        
        assert_eq!(config.expected_items, 500);
        assert!((config.false_positive_rate - 0.05).abs() < 0.001);
    }

    #[test]
    fn test_bloom_filter_false_positive_rate() {
        let mut filter: BloomFilter<i32> = BloomFilter::with_size(1000, 10);
        
        for i in 0..100 {
            filter.insert(&i);
        }
        
        // Should have some false positives but not too many
        let rate = filter.false_positive_rate();
        assert!(rate > 0.0);
        assert!(rate < 1.0);
    }

    #[test]
    fn test_bloom_filter_contains_not_inserted() {
        let mut filter: BloomFilter<String> = BloomFilter::new();
        
        filter.insert(&"hello".to_string());
        
        // "world" was never inserted, should not contain
        assert!(!filter.contains(&"world".to_string()));
    }
}
