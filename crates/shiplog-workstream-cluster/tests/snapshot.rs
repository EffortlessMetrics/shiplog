//! Snapshot tests for workstream clustering output structure.

use chrono::{TimeZone, Utc};
use shiplog_ids::EventId;
use shiplog_ports::WorkstreamClusterer;
use shiplog_schema::event::*;
use shiplog_workstream_cluster::RepoClusterer;

fn fixed_event(repo: &str, id: &str, number: u64, kind: EventKind) -> EventEnvelope {
    let ts = Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).single().unwrap();
    EventEnvelope {
        id: EventId::from_parts(["snap", id]),
        kind: kind.clone(),
        occurred_at: ts,
        actor: Actor {
            login: "dev".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: repo.into(),
            html_url: Some(format!("https://github.com/{repo}")),
            visibility: RepoVisibility::Unknown,
        },
        payload: match kind {
            EventKind::PullRequest => EventPayload::PullRequest(PullRequestEvent {
                number,
                title: format!("PR #{number}"),
                state: PullRequestState::Merged,
                created_at: ts,
                merged_at: Some(ts),
                additions: Some(10),
                deletions: Some(2),
                changed_files: Some(3),
                touched_paths_hint: vec![],
                window: None,
            }),
            EventKind::Review => EventPayload::Review(ReviewEvent {
                pull_number: number,
                pull_title: format!("Review for #{number}"),
                submitted_at: ts,
                state: "approved".into(),
                window: None,
            }),
            EventKind::Manual => EventPayload::Manual(ManualEvent {
                event_type: ManualEventType::Note,
                title: format!("Note #{number}"),
                description: None,
                started_at: None,
                ended_at: None,
                impact: None,
            }),
        },
        tags: vec![],
        links: vec![],
        source: SourceRef {
            system: SourceSystem::Github,
            url: None,
            opaque_id: None,
        },
    }
}

#[test]
fn snapshot_single_repo_cluster_structure() {
    let events = vec![
        fixed_event("acme/app", "pr1", 1, EventKind::PullRequest),
        fixed_event("acme/app", "pr2", 2, EventKind::PullRequest),
        fixed_event("acme/app", "rev1", 3, EventKind::Review),
    ];

    let ws = RepoClusterer.cluster(&events).unwrap();
    // Snapshot only the workstreams array (generated_at is non-deterministic)
    insta::assert_yaml_snapshot!("single_repo_workstreams", ws.workstreams);
}

#[test]
fn snapshot_multi_repo_cluster_structure() {
    let events = vec![
        fixed_event("acme/app", "a1", 1, EventKind::PullRequest),
        fixed_event("acme/lib", "b1", 2, EventKind::PullRequest),
        fixed_event("acme/lib", "b2", 3, EventKind::Review),
        fixed_event("acme/docs", "c1", 4, EventKind::Manual),
    ];

    let ws = RepoClusterer.cluster(&events).unwrap();
    insta::assert_yaml_snapshot!("multi_repo_workstreams", ws.workstreams);
}

#[test]
fn snapshot_empty_cluster() {
    let ws = RepoClusterer.cluster(&[]).unwrap();
    insta::assert_yaml_snapshot!("empty_workstreams", ws.workstreams);
}

#[test]
fn snapshot_mixed_kinds_stats() {
    let events = vec![
        fixed_event("org/proj", "pr1", 1, EventKind::PullRequest),
        fixed_event("org/proj", "pr2", 2, EventKind::PullRequest),
        fixed_event("org/proj", "rev1", 3, EventKind::Review),
        fixed_event("org/proj", "rev2", 4, EventKind::Review),
        fixed_event("org/proj", "man1", 5, EventKind::Manual),
    ];

    let ws = RepoClusterer.cluster(&events).unwrap();
    let stats: Vec<_> = ws
        .workstreams
        .iter()
        .map(|w| (&w.title, &w.stats))
        .collect();
    insta::assert_yaml_snapshot!("mixed_kinds_stats", stats);
}
