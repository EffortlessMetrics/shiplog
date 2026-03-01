use proptest::prelude::*;
use shiplog_cluster_llm_parse::parse_llm_response;
use shiplog_ids::EventId;
use shiplog_schema::event::*;

fn make_event(num: u64) -> EventEnvelope {
    use chrono::Utc;
    EventEnvelope {
        id: EventId::from_parts(["prop", "cluster", &num.to_string()]),
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
            title: format!("PR {num}"),
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
    fn prop_parser_covers_all_events_once(
        event_count in 0usize..64,
        workstream_specs in prop::collection::vec(
            (
                "[a-zA-Z ]{0,24}",
                prop::option::of("[a-zA-Z ]{0,40}"),
                prop::collection::vec("[a-z]{0,10}", 0..4),
                prop::collection::vec(0usize..128, 0..8),
                prop::collection::vec(0usize..128, 0..8),
            ),
            0..8,
        ),
    ) {
        let events: Vec<EventEnvelope> = (0..event_count).map(|i| make_event(i as u64)).collect();
        let workstreams: Vec<_> = workstream_specs
            .into_iter()
            .enumerate()
            .map(|(index, (title, summary, tags, event_indices, receipt_indices))| {
                serde_json::json!({
                    "title": format!("{}:{}", title, index),
                    "summary": summary.unwrap_or_else(|| "".to_string()),
                    "tags": tags,
                    "event_indices": event_indices,
                    "receipt_indices": receipt_indices,
                })
            })
            .collect();

        let payload = serde_json::json!({ "workstreams": workstreams }).to_string();
        let parsed = parse_llm_response(&payload, &events).unwrap();
        let parsed_ids: Vec<String> = parsed
            .workstreams
            .iter()
            .flat_map(|ws| ws.events.iter().map(|id| id.to_string()))
            .collect();

        let parsed_unique: std::collections::HashSet<String> = parsed_ids.iter().cloned().collect();
        let expected_ids: std::collections::HashSet<String> =
            events.iter().map(|ev| ev.id.to_string()).collect();

        prop_assert_eq!(parsed_ids.len(), event_count);
        prop_assert_eq!(parsed_unique.len(), event_count);
        prop_assert_eq!(parsed_unique, expected_ids);
    }

    #[test]
    fn prop_invalid_indices_can_only_create_uncategorized_workstream(
        event_count in 1usize..50,
        invalid_run in 0usize..5,
    ) {
        let events: Vec<EventEnvelope> = (0..event_count).map(|i| make_event(i as u64)).collect();
        let invalid_indices: Vec<usize> = (0..invalid_run)
            .map(|i| event_count + i)
            .collect();

        let payload = serde_json::json!({
            "workstreams": [{
                "title": "Invalid",
                "summary": "invalid indices",
                "tags": [],
                "event_indices": invalid_indices,
                "receipt_indices": [0, 1, 2]
            }]
        })
        .to_string();

        let parsed = parse_llm_response(&payload, &events).unwrap();
        let first = &parsed.workstreams[0];
        prop_assert_eq!(parsed.workstreams.len(), 1);
        prop_assert_eq!(&first.title, "Uncategorized");
        prop_assert_eq!(first.events.len(), event_count);
        prop_assert_eq!(first.receipts.len(), std::cmp::min(event_count, 10));
        prop_assert_eq!(&first.tags, &vec!["uncategorized".to_string()]);
    }
}
