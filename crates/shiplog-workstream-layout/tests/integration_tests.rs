//! Integration tests for shiplog-workstream-layout.

use chrono::{TimeZone, Utc};
use shiplog_ids::EventId;
use shiplog_schema::event::EventEnvelope;
use shiplog_schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};
use shiplog_workstream_cluster::RepoClusterer;
use shiplog_workstream_layout::{
    CURATED_FILENAME, SUGGESTED_FILENAME, WorkstreamManager, write_workstreams,
};
use tempfile::tempdir;

fn make_event(repo_name: &str, id: &str) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["github", id]),
        kind: shiplog_schema::event::EventKind::PullRequest,
        occurred_at: Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).single().unwrap(),
        actor: shiplog_schema::event::Actor {
            login: "tester".into(),
            id: None,
        },
        repo: shiplog_schema::event::RepoRef {
            full_name: repo_name.into(),
            html_url: Some(format!("https://example.com/{repo_name}")),
            visibility: shiplog_schema::event::RepoVisibility::Unknown,
        },
        payload: shiplog_schema::event::EventPayload::PullRequest(
            shiplog_schema::event::PullRequestEvent {
                number: 1,
                title: "Test PR".into(),
                state: shiplog_schema::event::PullRequestState::Merged,
                created_at: Utc::now(),
                merged_at: Some(Utc::now()),
                additions: Some(10),
                deletions: Some(2),
                changed_files: Some(1),
                touched_paths_hint: vec![],
                window: None,
            },
        ),
        tags: vec![],
        links: vec![],
        source: shiplog_schema::event::SourceRef {
            system: shiplog_schema::event::SourceSystem::Github,
            url: None,
            opaque_id: None,
        },
    }
}

fn make_file(title: &str) -> WorkstreamsFile {
    WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![Workstream {
            id: shiplog_ids::WorkstreamId::from_parts(["repo", title]),
            title: title.to_string(),
            summary: None,
            tags: vec!["repo".into()],
            stats: WorkstreamStats::zero(),
            events: vec![EventId::from_parts(["event", title])],
            receipts: vec![],
        }],
    }
}

#[test]
fn integration_prefers_curated_workstreams() {
    let temp_dir = tempdir().unwrap();

    let curated = temp_dir.path().join(CURATED_FILENAME);
    let suggested = temp_dir.path().join(SUGGESTED_FILENAME);
    write_workstreams(&curated, &make_file("curated")).unwrap();
    write_workstreams(&suggested, &make_file("suggested")).unwrap();

    let loaded = WorkstreamManager::try_load(temp_dir.path())
        .unwrap()
        .unwrap();
    assert_eq!(loaded.workstreams[0].title, "curated");
}

#[test]
fn integration_falls_back_to_suggested_when_curated_missing() {
    let temp_dir = tempdir().unwrap();

    let suggested = temp_dir.path().join(SUGGESTED_FILENAME);
    write_workstreams(&suggested, &make_file("suggested")).unwrap();

    let loaded = WorkstreamManager::try_load(temp_dir.path())
        .unwrap()
        .unwrap();
    assert_eq!(loaded.workstreams[0].title, "suggested");
}

#[test]
fn integration_generates_when_no_files_exist() {
    let temp_dir = tempdir().unwrap();
    let events = [make_event("acme/app", "1"), make_event("acme/app", "2")];

    let loaded =
        WorkstreamManager::load_effective(temp_dir.path(), &RepoClusterer, &events).unwrap();
    assert_eq!(loaded.workstreams.len(), 1);
    assert_eq!(loaded.workstreams[0].title, "acme/app");
    assert_eq!(
        WorkstreamManager::suggested_path(temp_dir.path())
            .file_name()
            .unwrap(),
        SUGGESTED_FILENAME
    );
    assert!(WorkstreamManager::suggested_path(temp_dir.path()).exists());
}
