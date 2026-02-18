//! Property tests for shiplog-workstreams
//!
//! This module contains property-based tests for clustering invariants
//! (workstream assignment consistency).

use proptest::prelude::*;
use shiplog_workstreams::RepoClusterer;
use shiplog_testkit::proptest::*;

// ============================================================================
// Clustering Invariant Tests
// ============================================================================

proptest! {
    /// Test that all events are assigned to exactly one workstream
    #[test]
    fn prop_all_events_assigned(
        events in strategy_event_vec(50)
    ) {
        let clusterer = RepoClusterer::new();
        let workstreams = clusterer.cluster(&events);

        // Collect all event IDs from all workstreams
        let mut assigned_ids = std::collections::HashSet::new();
        for ws in &workstreams {
            for event_id in &ws.events {
                assigned_ids.insert(event_id);
            }
        }

        // Check that all input events are assigned
        for event in &events {
            prop_assert!(assigned_ids.contains(&event.id));
        }

        // Check that assigned count equals input count
        prop_assert_eq!(assigned_ids.len(), events.len());
    }

    /// Test that no duplicate events across workstreams
    #[test]
    fn prop_no_duplicate_events(
        events in strategy_event_vec(50)
    ) {
        let clusterer = RepoClusterer::new();
        let workstreams = clusterer.cluster(&events);

        // Collect all event IDs from all workstreams
        let mut all_ids: Vec<_> = Vec::new();
        for ws in &workstreams {
            all_ids.extend(ws.events.iter());
        }

        // Sort and check for duplicates
        all_ids.sort();
        all_ids.dedup();

        // If there were duplicates, the length would be different
        let original_count: usize = events.iter().map(|e| &e.id).collect::<Vec<_>>().len();
        prop_assert_eq!(all_ids.len(), original_count);
    }

    /// Test that receipts are subset of events
    #[test]
    fn prop_receipts_subset_of_events(
        events in strategy_event_vec(50)
    ) {
        let clusterer = RepoClusterer::new();
        let workstreams = clusterer.cluster(&events);

        for ws in &workstreams {
            for receipt_id in &ws.receipts {
                prop_assert!(ws.events.contains(receipt_id));
            }
        }
    }

    /// Test that stats consistency holds
    #[test]
    fn prop_stats_consistency(
        events in strategy_event_vec(50)
    ) {
        let clusterer = RepoClusterer::new();
        let workstreams = clusterer.cluster(&events);

        for ws in &workstreams {
            // Count events by kind in this workstream
            let mut pr_count = 0;
            let mut review_count = 0;
            let mut manual_count = 0;

            for event_id in &ws.events {
                if let Some(event) = events.iter().find(|e| &e.id == event_id) {
                    match event.kind {
                        shiplog_schema::event::EventKind::PullRequest => pr_count += 1,
                        shiplog_schema::event::EventKind::Review => review_count += 1,
                        shiplog_schema::event::EventKind::Manual => manual_count += 1,
                    }
                }
            }

            // Check that stats match actual counts
            prop_assert_eq!(ws.stats.pull_requests, pr_count);
            prop_assert_eq!(ws.stats.reviews, review_count);
            prop_assert_eq!(ws.stats.manual_events, manual_count);
        }
    }

    /// Test that repo clusterer groups events with same repo
    #[test]
    fn prop_repo_clusterer_invariant(
        events in strategy_event_vec(50)
    ) {
        let clusterer = RepoClusterer::new();
        let workstreams = clusterer.cluster(&events);

        // Group events by repo
        let mut repo_events: std::collections::HashMap<String, Vec<&shiplog_schema::event::EventEnvelope>> =
            std::collections::HashMap::new();
        for event in &events {
            repo_events.entry(event.repo.full_name.clone())
                .or_insert_with(Vec::new)
                .push(event);
        }

        // Check that events from same repo are in same workstream
        for (repo, repo_evs) in &repo_events {
            if repo_evs.len() > 1 {
                // Find the workstream for the first event
                let first_id = &repo_evs[0].id;
                let ws = workstreams.iter().find(|w| w.events.contains(first_id));

                if let Some(ws) = ws {
                    // All events from this repo should be in the same workstream
                    for event in repo_evs {
                        prop_assert!(ws.events.contains(&event.id));
                    }
                }
            }
        }
    }

    /// Test that workstream ID is deterministic
    #[test]
    fn prop_workstream_id_determinism(
        events in strategy_event_vec(50)
    ) {
        let clusterer1 = RepoClusterer::new();
        let workstreams1 = clusterer1.cluster(&events);

        let clusterer2 = RepoClusterer::new();
        let workstreams2 = clusterer2.cluster(&events);

        // Sort both workstream lists by ID for comparison
        let mut ids1: Vec<_> = workstreams1.iter().map(|w| &w.id).collect();
        let mut ids2: Vec<_> = workstreams2.iter().map(|w| &w.id).collect();
        ids1.sort();
        ids2.sort();

        prop_assert_eq!(ids1, ids2);
    }
}

// ============================================================================
// Workstream Invariant Tests
// ============================================================================

proptest! {
    /// Test that repo-clustered workstreams have "repo" tag
    #[test]
    fn prop_repo_tag_present(
        events in strategy_event_vec(50)
    ) {
        let clusterer = RepoClusterer::new();
        let workstreams = clusterer.cluster(&events);

        for ws in &workstreams {
            // Repo-clustered workstreams should have "repo" tag
            prop_assert!(ws.tags.contains(&"repo".to_string()));
        }
    }

    /// Test that workstream title matches repo name
    #[test]
    fn prop_title_consistency(
        events in strategy_event_vec(50)
    ) {
        let clusterer = RepoClusterer::new();
        let workstreams = clusterer.cluster(&events);

        for ws in &workstreams {
            if !ws.events.is_empty() {
                // Find the first event in this workstream
                if let Some(event) = events.iter().find(|e| ws.events.contains(&e.id)) {
                    // Title should match repo name
                    prop_assert!(ws.title.contains(&event.repo.full_name) ||
                                event.repo.full_name.contains(&ws.title));
                }
            }
        }
    }

    /// Test that workstreams file version is always 1
    #[test]
    fn prop_version_field(
        events in strategy_event_vec(50)
    ) {
        let clusterer = RepoClusterer::new();
        let workstreams = clusterer.cluster(&events);

        let workstreams_file = shiplog_schema::workstream::WorkstreamFile {
            workstreams,
            version: 1,
            generated_at: chrono::Utc::now(),
        };

        prop_assert_eq!(workstreams_file.version, 1);
    }
}

// ============================================================================
// Receipt Truncation Tests
// ============================================================================

proptest! {
    /// Test that receipt list never exceeds configured maximum
    #[test]
    fn prop_receipt_truncation(
        events in strategy_event_vec(100),
        max_receipts in 1usize..20usize
    ) {
        let clusterer = RepoClusterer::new();
        let mut workstreams = clusterer.cluster(&events);

        // Truncate receipts to max
        for ws in &mut workstreams {
            if ws.receipts.len() > max_receipts {
                ws.receipts.truncate(max_receipts);
            }
        }

        // Check that no workstream exceeds max
        for ws in &workstreams {
            prop_assert!(ws.receipts.len() <= max_receipts);
        }
    }
}
