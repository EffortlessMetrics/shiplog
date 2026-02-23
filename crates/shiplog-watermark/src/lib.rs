//! Watermark utilities for stream processing in shiplog.
//!
//! This crate provides watermark implementations for event time processing in streams.

use chrono::{DateTime, Utc};
use std::collections::VecDeque;

/// A watermark tracks the progress of event time in a stream.
/// It represents the guarantee that all events with timestamps
/// earlier than the watermark have been processed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Watermark {
    timestamp: i64,
}

impl Watermark {
    /// Create a new watermark with the given timestamp (milliseconds since epoch)
    pub fn new(timestamp: i64) -> Self {
        Self { timestamp }
    }

    /// Create a watermark from a DateTime
    pub fn from_datetime(dt: &DateTime<Utc>) -> Self {
        Self {
            timestamp: dt.timestamp_millis(),
        }
    }

    /// Get the timestamp in milliseconds since epoch
    pub fn timestamp_millis(&self) -> i64 {
        self.timestamp
    }

    /// Get the watermark as a DateTime
    pub fn to_datetime(&self) -> DateTime<Utc> {
        DateTime::from_timestamp_millis(self.timestamp).unwrap_or(DateTime::<Utc>::MIN_UTC)
    }

    /// Check if the watermark is earlier than another timestamp
    pub fn is_before(&self, other: i64) -> bool {
        self.timestamp < other
    }

    /// Check if the watermark is later than another timestamp
    pub fn is_after(&self, other: i64) -> bool {
        self.timestamp > other
    }
}

impl Default for Watermark {
    fn default() -> Self {
        Self {
            timestamp: i64::MIN,
        }
    }
}

/// A periodic watermark generator that advances the watermark
/// based on a fixed lag behind the maximum observed timestamp.
pub struct PeriodicWatermarkGenerator {
    lag_ms: i64,
    max_timestamp: i64,
    current_watermark: Watermark,
}

impl PeriodicWatermarkGenerator {
    /// Create a new periodic watermark generator with the given lag (in milliseconds)
    pub fn new(lag_ms: i64) -> Self {
        Self {
            lag_ms,
            max_timestamp: i64::MIN,
            current_watermark: Watermark::default(),
        }
    }

    /// Update the generator with a new timestamp and get the new watermark
    pub fn update(&mut self, timestamp: i64) -> Watermark {
        if timestamp > self.max_timestamp {
            self.max_timestamp = timestamp;
            self.current_watermark = Watermark::new(timestamp - self.lag_ms);
        }
        self.current_watermark
    }

    /// Get the current watermark without updating
    pub fn current(&self) -> Watermark {
        self.current_watermark
    }
}

/// A tumbling watermark generator that advances at fixed intervals.
pub struct TumblingWatermarkGenerator {
    interval_ms: i64,
    next_watermark: i64,
}

impl TumblingWatermarkGenerator {
    /// Create a new tumbling watermark generator with the given interval (in milliseconds)
    pub fn new(interval_ms: i64) -> Self {
        let now = Utc::now().timestamp_millis();
        Self {
            interval_ms,
            next_watermark: now - (now % interval_ms),
        }
    }

    /// Get the current watermark, advancing if necessary
    pub fn current(&mut self) -> Watermark {
        let now = Utc::now().timestamp_millis();

        // Advance to next interval if we've passed it
        while self.next_watermark <= now {
            self.next_watermark += self.interval_ms;
        }

        Watermark::new(self.next_watermark - self.interval_ms)
    }
}

/// A watermark tracker that maintains a history of watermarks
/// for debugging and analysis purposes.
pub struct WatermarkTracker {
    watermarks: VecDeque<Watermark>,
    max_history: usize,
}

impl WatermarkTracker {
    /// Create a new watermark tracker with the specified history size
    pub fn new(max_history: usize) -> Self {
        Self {
            watermarks: VecDeque::with_capacity(max_history),
            max_history,
        }
    }

    /// Add a watermark to the tracker
    pub fn add(&mut self, watermark: Watermark) {
        if self.watermarks.len() >= self.max_history {
            self.watermarks.pop_front();
        }
        self.watermarks.push_back(watermark);
    }

    /// Get all tracked watermarks
    pub fn watermarks(&self) -> Vec<Watermark> {
        self.watermarks.iter().copied().collect()
    }

    /// Get the latest watermark
    pub fn latest(&self) -> Option<Watermark> {
        self.watermarks.back().copied()
    }

    /// Get the earliest watermark
    pub fn earliest(&self) -> Option<Watermark> {
        self.watermarks.front().copied()
    }
}

impl Default for WatermarkTracker {
    fn default() -> Self {
        Self::new(100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watermark_creation() {
        let wm = Watermark::new(1000);
        assert_eq!(wm.timestamp_millis(), 1000);
    }

    #[test]
    fn test_watermark_default() {
        let wm: Watermark = Default::default();
        assert_eq!(wm.timestamp_millis(), i64::MIN);
    }

    #[test]
    fn test_watermark_from_datetime() {
        let dt = Utc::now();
        let wm = Watermark::from_datetime(&dt);
        assert!(wm.timestamp_millis() > 0);
    }

    #[test]
    fn test_watermark_comparison() {
        let wm = Watermark::new(1000);
        assert!(wm.is_before(2000));
        assert!(!wm.is_before(500));
        assert!(wm.is_after(500));
        assert!(!wm.is_after(2000));
    }

    #[test]
    fn test_periodic_watermark_generator() {
        let mut generator = PeriodicWatermarkGenerator::new(100);

        // Initial update
        let wm1 = generator.update(1000);
        assert_eq!(wm1.timestamp_millis(), 900); // 1000 - 100 lag

        // Advance with higher timestamp
        let wm2 = generator.update(1500);
        assert_eq!(wm2.timestamp_millis(), 1400); // 1500 - 100 lag

        // No advance with lower timestamp
        let wm3 = generator.update(1200);
        assert_eq!(wm3.timestamp_millis(), 1400); // Still the previous watermark
    }

    #[test]
    fn test_tumbling_watermark_generator() {
        let mut generator = TumblingWatermarkGenerator::new(1000);
        let wm = generator.current();

        // Just verify it returns a valid watermark (not checking exact alignment)
        assert!(wm.timestamp_millis() > 0);
    }

    #[test]
    fn test_watermark_tracker() {
        let mut tracker = WatermarkTracker::new(3);

        tracker.add(Watermark::new(100));
        tracker.add(Watermark::new(200));
        tracker.add(Watermark::new(300));

        assert_eq!(tracker.latest().unwrap().timestamp_millis(), 300);
        assert_eq!(tracker.earliest().unwrap().timestamp_millis(), 100);

        // Add one more to exceed max history
        tracker.add(Watermark::new(400));

        assert_eq!(tracker.earliest().unwrap().timestamp_millis(), 200);
    }

    #[test]
    fn test_watermark_tracker_default() {
        let tracker: WatermarkTracker = Default::default();
        assert!(tracker.latest().is_none());
        assert!(tracker.earliest().is_none());
    }
}
