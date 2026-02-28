//! Determinism tests for shiplog-workstreams.
//!
//! Verifies that clustering is fully reproducible: same events always produce
//! the same workstream grouping and ordering.

use shiplog_ports::WorkstreamClusterer;
use shiplog_workstreams::RepoClusterer;

const ITERATIONS: usize = 100;

fn sample_events() -> Vec<shiplog_schema::event::EventEnvelope> {
    vec![
        shiplog_testkit::pr_event("acme/frontend", 1, "Add login page"),
        shiplog_testkit::pr_event("acme/frontend", 2, "Fix CSS layout"),
        shiplog_testkit::pr_event("acme/backend", 3, "Add REST API"),
        shiplog_testkit::pr_event("acme/backend", 4, "Fix auth middleware"),
        shiplog_testkit::pr_event("acme/infra", 5, "Setup CI pipeline"),
        shiplog_testkit::pr_event("acme/frontend", 6, "Add signup form"),
        shiplog_testkit::pr_event("acme/backend", 7, "Add rate limiter"),
    ]
}

// ---------------------------------------------------------------------------
// Same events → same workstream clustering across iterations
// ---------------------------------------------------------------------------

#[test]
fn clustering_deterministic_across_100_iterations() {
    let events = sample_events();
    let reference = RepoClusterer.cluster(&events).unwrap();

    for _ in 0..ITERATIONS {
        let result = RepoClusterer.cluster(&events).unwrap();
        assert_eq!(
            reference.workstreams.len(),
            result.workstreams.len(),
            "workstream count must be stable"
        );
        for (a, b) in reference.workstreams.iter().zip(result.workstreams.iter()) {
            assert_eq!(a.id, b.id, "workstream ID must be stable");
            assert_eq!(a.title, b.title, "workstream title must be stable");
            assert_eq!(a.events.len(), b.events.len(), "event count must be stable");
            assert_eq!(a.stats, b.stats, "stats must be stable");
            assert_eq!(a.tags, b.tags, "tags must be stable");
            assert_eq!(
                a.receipts.len(),
                b.receipts.len(),
                "receipt count must be stable"
            );
        }
    }
}

#[test]
fn clustering_event_assignment_deterministic() {
    let events = sample_events();
    let reference = RepoClusterer.cluster(&events).unwrap();

    for _ in 0..ITERATIONS {
        let result = RepoClusterer.cluster(&events).unwrap();
        for (a, b) in reference.workstreams.iter().zip(result.workstreams.iter()) {
            assert_eq!(
                a.events, b.events,
                "event assignment within workstream '{}' must be stable",
                a.title
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Same events → same workstream ordering
// ---------------------------------------------------------------------------

#[test]
fn workstream_ordering_deterministic_across_100_iterations() {
    let events = sample_events();
    let reference = RepoClusterer.cluster(&events).unwrap();
    let ref_order: Vec<String> = reference
        .workstreams
        .iter()
        .map(|w| w.title.clone())
        .collect();

    for _ in 0..ITERATIONS {
        let result = RepoClusterer.cluster(&events).unwrap();
        let order: Vec<String> = result
            .workstreams
            .iter()
            .map(|w| w.title.clone())
            .collect();
        assert_eq!(
            ref_order, order,
            "workstream ordering must be stable across calls"
        );
    }
}

#[test]
fn event_ordering_within_workstreams_deterministic() {
    let events = sample_events();
    let reference = RepoClusterer.cluster(&events).unwrap();

    for _ in 0..ITERATIONS {
        let result = RepoClusterer.cluster(&events).unwrap();
        for (a, b) in reference.workstreams.iter().zip(result.workstreams.iter()) {
            let a_ids: Vec<String> = a.events.iter().map(|e| e.to_string()).collect();
            let b_ids: Vec<String> = b.events.iter().map(|e| e.to_string()).collect();
            assert_eq!(
                a_ids, b_ids,
                "event order within workstream '{}' must be stable",
                a.title
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Many repos: determinism at scale
// ---------------------------------------------------------------------------

#[test]
fn clustering_deterministic_with_many_repos() {
    let events: Vec<_> = (0..100)
        .map(|i| {
            shiplog_testkit::pr_event(
                &format!("org/repo-{}", i % 15),
                i as u64,
                &format!("PR #{i}"),
            )
        })
        .collect();

    let reference = RepoClusterer.cluster(&events).unwrap();
    for _ in 0..10 {
        let result = RepoClusterer.cluster(&events).unwrap();
        assert_eq!(reference.workstreams.len(), result.workstreams.len());
        for (a, b) in reference.workstreams.iter().zip(result.workstreams.iter()) {
            assert_eq!(a.id, b.id);
            assert_eq!(a.title, b.title);
            assert_eq!(a.events, b.events);
        }
    }
}

// ---------------------------------------------------------------------------
// Empty and single-event edge cases
// ---------------------------------------------------------------------------

#[test]
fn empty_events_deterministic() {
    let reference = RepoClusterer.cluster(&[]).unwrap();
    for _ in 0..ITERATIONS {
        let result = RepoClusterer.cluster(&[]).unwrap();
        assert_eq!(reference.workstreams.len(), result.workstreams.len());
    }
}

#[test]
fn single_event_deterministic() {
    let events = vec![shiplog_testkit::pr_event("acme/solo", 1, "Only PR")];
    let reference = RepoClusterer.cluster(&events).unwrap();

    for _ in 0..ITERATIONS {
        let result = RepoClusterer.cluster(&events).unwrap();
        assert_eq!(reference.workstreams.len(), 1);
        assert_eq!(result.workstreams.len(), 1);
        assert_eq!(reference.workstreams[0].id, result.workstreams[0].id);
        assert_eq!(reference.workstreams[0].title, result.workstreams[0].title);
    }
}
