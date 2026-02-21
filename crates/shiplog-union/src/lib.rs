//! Union operations for combining streams in shiplog.
//!
//! This crate provides union implementations for combining multiple streams
//! of data with various merge strategies.

use std::collections::HashSet;

/// Union operation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnionMode {
    /// Include all elements (allow duplicates)
    All,
    /// Include only unique elements
    Distinct,
    /// Keep first occurrence only
    KeepFirst,
    /// Keep last occurrence only
    KeepLast,
}

/// Stream union combinator
pub struct StreamUnion<T> {
    streams: Vec<Vec<T>>,
    mode: UnionMode,
}

impl<T: Clone + Eq + std::hash::Hash> StreamUnion<T> {
    pub fn new() -> Self {
        Self {
            streams: Vec::new(),
            mode: UnionMode::All,
        }
    }

    /// Add a stream to the union
    pub fn add_stream(mut self, stream: Vec<T>) -> Self {
        self.streams.push(stream);
        self
    }

    /// Set the union mode
    pub fn with_mode(mut self, mode: UnionMode) -> Self {
        self.mode = mode;
        self
    }

    /// Execute the union operation
    pub fn execute(&self) -> Vec<T> {
        match self.mode {
            UnionMode::All => {
                let mut result = Vec::new();
                for stream in &self.streams {
                    result.extend(stream.clone());
                }
                result
            }
            UnionMode::Distinct => {
                let mut seen = HashSet::new();
                let mut result = Vec::new();
                for stream in &self.streams {
                    for item in stream {
                        if seen.insert(item.clone()) {
                            result.push(item.clone());
                        }
                    }
                }
                result
            }
            UnionMode::KeepFirst => {
                let mut seen = HashSet::new();
                let mut result = Vec::new();
                for stream in &self.streams {
                    for item in stream {
                        if !seen.contains(item) {
                            seen.insert(item.clone());
                            result.push(item.clone());
                        }
                    }
                }
                result
            }
            UnionMode::KeepLast => {
                // For KeepLast, we process in reverse and keep first in reverse
                let mut seen = HashSet::new();
                let mut result = Vec::new();
                for stream in self.streams.iter().rev() {
                    for item in stream.iter().rev() {
                        if !seen.contains(item) {
                            seen.insert(item.clone());
                            result.push(item.clone());
                        }
                    }
                }
                result.reverse();
                result
            }
        }
    }
}

/// Merged stream result with metadata
#[derive(Debug, Clone)]
pub struct MergedStream<T> {
    pub items: Vec<T>,
    pub source_counts: Vec<usize>,
}

impl<T: Clone + Eq + std::hash::Hash> StreamUnion<T> {
    /// Execute union with metadata about source counts
    pub fn execute_with_counts(&self) -> MergedStream<T> {
        let items = self.execute();
        let source_counts: Vec<usize> = self
            .streams
            .iter()
            .map(|s| s.len())
            .collect();

        MergedStream {
            items,
            source_counts,
        }
    }
}

/// Interleaved stream merger - merges streams in round-robin fashion
pub struct InterleavedMerger<T> {
    streams: Vec<Vec<T>>,
}

impl<T: Clone> InterleavedMerger<T> {
    pub fn new() -> Self {
        Self {
            streams: Vec::new(),
        }
    }

    pub fn add_stream(mut self, stream: Vec<T>) -> Self {
        self.streams.push(stream);
        self
    }

    /// Execute interleaved merge
    pub fn execute(&self) -> Vec<T> {
        let mut result = Vec::new();
        let max_len = self.streams.iter().map(|s| s.len()).max().unwrap_or(0);

        for i in 0..max_len {
            for stream in &self.streams {
                if i < stream.len() {
                    result.push(stream[i].clone());
                }
            }
        }

        result
    }
}

/// Chained merger - appends streams sequentially
pub struct ChainedMerger<T> {
    streams: Vec<Vec<T>>,
}

impl<T: Clone> ChainedMerger<T> {
    pub fn new() -> Self {
        Self {
            streams: Vec::new(),
        }
    }

    pub fn add_stream(mut self, stream: Vec<T>) -> Self {
        self.streams.push(stream);
        self
    }

    /// Execute chained merge
    pub fn execute(&self) -> Vec<T> {
        let mut result = Vec::new();
        for stream in &self.streams {
            result.extend(stream.clone());
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_union_all() {
        let stream1 = vec![1, 2, 3];
        let stream2 = vec![3, 4, 5];
        let stream3 = vec![5, 6, 7];

        let result = StreamUnion::new()
            .add_stream(stream1)
            .add_stream(stream2)
            .add_stream(stream3)
            .with_mode(UnionMode::All)
            .execute();

        assert_eq!(result, vec![1, 2, 3, 3, 4, 5, 5, 6, 7]);
    }

    #[test]
    fn test_union_distinct() {
        let stream1 = vec![1, 2, 3];
        let stream2 = vec![3, 4, 5];
        let stream3 = vec![5, 6, 7];

        let result = StreamUnion::new()
            .add_stream(stream1)
            .add_stream(stream2)
            .add_stream(stream3)
            .with_mode(UnionMode::Distinct)
            .execute();

        assert_eq!(result.len(), 7);
        let set: HashSet<_> = result.iter().cloned().collect();
        assert_eq!(set.len(), 7);
    }

    #[test]
    fn test_union_keep_first() {
        let stream1 = vec![1, 2, 3];
        let stream2 = vec![1, 4, 5];
        let stream3 = vec![1, 6, 7];

        let result = StreamUnion::new()
            .add_stream(stream1)
            .add_stream(stream2)
            .add_stream(stream3)
            .with_mode(UnionMode::KeepFirst)
            .execute();

        assert_eq!(result.len(), 7);
        // First occurrence of 1 is from stream1
        assert_eq!(result[0], 1);
    }

    #[test]
    fn test_union_keep_last() {
        let stream1 = vec![1, 2, 3];
        let stream2 = vec![1, 4, 5];
        let stream3 = vec![1, 6, 7];

        let result = StreamUnion::new()
            .add_stream(stream1)
            .add_stream(stream2)
            .add_stream(stream3)
            .with_mode(UnionMode::KeepLast)
            .execute();

        assert_eq!(result.len(), 7);
        // Last occurrence of 1 is from stream3
        assert_eq!(result.iter().filter(|&&x| x == 1).count(), 1);
    }

    #[test]
    fn test_interleaved_merger() {
        let stream1 = vec![1, 2, 3];
        let stream2 = vec![4, 5, 6];
        let stream3 = vec![7, 8, 9];

        let result = InterleavedMerger::new()
            .add_stream(stream1)
            .add_stream(stream2)
            .add_stream(stream3)
            .execute();

        assert_eq!(result, vec![1, 4, 7, 2, 5, 8, 3, 6, 9]);
    }

    #[test]
    fn test_interleaved_merger_uneven() {
        let stream1 = vec![1, 2];
        let stream2 = vec![3, 4, 5, 6];

        let result = InterleavedMerger::new()
            .add_stream(stream1)
            .add_stream(stream2)
            .execute();

        assert_eq!(result, vec![1, 3, 2, 4, 5, 6]);
    }

    #[test]
    fn test_chained_merger() {
        let stream1 = vec![1, 2, 3];
        let stream2 = vec![4, 5, 6];
        let stream3 = vec![7, 8, 9];

        let result = ChainedMerger::new()
            .add_stream(stream1)
            .add_stream(stream2)
            .add_stream(stream3)
            .execute();

        assert_eq!(result, vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn test_union_with_counts() {
        let stream1 = vec![1, 2];
        let stream2 = vec![3, 4, 5];

        let result = StreamUnion::new()
            .add_stream(stream1)
            .add_stream(stream2)
            .execute_with_counts();

        assert_eq!(result.items.len(), 5);
        assert_eq!(result.source_counts, vec![2, 3]);
    }

    #[test]
    fn test_union_empty_streams() {
        let result = StreamUnion::<i32>::new()
            .add_stream(vec![])
            .add_stream(vec![])
            .execute();

        assert!(result.is_empty());
    }

    #[test]
    fn test_union_single_stream() {
        let stream = vec![1, 2, 3, 2, 1];

        let result = StreamUnion::new()
            .add_stream(stream)
            .with_mode(UnionMode::Distinct)
            .execute();

        assert_eq!(result.len(), 3);
    }
}
