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

    // --- YAML parsing edge cases ---

    #[test]
    fn read_nonexistent_file_returns_error() {
        let result = read_manual_events(Path::new("/nonexistent/path/events.yaml"));
        assert!(result.is_err());
    }

    #[test]
    fn read_invalid_yaml_returns_error() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("bad.yaml");
        std::fs::write(&path, "not: [valid: yaml: {{{").unwrap();
        let result = read_manual_events(&path);
        assert!(result.is_err());
    }

    #[test]
    fn read_empty_events_list() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("empty_events.yaml");
        let file = ManualEventsFile {
            version: 1,
            generated_at: Utc::now(),
            events: vec![],
        };
        write_manual_events(&path, &file).unwrap();
        let read = read_manual_events(&path).unwrap();
        assert!(read.events.is_empty());
        assert_eq!(read.version, 1);
    }

    // --- create_empty_file ---

    #[test]
    fn create_empty_file_has_correct_defaults() {
        let file = create_empty_file();
        assert_eq!(file.version, 1);
        assert!(file.events.is_empty());
    }

    // --- entry_date_range ---

    #[test]
    fn entry_date_range_single_returns_same_start_end() {
        let d = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        let entry = make_entry("x", ManualDate::Single(d));
        let (start, end) = entry_date_range(&entry);
        assert_eq!(start, d);
        assert_eq!(end, d);
    }

    #[test]
    fn entry_date_range_range_returns_start_end() {
        let s = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let e = NaiveDate::from_ymd_opt(2025, 1, 31).unwrap();
        let entry = make_entry("x", ManualDate::Range { start: s, end: e });
        let (start, end) = entry_date_range(&entry);
        assert_eq!(start, s);
        assert_eq!(end, e);
    }

    // --- All event types ---

    #[test]
    fn all_manual_event_types_create_valid_entries() {
        let types = [
            ManualEventType::Note,
            ManualEventType::Incident,
            ManualEventType::Design,
            ManualEventType::Mentoring,
            ManualEventType::Launch,
            ManualEventType::Migration,
            ManualEventType::Review,
            ManualEventType::Other,
        ];
        let d = ManualDate::Single(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());
        for et in types {
            let entry = create_entry("t", et.clone(), d.clone(), "Title");
            let ev = entry_to_event(&entry, "user");
            assert_eq!(ev.kind, EventKind::Manual);
            if let EventPayload::Manual(m) = &ev.payload {
                assert_eq!(m.event_type, et);
            } else {
                panic!("expected Manual payload");
            }
        }
    }

    // --- entry_to_event edge cases ---

    #[test]
    fn entry_to_event_with_workstream_sets_repo_name() {
        let mut entry = make_entry(
            "ws-test",
            ManualDate::Single(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
        );
        entry.workstream = Some("infra/migrations".to_string());
        let ev = entry_to_event(&entry, "user");
        assert_eq!(ev.repo.full_name, "infra/migrations");
    }

    #[test]
    fn entry_to_event_without_workstream_defaults_to_manual_general() {
        let entry = make_entry(
            "no-ws",
            ManualDate::Single(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
        );
        let ev = entry_to_event(&entry, "user");
        assert_eq!(ev.repo.full_name, "manual/general");
    }

    #[test]
    fn entry_to_event_id_is_deterministic() {
        let entry = make_entry(
            "det-test",
            ManualDate::Single(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
        );
        let ev1 = entry_to_event(&entry, "user");
        let ev2 = entry_to_event(&entry, "user");
        assert_eq!(ev1.id, ev2.id);
    }

    #[test]
    fn entry_to_event_source_is_manual() {
        let entry = make_entry(
            "src-test",
            ManualDate::Single(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
        );
        let ev = entry_to_event(&entry, "user");
        assert_eq!(ev.source.system, SourceSystem::Manual);
        assert_eq!(ev.source.opaque_id, Some("src-test".to_string()));
    }

    #[test]
    fn entry_to_event_tags_and_links_preserved() {
        let mut entry = make_entry(
            "meta-test",
            ManualDate::Single(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
        );
        entry.tags = vec!["important".to_string(), "q1".to_string()];
        entry.receipts = vec![
            Link {
                label: "doc".to_string(),
                url: "https://example.com/doc".to_string(),
            },
            Link {
                label: "pr".to_string(),
                url: "https://github.com/org/repo/pull/1".to_string(),
            },
        ];
        let ev = entry_to_event(&entry, "user");
        assert_eq!(ev.tags.len(), 2);
        assert_eq!(ev.links.len(), 2);
    }

    // --- events_in_window edge cases ---

    #[test]
    fn events_in_window_empty_entries() {
        let window = TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        };
        let (events, warnings) = events_in_window(&[], "user", &window);
        assert!(events.is_empty());
        assert!(warnings.is_empty());
    }

    #[test]
    fn events_in_window_event_on_since_boundary_included() {
        let window = TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        };
        let entries = vec![make_entry(
            "boundary",
            ManualDate::Single(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
        )];
        let (events, warnings) = events_in_window(&entries, "user", &window);
        assert_eq!(events.len(), 1);
        assert!(warnings.is_empty());
    }

    #[test]
    fn events_in_window_event_on_until_boundary_excluded() {
        let window = TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        };
        let entries = vec![make_entry(
            "at-until",
            ManualDate::Single(NaiveDate::from_ymd_opt(2025, 2, 1).unwrap()),
        )];
        let (events, _) = events_in_window(&entries, "user", &window);
        assert!(events.is_empty());
    }

    #[test]
    fn events_in_window_range_partially_before_window_warns() {
        let window = TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 1, 15).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 2, 15).unwrap(),
        };
        let entries = vec![make_entry(
            "partial-before",
            ManualDate::Range {
                start: NaiveDate::from_ymd_opt(2025, 1, 10).unwrap(),
                end: NaiveDate::from_ymd_opt(2025, 1, 20).unwrap(),
            },
        )];
        let (events, warnings) = events_in_window(&entries, "user", &window);
        assert_eq!(events.len(), 1);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("partially outside"));
    }

    #[test]
    fn events_in_window_range_partially_after_window_warns() {
        let window = TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        };
        let entries = vec![make_entry(
            "partial-after",
            ManualDate::Range {
                start: NaiveDate::from_ymd_opt(2025, 1, 25).unwrap(),
                end: NaiveDate::from_ymd_opt(2025, 2, 5).unwrap(),
            },
        )];
        let (events, warnings) = events_in_window(&entries, "user", &window);
        assert_eq!(events.len(), 1);
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn events_in_window_range_entirely_before_excluded() {
        let window = TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
        };
        let entries = vec![make_entry(
            "before",
            ManualDate::Range {
                start: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                end: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
            },
        )];
        let (events, _) = events_in_window(&entries, "user", &window);
        assert!(events.is_empty());
    }

    #[test]
    fn events_in_window_range_entirely_after_excluded() {
        let window = TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        };
        let entries = vec![make_entry(
            "after",
            ManualDate::Range {
                start: NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
                end: NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
            },
        )];
        let (events, _) = events_in_window(&entries, "user", &window);
        assert!(events.is_empty());
    }

    // --- Write and re-read roundtrip with all event types ---

    #[test]
    fn roundtrip_write_read_all_event_types() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("all_types.yaml");

        let file = ManualEventsFile {
            version: 1,
            generated_at: Utc::now(),
            events: vec![
                create_entry(
                    "note-1",
                    ManualEventType::Note,
                    ManualDate::Single(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
                    "A Note",
                ),
                create_entry(
                    "incident-1",
                    ManualEventType::Incident,
                    ManualDate::Range {
                        start: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
                        end: NaiveDate::from_ymd_opt(2025, 2, 3).unwrap(),
                    },
                    "An Incident",
                ),
                create_entry(
                    "design-1",
                    ManualEventType::Design,
                    ManualDate::Single(NaiveDate::from_ymd_opt(2025, 3, 1).unwrap()),
                    "A Design",
                ),
            ],
        };

        write_manual_events(&path, &file).unwrap();
        let read = read_manual_events(&path).unwrap();
        assert_eq!(read.events.len(), 3);
        assert_eq!(read.events[0].event_type, ManualEventType::Note);
        assert_eq!(read.events[1].event_type, ManualEventType::Incident);
        assert_eq!(read.events[2].event_type, ManualEventType::Design);
    }

    // --- Snapshot tests ---

    #[test]
    fn snapshot_entry_to_event_single_date() {
        let entry = create_entry(
            "snap-single",
            ManualEventType::Note,
            ManualDate::Single(NaiveDate::from_ymd_opt(2025, 6, 15).unwrap()),
            "Snapshot Test Event",
        );
        let ev = entry_to_event(&entry, "testuser");
        insta::assert_yaml_snapshot!("entry_to_event_single_date", ev, {
            ".occurred_at" => "[timestamp]",
        });
    }

    #[test]
    fn snapshot_entry_to_event_range_date() {
        let mut entry = create_entry(
            "snap-range",
            ManualEventType::Incident,
            ManualDate::Range {
                start: NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
                end: NaiveDate::from_ymd_opt(2025, 3, 5).unwrap(),
            },
            "Multi-day Incident",
        );
        entry.description = Some("Handled production outage".to_string());
        entry.workstream = Some("infra/reliability".to_string());
        entry.impact = Some("Reduced downtime by 50%".to_string());
        let ev = entry_to_event(&entry, "responder");
        insta::assert_yaml_snapshot!("entry_to_event_range_date", ev, {
            ".occurred_at" => "[timestamp]",
        });
    }

    // --- Property tests ---

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

        #[test]
        fn entry_to_event_always_produces_manual_kind(
            id in "[a-zA-Z0-9_-]{1,30}",
            title in "[a-zA-Z0-9 _-]{1,50}",
            user in "[a-zA-Z0-9_]{1,20}"
        ) {
            let entry = create_entry(
                &id,
                ManualEventType::Note,
                ManualDate::Single(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
                &title,
            );
            let ev = entry_to_event(&entry, &user);
            prop_assert_eq!(ev.kind, EventKind::Manual);
            prop_assert_eq!(ev.actor.login, user);
            prop_assert_eq!(ev.source.system, SourceSystem::Manual);
        }

        #[test]
        fn entry_to_event_id_deterministic(
            id in "[a-zA-Z0-9_-]{1,30}"
        ) {
            let entry = make_entry(
                &id,
                ManualDate::Single(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
            );
            let ev1 = entry_to_event(&entry, "u");
            let ev2 = entry_to_event(&entry, "u");
            prop_assert_eq!(ev1.id, ev2.id);
        }

        #[test]
        fn entry_date_range_start_le_end_for_range(
            a in arb_date(),
            b in arb_date()
        ) {
            let (start, end) = if a <= b { (a, b) } else { (b, a) };
            let entry = make_entry("r", ManualDate::Range { start, end });
            let (s, e) = entry_date_range(&entry);
            prop_assert!(s <= e);
        }
    }
}
