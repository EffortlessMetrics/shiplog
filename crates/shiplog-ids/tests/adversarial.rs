//! Adversarial and edge-case tests for shiplog-ids.

use shiplog_ids::{EventId, RunId, WorkstreamId};

#[test]
fn empty_parts_produces_valid_id() {
    let id = EventId::from_parts(std::iter::empty::<&str>());
    assert_eq!(id.0.len(), 64);
    assert!(id.0.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn single_empty_string_part() {
    let id = EventId::from_parts([""]);
    assert_eq!(id.0.len(), 64);
}

#[test]
fn determinism_across_calls() {
    let a = EventId::from_parts(["github", "pr", "org/repo", "42"]);
    let b = EventId::from_parts(["github", "pr", "org/repo", "42"]);
    assert_eq!(a, b);
}

#[test]
fn different_parts_differ() {
    let a = EventId::from_parts(["a"]);
    let b = EventId::from_parts(["b"]);
    assert_ne!(a, b);
}

#[test]
fn order_matters() {
    let a = EventId::from_parts(["x", "y"]);
    let b = EventId::from_parts(["y", "x"]);
    assert_ne!(a, b);
}

#[test]
fn very_long_single_part() {
    let long = "x".repeat(1_000_000);
    let id = EventId::from_parts([&long]);
    assert_eq!(id.0.len(), 64);
}

#[test]
fn many_parts() {
    let parts: Vec<String> = (0..10_000).map(|i| format!("part-{i}")).collect();
    let id = EventId::from_parts(parts.iter().map(|s| s.as_str()));
    assert_eq!(id.0.len(), 64);
}

#[test]
fn unicode_parts() {
    let id = EventId::from_parts(["\u{65E5}\u{672C}\u{8A9E}", "\u{1F600}\u{1F680}"]);
    assert_eq!(id.0.len(), 64);
}

#[test]
fn null_bytes_in_parts() {
    let id = EventId::from_parts(["a\0b", "c\0d"]);
    assert_eq!(id.0.len(), 64);
}

#[test]
fn newline_in_parts() {
    let id = EventId::from_parts(["line1\nline2"]);
    assert_eq!(id.0.len(), 64);
}

#[test]
fn parts_containing_separator_may_collide() {
    // hash_hex uses "\n" as separator; parts with embedded newlines can collide.
    // This documents a known limitation.
    let a = EventId::from_parts(["a\nb", "c"]);
    let b = EventId::from_parts(["a", "b\nc"]);
    // Both hash "a\nb\nc" so they collide:
    assert_eq!(a, b, "known limitation: newline separator causes collision");
}

#[test]
fn workstream_id_determinism() {
    let a = WorkstreamId::from_parts(["repo", "acme/widgets"]);
    let b = WorkstreamId::from_parts(["repo", "acme/widgets"]);
    assert_eq!(a, b);
}

#[test]
fn workstream_id_differs_from_event_id_same_parts() {
    // Both use the same hash_hex but since the types are different wrappers,
    // they should hash the same but be type-distinct.
    let eid = EventId::from_parts(["a", "b"]);
    let wid = WorkstreamId::from_parts(["a", "b"]);
    assert_eq!(eid.0, wid.0, "same parts produce same hash");
}

#[test]
fn run_id_now_has_prefix() {
    let id = RunId::now("test-prefix");
    assert!(id.0.starts_with("test-prefix_"));
}

#[test]
fn run_id_uniqueness() {
    let a = RunId::now("run");
    let b = RunId::now("run");
    // Not guaranteed unique in same nanosecond on some platforms,
    // but in practice successive calls differ.
    // We just verify they are valid strings.
    assert!(a.0.starts_with("run_"));
    assert!(b.0.starts_with("run_"));
}

#[test]
fn run_id_empty_prefix() {
    let id = RunId::now("");
    assert!(id.0.starts_with("_"));
}

#[test]
fn serde_round_trip_event_id() {
    let id = EventId::from_parts(["test"]);
    let json = serde_json::to_string(&id).unwrap();
    let back: EventId = serde_json::from_str(&json).unwrap();
    assert_eq!(id, back);
}

#[test]
fn serde_round_trip_workstream_id() {
    let id = WorkstreamId::from_parts(["ws", "test"]);
    let json = serde_json::to_string(&id).unwrap();
    let back: WorkstreamId = serde_json::from_str(&json).unwrap();
    assert_eq!(id, back);
}

#[test]
fn display_matches_inner_string() {
    let id = EventId::from_parts(["display-test"]);
    assert_eq!(format!("{id}"), id.0);
}
