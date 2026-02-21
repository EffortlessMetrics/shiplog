//! Iterator utilities for shiplog.
//!
//! This crate provides iterator utilities and adapters for data processing.

/// Configuration for iterator processing
#[derive(Debug, Clone)]
pub struct IteratorConfig {
    pub batch_size: usize,
    pub buffer_size: usize,
}

impl Default for IteratorConfig {
    fn default() -> Self {
        Self {
            batch_size: 10,
            buffer_size: 100,
        }
    }
}

/// Builder for creating iterator configurations
#[derive(Debug)]
pub struct IteratorBuilder {
    config: IteratorConfig,
}

impl IteratorBuilder {
    pub fn new() -> Self {
        Self {
            config: IteratorConfig::default(),
        }
    }

    pub fn batch_size(mut self, size: usize) -> Self {
        self.config.batch_size = size;
        self
    }

    pub fn buffer_size(mut self, size: usize) -> Self {
        self.config.buffer_size = size;
        self
    }

    pub fn build(self) -> IteratorConfig {
        self.config
    }
}

impl Default for IteratorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Iterator metrics
#[derive(Debug, Default, Clone)]
pub struct IteratorMetrics {
    pub items_yielded: u64,
    pub items_skipped: u64,
}

impl IteratorMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_yield(&mut self) {
        self.items_yielded += 1;
    }

    pub fn record_skip(&mut self) {
        self.items_skipped += 1;
    }
}

/// An iterator that tracks metrics
pub struct Metered<I> {
    iter: I,
    metrics: IteratorMetrics,
}

impl<I> Metered<I> {
    pub fn new(iter: I) -> Self {
        Self {
            iter,
            metrics: IteratorMetrics::new(),
        }
    }

    pub fn metrics(&self) -> &IteratorMetrics {
        &self.metrics
    }
}

impl<I: Iterator> Iterator for Metered<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        match self.iter.next() {
            item @ Some(_) => {
                self.metrics.record_yield();
                item
            }
            None => None,
        }
    }
}

/// An iterator that skips items based on a predicate
pub struct SkipWhile<I, P> {
    iter: I,
    predicate: P,
    skipping: bool,
}

impl<I, P> SkipWhile<I, P> {
    pub fn new(iter: I, predicate: P) -> Self {
        Self {
            iter,
            predicate,
            skipping: true,
        }
    }
}

impl<I: Iterator, P: FnMut(&I::Item) -> bool> Iterator for SkipWhile<I, P> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.iter.next() {
                Some(item) => {
                    if self.skipping && (self.predicate)(&item) {
                        continue;
                    }
                    self.skipping = false;
                    return Some(item);
                }
                None => return None,
            }
        }
    }
}

/// An iterator that takes items based on a predicate
pub struct TakeWhile<I, P> {
    iter: I,
    predicate: P,
}

impl<I, P> TakeWhile<I, P> {
    pub fn new(iter: I, predicate: P) -> Self {
        Self { iter, predicate }
    }
}

impl<I: Iterator, P: FnMut(&I::Item) -> bool> Iterator for TakeWhile<I, P> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        match self.iter.next() {
            Some(item) if (self.predicate)(&item) => Some(item),
            _ => None,
        }
    }
}

/// Extension trait for iterators
pub trait IteratorExt: Iterator + Sized {
    fn metered(self) -> Metered<Self>;
    fn skip_while_item<P>(self, predicate: P) -> SkipWhile<Self, P>;
    fn take_while_item<P>(self, predicate: P) -> TakeWhile<Self, P>;
}

impl<I: Iterator + Sized> IteratorExt for I {
    fn metered(self) -> Metered<Self> {
        Metered::new(self)
    }

    fn skip_while_item<P>(self, predicate: P) -> SkipWhile<Self, P> {
        SkipWhile::new(self, predicate)
    }

    fn take_while_item<P>(self, predicate: P) -> TakeWhile<Self, P> {
        TakeWhile::new(self, predicate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iterator_config_default() {
        let config = IteratorConfig::default();
        assert_eq!(config.batch_size, 10);
        assert_eq!(config.buffer_size, 100);
    }

    #[test]
    fn test_iterator_builder() {
        let config = IteratorBuilder::new()
            .batch_size(20)
            .buffer_size(200)
            .build();
        
        assert_eq!(config.batch_size, 20);
        assert_eq!(config.buffer_size, 200);
    }

    #[test]
    fn test_iterator_metrics() {
        let mut metrics = IteratorMetrics::new();
        assert_eq!(metrics.items_yielded, 0);
        
        metrics.record_yield();
        assert_eq!(metrics.items_yielded, 1);
        
        metrics.record_skip();
        assert_eq!(metrics.items_skipped, 1);
    }

    #[test]
    fn test_metered_iterator() {
        let mut iter = vec![1, 2, 3].into_iter().metered();
        
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.next(), None);
        
        assert_eq!(iter.metrics().items_yielded, 3);
    }

    #[test]
    fn test_skip_while() {
        let iter = vec![1, 2, 3, 4, 5].into_iter().skip_while_item(|x: &i32| *x < 3);
        let collected: Vec<_> = iter.collect();
        
        assert_eq!(collected, vec![3, 4, 5]);
    }

    #[test]
    fn test_take_while() {
        let iter = vec![1, 2, 3, 4, 5].into_iter().take_while_item(|x: &i32| *x < 3);
        let collected: Vec<_> = iter.collect();
        
        assert_eq!(collected, vec![1, 2]);
    }
}
