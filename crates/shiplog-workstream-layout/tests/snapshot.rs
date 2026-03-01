//! Snapshot tests for workstream layout YAML output.

use chrono::{TimeZone, Utc};
use shiplog_ids::{EventId, WorkstreamId};
use shiplog_schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};
use shiplog_workstream_layout::write_workstreams;
use tempfile::tempdir;

fn fixed_ws_file() -> WorkstreamsFile {
    let ts = Utc
        .with_ymd_and_hms(2025, 6, 15, 12, 0, 0)
        .single()
        .unwrap();
    WorkstreamsFile {
        version: 1,
        generated_at: ts,
        workstreams: vec![
            Workstream {
                id: WorkstreamId::from_parts(["repo", "acme/app"]),
                title: "acme/app".into(),
                summary: Some("Main application".into()),
                tags: vec!["repo".into()],
                stats: WorkstreamStats {
                    pull_requests: 5,
                    reviews: 2,
                    manual_events: 1,
                },
                events: vec![
                    EventId::from_parts(["github", "pr1"]),
                    EventId::from_parts(["github", "pr2"]),
                ],
                receipts: vec![EventId::from_parts(["github", "pr1"])],
            },
            Workstream {
                id: WorkstreamId::from_parts(["repo", "acme/lib"]),
                title: "acme/lib".into(),
                summary: None,
                tags: vec!["repo".into()],
                stats: WorkstreamStats {
                    pull_requests: 1,
                    reviews: 0,
                    manual_events: 0,
                },
                events: vec![EventId::from_parts(["github", "lib1"])],
                receipts: vec![EventId::from_parts(["github", "lib1"])],
            },
        ],
    }
}

#[test]
fn snapshot_yaml_output_format() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("workstreams.yaml");
    let ws = fixed_ws_file();
    write_workstreams(&path, &ws).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    insta::assert_snapshot!("workstreams_yaml_format", text);
}

#[test]
fn snapshot_empty_workstreams_yaml() {
    let ts = Utc
        .with_ymd_and_hms(2025, 6, 15, 12, 0, 0)
        .single()
        .unwrap();
    let ws = WorkstreamsFile {
        version: 1,
        generated_at: ts,
        workstreams: vec![],
    };
    let dir = tempdir().unwrap();
    let path = dir.path().join("empty.yaml");
    write_workstreams(&path, &ws).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    insta::assert_snapshot!("empty_workstreams_yaml", text);
}

#[test]
fn snapshot_path_constants() {
    insta::assert_snapshot!(
        "curated_filename",
        shiplog_workstream_layout::CURATED_FILENAME
    );
    insta::assert_snapshot!(
        "suggested_filename",
        shiplog_workstream_layout::SUGGESTED_FILENAME
    );
}
