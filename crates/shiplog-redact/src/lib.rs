use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use shiplog_ports::Redactor;
use shiplog_schema::event::{EventEnvelope, EventPayload, RepoRef, RepoVisibility, SourceRef};
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
                // "manager" is still internal to the org. Keep receipts usable.
                // In a real org-mode, you'd have policies here. MVP keeps it identical.
                events.to_vec()
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
            RedactionProfile::Manager => workstreams.clone(),
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
}
