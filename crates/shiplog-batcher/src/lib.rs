//! Batching utilities for bulk shiplog operations.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::marker::PhantomData;

/// Batch configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchConfig {
    /// Maximum batch size
    pub max_size: usize,
    /// Maximum wait time in milliseconds before flushing
    #[serde(default = "default_flush_timeout")]
    pub flush_timeout_ms: u64,
}

fn default_flush_timeout() -> u64 {
    1000
}

/// Batch item wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchItem<T> {
    pub item: T,
    pub timestamp: u64,
}

/// Batcher for collecting items into batches
#[allow(clippy::type_complexity)]
pub struct Batcher<T> {
    config: BatchConfig,
    items: VecDeque<BatchItem<T>>,
    flush_callback: Option<Box<dyn Fn(Vec<T>) -> anyhow::Result<()> + Send + Sync>>,
}

impl<T: Clone> Batcher<T> {
    /// Create a new batcher
    pub fn new(config: BatchConfig) -> Self {
        Self {
            config,
            items: VecDeque::new(),
            flush_callback: None,
        }
    }

    /// Set the flush callback
    pub fn with_flush_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(Vec<T>) -> anyhow::Result<()> + Send + Sync + 'static,
    {
        self.flush_callback = Some(Box::new(callback));
        self
    }

    /// Add an item to the batch
    pub fn add(&mut self, item: T) -> anyhow::Result<Option<Vec<T>>> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        self.items.push_back(BatchItem {
            item,
            timestamp: now,
        });

        // Check if we should flush
        if self.items.len() >= self.config.max_size {
            return self.flush();
        }

        Ok(None)
    }

    /// Force flush the batch
    pub fn flush(&mut self) -> anyhow::Result<Option<Vec<T>>> {
        if self.items.is_empty() {
            return Ok(None);
        }

        let batch: Vec<T> = self.items.drain(..).map(|i| i.item).collect();

        if let Some(ref callback) = self.flush_callback {
            callback(batch.clone())?;
        }

        Ok(Some(batch))
    }

    /// Get current batch size
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if batcher is empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// Batch processor for handling items in groups
pub struct BatchProcessor<T> {
    config: BatchConfig,
    _phantom: PhantomData<T>,
}

impl<T> BatchProcessor<T> {
    /// Create a new batch processor
    pub fn new(config: BatchConfig) -> Self {
        Self {
            config,
            _phantom: PhantomData,
        }
    }

    /// Process items in batches
    pub fn process<F>(&self, items: &[T], mut processor: F) -> anyhow::Result<usize>
    where
        F: FnMut(&[T]) -> anyhow::Result<()>,
    {
        let mut processed = 0;

        for chunk in items.chunks(self.config.max_size) {
            processor(chunk)?;
            processed += chunk.len();
        }

        Ok(processed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_config_default() {
        let config = BatchConfig {
            max_size: 100,
            flush_timeout_ms: 1000,
        };
        assert_eq!(config.max_size, 100);
        assert_eq!(config.flush_timeout_ms, 1000);
    }

    #[test]
    fn batcher_add_and_flush() -> anyhow::Result<()> {
        let config = BatchConfig {
            max_size: 3,
            flush_timeout_ms: 1000,
        };

        let mut batcher = Batcher::new(config);

        batcher.add("item1".to_string())?;
        batcher.add("item2".to_string())?;

        assert_eq!(batcher.len(), 2);

        let flushed = batcher.flush()?;
        assert!(flushed.is_some());
        let batch = flushed.unwrap();
        assert_eq!(batch.len(), 2);

        assert!(batcher.is_empty());

        Ok(())
    }

    #[test]
    fn batcher_auto_flush() -> anyhow::Result<()> {
        let config = BatchConfig {
            max_size: 2,
            flush_timeout_ms: 1000,
        };

        let mut batcher = Batcher::new(config);

        batcher.add("item1".to_string())?;
        let result = batcher.add("item2".to_string())?;

        // Should auto-flush when max_size is reached
        assert!(result.is_some());
        assert!(batcher.is_empty());

        Ok(())
    }

    #[test]
    fn batch_processor_chunks() -> anyhow::Result<()> {
        let config = BatchConfig {
            max_size: 2,
            flush_timeout_ms: 1000,
        };

        let processor = BatchProcessor::new(config);
        let items = vec!["a", "b", "c", "d", "e"];

        let mut batches: Vec<Vec<&str>> = Vec::new();
        processor.process(&items, |chunk| {
            batches.push(chunk.to_vec());
            Ok(())
        })?;

        assert_eq!(batches.len(), 3); // [a,b], [c,d], [e]
        assert_eq!(batches[0], vec!["a", "b"]);
        assert_eq!(batches[1], vec!["c", "d"]);
        assert_eq!(batches[2], vec!["e"]);

        Ok(())
    }
}
