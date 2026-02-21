//! Advanced sliding window implementation for shiplog.
//!
//! This crate provides advanced sliding window data structures with
//! timestamp-based expiration and various window strategies.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// A timestamped item for time-based window operations
#[derive(Debug, Clone)]
pub struct Timestamped<T> {
    pub value: T,
    pub timestamp: Instant,
}

impl<T> Timestamped<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            timestamp: Instant::now(),
        }
    }

    pub fn with_timestamp(value: T, timestamp: Instant) -> Self {
        Self { value, timestamp }
    }

    pub fn age(&self) -> Duration {
        self.timestamp.elapsed()
    }

    pub fn is_expired(&self, max_age: Duration) -> bool {
        self.age() > max_age
    }
}

/// Advanced sliding window with timestamp-based expiration
pub struct TimeSlidingWindow<T> {
    window: VecDeque<Timestamped<T>>,
    max_size: usize,
    max_age: Duration,
}

impl<T> TimeSlidingWindow<T> {
    pub fn new(max_size: usize, max_age: Duration) -> Self {
        Self {
            window: VecDeque::new(),
            max_size,
            max_age,
        }
    }

    /// Add an item to the window
    pub fn push(&mut self, value: T) {
        // Remove expired items
        self.prune_expired();
        
        // Remove oldest if at capacity
        if self.window.len() >= self.max_size {
            self.window.pop_front();
        }
        
        self.window.push_back(Timestamped::new(value));
    }

    /// Remove expired items based on max_age
    pub fn prune_expired(&mut self) {
        while let Some(item) = self.window.front() {
            if item.is_expired(self.max_age) {
                self.window.pop_front();
            } else {
                break;
            }
        }
    }

    /// Get all valid items in the window
    pub fn get_valid(&self) -> Vec<&T> {
        let now = Instant::now();
        self.window
            .iter()
            .filter(|item| now.duration_since(item.timestamp) <= self.max_age)
            .map(|item| &item.value)
            .collect()
    }

    /// Get the current number of valid items
    pub fn len(&self) -> usize {
        // Can't prune in const method, just return raw length
        self.window.len()
    }

    /// Check if window is empty
    pub fn is_empty(&self) -> bool {
        self.window.is_empty()
    }

    /// Clear the window
    pub fn clear(&mut self) {
        self.window.clear();
    }
}

/// Window strategy types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowStrategy {
    /// Items are evicted when window is full (FIFO)
    TailDrop,
    /// Items are evicted based on age
    TimeBased,
    /// Both size and time are considered
    Hybrid,
}

/// Configurable sliding window with strategy
pub struct ConfigurableSlidingWindow<T> {
    window: VecDeque<Timestamped<T>>,
    max_size: usize,
    strategy: WindowStrategy,
    max_age: Duration,
}

impl<T> ConfigurableSlidingWindow<T> {
    pub fn new(max_size: usize) -> Self {
        Self {
            window: VecDeque::new(),
            max_size,
            strategy: WindowStrategy::TailDrop,
            max_age: Duration::from_secs(60),
        }
    }

    pub fn with_strategy(mut self, strategy: WindowStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn with_max_age(mut self, max_age: Duration) -> Self {
        self.max_age = max_age;
        self
    }

    /// Add an item based on the configured strategy
    pub fn push(&mut self, item: T) -> Option<T> {
        let evicted = match self.strategy {
            WindowStrategy::TailDrop => self.push_tail_drop(item),
            WindowStrategy::TimeBased => self.push_time_based(item),
            WindowStrategy::Hybrid => self.push_hybrid(item),
        };
        evicted
    }

    fn push_tail_drop(&mut self, item: T) -> Option<T> {
        let evicted = if self.window.len() >= self.max_size {
            self.window.pop_front().map(|ts| ts.value)
        } else {
            None
        };
        self.window.push_back(Timestamped::new(item));
        evicted
    }

    fn push_time_based(&mut self, item: T) -> Option<T> {
        let now = Instant::now();
        
        // Remove expired items
        while let Some(ts) = self.window.front() {
            if now.duration_since(ts.timestamp) > self.max_age {
                self.window.pop_front();
            } else {
                break;
            }
        }
        
        self.window.push_back(Timestamped::new(item));
        None
    }

    fn push_hybrid(&mut self, item: T) -> Option<T> {
        let now = Instant::now();
        
        // Remove expired items first
        while let Some(ts) = self.window.front() {
            if now.duration_since(ts.timestamp) > self.max_age {
                self.window.pop_front();
            } else {
                break;
            }
        }
        
        // Then handle size-based eviction
        let evicted = if self.window.len() >= self.max_size {
            self.window.pop_front().map(|ts| ts.value)
        } else {
            None
        };
        
        self.window.push_back(Timestamped::new(item));
        evicted
    }

    /// Get current window contents
    pub fn get(&self) -> Vec<&T> {
        self.window.iter().map(|item| &item.value).collect()
    }

    /// Get window length
    pub fn len(&self) -> usize {
        self.window.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.window.is_empty()
    }
}

/// A window that tracks statistics
pub struct WindowWithStats<T> {
    window: VecDeque<T>,
    max_size: usize,
    pub stats: WindowStatistics,
}

#[derive(Debug, Default, Clone)]
pub struct WindowStatistics {
    pub total_pushes: u64,
    pub total_evictions: u64,
    pub total_lookups: u64,
}

impl<T> WindowWithStats<T> {
    pub fn new(max_size: usize) -> Self {
        Self {
            window: VecDeque::new(),
            max_size,
            stats: WindowStatistics::default(),
        }
    }

    pub fn push(&mut self, item: T) {
        self.stats.total_pushes += 1;
        
        if self.window.len() >= self.max_size {
            self.window.pop_front();
            self.stats.total_evictions += 1;
        }
        
        self.window.push_back(item);
    }

    pub fn get(&mut self) -> Vec<&T> {
        self.stats.total_lookups += 1;
        self.window.iter().collect()
    }

    pub fn len(&self) -> usize {
        self.window.len()
    }

    pub fn is_empty(&self) -> bool {
        self.window.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_timestamped() {
        let item = Timestamped::new(42);
        assert_eq!(item.value, 42);
        assert!(!item.is_expired(Duration::from_secs(1)));
    }

    #[test]
    fn test_time_sliding_window_basic() {
        let mut window: TimeSlidingWindow<i32> = TimeSlidingWindow::new(3, Duration::from_secs(10));
        
        window.push(1);
        window.push(2);
        window.push(3);
        
        assert_eq!(window.len(), 3);
        
        let valid = window.get_valid();
        assert_eq!(valid, vec![&1, &2, &3]);
    }

    #[test]
    fn test_time_sliding_window_eviction() {
        let mut window: TimeSlidingWindow<i32> = TimeSlidingWindow::new(2, Duration::from_secs(10));
        
        window.push(1);
        window.push(2);
        window.push(3); // Should evict 1
        
        assert_eq!(window.len(), 2);
        
        let valid = window.get_valid();
        assert_eq!(valid, vec![&2, &3]);
    }

    #[test]
    fn test_time_sliding_window_expiration() {
        let mut window: TimeSlidingWindow<i32> = TimeSlidingWindow::new(5, Duration::from_millis(50));
        
        window.push(1);
        window.push(2);
        
        thread::sleep(Duration::from_millis(60));
        
        assert_eq!(window.len(), 2); // Still has items
        assert!(window.get_valid().is_empty()); // But all expired
    }

    #[test]
    fn test_tail_drop_strategy() {
        let mut window: ConfigurableSlidingWindow<i32> = 
            ConfigurableSlidingWindow::new(2)
                .with_strategy(WindowStrategy::TailDrop);
        
        window.push(1);
        window.push(2);
        window.push(3); // Should evict 1
        
        let items = window.get();
        assert_eq!(items, vec![&2, &3]);
    }

    #[test]
    fn test_time_based_strategy() {
        let mut window: ConfigurableSlidingWindow<i32> = 
            ConfigurableSlidingWindow::new(5)
                .with_strategy(WindowStrategy::TimeBased)
                .with_max_age(Duration::from_millis(50));
        
        window.push(1);
        window.push(2);
        
        thread::sleep(Duration::from_millis(60));
        
        window.push(3);
        
        let items = window.get();
        assert_eq!(items.len(), 1); // Only item 3 is valid
    }

    #[test]
    fn test_hybrid_strategy() {
        let mut window: ConfigurableSlidingWindow<i32> = 
            ConfigurableSlidingWindow::new(2)
                .with_strategy(WindowStrategy::Hybrid)
                .with_max_age(Duration::from_secs(10));
        
        window.push(1);
        window.push(2);
        window.push(3); // Size eviction
        
        let items = window.get();
        assert_eq!(items, vec![&2, &3]);
    }

    #[test]
    fn test_window_with_stats() {
        let mut window: WindowWithStats<i32> = WindowWithStats::new(3);
        
        window.push(1);
        window.push(2);
        window.push(3);
        window.push(4); // Should trigger eviction
        
        assert_eq!(window.stats.total_pushes, 4);
        assert_eq!(window.stats.total_evictions, 1);
        
        let _ = window.get(); // Trigger lookup
        assert_eq!(window.stats.total_lookups, 1);
    }

    #[test]
    fn test_configurable_window_builder() {
        let window: ConfigurableSlidingWindow<i32> = 
            ConfigurableSlidingWindow::new(100)
                .with_strategy(WindowStrategy::Hybrid)
                .with_max_age(Duration::from_secs(30));
        
        assert_eq!(window.len(), 0);
        assert!(window.is_empty());
    }
}
