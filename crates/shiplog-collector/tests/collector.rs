use proptest::prelude::*;
use shiplog_collector::{Collector, CollectorConfig, ConditionalCollector};

// ── Collector correctness ──────────────────────────────────────────

#[test]
fn collector_empty() {
    let c: Collector<i32> = Collector::new(10);
    assert!(c.is_empty());
    assert_eq!(c.len(), 0);
    assert!(!c.is_batch_ready());
}

#[test]
fn collector_push_and_len() {
    let mut c = Collector::new(5);
    c.push(1);
    c.push(2);
    c.push(3);
    assert_eq!(c.len(), 3);
    assert!(!c.is_batch_ready());
}

#[test]
fn collector_batch_ready() {
    let mut c = Collector::new(3);
    c.push(1);
    c.push(2);
    assert!(!c.is_batch_ready());
    c.push(3);
    assert!(c.is_batch_ready());
}

#[test]
fn collector_drain_batch() {
    let mut c = Collector::new(3);
    for i in 1..=5 {
        c.push(i);
    }
    let batch = c.drain_batch();
    assert_eq!(batch, vec![1, 2, 3]);
    assert_eq!(c.len(), 2);
}

#[test]
fn collector_drain_batch_partial() {
    let mut c = Collector::new(10);
    c.push(1);
    c.push(2);
    let batch = c.drain_batch();
    assert_eq!(batch, vec![1, 2]);
    assert!(c.is_empty());
}

#[test]
fn collector_drain_all() {
    let mut c = Collector::new(10);
    c.push(1);
    c.push(2);
    c.push(3);
    let all = c.drain_all();
    assert_eq!(all, vec![1, 2, 3]);
    assert!(c.is_empty());
}

#[test]
fn collector_drain_all_empty() {
    let mut c: Collector<i32> = Collector::new(10);
    let all = c.drain_all();
    assert!(all.is_empty());
}

#[test]
fn collector_push_batch() {
    let mut c = Collector::new(10);
    c.push_batch(vec![1, 2, 3]);
    assert_eq!(c.len(), 3);
}

#[test]
fn collector_with_config() {
    let config = CollectorConfig {
        batch_size: 42,
        flush_interval_ms: 500,
        name: "my-collector".to_string(),
    };
    let c: Collector<i32> = Collector::with_config(&config);
    assert_eq!(c.batch_size(), 42);
    assert_eq!(c.name(), "my-collector");
}

#[test]
fn collector_default_config() {
    let config = CollectorConfig::default();
    assert_eq!(config.batch_size, 100);
    assert_eq!(config.flush_interval_ms, 1000);
    assert_eq!(config.name, "collector");
}

#[test]
fn collector_default_name() {
    let c: Collector<i32> = Collector::new(10);
    assert_eq!(c.name(), "collector");
}

// ── Edge cases ─────────────────────────────────────────────────────

#[test]
fn collector_batch_size_1() {
    let mut c = Collector::new(1);
    c.push(42);
    assert!(c.is_batch_ready());
    let batch = c.drain_batch();
    assert_eq!(batch, vec![42]);
    assert!(c.is_empty());
}

#[test]
fn collector_multiple_drain_cycles() {
    let mut c = Collector::new(2);
    c.push(1);
    c.push(2);
    c.push(3);
    c.push(4);

    let b1 = c.drain_batch();
    assert_eq!(b1, vec![1, 2]);

    let b2 = c.drain_batch();
    assert_eq!(b2, vec![3, 4]);

    assert!(c.is_empty());
}

#[test]
fn collector_large_push_batch() {
    let mut c = Collector::new(100);
    let items: Vec<i32> = (0..1000).collect();
    c.push_batch(items);
    assert_eq!(c.len(), 1000);
    assert!(c.is_batch_ready());
}

// ── ConditionalCollector ───────────────────────────────────────────

#[test]
fn conditional_collector_triggers() {
    let mut cc = ConditionalCollector::new(|items: &[i32]| items.len() >= 3);
    assert!(!cc.push(1));
    assert!(!cc.push(2));
    assert!(cc.push(3));
    assert_eq!(cc.len(), 3);
}

#[test]
fn conditional_collector_never_triggers() {
    let mut cc = ConditionalCollector::new(|_: &[i32]| false);
    cc.push(1);
    cc.push(2);
    cc.push(3);
    assert!(!cc.is_empty());
    assert_eq!(cc.len(), 3);
}

#[test]
fn conditional_collector_always_triggers() {
    let mut cc = ConditionalCollector::new(|_: &[i32]| true);
    assert!(cc.push(1));
}

#[test]
fn conditional_collector_into_inner() {
    let mut cc = ConditionalCollector::new(|_: &[i32]| false);
    cc.push(10);
    cc.push(20);
    let items = cc.into_inner();
    assert_eq!(items, vec![10, 20]);
}

#[test]
fn conditional_collector_collect_ref() {
    let mut cc = ConditionalCollector::new(|_: &[i32]| false);
    cc.push(1);
    cc.push(2);
    assert_eq!(cc.collect(), &[1, 2]);
}

#[test]
fn conditional_collector_empty() {
    let cc = ConditionalCollector::new(|_: &[i32]| false);
    assert!(cc.is_empty());
    assert_eq!(cc.len(), 0);
}

// ── Composition: collector drain loop ──────────────────────────────

#[test]
fn composition_drain_loop() {
    let mut c = Collector::new(3);
    let mut all_batches = Vec::new();

    for i in 0..10 {
        c.push(i);
        if c.is_batch_ready() {
            all_batches.push(c.drain_batch());
        }
    }
    // Drain remaining
    let remainder = c.drain_all();
    if !remainder.is_empty() {
        all_batches.push(remainder);
    }

    let total: Vec<i32> = all_batches.into_iter().flatten().collect();
    assert_eq!(total, (0..10).collect::<Vec<_>>());
}

// ── Property tests ─────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_collector_drain_all_returns_everything(
        items in prop::collection::vec(any::<i32>(), 0..200)
    ) {
        let mut c = Collector::new(50);
        for &item in &items {
            c.push(item);
        }
        let all = c.drain_all();
        prop_assert_eq!(all, items);
    }

    #[test]
    fn prop_collector_drain_batch_bounded(
        items in prop::collection::vec(any::<i32>(), 0..200),
        batch_size in 1usize..50,
    ) {
        let mut c = Collector::new(batch_size);
        for &item in &items {
            c.push(item);
        }
        let batch = c.drain_batch();
        prop_assert!(batch.len() <= batch_size);
    }

    #[test]
    fn prop_collector_push_batch_equivalent_to_push_loop(
        items in prop::collection::vec(any::<i32>(), 0..100)
    ) {
        let mut c1 = Collector::new(50);
        let mut c2 = Collector::new(50);
        for &item in &items {
            c1.push(item);
        }
        c2.push_batch(items);
        prop_assert_eq!(c1.drain_all(), c2.drain_all());
    }

    #[test]
    fn prop_batch_ready_iff_len_gte_batch_size(
        items in prop::collection::vec(any::<i32>(), 0..100),
        batch_size in 1usize..50,
    ) {
        let mut c = Collector::new(batch_size);
        for &item in &items {
            c.push(item);
        }
        prop_assert_eq!(c.is_batch_ready(), c.len() >= batch_size);
    }

    #[test]
    fn prop_conditional_collector_len_matches(
        items in prop::collection::vec(any::<i32>(), 0..100)
    ) {
        let mut cc = ConditionalCollector::new(|_: &[i32]| false);
        for &item in &items {
            cc.push(item);
        }
        prop_assert_eq!(cc.len(), items.len());
    }
}
