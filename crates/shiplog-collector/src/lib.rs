//! Collector utilities for shiplog.
//!
//! This crate provides collector implementations for gathering and processing data.

use std::collections::VecDeque;

/// Configuration for collector behavior
#[derive(Debug, Clone)]
pub struct CollectorConfig {
    pub batch_size: usize,
    pub flush_interval_ms: u64,
    pub name: String,
}

impl Default for CollectorConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            flush_interval_ms: 1000,
            name: "collector".to_string(),
        }
    }
}

/// A collector that gathers items and flushes them in batches
pub struct Collector<T> {
    items: VecDeque<T>,
    batch_size: usize,
    count: usize,
    name: String,
}

impl<T> Collector<T> {
    pub fn new(batch_size: usize) -> Self {
        Self {
            items: VecDeque::new(),
            batch_size,
            count: 0,
            name: "collector".to_string(),
        }
    }

    pub fn with_config(config: &CollectorConfig) -> Self {
        Self {
            items: VecDeque::new(),
            batch_size: config.batch_size,
            count: 0,
            name: config.name.clone(),
        }
    }

    pub fn push(&mut self, item: T) {
        self.items.push_back(item);
        self.count += 1;
    }

    pub fn push_batch(&mut self, batch: Vec<T>) {
        for item in batch {
            self.push(item);
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn is_batch_ready(&self) -> bool {
        self.items.len() >= self.batch_size
    }

    pub fn drain_batch(&mut self) -> Vec<T> {
        let mut batch = Vec::with_capacity(self.batch_size);
        for _ in 0..self.batch_size {
            if let Some(item) = self.items.pop_front() {
                batch.push(item);
            } else {
                break;
            }
        }
        batch
    }

    pub fn drain_all(&mut self) -> Vec<T> {
        let mut items = Vec::new();
        while let Some(item) = self.items.pop_front() {
            items.push(item);
        }
        self.count = 0;
        items
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn batch_size(&self) -> usize {
        self.batch_size
    }
}

/// A simple collector that collects items until a predicate is satisfied
pub struct ConditionalCollector<T> {
    items: Vec<T>,
    predicate: Box<dyn Fn(&[T]) -> bool>,
}

impl<T> ConditionalCollector<T> {
    pub fn new<P>(predicate: P) -> Self
    where
        P: Fn(&[T]) -> bool + 'static,
    {
        Self {
            items: Vec::new(),
            predicate: Box::new(predicate),
        }
    }

    pub fn push(&mut self, item: T) -> bool {
        self.items.push(item);
        (self.predicate)(&self.items)
    }

    pub fn collect(&self) -> &[T] {
        &self.items
    }

    pub fn into_inner(self) -> Vec<T> {
        self.items
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collector_basic() {
        let mut collector: Collector<i32> = Collector::new(3);
        
        assert!(collector.is_empty());
        
        collector.push(1);
        collector.push(2);
        
        assert_eq!(collector.len(), 2);
    }

    #[test]
    fn test_collector_batch_ready() {
        let mut collector: Collector<i32> = Collector::new(3);
        
        collector.push(1);
        collector.push(2);
        
        assert!(!collector.is_batch_ready());
        
        collector.push(3);
        
        assert!(collector.is_batch_ready());
    }

    #[test]
    fn test_collector_drain_batch() {
        let mut collector: Collector<i32> = Collector::new(3);
        
        collector.push(1);
        collector.push(2);
        collector.push(3);
        collector.push(4);
        collector.push(5);
        
        let batch = collector.drain_batch();
        
        assert_eq!(batch, vec![1, 2, 3]);
        assert_eq!(collector.len(), 2); // 4 and 5 remain
    }

    #[test]
    fn test_collector_drain_all() {
        let mut collector: Collector<i32> = Collector::new(3);
        
        collector.push(1);
        collector.push(2);
        collector.push(3);
        
        let all = collector.drain_all();
        
        assert_eq!(all, vec![1, 2, 3]);
        assert!(collector.is_empty());
    }

    #[test]
    fn test_collector_with_config() {
        let config = CollectorConfig {
            batch_size: 50,
            flush_interval_ms: 500,
            name: "test-collector".to_string(),
        };
        
        let collector: Collector<i32> = Collector::with_config(&config);
        
        assert_eq!(collector.batch_size(), 50);
        assert_eq!(collector.name(), "test-collector");
    }

    #[test]
    fn test_conditional_collector() {
        let mut collector = ConditionalCollector::new(|items| items.len() >= 3);
        
        assert!(!collector.push(1));
        assert!(!collector.push(2));
        assert!(collector.push(3)); // Now we have 3 items
        
        assert_eq!(collector.len(), 3);
        assert_eq!(collector.collect(), &vec![1, 2, 3]);
    }

    #[test]
    fn test_conditional_collector_into_inner() {
        let collector = ConditionalCollector::new(|_| true);
        let mut c = collector;
        c.push(1);
        c.push(2);
        
        let items = c.into_inner();
        
        assert_eq!(items, vec![1, 2]);
    }
}
