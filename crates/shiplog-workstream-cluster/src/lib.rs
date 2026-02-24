//! Repo-based workstream clustering strategy.
//!
//! This crate intentionally has a single responsibility:
//! assign events to workstream buckets based on repository and build
//! deterministic workstream IDs, stats, and receipt ordering.

use anyhow::Result;
use chrono::Utc;
use shiplog_ids::WorkstreamId;
use shiplog_ports::WorkstreamClusterer;
use shiplog_schema::event::EventEnvelope;
use shiplog_schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};
use shiplog_workstream_receipt_policy::{
    should_include_cluster_receipt, truncate_cluster_receipts,
};
use std::collections::BTreeMap;

/// Default clustering strategy for shiplog.
///
/// - Group events by repository name.
/// - Build canonical workstream titles/ids/stats.
/// - Provide compact receipt lists by event kind.
pub struct RepoClusterer;

impl WorkstreamClusterer for RepoClusterer {
    fn cluster(&self, events: &[EventEnvelope]) -> Result<WorkstreamsFile> {
        let mut by_repo: BTreeMap<String, Vec<&EventEnvelope>> = BTreeMap::new();
        for ev in events {
            by_repo
                .entry(ev.repo.full_name.clone())
                .or_default()
                .push(ev);
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
                if should_include_cluster_receipt(&ev.kind, ws.receipts.len()) {
                    ws.receipts.push(ev.id.clone());
                }
            }
            truncate_cluster_receipts(&mut ws.receipts);

            workstreams.push(ws);
        }

        Ok(WorkstreamsFile {
            version: 1,
            generated_at: Utc::now(),
            workstreams,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use shiplog_ids::EventId;
    use shiplog_schema::event::*;
    use shiplog_workstream_receipt_policy::{
        WORKSTREAM_RECEIPT_LIMIT_MANUAL, WORKSTREAM_RECEIPT_LIMIT_REVIEW,
        WORKSTREAM_RECEIPT_LIMIT_TOTAL,
    };

    fn make_event(repo_name: &str, event_id: &str, number: u64, kind: EventKind) -> EventEnvelope {
        EventEnvelope {
            id: EventId::from_parts(["x", event_id]),
            kind: kind.clone(),
            occurred_at: Utc::now(),
            actor: Actor {
                login: "actor".into(),
                id: None,
            },
            repo: RepoRef {
                full_name: repo_name.into(),
                html_url: Some(format!("https://example.com/{repo_name}")),
                visibility: RepoVisibility::Unknown,
            },
            payload: match kind {
                EventKind::PullRequest => EventPayload::PullRequest(PullRequestEvent {
                    number,
                    title: "PR event".into(),
                    state: PullRequestState::Merged,
                    created_at: Utc::now(),
                    merged_at: Some(Utc::now()),
                    additions: Some(1),
                    deletions: Some(0),
                    changed_files: Some(1),
                    touched_paths_hint: vec![],
                    window: None,
                }),
                EventKind::Review => EventPayload::Review(ReviewEvent {
                    pull_number: number,
                    pull_title: "Review target".into(),
                    submitted_at: Utc::now(),
                    state: "approved".into(),
                    window: None,
                }),
                EventKind::Manual => EventPayload::Manual(ManualEvent {
                    event_type: ManualEventType::Note,
                    title: "Manual".into(),
                    description: None,
                    started_at: None,
                    ended_at: None,
                    impact: None,
                }),
            },
            tags: vec![],
            links: vec![],
            source: SourceRef {
                system: SourceSystem::Unknown,
                url: None,
                opaque_id: None,
            },
        }
    }

    #[test]
    fn clusters_by_repo() {
        let events = vec![
            make_event("repo/a", "1", 1, EventKind::PullRequest),
            make_event("repo/b", "2", 2, EventKind::PullRequest),
        ];

        let ws = RepoClusterer.cluster(&events).unwrap();
        assert_eq!(ws.workstreams.len(), 2);
        assert!(ws.workstreams[0].title == "repo/a" || ws.workstreams[1].title == "repo/a");
    }

    #[test]
    fn review_receipts_are_capped_at_5() {
        let events = (0..10)
            .map(|i| make_event("repo/reviews", &format!("r{i}"), i, EventKind::Review))
            .collect::<Vec<_>>();
        let ws = RepoClusterer.cluster(&events).unwrap();
        assert_eq!(ws.workstreams.len(), 1);
        assert_eq!(
            ws.workstreams[0].receipts.len(),
            WORKSTREAM_RECEIPT_LIMIT_REVIEW
        );
    }

    #[test]
    fn manual_receipts_are_capped_at_7_before_truncation() {
        let events = (0..10)
            .map(|i| make_event("repo/manuals", &format!("m{i}"), i, EventKind::Manual))
            .collect::<Vec<_>>();
        let ws = RepoClusterer.cluster(&events).unwrap();
        assert_eq!(ws.workstreams.len(), 1);
        assert_eq!(
            ws.workstreams[0].receipts.len(),
            WORKSTREAM_RECEIPT_LIMIT_MANUAL.min(WORKSTREAM_RECEIPT_LIMIT_TOTAL)
        );
    }

    #[test]
    fn deterministic_ids_same_inputs() {
        let events = vec![
            make_event("repo/deterministic", "a", 1, EventKind::PullRequest),
            make_event("repo/deterministic", "b", 2, EventKind::PullRequest),
        ];

        let a = RepoClusterer.cluster(&events).unwrap();
        let b = RepoClusterer.cluster(&events).unwrap();
        assert_eq!(
            a.workstreams[0].id.to_string(),
            b.workstreams[0].id.to_string()
        );
    }
}
