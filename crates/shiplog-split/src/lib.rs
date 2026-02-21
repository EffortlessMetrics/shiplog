//! Stream splitting utilities for shiplog.
//!
//! This crate provides stream splitting implementations for dividing
//! streams of data into multiple output streams based on various criteria.

use std::collections::HashMap;
use std::vec;

/// Predicate function type for splitting
pub type SplitPredicate<T> = fn(&T) -> bool;

/// Key extractor function type for group-based splitting
pub type SplitKeyExtractor<T, K> = fn(&T) -> K;

/// Split result containing matched and unmatched items
#[derive(Debug, Clone)]
pub struct SplitResult<T> {
    pub matched: Vec<T>,
    pub unmatched: Vec<T>,
}

/// Partition function type
pub type PartitionFn<T> = fn(&T) -> usize;

/// Stream splitter
pub struct StreamSplitter<T> {
    data: Vec<T>,
}

impl<T: Clone> Default for StreamSplitter<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> StreamSplitter<T> {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    /// Add data to be split
    pub fn with_data(mut self, data: Vec<T>) -> Self {
        self.data = data;
        self
    }

    /// Split by predicate
    pub fn split_by<P: Fn(&T) -> bool>(self, predicate: P) -> SplitResult<T> {
        let mut matched = Vec::new();
        let mut unmatched = Vec::new();

        for item in self.data {
            if predicate(&item) {
                matched.push(item);
            } else {
                unmatched.push(item);
            }
        }

        SplitResult { matched, unmatched }
    }

    /// Split into N partitions by index
    pub fn split_into_n(self, n: usize) -> Vec<Vec<T>> {
        if n == 0 {
            return vec![];
        }

        let len = self.data.len();
        if len == 0 {
            return vec![Vec::new(); n];
        }

        let base = len / n;
        let remainder = len % n;

        let mut result = Vec::with_capacity(n);
        let mut start = 0;

        for i in 0..n {
            let extra = if i < remainder { 1 } else { 0 };
            let end = start + base + extra;
            result.push(self.data[start..end].to_vec());
            start = end;
        }

        result
    }

    /// Partition by function returning partition index
    pub fn partition_by(self, part_fn: PartitionFn<T>) -> Vec<Vec<T>> {
        let mut result: HashMap<usize, Vec<T>> = HashMap::new();

        for item in self.data {
            let idx = part_fn(&item);
            result.entry(idx).or_default().push(item);
        }

        // Convert to sorted vector by key
        let mut keys: Vec<_> = result.keys().cloned().collect();
        keys.sort();
        keys.into_iter()
            .map(|k| result.remove(&k).unwrap())
            .collect()
    }

    /// Group by key extractor
    pub fn group_by_key<K: Eq + std::hash::Hash + Clone>(
        self,
        key_fn: SplitKeyExtractor<T, K>,
    ) -> HashMap<K, Vec<T>> {
        let mut groups: HashMap<K, Vec<T>> = HashMap::new();

        for item in self.data {
            let key = key_fn(&item);
            groups.entry(key).or_default().push(item);
        }

        groups
    }

    /// Take first N elements
    pub fn take(self, n: usize) -> Vec<T> {
        self.data.into_iter().take(n).collect()
    }

    /// Skip first N elements
    pub fn skip(self, n: usize) -> Vec<T> {
        self.data.into_iter().skip(n).collect()
    }

    /// Split at specific index
    pub fn split_at(self, index: usize) -> (Vec<T>, Vec<T>) {
        if index >= self.data.len() {
            return (self.data, Vec::new());
        }
        (self.data[..index].to_vec(), self.data[index..].to_vec())
    }
}

/// Split mode for round-robin distribution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitMode {
    /// Distribute elements round-robin
    RoundRobin,
    /// Put all elements in first non-empty stream
    First,
    /// Put elements in stream matching predicate
    ByPredicate,
}

/// Round-robin stream splitter
pub struct RoundRobinSplitter<T> {
    data: Vec<T>,
    num_streams: usize,
}

impl<T: Clone> RoundRobinSplitter<T> {
    pub fn new(num_streams: usize) -> Self {
        Self {
            data: Vec::new(),
            num_streams,
        }
    }

    pub fn with_data(mut self, data: Vec<T>) -> Self {
        self.data = data;
        self
    }

    /// Execute round-robin split
    pub fn execute(&self) -> Vec<Vec<T>> {
        if self.num_streams == 0 {
            return vec![];
        }

        let mut result: Vec<Vec<T>> = vec![Vec::new(); self.num_streams];

        for (idx, item) in self.data.iter().enumerate() {
            result[idx % self.num_streams].push(item.clone());
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_by_predicate() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        let result = StreamSplitter::new()
            .with_data(data)
            .split_by(|x| x % 2 == 0);

        assert_eq!(result.matched, vec![2, 4, 6, 8, 10]);
        assert_eq!(result.unmatched, vec![1, 3, 5, 7, 9]);
    }

    #[test]
    fn test_split_into_n() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        let result = StreamSplitter::new().with_data(data).split_into_n(3);

        assert_eq!(result.len(), 3);
        assert_eq!(result[0], vec![1, 2, 3, 4]);
        assert_eq!(result[1], vec![5, 6, 7]);
        assert_eq!(result[2], vec![8, 9, 10]);
    }

    #[test]
    fn test_split_into_n_uneven() {
        let data = vec![1, 2, 3, 4, 5];

        let result = StreamSplitter::new().with_data(data).split_into_n(3);

        assert_eq!(result.len(), 3);
        assert_eq!(result[0], vec![1, 2]);
        assert_eq!(result[1], vec![3, 4]);
        assert_eq!(result[2], vec![5]);
    }

    #[test]
    fn test_partition_by() {
        let data = vec![1, 2, 3, 4, 5, 6];

        let result = StreamSplitter::new()
            .with_data(data)
            .partition_by(|x| x % 3);

        assert_eq!(result.len(), 3);
        assert_eq!(result[0], vec![3, 6]);
        assert_eq!(result[1], vec![1, 4]);
        assert_eq!(result[2], vec![2, 5]);
    }

    #[test]
    fn test_group_by_key() {
        #[derive(Clone, Debug, PartialEq)]
        struct Item {
            id: i32,
            category: String,
        }

        let data = vec![
            Item {
                id: 1,
                category: "fruit".to_string(),
            },
            Item {
                id: 2,
                category: "vegetable".to_string(),
            },
            Item {
                id: 3,
                category: "fruit".to_string(),
            },
        ];

        let result = StreamSplitter::new()
            .with_data(data)
            .group_by_key(|item| item.category.clone());

        assert_eq!(result.len(), 2);
        assert_eq!(result["fruit"].len(), 2);
        assert_eq!(result["vegetable"].len(), 1);
    }

    #[test]
    fn test_take() {
        let data = vec![1, 2, 3, 4, 5];

        let result = StreamSplitter::new().with_data(data).take(3);

        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_skip() {
        let data = vec![1, 2, 3, 4, 5];

        let result = StreamSplitter::new().with_data(data).skip(2);

        assert_eq!(result, vec![3, 4, 5]);
    }

    #[test]
    fn test_split_at() {
        let data = vec![1, 2, 3, 4, 5];

        let (first, second) = StreamSplitter::new().with_data(data).split_at(3);

        assert_eq!(first, vec![1, 2, 3]);
        assert_eq!(second, vec![4, 5]);
    }

    #[test]
    fn test_round_robin_splitter() {
        let data = vec![1, 2, 3, 4, 5, 6];

        let result = RoundRobinSplitter::new(3).with_data(data).execute();

        assert_eq!(result.len(), 3);
        assert_eq!(result[0], vec![1, 4]);
        assert_eq!(result[1], vec![2, 5]);
        assert_eq!(result[2], vec![3, 6]);
    }

    #[test]
    fn test_split_empty() {
        let data: Vec<i32> = vec![];

        let result = StreamSplitter::new().with_data(data).split_by(|x| *x > 5);

        assert!(result.matched.is_empty());
        assert!(result.unmatched.is_empty());
    }

    #[test]
    fn test_split_into_n_zero() {
        let data = vec![1, 2, 3];

        let result = StreamSplitter::new().with_data(data).split_into_n(0);

        assert!(result.is_empty());
    }
}
