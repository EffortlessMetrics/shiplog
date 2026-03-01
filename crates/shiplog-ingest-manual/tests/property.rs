//! Property-based tests for the manual event ingestor.

use chrono::{NaiveDate, Utc};
use proptest::prelude::*;
use shiplog_ingest_manual::{ManualIngestor, write_manual_events};
use shiplog_ports::Ingestor;
use shiplog_schema::event::{ManualDate, ManualEventEntry, ManualEventType, ManualEventsFile};

// ---------------------------------------------------------------------------
// Strategies
// ---------------------------------------------------------------------------

fn arb_date() -> impl Strategy<Value = NaiveDate> {
    // 2020-01-01 … ~2030-12-31
    (0u32..4000).prop_map(|d| {
        NaiveDate::from_ymd_opt(2020, 1, 1)
            .unwrap()
            .checked_add_days(chrono::Days::new(d as u64))
            .unwrap()
    })
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

fn arb_event_type() -> impl Strategy<Value = ManualEventType> {
    prop_oneof![
        Just(ManualEventType::Note),
        Just(ManualEventType::Incident),
        Just(ManualEventType::Design),
        Just(ManualEventType::Mentoring),
        Just(ManualEventType::Launch),
        Just(ManualEventType::Migration),
        Just(ManualEventType::Review),
        Just(ManualEventType::Other),
    ]
}

fn arb_entry() -> impl Strategy<Value = ManualEventEntry> {
    (
        "[a-z0-9_-]{1,30}",
        arb_event_type(),
        arb_manual_date(),
        "[A-Za-z0-9 ]{1,60}",
    )
        .prop_map(|(id, event_type, date, title)| ManualEventEntry {
            id,
            event_type,
            date,
            title,
            description: None,
            workstream: None,
            tags: Vec::new(),
            receipts: Vec::new(),
            impact: None,
        })
}

fn arb_date_range() -> impl Strategy<Value = (NaiveDate, NaiveDate)> {
    (arb_date(), arb_date()).prop_map(|(a, b)| if a <= b { (a, b) } else { (b, a) })
}

// ---------------------------------------------------------------------------
// Properties
// ---------------------------------------------------------------------------

proptest! {
    /// Any generated YAML file round-trips through write→read→ingest without panic.
    #[test]
    fn valid_yaml_always_parses_without_panic(
        entries in proptest::collection::vec(arb_entry(), 0..10),
        (since, until) in arb_date_range(),
    ) {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("manual_events.yaml");

        let file = ManualEventsFile {
            version: 1,
            generated_at: Utc::now(),
            events: entries,
        };

        write_manual_events(&path, &file).unwrap();

        let ing = ManualIngestor::new(&path, "propuser".to_string(), since, until);
        let output = ing.ingest().unwrap();

        // Basic sanity: coverage is always present
        prop_assert!(!output.coverage.sources.is_empty());
        prop_assert_eq!(output.coverage.mode.as_str(), "manual");
    }

    /// Date filtering is consistent: every returned event overlaps the window.
    #[test]
    fn ingested_events_overlap_date_window(
        entries in proptest::collection::vec(arb_entry(), 1..15),
        (since, until) in arb_date_range(),
    ) {
        // Deduplicate IDs so lookup by opaque_id is unambiguous.
        let mut seen = std::collections::HashSet::new();
        let entries: Vec<_> = entries
            .into_iter()
            .enumerate()
            .map(|(i, mut e)| { e.id = format!("{}-{i}", e.id); e })
            .filter(|e| seen.insert(e.id.clone()))
            .collect();
        if entries.is_empty() { return Ok(()); }

        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("manual_events.yaml");

        let file = ManualEventsFile {
            version: 1,
            generated_at: Utc::now(),
            events: entries.clone(),
        };
        write_manual_events(&path, &file).unwrap();

        let ing = ManualIngestor::new(&path, "propuser".to_string(), since, until);
        let output = ing.ingest().unwrap();

        // Every returned event must overlap the [since, until) window.
        for ev in &output.events {
            let opaque = ev.source.opaque_id.as_deref().unwrap();
            let entry = entries.iter().find(|e| e.id == opaque).unwrap();
            let (start, end) = match &entry.date {
                ManualDate::Single(d) => (*d, *d),
                ManualDate::Range { start, end } => (*start, *end),
            };
            // Overlap condition: NOT (end < since OR start >= until)
            prop_assert!(
                !(end < since || start >= until),
                "event '{}' [{}, {}] should overlap window [{}, {})",
                opaque, start, end, since, until
            );
        }
    }

    /// Coverage counts are consistent with the input and output.
    #[test]
    fn coverage_counts_are_consistent(
        entries in proptest::collection::vec(arb_entry(), 0..10),
        (since, until) in arb_date_range(),
    ) {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("manual_events.yaml");

        let file = ManualEventsFile {
            version: 1,
            generated_at: Utc::now(),
            events: entries.clone(),
        };
        write_manual_events(&path, &file).unwrap();

        let ing = ManualIngestor::new(&path, "propuser".to_string(), since, until);
        let output = ing.ingest().unwrap();

        let slice = &output.coverage.slices[0];
        prop_assert_eq!(slice.total_count, entries.len() as u64);
        prop_assert_eq!(slice.fetched, output.events.len() as u64);
        prop_assert!(slice.fetched <= slice.total_count);
    }

    /// Filtering is idempotent: re-ingesting the same file gives the same count.
    #[test]
    fn filtering_is_idempotent(
        entries in proptest::collection::vec(arb_entry(), 0..10),
        (since, until) in arb_date_range(),
    ) {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("manual_events.yaml");

        let file = ManualEventsFile {
            version: 1,
            generated_at: Utc::now(),
            events: entries,
        };
        write_manual_events(&path, &file).unwrap();

        let ing = ManualIngestor::new(&path, "propuser".to_string(), since, until);
        let out1 = ing.ingest().unwrap();
        let out2 = ing.ingest().unwrap();

        prop_assert_eq!(out1.events.len(), out2.events.len());
    }
}
