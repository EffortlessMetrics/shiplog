//! Determinism tests for shiplog-ids.
//!
//! Verifies that ID generation is fully reproducible: same inputs always
//! produce the same outputs, regardless of call order or system time.

use shiplog_ids::{EventId, WorkstreamId};

const ITERATIONS: usize = 100;

// ---------------------------------------------------------------------------
// EventId: same inputs → same output across iterations
// ---------------------------------------------------------------------------

#[test]
fn event_id_deterministic_across_100_iterations() {
    let reference = EventId::from_parts(["github", "pr", "acme/widgets", "42"]);
    for _ in 0..ITERATIONS {
        let id = EventId::from_parts(["github", "pr", "acme/widgets", "42"]);
        assert_eq!(id, reference, "EventId must be identical on every call");
    }
}

#[test]
fn event_id_deterministic_with_varied_inputs() {
    let cases: Vec<Vec<&str>> = vec![
        vec!["a"],
        vec!["a", "b"],
        vec!["github", "pr", "owner/repo", "1"],
        vec!["github", "review", "owner/repo", "1", "42"],
        vec!["", ""],
        vec!["unicode", "日本語", "emoji🚀"],
    ];

    for parts in &cases {
        let reference = EventId::from_parts(parts.iter());
        for _ in 0..ITERATIONS {
            assert_eq!(
                EventId::from_parts(parts.iter()),
                reference,
                "EventId must be stable for parts: {parts:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// WorkstreamId: same inputs → same output across iterations
// ---------------------------------------------------------------------------

#[test]
fn workstream_id_deterministic_across_100_iterations() {
    let reference = WorkstreamId::from_parts(["repo", "acme/widgets"]);
    for _ in 0..ITERATIONS {
        let id = WorkstreamId::from_parts(["repo", "acme/widgets"]);
        assert_eq!(
            id, reference,
            "WorkstreamId must be identical on every call"
        );
    }
}

#[test]
fn workstream_id_deterministic_with_varied_inputs() {
    let cases: Vec<Vec<&str>> = vec![
        vec!["repo", "org/project"],
        vec!["custom", "my-workstream"],
        vec!["ws", "日本語"],
        vec!["a"],
    ];

    for parts in &cases {
        let reference = WorkstreamId::from_parts(parts.iter());
        for _ in 0..ITERATIONS {
            assert_eq!(
                WorkstreamId::from_parts(parts.iter()),
                reference,
                "WorkstreamId must be stable for parts: {parts:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Call-order independence: interleaving different IDs doesn't affect results
// ---------------------------------------------------------------------------

#[test]
fn event_id_independent_of_call_order() {
    // Generate IDs in order A, B, C
    let a1 = EventId::from_parts(["github", "pr", "acme/foo", "1"]);
    let b1 = EventId::from_parts(["github", "pr", "acme/bar", "2"]);
    let c1 = EventId::from_parts(["github", "review", "acme/foo", "1", "99"]);

    // Generate IDs in order C, A, B
    let c2 = EventId::from_parts(["github", "review", "acme/foo", "1", "99"]);
    let a2 = EventId::from_parts(["github", "pr", "acme/foo", "1"]);
    let b2 = EventId::from_parts(["github", "pr", "acme/bar", "2"]);

    assert_eq!(a1, a2, "EventId must not depend on call order");
    assert_eq!(b1, b2, "EventId must not depend on call order");
    assert_eq!(c1, c2, "EventId must not depend on call order");
}

#[test]
fn workstream_id_independent_of_call_order() {
    let a1 = WorkstreamId::from_parts(["repo", "org/alpha"]);
    let b1 = WorkstreamId::from_parts(["repo", "org/beta"]);

    let b2 = WorkstreamId::from_parts(["repo", "org/beta"]);
    let a2 = WorkstreamId::from_parts(["repo", "org/alpha"]);

    assert_eq!(a1, a2, "WorkstreamId must not depend on call order");
    assert_eq!(b1, b2, "WorkstreamId must not depend on call order");
}

// ---------------------------------------------------------------------------
// Time independence: EventId and WorkstreamId don't use system time
// ---------------------------------------------------------------------------

#[test]
fn event_id_independent_of_system_time() {
    let id1 = EventId::from_parts(["github", "pr", "time-test/repo", "7"]);
    std::thread::sleep(std::time::Duration::from_millis(50));
    let id2 = EventId::from_parts(["github", "pr", "time-test/repo", "7"]);

    assert_eq!(id1, id2, "EventId must not depend on system time");
}

#[test]
fn workstream_id_independent_of_system_time() {
    let id1 = WorkstreamId::from_parts(["repo", "time-test/repo"]);
    std::thread::sleep(std::time::Duration::from_millis(50));
    let id2 = WorkstreamId::from_parts(["repo", "time-test/repo"]);

    assert_eq!(id1, id2, "WorkstreamId must not depend on system time");
}

// ---------------------------------------------------------------------------
// Cross-type isolation: EventId and WorkstreamId from same parts differ
// ---------------------------------------------------------------------------

#[test]
fn event_and_workstream_id_share_same_hash_for_same_parts() {
    // Both use the same hash_hex function, so same parts → same hash.
    // This is by design — the type wrapper provides semantic distinction.
    let event = EventId::from_parts(["repo", "acme/widgets"]);
    let workstream = WorkstreamId::from_parts(["repo", "acme/widgets"]);
    assert_eq!(
        event.0, workstream.0,
        "same parts should produce the same underlying hash"
    );
}
