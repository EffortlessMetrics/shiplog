//! Determinism tests for shiplog-redact.
//!
//! Verifies that redaction is fully reproducible: same key + same input always
//! produce the same redacted output, regardless of call order.

use shiplog_ports::Redactor;
use shiplog_redact::DeterministicRedactor;

const ITERATIONS: usize = 100;

fn sample_events() -> Vec<shiplog_schema::event::EventEnvelope> {
    vec![
        shiplog_testkit::pr_event("acme/frontend", 1, "Add login page"),
        shiplog_testkit::pr_event("acme/frontend", 2, "Fix CSS layout"),
        shiplog_testkit::pr_event("acme/backend", 3, "Add REST API"),
        shiplog_testkit::pr_event("secret-org/private-repo", 4, "Sensitive change"),
    ]
}

// ---------------------------------------------------------------------------
// Same key + same input → same output across iterations
// ---------------------------------------------------------------------------

#[test]
fn redact_events_deterministic_across_100_iterations() {
    let redactor = DeterministicRedactor::new(b"determinism-test-key");
    let events = sample_events();

    let reference = redactor.redact_events(&events, "public").unwrap();
    for _ in 0..ITERATIONS {
        let result = redactor.redact_events(&events, "public").unwrap();
        assert_eq!(reference.len(), result.len());
        for (a, b) in reference.iter().zip(result.iter()) {
            assert_eq!(a.id, b.id, "redacted event ID must be stable");
            assert_eq!(
                a.repo.full_name, b.repo.full_name,
                "redacted repo name must be stable"
            );
            assert_eq!(a.kind, b.kind);
        }
    }
}

#[test]
fn redact_events_manager_profile_deterministic() {
    let redactor = DeterministicRedactor::new(b"manager-key");
    let events = sample_events();

    let reference = redactor.redact_events(&events, "manager").unwrap();
    for _ in 0..ITERATIONS {
        let result = redactor.redact_events(&events, "manager").unwrap();
        for (a, b) in reference.iter().zip(result.iter()) {
            assert_eq!(a.repo.full_name, b.repo.full_name);
        }
    }
}

#[test]
fn redact_workstreams_deterministic_across_100_iterations() {
    use chrono::Utc;
    use shiplog_ids::WorkstreamId;
    use shiplog_schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};

    let redactor = DeterministicRedactor::new(b"ws-determinism-key");
    let ws = WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![
            Workstream {
                id: WorkstreamId::from_parts(["repo", "acme/frontend"]),
                title: "acme/frontend".into(),
                summary: Some("Frontend work".into()),
                tags: vec!["repo".into()],
                stats: WorkstreamStats {
                    pull_requests: 2,
                    reviews: 0,
                    manual_events: 0,
                },
                events: vec![],
                receipts: vec![],
            },
            Workstream {
                id: WorkstreamId::from_parts(["repo", "secret-org/private-repo"]),
                title: "secret-org/private-repo".into(),
                summary: None,
                tags: vec!["repo".into()],
                stats: WorkstreamStats {
                    pull_requests: 1,
                    reviews: 0,
                    manual_events: 0,
                },
                events: vec![],
                receipts: vec![],
            },
        ],
    };

    let reference = redactor.redact_workstreams(&ws, "public").unwrap();
    for _ in 0..ITERATIONS {
        let result = redactor.redact_workstreams(&ws, "public").unwrap();
        assert_eq!(reference.workstreams.len(), result.workstreams.len());
        for (a, b) in reference.workstreams.iter().zip(result.workstreams.iter()) {
            assert_eq!(a.id, b.id, "redacted workstream ID must be stable");
            assert_eq!(a.title, b.title, "redacted workstream title must be stable");
        }
    }
}

// ---------------------------------------------------------------------------
// Call-order independence
// ---------------------------------------------------------------------------

#[test]
fn redaction_independent_of_call_order() {
    let events = sample_events();

    // First: redact public, then manager
    let r1 = DeterministicRedactor::new(b"order-test-key");
    let public_first = r1.redact_events(&events, "public").unwrap();
    let _manager_first = r1.redact_events(&events, "manager").unwrap();

    // Second: redact manager, then public
    let r2 = DeterministicRedactor::new(b"order-test-key");
    let _manager_second = r2.redact_events(&events, "manager").unwrap();
    let public_second = r2.redact_events(&events, "public").unwrap();

    for (a, b) in public_first.iter().zip(public_second.iter()) {
        assert_eq!(
            a.repo.full_name, b.repo.full_name,
            "redaction must not depend on profile call order"
        );
    }
}

#[test]
fn redaction_independent_of_event_order() {
    let events_fwd = sample_events();
    let mut events_rev = events_fwd.clone();
    events_rev.reverse();

    let r1 = DeterministicRedactor::new(b"event-order-key");
    let r2 = DeterministicRedactor::new(b"event-order-key");

    let fwd = r1.redact_events(&events_fwd, "public").unwrap();
    let rev = r2.redact_events(&events_rev, "public").unwrap();

    // Same event should get the same alias regardless of position
    for ev_fwd in &fwd {
        let matching = rev.iter().find(|e| e.id == ev_fwd.id).unwrap();
        assert_eq!(
            ev_fwd.repo.full_name, matching.repo.full_name,
            "alias for event {} must be the same regardless of input order",
            ev_fwd.id
        );
    }
}

// ---------------------------------------------------------------------------
// Alias map consistency
// ---------------------------------------------------------------------------

#[test]
fn alias_map_consistent_across_calls() {
    let redactor = DeterministicRedactor::new(b"alias-consistency-key");
    let events = sample_events();

    // Redact twice — aliases accumulated in the cache should be consistent
    let first = redactor.redact_events(&events, "public").unwrap();
    let second = redactor.redact_events(&events, "public").unwrap();

    for (a, b) in first.iter().zip(second.iter()) {
        assert_eq!(
            a.repo.full_name, b.repo.full_name,
            "alias map must produce consistent results across calls"
        );
    }
}

#[test]
fn different_keys_produce_different_aliases() {
    let events = sample_events();

    let r1 = DeterministicRedactor::new(b"key-alpha");
    let r2 = DeterministicRedactor::new(b"key-beta");

    let out1 = r1.redact_events(&events, "public").unwrap();
    let out2 = r2.redact_events(&events, "public").unwrap();

    // At least one repo alias should differ between different keys
    let any_differ = out1
        .iter()
        .zip(out2.iter())
        .any(|(a, b)| a.repo.full_name != b.repo.full_name);
    assert!(
        any_differ,
        "different keys should produce different aliases"
    );
}
