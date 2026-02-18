//! Property tests for shiplog-ids
//!
//! This module contains property-based tests for ID generation invariants.

use proptest::prelude::*;
use shiplog_ids::{EventId, RunId, WorkstreamId};

// ============================================================================
// EventId Property Tests
// ============================================================================

proptest! {
    // Same parts produce same EventId.
    #[test]
    fn prop_event_id_determinism(parts in proptest::collection::vec("[a-zA-Z0-9_-]{1,50}", 1..5)) {
        let id1 = EventId::from_parts(parts.iter().map(|s| s.as_str()).collect::<Vec<_>>().as_slice());
        let id2 = EventId::from_parts(parts.iter().map(|s| s.as_str()).collect::<Vec<_>>().as_slice());
        prop_assert_eq!(id1, id2);
    }

    // Different parts produce different EventIds.
    #[test]
    fn prop_event_id_uniqueness(
        parts1 in proptest::collection::vec("[a-zA-Z0-9_-]{1,50}", 1..5),
        parts2 in proptest::collection::vec("[a-zA-Z0-9_-]{1,50}", 1..5)
    ) {
        prop_assume!(parts1 != parts2);
        let id1 = EventId::from_parts(parts1.iter().map(|s| s.as_str()).collect::<Vec<_>>().as_slice());
        let id2 = EventId::from_parts(parts2.iter().map(|s| s.as_str()).collect::<Vec<_>>().as_slice());
        prop_assert_ne!(id1, id2);
    }

    // EventId is a 64-character lowercase hex string.
    #[test]
    fn prop_event_id_hex_format(parts in proptest::collection::vec("[a-zA-Z0-9_-]{1,50}", 1..5)) {
        let id = EventId::from_parts(parts.iter().map(|s| s.as_str()).collect::<Vec<_>>().as_slice());
        let id_str = id.to_string();
        prop_assert_eq!(id_str.len(), 64);
        prop_assert!(id_str.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
    }

    // Display matches inner value.
    #[test]
    fn prop_event_id_display_matches_inner(parts in proptest::collection::vec("[a-zA-Z0-9_-]{1,50}", 1..5)) {
        let id = EventId::from_parts(parts.iter().map(|s| s.as_str()).collect::<Vec<_>>().as_slice());
        let display = format!("{}", id);
        prop_assert_eq!(display, id.to_string());
    }
}

#[test]
fn event_id_part_boundary_matters() {
    let id1 = EventId::from_parts(["a", "bc"]);
    let id2 = EventId::from_parts(["ab", "c"]);
    assert_ne!(id1, id2);
}

#[test]
fn event_id_empty_parts_allowed() {
    let id = EventId::from_parts(std::iter::empty::<&str>());
    let id_str = id.to_string();
    assert_eq!(id_str.len(), 64);
}

#[test]
fn event_id_case_sensitivity() {
    let id1 = EventId::from_parts(["test", "ABC"]);
    let id2 = EventId::from_parts(["test", "abc"]);
    assert_ne!(id1, id2);
}

#[test]
fn event_id_whitespace_sensitivity() {
    let id1 = EventId::from_parts(["test", "abc"]);
    let id2 = EventId::from_parts(["test ", "abc"]);
    assert_ne!(id1, id2);
}

// ============================================================================
// WorkstreamId Property Tests
// ============================================================================

proptest! {
    // Same parts produce same WorkstreamId.
    #[test]
    fn prop_workstream_id_determinism(parts in proptest::collection::vec("[a-zA-Z0-9_-]{1,50}", 1..3)) {
        let id1 = WorkstreamId::from_parts(parts.iter().map(|s| s.as_str()).collect::<Vec<_>>().as_slice());
        let id2 = WorkstreamId::from_parts(parts.iter().map(|s| s.as_str()).collect::<Vec<_>>().as_slice());
        prop_assert_eq!(id1, id2);
    }

    // Different parts produce different WorkstreamIds.
    #[test]
    fn prop_workstream_id_uniqueness(
        parts1 in proptest::collection::vec("[a-zA-Z0-9_-]{1,50}", 1..3),
        parts2 in proptest::collection::vec("[a-zA-Z0-9_-]{1,50}", 1..3)
    ) {
        prop_assume!(parts1 != parts2);
        let id1 = WorkstreamId::from_parts(parts1.iter().map(|s| s.as_str()).collect::<Vec<_>>().as_slice());
        let id2 = WorkstreamId::from_parts(parts2.iter().map(|s| s.as_str()).collect::<Vec<_>>().as_slice());
        prop_assert_ne!(id1, id2);
    }

    // WorkstreamId is a 64-character lowercase hex string.
    #[test]
    fn prop_workstream_id_hex_format(parts in proptest::collection::vec("[a-zA-Z0-9_-]{1,50}", 1..3)) {
        let id = WorkstreamId::from_parts(parts.iter().map(|s| s.as_str()).collect::<Vec<_>>().as_slice());
        let id_str = id.to_string();
        prop_assert_eq!(id_str.len(), 64);
        prop_assert!(id_str.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
    }

    // Display matches inner value.
    #[test]
    fn prop_workstream_id_display_matches_inner(parts in proptest::collection::vec("[a-zA-Z0-9_-]{1,50}", 1..3)) {
        let id = WorkstreamId::from_parts(parts.iter().map(|s| s.as_str()).collect::<Vec<_>>().as_slice());
        let display = format!("{}", id);
        prop_assert_eq!(display, id.to_string());
    }
}

// ============================================================================
// RunId Property Tests
// ============================================================================

proptest! {
    // RunId starts with specified prefix.
    #[test]
    fn prop_run_id_prefix(prefix in "[a-z]{3,20}") {
        let id = RunId::now(&prefix);
        let id_str = id.to_string();
        let expected_prefix = format!("{}_", prefix);
        prop_assert!(id_str.starts_with(&expected_prefix));
    }

    // Sequential RunId.now() calls produce different values.
    #[test]
    fn prop_run_id_uniqueness(prefix in "[a-z]{3,20}") {
        let id1 = RunId::now(&prefix);
        std::thread::sleep(std::time::Duration::from_millis(10));
        let id2 = RunId::now(&prefix);
        prop_assert_ne!(id1, id2);
    }

    // RunId suffix after "<prefix>_" is numeric nanoseconds.
    #[test]
    fn prop_run_id_suffix_format(prefix in "[a-z]{3,20}") {
        let id = RunId::now(&prefix);
        let id_str = id.to_string();
        let expected_prefix = format!("{}_", prefix);
        prop_assert!(id_str.starts_with(&expected_prefix));
        let suffix = &id_str[prefix.len() + 1..];
        prop_assert!(!suffix.is_empty());
        prop_assert!(suffix.chars().all(|c| c.is_ascii_digit()));
    }

    // Display matches inner value.
    #[test]
    fn prop_run_id_display_matches_inner(prefix in "[a-z]{3,20}") {
        let id = RunId::now(&prefix);
        let display = format!("{}", id);
        prop_assert_eq!(display, id.to_string());
    }
}
