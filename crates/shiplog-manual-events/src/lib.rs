//! Manual events file and mapping utilities.
//!
//! This microcrate is responsible for translating manual event files into canonical
//! manual events and applying the half-open date-window filter used by the
//! `shiplog-ingest-manual` adapter.

use anyhow::{Context, Result};
use chrono::{NaiveDate, Utc};
use shiplog_ids::EventId;
use shiplog_schema::coverage::TimeWindow;
use shiplog_schema::event::{
    Actor, EventEnvelope, EventKind, EventPayload, ManualDate, ManualEvent, ManualEventEntry,
    ManualEventsFile, RepoRef, RepoVisibility, SourceRef, SourceSystem,
};
use std::path::Path;

/// Read a manual events file from disk.
pub fn read_manual_events(path: &Path) -> Result<ManualEventsFile> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("read manual events from {path:?}"))?;
    let file: ManualEventsFile = serde_yaml::from_str(&text)
        .with_context(|| format!("parse manual events yaml {path:?}"))?;
    Ok(file)
}

/// Write a manual events file to disk.
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

/// Build a simple manual entry.
pub fn create_entry(
    id: impl Into<String>,
    event_type: shiplog_schema::event::ManualEventType,
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

/// Returns the inclusive date range represented by a manual entry.
pub fn entry_date_range(entry: &ManualEventEntry) -> (NaiveDate, NaiveDate) {
    match &entry.date {
        ManualDate::Single(d) => (*d, *d),
        ManualDate::Range { start, end } => (*start, *end),
    }
}

/// Convert a single entry to a canonical envelope.
pub fn entry_to_event(entry: &ManualEventEntry, user: &str) -> EventEnvelope {
    let (start_date, end_date) = entry_date_range(entry);
    let occurred_at = end_date
        .and_hms_opt(12, 0, 0)
        .expect("NaiveDate -> NaiveDateTime conversion should be valid")
        .and_utc();

    let id = EventId::from_parts(["manual", &entry.id]);
    let manual_event = ManualEvent {
        event_type: entry.event_type.clone(),
        title: entry.title.clone(),
        description: entry.description.clone(),
        started_at: Some(start_date),
        ended_at: Some(end_date),
        impact: entry.impact.clone(),
    };

    EventEnvelope {
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
    }
}

/// Filter entries by `window` and generate warnings on partial overlaps.
///
/// * Includes only entries that intersect the half-open window.
/// * Emits a warning for partial overlaps on the boundary.
pub fn events_in_window(
    entries: &[ManualEventEntry],
    user: &str,
    window: &TimeWindow,
) -> (Vec<EventEnvelope>, Vec<String>) {
    let mut events = Vec::new();
    let mut warnings = Vec::new();

    for entry in entries {
        let (start_date, end_date) = entry_date_range(entry);

        if end_date < window.since || start_date >= window.until {
            continue;
        }

        if start_date < window.since || end_date >= window.until {
            warnings.push(format!(
                "Event '{}' partially outside date window",
                entry.id
            ));
        }

        events.push(entry_to_event(entry, user));
    }

    (events, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use proptest::prelude::*;
    use shiplog_schema::event::{Link, ManualEventType};

    fn make_entry(id: &str, date: ManualDate) -> ManualEventEntry {
        create_entry(id, ManualEventType::Note, date, format!("Event {id}"))
    }

    #[test]
    fn reads_and_writes_manual_events() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("manual_events.yaml");

        let file = ManualEventsFile {
            version: 1,
            generated_at: Utc::now(),
            events: vec![make_entry(
                "test-1",
                ManualDate::Single(NaiveDate::from_ymd_opt(2025, 3, 15).unwrap()),
            )],
        };

        write_manual_events(&path, &file).unwrap();
        let read = read_manual_events(&path).unwrap();

        assert_eq!(read.events.len(), 1);
        assert_eq!(read.events[0].id, "test-1");
    }

    #[test]
    fn events_in_window_keeps_single_inside() {
        let window = TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        };

        let entries = vec![
            make_entry(
                "inside",
                ManualDate::Single(NaiveDate::from_ymd_opt(2025, 1, 15).unwrap()),
            ),
            make_entry(
                "outside",
                ManualDate::Single(NaiveDate::from_ymd_opt(2025, 2, 15).unwrap()),
            ),
        ];

        let (events, warnings) = events_in_window(&entries, "user", &window);
        assert_eq!(events.len(), 1);
        let expected_id = EventId::from_parts(["manual", "inside"]);
        assert_eq!(events[0].id, expected_id);
        assert!(warnings.is_empty());
    }

    #[test]
    fn entry_to_event_sets_manual_defaults() {
        let mut entry = create_entry(
            "event-1",
            ManualEventType::Incident,
            ManualDate::Range {
                start: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
                end: NaiveDate::from_ymd_opt(2025, 2, 5).unwrap(),
            },
            "Incident",
        );
        entry.receipts = vec![Link {
            label: "summary".to_string(),
            url: "https://example.com/incident".to_string(),
        }];

        let ev = entry_to_event(&entry, "alice");

        assert_eq!(ev.kind, EventKind::Manual);
        assert_eq!(ev.actor.login, "alice");
        assert_eq!(ev.repo.full_name, "manual/general");
        assert!(!ev.links.is_empty());
    }

    fn arb_date() -> impl Strategy<Value = NaiveDate> {
        (-20_000i32..20_000)
            .prop_map(|offset| NaiveDate::from_num_days_from_ce_opt(offset).unwrap())
    }

    fn arb_manual_date() -> impl Strategy<Value = ManualDate> {
        prop_oneof![
            arb_date().prop_map(ManualDate::Single),
            (arb_date(), arb_date()).prop_map(|(a, b)| {
                let (start, end) = if a <= b { (a, b) } else { (b, a) };
                ManualDate::Range { start, end }
            }),
        ]
    }

    proptest! {
        #[test]
        fn events_in_window_matches_bounds(entry in arb_manual_date(),
                                          since in arb_date(),
                                          until in arb_date()) {
            let (window_since, window_until) = if since <= until {
                (since, until)
            } else {
                (until, since)
            };
            let window = TimeWindow { since: window_since, until: window_until };
            let (start, end) = entry_date_range(&make_entry("p", entry.clone()));
            let (events, warnings) = events_in_window(&[make_entry("p", entry)], "x", &window);
            let included = !(end < window_since || start >= window_until);
            let partial = included && (start < window_since || end >= window_until);

            if included {
                prop_assert_eq!(events.len(), 1);
                if partial {
                    prop_assert_eq!(warnings.len(), 1);
                    prop_assert!(warnings[0].contains("partially outside date window"));
                } else {
                    prop_assert!(warnings.is_empty());
                }
            } else {
                prop_assert!(events.is_empty());
                prop_assert!(warnings.is_empty());
            }
        }
    }
}
