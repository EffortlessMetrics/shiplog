//! Sliding window utilities for shiplog.
//!
//! This crate provides sliding window utilities for time-series and sequence processing.

use std::collections::VecDeque;

/// Configuration for sliding window
#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub size: usize,
    pub step: usize,
    pub name: String,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            size: 10,
            step: 1,
            name: "window".to_string(),
        }
    }
}

/// Builder for creating window configurations
#[derive(Debug)]
pub struct WindowBuilder {
    config: WindowConfig,
}

impl WindowBuilder {
    pub fn new() -> Self {
        Self {
            config: WindowConfig::default(),
        }
    }

    pub fn size(mut self, size: usize) -> Self {
        self.config.size = size;
        self
    }

    pub fn step(mut self, step: usize) -> Self {
        self.config.step = step;
        self
    }

    pub fn name(mut self, name: &str) -> Self {
        self.config.name = name.to_string();
        self
    }

    pub fn build(self) -> WindowConfig {
        self.config
    }
}

impl Default for WindowBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// A sliding window for processing sequences
pub struct SlidingWindow<T> {
    data: VecDeque<T>,
    size: usize,
    step: usize,
    name: String,
}

impl<T> SlidingWindow<T> {
    pub fn new(size: usize) -> Self {
        Self {
            data: VecDeque::new(),
            size,
            step: 1,
            name: "window".to_string(),
        }
    }

    pub fn with_config(config: &WindowConfig) -> Self {
        Self {
            data: VecDeque::new(),
            size: config.size,
            step: config.step,
            name: config.name.clone(),
        }
    }

    pub fn with_step(mut self, step: usize) -> Self {
        self.step = step;
        self
    }

    /// Add an item to the window
    pub fn push(&mut self, item: T) -> Option<T> {
        if self.data.len() >= self.size {
            self.data.pop_front();
        }
        self.data.push_back(item);
        None
    }

    /// Get the current window contents
    pub fn get_window(&self) -> Vec<&T> {
        self.data.iter().collect()
    }

    /// Get the current window as owned values
    pub fn to_vec(&self) -> Vec<T>
    where
        T: Clone,
    {
        self.data.iter().cloned().collect()
    }

    /// Check if window is full
    pub fn is_full(&self) -> bool {
        self.data.len() >= self.size
    }

    /// Get window size
    pub fn size(&self) -> usize {
        self.size
    }

    /// Get current number of elements
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if window is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get the name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Clear the window
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Apply a function to each element in the window
    pub fn map<U, F>(&self, f: F) -> Vec<U>
    where
        F: FnMut(&T) -> U,
    {
        self.data.iter().map(f).collect()
    }
}

/// A tumbling window that emits windows at fixed intervals
pub struct TumblingWindow<T> {
    data: Vec<T>,
    size: usize,
    name: String,
}

impl<T> TumblingWindow<T> {
    pub fn new(size: usize) -> Self {
        Self {
            data: Vec::new(),
            size,
            name: "tumbling-window".to_string(),
        }
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    /// Add an item and return a window if ready
    pub fn push(&mut self, item: T) -> Option<Vec<T>> {
        self.data.push(item);

        if self.data.len() >= self.size {
            Some(std::mem::take(&mut self.data))
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

/// Window statistics
#[derive(Debug, Default, Clone)]
pub struct WindowStats {
    pub windows_created: u64,
    pub items_processed: u64,
}

impl WindowStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_window(&mut self) {
        self.windows_created += 1;
    }

    pub fn record_item(&mut self) {
        self.items_processed += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_config_default() {
        let config = WindowConfig::default();
        assert_eq!(config.size, 10);
        assert_eq!(config.step, 1);
        assert_eq!(config.name, "window");
    }

    #[test]
    fn test_window_builder() {
        let config = WindowBuilder::new()
            .size(20)
            .step(2)
            .name("test-window")
            .build();

        assert_eq!(config.size, 20);
        assert_eq!(config.step, 2);
        assert_eq!(config.name, "test-window");
    }

    #[test]
    fn test_sliding_window_basic() {
        let mut window: SlidingWindow<i32> = SlidingWindow::new(3);

        window.push(1);
        window.push(2);

        assert_eq!(window.len(), 2);
        assert!(!window.is_full());

        window.push(3);
        assert!(window.is_full());

        let items: Vec<i32> = window.to_vec();
        assert_eq!(items, vec![1, 2, 3]);
    }

    #[test]
    fn test_sliding_window_overflow() {
        let mut window: SlidingWindow<i32> = SlidingWindow::new(3);

        window.push(1);
        window.push(2);
        window.push(3);
        window.push(4); // Should drop 1

        let items: Vec<i32> = window.to_vec();
        assert_eq!(items, vec![2, 3, 4]);
    }

    #[test]
    fn test_sliding_window_get() {
        let mut window: SlidingWindow<i32> = SlidingWindow::new(3);

        window.push(1);
        window.push(2);

        let window_refs = window.get_window();
        assert_eq!(window_refs, vec![&1, &2]);
    }

    #[test]
    fn test_sliding_window_map() {
        let mut window: SlidingWindow<i32> = SlidingWindow::new(3);

        window.push(1);
        window.push(2);
        window.push(3);

        let doubled: Vec<i32> = window.map(|x| x * 2);
        assert_eq!(doubled, vec![2, 4, 6]);
    }

    #[test]
    fn test_tumbling_window() {
        let mut window: TumblingWindow<i32> = TumblingWindow::new(3);

        assert!(window.push(1).is_none());
        assert!(window.push(2).is_none());

        let result = window.push(3);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), vec![1, 2, 3]);

        // Window should be reset
        assert!(window.is_empty());
    }

    #[test]
    fn test_window_stats() {
        let mut stats = WindowStats::new();

        stats.record_item();
        stats.record_item();
        stats.record_window();

        assert_eq!(stats.items_processed, 2);
        assert_eq!(stats.windows_created, 1);
    }

    #[test]
    fn test_sliding_window_with_config() {
        let config = WindowBuilder::new().size(5).step(2).name("custom").build();

        let window: SlidingWindow<i32> = SlidingWindow::with_config(&config);

        assert_eq!(window.size(), 5);
        assert_eq!(window.name(), "custom");
    }
}
