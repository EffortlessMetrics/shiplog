//! Iterator utilities and extensions for shiplog.
//!
//! This crate provides iterator utilities for processing shiplog data.

use std::collections::HashMap;

/// Configuration for iterator utilities
#[derive(Debug, Clone)]
pub struct IterConfig {
    pub batch_size: usize,
    pub parallel: bool,
    pub buffer_size: usize,
}

impl Default for IterConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            parallel: false,
            buffer_size: 1000,
        }
    }
}

/// Batch iterator configuration
#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub size: usize,
    pub overlap: usize,
}

impl BatchConfig {
    pub fn new(size: usize) -> Self {
        Self { size, overlap: 0 }
    }

    pub fn with_overlap(size: usize, overlap: usize) -> Self {
        Self { size, overlap }
    }
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self::new(100)
    }
}

/// Group items by a key function
pub fn group_by<I, K, F>(iter: I, key_fn: F) -> HashMap<K, Vec<I::Item>>
where
    I: Iterator,
    K: std::hash::Hash + Eq,
    F: Fn(&I::Item) -> K,
{
    let mut groups: HashMap<K, Vec<I::Item>> = HashMap::new();
    for item in iter {
        let key = key_fn(&item);
        groups.entry(key).or_insert_with(Vec::new).push(item);
    }
    groups
}

/// Flatten nested iterators
pub fn flatten_opt<I, T>(iter: I) -> impl Iterator<Item = T>
where
    I: Iterator<Item = Option<T>>,
{
    iter.flatten()
}

/// Collect items into chunks
pub fn chunk<I>(iter: I, size: usize) -> Vec<Vec<I::Item>>
where
    I: Iterator,
    I::Item: Clone,
{
    iter.collect::<Vec<_>>()
        .chunks(size)
        .map(|c| c.to_vec())
        .collect()
}

/// Window iterator for sliding window operations
pub struct WindowIter<I: Iterator> {
    iter: I,
    window_size: usize,
    buffer: Vec<I::Item>,
}

impl<I: Iterator> WindowIter<I>
where
    I::Item: Clone,
{
    pub fn new(iter: I, window_size: usize) -> Self {
        Self {
            iter,
            window_size,
            buffer: Vec::with_capacity(window_size),
        }
    }
}

impl<I: Iterator> Iterator for WindowIter<I>
where
    I::Item: Clone,
{
    type Item = Vec<I::Item>;

    fn next(&mut self) -> Option<Self::Item> {
        // If buffer is not full yet, try to fill it
        if self.buffer.len() < self.window_size {
            while self.buffer.len() < self.window_size {
                match self.iter.next() {
                    Some(item) => self.buffer.push(item),
                    None => break,
                }
            }
        } else {
            // Buffer is full, slide the window
            // Remove the first element
            self.buffer.remove(0);
            // Add the next element if available
            match self.iter.next() {
                Some(item) => self.buffer.push(item),
                None => {
                    // No more items, we're done
                    return None;
                }
            }
        }

        // Return the window if we have enough elements
        if self.buffer.len() == self.window_size {
            Some(self.buffer.clone())
        } else {
            None
        }
    }
}

/// Create a window iterator with sliding window
pub fn window<I>(iter: I, size: usize) -> WindowIter<I>
where
    I: Iterator,
    I::Item: Clone,
{
    WindowIter::new(iter, size)
}

/// Count iterator items efficiently
pub fn count<I>(iter: I) -> usize
where
    I: Iterator,
{
    iter.count()
}

/// Partition iterator by predicate
pub fn partition<I, F>(iter: I, predicate: F) -> (Vec<I::Item>, Vec<I::Item>)
where
    I: Iterator,
    F: Fn(&I::Item) -> bool,
{
    iter.fold((Vec::new(), Vec::new()), |(mut yes, mut no), item| {
        if predicate(&item) {
            yes.push(item);
        } else {
            no.push(item);
        }
        (yes, no)
    })
}

/// Unique iterator adapter using hash
pub struct UniqueIter<I: Iterator, H> {
    iter: I,
    seen: std::collections::HashSet<H>,
    hash_fn: fn(&I::Item) -> H,
}

impl<I: Iterator, H: std::hash::Hash + Eq> Iterator for UniqueIter<I, H> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        for item in self.iter.by_ref() {
            let hash = (self.hash_fn)(&item);
            if self.seen.insert(hash) {
                return Some(item);
            }
        }
        None
    }
}

/// Create unique iterator adapter
pub fn unique<I, H>(iter: I, hash_fn: fn(&I::Item) -> H) -> UniqueIter<I, H>
where
    I: Iterator,
    H: std::hash::Hash + Eq,
{
    UniqueIter {
        iter,
        seen: std::collections::HashSet::new(),
        hash_fn,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iter_config_default() {
        let config = IterConfig::default();
        assert_eq!(config.batch_size, 100);
        assert!(!config.parallel);
        assert_eq!(config.buffer_size, 1000);
    }

    #[test]
    fn test_batch_config() {
        let config = BatchConfig::new(50);
        assert_eq!(config.size, 50);
        assert_eq!(config.overlap, 0);

        let config_overlap = BatchConfig::with_overlap(50, 10);
        assert_eq!(config_overlap.size, 50);
        assert_eq!(config_overlap.overlap, 10);
    }

    #[test]
    fn test_group_by() {
        let items = vec![1, 2, 3, 4, 5, 6];
        let groups = group_by(items.into_iter(), |&x| x % 2);

        assert_eq!(groups.get(&0), Some(&vec![2, 4, 6]));
        assert_eq!(groups.get(&1), Some(&vec![1, 3, 5]));
    }

    #[test]
    fn test_flatten_opt() {
        let items = vec![Some(1), None, Some(2), Some(3), None];
        let result: Vec<_> = flatten_opt(items.into_iter()).collect();
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_chunk() {
        let items = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
        let chunks = chunk(items.into_iter(), 3);

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0], vec![1, 2, 3]);
        assert_eq!(chunks[1], vec![4, 5, 6]);
        assert_eq!(chunks[2], vec![7, 8, 9]);
    }

    #[test]
    fn test_chunk_remainder() {
        let items = vec![1, 2, 3, 4, 5];
        let chunks = chunk(items.into_iter(), 2);

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0], vec![1, 2]);
        assert_eq!(chunks[1], vec![3, 4]);
        assert_eq!(chunks[2], vec![5]);
    }

    #[test]
    fn test_window() {
        let items = vec![1, 2, 3, 4, 5];
        let windows: Vec<_> = window(items.into_iter(), 3).collect();

        assert_eq!(windows.len(), 3);
        assert_eq!(windows[0], vec![1, 2, 3]);
        assert_eq!(windows[1], vec![2, 3, 4]);
        assert_eq!(windows[2], vec![3, 4, 5]);
    }

    #[test]
    fn test_partition() {
        let items = vec![1, 2, 3, 4, 5, 6];
        let (even, odd) = partition(items.into_iter(), |&x| x % 2 == 0);

        assert_eq!(even, vec![2, 4, 6]);
        assert_eq!(odd, vec![1, 3, 5]);
    }

    #[test]
    fn test_unique() {
        let items = vec![1, 2, 2, 3, 3, 3, 4];
        let unique_items: Vec<_> = unique(items.into_iter(), |&x| x).collect();

        assert_eq!(unique_items, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_count() {
        let items = vec![1, 2, 3, 4, 5];
        assert_eq!(count(items.into_iter()), 5);

        let empty: Vec<i32> = vec![];
        assert_eq!(count(empty.into_iter()), 0);
    }
}
