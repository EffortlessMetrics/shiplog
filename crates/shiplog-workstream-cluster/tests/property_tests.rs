//! Property tests for repo clustering invariants.

use proptest::prelude::*;
use shiplog_ports::WorkstreamClusterer;
use shiplog_schema::event::EventEnvelope;
use shiplog_testkit::proptest::*;
use shiplog_workstream_cluster::RepoClusterer;
use shiplog_workstream_receipt_policy::WORKSTREAM_RECEIPT_LIMIT_TOTAL;

fn clustered_workstreams(events: &[EventEnvelope]) -> Vec<shiplog_schema::workstream::Workstream> {
    RepoClusterer.cluster(events).unwrap().workstreams
}

proptest! {
    #[test]
    fn prop_all_events_assigned(events in strategy_event_vec(50)) {
        let workstreams = clustered_workstreams(&events);
        let mut assigned_ids = std::collections::HashSet::new();

        for ws in &workstreams {
            for event_id in &ws.events {
                assigned_ids.insert(event_id.clone());
            }
        }

        for event in &events {
            prop_assert!(assigned_ids.contains(&event.id));
        }
        prop_assert_eq!(assigned_ids.len(), events.len());
    }

    #[test]
    fn prop_no_duplicate_events(events in strategy_event_vec(50)) {
        let workstreams = clustered_workstreams(&events);
        let mut all_ids: Vec<_> = Vec::new();
        for ws in &workstreams {
            all_ids.extend(ws.events.iter());
        }
        let unique_count: usize = all_ids
            .iter()
            .map(|id| id.to_string())
            .collect::<std::collections::HashSet<_>>()
            .len();
        prop_assert_eq!(unique_count, events.len());
    }

    #[test]
    fn prop_receipts_subset_of_events(events in strategy_event_vec(50)) {
        let workstreams = clustered_workstreams(&events);
        for ws in &workstreams {
            for receipt_id in &ws.receipts {
                prop_assert!(ws.events.contains(receipt_id));
            }
        }
    }

    #[test]
    fn prop_repo_clusterer_invariant(events in strategy_event_vec(50)) {
        let workstreams = clustered_workstreams(&events);
        let mut repo_to_ws = std::collections::HashMap::new();

        for event in events.iter() {
            let ws = workstreams
                .iter()
                .position(|ws| ws.events.iter().any(|id| id == &event.id))
                .expect("every event must be assigned");
            if let Some(&existing) = repo_to_ws.get(&event.repo.full_name) {
                prop_assert_eq!(existing, ws, "same repo must share workstream");
            } else {
                repo_to_ws.insert(event.repo.full_name.clone(), ws);
            }
        }
    }

    #[test]
    fn prop_receipts_never_exceed_total_limit(events in strategy_event_vec(100)) {
        let workstreams = clustered_workstreams(&events);
        for ws in &workstreams {
            prop_assert!(ws.receipts.len() <= WORKSTREAM_RECEIPT_LIMIT_TOTAL);
        }
    }

    #[test]
    fn prop_stats_sum_matches_event_count(events in strategy_event_vec(50)) {
        let workstreams = clustered_workstreams(&events);
        for ws in &workstreams {
            let stat_total = ws.stats.pull_requests + ws.stats.reviews + ws.stats.manual_events;
            prop_assert_eq!(stat_total, ws.events.len());
        }
    }

    #[test]
    fn prop_all_workstreams_tagged_repo(events in strategy_event_vec(50)) {
        let workstreams = clustered_workstreams(&events);
        for ws in &workstreams {
            prop_assert!(ws.tags.contains(&"repo".to_string()));
        }
    }

    #[test]
    fn prop_workstream_count_equals_distinct_repos(events in strategy_event_vec(50)) {
        let workstreams = clustered_workstreams(&events);
        let distinct_repos: std::collections::HashSet<_> =
            events.iter().map(|e| &e.repo.full_name).collect();
        prop_assert_eq!(workstreams.len(), distinct_repos.len());
    }

    #[test]
    fn prop_deterministic_clustering(events in strategy_event_vec(30)) {
        let a = RepoClusterer.cluster(&events).unwrap();
        let b = RepoClusterer.cluster(&events).unwrap();
        prop_assert_eq!(a.workstreams.len(), b.workstreams.len());
        for (wa, wb) in a.workstreams.iter().zip(b.workstreams.iter()) {
            prop_assert_eq!(&wa.id, &wb.id);
            prop_assert_eq!(&wa.title, &wb.title);
            prop_assert_eq!(&wa.events, &wb.events);
            prop_assert_eq!(&wa.receipts, &wb.receipts);
            prop_assert_eq!(&wa.stats, &wb.stats);
        }
    }
}
