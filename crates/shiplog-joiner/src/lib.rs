//! Stream join operations for shiplog.
//!
//! This crate provides join implementations for combining streams of data
//! based on key matching.

use std::collections::HashMap;

/// A join result containing matched pairs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinResult<L, R> {
    pub left: L,
    pub right: R,
}

/// Inner join operator for two streams
pub struct InnerJoin<L, R, K> {
    left_data: Vec<(K, L)>,
    right_data: Vec<(K, R)>,
}

impl<L: Clone, R: Clone, K: Eq + std::hash::Hash + Clone> Default for InnerJoin<L, R, K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<L: Clone, R: Clone, K: Eq + std::hash::Hash + Clone> InnerJoin<L, R, K> {
    pub fn new() -> Self {
        Self {
            left_data: Vec::new(),
            right_data: Vec::new(),
        }
    }

    pub fn with_left_data(mut self, data: Vec<(K, L)>) -> Self {
        self.left_data = data;
        self
    }

    pub fn with_right_data(mut self, data: Vec<(K, R)>) -> Self {
        self.right_data = data;
        self
    }

    /// Execute the inner join and return matching pairs
    pub fn execute(&self) -> Vec<JoinResult<L, R>> {
        let right_map: HashMap<_, _> = self.right_data.iter().cloned().collect();
        let mut results = Vec::new();

        for (key, left_val) in &self.left_data {
            if let Some(right_val) = right_map.get(key) {
                results.push(JoinResult {
                    left: left_val.clone(),
                    right: right_val.clone(),
                });
            }
        }

        results
    }
}

/// Left outer join operator for two streams
pub struct LeftJoin<L, R, K> {
    left_data: Vec<(K, L)>,
    right_data: Vec<(K, R)>,
}

impl<L: Clone, R: Clone, K: Eq + std::hash::Hash + Clone> Default for LeftJoin<L, R, K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<L: Clone, R: Clone, K: Eq + std::hash::Hash + Clone> LeftJoin<L, R, K> {
    pub fn new() -> Self {
        Self {
            left_data: Vec::new(),
            right_data: Vec::new(),
        }
    }

    pub fn with_left_data(mut self, data: Vec<(K, L)>) -> Self {
        self.left_data = data;
        self
    }

    pub fn with_right_data(mut self, data: Vec<(K, R)>) -> Self {
        self.right_data = data;
        self
    }

    /// Execute the left outer join
    pub fn execute(&self) -> Vec<JoinResult<L, Option<R>>> {
        let right_map: HashMap<_, _> = self.right_data.iter().cloned().collect();
        let mut results = Vec::new();

        for (key, left_val) in &self.left_data {
            let right_val = right_map.get(key).cloned();
            results.push(JoinResult {
                left: left_val.clone(),
                right: right_val,
            });
        }

        results
    }
}

/// Join key extractor function type
pub type KeyExtractor<T, K> = fn(&T) -> K;

/// Generic stream joiner
pub struct StreamJoiner<T, K> {
    data: Vec<T>,
    extractor: KeyExtractor<T, K>,
}

impl<T: Clone, K: Eq + std::hash::Hash + Clone> StreamJoiner<T, K> {
    pub fn new(extractor: KeyExtractor<T, K>) -> Self {
        Self {
            data: Vec::new(),
            extractor,
        }
    }

    pub fn push(mut self, value: T) -> Self {
        self.data.push(value);
        self
    }

    pub fn add_many(mut self, values: Vec<T>) -> Self {
        self.data.extend(values);
        self
    }

    /// Group data by key
    pub fn group_by_key(&self) -> HashMap<K, Vec<T>> {
        let mut groups: HashMap<K, Vec<T>> = HashMap::new();
        for item in &self.data {
            let key = (self.extractor)(item);
            groups.entry(key).or_default().push(item.clone());
        }
        groups
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inner_join_basic() {
        let left = vec![(1, "apple"), (2, "banana"), (3, "cherry")];
        let right = vec![(1, "red"), (2, "yellow"), (4, "green")];

        let result = InnerJoin::new()
            .with_left_data(left)
            .with_right_data(right)
            .execute();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].left, "apple");
        assert_eq!(result[0].right, "red");
        assert_eq!(result[1].left, "banana");
        assert_eq!(result[1].right, "yellow");
    }

    #[test]
    fn test_inner_join_no_matches() {
        let left = vec![(1, "apple"), (2, "banana")];
        let right = vec![(3, "cherry"), (4, "date")];

        let result = InnerJoin::new()
            .with_left_data(left)
            .with_right_data(right)
            .execute();

        assert!(result.is_empty());
    }

    #[test]
    fn test_left_join_basic() {
        let left = vec![(1, "apple"), (2, "banana"), (3, "cherry")];
        let right = vec![(1, "red"), (2, "yellow")];

        let result = LeftJoin::new()
            .with_left_data(left)
            .with_right_data(right)
            .execute();

        assert_eq!(result.len(), 3);
        assert_eq!(result[0].left, "apple");
        assert_eq!(result[0].right, Some("red"));
        assert_eq!(result[1].left, "banana");
        assert_eq!(result[1].right, Some("yellow"));
        assert_eq!(result[2].left, "cherry");
        assert_eq!(result[2].right, None);
    }

    #[test]
    fn test_stream_joiner_group_by() {
        #[derive(Clone, Debug, PartialEq)]
        struct Item {
            id: i32,
            category: String,
        }

        let extractor = |item: &Item| item.category.clone();

        let items = vec![
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

        let joiner = StreamJoiner::new(extractor).add_many(items);
        let groups = joiner.group_by_key();

        assert_eq!(groups.len(), 2);
        assert_eq!(groups.get("fruit").unwrap().len(), 2);
        assert_eq!(groups.get("vegetable").unwrap().len(), 1);
    }

    #[test]
    fn test_stream_joiner_empty() {
        let extractor = |&x: &i32| x;

        let joiner = StreamJoiner::new(extractor);
        let groups = joiner.group_by_key();

        assert!(groups.is_empty());
    }
}
