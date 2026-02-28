//! Integration tests for shiplog-latch: CountDownLatch and Barrier concurrency.

use shiplog_latch::{Barrier, CountDownLatch};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

// ── CountDownLatch: basic ─────────────────────────────────────────────

#[tokio::test]
async fn latch_zero_count_immediately_ready() {
    let latch = CountDownLatch::new(0);
    assert!(latch.try_wait());
    latch.wait().await; // should return immediately
}

#[tokio::test]
async fn latch_single_countdown() {
    let latch = CountDownLatch::new(1);
    assert!(!latch.try_wait());
    latch.count_down();
    assert!(latch.try_wait());
}

#[test]
fn latch_count_reflects_decrements() {
    let latch = CountDownLatch::new(5);
    assert_eq!(latch.count(), 5);
    latch.count_down();
    assert_eq!(latch.count(), 4);
    latch.count_down();
    latch.count_down();
    assert_eq!(latch.count(), 2);
}

// ── CountDownLatch: multi-task ────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn latch_multi_task_countdown() {
    let n = 10;
    let latch = CountDownLatch::new(n);
    let completed = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();
    for _ in 0..n {
        let l = latch.clone();
        let c = Arc::clone(&completed);
        handles.push(tokio::spawn(async move {
            c.fetch_add(1, Ordering::SeqCst);
            l.count_down();
        }));
    }

    latch.wait().await;

    for h in handles {
        h.await.unwrap();
    }

    assert_eq!(completed.load(Ordering::SeqCst), n);
    assert!(latch.try_wait());
}

// ── CountDownLatch: clone shares state ────────────────────────────────

#[test]
fn latch_clone_shares_count() {
    let latch = CountDownLatch::new(3);
    let clone = latch.clone();

    clone.count_down();
    assert_eq!(latch.count(), 2);

    latch.count_down();
    assert_eq!(clone.count(), 1);
}

// ── CountDownLatch: waiter unblocked by threads ───────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn latch_waiter_unblocked_by_spawned_tasks() {
    let latch = CountDownLatch::new(3);

    for _ in 0..3 {
        let l = latch.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            l.count_down();
        });
    }

    tokio::time::timeout(std::time::Duration::from_secs(5), latch.wait())
        .await
        .expect("latch should be released within timeout");
}

// ── Barrier: single party ─────────────────────────────────────────────

#[tokio::test]
async fn barrier_single_party() {
    let barrier = Barrier::new(1);
    let is_last = barrier.wait().await;
    assert!(is_last);
}

// ── Barrier: multiple parties ─────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn barrier_multiple_parties() {
    let n = 5;
    let barrier = Barrier::new(n);
    let reached = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();
    for _ in 0..n {
        let b = barrier.clone();
        let r = Arc::clone(&reached);
        handles.push(tokio::spawn(async move {
            r.fetch_add(1, Ordering::SeqCst);
            b.wait().await
        }));
    }

    let mut last_count = 0;
    for h in handles {
        if h.await.unwrap() {
            last_count += 1;
        }
    }

    assert_eq!(last_count, 1, "exactly one party should be 'last'");
    assert_eq!(reached.load(Ordering::SeqCst), n);
}

// ── Barrier: properties ───────────────────────────────────────────────

#[test]
fn barrier_parties_and_waiters() {
    let barrier = Barrier::new(10);
    assert_eq!(barrier.parties(), 10);
    assert_eq!(barrier.waiters(), 0);
}

// ── Barrier: clone shares state ───────────────────────────────────────

#[test]
fn barrier_clone_shares_parties() {
    let barrier = Barrier::new(3);
    let clone = barrier.clone();
    assert_eq!(clone.parties(), 3);
}

// ── CountDownLatch: used as task gate ─────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn latch_used_as_start_gate() {
    let gate = CountDownLatch::new(1);
    let results = Arc::new(std::sync::Mutex::new(Vec::new()));

    let mut handles = Vec::new();
    for i in 0..5u32 {
        let g = gate.clone();
        let r = Arc::clone(&results);
        handles.push(tokio::spawn(async move {
            g.wait().await; // all tasks wait for the gate
            r.lock().unwrap().push(i);
        }));
    }

    // Small delay to let tasks start waiting
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Open the gate
    gate.count_down();

    for h in handles {
        h.await.unwrap();
    }

    let mut r = results.lock().unwrap();
    r.sort();
    assert_eq!(*r, vec![0, 1, 2, 3, 4]);
}
