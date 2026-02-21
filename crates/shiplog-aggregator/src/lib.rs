//! Aggregation utilities for shiplog.
//!
//! This crate provides aggregation implementations for combining and summarizing data.

use std::collections::HashMap;

/// A simple aggregator that combines values using a provided function
pub struct Aggregator<T> {
    items: Vec<T>,
}

impl<T: Clone> Aggregator<T> {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn add(&mut self, item: T) {
        self.items.push(item);
    }

    pub fn add_many(&mut self, items: impl IntoIterator<Item = T>) {
        self.items.extend(items);
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn items(&self) -> &[T] {
        &self.items
    }

    pub fn sum(&self) -> Option<T>
    where
        T: Default + std::ops::Add<Output = T> + Copy,
    {
        if self.items.is_empty() {
            return None;
        }
        let mut result = T::default();
        for &item in &self.items {
            result = result + item;
        }
        Some(result)
    }

    pub fn count(&self) -> usize {
        self.items.len()
    }

    pub fn min(&self) -> Option<T>
    where
        T: Ord + Copy,
    {
        self.items.iter().copied().min()
    }

    pub fn max(&self) -> Option<T>
    where
        T: Ord + Copy,
    {
        self.items.iter().copied().max()
    }

    pub fn average(&self) -> Option<f64>
    where
        T: Copy + Into<f64>,
    {
        if self.items.is_empty() {
            return None;
        }
        let sum: f64 = self.items.iter().map(|&x| x.into()).sum();
        Some(sum / self.items.len() as f64)
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }
}

impl<T: Clone> Default for Aggregator<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// A keyed aggregator that groups and aggregates by key
pub struct KeyedAggregator<K, V> {
    groups: HashMap<K, Vec<V>>,
}

impl<K: std::hash::Hash + Eq + Clone, V: Clone> KeyedAggregator<K, V> {
    pub fn new() -> Self {
        Self {
            groups: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: K, value: V) {
        self.groups.entry(key).or_insert_with(Vec::new).push(value);
    }

    pub fn get(&self, key: &K) -> Option<&Vec<V>> {
        self.groups.get(key)
    }

    pub fn len(&self) -> usize {
        self.groups.len()
    }

    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.groups.keys()
    }

    pub fn totals(&self) -> HashMap<K, usize>
    where
        K: Clone,
    {
        self.groups
            .iter()
            .map(|(k, v)| (k.clone(), v.len()))
            .collect()
    }

    pub fn clear(&mut self) {
        self.groups.clear();
    }
}

impl<K: std::hash::Hash + Eq + Clone, V: Clone> Default for KeyedAggregator<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aggregator_basic() {
        let mut agg: Aggregator<i32> = Aggregator::new();
        
        assert!(agg.is_empty());
        
        agg.add(1);
        agg.add(2);
        agg.add(3);
        
        assert_eq!(agg.len(), 3);
    }

    #[test]
    fn test_aggregator_sum() {
        let mut agg: Aggregator<i32> = Aggregator::new();
        
        agg.add(10);
        agg.add(20);
        agg.add(30);
        
        assert_eq!(agg.sum(), Some(60));
    }

    #[test]
    fn test_aggregator_sum_empty() {
        let agg: Aggregator<i32> = Aggregator::new();
        
        assert_eq!(agg.sum(), None);
    }

    #[test]
    fn test_aggregator_min_max() {
        let mut agg: Aggregator<i32> = Aggregator::new();
        
        agg.add(5);
        agg.add(2);
        agg.add(8);
        agg.add(1);
        
        assert_eq!(agg.min(), Some(1));
        assert_eq!(agg.max(), Some(8));
    }

    #[test]
    fn test_aggregator_average() {
        let mut agg: Aggregator<i32> = Aggregator::new();
        
        agg.add(10);
        agg.add(20);
        agg.add(30);
        
        assert_eq!(agg.average(), Some(20.0));
    }

    #[test]
    fn test_aggregator_average_empty() {
        let agg: Aggregator<i32> = Aggregator::new();
        
        assert_eq!(agg.average(), None);
    }

    #[test]
    fn test_keyed_aggregator() {
        let mut agg: KeyedAggregator<&str, i32> = KeyedAggregator::new();
        
        agg.insert("a", 1);
        agg.insert("a", 2);
        agg.insert("b", 3);
        
        assert_eq!(agg.len(), 2);
        assert_eq!(agg.get(&"a"), Some(&vec![1, 2]));
        assert_eq!(agg.get(&"b"), Some(&vec![3]));
    }

    #[test]
    fn test_keyed_aggregator_totals() {
        let mut agg: KeyedAggregator<&str, i32> = KeyedAggregator::new();
        
        agg.insert("a", 1);
        agg.insert("a", 2);
        agg.insert("b", 3);
        
        let totals = agg.totals();
        
        assert_eq!(totals.get(&"a"), Some(&2));
        assert_eq!(totals.get(&"b"), Some(&1));
    }

    #[test]
    fn test_keyed_aggregator_empty() {
        let agg: KeyedAggregator<&str, i32> = KeyedAggregator::new();
        
        assert!(agg.is_empty());
        assert_eq!(agg.len(), 0);
    }
}
