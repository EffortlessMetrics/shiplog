//! Windowing utilities for stream processing in shiplog.
//!
//! This crate provides window implementations for organizing stream elements
//! into temporal groups for aggregation and processing.

use chrono::Utc;

/// A time window defined by start and end timestamps (in milliseconds)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    start: i64,
    end: i64,
}

impl Window {
    /// Create a new window with the given start and end timestamps (in milliseconds)
    pub fn new(start: i64, end: i64) -> Self {
        Self { start, end }
    }

    /// Create a window from a duration
    pub fn from_duration(start: i64, duration_ms: i64) -> Self {
        Self {
            start,
            end: start + duration_ms,
        }
    }

    /// Get the start timestamp
    pub fn start(&self) -> i64 {
        self.start
    }

    /// Get the end timestamp
    pub fn end(&self) -> i64 {
        self.end
    }

    /// Get the window size in milliseconds
    pub fn size(&self) -> i64 {
        self.end - self.start
    }

    /// Check if a timestamp falls within this window
    pub fn contains(&self, timestamp: i64) -> bool {
        timestamp >= self.start && timestamp < self.end
    }

    /// Check if this window overlaps with another
    pub fn overlaps(&self, other: &Window) -> bool {
        self.start < other.end && other.start < self.end
    }
}

/// A tumbling window assigns elements to non-overlapping, consecutive windows.
pub struct TumblingWindow {
    size_ms: i64,
    current_window_start: i64,
}

impl TumblingWindow {
    /// Create a new tumbling window with the given size (in milliseconds)
    pub fn new(size_ms: i64) -> Self {
        let now = Utc::now().timestamp_millis();
        Self {
            size_ms,
            current_window_start: now - (now % size_ms),
        }
    }

    /// Get the current window
    pub fn current(&self) -> Window {
        Window::new(self.current_window_start, self.current_window_start + self.size_ms)
    }

    /// Advance to the next window
    pub fn advance(&mut self) {
        self.current_window_start += self.size_ms;
    }

    /// Get the window for a given timestamp
    pub fn window_for(&self, timestamp: i64) -> Window {
        let window_start = (timestamp / self.size_ms) * self.size_ms;
        Window::new(window_start, window_start + self.size_ms)
    }
}

/// A sliding window assigns elements to overlapping windows of fixed size.
pub struct SlidingWindow {
    size_ms: i64,
    slide_ms: i64,
    latest_window_start: i64,
}

impl SlidingWindow {
    /// Create a new sliding window with the given size and slide (in milliseconds)
    pub fn new(size_ms: i64, slide_ms: i64) -> Self {
        let now = Utc::now().timestamp_millis();
        Self {
            size_ms,
            slide_ms,
            latest_window_start: now - (now % slide_ms),
        }
    }

    /// Get the current window
    pub fn current(&self) -> Window {
        Window::new(self.latest_window_start, self.latest_window_start + self.size_ms)
    }

    /// Advance to the next window
    pub fn advance(&mut self) {
        self.latest_window_start += self.slide_ms;
    }

    /// Get the window for a given timestamp
    pub fn window_for(&self, timestamp: i64) -> Window {
        let window_start = (timestamp / self.slide_ms) * self.slide_ms;
        Window::new(window_start, window_start + self.size_ms)
    }

    /// Get all windows that overlap with the given window
    pub fn overlapping_windows(&self, window: &Window) -> Vec<Window> {
        let mut windows = Vec::new();
        
        // Calculate the first window start that could overlap
        let mut window_start = window.start - (window.start % self.slide_ms);
        
        while window_start < window.end {
            let w = Window::new(window_start, window_start + self.size_ms);
            if w.overlaps(window) {
                windows.push(w);
            }
            window_start += self.slide_ms;
        }
        
        windows
    }
}

/// A session window groups elements by gaps in their timestamps.
pub struct SessionWindow {
    gap_ms: i64,
    sessions: Vec<Window>,
}

impl SessionWindow {
    /// Create a new session window with the given gap threshold (in milliseconds)
    pub fn new(gap_ms: i64) -> Self {
        Self {
            gap_ms,
            sessions: Vec::new(),
        }
    }

    /// Add a timestamp to the session window
    pub fn add(&mut self, timestamp: i64) {
        // Try to extend an existing session
        for session in &mut self.sessions {
            if timestamp - session.end <= self.gap_ms {
                session.end = timestamp;
                return;
            }
        }
        
        // Create a new session
        self.sessions.push(Window::new(timestamp, timestamp));
    }

    /// Get all sessions
    pub fn sessions(&self) -> &[Window] {
        &self.sessions
    }

    /// Clear all sessions
    pub fn clear(&mut self) {
        self.sessions.clear();
    }
}

impl Default for SessionWindow {
    fn default() -> Self {
        Self::new(1000) // 1 second default gap
    }
}

/// A window assigner that assigns elements to windows
pub trait WindowAssigner: Send + Sync {
    /// Assign an element to a window
    fn assign(&self, timestamp: i64) -> Window;
}

/// Fixed tumbling window assigner
pub struct TumblingWindowAssigner {
    size_ms: i64,
}

impl TumblingWindowAssigner {
    pub fn new(size_ms: i64) -> Self {
        Self { size_ms }
    }
}

impl WindowAssigner for TumblingWindowAssigner {
    fn assign(&self, timestamp: i64) -> Window {
        let window_start = (timestamp / self.size_ms) * self.size_ms;
        Window::new(window_start, window_start + self.size_ms)
    }
}

/// Sliding window assigner
pub struct SlidingWindowAssigner {
    size_ms: i64,
    slide_ms: i64,
}

impl SlidingWindowAssigner {
    pub fn new(size_ms: i64, slide_ms: i64) -> Self {
        Self { size_ms, slide_ms }
    }
}

impl WindowAssigner for SlidingWindowAssigner {
    fn assign(&self, timestamp: i64) -> Window {
        let window_start = (timestamp / self.slide_ms) * self.slide_ms;
        Window::new(window_start, window_start + self.size_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_creation() {
        let window = Window::new(0, 1000);
        assert_eq!(window.start(), 0);
        assert_eq!(window.end(), 1000);
        assert_eq!(window.size(), 1000);
    }

    #[test]
    fn test_window_contains() {
        let window = Window::new(0, 1000);
        assert!(window.contains(0));
        assert!(window.contains(500));
        assert!(!window.contains(1000));
        assert!(!window.contains(2000));
    }

    #[test]
    fn test_window_overlaps() {
        let w1 = Window::new(0, 500);
        let w2 = Window::new(400, 900);
        let w3 = Window::new(500, 1000);
        
        assert!(w1.overlaps(&w2));
        assert!(!w1.overlaps(&w3));
    }

    #[test]
    fn test_tumbling_window() {
        let mut tw = TumblingWindow::new(1000);
        
        let w = tw.current();
        assert!(w.size() == 1000);
        
        tw.advance();
        let w2 = tw.current();
        assert_eq!(w2.start(), w.start() + 1000);
    }

    #[test]
    fn test_tumbling_window_for_timestamp() {
        let tw = TumblingWindow::new(1000);
        
        let w = tw.window_for(1500);
        assert_eq!(w.start(), 1000);
        assert_eq!(w.end(), 2000);
    }

    #[test]
    fn test_sliding_window() {
        let mut sw = SlidingWindow::new(1000, 500);
        
        let w = sw.current();
        assert_eq!(w.size(), 1000);
        
        sw.advance();
        let w2 = sw.current();
        assert_eq!(w2.start(), w.start() + 500);
    }

    #[test]
    fn test_sliding_window_overlapping() {
        let sw = SlidingWindow::new(1000, 500);
        
        let windows = sw.overlapping_windows(&Window::new(800, 1200));
        
        // Should have windows starting at 500 and 1000
        assert!(windows.len() >= 2);
    }

    #[test]
    fn test_session_window() {
        let mut sw = SessionWindow::new(1000);
        
        sw.add(0);
        sw.add(100);
        sw.add(200);
        
        // All within 1000ms gap, should be one session
        assert_eq!(sw.sessions().len(), 1);
        
        // Add a gap > 1000ms
        sw.add(1500);
        
        // Should now have 2 sessions
        assert_eq!(sw.sessions().len(), 2);
    }

    #[test]
    fn test_session_window_default() {
        let sw: SessionWindow = Default::default();
        assert_eq!(sw.sessions().len(), 0);
    }

    #[test]
    fn test_tumbling_window_assigner() {
        let assigner = TumblingWindowAssigner::new(1000);
        
        let w = assigner.assign(1500);
        assert_eq!(w.start(), 1000);
        assert_eq!(w.end(), 2000);
    }

    #[test]
    fn test_sliding_window_assigner() {
        let assigner = SlidingWindowAssigner::new(1000, 500);
        
        let w = assigner.assign(1500);
        assert_eq!(w.start(), 1500);
        assert_eq!(w.end(), 2500);
    }
}
