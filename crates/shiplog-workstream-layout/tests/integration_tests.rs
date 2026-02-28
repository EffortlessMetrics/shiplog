//! Integration tests for shiplog-workstream-layout.

use chrono::{TimeZone, Utc};
use shiplog_ids::EventId;
use shiplog_schema::event::EventEnvelope;
use shiplog_schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};
use shiplog_workstream_cluster::RepoClusterer;
use shiplog_workstream_layout::{
    CURATED_FILENAME, SUGGESTED_FILENAME, WorkstreamManager, load_or_cluster, write_workstreams,
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

#[test]
fn integration_load_or_cluster_with_real_clusterer() {
    let events = [make_event("acme/app", "1"), make_event("acme/lib", "2")];

    let loaded = load_or_cluster(None, &RepoClusterer, &events).unwrap();
    assert_eq!(loaded.workstreams.len(), 2);
}

#[test]
fn integration_write_suggested_then_load_effective_reads_it() {
    let temp_dir = tempdir().unwrap();
    let ws = make_file("auto-suggested");
    WorkstreamManager::write_suggested(temp_dir.path(), &ws).unwrap();

    let loaded = WorkstreamManager::load_effective(temp_dir.path(), &RepoClusterer, &[]).unwrap();
    assert_eq!(loaded.workstreams[0].title, "auto-suggested");
}

#[test]
fn integration_curated_overrides_suggested_on_load_effective() {
    let temp_dir = tempdir().unwrap();
    WorkstreamManager::write_suggested(temp_dir.path(), &make_file("old-suggested")).unwrap();
    write_workstreams(
        &WorkstreamManager::curated_path(temp_dir.path()),
        &make_file("user-curated"),
    )
    .unwrap();

    let loaded = WorkstreamManager::load_effective(temp_dir.path(), &RepoClusterer, &[]).unwrap();
    assert_eq!(loaded.workstreams[0].title, "user-curated");
}

#[test]
fn integration_multiple_workstreams_roundtrip() {
    let temp_dir = tempdir().unwrap();
    let ws = WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![
            Workstream {
                id: shiplog_ids::WorkstreamId::from_parts(["repo", "a"]),
                title: "repo-a".into(),
                summary: Some("First".into()),
                tags: vec!["repo".into()],
                stats: WorkstreamStats {
                    pull_requests: 3,
                    reviews: 1,
                    manual_events: 0,
                },
                events: vec![EventId::from_parts(["e", "1"])],
                receipts: vec![EventId::from_parts(["e", "1"])],
            },
            Workstream {
                id: shiplog_ids::WorkstreamId::from_parts(["repo", "b"]),
                title: "repo-b".into(),
                summary: None,
                tags: vec!["repo".into()],
                stats: WorkstreamStats::zero(),
                events: vec![],
                receipts: vec![],
            },
        ],
    };

    let path = temp_dir.path().join(CURATED_FILENAME);
    write_workstreams(&path, &ws).unwrap();

    let loaded = WorkstreamManager::try_load(temp_dir.path())
        .unwrap()
        .unwrap();
    assert_eq!(loaded.workstreams.len(), 2);
    assert_eq!(loaded.workstreams[0].title, "repo-a");
    assert_eq!(loaded.workstreams[1].title, "repo-b");
}
