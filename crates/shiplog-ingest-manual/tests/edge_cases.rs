//! Edge-case tests for shiplog-ingest-manual.
//!
//! Covers YAML parsing edge cases (malformed dates, missing fields, extra
//! fields), multiple event types in one file, unicode in descriptions,
//! and empty events files.

use chrono::{NaiveDate, Utc};
use shiplog_ingest_manual::{ManualIngestor, read_manual_events, write_manual_events};
use shiplog_ports::Ingestor;
use shiplog_schema::event::{
    Link, ManualDate, ManualEventEntry, ManualEventType, ManualEventsFile,
};

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn window_ingestor(path: &std::path::Path) -> ManualIngestor {
    ManualIngestor::new(
        path,
        "testuser".to_string(),
        date(2025, 1, 1),
        date(2025, 12, 31),
    )
}

fn minimal_entry(id: &str, event_type: ManualEventType, d: NaiveDate) -> ManualEventEntry {
    ManualEventEntry {
        id: id.to_string(),
        event_type,
        date: ManualDate::Single(d),
        title: format!("Event {id}"),
        description: None,
        workstream: None,
        tags: vec![],
        receipts: vec![],
        impact: None,
    }
}

// ── YAML parsing: malformed dates ────────────────────────────────────────

#[test]
fn malformed_date_in_yaml_returns_error() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("manual_events.yaml");
    // Write YAML with an invalid date value
    std::fs::write(
        &path,
        r#"version: 1
generated_at: "2025-01-01T00:00:00Z"
events:
  - id: bad-date
    type: Note
    date: "not-a-date"
    title: "Bad date event"
"#,
    )
    .unwrap();

    let result = read_manual_events(&path);
    assert!(result.is_err(), "malformed date should cause parse error");
}

#[test]
fn incomplete_date_in_yaml_returns_error() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("manual_events.yaml");
    std::fs::write(
        &path,
        r#"version: 1
generated_at: "2025-01-01T00:00:00Z"
events:
  - id: incomplete-date
    type: Note
    date: "2025-13"
    title: "Incomplete date"
"#,
    )
    .unwrap();

    let result = read_manual_events(&path);
    assert!(result.is_err(), "incomplete date should cause parse error");
}

// ── YAML parsing: missing required fields ────────────────────────────────

#[test]
fn missing_id_field_returns_error() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("manual_events.yaml");
    std::fs::write(
        &path,
        r#"version: 1
generated_at: "2025-01-01T00:00:00Z"
events:
  - type: Note
    date: "2025-03-15"
    title: "No ID"
"#,
    )
    .unwrap();

    let result = read_manual_events(&path);
    assert!(result.is_err(), "missing id should cause error");
}

#[test]
fn missing_title_field_returns_error() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("manual_events.yaml");
    std::fs::write(
        &path,
        r#"version: 1
generated_at: "2025-01-01T00:00:00Z"
events:
  - id: no-title
    type: Note
    date: "2025-03-15"
"#,
    )
    .unwrap();

    let result = read_manual_events(&path);
    assert!(result.is_err(), "missing title should cause error");
}

#[test]
fn missing_type_field_returns_error() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("manual_events.yaml");
    std::fs::write(
        &path,
        r#"version: 1
generated_at: "2025-01-01T00:00:00Z"
events:
  - id: no-type
    date: "2025-03-15"
    title: "No type field"
"#,
    )
    .unwrap();

    let result = read_manual_events(&path);
    assert!(result.is_err(), "missing type should cause error");
}

#[test]
fn missing_date_field_returns_error() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("manual_events.yaml");
    std::fs::write(
        &path,
        r#"version: 1
generated_at: "2025-01-01T00:00:00Z"
events:
  - id: no-date
    type: Note
    title: "No date field"
"#,
    )
    .unwrap();

    let result = read_manual_events(&path);
    assert!(result.is_err(), "missing date should cause error");
}

// ── YAML parsing: extra fields are tolerated ─────────────────────────────

#[test]
fn extra_unknown_fields_are_ignored_or_tolerated() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("manual_events.yaml");
    // Write a valid entry plus an extra field
    std::fs::write(
        &path,
        r#"version: 1
generated_at: "2025-01-01T00:00:00Z"
events:
  - id: extra-fields
    type: Note
    date: "2025-03-15"
    title: "Has extra"
    unknown_field: "this should be ignored"
    another_extra: 42
"#,
    )
    .unwrap();

    // serde(deny_unknown_fields) would make this fail; otherwise it passes.
    // We just verify the behavior is consistent.
    let result = read_manual_events(&path);
    // If the schema denies unknown fields, this is an error; if it allows, it succeeds.
    // Either way, no panic.
    match result {
        Ok(file) => {
            assert_eq!(file.events.len(), 1);
            assert_eq!(file.events[0].id, "extra-fields");
        }
        Err(e) => {
            let msg = format!("{e:#}");
            assert!(
                msg.contains("unknown") || msg.contains("field"),
                "error should mention unknown fields: {msg}"
            );
        }
    }
}

// ── Multiple event types in one file ─────────────────────────────────────

#[test]
fn all_event_types_in_one_file_roundtrip() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("manual_events.yaml");

    let d = date(2025, 6, 1);
    let file = ManualEventsFile {
        version: 1,
        generated_at: Utc::now(),
        events: vec![
            minimal_entry("note-1", ManualEventType::Note, d),
            minimal_entry("incident-1", ManualEventType::Incident, d),
            minimal_entry("design-1", ManualEventType::Design, d),
            minimal_entry("mentoring-1", ManualEventType::Mentoring, d),
            minimal_entry("launch-1", ManualEventType::Launch, d),
            minimal_entry("migration-1", ManualEventType::Migration, d),
            minimal_entry("review-1", ManualEventType::Review, d),
            minimal_entry("other-1", ManualEventType::Other, d),
        ],
    };
    write_manual_events(&path, &file).unwrap();

    let ing = window_ingestor(&path);
    let output = ing.ingest().unwrap();
    assert_eq!(
        output.events.len(),
        8,
        "all 8 event types should be ingested"
    );
    assert_eq!(output.coverage.slices[0].total_count, 8);
}

#[test]
fn mixed_single_and_range_dates_in_one_file() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("manual_events.yaml");

    let file = ManualEventsFile {
        version: 1,
        generated_at: Utc::now(),
        events: vec![
            ManualEventEntry {
                id: "single-date".into(),
                event_type: ManualEventType::Note,
                date: ManualDate::Single(date(2025, 3, 15)),
                title: "Single date event".into(),
                description: None,
                workstream: None,
                tags: vec![],
                receipts: vec![],
                impact: None,
            },
            ManualEventEntry {
                id: "range-date".into(),
                event_type: ManualEventType::Incident,
                date: ManualDate::Range {
                    start: date(2025, 4, 1),
                    end: date(2025, 4, 5),
                },
                title: "Range date event".into(),
                description: None,
                workstream: None,
                tags: vec![],
                receipts: vec![],
                impact: None,
            },
        ],
    };
    write_manual_events(&path, &file).unwrap();

    let ing = window_ingestor(&path);
    let output = ing.ingest().unwrap();
    assert_eq!(output.events.len(), 2);
}

// ── Unicode in event descriptions ────────────────────────────────────────

#[test]
fn unicode_in_title_and_description_roundtrips() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("manual_events.yaml");

    let file = ManualEventsFile {
        version: 1,
        generated_at: Utc::now(),
        events: vec![ManualEventEntry {
            id: "unicode-1".into(),
            event_type: ManualEventType::Note,
            date: ManualDate::Single(date(2025, 5, 1)),
            title: "設計ドキュメント 🚀 αβγδ".into(),
            description: Some("Ñoño café résumé — «guillemets» ∑∫∂ 你好世界".into()),
            workstream: Some("internationalización".into()),
            tags: vec!["日本語".into(), "中文".into()],
            receipts: vec![Link {
                label: "ドキュメント".into(),
                url: "https://example.com/文書".into(),
            }],
            impact: Some("改善 — improved performance by 50%".into()),
        }],
    };
    write_manual_events(&path, &file).unwrap();

    let read = read_manual_events(&path).unwrap();
    assert_eq!(read.events[0].title, "設計ドキュメント 🚀 αβγδ");
    assert_eq!(
        read.events[0].description.as_deref(),
        Some("Ñoño café résumé — «guillemets» ∑∫∂ 你好世界")
    );
}

#[test]
fn unicode_events_are_ingested_correctly() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("manual_events.yaml");

    let file = ManualEventsFile {
        version: 1,
        generated_at: Utc::now(),
        events: vec![ManualEventEntry {
            id: "emoji-event".into(),
            event_type: ManualEventType::Design,
            date: ManualDate::Single(date(2025, 3, 1)),
            title: "🎉 Feature launch 🚀".into(),
            description: Some("Тест описания на русском".into()),
            workstream: None,
            tags: vec![],
            receipts: vec![],
            impact: None,
        }],
    };
    write_manual_events(&path, &file).unwrap();

    let ing = window_ingestor(&path);
    let output = ing.ingest().unwrap();
    assert_eq!(output.events.len(), 1);
}

// ── Empty events file ────────────────────────────────────────────────────

#[test]
fn empty_events_list_produces_complete_coverage() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("manual_events.yaml");

    let file = ManualEventsFile {
        version: 1,
        generated_at: Utc::now(),
        events: vec![],
    };
    write_manual_events(&path, &file).unwrap();

    let ing = window_ingestor(&path);
    let output = ing.ingest().unwrap();
    assert!(output.events.is_empty());
    assert_eq!(output.coverage.slices[0].total_count, 0);
    assert_eq!(output.coverage.slices[0].fetched, 0);
    assert!(output.coverage.warnings.is_empty());
}

#[test]
fn nonexistent_file_returns_empty_with_warning() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("does_not_exist.yaml");

    let ing = window_ingestor(&path);
    let output = ing.ingest().unwrap();
    assert!(output.events.is_empty());
    assert!(!output.coverage.warnings.is_empty());
    assert!(
        output
            .coverage
            .warnings
            .iter()
            .any(|w| w.contains("not found")),
        "warning should mention file not found"
    );
}

// ── Full optional fields populated ───────────────────────────────────────

#[test]
fn fully_populated_entry_roundtrips() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("manual_events.yaml");

    let file = ManualEventsFile {
        version: 1,
        generated_at: Utc::now(),
        events: vec![ManualEventEntry {
            id: "full-entry".into(),
            event_type: ManualEventType::Incident,
            date: ManualDate::Range {
                start: date(2025, 2, 1),
                end: date(2025, 2, 5),
            },
            title: "Major incident response".into(),
            description: Some("Handled P0 outage in production".into()),
            workstream: Some("reliability".into()),
            tags: vec!["p0".into(), "on-call".into(), "production".into()],
            receipts: vec![
                Link {
                    label: "incident report".into(),
                    url: "https://example.com/incident/123".into(),
                },
                Link {
                    label: "postmortem".into(),
                    url: "https://example.com/postmortem/456".into(),
                },
            ],
            impact: Some("Reduced MTTR from 4h to 45min".into()),
        }],
    };
    write_manual_events(&path, &file).unwrap();

    let read = read_manual_events(&path).unwrap();
    let entry = &read.events[0];
    assert_eq!(entry.id, "full-entry");
    assert_eq!(entry.tags.len(), 3);
    assert_eq!(entry.receipts.len(), 2);
    assert_eq!(
        entry.impact.as_deref(),
        Some("Reduced MTTR from 4h to 45min")
    );
}

// ── Completely empty YAML file ───────────────────────────────────────────

#[test]
fn completely_empty_file_returns_error() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("manual_events.yaml");
    std::fs::write(&path, "").unwrap();

    let ing = window_ingestor(&path);
    let result = ing.ingest();
    assert!(result.is_err(), "empty file should cause parse error");
}

#[test]
fn whitespace_only_file_returns_error() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("manual_events.yaml");
    std::fs::write(&path, "   \n\n   \n").unwrap();

    let ing = window_ingestor(&path);
    let result = ing.ingest();
    assert!(
        result.is_err(),
        "whitespace-only file should cause parse error"
    );
}

// ── Binary / garbage content ─────────────────────────────────────────────

#[test]
fn binary_content_returns_error() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("manual_events.yaml");
    std::fs::write(&path, [0xFF, 0xFE, 0x00, 0x01, 0x02, 0x03]).unwrap();

    let ing = window_ingestor(&path);
    let result = ing.ingest();
    assert!(result.is_err(), "binary content should cause parse error");
}
