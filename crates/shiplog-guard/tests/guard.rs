//! Integration tests for shiplog-guard: RAII guards, concurrency, property tests.

use proptest::prelude::*;
use shiplog_guard::{CounterGuard, Guard, GuardBuilder, GuardConfig, MutexGuard, OnceGuard};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

// ── Guard: cleanup on drop ────────────────────────────────────────────

#[test]
fn guard_cleanup_runs_on_drop() {
    let cleaned = Arc::new(AtomicBool::new(false));
    let c = Arc::clone(&cleaned);

    {
        let _guard = Guard::new(move || {
            c.store(true, Ordering::SeqCst);
        });
        assert!(!cleaned.load(Ordering::SeqCst));
    }

    assert!(cleaned.load(Ordering::SeqCst));
}

#[test]
fn guard_disarm_prevents_cleanup() {
    let cleaned = Arc::new(AtomicBool::new(false));
    let c = Arc::clone(&cleaned);

    {
        let guard = Guard::new(move || {
            c.store(true, Ordering::SeqCst);
        });
        guard.disarm();
    }

    assert!(!cleaned.load(Ordering::SeqCst));
}

#[test]
fn guard_cleanup_on_panic() {
    let cleaned = Arc::new(AtomicBool::new(false));
    let c = Arc::clone(&cleaned);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = Guard::new(move || {
            c.store(true, Ordering::SeqCst);
        });
        panic!("test panic");
    }));

    assert!(result.is_err());
    assert!(
        cleaned.load(Ordering::SeqCst),
        "cleanup should run even on panic"
    );
}

// ── Guard: multi-threaded drop ────────────────────────────────────────

#[test]
fn guard_cleanup_in_thread() {
    let cleaned = Arc::new(AtomicBool::new(false));
    let c = Arc::clone(&cleaned);

    let handle = thread::spawn(move || {
        let _guard = Guard::new(move || {
            c.store(true, Ordering::SeqCst);
        });
    });

    handle.join().unwrap();
    assert!(cleaned.load(Ordering::SeqCst));
}

// ── OnceGuard: thread-safety ──────────────────────────────────────────

#[test]
fn once_guard_trigger_exactly_once_concurrent() {
    let guard = Arc::new(OnceGuard::new());
    let success_count = Arc::new(AtomicUsize::new(0));

    let threads: Vec<_> = (0..20)
        .map(|_| {
            let g = Arc::clone(&guard);
            let s = Arc::clone(&success_count);
            thread::spawn(move || {
                if g.try_trigger() {
                    s.fetch_add(1, Ordering::SeqCst);
                }
            })
        })
        .collect();

    for t in threads {
        t.join().unwrap();
    }

    assert!(guard.is_done());
    assert_eq!(
        success_count.load(Ordering::SeqCst),
        1,
        "exactly one thread should succeed"
    );
}

#[test]
fn once_guard_default_not_done() {
    let guard = OnceGuard::default();
    assert!(!guard.is_done());
}

#[test]
fn once_guard_sequential_triggers() {
    let guard = OnceGuard::new();
    assert!(guard.try_trigger());
    assert!(!guard.try_trigger());
    assert!(!guard.try_trigger());
    assert!(guard.is_done());
}

// ── CounterGuard: RAII counter ────────────────────────────────────────
// Each test uses its own static to avoid cross-test interference.

#[test]
fn counter_guard_increment_and_decrement() {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    assert_eq!(COUNTER.load(Ordering::SeqCst), 0);

    {
        let _g1 = CounterGuard::new(&COUNTER);
        assert_eq!(COUNTER.load(Ordering::SeqCst), 1);

        {
            let _g2 = CounterGuard::new(&COUNTER);
            assert_eq!(COUNTER.load(Ordering::SeqCst), 2);
        }
        assert_eq!(COUNTER.load(Ordering::SeqCst), 1);
    }
    assert_eq!(COUNTER.load(Ordering::SeqCst), 0);
}

#[test]
fn counter_guard_concurrent() {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    let barrier = Arc::new(std::sync::Barrier::new(10));

    let threads: Vec<_> = (0..10)
        .map(|_| {
            let b = Arc::clone(&barrier);
            thread::spawn(move || {
                let _guard = CounterGuard::new(&COUNTER);
                b.wait();
                // All 10 guards are alive at this point
                let val = COUNTER.load(Ordering::SeqCst);
                assert!((1..=10).contains(&val));
            })
        })
        .collect();

    for t in threads {
        t.join().unwrap();
    }

    assert_eq!(COUNTER.load(Ordering::SeqCst), 0);
}

// ── MutexGuard ────────────────────────────────────────────────────────

#[test]
fn mutex_guard_get() {
    let mutex = Mutex::new(42);
    let std_guard = mutex.lock().unwrap();
    let guard = MutexGuard::new(std_guard);
    assert_eq!(*guard.get().unwrap(), 42);
}

#[test]
fn mutex_guard_release() {
    let mutex = Mutex::new(42);
    let std_guard = mutex.lock().unwrap();
    let guard = MutexGuard::new(std_guard);
    guard.release();
    // After release, mutex should be unlockable again
    let _g2 = mutex.lock().unwrap();
}

// ── GuardConfig / GuardBuilder ────────────────────────────────────────

#[test]
fn guard_config_default() {
    let cfg = GuardConfig::default();
    assert!(!cfg.panic_on_drop);
    assert!(!cfg.log_on_drop);
}

#[test]
fn guard_builder_all_options() {
    let cfg = GuardBuilder::new()
        .panic_on_drop(true)
        .log_on_drop(true)
        .build();
    assert!(cfg.panic_on_drop);
    assert!(cfg.log_on_drop);
}

#[test]
fn guard_builder_default() {
    let cfg = GuardBuilder::default().build();
    assert!(!cfg.panic_on_drop);
    assert!(!cfg.log_on_drop);
}

// ── Property tests ────────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_once_guard_trigger_idempotent(_attempts in 1usize..100) {
        let guard = OnceGuard::new();
        let first = guard.try_trigger();
        prop_assert!(first);

        for _ in 1.._attempts {
            prop_assert!(!guard.try_trigger());
        }
        prop_assert!(guard.is_done());
    }

    #[test]
    fn prop_guard_always_runs_cleanup(val in 0i32..1000) {
        let result = Arc::new(Mutex::new(None));
        let r = Arc::clone(&result);

        {
            let _guard = Guard::new(move || {
                *r.lock().unwrap() = Some(val);
            });
        }

        prop_assert_eq!(*result.lock().unwrap(), Some(val));
    }
}
