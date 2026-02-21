//! Accumulator pattern utilities for shiplog.
//!
//! This crate provides accumulator implementations for collecting and combining values.

use std::collections::HashMap;

/// A generic accumulator that collects and combines values
pub struct Accumulator<T> {
    values: Vec<T>,
    count: usize,
}

impl<T> Accumulator<T> {
    pub fn new() -> Self {
        Self {
            values: Vec::new(),
            count: 0,
        }
    }

    pub fn add(&mut self, value: T) {
        self.values.push(value);
        self.count += 1;
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn values(&self) -> &[T] {
        &self.values
    }

    pub fn clear(&mut self) {
        self.values.clear();
        self.count = 0;
    }
}

impl<T> Default for Accumulator<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// A summing accumulator for numeric types
pub struct SumAccumulator<T> {
    sum: T,
    count: usize,
}

impl<T: Default + Copy + std::ops::Add<Output = T>> SumAccumulator<T> {
    pub fn new() -> Self {
        Self {
            sum: T::default(),
            count: 0,
        }
    }

    pub fn add(&mut self, value: T) {
        self.sum = self.sum + value;
        self.count += 1;
    }

    pub fn sum(&self) -> T {
        self.sum
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn average(&self) -> Option<T>
    where
        T: std::ops::Div<Output = T> + From<u8>,
    {
        if self.count == 0 {
            None
        } else {
            Some(self.sum / T::from(self.count as u8))
        }
    }
}

impl<T: Default + Copy + std::ops::Add<Output = T>> Default for SumAccumulator<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// A grouping accumulator that groups values by a key
pub struct GroupAccumulator<K, V> {
    groups: HashMap<K, Vec<V>>,
}

impl<K: std::hash::Hash + Eq + Clone, V> GroupAccumulator<K, V> {
    pub fn new() -> Self {
        Self {
            groups: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: K, value: V) {
        self.groups.entry(key).or_default().push(value);
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
}

impl<K: std::hash::Hash + Eq + Clone, V> Default for GroupAccumulator<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accumulator_basic() {
        let mut acc: Accumulator<i32> = Accumulator::new();

        assert!(acc.is_empty());

        acc.add(1);
        acc.add(2);
        acc.add(3);

        assert_eq!(acc.len(), 3);
        assert_eq!(acc.values(), &[1, 2, 3]);
    }

    #[test]
    fn test_accumulator_clear() {
        let mut acc: Accumulator<i32> = Accumulator::new();
        acc.add(1);
        acc.add(2);

        acc.clear();

        assert!(acc.is_empty());
        assert_eq!(acc.len(), 0);
    }

    #[test]
    fn test_sum_accumulator() {
        let mut acc: SumAccumulator<i32> = SumAccumulator::new();

        acc.add(10);
        acc.add(20);
        acc.add(30);

        assert_eq!(acc.sum(), 60);
        assert_eq!(acc.count(), 3);
        assert_eq!(acc.average(), Some(20));
    }

    #[test]
    fn test_sum_accumulator_empty() {
        let acc: SumAccumulator<i32> = SumAccumulator::new();

        assert_eq!(acc.sum(), 0);
        assert_eq!(acc.count(), 0);
        assert!(acc.average().is_none());
    }

    #[test]
    fn test_group_accumulator() {
        let mut acc: GroupAccumulator<&str, i32> = GroupAccumulator::new();

        acc.insert("even", 2);
        acc.insert("even", 4);
        acc.insert("odd", 1);
        acc.insert("odd", 3);

        assert_eq!(acc.len(), 2);

        assert_eq!(acc.get(&"even"), Some(&vec![2, 4]));
        assert_eq!(acc.get(&"odd"), Some(&vec![1, 3]));
    }

    #[test]
    fn test_group_accumulator_empty() {
        let acc: GroupAccumulator<&str, i32> = GroupAccumulator::new();

        assert!(acc.is_empty());
        assert_eq!(acc.len(), 0);
    }
}
