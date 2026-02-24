//! Integration tests for cross-crate boundaries (trait contract + API shape).

use shiplog_ports::WorkstreamClusterer;
use shiplog_schema::event::*;
use shiplog_workstream_cluster::RepoClusterer;
use shiplog_workstream_receipt_policy::{
    WORKSTREAM_RECEIPT_LIMIT_MANUAL, WORKSTREAM_RECEIPT_LIMIT_REVIEW, WORKSTREAM_RECEIPT_LIMIT_TOTAL,
};

fn event(repo: &str, id: &str, number: u64, kind: EventKind) -> EventEnvelope {
    EventEnvelope {
        id: shiplog_ids::EventId::from_parts(["integration", id]),
        kind: kind.clone(),
        occurred_at: chrono::Utc::now(),
        actor: Actor {
            login: "agent".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: repo.to_string(),
            html_url: Some(format!("https://example.test/{repo}")),
            visibility: RepoVisibility::Unknown,
        },
        payload: match kind {
            EventKind::PullRequest => EventPayload::PullRequest(PullRequestEvent {
                number,
                title: "Integration PR".into(),
                state: PullRequestState::Merged,
                created_at: chrono::Utc::now(),
                merged_at: Some(chrono::Utc::now()),
                additions: Some(0),
                deletions: Some(0),
                changed_files: Some(0),
                touched_paths_hint: vec![],
                window: None,
            }),
            EventKind::Review => EventPayload::Review(ReviewEvent {
                pull_number: number,
                pull_title: "Integration review".into(),
                submitted_at: chrono::Utc::now(),
                state: "approved".into(),
                window: None,
            }),
            EventKind::Manual => EventPayload::Manual(ManualEvent {
                event_type: ManualEventType::Note,
                title: "Integration manual".into(),
                description: None,
                started_at: None,
                ended_at: None,
                impact: None,
            }),
        },
        tags: vec![],
        links: vec![],
        source: SourceRef {
            system: SourceSystem::Unknown,
            url: None,
            opaque_id: None,
        },
    }
}

#[test]
fn repo_clusterer_assigns_by_repository_and_is_trait_object_safe() {
    let events = vec![
        event("repo/alpha", "a1", 1, EventKind::PullRequest),
        event("repo/alpha", "a2", 2, EventKind::PullRequest),
        event("repo/beta", "b1", 3, EventKind::PullRequest),
    ];

    let clusterer: Box<dyn WorkstreamClusterer> = Box::new(RepoClusterer);
    let output = clusterer
        .cluster(&events)
        .expect("clusterer should produce a workstream file");

    assert_eq!(output.version, 1);
    assert_eq!(output.workstreams.len(), 2);
    assert!(output.workstreams.iter().all(|ws| ws.tags.contains(&"repo".to_string())));
}

#[test]
fn repo_clusterer_obeys_receipt_policy_caps() {
    let events = (0..12)
        .map(|i| event("repo/review-policy", &format!("r{i}"), i, EventKind::Review))
        .chain((0..12).map(|i| {
            event("repo/manual-policy", &format!("m{i}"), i, EventKind::Manual)
        }))
        .chain((0..12).map(|i| event("repo/pr-policy", &format!("p{i}"), i, EventKind::PullRequest)))
        .collect::<Vec<_>>();

    let workstreams = RepoClusterer.cluster(&events).unwrap().workstreams;
    let review_ws = workstreams.iter().find(|ws| ws.title == "repo/review-policy").unwrap();
    let manual_ws = workstreams.iter().find(|ws| ws.title == "repo/manual-policy").unwrap();
    let pr_ws = workstreams.iter().find(|ws| ws.title == "repo/pr-policy").unwrap();

    assert_eq!(review_ws.receipts.len(), WORKSTREAM_RECEIPT_LIMIT_REVIEW);
    assert_eq!(manual_ws.receipts.len(), WORKSTREAM_RECEIPT_LIMIT_MANUAL);
    assert_eq!(pr_ws.receipts.len(), WORKSTREAM_RECEIPT_LIMIT_TOTAL);
}
