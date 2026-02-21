//! Report generation from workstream data.
//!
//! This crate provides functionality for generating reports from workstream data.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Report format
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReportFormat {
    Json,
    Markdown,
    Html,
}

/// Report section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSection {
    pub title: String,
    pub content: String,
    pub order: usize,
}

/// Report metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportMetadata {
    pub title: String,
    pub author: Option<String>,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub generated_at: DateTime<Utc>,
}

/// Complete report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub metadata: ReportMetadata,
    pub sections: Vec<ReportSection>,
    pub format: ReportFormat,
}

impl Report {
    pub fn new(
        title: String,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
        format: ReportFormat,
    ) -> Self {
        Self {
            metadata: ReportMetadata {
                title,
                author: None,
                period_start,
                period_end,
                generated_at: Utc::now(),
            },
            sections: Vec::new(),
            format,
        }
    }

    /// Add a section to the report
    pub fn add_section(&mut self, title: String, content: String, order: usize) {
        self.sections.push(ReportSection {
            title,
            content,
            order,
        });
    }

    /// Sort sections by order
    pub fn sort_sections(&mut self) {
        self.sections.sort_by_key(|s| s.order);
    }
}

/// Workstream summary
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkstreamSummary {
    pub name: String,
    pub event_count: usize,
    pub coverage: f64,
}

/// Report generator
pub struct ReportGenerator {
    workstreams: HashMap<String, WorkstreamSummary>,
}

impl ReportGenerator {
    pub fn new() -> Self {
        Self {
            workstreams: HashMap::new(),
        }
    }

    /// Add workstream data
    pub fn add_workstream(&mut self, name: &str, event_count: usize, coverage: f64) {
        self.workstreams.insert(
            name.to_string(),
            WorkstreamSummary {
                name: name.to_string(),
                event_count,
                coverage,
            },
        );
    }

    /// Generate a report
    pub fn generate(
        &self,
        title: String,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
        format: ReportFormat,
    ) -> Report {
        let mut report = Report::new(title, period_start, period_end, format);

        // Add summary section
        let total_events: usize = self.workstreams.values().map(|w| w.event_count).sum();
        let avg_coverage: f64 = if !self.workstreams.is_empty() {
            self.workstreams.values().map(|w| w.coverage).sum::<f64>()
                / self.workstreams.len() as f64
        } else {
            0.0
        };

        let summary_content = format!(
            "Total workstreams: {}\nTotal events: {}\nAverage coverage: {:.1}%",
            self.workstreams.len(),
            total_events,
            avg_coverage
        );

        report.add_section("Summary".to_string(), summary_content, 0);

        // Add workstream details
        let mut order = 1;
        for ws in self.workstreams.values() {
            let content = format!("Events: {}\nCoverage: {:.1}%", ws.event_count, ws.coverage);
            report.add_section(ws.name.clone(), content, order);
            order += 1;
        }

        report.sort_sections();
        report
    }
}

impl Default for ReportGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_creation() {
        let report = Report::new(
            "Test Report".to_string(),
            Utc::now(),
            Utc::now(),
            ReportFormat::Json,
        );

        assert_eq!(report.metadata.title, "Test Report");
        assert!(report.sections.is_empty());
    }

    #[test]
    fn test_report_add_section() {
        let mut report = Report::new(
            "Test Report".to_string(),
            Utc::now(),
            Utc::now(),
            ReportFormat::Markdown,
        );

        report.add_section(
            "Introduction".to_string(),
            "This is the intro".to_string(),
            1,
        );
        report.add_section("Summary".to_string(), "Summary here".to_string(), 0);

        assert_eq!(report.sections.len(), 2);
    }

    #[test]
    fn test_report_sort_sections() {
        let mut report = Report::new(
            "Test Report".to_string(),
            Utc::now(),
            Utc::now(),
            ReportFormat::Json,
        );

        report.add_section("Third".to_string(), "third".to_string(), 2);
        report.add_section("First".to_string(), "first".to_string(), 0);
        report.add_section("Second".to_string(), "second".to_string(), 1);

        report.sort_sections();

        assert_eq!(report.sections[0].title, "First");
        assert_eq!(report.sections[1].title, "Second");
        assert_eq!(report.sections[2].title, "Third");
    }

    #[test]
    fn test_report_generator() {
        let mut generator = ReportGenerator::new();

        generator.add_workstream("backend", 50, 85.0);
        generator.add_workstream("frontend", 30, 90.0);

        let report = generator.generate(
            "Weekly Report".to_string(),
            Utc::now(),
            Utc::now(),
            ReportFormat::Html,
        );

        assert_eq!(report.metadata.title, "Weekly Report");
        assert_eq!(report.sections.len(), 3); // Summary + 2 workstreams
    }

    #[test]
    fn test_workstream_summary() {
        let ws = WorkstreamSummary {
            name: "test".to_string(),
            event_count: 100,
            coverage: 75.5,
        };

        assert_eq!(ws.name, "test");
        assert_eq!(ws.event_count, 100);
        assert_eq!(ws.coverage, 75.5);
    }
}
