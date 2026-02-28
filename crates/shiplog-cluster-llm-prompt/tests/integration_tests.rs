use chrono::Utc;
use shiplog_cluster_llm_prompt::*;
use shiplog_ids::EventId;
use shiplog_schema::event::*;

fn make_test_event(repo: &str, num: u64, title: &str) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["test", "pr", repo, &num.to_string()]),
        kind: EventKind::PullRequest,
        occurred_at: Utc::now(),
        actor: Actor {
            login: "user".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: repo.into(),
            html_url: None,
            visibility: RepoVisibility::Unknown,
        },
        payload: EventPayload::PullRequest(PullRequestEvent {
            number: num,
            title: title.into(),
            state: PullRequestState::Merged,
            created_at: Utc::now(),
            merged_at: Some(Utc::now()),
            additions: Some(4),
            deletions: Some(1),
            changed_files: Some(2),
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

#[test]
fn prompt_formatting_and_chunking_work_together() {
    let events = vec![
        make_test_event("org/api", 1, "Add auth endpoint"),
        make_test_event("org/api", 2, "Fix auth bug"),
        make_test_event("org/ui", 3, "Update docs"),
    ];

    let event_list = format_event_list(&events);
    assert!(event_list.contains("[0]"));
    assert!(event_list.contains("Add auth endpoint"));
    assert!(event_list.contains("Fix auth bug"));
    assert!(event_list.contains("Update docs"));

    let chunks = chunk_events(&events, 500);
    assert_eq!(chunks.len(), 1);

    let prompt = system_prompt(Some(2));
    assert!(prompt.contains("Create at most 2 workstreams."));
    assert!(system_prompt(None).contains("workstream"));

    let summary = summarize_event(&events[0]);
    assert!(summary.contains("PR#1"));
    assert!(summary.contains("org/api"));
}

#[test]
fn chunking_preserves_index_order_with_many_events() {
    let events: Vec<EventEnvelope> = (0..40)
        .map(|idx| make_test_event("org/api", idx, &format!("Event {idx}")))
        .collect();

    let chunks = chunk_events(&events, 80); // force multiple chunks
    let flattened: Vec<usize> = chunks
        .iter()
        .flat_map(|chunk| chunk.iter().copied())
        .collect();
    let expected: Vec<usize> = (0..events.len()).collect();

    assert_eq!(flattened.len(), events.len());
    assert_eq!(flattened, expected);
}

fn make_review_event(repo: &str, pr_num: u64, title: &str) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["test", "review", repo, &pr_num.to_string()]),
        kind: EventKind::Review,
        occurred_at: Utc::now(),
        actor: Actor {
            login: "reviewer".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: repo.into(),
            html_url: None,
            visibility: RepoVisibility::Unknown,
        },
        payload: EventPayload::Review(ReviewEvent {
            pull_number: pr_num,
            pull_title: title.into(),
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

fn make_manual_event(title: &str) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["test", "manual", title]),
        kind: EventKind::Manual,
        occurred_at: Utc::now(),
        actor: Actor {
            login: "user".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: "org/repo".into(),
            html_url: None,
            visibility: RepoVisibility::Unknown,
        },
        payload: EventPayload::Manual(ManualEvent {
            event_type: ManualEventType::Note,
            title: title.into(),
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
fn summarize_review_event_contains_pr_number_and_state() {
    let ev = make_review_event("org/api", 42, "Fix auth");
    let summary = summarize_event(&ev);
    assert!(summary.contains("Review on PR#42"));
    assert!(summary.contains("org/api"));
    assert!(summary.contains("Fix auth"));
    assert!(summary.contains("approved"));
}

#[test]
fn summarize_manual_event_contains_type_and_title() {
    let ev = make_manual_event("Outage response");
    let summary = summarize_event(&ev);
    assert!(summary.contains("Note"));
    assert!(summary.contains("Outage response"));
}

#[test]
fn format_event_list_empty_returns_empty_string() {
    let result = format_event_list(&[]);
    assert_eq!(result, "");
}

#[test]
fn format_event_list_mixed_event_types() {
    let events = vec![
        make_test_event("org/api", 1, "Add feature"),
        make_review_event("org/api", 1, "Add feature"),
        make_manual_event("Design doc"),
    ];
    let result = format_event_list(&events);
    let lines: Vec<&str> = result.lines().collect();
    assert_eq!(lines.len(), 3);
    assert!(lines[0].starts_with("[0]"));
    assert!(lines[1].starts_with("[1]"));
    assert!(lines[2].starts_with("[2]"));
    assert!(lines[0].contains("PR#1"));
    assert!(lines[1].contains("Review on PR#1"));
    assert!(lines[2].contains("Note"));
}

#[test]
fn system_prompt_with_limit_1() {
    let prompt = system_prompt(Some(1));
    assert!(prompt.contains("at most 1 workstreams"));
}

#[test]
fn system_prompt_contains_json_structure() {
    let prompt = system_prompt(None);
    assert!(prompt.contains("event_indices"));
    assert!(prompt.contains("receipt_indices"));
    assert!(prompt.contains("tags"));
    assert!(prompt.contains("JSON"));
}

#[test]
fn chunk_events_zero_budget_one_event_per_chunk() {
    let events: Vec<EventEnvelope> = (0..5)
        .map(|i| make_test_event("org/repo", i, &format!("Event {i}")))
        .collect();
    let chunks = chunk_events(&events, 0);
    assert_eq!(chunks.len(), events.len());
    for chunk in &chunks {
        assert_eq!(chunk.len(), 1);
    }
}

#[test]
fn summarize_pr_event_includes_date() {
    let ev = make_test_event("org/repo", 1, "Test PR");
    let summary = summarize_event(&ev);
    let today = Utc::now().format("%Y-%m-%d").to_string();
    assert!(summary.contains(&today));
}

#[test]
fn chunk_events_empty_input_returns_empty() {
    let chunks = chunk_events(&[], 1000);
    assert!(chunks.is_empty());
}
