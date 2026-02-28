//! Integration tests for shiplog-once: OnceCell, Lazy, AsyncOnceCell.

use shiplog_once::{AsyncOnceCell, Lazy, OnceCell};

// ── OnceCell: basic operations ────────────────────────────────────────

#[test]
fn once_cell_empty_then_set() {
    let cell: OnceCell<i32> = OnceCell::new();
    assert!(!cell.is_initialized());
    assert!(cell.get().is_none());

    assert!(cell.set(42).is_ok());
    assert!(cell.is_initialized());
    assert_eq!(*cell.get().unwrap(), 42);
}

#[test]
fn once_cell_double_set_fails() {
    let cell = OnceCell::new();
    cell.set(1).unwrap();

    let err = cell.set(2).unwrap_err();
    assert_eq!(err, 2);
    assert_eq!(*cell.get().unwrap(), 1, "first value should persist");
}

#[test]
fn once_cell_get_or_init_first_call() {
    let cell = OnceCell::new();
    let val = cell.get_or_init(|| 99);
    assert_eq!(*val, 99);
}

#[test]
fn once_cell_get_or_init_subsequent_calls() {
    let cell = OnceCell::new();
    cell.get_or_init(|| 10);
    let val = cell.get_or_init(|| 20); // should not call closure
    assert_eq!(*val, 10);
}

#[test]
fn once_cell_take() {
    let mut cell = OnceCell::new();
    cell.set(42).unwrap();

    let taken = cell.take();
    assert_eq!(taken, Some(42));
    assert!(!cell.is_initialized());
    assert!(cell.get().is_none());
}

#[test]
fn once_cell_take_empty() {
    let mut cell: OnceCell<i32> = OnceCell::new();
    assert_eq!(cell.take(), None);
}

#[test]
fn once_cell_get_mut() {
    let mut cell = OnceCell::new();
    cell.set(10).unwrap();

    if let Some(v) = cell.get_mut() {
        *v = 20;
    }
    assert_eq!(*cell.get().unwrap(), 20);
}

#[test]
fn once_cell_get_mut_empty() {
    let mut cell: OnceCell<i32> = OnceCell::new();
    assert!(cell.get_mut().is_none());
}

#[test]
fn once_cell_from_value() {
    let cell = OnceCell::from(42);
    assert!(cell.is_initialized());
    assert_eq!(*cell.get().unwrap(), 42);
}

#[test]
fn once_cell_default_is_empty() {
    let cell: OnceCell<String> = OnceCell::default();
    assert!(!cell.is_initialized());
}

// ── Lazy: basic ───────────────────────────────────────────────────────

#[test]
fn lazy_deferred_init() {
    let lazy = Lazy::new(|| {
        // This would panic if called prematurely in tests
        String::from("computed")
    });

    assert!(!lazy.is_initialized());
    assert_eq!(lazy.get(), "computed");
    assert!(lazy.is_initialized());
}

#[test]
fn lazy_init_only_once() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

    let lazy = Lazy::new(|| {
        CALL_COUNT.fetch_add(1, Ordering::SeqCst);
        42
    });

    assert_eq!(*lazy.get(), 42);
    assert_eq!(*lazy.get(), 42);
    assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1);
}

// ── AsyncOnceCell: basic ──────────────────────────────────────────────

#[test]
fn async_once_cell_empty() {
    let cell: AsyncOnceCell<i32> = AsyncOnceCell::new();
    assert!(cell.get().is_none());
}

#[test]
fn async_once_cell_set_and_get() {
    let cell = AsyncOnceCell::new();
    assert!(cell.set(42).is_ok());
    assert_eq!(*cell.get().unwrap(), 42);
}

#[test]
fn async_once_cell_double_set_fails() {
    let cell = AsyncOnceCell::new();
    cell.set(1).unwrap();
    let err = cell.set(2).unwrap_err();
    assert_eq!(err, 2);
    assert_eq!(*cell.get().unwrap(), 1);
}

#[test]
fn async_once_cell_default() {
    let cell: AsyncOnceCell<i32> = AsyncOnceCell::default();
    assert!(cell.get().is_none());
}

#[tokio::test]
async fn async_once_cell_wait_notified() {
    let cell = std::sync::Arc::new(AsyncOnceCell::new());
    let cell2 = cell.clone();

    let handle = tokio::spawn(async move {
        // Small delay to ensure waiter is registered
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        cell2.set(99).unwrap();
    });

    cell.wait().await;
    assert_eq!(*cell.get().unwrap(), 99);

    handle.await.unwrap();
}

// ── OnceCell: edge cases ──────────────────────────────────────────────

#[test]
fn once_cell_with_string_type() {
    let cell = OnceCell::new();
    cell.set("hello".to_string()).unwrap();
    assert_eq!(cell.get().unwrap(), "hello");
}

#[test]
fn once_cell_with_vec_type() {
    let cell = OnceCell::new();
    cell.set(vec![1, 2, 3]).unwrap();
    assert_eq!(cell.get().unwrap(), &vec![1, 2, 3]);
}

#[test]
fn once_cell_set_then_take_then_set_again() {
    let mut cell = OnceCell::new();
    cell.set(1).unwrap();
    assert_eq!(cell.take(), Some(1));

    cell.set(2).unwrap();
    assert_eq!(*cell.get().unwrap(), 2);
}
