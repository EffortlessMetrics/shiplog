//! Edge-case tests for shiplog-workstream-cluster.
//!
//! Covers clustering with many repos, special characters in repo names,
//! unicode repos, sorting stability, and custom clusterer strategies.

use chrono::Utc;
use shiplog_ids::EventId;
use shiplog_ports::WorkstreamClusterer;
use shiplog_schema::event::*;
use shiplog_schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};
use shiplog_workstream_cluster::RepoClusterer;
use shiplog_workstream_receipt_policy::WORKSTREAM_RECEIPT_LIMIT_TOTAL;

fn event(repo: &str, id: &str, number: u64, kind: EventKind) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["edge", id]),
        kind: kind.clone(),
        occurred_at: Utc::now(),
        actor: Actor {
            login: "dev".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: repo.to_string(),
            html_url: Some(format!("https://example.com/{repo}")),
            visibility: RepoVisibility::Unknown,
        },
        payload: match kind {
            EventKind::PullRequest => EventPayload::PullRequest(PullRequestEvent {
                number,
                title: format!("PR #{number}"),
                state: PullRequestState::Merged,
                created_at: Utc::now(),
                merged_at: Some(Utc::now()),
                additions: Some(1),
                deletions: Some(0),
                changed_files: Some(1),
                touched_paths_hint: vec![],
                window: None,
            }),
            EventKind::Review => EventPayload::Review(ReviewEvent {
                pull_number: number,
                pull_title: format!("Review #{number}"),
                submitted_at: Utc::now(),
                state: "approved".into(),
                window: None,
            }),
            EventKind::Manual => EventPayload::Manual(ManualEvent {
                event_type: ManualEventType::Note,
                title: format!("Manual #{number}"),
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

// ===========================================================================
// Many repos edge cases
// ===========================================================================

#[test]
fn hundred_repos_each_get_own_workstream() {
    let events: Vec<_> = (0..100)
        .map(|i| {
            event(
                &format!("org/repo-{i:03}"),
                &format!("e{i}"),
                i,
                EventKind::PullRequest,
            )
        })
        .collect();

    let ws = RepoClusterer.cluster(&events).unwrap();
    assert_eq!(ws.workstreams.len(), 100);

    for w in &ws.workstreams {
        assert_eq!(w.events.len(), 1);
        assert_eq!(w.stats.pull_requests, 1);
    }
}

#[test]
fn hundred_events_single_repo() {
    let events: Vec<_> = (0..100)
        .map(|i| event("mega/repo", &format!("e{i}"), i, EventKind::PullRequest))
        .collect();

    let ws = RepoClusterer.cluster(&events).unwrap();
    assert_eq!(ws.workstreams.len(), 1);
    assert_eq!(ws.workstreams[0].events.len(), 100);
    assert_eq!(ws.workstreams[0].stats.pull_requests, 100);
    // Receipts capped by total limit
    assert_eq!(
        ws.workstreams[0].receipts.len(),
        WORKSTREAM_RECEIPT_LIMIT_TOTAL
    );
}

// ===========================================================================
// Special characters in repo names
// ===========================================================================

#[test]
fn repo_names_with_dots_and_hyphens() {
    let events = vec![
        event("org/my-project.js", "e1", 1, EventKind::PullRequest),
        event("org/my-project.js", "e2", 2, EventKind::PullRequest),
        event("org/another.py", "e3", 3, EventKind::PullRequest),
    ];

    let ws = RepoClusterer.cluster(&events).unwrap();
    assert_eq!(ws.workstreams.len(), 2);

    let titles: Vec<&str> = ws.workstreams.iter().map(|w| w.title.as_str()).collect();
    assert!(titles.contains(&"org/another.py"));
    assert!(titles.contains(&"org/my-project.js"));
}

#[test]
fn repo_names_with_underscores_and_numbers() {
    let events = vec![
        event("user_123/repo_456", "e1", 1, EventKind::PullRequest),
        event("user_123/repo_789", "e2", 2, EventKind::Review),
    ];

    let ws = RepoClusterer.cluster(&events).unwrap();
    assert_eq!(ws.workstreams.len(), 2);

    let titles: Vec<&str> = ws.workstreams.iter().map(|w| w.title.as_str()).collect();
    assert!(titles.contains(&"user_123/repo_456"));
    assert!(titles.contains(&"user_123/repo_789"));
}

#[test]
fn unicode_repo_names_cluster_correctly() {
    let events = vec![
        event("组织/项目", "e1", 1, EventKind::PullRequest),
        event("组织/项目", "e2", 2, EventKind::Review),
        event("организация/проект", "e3", 3, EventKind::Manual),
    ];

    let ws = RepoClusterer.cluster(&events).unwrap();
    assert_eq!(ws.workstreams.len(), 2);

    let cn_ws = ws
        .workstreams
        .iter()
        .find(|w| w.title == "组织/项目")
        .unwrap();
    assert_eq!(cn_ws.events.len(), 2);

    let ru_ws = ws
        .workstreams
        .iter()
        .find(|w| w.title == "организация/проект")
        .unwrap();
    assert_eq!(ru_ws.events.len(), 1);
}

// ===========================================================================
// Sorting behavior
// ===========================================================================

#[test]
fn workstreams_sorted_by_btree_order() {
    let events = vec![
        event("zzz/last", "e1", 1, EventKind::PullRequest),
        event("aaa/first", "e2", 2, EventKind::PullRequest),
        event("mmm/middle", "e3", 3, EventKind::PullRequest),
        event("AAA/upper", "e4", 4, EventKind::PullRequest),
    ];

    let ws = RepoClusterer.cluster(&events).unwrap();
    let titles: Vec<&str> = ws.workstreams.iter().map(|w| w.title.as_str()).collect();

    // BTreeMap ordering: uppercase < lowercase in ASCII
    assert_eq!(
        titles,
        vec!["AAA/upper", "aaa/first", "mmm/middle", "zzz/last"]
    );
}

#[test]
fn sorting_is_stable_across_repeated_calls() {
    let events = vec![
        event("org/charlie", "e1", 1, EventKind::PullRequest),
        event("org/alpha", "e2", 2, EventKind::PullRequest),
        event("org/bravo", "e3", 3, EventKind::PullRequest),
    ];

    let titles_1: Vec<String> = RepoClusterer
        .cluster(&events)
        .unwrap()
        .workstreams
        .iter()
        .map(|w| w.title.clone())
        .collect();

    let titles_2: Vec<String> = RepoClusterer
        .cluster(&events)
        .unwrap()
        .workstreams
        .iter()
        .map(|w| w.title.clone())
        .collect();

    assert_eq!(titles_1, titles_2);
    assert_eq!(titles_1, vec!["org/alpha", "org/bravo", "org/charlie"]);
}

// ===========================================================================
// Custom clusterer strategy
// ===========================================================================

struct SingleBucketClusterer;

impl WorkstreamClusterer for SingleBucketClusterer {
    fn cluster(&self, events: &[EventEnvelope]) -> anyhow::Result<WorkstreamsFile> {
        let mut ws = Workstream {
            id: shiplog_ids::WorkstreamId::from_parts(["all"]),
            title: "All Work".into(),
            summary: Some("Everything in one bucket".into()),
            tags: vec!["custom".into()],
            stats: WorkstreamStats::zero(),
            events: vec![],
            receipts: vec![],
        };

        for ev in events {
            ws.events.push(ev.id.clone());
            ws.bump_stats(&ev.kind);
        }

        Ok(WorkstreamsFile {
            version: 1,
            generated_at: Utc::now(),
            workstreams: if events.is_empty() { vec![] } else { vec![ws] },
        })
    }
}

#[test]
fn custom_clusterer_puts_all_events_in_single_workstream() {
    let events = vec![
        event("org/a", "e1", 1, EventKind::PullRequest),
        event("org/b", "e2", 2, EventKind::Review),
        event("org/c", "e3", 3, EventKind::Manual),
    ];

    let ws = SingleBucketClusterer.cluster(&events).unwrap();
    assert_eq!(ws.workstreams.len(), 1);
    assert_eq!(ws.workstreams[0].events.len(), 3);
    assert_eq!(ws.workstreams[0].title, "All Work");
    assert_eq!(ws.workstreams[0].tags, vec!["custom"]);
}

#[test]
fn custom_clusterer_with_empty_events() {
    let ws = SingleBucketClusterer.cluster(&[]).unwrap();
    assert!(ws.workstreams.is_empty());
}

#[test]
fn custom_clusterer_is_trait_object_safe() {
    let clusterer: Box<dyn WorkstreamClusterer> = Box::new(SingleBucketClusterer);
    let events = vec![event("org/a", "e1", 1, EventKind::PullRequest)];
    let ws = clusterer.cluster(&events).unwrap();
    assert_eq!(ws.workstreams.len(), 1);
}

// ===========================================================================
// Stats per workstream
// ===========================================================================

#[test]
fn only_reviews_in_repo_gives_zero_prs_and_manual() {
    let events: Vec<_> = (0..5)
        .map(|i| event("org/reviews-only", &format!("r{i}"), i, EventKind::Review))
        .collect();

    let ws = RepoClusterer.cluster(&events).unwrap();
    assert_eq!(ws.workstreams.len(), 1);
    assert_eq!(ws.workstreams[0].stats.reviews, 5);
    assert_eq!(ws.workstreams[0].stats.pull_requests, 0);
    assert_eq!(ws.workstreams[0].stats.manual_events, 0);
}

#[test]
fn only_manual_events_in_repo_gives_zero_prs_and_reviews() {
    let events: Vec<_> = (0..3)
        .map(|i| event("org/manual-only", &format!("m{i}"), i, EventKind::Manual))
        .collect();

    let ws = RepoClusterer.cluster(&events).unwrap();
    assert_eq!(ws.workstreams.len(), 1);
    assert_eq!(ws.workstreams[0].stats.manual_events, 3);
    assert_eq!(ws.workstreams[0].stats.pull_requests, 0);
    assert_eq!(ws.workstreams[0].stats.reviews, 0);
}

// ===========================================================================
// Workstream ID determinism with edge-case repo names
// ===========================================================================

#[test]
fn workstream_ids_differ_for_similar_repo_names() {
    let events = vec![
        event("org/repo", "e1", 1, EventKind::PullRequest),
        event("org/repo-2", "e2", 2, EventKind::PullRequest),
        event("org/repo-20", "e3", 3, EventKind::PullRequest),
    ];

    let ws = RepoClusterer.cluster(&events).unwrap();
    let ids: std::collections::HashSet<String> =
        ws.workstreams.iter().map(|w| w.id.to_string()).collect();

    assert_eq!(
        ids.len(),
        3,
        "similar repo names must produce different IDs"
    );
}

#[test]
fn workstream_id_stable_regardless_of_event_order() {
    let events_a = vec![
        event("org/repo", "e1", 1, EventKind::PullRequest),
        event("org/repo", "e2", 2, EventKind::Review),
    ];
    let events_b = vec![
        event("org/repo", "e2", 2, EventKind::Review),
        event("org/repo", "e1", 1, EventKind::PullRequest),
    ];

    let ws_a = RepoClusterer.cluster(&events_a).unwrap();
    let ws_b = RepoClusterer.cluster(&events_b).unwrap();

    // IDs are based on repo name, not event order
    assert_eq!(
        ws_a.workstreams[0].id.to_string(),
        ws_b.workstreams[0].id.to_string()
    );
}
