//! Manual YAML ingestor for non-GitHub work.
//!
//! Reads `manual_events.yaml`, filters entries by date window, converts entries
//! into canonical events, and emits coverage metadata for the import.

use anyhow::Result;
use chrono::Utc;
use shiplog_ports::{IngestOutput, Ingestor};
use shiplog_schema::coverage::{Completeness, CoverageManifest, CoverageSlice, TimeWindow};
use shiplog_schema::event::{ManualDate, ManualEventEntry, ManualEventType, ManualEventsFile};
use std::path::Path;

/// Ingestor for manual events from YAML files.
///
/// This allows users to include non-GitHub work in their packets:
/// - Incidents handled
/// - Design docs written
/// - Mentoring
/// - Migrations planned
/// - etc.
///
/// # Examples
///
/// ```rust,no_run
/// use shiplog_ingest_manual::ManualIngestor;
/// use shiplog_ports::Ingestor;
/// use chrono::NaiveDate;
///
/// let ingestor = ManualIngestor::new(
///     "manual_events.yaml",
///     "octocat".to_string(),
///     NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
///     NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
/// );
/// let output = ingestor.ingest()?;
/// println!("Found {} manual events", output.events.len());
/// # Ok::<(), anyhow::Error>(())
/// ```
pub struct ManualIngestor {
    pub events_path: std::path::PathBuf,
    pub user: String,
    pub window: TimeWindow,
}

impl ManualIngestor {
    /// Create a new manual ingestor for the given YAML file and date window.
    ///
    /// # Examples
    ///
    /// ```
    /// use shiplog_ingest_manual::ManualIngestor;
    /// use chrono::NaiveDate;
    ///
    /// let ingestor = ManualIngestor::new(
    ///     "manual_events.yaml",
    ///     "octocat".to_string(),
    ///     NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
    ///     NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
    /// );
    /// ```
    pub fn new(
        events_path: impl AsRef<Path>,
        user: String,
        since: chrono::NaiveDate,
        until: chrono::NaiveDate,
    ) -> Self {
        Self {
            events_path: events_path.as_ref().to_path_buf(),
            user,
            window: TimeWindow { since, until },
        }
    }
}

impl Ingestor for ManualIngestor {
    fn ingest(&self) -> Result<IngestOutput> {
        if !self.events_path.exists() {
            // Return empty output if file doesn't exist
            return Ok(IngestOutput {
                events: Vec::new(),
                coverage: CoverageManifest {
                    run_id: shiplog_ids::RunId::now("manual"),
                    generated_at: Utc::now(),
                    user: self.user.clone(),
                    window: self.window.clone(),
                    mode: "manual".to_string(),
                    sources: vec!["manual".to_string()],
                    slices: vec![CoverageSlice {
                        window: self.window.clone(),
                        query: format!("file:{:?}", self.events_path),
                        total_count: 0,
                        fetched: 0,
                        incomplete_results: Some(false),
                        notes: vec!["manual_events_file_not_found".to_string()],
                    }],
                    warnings: vec![format!(
                        "Manual events file not found: {:?}",
                        self.events_path
                    )],
                    completeness: Completeness::Unknown,
                },
            });
        }

        let file = read_manual_events(&self.events_path)?;
        let (events, warnings) =
            shiplog_manual_events::events_in_window(&file.events, &self.user, &self.window);

        let coverage = CoverageManifest {
            run_id: shiplog_ids::RunId::now("manual"),
            generated_at: Utc::now(),
            user: self.user.clone(),
            window: self.window.clone(),
            mode: "manual".to_string(),
            sources: vec!["manual".to_string()],
            slices: vec![CoverageSlice {
                window: self.window.clone(),
                query: format!("file:{:?}", self.events_path),
                total_count: file.events.len() as u64,
                fetched: events.len() as u64,
                incomplete_results: Some(false),
                notes: vec!["manual_events".to_string()],
            }],
            warnings,
            completeness: Completeness::Complete,
        };

        Ok(IngestOutput { events, coverage })
    }
}

/// Read manual events from a YAML file.
///
/// # Examples
///
/// ```rust,no_run
/// use shiplog_ingest_manual::read_manual_events;
/// use std::path::Path;
///
/// let file = read_manual_events(Path::new("manual_events.yaml"))?;
/// println!("Found {} events", file.events.len());
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn read_manual_events(path: &Path) -> Result<ManualEventsFile> {
    shiplog_manual_events::read_manual_events(path)
}

/// Write manual events to a YAML file.
///
/// # Examples
///
/// ```rust,no_run
/// use shiplog_ingest_manual::{create_empty_file, write_manual_events};
/// use std::path::Path;
///
/// let file = create_empty_file();
/// write_manual_events(Path::new("manual_events.yaml"), &file)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn write_manual_events(path: &Path, file: &ManualEventsFile) -> Result<()> {
    shiplog_manual_events::write_manual_events(path, file)
}

/// Create a new empty manual events file.
///
/// # Examples
///
/// ```
/// use shiplog_ingest_manual::create_empty_file;
///
/// let file = create_empty_file();
/// assert_eq!(file.version, 1);
/// assert!(file.events.is_empty());
/// ```
pub fn create_empty_file() -> ManualEventsFile {
    shiplog_manual_events::create_empty_file()
}

/// Helper to create a simple manual event entry.
///
/// # Examples
///
/// ```
/// use shiplog_ingest_manual::create_entry;
/// use shiplog_schema::event::{ManualEventType, ManualDate};
/// use chrono::NaiveDate;
///
/// let entry = create_entry(
///     "design-review-1",
///     ManualEventType::Design,
///     ManualDate::Single(NaiveDate::from_ymd_opt(2025, 3, 15).unwrap()),
///     "API design review",
/// );
/// assert_eq!(entry.id, "design-review-1");
/// assert_eq!(entry.title, "API design review");
/// ```
pub fn create_entry(
    id: impl Into<String>,
    event_type: ManualEventType,
    date: ManualDate,
    title: impl Into<String>,
) -> ManualEventEntry {
    shiplog_manual_events::create_entry(id, event_type, date, title)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn make_test_entry(id: &str) -> ManualEventEntry {
        ManualEventEntry {
            id: id.to_string(),
            event_type: ManualEventType::Note,
            date: ManualDate::Single(NaiveDate::from_ymd_opt(2025, 3, 15).unwrap()),
            title: "Test Event".to_string(),
            description: Some("A test event".to_string()),
            workstream: Some("test-workstream".to_string()),
            tags: vec!["test".to_string()],
            receipts: vec![shiplog_schema::event::Link {
                label: "doc".to_string(),
                url: "https://example.com/doc".to_string(),
            }],
            impact: Some("Made things better".to_string()),
        }
    }

    #[test]
    fn reads_and_writes_manual_events() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("manual_events.yaml");

        let file = ManualEventsFile {
            version: 1,
            generated_at: Utc::now(),
            events: vec![make_test_entry("test-1")],
        };

        write_manual_events(&path, &file).unwrap();
        let read = read_manual_events(&path).unwrap();

        assert_eq!(read.events.len(), 1);
        assert_eq!(read.events[0].id, "test-1");
    }

    #[test]
    fn ingest_filters_by_date() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("manual_events.yaml");

        // Create file with events inside and outside window
        let file = ManualEventsFile {
            version: 1,
            generated_at: Utc::now(),
            events: vec![
                ManualEventEntry {
                    id: "inside".to_string(),
                    event_type: ManualEventType::Note,
                    date: ManualDate::Single(NaiveDate::from_ymd_opt(2025, 3, 15).unwrap()),
                    title: "Inside".to_string(),
                    description: None,
                    workstream: None,
                    tags: vec![],
                    receipts: vec![],
                    impact: None,
                },
                ManualEventEntry {
                    id: "outside".to_string(),
                    event_type: ManualEventType::Note,
                    date: ManualDate::Single(NaiveDate::from_ymd_opt(2025, 6, 15).unwrap()),
                    title: "Outside".to_string(),
                    description: None,
                    workstream: None,
                    tags: vec![],
                    receipts: vec![],
                    impact: None,
                },
            ],
        };

        write_manual_events(&path, &file).unwrap();

        let ing = ManualIngestor::new(
            &path,
            "testuser".to_string(),
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
        );

        let output = ing.ingest().unwrap();
        assert_eq!(output.events.len(), 1);
        assert_eq!(
            output.events[0].source.opaque_id,
            Some("inside".to_string())
        );
    }

    #[test]
    fn event_with_end_date_equal_to_window_since_is_included() {
        // end_date == window.since → NOT skipped (since `end_date < since` is false)
        // This kills the `<` → `<=` mutation at line 88.
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("manual_events.yaml");
        let file = ManualEventsFile {
            version: 1,
            generated_at: Utc::now(),
            events: vec![ManualEventEntry {
                id: "boundary".to_string(),
                event_type: ManualEventType::Note,
                date: ManualDate::Single(NaiveDate::from_ymd_opt(2025, 3, 1).unwrap()),
                title: "Boundary Event".to_string(),
                description: None,
                workstream: None,
                tags: vec![],
                receipts: vec![],
                impact: None,
            }],
        };
        write_manual_events(&path, &file).unwrap();

        // Window since = 2025-03-01 (same as event date), until = 2025-04-01
        let ing = ManualIngestor::new(
            &path,
            "testuser".to_string(),
            NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
        );
        let output = ing.ingest().unwrap();
        // Event's end_date (2025-03-01) == window.since (2025-03-01)
        // Condition: end_date < since → false, so it is NOT excluded.
        assert_eq!(output.events.len(), 1);
    }

    #[test]
    fn event_ending_before_window_is_excluded() {
        // end_date < window.since → entirely before window, excluded
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("manual_events.yaml");
        let file = ManualEventsFile {
            version: 1,
            generated_at: Utc::now(),
            events: vec![ManualEventEntry {
                id: "before".to_string(),
                event_type: ManualEventType::Note,
                date: ManualDate::Single(NaiveDate::from_ymd_opt(2025, 2, 28).unwrap()),
                title: "Before Window".to_string(),
                description: None,
                workstream: None,
                tags: vec![],
                receipts: vec![],
                impact: None,
            }],
        };
        write_manual_events(&path, &file).unwrap();

        let ing = ManualIngestor::new(
            &path,
            "testuser".to_string(),
            NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
        );
        let output = ing.ingest().unwrap();
        assert_eq!(output.events.len(), 0);
    }

    #[test]
    fn event_starting_at_window_until_is_excluded() {
        // start_date >= window.until → entirely after window, excluded
        // Kills `>=` → `>` mutation at line 88
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("manual_events.yaml");
        let file = ManualEventsFile {
            version: 1,
            generated_at: Utc::now(),
            events: vec![ManualEventEntry {
                id: "at-until".to_string(),
                event_type: ManualEventType::Note,
                date: ManualDate::Single(NaiveDate::from_ymd_opt(2025, 4, 1).unwrap()),
                title: "At Until Boundary".to_string(),
                description: None,
                workstream: None,
                tags: vec![],
                receipts: vec![],
                impact: None,
            }],
        };
        write_manual_events(&path, &file).unwrap();

        let ing = ManualIngestor::new(
            &path,
            "testuser".to_string(),
            NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
        );
        let output = ing.ingest().unwrap();
        assert_eq!(output.events.len(), 0);
    }

    #[test]
    fn event_spanning_before_window_start_triggers_warning() {
        // Range starts before window, ends inside → included but warns "partially outside"
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("manual_events.yaml");
        let file = ManualEventsFile {
            version: 1,
            generated_at: Utc::now(),
            events: vec![ManualEventEntry {
                id: "span-before".to_string(),
                event_type: ManualEventType::Note,
                date: ManualDate::Range {
                    start: NaiveDate::from_ymd_opt(2025, 2, 15).unwrap(),
                    end: NaiveDate::from_ymd_opt(2025, 3, 15).unwrap(),
                },
                title: "Spans Before".to_string(),
                description: None,
                workstream: None,
                tags: vec![],
                receipts: vec![],
                impact: None,
            }],
        };
        write_manual_events(&path, &file).unwrap();

        let ing = ManualIngestor::new(
            &path,
            "testuser".to_string(),
            NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
        );
        let output = ing.ingest().unwrap();
        assert_eq!(output.events.len(), 1);
        assert!(
            output
                .coverage
                .warnings
                .iter()
                .any(|w| w.contains("partially outside"))
        );
    }

    #[test]
    fn event_spanning_after_window_end_triggers_warning() {
        // Range starts inside window, ends at or after until → included but warns "partially outside"
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("manual_events.yaml");
        let file = ManualEventsFile {
            version: 1,
            generated_at: Utc::now(),
            events: vec![ManualEventEntry {
                id: "span-after".to_string(),
                event_type: ManualEventType::Note,
                date: ManualDate::Range {
                    start: NaiveDate::from_ymd_opt(2025, 3, 15).unwrap(),
                    end: NaiveDate::from_ymd_opt(2025, 4, 15).unwrap(),
                },
                title: "Spans After".to_string(),
                description: None,
                workstream: None,
                tags: vec![],
                receipts: vec![],
                impact: None,
            }],
        };
        write_manual_events(&path, &file).unwrap();

        let ing = ManualIngestor::new(
            &path,
            "testuser".to_string(),
            NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
        );
        let output = ing.ingest().unwrap();
        assert_eq!(output.events.len(), 1);
        assert!(
            output
                .coverage
                .warnings
                .iter()
                .any(|w| w.contains("partially outside"))
        );
    }

    #[test]
    fn event_entirely_outside_window_excluded() {
        // Both start and end well outside the window → excluded, 0 events
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("manual_events.yaml");
        let file = ManualEventsFile {
            version: 1,
            generated_at: Utc::now(),
            events: vec![ManualEventEntry {
                id: "outside".to_string(),
                event_type: ManualEventType::Note,
                date: ManualDate::Range {
                    start: NaiveDate::from_ymd_opt(2025, 5, 1).unwrap(),
                    end: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
                },
                title: "Entirely Outside".to_string(),
                description: None,
                workstream: None,
                tags: vec![],
                receipts: vec![],
                impact: None,
            }],
        };
        write_manual_events(&path, &file).unwrap();

        let ing = ManualIngestor::new(
            &path,
            "testuser".to_string(),
            NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
        );
        let output = ing.ingest().unwrap();
        assert_eq!(output.events.len(), 0);
        assert!(output.coverage.warnings.is_empty());
    }

    #[test]
    fn handles_missing_file() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("nonexistent.yaml");

        let ing = ManualIngestor::new(
            &path,
            "testuser".to_string(),
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
        );

        let output = ing.ingest().unwrap();
        assert!(output.events.is_empty());
        assert!(!output.coverage.warnings.is_empty());
    }

    #[test]
    fn event_ending_one_day_before_window_since_is_excluded() {
        // end_date == window.since - 1 → end_date < since → excluded.
        // Combined with event_with_end_date_equal_to_window_since_is_included,
        // this kills the < → <= mutation on the boundary check.
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("manual_events.yaml");
        let file = ManualEventsFile {
            version: 1,
            generated_at: Utc::now(),
            events: vec![ManualEventEntry {
                id: "day-before".to_string(),
                event_type: ManualEventType::Note,
                date: ManualDate::Single(NaiveDate::from_ymd_opt(2025, 2, 28).unwrap()),
                title: "Day Before Window".to_string(),
                description: None,
                workstream: None,
                tags: vec![],
                receipts: vec![],
                impact: None,
            }],
        };
        write_manual_events(&path, &file).unwrap();

        // Window since = 2025-03-01; event ends 2025-02-28 (one day before)
        let ing = ManualIngestor::new(
            &path,
            "testuser".to_string(),
            NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
        );
        let output = ing.ingest().unwrap();
        assert_eq!(
            output.events.len(),
            0,
            "event ending before window.since must be excluded"
        );

        // Now verify the boundary: end_date == window.since IS included
        let file2 = ManualEventsFile {
            version: 1,
            generated_at: Utc::now(),
            events: vec![ManualEventEntry {
                id: "at-since".to_string(),
                event_type: ManualEventType::Note,
                date: ManualDate::Single(NaiveDate::from_ymd_opt(2025, 3, 1).unwrap()),
                title: "At Window Since".to_string(),
                description: None,
                workstream: None,
                tags: vec![],
                receipts: vec![],
                impact: None,
            }],
        };
        write_manual_events(&path, &file2).unwrap();

        let ing2 = ManualIngestor::new(
            &path,
            "testuser".to_string(),
            NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
        );
        let output2 = ing2.ingest().unwrap();
        assert_eq!(
            output2.events.len(),
            1,
            "event on window.since boundary must be included"
        );
    }
}
