mod client;
mod config;
mod parse;
mod prompt;

pub use client::{FailingLlmBackend, LlmBackend, MockLlmBackend, OpenAiCompatibleBackend};
pub use config::LlmConfig;

use anyhow::Result;
use shiplog_ports::WorkstreamClusterer;
use shiplog_schema::event::EventEnvelope;
use shiplog_schema::workstream::WorkstreamsFile;
use shiplog_workstreams::RepoClusterer;

/// LLM-assisted workstream clusterer.
pub struct LlmClusterer {
    backend: Box<dyn LlmBackend>,
    config: LlmConfig,
}

impl LlmClusterer {
    pub fn new(backend: Box<dyn LlmBackend>, config: LlmConfig) -> Self {
        Self { backend, config }
    }

    fn cluster_chunk(&self, events: &[EventEnvelope], indices: &[usize]) -> Result<String> {
        let subset: Vec<&EventEnvelope> = indices.iter().map(|&i| &events[i]).collect();
        let event_list: String = subset
            .iter()
            .enumerate()
            .map(|(i, ev)| format!("[{}] {}", i, prompt::summarize_event(ev)))
            .collect::<Vec<_>>()
            .join("\n");

        let system = prompt::system_prompt(self.config.max_workstreams);
        let user_msg =
            format!("Cluster these development events into workstreams:\n\n{event_list}");

        self.backend.complete(&system, &user_msg)
    }
}

impl WorkstreamClusterer for LlmClusterer {
    fn cluster(&self, events: &[EventEnvelope]) -> Result<WorkstreamsFile> {
        let chunks = prompt::chunk_events(events, self.config.max_input_tokens);

        if chunks.len() <= 1 {
            // Single pass
            let event_list = prompt::format_event_list(events);
            let system = prompt::system_prompt(self.config.max_workstreams);
            let user_msg =
                format!("Cluster these development events into workstreams:\n\n{event_list}");
            let response = self.backend.complete(&system, &user_msg)?;
            parse::parse_llm_response(&response, events)
        } else {
            // Multi-chunk: cluster each chunk, merge results
            let mut all_workstreams = Vec::new();

            for chunk_indices in &chunks {
                let response = self.cluster_chunk(events, chunk_indices)?;
                // Map local indices back to global
                let chunk_events: Vec<EventEnvelope> =
                    chunk_indices.iter().map(|&i| events[i].clone()).collect();
                let mut ws_file = parse::parse_llm_response(&response, &chunk_events)?;
                all_workstreams.append(&mut ws_file.workstreams);
            }

            Ok(WorkstreamsFile {
                version: 1,
                generated_at: chrono::Utc::now(),
                workstreams: all_workstreams,
            })
        }
    }
}

/// Wrapper that falls back to repo-based clustering on LLM failure.
pub struct LlmWithFallback {
    llm: LlmClusterer,
    fallback: RepoClusterer,
}

impl LlmWithFallback {
    pub fn new(llm: LlmClusterer) -> Self {
        Self {
            llm,
            fallback: RepoClusterer,
        }
    }
}

impl WorkstreamClusterer for LlmWithFallback {
    fn cluster(&self, events: &[EventEnvelope]) -> Result<WorkstreamsFile> {
        match self.llm.cluster(events) {
            Ok(ws) => Ok(ws),
            Err(e) => {
                eprintln!("WARN: LLM clustering failed, falling back to repo-based: {e}");
                self.fallback.cluster(events)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use shiplog_ids::EventId;
    use shiplog_schema::event::*;

    fn make_test_event(repo: &str, pr_num: u64, title: &str) -> EventEnvelope {
        EventEnvelope {
            id: EventId::from_parts(["test", &pr_num.to_string()]),
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
                number: pr_num,
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
    fn mock_llm_produces_valid_workstreams() {
        let mock_response = serde_json::json!({
            "workstreams": [{
                "title": "Auth improvements",
                "summary": "Authentication work",
                "tags": ["auth", "backend"],
                "event_indices": [0, 1],
                "receipt_indices": [0]
            }]
        });

        let backend = MockLlmBackend {
            response: mock_response.to_string(),
        };
        let config = LlmConfig::default();
        let clusterer = LlmClusterer::new(Box::new(backend), config);

        let events = vec![
            make_test_event("org/auth", 1, "Add OAuth"),
            make_test_event("org/auth", 2, "Fix token refresh"),
        ];

        let ws = clusterer.cluster(&events).unwrap();
        assert_eq!(ws.workstreams.len(), 1);
        assert_eq!(ws.workstreams[0].title, "Auth improvements");
        assert_eq!(ws.workstreams[0].events.len(), 2);
    }

    #[test]
    fn fallback_on_llm_failure() {
        let backend = FailingLlmBackend;
        let config = LlmConfig::default();
        let llm = LlmClusterer::new(Box::new(backend), config);
        let clusterer = LlmWithFallback::new(llm);

        let events = vec![
            make_test_event("org/repo-a", 1, "Feature A"),
            make_test_event("org/repo-b", 2, "Feature B"),
        ];

        let ws = clusterer.cluster(&events).unwrap();
        // Should fall back to repo-based clustering
        assert_eq!(ws.workstreams.len(), 2);
    }

    #[test]
    fn orphan_events_collected() {
        // LLM only assigns event 0, event 1 becomes orphan
        let mock_response = serde_json::json!({
            "workstreams": [{
                "title": "Only first",
                "summary": "test",
                "tags": [],
                "event_indices": [0],
                "receipt_indices": [0]
            }]
        });

        let backend = MockLlmBackend {
            response: mock_response.to_string(),
        };
        let config = LlmConfig::default();
        let clusterer = LlmClusterer::new(Box::new(backend), config);

        let events = vec![
            make_test_event("org/repo", 1, "Assigned"),
            make_test_event("org/repo", 2, "Orphan"),
        ];

        let ws = clusterer.cluster(&events).unwrap();
        assert_eq!(ws.workstreams.len(), 2);
        assert_eq!(ws.workstreams[1].title, "Uncategorized");
    }

    #[test]
    fn out_of_bounds_indices_skipped() {
        let mock_response = serde_json::json!({
            "workstreams": [{
                "title": "Test",
                "summary": "test",
                "tags": [],
                "event_indices": [0, 99],
                "receipt_indices": [0]
            }]
        });

        let backend = MockLlmBackend {
            response: mock_response.to_string(),
        };
        let config = LlmConfig::default();
        let clusterer = LlmClusterer::new(Box::new(backend), config);

        let events = vec![make_test_event("org/repo", 1, "Only event")];

        let ws = clusterer.cluster(&events).unwrap();
        assert_eq!(ws.workstreams.len(), 1);
        assert_eq!(ws.workstreams[0].events.len(), 1);
    }

    #[test]
    fn malformed_json_returns_error() {
        let backend = MockLlmBackend {
            response: "not json at all".to_string(),
        };
        let config = LlmConfig::default();
        let clusterer = LlmClusterer::new(Box::new(backend), config);

        let events = vec![make_test_event("org/repo", 1, "Test")];
        assert!(clusterer.cluster(&events).is_err());
    }

    #[test]
    fn summarize_event_formats_correctly() {
        let ev = make_test_event("myorg/myrepo", 42, "Add login page");
        let summary = prompt::summarize_event(&ev);
        assert!(summary.contains("PR#42"));
        assert!(summary.contains("myorg/myrepo"));
        assert!(summary.contains("Add login page"));
        assert!(summary.contains("+10/-5"));
    }

    #[test]
    fn chunk_events_splits_large_sets() {
        // Create enough events to exceed a small token budget
        let events: Vec<EventEnvelope> = (0..100)
            .map(|i| make_test_event("org/repo", i, &format!("Event {i} with some extra text")))
            .collect();

        let chunks = prompt::chunk_events(&events, 500); // very small budget
        assert!(chunks.len() > 1, "should split into multiple chunks");

        // All indices should be covered
        let all_indices: Vec<usize> = chunks.into_iter().flatten().collect();
        assert_eq!(all_indices.len(), 100);
    }
}
