use chrono::Utc;
use shiplog_cluster_llm_parse::parse_llm_response;
use shiplog_ids::EventId;
use shiplog_schema::event::*;

fn make_event(id: &str, num: u64, kind: &str) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["it", id, &num.to_string()]),
        kind: match kind {
            "review" => EventKind::Review,
            "manual" => EventKind::Manual,
            _ => EventKind::PullRequest,
        },
        occurred_at: Utc::now(),
        actor: Actor {
            login: "integration".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: format!("org/{id}"),
            html_url: None,
            visibility: RepoVisibility::Unknown,
        },
        payload: match kind {
            "review" => EventPayload::Review(ReviewEvent {
                pull_number: num,
                pull_title: "review".into(),
                submitted_at: Utc::now(),
                state: "approved".into(),
                window: None,
            }),
            "manual" => EventPayload::Manual(ManualEvent {
                event_type: ManualEventType::Note,
                title: "manual event".into(),
                description: None,
                started_at: None,
                ended_at: None,
                impact: None,
            }),
            _ => EventPayload::PullRequest(PullRequestEvent {
                number: num,
                title: format!("PR {num}"),
                state: PullRequestState::Merged,
                created_at: Utc::now(),
                merged_at: Some(Utc::now()),
                additions: Some(1),
                deletions: Some(2),
                changed_files: Some(3),
                touched_paths_hint: vec![],
                window: None,
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
fn parse_integration_with_mixed_event_types() {
    let events = vec![
        make_event("acme", 1, "review"),
        make_event("acme", 2, "manual"),
        make_event("acme", 3, "pr"),
        make_event("acme", 4, "pr"),
    ];

    let payload = serde_json::json!({
        "workstreams": [{
            "title": "Auth",
            "summary": "Auth work",
            "tags": ["auth", "integration"],
            "event_indices": [0, 2],
            "receipt_indices": [0]
        }]
    })
    .to_string();

    let parsed = parse_llm_response(&payload, &events).unwrap();

    assert_eq!(parsed.workstreams.len(), 2);
    assert_eq!(parsed.workstreams[0].title, "Auth");
    assert_eq!(parsed.workstreams[0].events.len(), 2);
    assert_eq!(parsed.workstreams[0].stats.reviews, 1);
    assert_eq!(parsed.workstreams[1].title, "Uncategorized");
    assert_eq!(parsed.workstreams[1].events.len(), 2);
}

#[test]
fn parse_integration_rejects_invalid_indices_and_caps_uncategorized_receipts() {
    let events: Vec<EventEnvelope> = (0..15).map(|idx| make_event("acme", idx + 1, "pr")).collect();

    let payload = serde_json::json!({
        "workstreams": [{
            "title": "Invalid",
            "summary": "none",
            "tags": ["invalid"],
            "event_indices": [900, 901, 902],
            "receipt_indices": [900, 901, 902]
        }]
    })
    .to_string();

    let parsed = parse_llm_response(&payload, &events).unwrap();
    assert_eq!(parsed.workstreams.len(), 1);
    assert_eq!(parsed.workstreams[0].title, "Uncategorized");
    assert_eq!(parsed.workstreams[0].events.len(), 15);
    assert_eq!(parsed.workstreams[0].receipts.len(), 10);
}
