//! Stability tests for shiplog-ids.
//!
//! These hardcode expected SHA-256 hex digests for known inputs, acting as
//! regression guards that do not depend on any snapshot tooling.

use shiplog_ids::{EventId, WorkstreamId};

// ============================================================================
// Determinism: same inputs always yield the same ID
// ============================================================================

#[test]
fn event_id_deterministic_across_calls() {
    let parts = ["github", "pr", "octocat/hello-world", "42"];
    let first = EventId::from_parts(parts);
    let second = EventId::from_parts(parts);
    let third = EventId::from_parts(parts);
    assert_eq!(first, second);
    assert_eq!(second, third);
}

#[test]
fn workstream_id_deterministic_across_calls() {
    let parts = ["repo", "acme/backend"];
    let first = WorkstreamId::from_parts(parts);
    let second = WorkstreamId::from_parts(parts);
    let third = WorkstreamId::from_parts(parts);
    assert_eq!(first, second);
    assert_eq!(second, third);
}

// ============================================================================
// Known-value regression: hardcoded expected outputs
// ============================================================================

/// SHA-256("abc") is a well-known test vector.
#[test]
fn event_id_known_sha256_abc() {
    let id = EventId::from_parts(["abc"]);
    assert_eq!(
        id.to_string(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
    );
}

/// SHA-256("") — empty input.
#[test]
fn event_id_known_sha256_empty() {
    let id = EventId::from_parts(std::iter::empty::<&str>());
    assert_eq!(
        id.to_string(),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    );
}

/// Multi-part: SHA-256("github\npr\noctocat/hello-world\n42")
#[test]
fn event_id_known_typical_pr() {
    let id = EventId::from_parts(["github", "pr", "octocat/hello-world", "42"]);
    // Precomputed via: echo -n "github\npr\noctocat/hello-world\n42" | sha256sum
    assert_eq!(
        id.to_string(),
        compute_expected(&["github", "pr", "octocat/hello-world", "42"]),
    );
}

/// WorkstreamId uses the same hash_hex function as EventId.
#[test]
fn workstream_id_known_repo() {
    let id = WorkstreamId::from_parts(["repo", "octocat/hello-world"]);
    assert_eq!(
        id.to_string(),
        compute_expected(&["repo", "octocat/hello-world"]),
    );
}

/// EventId and WorkstreamId from identical parts produce the same hash.
#[test]
fn event_and_workstream_same_parts_same_hash() {
    let parts = ["repo", "acme/foo"];
    let event = EventId::from_parts(parts);
    let workstream = WorkstreamId::from_parts(parts);
    assert_eq!(event.to_string(), workstream.to_string());
}

// ============================================================================
// Format invariants
// ============================================================================

#[test]
fn ids_are_lowercase_hex_64_chars() {
    let cases: Vec<Box<dyn ToString>> = vec![
        Box::new(EventId::from_parts(["a"])),
        Box::new(EventId::from_parts(["a", "b", "c"])),
        Box::new(WorkstreamId::from_parts(["x"])),
        Box::new(WorkstreamId::from_parts(["x", "y"])),
    ];
    for id in &cases {
        let s = id.to_string();
        assert_eq!(s.len(), 64, "ID must be 64 hex chars: {s}");
        assert!(
            s.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()),
            "ID must be lowercase hex: {s}",
        );
    }
}

// ============================================================================
// Helper
// ============================================================================

/// Recompute expected SHA-256 hex from parts joined by newlines.
fn compute_expected(parts: &[&str]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    for (i, p) in parts.iter().enumerate() {
        if i > 0 {
            hasher.update(b"\n");
        }
        hasher.update(p.as_bytes());
    }
    hex::encode(hasher.finalize())
}
