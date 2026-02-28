//! Edge-case tests for shiplog-workstreams.
//!
//! Covers YAML override parsing with special characters, merging suggested and
//! curated workstreams, stats calculation accuracy, empty workstreams, and
//! duplicate event assignments.

use chrono::Utc;
use shiplog_ids::{EventId, WorkstreamId};
use shiplog_ports::WorkstreamClusterer;
use shiplog_schema::event::*;
use shiplog_schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};
use shiplog_workstreams::{RepoClusterer, WorkstreamManager, write_workstreams};
use tempfile::tempdir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_event(repo: &str, id_suffix: &str, number: u64, kind: EventKind) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["edge", id_suffix]),
        kind: kind.clone(),
        occurred_at: Utc::now(),
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
                created_at: Utc::now(),
                merged_at: Some(Utc::now()),
                additions: Some(5),
                deletions: Some(1),
                changed_files: Some(2),
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

fn make_ws_file(title: &str, events: Vec<EventId>) -> WorkstreamsFile {
    WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![Workstream {
            id: WorkstreamId::from_parts(["edge", title]),
            title: title.to_string(),
            summary: Some("edge-case test".into()),
            tags: vec!["repo".into()],
            stats: WorkstreamStats::zero(),
            events,
            receipts: vec![],
        }],
    }
}

// ===========================================================================
// YAML override parsing with special characters
// ===========================================================================

#[test]
fn yaml_roundtrip_with_special_chars_in_title() {
    let dir = tempdir().unwrap();
    let ws = WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![Workstream {
            id: WorkstreamId::from_parts(["special"]),
            title: "Title with \"quotes\" & <angle> 'apostrophes' #hash @at".into(),
            summary: Some("Summary: colon, {braces}, [brackets]".into()),
            tags: vec!["tag:with:colons".into(), "日本語".into(), "emoji🚀".into()],
            stats: WorkstreamStats {
                pull_requests: 2,
                reviews: 1,
                manual_events: 0,
            },
            events: vec![],
            receipts: vec![],
        }],
    };

    let path = dir.path().join("special.yaml");
    write_workstreams(&path, &ws).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    let loaded: WorkstreamsFile = serde_yaml::from_str(&text).unwrap();

    assert_eq!(loaded.workstreams[0].title, ws.workstreams[0].title);
    assert_eq!(loaded.workstreams[0].summary, ws.workstreams[0].summary);
    assert_eq!(loaded.workstreams[0].tags, ws.workstreams[0].tags);
}

#[test]
fn yaml_roundtrip_with_very_long_workstream_name() {
    let dir = tempdir().unwrap();
    let long_name = "a".repeat(1000);
    let ws = make_ws_file(&long_name, vec![]);

    let path = dir.path().join("long.yaml");
    write_workstreams(&path, &ws).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    let loaded: WorkstreamsFile = serde_yaml::from_str(&text).unwrap();
    assert_eq!(loaded.workstreams[0].title, long_name);
}

#[test]
fn yaml_roundtrip_with_unicode_repo_names() {
    let dir = tempdir().unwrap();
    let ws = WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![
            Workstream {
                id: WorkstreamId::from_parts(["repo", "组织/项目"]),
                title: "组织/项目".into(),
                summary: None,
                tags: vec!["repo".into()],
                stats: WorkstreamStats::zero(),
                events: vec![],
                receipts: vec![],
            },
            Workstream {
                id: WorkstreamId::from_parts(["repo", "орг/проект"]),
                title: "орг/проект".into(),
                summary: None,
                tags: vec!["repo".into()],
                stats: WorkstreamStats::zero(),
                events: vec![],
                receipts: vec![],
            },
        ],
    };

    let path = dir.path().join("unicode.yaml");
    write_workstreams(&path, &ws).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    let loaded: WorkstreamsFile = serde_yaml::from_str(&text).unwrap();
    assert_eq!(loaded.workstreams.len(), 2);
    assert_eq!(loaded.workstreams[0].title, "组织/项目");
    assert_eq!(loaded.workstreams[1].title, "орг/проект");
}

// ===========================================================================
// Merging suggested and curated workstreams
// ===========================================================================

#[test]
fn curated_completely_replaces_suggested_content() {
    let dir = tempdir().unwrap();

    let suggested = WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![
            Workstream {
                id: WorkstreamId::from_parts(["repo", "a"]),
                title: "suggested-a".into(),
                summary: None,
                tags: vec!["repo".into()],
                stats: WorkstreamStats {
                    pull_requests: 5,
                    reviews: 0,
                    manual_events: 0,
                },
                events: vec![EventId::from_parts(["e", "1"])],
                receipts: vec![],
            },
            Workstream {
                id: WorkstreamId::from_parts(["repo", "b"]),
                title: "suggested-b".into(),
                summary: None,
                tags: vec!["repo".into()],
                stats: WorkstreamStats::zero(),
                events: vec![],
                receipts: vec![],
            },
        ],
    };

    // Curated has different titles and only one workstream
    let curated = WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![Workstream {
            id: WorkstreamId::from_parts(["custom", "merged"]),
            title: "Auth Overhaul".into(),
            summary: Some("Combined auth work".into()),
            tags: vec!["custom".into()],
            stats: WorkstreamStats {
                pull_requests: 10,
                reviews: 3,
                manual_events: 1,
            },
            events: vec![EventId::from_parts(["e", "1"])],
            receipts: vec![],
        }],
    };

    write_workstreams(&WorkstreamManager::suggested_path(dir.path()), &suggested).unwrap();
    write_workstreams(&WorkstreamManager::curated_path(dir.path()), &curated).unwrap();

    let loaded = WorkstreamManager::load_effective(dir.path(), &RepoClusterer, &[]).unwrap();

    // Curated entirely replaces suggested — only 1 workstream, with curated data
    assert_eq!(loaded.workstreams.len(), 1);
    assert_eq!(loaded.workstreams[0].title, "Auth Overhaul");
    assert_eq!(
        loaded.workstreams[0].summary,
        Some("Combined auth work".into())
    );
    assert_eq!(loaded.workstreams[0].tags, vec!["custom"]);
}

#[test]
fn suggested_is_preserved_when_curated_absent() {
    let dir = tempdir().unwrap();

    let suggested = WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![
            Workstream {
                id: WorkstreamId::from_parts(["repo", "a"]),
                title: "suggested-a".into(),
                summary: None,
                tags: vec!["repo".into()],
                stats: WorkstreamStats::zero(),
                events: vec![],
                receipts: vec![],
            },
            Workstream {
                id: WorkstreamId::from_parts(["repo", "b"]),
                title: "suggested-b".into(),
                summary: None,
                tags: vec!["repo".into()],
                stats: WorkstreamStats::zero(),
                events: vec![],
                receipts: vec![],
            },
        ],
    };

    write_workstreams(&WorkstreamManager::suggested_path(dir.path()), &suggested).unwrap();

    let loaded = WorkstreamManager::load_effective(dir.path(), &RepoClusterer, &[]).unwrap();
    assert_eq!(loaded.workstreams.len(), 2);
    assert_eq!(loaded.workstreams[0].title, "suggested-a");
    assert_eq!(loaded.workstreams[1].title, "suggested-b");
}

// ===========================================================================
// Workstream stats calculation accuracy
// ===========================================================================

#[test]
fn stats_accumulate_correctly_across_kinds() {
    let events = vec![
        make_event("org/stats", "pr1", 1, EventKind::PullRequest),
        make_event("org/stats", "pr2", 2, EventKind::PullRequest),
        make_event("org/stats", "pr3", 3, EventKind::PullRequest),
        make_event("org/stats", "rev1", 10, EventKind::Review),
        make_event("org/stats", "rev2", 11, EventKind::Review),
        make_event("org/stats", "man1", 20, EventKind::Manual),
    ];

    let ws = RepoClusterer.cluster(&events).unwrap();
    assert_eq!(ws.workstreams.len(), 1);

    let stats = &ws.workstreams[0].stats;
    assert_eq!(stats.pull_requests, 3);
    assert_eq!(stats.reviews, 2);
    assert_eq!(stats.manual_events, 1);

    // Total events matches sum of stats
    let stat_total = stats.pull_requests + stats.reviews + stats.manual_events;
    assert_eq!(stat_total, ws.workstreams[0].events.len());
}

#[test]
fn stats_correct_across_multiple_repos() {
    let events = vec![
        make_event("org/alpha", "a1", 1, EventKind::PullRequest),
        make_event("org/alpha", "a2", 2, EventKind::Review),
        make_event("org/beta", "b1", 3, EventKind::Manual),
        make_event("org/beta", "b2", 4, EventKind::Manual),
        make_event("org/beta", "b3", 5, EventKind::PullRequest),
    ];

    let ws = RepoClusterer.cluster(&events).unwrap();
    assert_eq!(ws.workstreams.len(), 2);

    let alpha = ws
        .workstreams
        .iter()
        .find(|w| w.title == "org/alpha")
        .unwrap();
    assert_eq!(alpha.stats.pull_requests, 1);
    assert_eq!(alpha.stats.reviews, 1);
    assert_eq!(alpha.stats.manual_events, 0);

    let beta = ws
        .workstreams
        .iter()
        .find(|w| w.title == "org/beta")
        .unwrap();
    assert_eq!(beta.stats.pull_requests, 1);
    assert_eq!(beta.stats.reviews, 0);
    assert_eq!(beta.stats.manual_events, 2);
}

#[test]
fn stats_zero_for_empty_workstream_from_yaml() {
    let dir = tempdir().unwrap();
    let ws = WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![Workstream {
            id: WorkstreamId::from_parts(["empty"]),
            title: "empty-ws".into(),
            summary: None,
            tags: vec![],
            stats: WorkstreamStats::zero(),
            events: vec![],
            receipts: vec![],
        }],
    };

    let path = dir.path().join("zero-stats.yaml");
    write_workstreams(&path, &ws).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    let loaded: WorkstreamsFile = serde_yaml::from_str(&text).unwrap();
    assert_eq!(loaded.workstreams[0].stats.pull_requests, 0);
    assert_eq!(loaded.workstreams[0].stats.reviews, 0);
    assert_eq!(loaded.workstreams[0].stats.manual_events, 0);
}

// ===========================================================================
// Empty workstreams handling
// ===========================================================================

#[test]
fn empty_events_produce_empty_file_through_manager() {
    let dir = tempdir().unwrap();
    let loaded = WorkstreamManager::load_effective(dir.path(), &RepoClusterer, &[]).unwrap();

    assert!(loaded.workstreams.is_empty());
    assert_eq!(loaded.version, 1);
    // Suggested file should still be written
    assert!(WorkstreamManager::suggested_path(dir.path()).exists());
}

#[test]
fn try_load_returns_empty_workstreams_list_from_file() {
    let dir = tempdir().unwrap();
    let ws = WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![],
    };
    write_workstreams(&WorkstreamManager::curated_path(dir.path()), &ws).unwrap();

    let loaded = WorkstreamManager::try_load(dir.path()).unwrap().unwrap();
    assert!(loaded.workstreams.is_empty());
    assert_eq!(loaded.version, 1);
}

#[test]
fn write_and_read_empty_workstreams_file() {
    let dir = tempdir().unwrap();
    let ws = WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![],
    };

    WorkstreamManager::write_suggested(dir.path(), &ws).unwrap();
    let loaded = WorkstreamManager::try_load(dir.path()).unwrap().unwrap();
    assert!(loaded.workstreams.is_empty());
}

// ===========================================================================
// Duplicate event assignments
// ===========================================================================

#[test]
fn same_repo_events_share_single_workstream() {
    let events = vec![
        make_event("org/shared", "e1", 1, EventKind::PullRequest),
        make_event("org/shared", "e2", 2, EventKind::PullRequest),
        make_event("org/shared", "e3", 3, EventKind::Review),
        make_event("org/shared", "e4", 4, EventKind::Manual),
    ];

    let ws = RepoClusterer.cluster(&events).unwrap();
    assert_eq!(ws.workstreams.len(), 1);
    assert_eq!(ws.workstreams[0].events.len(), 4);
}

#[test]
fn no_event_appears_in_multiple_workstreams() {
    let events = vec![
        make_event("org/a", "1", 1, EventKind::PullRequest),
        make_event("org/b", "2", 2, EventKind::PullRequest),
        make_event("org/a", "3", 3, EventKind::Review),
        make_event("org/c", "4", 4, EventKind::Manual),
        make_event("org/b", "5", 5, EventKind::Review),
    ];

    let ws = RepoClusterer.cluster(&events).unwrap();
    let all_event_ids: Vec<String> = ws
        .workstreams
        .iter()
        .flat_map(|w| w.events.iter().map(|id| id.to_string()))
        .collect();

    let unique: std::collections::HashSet<&String> = all_event_ids.iter().collect();
    assert_eq!(
        all_event_ids.len(),
        unique.len(),
        "no event should appear in multiple workstreams"
    );
    assert_eq!(all_event_ids.len(), 5);
}

#[test]
fn every_input_event_is_assigned() {
    let events: Vec<_> = (0..100)
        .map(|i| {
            make_event(
                &format!("org/repo-{}", i % 10),
                &format!("e{i}"),
                i,
                EventKind::PullRequest,
            )
        })
        .collect();

    let ws = RepoClusterer.cluster(&events).unwrap();
    let total_assigned: usize = ws.workstreams.iter().map(|w| w.events.len()).sum();
    assert_eq!(total_assigned, 100);
    assert_eq!(ws.workstreams.len(), 10);
}

// ===========================================================================
// Curated YAML with many workstreams
// ===========================================================================

#[test]
fn curated_yaml_with_many_workstreams_roundtrips() {
    let dir = tempdir().unwrap();
    let workstreams: Vec<Workstream> = (0..50)
        .map(|i| Workstream {
            id: WorkstreamId::from_parts(["ws", &format!("w{i}")]),
            title: format!("Workstream #{i}"),
            summary: if i % 2 == 0 {
                Some(format!("Summary for {i}"))
            } else {
                None
            },
            tags: vec!["repo".into()],
            stats: WorkstreamStats {
                pull_requests: i,
                reviews: i / 2,
                manual_events: i % 3,
            },
            events: vec![EventId::from_parts(["e", &format!("{i}")])],
            receipts: vec![],
        })
        .collect();

    let ws = WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams,
    };

    write_workstreams(&WorkstreamManager::curated_path(dir.path()), &ws).unwrap();
    let loaded = WorkstreamManager::try_load(dir.path()).unwrap().unwrap();
    assert_eq!(loaded.workstreams.len(), 50);

    for (original, loaded) in ws.workstreams.iter().zip(loaded.workstreams.iter()) {
        assert_eq!(original.title, loaded.title);
        assert_eq!(original.stats, loaded.stats);
    }
}
