//! Multi-threaded stress tests for concurrent workstream clustering.

use shiplog_ports::WorkstreamClusterer;
use shiplog_workstreams::RepoClusterer;

#[test]
fn concurrent_clustering_determinism() {
    let events: Vec<_> = (0..20)
        .map(|i| {
            shiplog_testkit::pr_event(&format!("org/repo-{}", i % 5), i as u64, &format!("PR {i}"))
        })
        .collect();

    let results: Vec<_> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let evs = &events;
                s.spawn(move || RepoClusterer.cluster(evs).unwrap())
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });

    let reference = &results[0];
    for result in &results[1..] {
        assert_eq!(reference.workstreams.len(), result.workstreams.len());
        for (a, b) in reference.workstreams.iter().zip(result.workstreams.iter()) {
            assert_eq!(a.id.to_string(), b.id.to_string());
            assert_eq!(a.title, b.title);
            assert_eq!(a.events.len(), b.events.len());
            assert_eq!(a.receipts.len(), b.receipts.len());
            assert_eq!(a.stats, b.stats);
        }
    }
}

#[test]
fn concurrent_clustering_workstream_ids_unique() {
    let events: Vec<_> = (0..30)
        .map(|i| {
            shiplog_testkit::pr_event(&format!("org/repo-{}", i % 6), i as u64, &format!("PR {i}"))
        })
        .collect();

    let results: Vec<_> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let evs = &events;
                s.spawn(move || RepoClusterer.cluster(evs).unwrap())
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });

    // Within each result, all workstream IDs must be unique.
    for result in &results {
        let ids: std::collections::HashSet<String> = result
            .workstreams
            .iter()
            .map(|w| w.id.to_string())
            .collect();
        assert_eq!(
            ids.len(),
            result.workstreams.len(),
            "workstream IDs must be unique within a clustering"
        );
    }
}
