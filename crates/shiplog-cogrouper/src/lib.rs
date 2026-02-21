//! Co-grouping for stream operations in shiplog.
//!
//! This crate provides co-grouping implementations for organizing multiple
//! streams of data by a common key.

use std::collections::HashMap;

/// A co-group result containing grouped data from multiple streams
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoGroup<K, T> {
    pub key: K,
    pub groups: Vec<Vec<T>>,
}

/// CoGrouper for grouping multiple streams by a common key
pub struct CoGrouper<K, T> {
    streams: Vec<Vec<(K, T)>>,
}

impl<K: Eq + std::hash::Hash + Clone, T: Clone> CoGrouper<K, T> {
    pub fn new() -> Self {
        Self {
            streams: Vec::new(),
        }
    }

    /// Add a stream to be co-grouped
    pub fn add_stream(mut self, stream: Vec<(K, T)>) -> Self {
        self.streams.push(stream);
        self
    }

    /// Execute the co-group operation
    pub fn execute(&self) -> HashMap<K, CoGroup<K, T>> {
        let mut result: HashMap<K, CoGroup<K, T>> = HashMap::new();

        // Initialize all keys from all streams
        for stream in &self.streams {
            for (key, _) in stream {
                if !result.contains_key(key) {
                    result.insert(
                        key.clone(),
                        CoGroup {
                            key: key.clone(),
                            groups: vec![Vec::new(); self.streams.len()],
                        },
                    );
                }
            }
        }

        // Group data from each stream
        for (stream_idx, stream) in self.streams.iter().enumerate() {
            for (key, value) in stream {
                if let Some(cogroup) = result.get_mut(key) {
                    cogroup.groups[stream_idx].push(value.clone());
                }
            }
        }

        result
    }

    /// Get keys that appear in all streams (intersection)
    pub fn common_keys(&self) -> Vec<K> {
        let mut key_sets: Vec<std::collections::HashSet<K>> = self
            .streams
            .iter()
            .map(|s| s.iter().map(|(k, _)| k.clone()).collect())
            .collect();

        if key_sets.is_empty() {
            return Vec::new();
        }

        let mut result = key_sets.remove(0);
        for set in key_sets {
            result = result.intersection(&set).cloned().collect();
        }

        result.into_iter().collect()
    }

    /// Get all unique keys across all streams (union)
    pub fn all_keys(&self) -> Vec<K> {
        let mut keys = std::collections::HashSet::new();
        for stream in &self.streams {
            for (key, _) in stream {
                keys.insert(key.clone());
            }
        }
        keys.into_iter().collect()
    }
}

/// Key function type for extracting keys from items
pub type KeyFn<T, K> = fn(&T) -> K;

/// Stream co-grouper with key extraction function
pub struct StreamCoGrouper<T, K> {
    streams: Vec<Vec<T>>,
    key_fn: KeyFn<T, K>,
}

impl<T: Clone, K: Eq + std::hash::Hash + Clone> StreamCoGrouper<T, K> {
    pub fn new(key_fn: KeyFn<T, K>) -> Self {
        Self {
            streams: Vec::new(),
            key_fn,
        }
    }

    /// Add a stream with data
    pub fn add_stream(mut self, stream: Vec<T>) -> Self {
        self.streams.push(stream);
        self
    }

    /// Execute co-grouping
    pub fn execute(&self) -> HashMap<K, Vec<Vec<T>>> {
        let mut result: HashMap<K, Vec<Vec<T>>> = HashMap::new();

        // Initialize all keys from all streams
        for stream in &self.streams {
            for item in stream {
                let key = (self.key_fn)(item);
                if !result.contains_key(&key) {
                    result.insert(key, vec![Vec::new(); self.streams.len()]);
                }
            }
        }

        // Group data from each stream
        for (stream_idx, stream) in self.streams.iter().enumerate() {
            for item in stream {
                let key = (self.key_fn)(item);
                if let Some(groups) = result.get_mut(&key) {
                    groups[stream_idx].push(item.clone());
                }
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cogrouper_basic() {
        let stream1 = vec![
            (1, "a"),
            (1, "b"),
            (2, "c"),
        ];
        let stream2 = vec![
            (1, "x"),
            (2, "y"),
            (2, "z"),
        ];

        let result = CoGrouper::new()
            .add_stream(stream1)
            .add_stream(stream2)
            .execute();

        assert_eq!(result.len(), 2);

        let group1 = &result[&1];
        assert_eq!(group1.groups[0], vec!["a", "b"]);
        assert_eq!(group1.groups[1], vec!["x"]);

        let group2 = &result[&2];
        assert_eq!(group2.groups[0], vec!["c"]);
        assert_eq!(group2.groups[1], vec!["y", "z"]);
    }

    #[test]
    fn test_cogrouper_single_stream() {
        let stream = vec![
            (1, "a"),
            (2, "b"),
            (1, "c"),
        ];

        let result = CoGrouper::new()
            .add_stream(stream)
            .execute();

        assert_eq!(result.len(), 2);
        assert_eq!(result[&1].groups[0], vec!["a", "c"]);
        assert_eq!(result[&2].groups[0], vec!["b"]);
    }

    #[test]
    fn test_cogrouper_common_keys() {
        let stream1 = vec![(1, "a"), (2, "b"), (3, "c")];
        let stream2 = vec![(2, "x"), (3, "y"), (4, "z")];

        let common = CoGrouper::new()
            .add_stream(stream1)
            .add_stream(stream2)
            .common_keys();

        assert_eq!(common, vec![2, 3]);
    }

    #[test]
    fn test_cogrouper_all_keys() {
        let stream1 = vec![(1, "a"), (2, "b")];
        let stream2 = vec![(2, "x"), (3, "y")];

        let all = CoGrouper::new()
            .add_stream(stream1)
            .add_stream(stream2)
            .all_keys();

        assert_eq!(all.len(), 3);
        assert!(all.contains(&1));
        assert!(all.contains(&2));
        assert!(all.contains(&3));
    }

    #[test]
    fn test_stream_cogrouper() {
        #[derive(Clone, Debug, PartialEq)]
        struct Item {
            id: i32,
            category: String,
        }

        let stream1 = vec![
            Item { id: 1, category: "fruit".to_string() },
            Item { id: 2, category: "fruit".to_string() },
        ];
        let stream2 = vec![
            Item { id: 3, category: "vegetable".to_string() },
            Item { id: 4, category: "fruit".to_string() },
        ];

        let cogrouper = StreamCoGrouper::new(|item: &Item| item.category.clone())
            .add_stream(stream1)
            .add_stream(stream2);

        let result = cogrouper.execute();

        assert_eq!(result.len(), 2);
        assert_eq!(result["fruit"].len(), 2);
        assert_eq!(result["vegetable"].len(), 2);
    }

    #[test]
    fn test_cogrouper_empty() {
        let result = CoGrouper::<i32, &str>::new().execute();
        assert!(result.is_empty());
    }

    #[test]
    fn test_cogrouper_empty_streams() {
        let stream1: Vec<(i32, &str)> = vec![];
        let stream2 = vec![(1, "a")];

        let result = CoGrouper::new()
            .add_stream(stream1)
            .add_stream(stream2)
            .execute();

        assert_eq!(result.len(), 1);
    }
}
