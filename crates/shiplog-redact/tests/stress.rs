//! Multi-threaded stress tests for concurrent redaction operations.

use shiplog_ports::Redactor;
use shiplog_redact::DeterministicRedactor;
use std::collections::HashSet;
use std::sync::Arc;

#[test]
fn concurrent_event_redaction_consistency() {
    let redactor = Arc::new(DeterministicRedactor::new(b"stress-test-key"));
    let events: Vec<_> = (0..10)
        .map(|i| shiplog_testkit::pr_event(&format!("org/repo-{i}"), i as u64, &format!("PR {i}")))
        .collect();

    let results: Vec<Vec<_>> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let r = Arc::clone(&redactor);
                let evs = events.clone();
                s.spawn(move || r.redact_events(&evs, "public").unwrap())
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });

    // All threads must produce identical redacted output.
    let reference = &results[0];
    for result in &results[1..] {
        assert_eq!(reference.len(), result.len());
        for (a, b) in reference.iter().zip(result.iter()) {
            assert_eq!(a.id, b.id);
            assert_eq!(a.repo.full_name, b.repo.full_name);
            assert_eq!(a.kind, b.kind);
        }
    }
}

#[test]
fn concurrent_redaction_all_profiles() {
    let redactor = Arc::new(DeterministicRedactor::new(b"multi-profile-key"));
    let events = vec![
        shiplog_testkit::pr_event("org/secret", 1, "Secret PR"),
        shiplog_testkit::pr_event("org/public", 2, "Public PR"),
    ];

    for profile in &["internal", "manager", "public"] {
        let results: Vec<Vec<_>> = std::thread::scope(|s| {
            let handles: Vec<_> = (0..4)
                .map(|_| {
                    let r = Arc::clone(&redactor);
                    let evs = events.clone();
                    let p = *profile;
                    s.spawn(move || r.redact_events(&evs, p).unwrap())
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });

        for result in &results[1..] {
            assert_eq!(results[0].len(), result.len());
            for (a, b) in results[0].iter().zip(result.iter()) {
                assert_eq!(a.repo.full_name, b.repo.full_name);
            }
        }
    }
}

#[test]
fn concurrent_alias_generation_stable() {
    let redactor = Arc::new(DeterministicRedactor::new(b"alias-key"));
    let events: Vec<_> = (0..5)
        .map(|i| shiplog_testkit::pr_event("same/repo", i as u64 + 1, &format!("PR {i}")))
        .collect();

    let results: Vec<Vec<_>> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let r = Arc::clone(&redactor);
                let evs = events.clone();
                s.spawn(move || r.redact_events(&evs, "public").unwrap())
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });

    // All repo aliases must be identical (deterministic aliasing).
    let repo_names: HashSet<_> = results
        .iter()
        .flat_map(|r| r.iter().map(|e| e.repo.full_name.clone()))
        .collect();
    assert_eq!(
        repo_names.len(),
        1,
        "same repo should always alias to the same value"
    );
}
