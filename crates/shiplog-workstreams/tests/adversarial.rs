//! Adversarial and edge-case tests for shiplog-workstreams clustering.

use chrono::Utc;
use shiplog_ids::EventId;
use shiplog_ports::WorkstreamClusterer;
use shiplog_schema::event::*;
use shiplog_workstreams::RepoClusterer;

fn make_pr_event(repo: &str, id_suffix: &str) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["test", id_suffix]),
        kind: EventKind::PullRequest,
        occurred_at: Utc::now(),
        actor: Actor { login: "user".into(), id: None },
        repo: RepoRef {
            full_name: repo.into(),
            html_url: None,
            visibility: RepoVisibility::Public,
        },
        payload: EventPayload::PullRequest(PullRequestEvent {
            number: 1,
            title: "test".into(),
            state: PullRequestState::Merged,
            created_at: Utc::now(),
            merged_at: Some(Utc::now()),
            additions: Some(1),
            deletions: Some(0),
            changed_files: Some(1),
            touched_paths_hint: vec![],
            window: None,
        }),
        tags: vec![],
        links: vec![],
        source: SourceRef {
            system: SourceSystem::Github,
            url: None,
            opaque_id: None,
        },
    }
}

fn make_review_event(repo: &str, id_suffix: &str) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["review", id_suffix]),
        kind: EventKind::Review,
        occurred_at: Utc::now(),
        actor: Actor { login: "reviewer".into(), id: None },
        repo: RepoRef {
            full_name: repo.into(),
            html_url: None,
            visibility: RepoVisibility::Public,
        },
        payload: EventPayload::Review(ReviewEvent {
            pull_number: 1,
            pull_title: "reviewed PR".into(),
            submitted_at: Utc::now(),
            state: "approved".into(),
            window: None,
        }),
        tags: vec![],
        links: vec![],
        source: SourceRef {
            system: SourceSystem::Github,
            url: None,
            opaque_id: None,
        },
    }
}

fn make_manual_event(repo: &str, id_suffix: &str) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["manual", id_suffix]),
        kind: EventKind::Manual,
        occurred_at: Utc::now(),
        actor: Actor { login: "author".into(), id: None },
        repo: RepoRef {
            full_name: repo.into(),
            html_url: None,
            visibility: RepoVisibility::Unknown,
        },
        payload: EventPayload::Manual(ManualEvent {
            event_type: ManualEventType::Note,
            title: "manual entry".into(),
            description: None,
            started_at: None,
            ended_at: None,
            impact: None,
        }),
        tags: vec![],
        links: vec![],
        source: SourceRef {
            system: SourceSystem::Manual,
            url: None,
            opaque_id: None,
        },
    }
}

#[test]
fn empty_repo_name_creates_workstream() {
    let events = vec![make_pr_event("", "e1")];
    let ws = RepoClusterer.cluster(&events).unwrap();
    assert_eq!(ws.workstreams.len(), 1);
    assert_eq!(ws.workstreams[0].title, "");
}

#[test]
fn whitespace_repo_names_are_distinct() {
    let events = vec![
        make_pr_event("  ", "e1"),
        make_pr_event(" ", "e2"),
        make_pr_event("", "e3"),
    ];
    let ws = RepoClusterer.cluster(&events).unwrap();
    assert_eq!(ws.workstreams.len(), 3);
}

#[test]
fn unicode_repo_names() {
    let events = vec![
        make_pr_event("\u{7D44}\u{7E54}/\u{30EA}\u{30DD}", "e1"),
        make_pr_event("\u{7EC4}\u{7EC7}/\u{4ED3}\u{5E93}", "e2"),
    ];
    let ws = RepoClusterer.cluster(&events).unwrap();
    assert_eq!(ws.workstreams.len(), 2);
}

#[test]
fn emoji_repo_names() {
    let events = vec![make_pr_event("\u{1F680}/\u{1F4AF}", "e1")];
    let ws = RepoClusterer.cluster(&events).unwrap();
    assert_eq!(ws.workstreams[0].title, "\u{1F680}/\u{1F4AF}");
}

#[test]
fn duplicate_event_ids_both_counted() {
    let e1 = make_pr_event("org/repo", "same");
    let e2 = make_pr_event("org/repo", "same");
    let ws = RepoClusterer.cluster(&[e1, e2]).unwrap();
    assert_eq!(ws.workstreams[0].events.len(), 2);
}

#[test]
fn mixed_event_kinds_in_one_repo() {
    let events = vec![
        make_pr_event("org/repo", "pr1"),
        make_review_event("org/repo", "rev1"),
        make_manual_event("org/repo", "man1"),
    ];
    let ws = RepoClusterer.cluster(&events).unwrap();
    assert_eq!(ws.workstreams.len(), 1);
    assert_eq!(ws.workstreams[0].events.len(), 3);
    assert_eq!(ws.workstreams[0].stats.pull_requests, 1);
    assert_eq!(ws.workstreams[0].stats.reviews, 1);
    assert_eq!(ws.workstreams[0].stats.manual_events, 1);
}

#[test]
fn very_long_repo_name() {
    let long = "a".repeat(10_000);
    let events = vec![make_pr_event(&long, "e1")];
    let ws = RepoClusterer.cluster(&events).unwrap();
    assert_eq!(ws.workstreams[0].title, long);
}

#[test]
fn stress_many_repos() {
    let events: Vec<_> = (0..1000)
        .map(|i| make_pr_event(&format!("org/repo-{i}"), &format!("e{i}")))
        .collect();
    let ws = RepoClusterer.cluster(&events).unwrap();
    assert_eq!(ws.workstreams.len(), 1000);
}

#[test]
fn special_chars_sort_correctly() {
    let events = vec![
        make_pr_event("z/repo", "e1"),
        make_pr_event("a/repo", "e2"),
        make_pr_event("A/repo", "e3"),
        make_pr_event("0/repo", "e4"),
        make_pr_event("_/repo", "e5"),
    ];
    let ws = RepoClusterer.cluster(&events).unwrap();
    let titles: Vec<_> = ws.workstreams.iter().map(|w| w.title.as_str()).collect();
    let mut sorted = titles.clone();
    sorted.sort();
    assert_eq!(titles, sorted);
}

#[test]
fn empty_yaml_file_is_error() {
    let result = serde_yaml::from_str::<shiplog_schema::workstream::WorkstreamsFile>("");
    assert!(result.is_err());
}

#[test]
fn wrong_structure_yaml_is_error() {
    let yaml = "key: value\nanother: 42\n";
    let result = serde_yaml::from_str::<shiplog_schema::workstream::WorkstreamsFile>(yaml);
    assert!(result.is_err());
}

#[test]
fn extra_yaml_fields_tolerated() {
    let ws = shiplog_schema::workstream::WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![],
    };
    let yaml = serde_yaml::to_string(&ws).unwrap();
    let yaml_with_extra = format!("{yaml}extra_field: surprise\n");
    let result =
        serde_yaml::from_str::<shiplog_schema::workstream::WorkstreamsFile>(&yaml_with_extra);
    assert!(result.is_ok(), "extra YAML fields should be tolerated");
}

#[test]
fn suggested_path_ends_with_suggested_yaml() {
    use shiplog_workstreams::WorkstreamManager;
    use std::path::Path;
    let p = WorkstreamManager::suggested_path(Path::new("/out/run_1"));
    assert!(p.to_string_lossy().contains("suggested"));
}
