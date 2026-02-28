use chrono::Utc;
use shiplog_cluster_llm::{
    FailingLlmBackend, LlmBackend, LlmClusterer, LlmConfig, LlmWithFallback, MockLlmBackend,
};
use shiplog_ids::EventId;
use shiplog_ports::WorkstreamClusterer;
use shiplog_schema::event::*;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_pr(repo: &str, num: u64, title: &str) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["it", repo, &num.to_string()]),
        kind: EventKind::PullRequest,
        occurred_at: Utc::now(),
        actor: Actor {
            login: "dev".into(),
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

fn make_review(repo: &str, num: u64) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["it", "review", repo, &num.to_string()]),
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
            pull_number: num,
            pull_title: format!("PR {num}"),
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

fn make_manual(num: u64) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["it", "manual", &num.to_string()]),
        kind: EventKind::Manual,
        occurred_at: Utc::now(),
        actor: Actor {
            login: "dev".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: "org/misc".into(),
            html_url: None,
            visibility: RepoVisibility::Unknown,
        },
        payload: EventPayload::Manual(ManualEvent {
            event_type: ManualEventType::Note,
            title: format!("Note {num}"),
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

fn mock_clusterer(response: &str) -> LlmClusterer {
    LlmClusterer::new(
        Box::new(MockLlmBackend {
            response: response.to_string(),
        }),
        LlmConfig::default(),
    )
}

// ---------------------------------------------------------------------------
// Mock LLM – single workstream
// ---------------------------------------------------------------------------

#[test]
fn single_workstream_assigns_all_events() {
    let events = vec![
        make_pr("org/api", 1, "Add endpoint"),
        make_pr("org/api", 2, "Fix endpoint"),
        make_pr("org/api", 3, "Docs for endpoint"),
    ];

    let resp = serde_json::json!({
        "workstreams": [{
            "title": "API endpoint work",
            "summary": "All API endpoint PRs",
            "tags": ["api"],
            "event_indices": [0, 1, 2],
            "receipt_indices": [0]
        }]
    });

    let ws = mock_clusterer(&resp.to_string()).cluster(&events).unwrap();
    assert_eq!(ws.workstreams.len(), 1);
    assert_eq!(ws.workstreams[0].title, "API endpoint work");
    assert_eq!(ws.workstreams[0].events.len(), 3);
    assert_eq!(ws.workstreams[0].receipts.len(), 1);
    assert_eq!(ws.workstreams[0].stats.pull_requests, 3);
    assert_eq!(ws.version, 1);
}

// ---------------------------------------------------------------------------
// Mock LLM – multiple workstreams
// ---------------------------------------------------------------------------

#[test]
fn multiple_workstreams_partition_events() {
    let events = vec![
        make_pr("org/api", 1, "Auth"),
        make_pr("org/ui", 2, "Dashboard"),
        make_review("org/api", 1),
        make_manual(1),
    ];

    let resp = serde_json::json!({
        "workstreams": [
            {
                "title": "Auth",
                "summary": "Auth work",
                "tags": ["auth"],
                "event_indices": [0, 2],
                "receipt_indices": [0]
            },
            {
                "title": "Dashboard",
                "summary": "UI work",
                "tags": ["ui"],
                "event_indices": [1, 3],
                "receipt_indices": [1]
            }
        ]
    });

    let ws = mock_clusterer(&resp.to_string()).cluster(&events).unwrap();
    assert_eq!(ws.workstreams.len(), 2);
    assert_eq!(ws.workstreams[0].stats.pull_requests, 1);
    assert_eq!(ws.workstreams[0].stats.reviews, 1);
    assert_eq!(ws.workstreams[1].stats.pull_requests, 1);
    assert_eq!(ws.workstreams[1].stats.manual_events, 1);
}

// ---------------------------------------------------------------------------
// Orphan collection
// ---------------------------------------------------------------------------

#[test]
fn unassigned_events_become_uncategorized() {
    let events = vec![
        make_pr("org/a", 1, "Assigned"),
        make_pr("org/b", 2, "Orphan 1"),
        make_pr("org/c", 3, "Orphan 2"),
    ];

    let resp = serde_json::json!({
        "workstreams": [{
            "title": "Assigned",
            "event_indices": [0],
            "receipt_indices": [0]
        }]
    });

    let ws = mock_clusterer(&resp.to_string()).cluster(&events).unwrap();
    assert_eq!(ws.workstreams.len(), 2);
    assert_eq!(ws.workstreams[1].title, "Uncategorized");
    assert_eq!(ws.workstreams[1].events.len(), 2);
    assert_eq!(ws.workstreams[1].tags, vec!["uncategorized".to_string()]);
}

// ---------------------------------------------------------------------------
// Empty events
// ---------------------------------------------------------------------------

#[test]
fn empty_events_produces_empty_workstreams() {
    let resp = serde_json::json!({ "workstreams": [] });
    let ws = mock_clusterer(&resp.to_string()).cluster(&[]).unwrap();
    assert!(ws.workstreams.is_empty());
}

// ---------------------------------------------------------------------------
// Duplicate index across workstreams – first-wins
// ---------------------------------------------------------------------------

#[test]
fn duplicate_indices_use_first_wins_semantics() {
    let events = vec![make_pr("org/x", 1, "Contested"), make_pr("org/x", 2, "Also")];

    let resp = serde_json::json!({
        "workstreams": [
            { "title": "First", "event_indices": [0, 1], "receipt_indices": [] },
            { "title": "Second", "event_indices": [0, 1], "receipt_indices": [] }
        ]
    });

    let ws = mock_clusterer(&resp.to_string()).cluster(&events).unwrap();
    // Second workstream has no valid indices → skipped
    assert_eq!(ws.workstreams.len(), 1);
    assert_eq!(ws.workstreams[0].title, "First");
}

// ---------------------------------------------------------------------------
// Out-of-bounds indices are silently skipped
// ---------------------------------------------------------------------------

#[test]
fn out_of_bounds_indices_are_filtered() {
    let events = vec![make_pr("org/x", 1, "Solo")];

    let resp = serde_json::json!({
        "workstreams": [{
            "title": "Mixed",
            "event_indices": [0, 50, 100],
            "receipt_indices": [0, 50]
        }]
    });

    let ws = mock_clusterer(&resp.to_string()).cluster(&events).unwrap();
    assert_eq!(ws.workstreams.len(), 1);
    assert_eq!(ws.workstreams[0].events.len(), 1);
    assert_eq!(ws.workstreams[0].receipts.len(), 1);
}

// ---------------------------------------------------------------------------
// Malformed JSON → error
// ---------------------------------------------------------------------------

#[test]
fn malformed_json_returns_error() {
    let events = vec![make_pr("org/x", 1, "Test")];
    let result = mock_clusterer("not json!").cluster(&events);
    assert!(result.is_err());
}

#[test]
fn incomplete_json_returns_error() {
    let events = vec![make_pr("org/x", 1, "Test")];
    let result = mock_clusterer(r#"{"workstreams": [{"title":"Oops"}"#).cluster(&events);
    assert!(result.is_err());
}

#[test]
fn missing_workstreams_key_returns_error() {
    let events = vec![make_pr("org/x", 1, "Test")];
    let result = mock_clusterer(r#"{"clusters": []}"#).cluster(&events);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Empty workstreams array → all orphaned
// ---------------------------------------------------------------------------

#[test]
fn empty_workstreams_array_orphans_everything() {
    let events = vec![make_pr("org/a", 1, "A"), make_pr("org/b", 2, "B")];
    let resp = serde_json::json!({ "workstreams": [] });

    let ws = mock_clusterer(&resp.to_string()).cluster(&events).unwrap();
    assert_eq!(ws.workstreams.len(), 1);
    assert_eq!(ws.workstreams[0].title, "Uncategorized");
    assert_eq!(ws.workstreams[0].events.len(), 2);
}

// ---------------------------------------------------------------------------
// Mixed event kinds tracked in stats
// ---------------------------------------------------------------------------

#[test]
fn stats_count_mixed_event_kinds() {
    let events = vec![
        make_pr("org/a", 1, "PR"),
        make_review("org/a", 1),
        make_manual(1),
    ];

    let resp = serde_json::json!({
        "workstreams": [{
            "title": "Mixed",
            "event_indices": [0, 1, 2],
            "receipt_indices": [0]
        }]
    });

    let ws = mock_clusterer(&resp.to_string()).cluster(&events).unwrap();
    let s = &ws.workstreams[0].stats;
    assert_eq!(s.pull_requests, 1);
    assert_eq!(s.reviews, 1);
    assert_eq!(s.manual_events, 1);
}

// ---------------------------------------------------------------------------
// Fallback: LlmWithFallback on LLM failure
// ---------------------------------------------------------------------------

#[test]
fn fallback_activates_on_llm_failure() {
    let llm = LlmClusterer::new(Box::new(FailingLlmBackend), LlmConfig::default());
    let clusterer = LlmWithFallback::new(llm);

    let events = vec![
        make_pr("org/alpha", 1, "Alpha feature"),
        make_pr("org/beta", 2, "Beta feature"),
    ];

    let ws = clusterer.cluster(&events).unwrap();
    // Repo-based fallback groups by repo
    assert_eq!(ws.workstreams.len(), 2);
}

#[test]
fn fallback_not_triggered_when_llm_succeeds() {
    let resp = serde_json::json!({
        "workstreams": [{
            "title": "LLM Result",
            "event_indices": [0, 1],
            "receipt_indices": [0]
        }]
    });

    let llm = LlmClusterer::new(
        Box::new(MockLlmBackend {
            response: resp.to_string(),
        }),
        LlmConfig::default(),
    );
    let clusterer = LlmWithFallback::new(llm);

    let events = vec![
        make_pr("org/alpha", 1, "A"),
        make_pr("org/beta", 2, "B"),
    ];

    let ws = clusterer.cluster(&events).unwrap();
    assert_eq!(ws.workstreams.len(), 1);
    assert_eq!(ws.workstreams[0].title, "LLM Result");
}

// ---------------------------------------------------------------------------
// Fallback with empty events
// ---------------------------------------------------------------------------

#[test]
fn fallback_with_empty_events() {
    let llm = LlmClusterer::new(Box::new(FailingLlmBackend), LlmConfig::default());
    let clusterer = LlmWithFallback::new(llm);
    let ws = clusterer.cluster(&[]).unwrap();
    assert!(ws.workstreams.is_empty());
}

// ---------------------------------------------------------------------------
// LlmConfig defaults
// ---------------------------------------------------------------------------

#[test]
fn config_default_values() {
    let cfg = LlmConfig::default();
    assert_eq!(cfg.model, "gpt-4o-mini");
    assert_eq!(cfg.max_input_tokens, 8000);
    assert!((cfg.temperature - 0.2).abs() < f64::EPSILON);
    assert_eq!(cfg.timeout_secs, 60);
    assert!(cfg.max_workstreams.is_none());
    assert!(cfg.api_endpoint.contains("openai.com"));
}

// ---------------------------------------------------------------------------
// Custom backend via trait object
// ---------------------------------------------------------------------------

struct UppercaseBackend;
impl LlmBackend for UppercaseBackend {
    fn complete(&self, _system: &str, user: &str) -> anyhow::Result<String> {
        // Return a valid workstream JSON that echoes input length
        let len = user.len();
        Ok(serde_json::json!({
            "workstreams": [{
                "title": format!("Processed {len} chars"),
                "event_indices": [0],
                "receipt_indices": [0]
            }]
        })
        .to_string())
    }
}

#[test]
fn custom_backend_implementation() {
    let clusterer = LlmClusterer::new(Box::new(UppercaseBackend), LlmConfig::default());
    let events = vec![make_pr("org/x", 1, "Test")];
    let ws = clusterer.cluster(&events).unwrap();
    assert_eq!(ws.workstreams.len(), 1);
    assert!(ws.workstreams[0].title.starts_with("Processed "));
}

// ---------------------------------------------------------------------------
// Multi-chunk clustering
// ---------------------------------------------------------------------------

#[test]
fn multi_chunk_merges_workstreams() {
    // Use very small max_input_tokens to force chunking
    let cfg = LlmConfig {
        max_input_tokens: 20, // tiny budget → each event its own chunk
        ..LlmConfig::default()
    };

    let resp = serde_json::json!({
        "workstreams": [{
            "title": "Chunk result",
            "event_indices": [0],
            "receipt_indices": [0]
        }]
    });

    let clusterer = LlmClusterer::new(
        Box::new(MockLlmBackend {
            response: resp.to_string(),
        }),
        cfg,
    );

    let events = vec![
        make_pr("org/a", 1, "First event with a pretty long title to blow the budget"),
        make_pr("org/b", 2, "Second event with a pretty long title to blow the budget"),
    ];

    let ws = clusterer.cluster(&events).unwrap();
    // Each chunk produces one workstream
    assert!(ws.workstreams.len() >= 2);
}

// ---------------------------------------------------------------------------
// Single-chunk path when budget is large
// ---------------------------------------------------------------------------

#[test]
fn single_chunk_path_when_budget_is_large() {
    let cfg = LlmConfig {
        max_input_tokens: 1_000_000,
        ..LlmConfig::default()
    };

    let resp = serde_json::json!({
        "workstreams": [{
            "title": "All in one",
            "event_indices": [0, 1],
            "receipt_indices": [0]
        }]
    });

    let clusterer = LlmClusterer::new(
        Box::new(MockLlmBackend {
            response: resp.to_string(),
        }),
        cfg,
    );

    let events = vec![make_pr("org/a", 1, "A"), make_pr("org/b", 2, "B")];

    let ws = clusterer.cluster(&events).unwrap();
    assert_eq!(ws.workstreams.len(), 1);
    assert_eq!(ws.workstreams[0].events.len(), 2);
}

// ---------------------------------------------------------------------------
// Tags and summary are preserved
// ---------------------------------------------------------------------------

#[test]
fn tags_and_summary_preserved() {
    let events = vec![make_pr("org/a", 1, "Test")];

    let resp = serde_json::json!({
        "workstreams": [{
            "title": "Tagged",
            "summary": "A careful summary",
            "tags": ["backend", "api", "auth"],
            "event_indices": [0],
            "receipt_indices": []
        }]
    });

    let ws = mock_clusterer(&resp.to_string()).cluster(&events).unwrap();
    let w = &ws.workstreams[0];
    assert_eq!(w.summary.as_deref(), Some("A careful summary"));
    assert_eq!(w.tags, vec!["backend", "api", "auth"]);
}

// ---------------------------------------------------------------------------
// Version field is always 1
// ---------------------------------------------------------------------------

#[test]
fn version_is_always_one() {
    let events = vec![make_pr("org/a", 1, "Test")];
    let resp = serde_json::json!({
        "workstreams": [{
            "title": "V",
            "event_indices": [0],
            "receipt_indices": []
        }]
    });

    let ws = mock_clusterer(&resp.to_string()).cluster(&events).unwrap();
    assert_eq!(ws.version, 1);
}

// ---------------------------------------------------------------------------
// Receipts filtered to valid events only
// ---------------------------------------------------------------------------

#[test]
fn receipt_indices_filtered_to_claimed_events() {
    let events = vec![make_pr("org/a", 1, "A"), make_pr("org/a", 2, "B")];

    let resp = serde_json::json!({
        "workstreams": [{
            "title": "Partial",
            "event_indices": [0],
            "receipt_indices": [0, 1]
        }]
    });

    let ws = mock_clusterer(&resp.to_string()).cluster(&events).unwrap();
    // receipt index 1 is not in event_indices for this workstream
    assert_eq!(ws.workstreams[0].receipts.len(), 1);
}
