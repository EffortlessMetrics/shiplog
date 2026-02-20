//! Statistics and analytics on events/workstreams.
//!
//! This crate provides functionality for computing statistics on shipping packets
//! and workstream data.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Event statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventStats {
    pub total_count: usize,
    pub by_type: HashMap<String, usize>,
    pub by_source: HashMap<String, usize>,
}

/// Workstream statistics  
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkstreamStats {
    pub total_workstreams: usize,
    pub total_events: usize,
    pub events_per_workstream: HashMap<String, usize>,
    pub coverage_percentage: f64,
}

/// Statistics analyzer
pub struct StatsAnalyzer {
    event_stats: EventStats,
    workstream_stats: WorkstreamStats,
}

impl StatsAnalyzer {
    pub fn new() -> Self {
        Self {
            event_stats: EventStats::default(),
            workstream_stats: WorkstreamStats::default(),
        }
    }

    /// Record an event
    pub fn record_event(&mut self, event_type: &str, source: &str) {
        self.event_stats.total_count += 1;
        *self.event_stats.by_type.entry(event_type.to_string()).or_insert(0) += 1;
        *self.event_stats.by_source.entry(source.to_string()).or_insert(0) += 1;
    }

    /// Record a workstream with event count
    pub fn record_workstream(&mut self, name: &str, event_count: usize) {
        self.workstream_stats.total_workstreams += 1;
        self.workstream_stats.total_events += event_count;
        self.workstream_stats.events_per_workstream.insert(name.to_string(), event_count);
    }

    /// Calculate coverage percentage
    pub fn calculate_coverage(&mut self, covered: usize, total: usize) {
        if total > 0 {
            self.workstream_stats.coverage_percentage = (covered as f64 / total as f64) * 100.0;
        }
    }

    /// Get event statistics
    pub fn event_stats(&self) -> &EventStats {
        &self.event_stats
    }

    /// Get workstream statistics
    pub fn workstream_stats(&self) -> &WorkstreamStats {
        &self.workstream_stats
    }
}

impl Default for StatsAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsSummary {
    pub generated_at: DateTime<Utc>,
    pub events: EventStats,
    pub workstreams: WorkstreamStats,
}

impl StatsAnalyzer {
    /// Generate a summary report
    pub fn generate_summary(&self) -> StatsSummary {
        StatsSummary {
            generated_at: Utc::now(),
            events: self.event_stats.clone(),
            workstreams: self.workstream_stats.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_stats_recording() {
        let mut analyzer = StatsAnalyzer::new();
        
        analyzer.record_event("commit", "github");
        analyzer.record_event("commit", "github");
        analyzer.record_event("pr", "github");
        
        assert_eq!(analyzer.event_stats().total_count, 3);
        assert_eq!(analyzer.event_stats().by_type.get("commit"), Some(&2));
        assert_eq!(analyzer.event_stats().by_type.get("pr"), Some(&1));
    }

    #[test]
    fn test_workstream_stats() {
        let mut analyzer = StatsAnalyzer::new();
        
        analyzer.record_workstream("backend", 10);
        analyzer.record_workstream("frontend", 5);
        
        assert_eq!(analyzer.workstream_stats().total_workstreams, 2);
        assert_eq!(analyzer.workstream_stats().total_events, 15);
    }

    #[test]
    fn test_coverage_calculation() {
        let mut analyzer = StatsAnalyzer::new();
        
        analyzer.calculate_coverage(80, 100);
        assert_eq!(analyzer.workstream_stats().coverage_percentage, 80.0);
        
        analyzer.calculate_coverage(3, 4);
        assert_eq!(analyzer.workstream_stats().coverage_percentage, 75.0);
    }

    #[test]
    fn test_stats_summary() {
        let analyzer = StatsAnalyzer::new();
        
        let summary = analyzer.generate_summary();
        assert!(summary.generated_at <= Utc::now());
    }
}
