use futures::stream::{self, StreamExt};
use shiplog_stream::{
    MeteredStream, StreamBuilder, StreamConfig, StreamMetrics, StreamProcessor, collect_stream,
    count_stream, iter_stream, skip_stream, take_stream,
};

// ── StreamConfig ──────────────────────────────────────────────────

#[test]
fn config_default_values() {
    let cfg = StreamConfig::default();
    assert_eq!(cfg.buffer_size, 100);
    assert_eq!(cfg.batch_size, 10);
    assert_eq!(cfg.name, "stream");
}

#[test]
fn config_clone_preserves_values() {
    let cfg = StreamConfig {
        buffer_size: 42,
        batch_size: 7,
        name: "cloned".into(),
    };
    let c2 = cfg.clone();
    assert_eq!(c2.buffer_size, 42);
    assert_eq!(c2.batch_size, 7);
    assert_eq!(c2.name, "cloned");
}

// ── StreamBuilder ─────────────────────────────────────────────────

#[test]
fn builder_defaults_match_config_defaults() {
    let built = StreamBuilder::new().build();
    let def = StreamConfig::default();
    assert_eq!(built.buffer_size, def.buffer_size);
    assert_eq!(built.batch_size, def.batch_size);
    assert_eq!(built.name, def.name);
}

#[test]
fn builder_overrides() {
    let cfg = StreamBuilder::new()
        .buffer_size(500)
        .batch_size(50)
        .name("custom")
        .build();
    assert_eq!(cfg.buffer_size, 500);
    assert_eq!(cfg.batch_size, 50);
    assert_eq!(cfg.name, "custom");
}

#[test]
fn builder_default_trait() {
    let b: StreamBuilder = Default::default();
    let cfg = b.build();
    assert_eq!(cfg.buffer_size, 100);
}

#[test]
fn builder_zero_sizes() {
    let cfg = StreamBuilder::new().buffer_size(0).batch_size(0).build();
    assert_eq!(cfg.buffer_size, 0);
    assert_eq!(cfg.batch_size, 0);
}

#[test]
fn builder_empty_name() {
    let cfg = StreamBuilder::new().name("").build();
    assert_eq!(cfg.name, "");
}

#[test]
fn builder_special_chars_in_name() {
    let cfg = StreamBuilder::new()
        .name("stream/with spaces & 日本語")
        .build();
    assert_eq!(cfg.name, "stream/with spaces & 日本語");
}

// ── StreamProcessor ───────────────────────────────────────────────

#[test]
fn processor_config_ref() {
    let cfg = StreamBuilder::new().name("proc").build();
    let proc: StreamProcessor<String> = StreamProcessor::new(cfg);
    assert_eq!(proc.config().name, "proc");
}

// ── StreamMetrics ─────────────────────────────────────────────────

#[test]
fn metrics_new_is_zero() {
    let m = StreamMetrics::new();
    assert_eq!(m.items_processed, 0);
    assert_eq!(m.items_filtered, 0);
    assert_eq!(m.errors, 0);
}

#[test]
fn metrics_default_is_zero() {
    let m = StreamMetrics::default();
    assert_eq!(m.items_processed, 0);
}

#[test]
fn metrics_increments() {
    let mut m = StreamMetrics::new();
    for _ in 0..100 {
        m.record_processed();
    }
    for _ in 0..30 {
        m.record_filtered();
    }
    for _ in 0..5 {
        m.record_error();
    }
    assert_eq!(m.items_processed, 100);
    assert_eq!(m.items_filtered, 30);
    assert_eq!(m.errors, 5);
}

#[test]
fn metrics_clone() {
    let mut m = StreamMetrics::new();
    m.record_processed();
    let m2 = m.clone();
    assert_eq!(m2.items_processed, 1);
}

// ── Async helpers ─────────────────────────────────────────────────

#[tokio::test]
async fn count_empty_stream() {
    let s = stream::iter(Vec::<i32>::new());
    assert_eq!(count_stream(s).await, 0);
}

#[tokio::test]
async fn count_large_stream() {
    let s = stream::iter(0..10_000);
    assert_eq!(count_stream(s).await, 10_000);
}

#[tokio::test]
async fn collect_empty_stream() {
    let s = stream::iter(Vec::<i32>::new());
    let v: Vec<i32> = collect_stream(s).await;
    assert!(v.is_empty());
}

#[tokio::test]
async fn collect_preserves_order() {
    let s = stream::iter(vec![3, 1, 4, 1, 5]);
    assert_eq!(collect_stream(s).await, vec![3, 1, 4, 1, 5]);
}

#[tokio::test]
async fn collect_string_items() {
    let items: Vec<String> = vec!["hello".into(), "世界".into(), "".into()];
    let s = stream::iter(items.clone());
    assert_eq!(collect_stream(s).await, items);
}

#[tokio::test]
async fn take_fewer_than_available() {
    let s = stream::iter(0..10);
    let taken: Vec<_> = take_stream(s, 3).collect().await;
    assert_eq!(taken, vec![0, 1, 2]);
}

#[tokio::test]
async fn take_more_than_available() {
    let s = stream::iter(vec![1, 2]);
    let taken: Vec<_> = take_stream(s, 100).collect().await;
    assert_eq!(taken, vec![1, 2]);
}

#[tokio::test]
async fn take_zero() {
    let s = stream::iter(vec![1, 2, 3]);
    let taken: Vec<_> = take_stream(s, 0).collect().await;
    assert!(taken.is_empty());
}

#[tokio::test]
async fn skip_fewer_than_available() {
    let s = stream::iter(0..5);
    let rest: Vec<_> = skip_stream(s, 2).collect().await;
    assert_eq!(rest, vec![2, 3, 4]);
}

#[tokio::test]
async fn skip_all() {
    let s = stream::iter(vec![1, 2, 3]);
    let rest: Vec<_> = skip_stream(s, 3).collect().await;
    assert!(rest.is_empty());
}

#[tokio::test]
async fn skip_more_than_available() {
    let s = stream::iter(vec![1]);
    let rest: Vec<_> = skip_stream(s, 100).collect().await;
    assert!(rest.is_empty());
}

#[tokio::test]
async fn skip_zero() {
    let s = stream::iter(vec![1, 2, 3]);
    let rest: Vec<_> = skip_stream(s, 0).collect().await;
    assert_eq!(rest, vec![1, 2, 3]);
}

// ── iter_stream ───────────────────────────────────────────────────

#[tokio::test]
async fn iter_stream_from_vec() {
    let s = iter_stream(vec![10, 20, 30]);
    let v: Vec<_> = s.collect().await;
    assert_eq!(v, vec![10, 20, 30]);
}

#[tokio::test]
async fn iter_stream_empty() {
    let s = iter_stream(Vec::<u8>::new());
    let v: Vec<_> = s.collect().await;
    assert!(v.is_empty());
}

// ── MeteredStream ─────────────────────────────────────────────────

#[tokio::test]
async fn metered_stream_counts_all_items() {
    let s = stream::iter(vec![1, 2, 3, 4, 5]);
    let mut m = MeteredStream::new(s);
    while m.next().await.is_some() {}
    assert_eq!(m.metrics().items_processed, 5);
}

#[tokio::test]
async fn metered_stream_empty() {
    let s = stream::iter(Vec::<i32>::new());
    let mut m = MeteredStream::new(s);
    while m.next().await.is_some() {}
    assert_eq!(m.metrics().items_processed, 0);
}

#[tokio::test]
async fn metered_stream_partial_consumption() {
    let s = stream::iter(0..10);
    let mut m = MeteredStream::new(s);
    // Only consume 3 items
    m.next().await;
    m.next().await;
    m.next().await;
    assert_eq!(m.metrics().items_processed, 3);
}

#[tokio::test]
async fn metered_stream_metrics_mut() {
    let s = stream::iter(vec![1]);
    let mut m = MeteredStream::new(s);
    m.metrics_mut().record_error();
    assert_eq!(m.metrics().errors, 1);
}

// ── Property tests ────────────────────────────────────────────────

mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn builder_roundtrip_buffer_size(size in 0..10_000usize) {
            let cfg = StreamBuilder::new().buffer_size(size).build();
            prop_assert_eq!(cfg.buffer_size, size);
        }

        #[test]
        fn builder_roundtrip_batch_size(size in 0..10_000usize) {
            let cfg = StreamBuilder::new().batch_size(size).build();
            prop_assert_eq!(cfg.batch_size, size);
        }

        #[test]
        fn builder_roundtrip_name(name in "\\PC{0,100}") {
            let cfg = StreamBuilder::new().name(&name).build();
            prop_assert_eq!(cfg.name, name);
        }

        #[test]
        fn metrics_processed_count(n in 0..1000u64) {
            let mut m = StreamMetrics::new();
            for _ in 0..n {
                m.record_processed();
            }
            prop_assert_eq!(m.items_processed, n);
        }
    }
}
