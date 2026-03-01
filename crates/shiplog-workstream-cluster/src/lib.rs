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
///
/// # Examples
///
/// Cluster an empty event list:
///
/// ```
/// use shiplog_workstream_cluster::RepoClusterer;
/// use shiplog_ports::WorkstreamClusterer;
///
/// let ws = RepoClusterer.cluster(&[]).unwrap();
/// assert!(ws.workstreams.is_empty());
/// assert_eq!(ws.version, 1);
/// ```
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

    #[test]
    fn empty_events_produce_no_workstreams() {
        let ws = RepoClusterer.cluster(&[]).unwrap();
        assert!(ws.workstreams.is_empty());
        assert_eq!(ws.version, 1);
    }

    #[test]
    fn single_event_produces_one_workstream() {
        let events = vec![make_event("repo/solo", "only", 1, EventKind::PullRequest)];
        let ws = RepoClusterer.cluster(&events).unwrap();
        assert_eq!(ws.workstreams.len(), 1);
        assert_eq!(ws.workstreams[0].title, "repo/solo");
        assert_eq!(ws.workstreams[0].events.len(), 1);
        assert_eq!(ws.workstreams[0].receipts.len(), 1);
    }

    #[test]
    fn stats_are_bumped_correctly_for_mixed_kinds() {
        let events = vec![
            make_event("repo/mix", "pr1", 1, EventKind::PullRequest),
            make_event("repo/mix", "pr2", 2, EventKind::PullRequest),
            make_event("repo/mix", "rev1", 3, EventKind::Review),
            make_event("repo/mix", "man1", 4, EventKind::Manual),
            make_event("repo/mix", "man2", 5, EventKind::Manual),
            make_event("repo/mix", "man3", 6, EventKind::Manual),
        ];
        let ws = RepoClusterer.cluster(&events).unwrap();
        assert_eq!(ws.workstreams.len(), 1);
        let stats = &ws.workstreams[0].stats;
        assert_eq!(stats.pull_requests, 2);
        assert_eq!(stats.reviews, 1);
        assert_eq!(stats.manual_events, 3);
    }

    #[test]
    fn all_workstreams_tagged_with_repo() {
        let events = vec![
            make_event("repo/a", "1", 1, EventKind::PullRequest),
            make_event("repo/b", "2", 2, EventKind::Review),
            make_event("repo/c", "3", 3, EventKind::Manual),
        ];
        let ws = RepoClusterer.cluster(&events).unwrap();
        for w in &ws.workstreams {
            assert!(w.tags.contains(&"repo".to_string()));
        }
    }

    #[test]
    fn workstreams_are_sorted_by_repo_name() {
        let events = vec![
            make_event("repo/zulu", "z1", 1, EventKind::PullRequest),
            make_event("repo/alpha", "a1", 2, EventKind::PullRequest),
            make_event("repo/mike", "m1", 3, EventKind::PullRequest),
        ];
        let ws = RepoClusterer.cluster(&events).unwrap();
        let titles: Vec<_> = ws.workstreams.iter().map(|w| &w.title).collect();
        assert_eq!(titles, vec!["repo/alpha", "repo/mike", "repo/zulu"]);
    }

    #[test]
    fn pr_receipts_are_capped_by_total_limit() {
        let events = (0..20)
            .map(|i| make_event("repo/prs", &format!("p{i}"), i, EventKind::PullRequest))
            .collect::<Vec<_>>();
        let ws = RepoClusterer.cluster(&events).unwrap();
        assert_eq!(ws.workstreams[0].events.len(), 20);
        assert_eq!(
            ws.workstreams[0].receipts.len(),
            WORKSTREAM_RECEIPT_LIMIT_TOTAL
        );
    }

    #[test]
    fn different_repos_produce_different_ids() {
        let events = vec![
            make_event("repo/a", "1", 1, EventKind::PullRequest),
            make_event("repo/b", "2", 2, EventKind::PullRequest),
        ];
        let ws = RepoClusterer.cluster(&events).unwrap();
        assert_ne!(
            ws.workstreams[0].id.to_string(),
            ws.workstreams[1].id.to_string()
        );
    }
}
