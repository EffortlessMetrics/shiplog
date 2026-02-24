use shiplog_schema::event::{EventEnvelope, EventPayload};

/// One-line summary of an event for LLM context.
pub fn summarize_event(ev: &EventEnvelope) -> String {
    match &ev.payload {
        EventPayload::PullRequest(pr) => {
            let stats = match (pr.additions, pr.deletions, pr.changed_files) {
                (Some(a), Some(d), Some(f)) => format!(" [+{a}/-{d}, {f} files]"),
                _ => String::new(),
            };
            let date = ev.occurred_at.format("%Y-%m-%d");
            format!(
                "PR#{} in {}: {}{}  ({})",
                pr.number, ev.repo.full_name, pr.title, stats, date
            )
        }
        EventPayload::Review(r) => {
            let date = ev.occurred_at.format("%Y-%m-%d");
            format!(
                "Review on PR#{} in {}: {} [{}] ({})",
                r.pull_number, ev.repo.full_name, r.pull_title, r.state, date
            )
        }
        EventPayload::Manual(m) => {
            let date = ev.occurred_at.format("%Y-%m-%d");
            format!("{:?}: {} ({})", m.event_type, m.title, date)
        }
    }
}

/// Format events as a numbered list for the LLM prompt.
pub fn format_event_list(events: &[EventEnvelope]) -> String {
    events
        .iter()
        .enumerate()
        .map(|(i, ev)| format!("[{i}] {}", summarize_event(ev)))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Split events into chunks that fit within a token budget.
/// Uses ~4 chars per token heuristic.
pub fn chunk_events(events: &[EventEnvelope], max_tokens: usize) -> Vec<Vec<usize>> {
    let max_chars = max_tokens * 4;
    let mut chunks = Vec::new();
    let mut current_chunk = Vec::new();
    let mut current_chars = 0;

    for (i, ev) in events.iter().enumerate() {
        let summary = summarize_event(ev);
        let line_chars = summary.len() + 10; // overhead for "[N] " prefix + newline

        if current_chars + line_chars > max_chars && !current_chunk.is_empty() {
            chunks.push(current_chunk);
            current_chunk = Vec::new();
            current_chars = 0;
        }

        current_chunk.push(i);
        current_chars += line_chars;
    }

    if !current_chunk.is_empty() {
        chunks.push(current_chunk);
    }

    chunks
}

/// System prompt instructing the LLM how to cluster events.
pub fn system_prompt(max_workstreams: Option<usize>) -> String {
    let ws_limit = max_workstreams
        .map(|n| format!("Create at most {n} workstreams."))
        .unwrap_or_default();

    format!(
        r#"You are a software engineering work categorizer. Given a list of development events (pull requests, reviews, manual entries), group them into thematic workstreams.

Each workstream should represent a coherent body of work (e.g., "Authentication improvements", "CI/CD pipeline", "Bug fixes for billing module").

{ws_limit}

Return a JSON object with this exact structure:
{{
  "workstreams": [
    {{
      "title": "Human-readable workstream title",
      "summary": "One-sentence description of what this workstream covers",
      "tags": ["relevant", "tags"],
      "event_indices": [0, 1, 5],
      "receipt_indices": [0, 1]
    }}
  ]
}}

Rules:
- event_indices: indices from the provided event list that belong to this workstream
- receipt_indices: subset of event_indices to highlight as key receipts (max 10 per workstream)
- Every event index should appear in exactly one workstream
- Tags should be lowercase, descriptive (e.g., "backend", "frontend", "infrastructure", "bugfix")
- Group by theme, not just by repository — cross-repo themes are valuable
- Return ONLY valid JSON, no markdown fences or extra text"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use shiplog_ids::EventId;
    use shiplog_schema::event::*;

    fn make_pr_event(num: u64, title: &str) -> EventEnvelope {
        EventEnvelope {
            id: EventId::from_parts(["test", "pr", &num.to_string()]),
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

    #[test]
    fn format_event_list_with_events() {
        let events = vec![make_pr_event(1, "Add login"), make_pr_event(2, "Fix bug")];
        let result = format_event_list(&events);
        assert!(result.contains("[0]"), "should contain index [0]");
        assert!(result.contains("[1]"), "should contain index [1]");
        assert!(result.contains("Add login"), "should contain event content");
        assert!(result.contains("Fix bug"), "should contain event content");
    }

    #[test]
    fn format_event_list_empty() {
        let result = format_event_list(&[]);
        assert_eq!(result, "", "empty events should produce empty string");
    }

    #[test]
    fn format_event_list_single_event_no_trailing_newline() {
        let events = vec![make_pr_event(1, "Solo")];
        let result = format_event_list(&events);
        assert!(result.starts_with("[0]"));
        assert!(
            !result.contains('\n'),
            "single event should have no newline"
        );
    }

    #[test]
    fn chunk_events_single_event_one_chunk() {
        let events = vec![make_pr_event(1, "Small event")];
        let chunks = chunk_events(&events, 10000);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], vec![0]);
    }

    #[test]
    fn chunk_events_empty_zero_chunks() {
        let chunks = chunk_events(&[], 10000);
        assert_eq!(chunks.len(), 0);
    }

    #[test]
    fn chunk_events_all_indices_covered() {
        let events: Vec<EventEnvelope> = (0..50)
            .map(|i| make_pr_event(i, &format!("Event number {i} with padding text")))
            .collect();

        let chunks = chunk_events(&events, 200); // small budget to force splits
        let all_indices: Vec<usize> = chunks.into_iter().flatten().collect();
        assert_eq!(all_indices.len(), 50, "all 50 indices must be covered");
        for i in 0..50 {
            assert!(
                all_indices.contains(&i),
                "index {i} should be in some chunk"
            );
        }
    }

    #[test]
    fn chunk_events_large_budget_single_chunk() {
        let events: Vec<EventEnvelope> = (0..10).map(|i| make_pr_event(i, "Short")).collect();

        let chunks = chunk_events(&events, 1_000_000);
        assert_eq!(
            chunks.len(),
            1,
            "large budget should keep everything in one chunk"
        );
        assert_eq!(chunks[0].len(), 10);
    }

    #[test]
    fn system_prompt_no_limit_contains_workstream() {
        let prompt = system_prompt(None);
        assert!(
            prompt.contains("workstream"),
            "system prompt should mention workstream"
        );
    }

    #[test]
    fn system_prompt_with_limit_contains_max() {
        let prompt = system_prompt(Some(5));
        assert!(
            prompt.contains("at most 5"),
            "system prompt should contain 'at most 5'"
        );
    }

    #[test]
    fn system_prompt_none_does_not_contain_at_most() {
        let prompt = system_prompt(None);
        assert!(
            !prompt.contains("at most"),
            "no limit should not produce 'at most' text"
        );
    }

    #[test]
    fn summarize_event_review() {
        let ev = EventEnvelope {
            id: EventId::from_parts(["test", "review", "1"]),
            kind: EventKind::Review,
            occurred_at: Utc::now(),
            actor: Actor {
                login: "reviewer".into(),
                id: None,
            },
            repo: RepoRef {
                full_name: "org/repo".into(),
                html_url: None,
                visibility: RepoVisibility::Unknown,
            },
            payload: EventPayload::Review(ReviewEvent {
                pull_number: 42,
                pull_title: "Fix auth".into(),
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
        };
        let summary = summarize_event(&ev);
        assert!(summary.contains("Review on PR#42"));
        assert!(summary.contains("Fix auth"));
        assert!(summary.contains("approved"));
    }

    #[test]
    fn summarize_event_manual() {
        let ev = EventEnvelope {
            id: EventId::from_parts(["test", "manual", "1"]),
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
                event_type: ManualEventType::Incident,
                title: "Outage response".into(),
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
        };
        let summary = summarize_event(&ev);
        assert!(summary.contains("Incident"), "should contain event type");
        assert!(summary.contains("Outage response"), "should contain title");
    }

    #[test]
    fn summarize_event_pr_without_stats() {
        let ev = EventEnvelope {
            id: EventId::from_parts(["test", "pr", "1"]),
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
                number: 99,
                title: "No stats PR".into(),
                state: PullRequestState::Open,
                created_at: Utc::now(),
                merged_at: None,
                additions: None,
                deletions: None,
                changed_files: None,
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
        };
        let summary = summarize_event(&ev);
        assert!(summary.contains("PR#99"));
        assert!(summary.contains("No stats PR"));
        // Should not have the stats bracket when additions/deletions/changed_files are None
        assert!(!summary.contains("[+"), "should not have stats bracket");
    }
}
