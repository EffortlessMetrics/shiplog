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
    let flattened: Vec<usize> = chunks.iter().flat_map(|chunk| chunk.iter().copied()).collect();
    let expected: Vec<usize> = (0..events.len()).collect();

    assert_eq!(flattened.len(), events.len());
    assert_eq!(flattened, expected);
}
