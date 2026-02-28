//! Boundary value tests for shiplog-ids.
//!
//! Covers maximum-length inputs, empty inputs, special characters,
//! and very similar but different inputs.

use shiplog_ids::{EventId, RunId, WorkstreamId};

// ============================================================================
// Empty string inputs
// ============================================================================

#[test]
fn event_id_from_no_parts() {
    let id = EventId::from_parts(std::iter::empty::<&str>());
    // SHA-256 of empty input
    assert_eq!(id.0.len(), 64);
    assert!(id.0.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn event_id_from_single_empty_string() {
    let id = EventId::from_parts([""]);
    assert_eq!(id.0.len(), 64);
}

#[test]
fn event_id_from_multiple_empty_strings() {
    let id = EventId::from_parts(["", "", ""]);
    assert_eq!(id.0.len(), 64);
    // Must differ from single empty string
    let single = EventId::from_parts([""]);
    assert_ne!(id, single, "multiple empty parts should differ from single empty part");
}

#[test]
fn workstream_id_from_empty_parts() {
    let id = WorkstreamId::from_parts(std::iter::empty::<&str>());
    assert_eq!(id.0.len(), 64);
}

#[test]
fn empty_parts_vs_no_parts() {
    let no_parts = EventId::from_parts(std::iter::empty::<&str>());
    let one_empty = EventId::from_parts([""]);
    // SHA-256("") vs SHA-256("") — these are the same because hashing empty string
    // is the same as hashing a single empty part
    assert_eq!(no_parts, one_empty);
}

// ============================================================================
// Special characters (null bytes, newlines, unicode)
// ============================================================================

#[test]
fn event_id_with_null_bytes() {
    let id = EventId::from_parts(["hello\0world"]);
    assert_eq!(id.0.len(), 64);
    let plain = EventId::from_parts(["helloworld"]);
    assert_ne!(id, plain, "null byte in input should produce different hash");
}

#[test]
fn event_id_with_newlines_in_parts() {
    let with_newline = EventId::from_parts(["hello\nworld"]);
    let split = EventId::from_parts(["hello", "world"]);
    // "hello\nworld" as single part vs "hello" + "\n" + "world" as two parts
    // These should be the same because the separator is \n
    assert_eq!(with_newline, split);
}

#[test]
fn event_id_with_unicode() {
    let id = EventId::from_parts(["日本語", "テスト"]);
    assert_eq!(id.0.len(), 64);
    assert!(id.0.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn event_id_with_emoji() {
    let id = EventId::from_parts(["🚀", "feature"]);
    assert_eq!(id.0.len(), 64);
}

#[test]
fn event_id_with_mixed_special_chars() {
    let id = EventId::from_parts(["tab\there", "cr\rhere", "null\0here"]);
    assert_eq!(id.0.len(), 64);
}

// ============================================================================
// Maximum length inputs
// ============================================================================

#[test]
fn event_id_with_very_long_single_part() {
    let long_str = "a".repeat(100_000);
    let id = EventId::from_parts([long_str.as_str()]);
    assert_eq!(id.0.len(), 64);
    assert!(id.0.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn event_id_with_many_parts() {
    let parts: Vec<String> = (0..10_000).map(|i| format!("part-{i}")).collect();
    let refs: Vec<&str> = parts.iter().map(|s| s.as_str()).collect();
    let id = EventId::from_parts(refs);
    assert_eq!(id.0.len(), 64);
}

#[test]
fn event_id_long_input_is_deterministic() {
    let long_str = "x".repeat(50_000);
    let id1 = EventId::from_parts([long_str.as_str()]);
    let id2 = EventId::from_parts([long_str.as_str()]);
    assert_eq!(id1, id2);
}

#[test]
fn workstream_id_with_very_long_part() {
    let long_str = "b".repeat(100_000);
    let id = WorkstreamId::from_parts([long_str.as_str()]);
    assert_eq!(id.0.len(), 64);
}

// ============================================================================
// Very similar but different inputs
// ============================================================================

#[test]
fn single_character_difference() {
    let id_a = EventId::from_parts(["github", "pr", "owner/repo", "42"]);
    let id_b = EventId::from_parts(["github", "pr", "owner/repo", "43"]);
    assert_ne!(id_a, id_b);
}

#[test]
fn trailing_space_difference() {
    let id_a = EventId::from_parts(["hello"]);
    let id_b = EventId::from_parts(["hello "]);
    assert_ne!(id_a, id_b);
}

#[test]
fn leading_space_difference() {
    let id_a = EventId::from_parts(["hello"]);
    let id_b = EventId::from_parts([" hello"]);
    assert_ne!(id_a, id_b);
}

#[test]
fn case_sensitivity() {
    let id_a = EventId::from_parts(["Hello"]);
    let id_b = EventId::from_parts(["hello"]);
    assert_ne!(id_a, id_b);
}

#[test]
fn part_boundary_collision_resistance() {
    // "ab" + "cd" vs "a" + "bcd" — newline separator prevents collision
    let id1 = EventId::from_parts(["ab", "cd"]);
    let id2 = EventId::from_parts(["a", "bcd"]);
    assert_ne!(id1, id2);
}

#[test]
fn part_boundary_collision_resistance_empty_vs_content() {
    let id1 = EventId::from_parts(["", "abc"]);
    let id2 = EventId::from_parts(["abc"]);
    assert_ne!(id1, id2, "empty prefix part should differ from no prefix");
}

#[test]
fn similar_numeric_strings() {
    let id_a = EventId::from_parts(["0"]);
    let id_b = EventId::from_parts(["00"]);
    let id_c = EventId::from_parts(["000"]);
    assert_ne!(id_a, id_b);
    assert_ne!(id_b, id_c);
    assert_ne!(id_a, id_c);
}

#[test]
fn consecutive_integers() {
    let ids: Vec<EventId> = (0..100)
        .map(|i| EventId::from_parts([&i.to_string()]))
        .collect();
    // All must be unique
    for i in 0..ids.len() {
        for j in (i + 1)..ids.len() {
            assert_ne!(ids[i], ids[j], "IDs for {i} and {j} must differ");
        }
    }
}

// ============================================================================
// RunId boundary tests
// ============================================================================

#[test]
fn run_id_empty_prefix() {
    let id = RunId::now("");
    assert!(id.0.starts_with('_'));
}

#[test]
fn run_id_long_prefix() {
    let prefix = "a".repeat(1000);
    let id = RunId::now(&prefix);
    assert!(id.0.starts_with(&prefix));
}

#[test]
fn run_id_special_chars_in_prefix() {
    let id = RunId::now("test/run-2025");
    assert!(id.0.starts_with("test/run-2025_"));
}

// ============================================================================
// Display trait consistency
// ============================================================================

#[test]
fn display_matches_inner_for_all_types() {
    let eid = EventId::from_parts(["display", "test"]);
    assert_eq!(format!("{eid}"), eid.0);

    let wid = WorkstreamId::from_parts(["display", "test"]);
    assert_eq!(format!("{wid}"), wid.0);

    let rid = RunId::now("display");
    assert_eq!(format!("{rid}"), rid.0);
}
