//! Time wheel scheduler for shiplog.
//!
//! This crate provides a time wheel implementation for efficient
//! timer and scheduler management.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// A scheduled task in the time wheel
#[derive(Debug, Clone)]
pub struct ScheduledTask<T> {
    pub id: u64,
    pub data: T,
    pub delay: Duration,
    pub scheduled_time: Instant,
}

impl<T> ScheduledTask<T> {
    pub fn new(id: u64, data: T, delay: Duration) -> Self {
        Self {
            id,
            data,
            delay,
            scheduled_time: Instant::now() + delay,
        }
    }

    pub fn is_ready(&self) -> bool {
        Instant::now() >= self.scheduled_time
    }

    pub fn remaining_time(&self) -> Duration {
        let elapsed = self.scheduled_time.elapsed();
        if elapsed > self.delay {
            Duration::ZERO
        } else {
            self.delay - elapsed
        }
    }
}

/// A slot in the time wheel
#[derive(Debug, Clone, Default)]
struct WheelSlot<T> {
    tasks: VecDeque<ScheduledTask<T>>,
}

impl<T> WheelSlot<T> {
    fn new() -> Self {
        Self {
            tasks: VecDeque::new(),
        }
    }

    fn add_task(&mut self, task: ScheduledTask<T>) {
        self.tasks.push_back(task);
    }

    fn take_ready_tasks(&mut self) -> Vec<ScheduledTask<T>> {
        let mut ready = Vec::new();
        let now = Instant::now();

        while let Some(task) = self.tasks.pop_front() {
            if now >= task.scheduled_time {
                ready.push(task);
            } else {
                self.tasks.push_front(task);
                break;
            }
        }

        ready
    }

    fn len(&self) -> usize {
        self.tasks.len()
    }

    fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
}

/// Time wheel configuration
#[derive(Debug, Clone)]
pub struct TimeWheelConfig {
    pub tick_duration: Duration,
    pub wheel_size: usize,
}

impl Default for TimeWheelConfig {
    fn default() -> Self {
        Self {
            tick_duration: Duration::from_millis(100),
            wheel_size: 60,
        }
    }
}

/// A hierarchical time wheel scheduler
pub struct TimeWheel<T> {
    slots: Vec<WheelSlot<T>>,
    current_tick: usize,
    tick_duration: Duration,
    next_task_id: u64,
    total_scheduled: u64,
    total_executed: u64,
}

impl<T> TimeWheel<T> {
    pub fn new(config: &TimeWheelConfig) -> Self {
        let wheel_size = config.wheel_size.max(1);
        let mut slots = Vec::with_capacity(wheel_size);
        for _ in 0..wheel_size {
            slots.push(WheelSlot::new());
        }

        Self {
            slots,
            current_tick: 0,
            tick_duration: config.tick_duration,
            next_task_id: 0,
            total_scheduled: 0,
            total_executed: 0,
        }
    }

    pub fn with_tick_duration(tick_duration: Duration) -> Self {
        Self::new(&TimeWheelConfig {
            tick_duration,
            wheel_size: 60,
        })
    }

    /// Schedule a task to run after the given delay
    pub fn schedule(&mut self, data: T, delay: Duration) -> u64 {
        let id = self.next_task_id;
        self.next_task_id += 1;

        // Calculate which slot the task belongs to
        let ticks = (delay.as_millis() / self.tick_duration.as_millis()) as usize;
        let slot_index = (self.current_tick + ticks) % self.slots.len();

        let task = ScheduledTask::new(id, data, delay);
        self.slots[slot_index].add_task(task);
        self.total_scheduled += 1;

        id
    }

    /// Advance the time wheel and return ready tasks
    pub fn advance(&mut self) -> Vec<ScheduledTask<T>> {
        let mut ready = Vec::new();

        // Get tasks from current slot using the slot's method
        let slot_ready = self.slots[self.current_tick].take_ready_tasks();
        ready.extend(slot_ready);

        // Move to next tick
        self.current_tick = (self.current_tick + 1) % self.slots.len();

        self.total_executed += ready.len() as u64;
        ready
    }

    /// Try to advance with a maximum wait time
    pub fn try_advance(&self) -> bool {
        // Check if current slot has ready tasks
        !self.slots[self.current_tick].is_empty()
    }

    /// Get the current tick position
    pub fn current_tick(&self) -> usize {
        self.current_tick
    }

    /// Get the number of pending tasks
    pub fn pending_count(&self) -> usize {
        self.slots.iter().map(|slot| slot.len()).sum()
    }

    /// Get total number of scheduled tasks
    pub fn total_scheduled(&self) -> u64 {
        self.total_scheduled
    }

    /// Get total number of executed tasks
    pub fn total_executed(&self) -> u64 {
        self.total_executed
    }

    /// Get the wheel size
    pub fn wheel_size(&self) -> usize {
        self.slots.len()
    }

    /// Get the tick duration
    pub fn tick_duration(&self) -> Duration {
        self.tick_duration
    }

    /// Cancel a task by id (returns the task if found)
    pub fn cancel(&mut self, task_id: u64) -> Option<ScheduledTask<T>> {
        for slot in &mut self.slots {
            if let Some(pos) = slot.tasks.iter().position(|t| t.id == task_id) {
                return slot.tasks.remove(pos);
            }
        }
        None
    }
}

/// Simple delay queue for immediate use
pub struct DelayQueue<T> {
    queue: Vec<(Instant, T)>,
}

impl<T> DelayQueue<T> {
    pub fn new() -> Self {
        Self { queue: Vec::new() }
    }

    /// Push an item with a delay
    pub fn push(&mut self, item: T, delay: Duration) {
        let deadline = Instant::now() + delay;
        self.queue.push((deadline, item));
        self.queue.sort_by_key(|(deadline, _)| *deadline);
    }

    /// Pop ready items
    pub fn pop_ready(&mut self) -> Vec<T> {
        let now = Instant::now();
        let mut ready = Vec::new();
        let mut not_ready = Vec::new();

        std::mem::swap(&mut self.queue, &mut not_ready);

        for (deadline, item) in not_ready {
            if deadline <= now {
                ready.push(item);
            } else {
                self.queue.push((deadline, item));
            }
        }

        ready
    }

    /// Peek at the next ready time
    pub fn next_ready_time(&self) -> Option<Duration> {
        self.queue.first().map(|(deadline, _)| {
            let elapsed = deadline.elapsed();
            if elapsed.is_zero() {
                Duration::ZERO
            } else {
                elapsed
            }
        })
    }

    /// Get queue length
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

impl<T> Default for DelayQueue<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_scheduled_task() {
        let task = ScheduledTask::new(1, "test", Duration::from_secs(1));

        assert_eq!(task.id, 1);
        assert!(!task.is_ready());
    }

    #[test]
    fn test_time_wheel_basic() {
        let config = TimeWheelConfig {
            tick_duration: Duration::from_millis(10),
            wheel_size: 10,
        };

        let mut wheel: TimeWheel<&str> = TimeWheel::new(&config);

        // Schedule a task
        let id = wheel.schedule("task1", Duration::from_millis(50));
        assert_eq!(id, 0);

        assert_eq!(wheel.pending_count(), 1);
        assert_eq!(wheel.total_scheduled(), 1);
    }

    #[test]
    fn test_time_wheel_advance() {
        let config = TimeWheelConfig {
            tick_duration: Duration::from_millis(10),
            wheel_size: 100,
        };

        let mut wheel: TimeWheel<i32> = TimeWheel::new(&config);

        wheel.schedule(1, Duration::from_millis(50));

        // Advance a few ticks without waiting long enough
        let ready = wheel.advance();
        assert!(ready.is_empty());
    }

    #[test]
    fn test_time_wheel_advance_ready() {
        let config = TimeWheelConfig {
            tick_duration: Duration::from_millis(10),
            wheel_size: 10,
        };

        let mut wheel: TimeWheel<i32> = TimeWheel::new(&config);

        wheel.schedule(1, Duration::from_millis(5));

        // Wait and advance
        thread::sleep(Duration::from_millis(10));

        let ready = wheel.advance();
        assert!(!ready.is_empty());
        assert_eq!(ready[0].data, 1);
    }

    #[test]
    fn test_time_wheel_cancel() {
        let config = TimeWheelConfig::default();
        let mut wheel: TimeWheel<i32> = TimeWheel::new(&config);

        let id = wheel.schedule(1, Duration::from_secs(10));
        assert_eq!(wheel.pending_count(), 1);

        let cancelled = wheel.cancel(id);
        assert!(cancelled.is_some());
        assert_eq!(wheel.pending_count(), 0);
    }

    #[test]
    fn test_delay_queue_basic() {
        let mut queue: DelayQueue<i32> = DelayQueue::new();

        queue.push(1, Duration::from_millis(100));
        queue.push(2, Duration::from_millis(50));

        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn test_delay_queue_pop_ready() {
        let mut queue: DelayQueue<i32> = DelayQueue::new();

        queue.push(1, Duration::from_millis(100));
        queue.push(2, Duration::from_millis(10));

        // Pop immediately - nothing ready
        let ready = queue.pop_ready();
        assert!(ready.is_empty());

        // Wait and pop
        thread::sleep(Duration::from_millis(150));

        let ready = queue.pop_ready();
        assert_eq!(ready.len(), 2);
    }

    #[test]
    fn test_delay_queue_sorted() {
        let mut queue: DelayQueue<i32> = DelayQueue::new();

        queue.push(1, Duration::from_millis(100));
        queue.push(2, Duration::from_millis(50));

        let next_time = queue.next_ready_time();
        assert!(next_time.is_some());
    }

    #[test]
    fn test_time_wheel_statistics() {
        let config = TimeWheelConfig::default();
        let mut wheel: TimeWheel<i32> = TimeWheel::new(&config);

        wheel.schedule(1, Duration::from_millis(50));

        assert_eq!(wheel.total_scheduled(), 1);
        assert_eq!(wheel.total_executed(), 0);
    }
}
