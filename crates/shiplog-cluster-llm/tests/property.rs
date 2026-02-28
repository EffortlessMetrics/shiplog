use proptest::prelude::*;
use shiplog_cluster_llm::{LlmClusterer, LlmConfig, MockLlmBackend};
use shiplog_cluster_llm_parse::parse_llm_response;
use shiplog_cluster_llm_prompt::{chunk_events, format_event_list, summarize_event, system_prompt};
use shiplog_ids::EventId;
use shiplog_ports::WorkstreamClusterer;
use shiplog_schema::event::*;
use std::collections::HashSet;

fn make_pr_event(num: u64, title: &str) -> EventEnvelope {
    use chrono::Utc;
    EventEnvelope {
        id: EventId::from_parts(["prop", "clllm", &num.to_string()]),
        kind: EventKind::PullRequest,
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
        payload: EventPayload::PullRequest(PullRequestEvent {
            number: num,
            title: title.into(),
            state: PullRequestState::Merged,
            created_at: Utc::now(),
            merged_at: Some(Utc::now()),
            additions: Some(10),
            deletions: Some(5),
            changed_files: Some(3),
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

// ---------------------------------------------------------------------------
// Property: summarize_event never panics for arbitrary titles
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn prop_summarize_event_never_panics(
        title in ".*",
        num in 0u64..10_000,
    ) {
        let ev = make_pr_event(num, &title);
        let _ = summarize_event(&ev);
    }
}

// ---------------------------------------------------------------------------
// Property: format_event_list never panics for arbitrary event counts
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn prop_format_event_list_never_panics(
        titles in prop::collection::vec(".{0,30}", 0..100usize),
    ) {
        let events: Vec<EventEnvelope> = titles
            .into_iter()
            .enumerate()
            .map(|(i, t)| make_pr_event(i as u64, &t))
            .collect();
        let _ = format_event_list(&events);
    }
}

// ---------------------------------------------------------------------------
// Property: system_prompt never panics for arbitrary limits
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn prop_system_prompt_never_panics(limit in prop::option::of(0usize..1000)) {
        let _ = system_prompt(limit);
    }
}

// ---------------------------------------------------------------------------
// Property: chunk_events preserves all indices for arbitrary inputs
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn prop_chunk_events_covers_all_indices(
        count in 0usize..100,
        max_tokens in 1usize..10_000,
    ) {
        let events: Vec<EventEnvelope> = (0..count)
            .map(|i| make_pr_event(i as u64, "event"))
            .collect();
        let chunks = chunk_events(&events, max_tokens);
        let flat: Vec<usize> = chunks.into_iter().flatten().collect();
        let expected: Vec<usize> = (0..count).collect();
        prop_assert_eq!(flat, expected);
    }
}

// ---------------------------------------------------------------------------
// Property: parse_llm_response handles arbitrary strings gracefully
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn prop_parse_handles_arbitrary_strings(input in ".*") {
        let events = vec![make_pr_event(1, "Test")];
        // Should either succeed or return Err, never panic
        let _ = parse_llm_response(&input, &events);
    }
}

// ---------------------------------------------------------------------------
// Property: parse always covers every event exactly once
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn prop_parse_covers_all_events_exactly_once(
        event_count in 1usize..50,
        ws_specs in prop::collection::vec(
            (
                "[a-zA-Z ]{1,20}",
                prop::collection::vec(0usize..100, 0..8),
            ),
            0..6,
        ),
    ) {
        let events: Vec<EventEnvelope> = (0..event_count)
            .map(|i| make_pr_event(i as u64, &format!("PR {i}")))
            .collect();

        let workstreams: Vec<_> = ws_specs
            .into_iter()
            .enumerate()
            .map(|(i, (title, indices))| {
                serde_json::json!({
                    "title": format!("{title}_{i}"),
                    "event_indices": indices,
                    "receipt_indices": []
                })
            })
            .collect();

        let payload = serde_json::json!({ "workstreams": workstreams }).to_string();
        let result = parse_llm_response(&payload, &events).unwrap();

        let all_ids: Vec<String> = result
            .workstreams
            .iter()
            .flat_map(|ws| ws.events.iter().map(|id| id.to_string()))
            .collect();

        let unique: HashSet<String> = all_ids.iter().cloned().collect();
        let expected: HashSet<String> = events.iter().map(|e| e.id.to_string()).collect();

        prop_assert_eq!(all_ids.len(), event_count, "every event claimed exactly once");
        prop_assert_eq!(unique, expected, "exact set of event IDs matches");
    }
}

// ---------------------------------------------------------------------------
// Property: LlmClusterer with mock never panics
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn prop_llm_clusterer_with_mock_never_panics(
        event_count in 0usize..30,
        response in ".*",
    ) {
        let events: Vec<EventEnvelope> = (0..event_count)
            .map(|i| make_pr_event(i as u64, "Test"))
            .collect();

        let clusterer = LlmClusterer::new(
            Box::new(MockLlmBackend { response }),
            LlmConfig::default(),
        );

        // Should not panic regardless of input
        let _ = clusterer.cluster(&events);
    }
}
