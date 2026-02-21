//! Stream utilities for shiplog.
//!
//! This crate provides stream utilities for async data processing.

use futures::stream::{self, Stream, StreamExt};
use std::pin::Pin;
use std::task::{Context, Poll};

/// Configuration for stream processing
#[derive(Debug, Clone)]
pub struct StreamConfig {
    pub buffer_size: usize,
    pub batch_size: usize,
    pub name: String,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            buffer_size: 100,
            batch_size: 10,
            name: "stream".to_string(),
        }
    }
}

/// Builder for creating stream configurations
#[derive(Debug)]
pub struct StreamBuilder {
    config: StreamConfig,
}

impl StreamBuilder {
    pub fn new() -> Self {
        Self {
            config: StreamConfig::default(),
        }
    }

    pub fn buffer_size(mut self, size: usize) -> Self {
        self.config.buffer_size = size;
        self
    }

    pub fn batch_size(mut self, size: usize) -> Self {
        self.config.batch_size = size;
        self
    }

    pub fn name(mut self, name: &str) -> Self {
        self.config.name = name.to_string();
        self
    }

    pub fn build(self) -> StreamConfig {
        self.config
    }
}

impl Default for StreamBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Stream processor for handling items
pub struct StreamProcessor<T> {
    config: StreamConfig,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> StreamProcessor<T> {
    pub fn new(config: StreamConfig) -> Self {
        Self {
            config,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn config(&self) -> &StreamConfig {
        &self.config
    }
}

/// Count stream items
pub async fn count_stream<S>(stream: S) -> usize
where
    S: StreamExt + Unpin,
{
    stream.count().await
}

/// Collect stream items into a vector
pub async fn collect_stream<S>(stream: S) -> Vec<S::Item>
where
    S: StreamExt + Unpin,
{
    stream.collect().await
}

/// Take first n items from stream
pub fn take_stream<S>(stream: S, n: usize) -> impl Stream<Item = S::Item>
where
    S: Stream,
{
    stream.take(n)
}

/// Skip first n items from stream
pub fn skip_stream<S>(stream: S, n: usize) -> impl Stream<Item = S::Item>
where
    S: Stream,
{
    stream.skip(n)
}

/// Stream metrics
#[derive(Debug, Default, Clone)]
pub struct StreamMetrics {
    pub items_processed: u64,
    pub items_filtered: u64,
    pub errors: u64,
}

impl StreamMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_processed(&mut self) {
        self.items_processed += 1;
    }

    pub fn record_filtered(&mut self) {
        self.items_filtered += 1;
    }

    pub fn record_error(&mut self) {
        self.errors += 1;
    }
}

/// A stream wrapper that tracks metrics
pub struct MeteredStream<S> {
    stream: S,
    metrics: StreamMetrics,
}

impl<S> MeteredStream<S> {
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            metrics: StreamMetrics::new(),
        }
    }

    pub fn metrics(&self) -> &StreamMetrics {
        &self.metrics
    }

    pub fn metrics_mut(&mut self) -> &mut StreamMetrics {
        &mut self.metrics
    }
}

impl<S: Stream + Unpin> Stream for MeteredStream<S> {
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let item = Pin::new(&mut self.stream).poll_next(cx);
        if let Poll::Ready(Some(_)) = item {
            self.metrics.record_processed();
        }
        item
    }
}

/// Create an iterator stream from an iterable
pub fn iter_stream<T: Send + Sync + 'static>(iter: impl IntoIterator<Item = T>) -> impl Stream<Item = T> {
    stream::iter(iter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_config_default() {
        let config = StreamConfig::default();
        assert_eq!(config.buffer_size, 100);
        assert_eq!(config.batch_size, 10);
        assert_eq!(config.name, "stream");
    }

    #[test]
    fn test_stream_builder() {
        let config = StreamBuilder::new()
            .buffer_size(200)
            .batch_size(20)
            .name("test-stream")
            .build();
        
        assert_eq!(config.buffer_size, 200);
        assert_eq!(config.batch_size, 20);
        assert_eq!(config.name, "test-stream");
    }

    #[test]
    fn test_stream_processor() {
        let processor: StreamProcessor<i32> = StreamProcessor::new(StreamConfig::default());
        assert_eq!(processor.config().name, "stream");
    }

    #[test]
    fn test_stream_metrics() {
        let mut metrics = StreamMetrics::new();
        assert_eq!(metrics.items_processed, 0);
        
        metrics.record_processed();
        assert_eq!(metrics.items_processed, 1);
        
        metrics.record_filtered();
        assert_eq!(metrics.items_filtered, 1);
        
        metrics.record_error();
        assert_eq!(metrics.errors, 1);
    }

    #[tokio::test]
    async fn test_count_stream() {
        let s = stream::iter(vec![1, 2, 3, 4, 5]);
        let count = count_stream(s).await;
        assert_eq!(count, 5);
    }

    #[tokio::test]
    async fn test_collect_stream() {
        let s = stream::iter(vec![1, 2, 3]);
        let collected = collect_stream(s).await;
        assert_eq!(collected, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_metered_stream() {
        let s = stream::iter(vec![1, 2, 3]);
        let mut metered = MeteredStream::new(s);
        
        while let Some(_item) = metered.next().await {
            // Process items
        }
        
        assert_eq!(metered.metrics().items_processed, 3);
    }
}
