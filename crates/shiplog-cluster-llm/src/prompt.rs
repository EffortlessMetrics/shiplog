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
