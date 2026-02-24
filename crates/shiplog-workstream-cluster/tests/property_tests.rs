//! Property tests for repo clustering invariants.

use proptest::prelude::*;
use shiplog_ports::WorkstreamClusterer;
use shiplog_schema::event::EventEnvelope;
use shiplog_testkit::proptest::*;
use shiplog_workstream_cluster::RepoClusterer;

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
}
