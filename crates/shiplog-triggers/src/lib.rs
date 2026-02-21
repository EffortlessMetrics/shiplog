//! Trigger utilities for stream processing in shiplog.
//!
//! This crate provides trigger implementations for controlling when
//! window results are emitted during stream processing.

use chrono::Utc;

/// Result of a trigger evaluation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerResult {
    /// Trigger fired, should emit results
    Fire,
    /// Trigger did not fire
    Continue,
    /// Trigger is finished, should not receive more elements
    Finish,
}

/// A trigger that controls when window results are emitted
pub trait Trigger: Send + Sync {
    /// Evaluate the trigger at the given timestamp
    fn evaluate(&mut self, timestamp: i64) -> TriggerResult;
    
    /// Reset the trigger state
    fn reset(&mut self);
}

/// A count-based trigger that fires after a fixed number of elements
pub struct CountTrigger {
    count: usize,
    current_count: usize,
}

impl CountTrigger {
    pub fn new(count: usize) -> Self {
        Self {
            count,
            current_count: 0,
        }
    }
}

impl Trigger for CountTrigger {
    fn evaluate(&mut self, _timestamp: i64) -> TriggerResult {
        if self.current_count >= self.count {
            TriggerResult::Fire
        } else {
            TriggerResult::Continue
        }
    }

    fn reset(&mut self) {
        self.current_count = 0;
    }
}

impl CountTrigger {
    /// Increment the count and return whether it should fire
    pub fn increment(&mut self) -> bool {
        self.current_count += 1;
        self.current_count >= self.count
    }
}

/// A time-based trigger that fires at fixed intervals
pub struct TimeTrigger {
    interval_ms: i64,
    next_fire_time: i64,
}

impl TimeTrigger {
    pub fn new(interval_ms: i64) -> Self {
        let now = Utc::now().timestamp_millis();
        Self {
            interval_ms,
            next_fire_time: now + interval_ms,
        }
    }
}

impl Trigger for TimeTrigger {
    fn evaluate(&mut self, timestamp: i64) -> TriggerResult {
        if timestamp >= self.next_fire_time {
            TriggerResult::Fire
        } else {
            TriggerResult::Continue
        }
    }

    fn reset(&mut self) {
        let now = Utc::now().timestamp_millis();
        self.next_fire_time = now + self.interval_ms;
    }
}

impl TimeTrigger {
    /// Update the trigger and get the next fire time
    pub fn update(&mut self, timestamp: i64) -> i64 {
        while self.next_fire_time <= timestamp {
            self.next_fire_time += self.interval_ms;
        }
        self.next_fire_time
    }
}

/// A watermark-based trigger that fires when the watermark passes a threshold
pub struct WatermarkTrigger {
    threshold: i64,
    has_fired: bool,
}

impl WatermarkTrigger {
    pub fn new(threshold: i64) -> Self {
        Self {
            threshold,
            has_fired: false,
        }
    }
}

impl Trigger for WatermarkTrigger {
    fn evaluate(&mut self, watermark_timestamp: i64) -> TriggerResult {
        if !self.has_fired && watermark_timestamp >= self.threshold {
            self.has_fired = true;
            TriggerResult::Fire
        } else if self.has_fired {
            TriggerResult::Finish
        } else {
            TriggerResult::Continue
        }
    }

    fn reset(&mut self) {
        self.has_fired = false;
    }
}

/// A composite trigger that combines multiple triggers with AND logic
pub struct AndTrigger<T: Trigger, U: Trigger> {
    first: T,
    second: U,
}

impl<T: Trigger, U: Trigger> AndTrigger<T, U> {
    pub fn new(first: T, second: U) -> Self {
        Self { first, second }
    }
}

impl<T: Trigger, U: Trigger> Trigger for AndTrigger<T, U> {
    fn evaluate(&mut self, timestamp: i64) -> TriggerResult {
        match (self.first.evaluate(timestamp), self.second.evaluate(timestamp)) {
            (TriggerResult::Fire, TriggerResult::Fire) => TriggerResult::Fire,
            (TriggerResult::Finish, _) | (_, TriggerResult::Finish) => TriggerResult::Finish,
            _ => TriggerResult::Continue,
        }
    }

    fn reset(&mut self) {
        self.first.reset();
        self.second.reset();
    }
}

/// A composite trigger that combines multiple triggers with OR logic
pub struct OrTrigger<T: Trigger, U: Trigger> {
    first: T,
    second: U,
}

impl<T: Trigger, U: Trigger> OrTrigger<T, U> {
    pub fn new(first: T, second: U) -> Self {
        Self { first, second }
    }
}

impl<T: Trigger, U: Trigger> Trigger for OrTrigger<T, U> {
    fn evaluate(&mut self, timestamp: i64) -> TriggerResult {
        match (self.first.evaluate(timestamp), self.second.evaluate(timestamp)) {
            (TriggerResult::Fire, _) | (_, TriggerResult::Fire) => TriggerResult::Fire,
            (TriggerResult::Finish, TriggerResult::Finish) => TriggerResult::Finish,
            _ => TriggerResult::Continue,
        }
    }

    fn reset(&mut self) {
        self.first.reset();
        self.second.reset();
    }
}

/// A repeated trigger that fires repeatedly at a fixed interval after an initial delay
pub struct RepeatedTimeTrigger {
    delay_ms: i64,
    interval_ms: i64,
    next_fire_time: i64,
}

impl RepeatedTimeTrigger {
    pub fn new(delay_ms: i64, interval_ms: i64) -> Self {
        let now = Utc::now().timestamp_millis();
        Self {
            delay_ms,
            interval_ms,
            next_fire_time: now + delay_ms,
        }
    }
}

impl Trigger for RepeatedTimeTrigger {
    fn evaluate(&mut self, timestamp: i64) -> TriggerResult {
        if timestamp >= self.next_fire_time {
            TriggerResult::Fire
        } else {
            TriggerResult::Continue
        }
    }

    fn reset(&mut self) {
        let now = Utc::now().timestamp_millis();
        self.next_fire_time = now + self.delay_ms;
    }
}

impl RepeatedTimeTrigger {
    /// Update the trigger after firing
    pub fn on_fire(&mut self) {
        self.next_fire_time += self.interval_ms;
    }
}

/// A never trigger that never fires
pub struct NeverTrigger;

impl NeverTrigger {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NeverTrigger {
    fn default() -> Self {
        Self::new()
    }
}

impl Trigger for NeverTrigger {
    fn evaluate(&mut self, _timestamp: i64) -> TriggerResult {
        TriggerResult::Continue
    }

    fn reset(&mut self) {}
}

/// A always trigger that always fires
pub struct AlwaysTrigger;

impl AlwaysTrigger {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AlwaysTrigger {
    fn default() -> Self {
        Self::new()
    }
}

impl Trigger for AlwaysTrigger {
    fn evaluate(&mut self, _timestamp: i64) -> TriggerResult {
        TriggerResult::Fire
    }

    fn reset(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_trigger() {
        let mut trigger = CountTrigger::new(3);
        
        assert_eq!(trigger.evaluate(0), TriggerResult::Continue);
        
        assert!(!trigger.increment());
        assert_eq!(trigger.evaluate(0), TriggerResult::Continue);
        
        assert!(!trigger.increment());
        assert_eq!(trigger.evaluate(0), TriggerResult::Continue);
        
        assert!(trigger.increment());
        assert_eq!(trigger.evaluate(0), TriggerResult::Fire);
    }

    #[test]
    fn test_count_trigger_reset() {
        let mut trigger = CountTrigger::new(2);
        
        trigger.increment();
        trigger.increment();
        assert_eq!(trigger.evaluate(0), TriggerResult::Fire);
        
        trigger.reset();
        assert_eq!(trigger.evaluate(0), TriggerResult::Continue);
    }

    #[test]
    fn test_time_trigger() {
        let mut trigger = TimeTrigger::new(1000);
        
        // Should not fire immediately
        assert_eq!(trigger.evaluate(Utc::now().timestamp_millis()), TriggerResult::Continue);
    }

    #[test]
    fn test_watermark_trigger() {
        let mut trigger = WatermarkTrigger::new(1000);
        
        assert_eq!(trigger.evaluate(500), TriggerResult::Continue);
        assert_eq!(trigger.evaluate(1000), TriggerResult::Fire);
        assert_eq!(trigger.evaluate(1500), TriggerResult::Finish);
    }

    #[test]
    fn test_watermark_trigger_reset() {
        let mut trigger = WatermarkTrigger::new(1000);
        
        trigger.evaluate(1500);
        assert_eq!(trigger.evaluate(1500), TriggerResult::Finish);
        
        trigger.reset();
        assert_eq!(trigger.evaluate(500), TriggerResult::Continue);
        assert_eq!(trigger.evaluate(1000), TriggerResult::Fire);
    }

    #[test]
    fn test_and_trigger() {
        let mut trigger = AndTrigger::new(CountTrigger::new(2), CountTrigger::new(3));
        
        // Both need to fire
        assert_eq!(trigger.evaluate(0), TriggerResult::Continue);
    }

    #[test]
    fn test_or_trigger() {
        let mut trigger = OrTrigger::new(CountTrigger::new(2), CountTrigger::new(3));
        
        // Either one firing triggers
        assert_eq!(trigger.evaluate(0), TriggerResult::Continue);
    }

    #[test]
    fn test_never_trigger() {
        let mut trigger = NeverTrigger::new();
        
        assert_eq!(trigger.evaluate(0), TriggerResult::Continue);
        assert_eq!(trigger.evaluate(i64::MAX), TriggerResult::Continue);
    }

    #[test]
    fn test_always_trigger() {
        let mut trigger = AlwaysTrigger::new();
        
        assert_eq!(trigger.evaluate(0), TriggerResult::Fire);
        assert_eq!(trigger.evaluate(i64::MAX), TriggerResult::Fire);
    }

    #[test]
    fn test_repeated_time_trigger() {
        let mut trigger = RepeatedTimeTrigger::new(100, 50);
        
        // First evaluation should not fire (delay not passed)
        let now = Utc::now().timestamp_millis();
        
        trigger.on_fire();
        
        // After firing, should fire again at interval
        let result = trigger.evaluate(now + 200);
        assert_eq!(result, TriggerResult::Fire);
    }
}
