use proptest::prelude::*;
use shiplog_slidingwindow::*;
use std::time::Duration;

// ── TimeSlidingWindow ───────────────────────────────────────────────

#[test]
fn time_window_push_and_get() {
    let mut w: TimeSlidingWindow<i32> = TimeSlidingWindow::new(5, Duration::from_secs(60));
    w.push(1);
    w.push(2);
    assert_eq!(w.get_valid(), vec![&1, &2]);
    assert_eq!(w.len(), 2);
}

#[test]
fn time_window_evicts_oldest_at_capacity() {
    let mut w: TimeSlidingWindow<i32> = TimeSlidingWindow::new(2, Duration::from_secs(60));
    w.push(1);
    w.push(2);
    w.push(3);
    assert_eq!(w.get_valid(), vec![&2, &3]);
}

#[test]
fn time_window_expired_items_not_valid() {
    let mut w: TimeSlidingWindow<i32> = TimeSlidingWindow::new(10, Duration::from_millis(30));
    w.push(1);
    std::thread::sleep(Duration::from_millis(50));
    assert!(w.get_valid().is_empty());
}

#[test]
fn time_window_prune_removes_expired() {
    let mut w: TimeSlidingWindow<i32> = TimeSlidingWindow::new(10, Duration::from_millis(30));
    w.push(1);
    std::thread::sleep(Duration::from_millis(50));
    w.prune_expired();
    assert_eq!(w.len(), 0);
}

#[test]
fn time_window_clear() {
    let mut w: TimeSlidingWindow<i32> = TimeSlidingWindow::new(10, Duration::from_secs(60));
    w.push(1);
    w.push(2);
    w.clear();
    assert!(w.is_empty());
}

// ── ConfigurableSlidingWindow: TailDrop ─────────────────────────────

#[test]
fn tail_drop_evicts_oldest() {
    let mut w: ConfigurableSlidingWindow<i32> =
        ConfigurableSlidingWindow::new(2).with_strategy(WindowStrategy::TailDrop);
    assert!(w.push(1).is_none());
    assert!(w.push(2).is_none());
    let evicted = w.push(3);
    assert_eq!(evicted, Some(1));
    assert_eq!(w.get(), vec![&2, &3]);
}

#[test]
fn tail_drop_returns_evicted_value() {
    let mut w: ConfigurableSlidingWindow<&str> =
        ConfigurableSlidingWindow::new(1).with_strategy(WindowStrategy::TailDrop);
    w.push("first");
    let evicted = w.push("second");
    assert_eq!(evicted, Some("first"));
}

// ── ConfigurableSlidingWindow: TimeBased ────────────────────────────

#[test]
fn time_based_removes_expired_on_push() {
    let mut w: ConfigurableSlidingWindow<i32> = ConfigurableSlidingWindow::new(100)
        .with_strategy(WindowStrategy::TimeBased)
        .with_max_age(Duration::from_millis(30));

    w.push(1);
    std::thread::sleep(Duration::from_millis(50));
    w.push(2);
    assert_eq!(w.len(), 1);
    assert_eq!(w.get(), vec![&2]);
}

// ── ConfigurableSlidingWindow: Hybrid ───────────────────────────────

#[test]
fn hybrid_evicts_by_both_time_and_size() {
    let mut w: ConfigurableSlidingWindow<i32> = ConfigurableSlidingWindow::new(2)
        .with_strategy(WindowStrategy::Hybrid)
        .with_max_age(Duration::from_secs(60));

    w.push(1);
    w.push(2);
    let evicted = w.push(3);
    assert_eq!(evicted, Some(1));
    assert_eq!(w.get(), vec![&2, &3]);
}

#[test]
fn hybrid_time_expiry_prevents_size_eviction() {
    let mut w: ConfigurableSlidingWindow<i32> = ConfigurableSlidingWindow::new(3)
        .with_strategy(WindowStrategy::Hybrid)
        .with_max_age(Duration::from_millis(30));

    w.push(1);
    w.push(2);
    std::thread::sleep(Duration::from_millis(50));
    let evicted = w.push(3);
    // All expired items pruned first; no size eviction needed
    assert!(evicted.is_none());
    assert_eq!(w.len(), 1);
}

// ── WindowWithStats ─────────────────────────────────────────────────

#[test]
fn stats_track_pushes_and_evictions() {
    let mut w: WindowWithStats<i32> = WindowWithStats::new(2);
    w.push(1);
    w.push(2);
    w.push(3);
    assert_eq!(w.stats.total_pushes, 3);
    assert_eq!(w.stats.total_evictions, 1);
}

#[test]
fn stats_track_lookups() {
    let mut w: WindowWithStats<i32> = WindowWithStats::new(5);
    w.push(1);
    let _ = w.get();
    let _ = w.get();
    assert_eq!(w.stats.total_lookups, 2);
}

#[test]
fn stats_window_capacity_enforced() {
    let mut w: WindowWithStats<i32> = WindowWithStats::new(3);
    for i in 0..10 {
        w.push(i);
    }
    assert_eq!(w.len(), 3);
    assert_eq!(w.stats.total_evictions, 7);
}

// ── Edge cases ──────────────────────────────────────────────────────

#[test]
fn time_window_max_size_one() {
    let mut w: TimeSlidingWindow<i32> = TimeSlidingWindow::new(1, Duration::from_secs(60));
    w.push(1);
    w.push(2);
    assert_eq!(w.get_valid(), vec![&2]);
}

#[test]
fn configurable_window_empty_operations() {
    let w: ConfigurableSlidingWindow<i32> = ConfigurableSlidingWindow::new(5);
    assert!(w.is_empty());
    assert_eq!(w.len(), 0);
    assert!(w.get().is_empty());
}

#[test]
fn stats_window_empty() {
    let w: WindowWithStats<i32> = WindowWithStats::new(5);
    assert!(w.is_empty());
    assert_eq!(w.len(), 0);
}

// ── Timestamped ─────────────────────────────────────────────────────

#[test]
fn timestamped_not_expired_initially() {
    let ts = Timestamped::new(42);
    assert_eq!(ts.value, 42);
    assert!(!ts.is_expired(Duration::from_secs(10)));
}

#[test]
fn timestamped_age_grows() {
    let ts = Timestamped::new(1);
    std::thread::sleep(Duration::from_millis(20));
    assert!(ts.age() >= Duration::from_millis(15));
}

// ── Property tests ──────────────────────────────────────────────────

proptest! {
    #[test]
    fn time_window_never_exceeds_max_size(max_size in 1usize..50, pushes in 1usize..200) {
        let mut w: TimeSlidingWindow<usize> = TimeSlidingWindow::new(max_size, Duration::from_secs(600));
        for i in 0..pushes {
            w.push(i);
        }
        prop_assert!(w.len() <= max_size);
    }

    #[test]
    fn tail_drop_window_never_exceeds_max_size(max_size in 1usize..50, pushes in 1usize..200) {
        let mut w: ConfigurableSlidingWindow<usize> =
            ConfigurableSlidingWindow::new(max_size).with_strategy(WindowStrategy::TailDrop);
        for i in 0..pushes {
            w.push(i);
        }
        prop_assert!(w.len() <= max_size);
    }

    #[test]
    fn stats_pushes_equal_total_pushes(pushes in 0usize..100) {
        let mut w: WindowWithStats<usize> = WindowWithStats::new(10);
        for i in 0..pushes {
            w.push(i);
        }
        prop_assert_eq!(w.stats.total_pushes, pushes as u64);
    }

    #[test]
    fn stats_evictions_correct(max_size in 1usize..20, pushes in 0usize..100) {
        let mut w: WindowWithStats<usize> = WindowWithStats::new(max_size);
        for i in 0..pushes {
            w.push(i);
        }
        let expected_evictions = pushes.saturating_sub(max_size);
        prop_assert_eq!(w.stats.total_evictions, expected_evictions as u64);
    }
}
