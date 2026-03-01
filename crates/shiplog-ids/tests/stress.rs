//! Multi-threaded stress tests for concurrent ID generation.

use shiplog_ids::{EventId, RunId, WorkstreamId};
use std::collections::HashSet;

#[test]
fn concurrent_event_id_determinism() {
    let results: Vec<Vec<EventId>> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..8)
            .map(|_| {
                s.spawn(|| {
                    (0..200)
                        .map(|i| {
                            EventId::from_parts(["github", "pr", "owner/repo", &i.to_string()])
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });

    // Every thread must produce identical IDs for identical inputs.
    for thread_result in &results[1..] {
        assert_eq!(&results[0], thread_result);
    }
}

#[test]
fn concurrent_workstream_id_determinism() {
    let results: Vec<Vec<WorkstreamId>> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..8)
            .map(|_| {
                s.spawn(|| {
                    (0..200)
                        .map(|i| WorkstreamId::from_parts(["repo", &format!("org/repo-{i}")]))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });

    for thread_result in &results[1..] {
        assert_eq!(&results[0], thread_result);
    }
}

#[test]
fn concurrent_event_ids_unique_across_inputs() {
    let all_ids: Vec<EventId> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..8)
            .map(|t| {
                s.spawn(move || {
                    (0..200)
                        .map(|i| EventId::from_parts(["stress", &format!("t{t}"), &i.to_string()]))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect()
    });

    let unique: HashSet<&str> = all_ids.iter().map(|id| id.0.as_str()).collect();
    assert_eq!(
        unique.len(),
        8 * 200,
        "all IDs from distinct inputs must be unique"
    );
}

#[test]
fn concurrent_run_id_uniqueness() {
    let all_ids: Vec<RunId> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..8)
            .map(|_| s.spawn(|| (0..100).map(|_| RunId::now("stress")).collect::<Vec<_>>()))
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect()
    });

    let unique: HashSet<&str> = all_ids.iter().map(|id| id.0.as_str()).collect();
    let total = 8 * 100;
    // Nanosecond-based IDs may collide under contention, but most should be unique.
    assert!(
        unique.len() > total / 2,
        "expected majority unique RunIds, got {}/{total}",
        unique.len()
    );
}
