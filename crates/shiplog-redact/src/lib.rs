use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use shiplog_ports::Redactor;
use shiplog_schema::event::{EventEnvelope, EventPayload, RepoRef, RepoVisibility};
use shiplog_schema::workstream::{Workstream, WorkstreamsFile};
use std::collections::BTreeMap;

/// Rendering profiles.
///
/// The tool produces multiple projections from the same ledger.
/// Think of them as lenses, not forks.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RedactionProfile {
    Internal,
    Manager,
    Public,
}

impl RedactionProfile {
    pub fn as_str(&self) -> &'static str {
        match self {
            RedactionProfile::Internal => "internal",
            RedactionProfile::Manager => "manager",
            RedactionProfile::Public => "public",
        }
    }
}

/// Deterministic redactor.
///
/// This intentionally does not try to be clever.
/// - It doesn't do NLP.
/// - It doesn't detect secrets.
/// - It does *structural* redaction so you can safely share packets.
pub struct DeterministicRedactor {
    key: Vec<u8>,
    /// Optional map to preserve stable aliases across runs.
    /// (For MVP we keep it in-memory; later this becomes a file.)
    cache: std::sync::Mutex<BTreeMap<String, String>>,
}

impl DeterministicRedactor {
    pub fn new(key: impl AsRef<[u8]>) -> Self {
        Self {
            key: key.as_ref().to_vec(),
            cache: std::sync::Mutex::new(BTreeMap::new()),
        }
    }

    fn alias(&self, kind: &str, value: &str) -> String {
        let cache_key = format!("{kind}:{value}");
        if let Ok(cache) = self.cache.lock() {
            if let Some(v) = cache.get(&cache_key) {
                return v.clone();
            }
        }

        let mut h = Sha256::new();
        h.update(&self.key);
        h.update(b"\n");
        h.update(kind.as_bytes());
        h.update(b"\n");
        h.update(value.as_bytes());
        let out = hex::encode(h.finalize());
        let alias = format!("{kind}-{}", &out[..12]);

        if let Ok(mut cache) = self.cache.lock() {
            cache.insert(cache_key, alias.clone());
        }
        alias
    }

    fn redact_repo_public(&self, repo: &RepoRef) -> RepoRef {
        RepoRef {
            full_name: self.alias("repo", &repo.full_name),
            html_url: None,
            visibility: RepoVisibility::Unknown,
        }
    }

    fn redact_event_public(&self, mut ev: EventEnvelope) -> EventEnvelope {
        ev.repo = self.redact_repo_public(&ev.repo);

        // Titles leak. Strip.
        match &mut ev.payload {
            EventPayload::PullRequest(pr) => {
                pr.title = "[redacted]".to_string();
                pr.touched_paths_hint.clear();
            }
            EventPayload::Review(r) => {
                r.pull_title = "[redacted]".to_string();
            }
            EventPayload::Manual(m) => {
                // Manual events: redact title and description
                m.title = "[redacted]".to_string();
                m.description = None;
                m.impact = None;
            }
        }

        // Links leak. Strip.
        ev.links.clear();

        // Source url leaks.
        ev.source.url = None;

        ev
    }

    fn redact_workstream_public(&self, mut ws: Workstream) -> Workstream {
        ws.title = self.alias("ws", &ws.title);
        ws.summary = None;
        ws.tags.retain(|t| t != "repo");
        ws
    }

    /// Redact an event for manager view.
    /// 
    /// Manager view is a middle ground:
    /// - Keep titles and repo names (managers need context)
    /// - Remove sensitive details (touched_paths, descriptions)
    /// - Remove links (might contain external references)
    /// - Keep source URLs (internal org use)
    fn redact_event_manager(&self, mut ev: EventEnvelope) -> EventEnvelope {
        match &mut ev.payload {
            EventPayload::PullRequest(pr) => {
                // Remove path hints (sensitive) but keep title
                pr.touched_paths_hint.clear();
            }
            EventPayload::Review(_) => {
                // Reviews are fine as-is
            }
            EventPayload::Manual(m) => {
                // Keep title but remove detailed descriptions
                m.description = None;
                m.impact = None;
            }
        }

        // Remove links (might contain external references)
        ev.links.clear();

        // Keep source URL for internal debugging
        
        ev
    }

    /// Redact a workstream for manager view.
    /// 
    /// Keep titles readable but remove summaries (might contain sensitive details).
    fn redact_workstream_manager(&self, mut ws: Workstream) -> Workstream {
        ws.summary = None;
        // Keep all tags for context
        ws
    }
}

impl Redactor for DeterministicRedactor {
    fn redact_events(&self, events: &[EventEnvelope], profile: &str) -> Result<Vec<EventEnvelope>> {
        let p = match profile {
            "internal" => RedactionProfile::Internal,
            "manager" => RedactionProfile::Manager,
            "public" => RedactionProfile::Public,
            _ => RedactionProfile::Public,
        };

        let out = match p {
            RedactionProfile::Internal => events.to_vec(),
            RedactionProfile::Manager => {
                // Manager view: keep titles/repo names but remove sensitive details
                events.iter().cloned().map(|e| self.redact_event_manager(e)).collect()
            }
            RedactionProfile::Public => events.iter().cloned().map(|e| self.redact_event_public(e)).collect(),
        };

        Ok(out)
    }

    fn redact_workstreams(
        &self,
        workstreams: &WorkstreamsFile,
        profile: &str,
    ) -> Result<WorkstreamsFile> {
        let p = match profile {
            "internal" => RedactionProfile::Internal,
            "manager" => RedactionProfile::Manager,
            "public" => RedactionProfile::Public,
            _ => RedactionProfile::Public,
        };

        let out = match p {
            RedactionProfile::Internal => workstreams.clone(),
            RedactionProfile::Manager => WorkstreamsFile {
                workstreams: workstreams
                    .workstreams
                    .iter()
                    .cloned()
                    .map(|ws| self.redact_workstream_manager(ws))
                    .collect(),
                ..workstreams.clone()
            },
            RedactionProfile::Public => WorkstreamsFile {
                workstreams: workstreams
                    .workstreams
                    .iter()
                    .cloned()
                    .map(|ws| self.redact_workstream_public(ws))
                    .collect(),
                ..workstreams.clone()
            },
        };

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use shiplog_schema::event::*;
    use shiplog_ids::EventId;
    use chrono::Utc;

    proptest! {
        #[test]
        fn aliases_are_stable_for_same_key(kind in "repo|ws", value in ".*") {
            let r = DeterministicRedactor::new(b"test-key");
            let a1 = r.alias(&kind, &value);
            let a2 = r.alias(&kind, &value);
            prop_assert_eq!(a1, a2);
        }
    }

    #[test]
    fn public_profile_strips_titles_and_links() {
        let r = DeterministicRedactor::new(b"k");
        let ev = EventEnvelope {
            id: EventId::from_parts(["x","1"]),
            kind: EventKind::PullRequest,
            occurred_at: Utc::now(),
            actor: Actor { login: "a".into(), id: None },
            repo: RepoRef { full_name: "o/r".into(), html_url: Some("https://github.com/o/r".into()), visibility: RepoVisibility::Private },
            payload: EventPayload::PullRequest(PullRequestEvent {
                number: 1,
                title: "secret pr title".into(),
                state: PullRequestState::Merged,
                created_at: Utc::now(),
                merged_at: Some(Utc::now()),
                additions: Some(1),
                deletions: Some(1),
                changed_files: Some(1),
                touched_paths_hint: vec!["secret/path".into()],
                window: None,
            }),
            tags: vec![],
            links: vec![Link { label: "pr".into(), url: "https://github.com/o/r/pull/1".into() }],
            source: SourceRef { system: SourceSystem::Github, url: Some("https://api.github.com/...".into()), opaque_id: None },
        };

        let out = r.redact_events(&[ev], "public").unwrap();
        match &out[0].payload {
            EventPayload::PullRequest(pr) => {
                assert_eq!(pr.title, "[redacted]");
                assert!(pr.touched_paths_hint.is_empty());
            }
            _ => panic!("expected pr"),
        }
        assert!(out[0].links.is_empty());
        assert!(out[0].source.url.is_none());
        assert_ne!(out[0].repo.full_name, "o/r");
    }

    /// Property test: PR titles must not appear in public redacted output
    #[test]
    fn public_redaction_no_leak_pr_title() {
        let r = DeterministicRedactor::new(b"test-key");
        let sensitive_title = "Secret Feature: Internal Auth Bypass";
        
        let ev = EventEnvelope {
            id: EventId::from_parts(["x","1"]),
            kind: EventKind::PullRequest,
            occurred_at: Utc::now(),
            actor: Actor { login: "a".into(), id: None },
            repo: RepoRef { full_name: "o/r".into(), html_url: None, visibility: RepoVisibility::Private },
            payload: EventPayload::PullRequest(PullRequestEvent {
                number: 1,
                title: sensitive_title.into(),
                state: PullRequestState::Merged,
                created_at: Utc::now(),
                merged_at: Some(Utc::now()),
                additions: Some(10),
                deletions: Some(5),
                changed_files: Some(2),
                touched_paths_hint: vec![],
                window: None,
            }),
            tags: vec![],
            links: vec![],
            source: SourceRef { system: SourceSystem::Github, url: None, opaque_id: None },
        };

        let out = r.redact_events(&[ev], "public").unwrap();
        let json = serde_json::to_string(&out).unwrap();
        
        assert!(!json.contains(sensitive_title), "Sensitive PR title leaked in JSON output");
        assert!(!json.contains("Auth Bypass"), "Partial sensitive content leaked");
    }

    /// Property test: Repo names must not appear in public redacted output
    #[test]
    fn public_redaction_no_leak_repo_name() {
        let r = DeterministicRedactor::new(b"test-key");
        let sensitive_repo = "acme-corp/top-secret-project";
        
        let ev = EventEnvelope {
            id: EventId::from_parts(["x","1"]),
            kind: EventKind::PullRequest,
            occurred_at: Utc::now(),
            actor: Actor { login: "a".into(), id: None },
            repo: RepoRef { full_name: sensitive_repo.into(), html_url: None, visibility: RepoVisibility::Private },
            payload: EventPayload::PullRequest(PullRequestEvent {
                number: 1,
                title: "test".into(),
                state: PullRequestState::Merged,
                created_at: Utc::now(),
                merged_at: Some(Utc::now()),
                additions: Some(1),
                deletions: Some(1),
                changed_files: Some(1),
                touched_paths_hint: vec![],
                window: None,
            }),
            tags: vec![],
            links: vec![],
            source: SourceRef { system: SourceSystem::Github, url: None, opaque_id: None },
        };

        let out = r.redact_events(&[ev], "public").unwrap();
        let json = serde_json::to_string(&out).unwrap();
        
        assert!(!json.contains(sensitive_repo), "Sensitive repo name leaked in JSON output");
        assert!(!json.contains("acme-corp"), "Org name leaked in JSON output");
        assert!(!json.contains("top-secret"), "Project name leaked in JSON output");
    }

    /// Property test: Manual event content must not leak in public mode
    #[test]
    fn public_redaction_no_leak_manual_content() {
        use chrono::NaiveDate;
        
        let r = DeterministicRedactor::new(b"test-key");
        let sensitive_title = "Security Incident: Data Breach Response";
        let sensitive_desc = "Customer PII was exposed in production logs";
        let sensitive_impact = "Affected 10,000 user records";
        
        let ev = EventEnvelope {
            id: EventId::from_parts(["x","1"]),
            kind: EventKind::Manual,
            occurred_at: Utc::now(),
            actor: Actor { login: "a".into(), id: None },
            repo: RepoRef { full_name: "o/r".into(), html_url: None, visibility: RepoVisibility::Private },
            payload: EventPayload::Manual(ManualEvent {
                event_type: ManualEventType::Incident,
                title: sensitive_title.into(),
                description: Some(sensitive_desc.into()),
                impact: Some(sensitive_impact.into()),
                started_at: Some(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap()),
                ended_at: Some(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap()),
            }),
            tags: vec![],
            links: vec![],
            source: SourceRef { system: SourceSystem::Manual, url: None, opaque_id: None },
        };

        let out = r.redact_events(&[ev], "public").unwrap();
        let json = serde_json::to_string(&out).unwrap();
        
        assert!(!json.contains(sensitive_title), "Sensitive manual event title leaked");
        assert!(!json.contains(sensitive_desc), "Sensitive manual event description leaked");
        assert!(!json.contains(sensitive_impact), "Sensitive manual event impact leaked");
        assert!(!json.contains("Data Breach"), "Partial sensitive content leaked");
        assert!(!json.contains("PII"), "Sensitive abbreviation leaked");
    }

    /// Property test: All URL patterns must be stripped from public output
    #[test]
    fn public_redaction_strips_all_urls() {
        let r = DeterministicRedactor::new(b"test-key");
        
        let urls = vec![
            "https://github.com/acme-corp/secret/pull/42",
            "https://api.github.com/repos/acme-corp/secret/issues/1",
            "https://jira.internal.company.com/SECRET-123",
            "https://docs.google.com/document/d/secret-doc-id",
        ];
        
        for url in urls {
            let ev = EventEnvelope {
                id: EventId::from_parts(["x","1"]),
                kind: EventKind::PullRequest,
                occurred_at: Utc::now(),
                actor: Actor { login: "a".into(), id: None },
                repo: RepoRef { full_name: "o/r".into(), html_url: Some(url.into()), visibility: RepoVisibility::Private },
                payload: EventPayload::PullRequest(PullRequestEvent {
                    number: 1,
                    title: "test".into(),
                    state: PullRequestState::Merged,
                    created_at: Utc::now(),
                    merged_at: Some(Utc::now()),
                    additions: Some(1),
                    deletions: Some(1),
                    changed_files: Some(1),
                    touched_paths_hint: vec![],
                    window: None,
                }),
                tags: vec![],
                links: vec![Link { label: "link".into(), url: url.into() }],
                source: SourceRef { system: SourceSystem::Github, url: Some(url.into()), opaque_id: None },
            };

            let out = r.redact_events(&[ev], "public").unwrap();
            let json = serde_json::to_string(&out).unwrap();
            
            // URLs should be completely gone
            assert!(!json.contains("github.com/acme-corp"), "GitHub URL leaked: {}", url);
            assert!(!json.contains("jira.internal"), "Jira URL leaked: {}", url);
            assert!(!json.contains("docs.google.com"), "Google Docs URL leaked: {}", url);
            assert!(!json.contains("http"), "HTTP prefix leaked in: {}", url);
        }
    }

    /// Property test: Workstream titles and summaries must not leak in public mode
    #[test]
    fn workstream_redaction_no_leak() {
        use shiplog_schema::workstream::WorkstreamStats;
        use shiplog_ids::WorkstreamId;
        
        let r = DeterministicRedactor::new(b"test-key");
        
        let ws = Workstream {
            id: WorkstreamId::from_parts(["ws", "test"]),
            title: "Secret Project: Quantum Encryption".into(),
            summary: Some("Developing military-grade encryption for classified communications".into()),
            tags: vec!["security".into(), "classified".into(), "repo".into()],
            stats: WorkstreamStats::zero(),
            events: vec![],
            receipts: vec![],
        };
        
        let ws_file = WorkstreamsFile {
            workstreams: vec![ws],
            version: 1,
            generated_at: Utc::now(),
        };
        
        let out = r.redact_workstreams(&ws_file, "public").unwrap();
        let json = serde_json::to_string(&out).unwrap();
        
        // Original title should not appear (aliased instead)
        assert!(!json.contains("Quantum Encryption"), "Workstream title leaked");
        assert!(!json.contains("military-grade"), "Workstream summary leaked");
        
        // Summary should be None (not present in output)
        assert!(!json.contains("Developing"), "Workstream description leaked");
        
        // "repo" tag should be filtered out, but other tags remain
        let ws_out = &out.workstreams[0];
        assert!(!ws_out.tags.contains(&"repo".into()), "repo tag should be filtered");
        assert!(ws_out.tags.contains(&"security".into()), "security tag should remain");
        assert!(ws_out.tags.contains(&"classified".into()), "classified tag should remain (only 'repo' is filtered)");
    }

    /// Property test: Internal profile should NOT redact (sanity check)
    #[test]
    fn internal_profile_preserves_all_data() {
        let r = DeterministicRedactor::new(b"test-key");
        let sensitive_title = "Secret Feature Title";
        let sensitive_repo = "secret-org/secret-repo";
        
        let ev = EventEnvelope {
            id: EventId::from_parts(["x","1"]),
            kind: EventKind::PullRequest,
            occurred_at: Utc::now(),
            actor: Actor { login: "a".into(), id: None },
            repo: RepoRef { full_name: sensitive_repo.into(), html_url: Some("https://github.com/secret".into()), visibility: RepoVisibility::Private },
            payload: EventPayload::PullRequest(PullRequestEvent {
                number: 1,
                title: sensitive_title.into(),
                state: PullRequestState::Merged,
                created_at: Utc::now(),
                merged_at: Some(Utc::now()),
                additions: Some(1),
                deletions: Some(1),
                changed_files: Some(1),
                touched_paths_hint: vec!["secret/path".into()],
                window: None,
            }),
            tags: vec![],
            links: vec![Link { label: "pr".into(), url: "https://github.com/secret".into() }],
            source: SourceRef { system: SourceSystem::Github, url: Some("https://api.github.com/secret".into()), opaque_id: None },
        };

        let out = r.redact_events(&[ev], "internal").unwrap();
        let json = serde_json::to_string(&out).unwrap();
        
        // All sensitive data should be preserved
        assert!(json.contains(sensitive_title), "Internal profile should preserve title");
        assert!(json.contains(sensitive_repo), "Internal profile should preserve repo");
        assert!(json.contains("https://github.com/secret"), "Internal profile should preserve URLs");
    }

    /// Property test: Manager profile keeps titles but removes sensitive details
    #[test]
    fn manager_profile_keeps_context_but_strips_details() {
        let r = DeterministicRedactor::new(b"test-key");
        let pr_title = "Feature: Add user authentication".to_string();
        
        let ev = EventEnvelope {
            id: EventId::from_parts(["x","1"]),
            kind: EventKind::PullRequest,
            occurred_at: Utc::now(),
            actor: Actor { login: "a".into(), id: None },
            repo: RepoRef { full_name: "myorg/auth-service".into(), html_url: Some("https://github.com/myorg/auth-service".into()), visibility: RepoVisibility::Private },
            payload: EventPayload::PullRequest(PullRequestEvent {
                number: 42,
                title: pr_title.clone(),
                state: PullRequestState::Merged,
                created_at: Utc::now(),
                merged_at: Some(Utc::now()),
                additions: Some(100),
                deletions: Some(50),
                changed_files: Some(5),
                touched_paths_hint: vec!["src/auth/internal.rs".into(), "src/secrets.rs".into()],
                window: None,
            }),
            tags: vec![],
            links: vec![Link { label: "pr".into(), url: "https://github.com/myorg/auth-service/pull/42".into() }],
            source: SourceRef { system: SourceSystem::Github, url: Some("https://api.github.com/...".into()), opaque_id: None },
        };

        let out = r.redact_events(&[ev], "manager").unwrap();
        
        // Title should be preserved
        match &out[0].payload {
            EventPayload::PullRequest(pr) => {
                assert_eq!(pr.title, pr_title);
                // But touched_paths should be cleared
                assert!(pr.touched_paths_hint.is_empty(), "touched_paths_hint should be cleared in manager view");
            }
            _ => panic!("expected pr"),
        }
        
        // Links should be stripped
        assert!(out[0].links.is_empty(), "links should be stripped in manager view");
        
        // Repo and source URL should be preserved
        assert_eq!(out[0].repo.full_name, "myorg/auth-service");
        assert!(out[0].source.url.is_some());
    }

    /// Property test: Manager profile handles manual events correctly
    #[test]
    fn manager_profile_handles_manual_events() {
        use chrono::NaiveDate;
        
        let r = DeterministicRedactor::new(b"test-key");
        
        let ev = EventEnvelope {
            id: EventId::from_parts(["x","1"]),
            kind: EventKind::Manual,
            occurred_at: Utc::now(),
            actor: Actor { login: "a".into(), id: None },
            repo: RepoRef { full_name: "o/r".into(), html_url: None, visibility: RepoVisibility::Private },
            payload: EventPayload::Manual(ManualEvent {
                event_type: ManualEventType::Incident,
                title: "Database outage resolution".into(),
                description: Some("Detailed technical description of the fix".into()),
                impact: Some("Affected 1000 users for 5 minutes".into()),
                started_at: Some(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap()),
                ended_at: Some(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap()),
            }),
            tags: vec![],
            links: vec![Link { label: "runbook".into(), url: "https://wiki.internal/runbook".into() }],
            source: SourceRef { system: SourceSystem::Manual, url: None, opaque_id: None },
        };

        let out = r.redact_events(&[ev], "manager").unwrap();
        
        match &out[0].payload {
            EventPayload::Manual(m) => {
                // Title should be preserved
                assert_eq!(m.title, "Database outage resolution");
                // Description and impact should be removed
                assert!(m.description.is_none(), "description should be removed in manager view");
                assert!(m.impact.is_none(), "impact should be removed in manager view");
            }
            _ => panic!("expected manual event"),
        }
        
        // Links should be stripped
        assert!(out[0].links.is_empty());
    }

    /// Property test: Manager profile handles workstreams
    #[test]
    fn manager_profile_handles_workstreams() {
        use shiplog_schema::workstream::WorkstreamStats;
        use shiplog_ids::WorkstreamId;
        
        let r = DeterministicRedactor::new(b"test-key");
        
        let ws = Workstream {
            id: WorkstreamId::from_parts(["ws", "test"]),
            title: "Authentication Service Improvements".into(),
            summary: Some("Internal details about security architecture".into()),
            tags: vec!["security".into(), "backend".into(), "repo".into()],
            stats: WorkstreamStats::zero(),
            events: vec![],
            receipts: vec![],
        };
        
        let ws_file = WorkstreamsFile {
            workstreams: vec![ws],
            version: 1,
            generated_at: Utc::now(),
        };
        
        let out = r.redact_workstreams(&ws_file, "manager").unwrap();
        
        let ws_out = &out.workstreams[0];
        
        // Title should be preserved (not aliased)
        assert_eq!(ws_out.title, "Authentication Service Improvements");
        
        // Summary should be removed
        assert!(ws_out.summary.is_none(), "summary should be removed in manager view");
        
        // All tags should be preserved (including "repo")
        assert!(ws_out.tags.contains(&"security".into()));
        assert!(ws_out.tags.contains(&"backend".into()));
        assert!(ws_out.tags.contains(&"repo".into()));
    }

    // Property test using proptest: arbitrary strings should not leak through redaction
    proptest! {
        #[test]
        fn prop_sensitive_strings_redacted(
            title in "[a-zA-Z0-9_-]{10,50}",
            repo in r"[a-z0-9_-]+/[a-z0-9_-]+"
        ) {
            let r = DeterministicRedactor::new(b"test-key");
            
            let ev = EventEnvelope {
                id: EventId::from_parts(["x","1"]),
                kind: EventKind::PullRequest,
                occurred_at: Utc::now(),
                actor: Actor { login: "a".into(), id: None },
                repo: RepoRef { full_name: repo.clone(), html_url: None, visibility: RepoVisibility::Private },
                payload: EventPayload::PullRequest(PullRequestEvent {
                    number: 1,
                    title: title.clone(),
                    state: PullRequestState::Merged,
                    created_at: Utc::now(),
                    merged_at: Some(Utc::now()),
                    additions: Some(1),
                    deletions: Some(1),
                    changed_files: Some(1),
                    touched_paths_hint: vec![],
                    window: None,
                }),
                tags: vec![],
                links: vec![],
                source: SourceRef { system: SourceSystem::Github, url: None, opaque_id: None },
            };

            let out = r.redact_events(&[ev], "public").unwrap();
            let json = serde_json::to_string(&out)?;
            
            // Title should be replaced, not preserved
            prop_assert!(!json.contains(&title), "Title '{}' leaked in output", title);
            
            // Repo should be aliased, not preserved
            if !repo.is_empty() {
                prop_assert!(!json.contains(&repo), "Repo '{}' leaked in output", repo);
            }
            
            // Title should be the literal redaction marker
            match &out[0].payload {
                EventPayload::PullRequest(pr) => {
                    prop_assert_eq!(&pr.title, "[redacted]");
                }
                _ => prop_assert!(false, "Expected PR payload"),
            }
        }
    }
}
