use anyhow::{Context, Result};
use chrono::Utc;
use shiplog_ids::EventId;
use shiplog_ports::{IngestOutput, Ingestor};
use shiplog_schema::coverage::{Completeness, CoverageManifest, CoverageSlice, TimeWindow};
use shiplog_schema::event::{
    Actor, EventEnvelope, EventKind, EventPayload, ManualDate, ManualEvent, ManualEventEntry,
    ManualEventType, ManualEventsFile, RepoRef, RepoVisibility, SourceRef, SourceSystem,
};
use std::path::Path;

/// Ingestor for manual events from YAML files.
///
/// This allows users to include non-GitHub work in their packets:
/// - Incidents handled
/// - Design docs written
/// - Mentoring
/// - Migrations planned
/// - etc.
pub struct ManualIngestor {
    pub events_path: std::path::PathBuf,
    pub user: String,
    pub window: TimeWindow,
}

impl ManualIngestor {
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
        let mut events = Vec::new();
        let mut warnings = Vec::new();

        for entry in &file.events {
            // Filter events by date range
            let (start_date, end_date) = match &entry.date {
                ManualDate::Single(d) => (*d, *d),
                ManualDate::Range { start, end } => (*start, *end),
            };

            // Skip if entirely outside window
            if end_date < self.window.since || start_date >= self.window.until {
                continue;
            }

            // Warn if partially outside window
            if start_date < self.window.since || end_date >= self.window.until {
                warnings.push(format!(
                    "Event '{}' partially outside date window",
                    entry.id
                ));
            }

            let ev = entry_to_event(entry, &self.user)?;
            events.push(ev);
        }

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
pub fn read_manual_events(path: &Path) -> Result<ManualEventsFile> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("read manual events from {path:?}"))?;
    let file: ManualEventsFile = serde_yaml::from_str(&text)
        .with_context(|| format!("parse manual events yaml {path:?}"))?;
    Ok(file)
}

/// Write manual events to a YAML file.
pub fn write_manual_events(path: &Path, file: &ManualEventsFile) -> Result<()> {
    let yaml = serde_yaml::to_string(file)?;
    std::fs::write(path, yaml).with_context(|| format!("write manual events to {path:?}"))?;
    Ok(())
}

/// Create a new empty manual events file.
pub fn create_empty_file() -> ManualEventsFile {
    ManualEventsFile {
        version: 1,
        generated_at: Utc::now(),
        events: Vec::new(),
    }
}

/// Convert a ManualEventEntry to an EventEnvelope.
fn entry_to_event(entry: &ManualEventEntry, user: &str) -> Result<EventEnvelope> {
    let (start_date, end_date) = match &entry.date {
        ManualDate::Single(d) => (*d, *d),
        ManualDate::Range { start, end } => (*start, *end),
    };

    // Use end date as occurred_at (noon UTC)
    let occurred_at = end_date
        .and_hms_opt(12, 0, 0)
        .unwrap()
        .and_local_timezone(Utc)
        .unwrap();

    let id = EventId::from_parts(["manual", &entry.id]);

    let manual_event = ManualEvent {
        event_type: entry.event_type.clone(),
        title: entry.title.clone(),
        description: entry.description.clone(),
        started_at: Some(start_date),
        ended_at: Some(end_date),
        impact: entry.impact.clone(),
    };

    Ok(EventEnvelope {
        id,
        kind: EventKind::Manual,
        occurred_at,
        actor: Actor {
            login: user.to_string(),
            id: None,
        },
        repo: RepoRef {
            full_name: entry
                .workstream
                .clone()
                .unwrap_or_else(|| "manual/general".to_string()),
            html_url: None,
            visibility: RepoVisibility::Unknown,
        },
        payload: EventPayload::Manual(manual_event),
        tags: entry.tags.clone(),
        links: entry.receipts.clone(),
        source: SourceRef {
            system: SourceSystem::Manual,
            url: None,
            opaque_id: Some(entry.id.clone()),
        },
    })
}

/// Helper to create a simple manual event entry.
pub fn create_entry(
    id: impl Into<String>,
    event_type: ManualEventType,
    date: ManualDate,
    title: impl Into<String>,
) -> ManualEventEntry {
    ManualEventEntry {
        id: id.into(),
        event_type,
        date,
        title: title.into(),
        description: None,
        workstream: None,
        tags: Vec::new(),
        receipts: Vec::new(),
        impact: None,
    }
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
}
