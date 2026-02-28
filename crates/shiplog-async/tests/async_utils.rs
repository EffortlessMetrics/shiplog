//! Comprehensive tests for shiplog-async: config, builder, result, timeout, futures.

use shiplog_async::{AsyncBuilder, AsyncConfig, AsyncResult, Timeout, TimeoutExt};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

// ── AsyncConfig ─────────────────────────────────────────────────────────────

#[test]
fn config_default_values() {
    let cfg = AsyncConfig::default();
    assert_eq!(cfg.buffer_size, 100);
    assert_eq!(cfg.max_concurrent, 10);
    assert_eq!(cfg.spawn_timeout_ms, 5000);
}

#[test]
fn config_clone_preserves_values() {
    let cfg = AsyncConfig {
        buffer_size: 50,
        max_concurrent: 3,
        spawn_timeout_ms: 1000,
    };
    let cloned = cfg.clone();
    assert_eq!(cloned.buffer_size, 50);
    assert_eq!(cloned.max_concurrent, 3);
    assert_eq!(cloned.spawn_timeout_ms, 1000);
}

#[test]
fn config_debug_impl() {
    let cfg = AsyncConfig::default();
    let debug = format!("{:?}", cfg);
    assert!(debug.contains("AsyncConfig"));
    assert!(debug.contains("100"));
}

// ── AsyncBuilder ────────────────────────────────────────────────────────────

#[test]
fn builder_default_matches_config_default() {
    let from_builder = AsyncBuilder::default().build();
    let from_config = AsyncConfig::default();
    assert_eq!(from_builder.buffer_size, from_config.buffer_size);
    assert_eq!(from_builder.max_concurrent, from_config.max_concurrent);
    assert_eq!(from_builder.spawn_timeout_ms, from_config.spawn_timeout_ms);
}

#[test]
fn builder_new_equals_default() {
    let from_new = AsyncBuilder::new().build();
    let from_default = AsyncBuilder::default().build();
    assert_eq!(from_new.buffer_size, from_default.buffer_size);
}

#[test]
fn builder_buffer_size() {
    let cfg = AsyncBuilder::new().buffer_size(500).build();
    assert_eq!(cfg.buffer_size, 500);
}

#[test]
fn builder_max_concurrent() {
    let cfg = AsyncBuilder::new().max_concurrent(1).build();
    assert_eq!(cfg.max_concurrent, 1);
}

#[test]
fn builder_spawn_timeout() {
    let cfg = AsyncBuilder::new().spawn_timeout(10000).build();
    assert_eq!(cfg.spawn_timeout_ms, 10000);
}

#[test]
fn builder_chaining_all_methods() {
    let cfg = AsyncBuilder::new()
        .buffer_size(1)
        .max_concurrent(2)
        .spawn_timeout(3)
        .build();
    assert_eq!(cfg.buffer_size, 1);
    assert_eq!(cfg.max_concurrent, 2);
    assert_eq!(cfg.spawn_timeout_ms, 3);
}

#[test]
fn builder_last_setter_wins() {
    let cfg = AsyncBuilder::new()
        .buffer_size(10)
        .buffer_size(20)
        .buffer_size(30)
        .build();
    assert_eq!(cfg.buffer_size, 30);
}

#[test]
fn builder_zero_values() {
    let cfg = AsyncBuilder::new()
        .buffer_size(0)
        .max_concurrent(0)
        .spawn_timeout(0)
        .build();
    assert_eq!(cfg.buffer_size, 0);
    assert_eq!(cfg.max_concurrent, 0);
    assert_eq!(cfg.spawn_timeout_ms, 0);
}

// ── AsyncResult ─────────────────────────────────────────────────────────────

#[test]
fn result_new_is_ok() {
    let r = AsyncResult::new(42, 100);
    assert!(r.is_ok());
    assert!(!r.is_timeout());
    assert_eq!(r.value, Some(42));
    assert_eq!(r.elapsed_ms, 100);
}

#[test]
fn result_timeout_is_not_ok() {
    let r = AsyncResult::<String>::timeout(5000);
    assert!(!r.is_ok());
    assert!(r.is_timeout());
    assert!(r.value.is_none());
    assert_eq!(r.elapsed_ms, 5000);
}

#[test]
fn result_new_with_zero_elapsed() {
    let r = AsyncResult::new("fast", 0);
    assert!(r.is_ok());
    assert_eq!(r.elapsed_ms, 0);
}

#[test]
fn result_clone() {
    let r = AsyncResult::new(vec![1, 2, 3], 50);
    let cloned = r.clone();
    assert_eq!(cloned.value, Some(vec![1, 2, 3]));
    assert_eq!(cloned.elapsed_ms, 50);
}

#[test]
fn result_debug_impl() {
    let r = AsyncResult::new(42, 100);
    let debug = format!("{:?}", r);
    assert!(debug.contains("AsyncResult"));
    assert!(debug.contains("42"));
}

#[test]
fn result_with_unit_type() {
    let r = AsyncResult::new((), 10);
    assert!(r.is_ok());
    assert_eq!(r.value, Some(()));
}

// ── Timeout future ──────────────────────────────────────────────────────────

#[tokio::test]
async fn timeout_wraps_ready_future() {
    let fut = std::future::ready(99);
    let timeout = Timeout::new(fut);
    let val = timeout.await;
    assert_eq!(val, 99);
}

// ── Async tests with tokio ──────────────────────────────────────────────────

#[tokio::test]
async fn async_ready_value() {
    let val = async { 42 }.await;
    assert_eq!(val, 42);
}

#[tokio::test]
async fn async_result_from_computation() {
    let start = std::time::Instant::now();
    let value = async { 2 + 2 }.await;
    let elapsed = start.elapsed().as_millis() as u64;
    let result = AsyncResult::new(value, elapsed);
    assert!(result.is_ok());
    assert_eq!(result.value, Some(4));
}

#[tokio::test]
async fn timeout_ext_on_ready_future() {
    let fut = std::future::ready(10);
    let wrapped = fut.timeout();
    let val = wrapped.await;
    assert_eq!(val, 10);
}

#[tokio::test]
async fn timeout_ext_on_async_block() {
    let fut = Box::pin(async { "hello" });
    let wrapped = fut.timeout();
    let val = wrapped.await;
    assert_eq!(val, "hello");
}

#[tokio::test]
async fn multiple_async_results() {
    let mut results = Vec::new();
    for i in 0..5 {
        let val = async move { i * 10 }.await;
        results.push(AsyncResult::new(val, 0));
    }
    assert_eq!(results.len(), 5);
    assert!(results.iter().all(|r| r.is_ok()));
    assert_eq!(results[3].value, Some(30));
}

#[tokio::test]
async fn tokio_sleep_with_result() {
    let start = std::time::Instant::now();
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    let elapsed = start.elapsed().as_millis() as u64;
    let result = AsyncResult::new("done", elapsed);
    assert!(result.is_ok());
    assert!(result.elapsed_ms >= 5); // Allow some jitter
}

#[tokio::test]
async fn tokio_spawn_with_result() {
    let handle = tokio::spawn(async { 123 });
    let val = handle.await.unwrap();
    let result = AsyncResult::new(val, 0);
    assert!(result.is_ok());
    assert_eq!(result.value, Some(123));
}

#[tokio::test]
async fn async_config_used_for_concurrency_limit() {
    let config = AsyncBuilder::new().max_concurrent(3).build();
    let semaphore = tokio::sync::Semaphore::new(config.max_concurrent);

    let mut handles = Vec::new();
    for _ in 0..config.max_concurrent {
        let permit = semaphore.try_acquire();
        assert!(permit.is_ok());
        handles.push(permit.unwrap());
    }
    // Next acquire should fail
    assert!(semaphore.try_acquire().is_err());
}

// ── Custom future with Timeout ──────────────────────────────────────────────

struct CountdownFuture {
    remaining: u32,
}

impl Future for CountdownFuture {
    type Output = &'static str;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.remaining == 0 {
            Poll::Ready("done")
        } else {
            self.remaining -= 1;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

impl Unpin for CountdownFuture {}

#[tokio::test]
async fn timeout_wraps_custom_future() {
    let fut = CountdownFuture { remaining: 3 };
    let wrapped = Timeout::new(fut);
    let result = wrapped.await;
    assert_eq!(result, "done");
}

#[tokio::test]
async fn timeout_ext_on_custom_future() {
    let fut = CountdownFuture { remaining: 1 };
    let result = fut.timeout().await;
    assert_eq!(result, "done");
}
