//! Integration tests for shiplog-atomic: concurrency, property tests, edge cases.

use proptest::prelude::*;
use shiplog_atomic::{AtomicFlag, AtomicI64, AtomicU64, Counter, Sequence};
use std::sync::Arc;
use std::thread;

// ── Counter: concurrent increments ────────────────────────────────────

#[test]
fn counter_concurrent_increment_many_threads() {
    let counter = Arc::new(Counter::new(0));
    let threads: Vec<_> = (0..20)
        .map(|_| {
            let c = Arc::clone(&counter);
            thread::spawn(move || {
                for _ in 0..5_000 {
                    c.increment();
                }
            })
        })
        .collect();

    for t in threads {
        t.join().unwrap();
    }

    assert_eq!(counter.get(), 100_000);
}

#[test]
fn counter_concurrent_increment_and_decrement() {
    let counter = Arc::new(Counter::new(50_000));

    let mut handles = Vec::new();
    // 10 threads increment
    for _ in 0..10 {
        let c = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            for _ in 0..1_000 {
                c.increment();
            }
        }));
    }
    // 10 threads decrement
    for _ in 0..10 {
        let c = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            for _ in 0..1_000 {
                c.decrement();
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    assert_eq!(counter.get(), 50_000);
}

// ── Counter: concurrent add/sub ───────────────────────────────────────

#[test]
fn counter_concurrent_add() {
    let counter = Arc::new(Counter::new(0));
    let threads: Vec<_> = (0..10)
        .map(|_| {
            let c = Arc::clone(&counter);
            thread::spawn(move || {
                for _ in 0..100 {
                    c.add(10);
                }
            })
        })
        .collect();

    for t in threads {
        t.join().unwrap();
    }

    assert_eq!(counter.get(), 10_000);
}

// ── Counter: compare-and-swap races ───────────────────────────────────

#[test]
fn counter_cas_race() {
    let counter = Arc::new(Counter::new(0));
    let success_count = Arc::new(Counter::new(0));

    // Multiple threads try to CAS from 0 → 1; exactly one should succeed
    let threads: Vec<_> = (0..20)
        .map(|_| {
            let c = Arc::clone(&counter);
            let s = Arc::clone(&success_count);
            thread::spawn(move || {
                let old = c.compare_and_swap(0, 1);
                if old == 0 {
                    s.increment();
                }
            })
        })
        .collect();

    for t in threads {
        t.join().unwrap();
    }

    assert_eq!(counter.get(), 1);
    assert_eq!(success_count.get(), 1);
}

// ── Counter: swap ─────────────────────────────────────────────────────

#[test]
fn counter_swap_returns_old() {
    let counter = Counter::new(100);
    let old = counter.swap(200);
    assert_eq!(old, 100);
    assert_eq!(counter.get(), 200);
}

// ── Counter: reset ────────────────────────────────────────────────────

#[test]
fn counter_reset_to_zero() {
    let counter = Counter::new(999);
    counter.reset();
    assert_eq!(counter.get(), 0);
}

#[test]
fn counter_default_starts_at_zero() {
    let counter = Counter::default();
    assert_eq!(counter.get(), 0);
}

// ── AtomicU64: concurrent increments ──────────────────────────────────

#[test]
fn atomic_u64_concurrent_increments() {
    let val = Arc::new(AtomicU64::new(0));
    let threads: Vec<_> = (0..10)
        .map(|_| {
            let v = Arc::clone(&val);
            thread::spawn(move || {
                for _ in 0..1_000 {
                    v.increment();
                }
            })
        })
        .collect();

    for t in threads {
        t.join().unwrap();
    }

    assert_eq!(val.get(), 10_000);
}

#[test]
fn atomic_u64_set_and_get() {
    let val = AtomicU64::new(0);
    val.set(u64::MAX);
    assert_eq!(val.get(), u64::MAX);
}

#[test]
fn atomic_u64_add() {
    let val = AtomicU64::new(10);
    assert_eq!(val.add(5), 15);
    assert_eq!(val.get(), 15);
}

#[test]
fn atomic_u64_default() {
    let val = AtomicU64::default();
    assert_eq!(val.get(), 0);
}

// ── AtomicI64: concurrent ─────────────────────────────────────────────

#[test]
fn atomic_i64_concurrent_add() {
    let val = Arc::new(AtomicI64::new(0));
    let threads: Vec<_> = (0..10)
        .map(|_| {
            let v = Arc::clone(&val);
            thread::spawn(move || {
                for _ in 0..100 {
                    v.add(1);
                }
                for _ in 0..100 {
                    v.add(-1);
                }
            })
        })
        .collect();

    for t in threads {
        t.join().unwrap();
    }

    assert_eq!(val.get(), 0);
}

#[test]
fn atomic_i64_negative_values() {
    let val = AtomicI64::new(-100);
    assert_eq!(val.get(), -100);
    assert_eq!(val.increment(), -99);
    assert_eq!(val.decrement(), -100);
}

#[test]
fn atomic_i64_default() {
    let val = AtomicI64::default();
    assert_eq!(val.get(), 0);
}

// ── AtomicFlag: concurrent ────────────────────────────────────────────

#[test]
fn atomic_flag_concurrent_set_true() {
    let flag = Arc::new(AtomicFlag::new(false));
    let first_setter = Arc::new(Counter::new(0));

    let threads: Vec<_> = (0..20)
        .map(|_| {
            let f = Arc::clone(&flag);
            let s = Arc::clone(&first_setter);
            thread::spawn(move || {
                let was_false = !f.set_true();
                if was_false {
                    s.increment();
                }
            })
        })
        .collect();

    for t in threads {
        t.join().unwrap();
    }

    assert!(flag.get());
    assert_eq!(first_setter.get(), 1, "exactly one thread should be first");
}

#[test]
fn atomic_flag_compare_and_set_race() {
    let flag = Arc::new(AtomicFlag::new(false));
    let success_count = Arc::new(Counter::new(0));

    let threads: Vec<_> = (0..20)
        .map(|_| {
            let f = Arc::clone(&flag);
            let s = Arc::clone(&success_count);
            thread::spawn(move || {
                if f.compare_and_set(false, true) {
                    s.increment();
                }
            })
        })
        .collect();

    for t in threads {
        t.join().unwrap();
    }

    assert!(flag.get());
    assert_eq!(success_count.get(), 1);
}

#[test]
fn atomic_flag_default() {
    let flag = AtomicFlag::default();
    assert!(!flag.get());
}

// ── Sequence: concurrent ──────────────────────────────────────────────

#[test]
fn sequence_concurrent_uniqueness() {
    let seq = Arc::new(Sequence::new(0));
    let results: Vec<_> = (0..10)
        .map(|_| {
            let s = Arc::clone(&seq);
            thread::spawn(move || {
                let mut vals = Vec::new();
                for _ in 0..100 {
                    vals.push(s.next());
                }
                vals
            })
        })
        .collect();

    let mut all_values: Vec<usize> = results
        .into_iter()
        .flat_map(|h| h.join().unwrap())
        .collect();

    all_values.sort();
    all_values.dedup();
    assert_eq!(all_values.len(), 1000, "all sequence values must be unique");
}

#[test]
fn sequence_reset() {
    let seq = Sequence::new(0);
    seq.next();
    seq.next();
    seq.reset(100);
    assert_eq!(seq.current(), 100);
}

#[test]
fn sequence_default_starts_at_zero() {
    let seq = Sequence::default();
    assert_eq!(seq.current(), 0);
    assert_eq!(seq.next(), 1);
}

// ── Property tests ────────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_counter_add_sub_identity(init in 1000usize..100_000, delta in 0usize..1000) {
        let counter = Counter::new(init);
        counter.add(delta);
        counter.sub(delta);
        prop_assert_eq!(counter.get(), init);
    }

    #[test]
    fn prop_counter_increment_decrement_identity(init in 1usize..100_000) {
        let counter = Counter::new(init);
        counter.increment();
        counter.decrement();
        prop_assert_eq!(counter.get(), init);
    }

    #[test]
    fn prop_atomic_u64_add_monotonic(a in 0u64..1_000_000, b in 0u64..1_000_000) {
        let val = AtomicU64::new(a);
        let result = val.add(b);
        prop_assert_eq!(result, a + b);
    }

    #[test]
    fn prop_atomic_i64_add(a in -500_000i64..500_000, b in -500_000i64..500_000) {
        let val = AtomicI64::new(a);
        let result = val.add(b);
        prop_assert_eq!(result, a + b);
    }

    #[test]
    fn prop_flag_set_true_returns_old(init in proptest::bool::ANY) {
        let flag = AtomicFlag::new(init);
        let old = flag.set_true();
        prop_assert_eq!(old, init);
        prop_assert!(flag.get());
    }

    #[test]
    fn prop_counter_swap_returns_old(init in 0usize..100_000, new_val in 0usize..100_000) {
        let counter = Counter::new(init);
        let old = counter.swap(new_val);
        prop_assert_eq!(old, init);
        prop_assert_eq!(counter.get(), new_val);
    }
}
