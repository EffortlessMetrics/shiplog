//! Latch and barrier utilities for synchronization in shiplog.
//!
//! This crate provides synchronization primitives for coordinating multiple tasks.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Notify;

/// A countdown latch that blocks until the count reaches zero.
///
/// The latch starts with a given count and decrements each time `count_down()` is called.
/// When the count reaches zero, all waiting tasks are notified.
#[derive(Debug)]
pub struct CountDownLatch {
    count: Arc<AtomicUsize>,
    notify: Arc<Notify>,
}

impl CountDownLatch {
    /// Create a new latch with the given initial count.
    pub fn new(count: usize) -> Self {
        Self {
            count: Arc::new(AtomicUsize::new(count)),
            notify: Arc::new(Notify::new()),
        }
    }

    /// Get the current count.
    pub fn count(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }

    /// Decrement the count by one.
    ///
    /// If the count reaches zero, all waiting tasks are notified.
    pub fn count_down(&self) {
        let old_count = self.count.fetch_sub(1, Ordering::SeqCst);
        if old_count == 1 {
            // Count reached zero, notify all waiters
            self.notify.notify_waiters();
        }
    }

    /// Wait until the count reaches zero.
    pub async fn wait(&self) {
        if self.count.load(Ordering::SeqCst) == 0 {
            return;
        }
        self.notify.notified().await;
    }

    /// Try to wait without blocking.
    ///
    /// Returns `true` if the count is already zero.
    pub fn try_wait(&self) -> bool {
        self.count.load(Ordering::SeqCst) == 0
    }
}

impl Clone for CountDownLatch {
    fn clone(&self) -> Self {
        Self {
            count: Arc::clone(&self.count),
            notify: Arc::clone(&self.notify),
        }
    }
}

/// A barrier that blocks until all parties have reached it.
///
/// Similar to `std::sync::Barrier` but async-aware.
#[derive(Debug)]
pub struct Barrier {
    count: Arc<AtomicUsize>,
    waiters: Arc<AtomicUsize>,
    notify: Arc<Notify>,
}

impl Barrier {
    /// Create a new barrier for the given number of parties.
    pub fn new(count: usize) -> Self {
        Self {
            count: Arc::new(AtomicUsize::new(count)),
            waiters: Arc::new(AtomicUsize::new(0)),
            notify: Arc::new(Notify::new()),
        }
    }

    /// Wait until all parties have reached the barrier.
    ///
    /// Returns `true` if this is the last party to reach the barrier.
    pub async fn wait(&self) -> bool {
        let waiters = self.waiters.fetch_add(1, Ordering::SeqCst);
        let total = self.count.load(Ordering::SeqCst);

        if waiters + 1 == total {
            // This is the last waiter, reset and notify all
            self.waiters.store(0, Ordering::SeqCst);
            self.notify.notify_waiters();
            true
        } else {
            // Wait for the last party
            self.notify.notified().await;
            false
        }
    }

    /// Get the number of parties.
    pub fn parties(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }

    /// Get the number of currently waiting parties.
    pub fn waiters(&self) -> usize {
        self.waiters.load(Ordering::SeqCst)
    }
}

impl Clone for Barrier {
    fn clone(&self) -> Self {
        Self {
            count: Arc::clone(&self.count),
            waiters: Arc::clone(&self.waiters),
            notify: Arc::clone(&self.notify),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;
    use tokio::runtime::Runtime;

    #[test]
    fn test_count_down_latch_creation() {
        let latch = CountDownLatch::new(5);
        assert_eq!(latch.count(), 5);
    }

    #[test]
    fn test_count_down_latch_decrement() {
        let latch = CountDownLatch::new(3);
        assert_eq!(latch.count(), 3);
        
        latch.count_down();
        assert_eq!(latch.count(), 2);
        
        latch.count_down();
        latch.count_down();
        assert_eq!(latch.count(), 0);
    }

    #[test]
    fn test_count_down_latch_try_wait() {
        let latch = CountDownLatch::new(1);
        assert!(!latch.try_wait());
        
        latch.count_down();
        assert!(latch.try_wait());
    }

    #[test]
    fn test_count_down_latch_async_wait() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let latch = CountDownLatch::new(1);
            
            let latch_clone = latch.clone();
            let handle = thread::spawn(move || {
                latch_clone.count_down();
            });
            
            latch.wait().await;
            
            handle.join().unwrap();
            assert!(latch.try_wait());
        });
    }

    #[test]
    fn test_barrier_creation() {
        let barrier = Barrier::new(3);
        assert_eq!(barrier.parties(), 3);
        assert_eq!(barrier.waiters(), 0);
    }

    #[test]
    fn test_barrier_single_wait() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let barrier = Barrier::new(1);
            let is_last = barrier.wait().await;
            assert!(is_last);
        });
    }

    #[test]
    fn test_barrier_multiple_waiters() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let barrier = Arc::new(Barrier::new(3));
            let mut handles = Vec::new();
            
            for _ in 0..3 {
                let barrier_clone = barrier.clone();
                let handle = tokio::spawn(async move {
                    barrier_clone.wait().await
                });
                handles.push(handle);
            }
            
            let mut last_count = 0;
            for handle in handles {
                let is_last = handle.await.unwrap();
                if is_last {
                    last_count += 1;
                }
            }
            
            // Exactly one should be the last
            assert_eq!(last_count, 1);
        });
    }
}
