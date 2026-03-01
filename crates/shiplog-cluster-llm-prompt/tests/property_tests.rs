use proptest::prelude::*;
use shiplog_cluster_llm_prompt::{chunk_events, format_event_list, summarize_event};
use shiplog_ids::EventId;
use shiplog_schema::event::*;

fn make_test_event(num: u64, title: &str) -> EventEnvelope {
    use chrono::Utc;
    EventEnvelope {
        id: EventId::from_parts(["prop", "cluster-llm-prompt", &num.to_string()]),
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
            title: title.to_string(),
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

proptest! {
    #[test]
    fn prop_format_event_list_contains_expected_indices(titles in prop::collection::vec(".{0,24}", 0..50usize)) {
        let events: Vec<EventEnvelope> = titles
            .into_iter()
            .enumerate()
            .map(|(i, title)| make_test_event(i as u64, &title))
            .collect();

        let rendered = format_event_list(&events);
        if events.is_empty() {
            prop_assert_eq!(rendered, "");
            return Ok(());
        }

        let lines: Vec<&str> = rendered.split('\n').collect();
        prop_assert_eq!(lines.len(), events.len());
        for (idx, event) in events.iter().enumerate() {
            let prefix = format!("[{idx}] ");
            prop_assert!(lines[idx].starts_with(&prefix));
            prop_assert!(rendered.contains(&summarize_event(event)));
        }
    }

    #[test]
    fn prop_chunk_events_preserves_order_and_coverage(
        titles in prop::collection::vec(".{0,24}", 0..200usize),
        max_tokens in 0usize..8_000,
    ) {
        let events: Vec<EventEnvelope> = titles
            .into_iter()
            .enumerate()
            .map(|(i, title)| make_test_event(i as u64, &title))
            .collect();

        let chunks = chunk_events(&events, max_tokens);
        let flattened: Vec<usize> = chunks.iter().flatten().copied().collect();
        let expected: Vec<usize> = (0..events.len()).collect();
        prop_assert_eq!(flattened, expected);
        if !events.is_empty() {
            for chunk in chunks {
                for &idx in &chunk {
                    prop_assert!(idx < events.len());
                }
            }
        }
    }
}
