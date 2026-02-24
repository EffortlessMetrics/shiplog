//! Property tests for shiplog-workstream-layout.

use proptest::prelude::*;
use shiplog_ids::{EventId, WorkstreamId};
use shiplog_schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};
use shiplog_workstream_layout::write_workstreams;
use tempfile::tempdir;

fn alpha_token() -> impl Strategy<Value = String> {
    prop::collection::vec(0u8..=25u8, 1..10).prop_map(|chars| {
        chars
            .into_iter()
            .map(|b| (b + b'a') as char)
            .collect::<String>()
    })
}

fn event_ids() -> impl Strategy<Value = Vec<EventId>> {
    prop::collection::vec(alpha_token(), 0..5).prop_map(|tokens| {
        tokens
            .into_iter()
            .map(|token| EventId::from_parts(["id", &token]))
            .collect()
    })
}

fn workstream() -> impl Strategy<Value = Workstream> {
    (
        alpha_token(),
        prop::option::of(alpha_token()),
        event_ids(),
        event_ids(),
    )
        .prop_map(|(title, summary, events, receipts)| Workstream {
            id: WorkstreamId::from_parts(["ws", &title]),
            title,
            summary,
            tags: vec!["repo".into()],
            stats: WorkstreamStats::zero(),
            events,
            receipts,
        })
}

fn workstreams_file() -> impl Strategy<Value = WorkstreamsFile> {
    prop::collection::vec(workstream(), 0..6).prop_map(|workstreams| WorkstreamsFile {
        version: 1,
        generated_at: chrono::Utc::now(),
        workstreams,
    })
}

proptest! {
    #[test]
    fn prop_write_roundtrip_preserves_data(ws in workstreams_file()) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("workstreams.yaml");
        write_workstreams(&path, &ws).unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        let roundtrip: WorkstreamsFile = serde_yaml::from_str(&text).unwrap();
        prop_assert_eq!(roundtrip, ws);
    }

    #[test]
    fn prop_reading_serialized_yaml_roundtrips(ws in workstreams_file()) {
        let yaml = serde_yaml::to_string(&ws).unwrap();
        let parsed: WorkstreamsFile = serde_yaml::from_str(&yaml).unwrap();
        prop_assert_eq!(parsed, ws);
    }
}
