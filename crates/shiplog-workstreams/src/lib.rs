use anyhow::{Context, Result};
use chrono::Utc;
use shiplog_ids::WorkstreamId;
use shiplog_ports::WorkstreamClusterer;
use shiplog_schema::event::{EventEnvelope, EventKind};
use shiplog_schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};
use std::collections::BTreeMap;

/// Default clustering strategy:
/// - group by repo
///
/// It's intentionally boring. It gives the user a stable starting point.
/// A later LLM-assisted clusterer can sit behind the same port.
pub struct RepoClusterer;

impl WorkstreamClusterer for RepoClusterer {
    fn cluster(&self, events: &[EventEnvelope]) -> Result<WorkstreamsFile> {
        let mut by_repo: BTreeMap<String, Vec<&EventEnvelope>> = BTreeMap::new();
        for ev in events {
            by_repo.entry(ev.repo.full_name.clone()).or_default().push(ev);
        }

        let mut workstreams = Vec::new();
        for (repo, evs) in by_repo {
            let id = WorkstreamId::from_parts(["repo", &repo]);
            let mut ws = Workstream {
                id,
                title: repo.clone(),
                summary: None,
                tags: vec!["repo".to_string()],
                stats: WorkstreamStats::zero(),
                events: vec![],
                receipts: vec![],
            };

            for ev in evs {
                ws.events.push(ev.id.clone());
                ws.bump_stats(&ev.kind);

                // simple receipt heuristic: keep PRs, plus reviews on top repos.
                match ev.kind {
                    EventKind::PullRequest => ws.receipts.push(ev.id.clone()),
                    EventKind::Review => {
                        if ws.receipts.len() < 5 {
                            ws.receipts.push(ev.id.clone())
                        }
                    }
                }
            }

            // avoid enormous receipt lists in the packet.
            ws.receipts.truncate(10);

            workstreams.push(ws);
        }

        Ok(WorkstreamsFile {
            version: 1,
            generated_at: Utc::now(),
            workstreams,
        })
    }
}

/// Load an existing workstreams.yaml if present, otherwise generate.
///
/// This is a critical design point: workstreams are user-owned.
/// The tool can suggest structure, but shouldn't trap the user.
pub fn load_or_cluster(
    maybe_yaml: Option<&std::path::Path>,
    clusterer: &dyn WorkstreamClusterer,
    events: &[EventEnvelope],
) -> Result<WorkstreamsFile> {
    if let Some(path) = maybe_yaml {
        if path.exists() {
            let text = std::fs::read_to_string(path)
                .with_context(|| format!("read workstreams from {path:?}"))?;
            let ws: WorkstreamsFile = serde_yaml::from_str(&text)
                .with_context(|| format!("parse workstreams yaml {path:?}"))?;
            return Ok(ws);
        }
    }

    clusterer.cluster(events)
}

#[cfg(test)]
mod tests {
    use super::*;
    use shiplog_schema::event::*;
    use shiplog_ids::EventId;

    #[test]
    fn clusters_by_repo() {
        let ev1 = EventEnvelope {
            id: EventId::from_parts(["x", "1"]),
            kind: EventKind::PullRequest,
            occurred_at: Utc::now(),
            actor: Actor { login: "a".into(), id: None },
            repo: RepoRef {
                full_name: "o/r1".into(),
                html_url: Some("https://github.com/o/r1".into()),
                visibility: RepoVisibility::Unknown,
            },
            payload: EventPayload::PullRequest(PullRequestEvent{
                number: 1,
                title: "t".into(),
                state: PullRequestState::Merged,
                created_at: Utc::now(),
                merged_at: Some(Utc::now()),
                additions: Some(1),
                deletions: Some(0),
                changed_files: Some(1),
                touched_paths_hint: vec![],
                window: None,
            }),
            tags: vec![],
            links: vec![],
            source: SourceRef { system: SourceSystem::Unknown, url: None, opaque_id: None },
        };

        let ev2 = EventEnvelope { repo: RepoRef { full_name: "o/r2".into(), ..ev1.repo.clone() }, id: EventId::from_parts(["x","2"]), ..ev1.clone() };
        let ws = RepoClusterer.cluster(&[ev1, ev2]).unwrap();
        assert_eq!(ws.workstreams.len(), 2);
    }
}
