use proptest::prelude::*;
use shiplog_timewheel::*;
use std::time::Duration;

// ── ScheduledTask ───────────────────────────────────────────────────

#[test]
fn task_not_ready_immediately() {
    let t = ScheduledTask::new(0, "data", Duration::from_secs(10));
    assert!(!t.is_ready());
    assert!(t.remaining_time() > Duration::ZERO);
}

#[test]
fn task_ready_after_delay() {
    let t = ScheduledTask::new(0, "data", Duration::from_millis(20));
    std::thread::sleep(Duration::from_millis(40));
    assert!(t.is_ready());
}

#[test]
fn task_zero_delay_ready_immediately() {
    let t = ScheduledTask::new(0, "data", Duration::ZERO);
    assert!(t.is_ready());
    assert_eq!(t.remaining_time(), Duration::ZERO);
}

// ── TimeWheel basic ─────────────────────────────────────────────────

#[test]
fn schedule_returns_incrementing_ids() {
    let config = TimeWheelConfig::default();
    let mut wheel: TimeWheel<i32> = TimeWheel::new(&config);
    assert_eq!(wheel.schedule(1, Duration::from_millis(100)), 0);
    assert_eq!(wheel.schedule(2, Duration::from_millis(100)), 1);
    assert_eq!(wheel.schedule(3, Duration::from_millis(100)), 2);
}

#[test]
fn pending_count_tracks_tasks() {
    let config = TimeWheelConfig::default();
    let mut wheel: TimeWheel<i32> = TimeWheel::new(&config);
    wheel.schedule(1, Duration::from_secs(10));
    wheel.schedule(2, Duration::from_secs(10));
    assert_eq!(wheel.pending_count(), 2);
}

#[test]
fn total_scheduled_increments() {
    let config = TimeWheelConfig::default();
    let mut wheel: TimeWheel<i32> = TimeWheel::new(&config);
    wheel.schedule(1, Duration::from_secs(1));
    wheel.schedule(2, Duration::from_secs(2));
    assert_eq!(wheel.total_scheduled(), 2);
}

// ── Advance ─────────────────────────────────────────────────────────

#[test]
fn advance_returns_ready_tasks() {
    let config = TimeWheelConfig {
        tick_duration: Duration::from_millis(10),
        wheel_size: 10,
    };
    let mut wheel: TimeWheel<i32> = TimeWheel::new(&config);
    wheel.schedule(42, Duration::from_millis(5));
    std::thread::sleep(Duration::from_millis(15));
    let ready = wheel.advance();
    assert!(!ready.is_empty());
    assert_eq!(ready[0].data, 42);
}

#[test]
fn advance_empty_when_not_ready() {
    let config = TimeWheelConfig {
        tick_duration: Duration::from_millis(10),
        wheel_size: 100,
    };
    let mut wheel: TimeWheel<i32> = TimeWheel::new(&config);
    wheel.schedule(1, Duration::from_secs(60));
    let ready = wheel.advance();
    assert!(ready.is_empty());
}

#[test]
fn advance_increments_current_tick() {
    let config = TimeWheelConfig {
        tick_duration: Duration::from_millis(10),
        wheel_size: 10,
    };
    let mut wheel: TimeWheel<i32> = TimeWheel::new(&config);
    assert_eq!(wheel.current_tick(), 0);
    wheel.advance();
    assert_eq!(wheel.current_tick(), 1);
}

#[test]
fn advance_wraps_around() {
    let config = TimeWheelConfig {
        tick_duration: Duration::from_millis(1),
        wheel_size: 3,
    };
    let mut wheel: TimeWheel<i32> = TimeWheel::new(&config);
    wheel.advance();
    wheel.advance();
    wheel.advance();
    assert_eq!(wheel.current_tick(), 0); // wrapped
}

#[test]
fn total_executed_tracks_output() {
    let config = TimeWheelConfig {
        tick_duration: Duration::from_millis(10),
        wheel_size: 10,
    };
    let mut wheel: TimeWheel<i32> = TimeWheel::new(&config);
    wheel.schedule(1, Duration::from_millis(5));
    std::thread::sleep(Duration::from_millis(15));
    wheel.advance();
    assert!(wheel.total_executed() >= 1);
}

// ── Cancel ──────────────────────────────────────────────────────────

#[test]
fn cancel_removes_task() {
    let config = TimeWheelConfig::default();
    let mut wheel: TimeWheel<i32> = TimeWheel::new(&config);
    let id = wheel.schedule(1, Duration::from_secs(10));
    assert_eq!(wheel.pending_count(), 1);
    let cancelled = wheel.cancel(id);
    assert!(cancelled.is_some());
    assert_eq!(cancelled.unwrap().data, 1);
    assert_eq!(wheel.pending_count(), 0);
}

#[test]
fn cancel_nonexistent_returns_none() {
    let config = TimeWheelConfig::default();
    let mut wheel: TimeWheel<i32> = TimeWheel::new(&config);
    assert!(wheel.cancel(999).is_none());
}

// ── Configuration ───────────────────────────────────────────────────

#[test]
fn default_config() {
    let config = TimeWheelConfig::default();
    assert_eq!(config.tick_duration, Duration::from_millis(100));
    assert_eq!(config.wheel_size, 60);
}

#[test]
fn with_tick_duration_constructor() {
    let wheel: TimeWheel<i32> = TimeWheel::with_tick_duration(Duration::from_millis(50));
    assert_eq!(wheel.tick_duration(), Duration::from_millis(50));
    assert_eq!(wheel.wheel_size(), 60);
}

#[test]
fn wheel_size_minimum_is_one() {
    let config = TimeWheelConfig {
        tick_duration: Duration::from_millis(10),
        wheel_size: 0, // should be clamped to 1
    };
    let wheel: TimeWheel<i32> = TimeWheel::new(&config);
    assert_eq!(wheel.wheel_size(), 1);
}

// ── try_advance ─────────────────────────────────────────────────────

#[test]
fn try_advance_false_when_empty() {
    let config = TimeWheelConfig::default();
    let wheel: TimeWheel<i32> = TimeWheel::new(&config);
    assert!(!wheel.try_advance());
}

// ── DelayQueue (inline in timewheel crate) ──────────────────────────

#[test]
fn delay_queue_push_and_len() {
    let mut q: shiplog_timewheel::DelayQueue<i32> = shiplog_timewheel::DelayQueue::new();
    q.push(1, Duration::from_millis(100));
    q.push(2, Duration::from_millis(200));
    assert_eq!(q.len(), 2);
    assert!(!q.is_empty());
}

#[test]
fn delay_queue_pop_ready_empty_initially() {
    let mut q: shiplog_timewheel::DelayQueue<i32> = shiplog_timewheel::DelayQueue::new();
    q.push(1, Duration::from_secs(10));
    assert!(q.pop_ready().is_empty());
}

#[test]
fn delay_queue_pop_ready_after_wait() {
    let mut q: shiplog_timewheel::DelayQueue<i32> = shiplog_timewheel::DelayQueue::new();
    q.push(1, Duration::from_millis(10));
    q.push(2, Duration::from_millis(10));
    std::thread::sleep(Duration::from_millis(30));
    let ready = q.pop_ready();
    assert_eq!(ready.len(), 2);
}

// ── Edge cases ──────────────────────────────────────────────────────

#[test]
fn schedule_zero_delay() {
    let config = TimeWheelConfig {
        tick_duration: Duration::from_millis(10),
        wheel_size: 10,
    };
    let mut wheel: TimeWheel<i32> = TimeWheel::new(&config);
    wheel.schedule(1, Duration::ZERO);
    // Should land in current slot
    assert_eq!(wheel.pending_count(), 1);
}

// ── Property tests ──────────────────────────────────────────────────

proptest! {
    #[test]
    fn schedule_always_increments_total(n in 1usize..50) {
        let config = TimeWheelConfig::default();
        let mut wheel: TimeWheel<usize> = TimeWheel::new(&config);
        for i in 0..n {
            wheel.schedule(i, Duration::from_secs(60));
        }
        prop_assert_eq!(wheel.total_scheduled(), n as u64);
        prop_assert_eq!(wheel.pending_count(), n);
    }

    #[test]
    fn cancel_decrements_pending(n in 1usize..20) {
        let config = TimeWheelConfig::default();
        let mut wheel: TimeWheel<usize> = TimeWheel::new(&config);
        let mut ids = Vec::new();
        for i in 0..n {
            ids.push(wheel.schedule(i, Duration::from_secs(60)));
        }
        for id in &ids {
            wheel.cancel(*id);
        }
        prop_assert_eq!(wheel.pending_count(), 0);
    }

    #[test]
    fn advance_wraps_correctly(wheel_size in 1usize..100, advances in 0usize..500) {
        let config = TimeWheelConfig {
            tick_duration: Duration::from_millis(1),
            wheel_size,
        };
        let mut wheel: TimeWheel<i32> = TimeWheel::new(&config);
        for _ in 0..advances {
            wheel.advance();
        }
        prop_assert!(wheel.current_tick() < wheel.wheel_size());
    }
}
