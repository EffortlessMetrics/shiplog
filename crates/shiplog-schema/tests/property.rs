//! Additional property tests for shiplog-schema crates.io readiness.
//!
//! Covers SourceSystem roundtrip, Event serialization stability, EventKind
//! coverage, backward compatibility, Event ID determinism, and ManualEventType /
//! BundleProfile roundtrips.

use proptest::prelude::*;
use shiplog_schema::bundle::BundleProfile;
use shiplog_schema::coverage::TimeWindow;
use shiplog_schema::event::*;

// ============================================================================
// SourceSystem JSON Roundtrip (property)
// ============================================================================

proptest! {
    #[test]
    fn source_system_json_roundtrip(ss in shiplog_testkit::proptest::base_strategy_source_system()) {
        let json = serde_json::to_string(&ss).unwrap();
        let back: SourceSystem = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(ss, back);
    }
}

// ============================================================================
// SourceSystem backward compat – PascalCase strings
// ============================================================================

#[test]
fn source_system_pascal_case_backward_compat_all_known() {
    let cases = [
        ("Github", SourceSystem::Github),
        ("GITHUB", SourceSystem::Github),
        ("JsonImport", SourceSystem::JsonImport),
        ("JSONIMPORT", SourceSystem::JsonImport),
        ("JSON_IMPORT", SourceSystem::JsonImport),
        ("LocalGit", SourceSystem::LocalGit),
        ("LOCALGIT", SourceSystem::LocalGit),
        ("LOCAL_GIT", SourceSystem::LocalGit),
        ("Manual", SourceSystem::Manual),
        ("MANUAL", SourceSystem::Manual),
        ("Unknown", SourceSystem::Unknown),
        ("UNKNOWN", SourceSystem::Unknown),
    ];
    for (input, expected) in cases {
        let json = format!("\"{input}\"");
        let back: SourceSystem = serde_json::from_str(&json).unwrap();
        assert_eq!(back, expected, "backward compat for {input}");
    }
}

// ============================================================================
// SourceSystem::Other – arbitrary strings that don't collide with known
// ============================================================================

proptest! {
    #[test]
    fn source_system_other_roundtrip(name in "[a-z]{5,15}") {
        // Skip strings that happen to match known variants
        let known = ["github", "json_import", "jsonimport", "local_git", "localgit", "manual", "unknown"];
        prop_assume!(!known.contains(&name.as_str()));

        let ss = SourceSystem::Other(name.clone());
        let json = serde_json::to_string(&ss).unwrap();
        let back: SourceSystem = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(SourceSystem::Other(name), back);
    }
}

// ============================================================================
// SourceSystem::from_str_lossy idempotence
// ============================================================================

proptest! {
    #[test]
    fn source_system_from_str_lossy_idempotent(s in "[a-zA-Z_]{1,30}") {
        let first = SourceSystem::from_str_lossy(&s);
        let canonical = first.as_str().to_string();
        let second = SourceSystem::from_str_lossy(&canonical);
        prop_assert_eq!(first, second, "from_str_lossy should be idempotent on canonical form");
    }
}

// ============================================================================
// SourceSystem Display == serialized value
// ============================================================================

proptest! {
    #[test]
    fn source_system_display_matches_serde(ss in shiplog_testkit::proptest::base_strategy_source_system()) {
        let display = format!("{ss}");
        let serialized: String = serde_json::from_str(&serde_json::to_string(&ss).unwrap()).unwrap();
        prop_assert_eq!(display, serialized);
    }
}

// ============================================================================
// EventEnvelope full-fidelity JSON roundtrip (identity)
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]
    #[test]
    fn event_envelope_serialize_deserialize_identity(event in shiplog_testkit::proptest::strategy_event_envelope()) {
        let json = serde_json::to_string(&event).unwrap();
        let back: EventEnvelope = serde_json::from_str(&json).unwrap();
        // Full structural equality
        prop_assert_eq!(&event.id, &back.id);
        prop_assert_eq!(&event.kind, &back.kind);
        prop_assert_eq!(&event.actor, &back.actor);
        prop_assert_eq!(&event.repo, &back.repo);
        prop_assert_eq!(&event.tags, &back.tags);
        prop_assert_eq!(&event.links, &back.links);
        prop_assert_eq!(&event.source, &back.source);
        // occurred_at equality (chrono DateTime roundtrips)
        prop_assert_eq!(event.occurred_at, back.occurred_at);
    }
}

// ============================================================================
// EventKind coverage – every variant round-trips
// ============================================================================

#[test]
fn event_kind_all_variants_roundtrip() {
    let variants = [EventKind::PullRequest, EventKind::Review, EventKind::Manual];
    for kind in &variants {
        let json = serde_json::to_string(kind).unwrap();
        let back: EventKind = serde_json::from_str(&json).unwrap();
        assert_eq!(kind, &back, "EventKind roundtrip failed for {json}");
    }
}

// ============================================================================
// ManualEventType coverage – every variant round-trips
// ============================================================================

#[test]
fn manual_event_type_all_variants_roundtrip() {
    let variants = [
        ManualEventType::Note,
        ManualEventType::Incident,
        ManualEventType::Design,
        ManualEventType::Mentoring,
        ManualEventType::Launch,
        ManualEventType::Migration,
        ManualEventType::Review,
        ManualEventType::Other,
    ];
    for v in &variants {
        let json = serde_json::to_string(v).unwrap();
        let back: ManualEventType = serde_json::from_str(&json).unwrap();
        assert_eq!(v, &back, "ManualEventType roundtrip failed for {json}");
    }
}

// ============================================================================
// PullRequestState coverage
// ============================================================================

#[test]
fn pr_state_all_variants_roundtrip() {
    let variants = [
        PullRequestState::Open,
        PullRequestState::Closed,
        PullRequestState::Merged,
        PullRequestState::Unknown,
    ];
    for v in &variants {
        let json = serde_json::to_string(v).unwrap();
        let back: PullRequestState = serde_json::from_str(&json).unwrap();
        assert_eq!(v, &back, "PullRequestState roundtrip failed for {json}");
    }
}

// ============================================================================
// RepoVisibility coverage
// ============================================================================

#[test]
fn repo_visibility_all_variants_roundtrip() {
    let variants = [
        RepoVisibility::Public,
        RepoVisibility::Private,
        RepoVisibility::Unknown,
    ];
    for v in &variants {
        let json = serde_json::to_string(v).unwrap();
        let back: RepoVisibility = serde_json::from_str(&json).unwrap();
        assert_eq!(v, &back, "RepoVisibility roundtrip failed for {json}");
    }
}

// ============================================================================
// BundleProfile coverage
// ============================================================================

#[test]
fn bundle_profile_all_variants_roundtrip() {
    let variants = [
        BundleProfile::Internal,
        BundleProfile::Manager,
        BundleProfile::Public,
    ];
    for v in &variants {
        let json = serde_json::to_string(v).unwrap();
        let back: BundleProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(v, &back, "BundleProfile roundtrip failed for {json}");
    }
}

// ============================================================================
// Event ID determinism – same inputs always produce the same ID
// ============================================================================

proptest! {
    #[test]
    fn event_id_deterministic(
        parts in proptest::collection::vec("[a-zA-Z0-9_/-]{1,30}", 1..6)
    ) {
        let id1 = shiplog_ids::EventId::from_parts(parts.iter().map(|s| s.as_str()));
        let id2 = shiplog_ids::EventId::from_parts(parts.iter().map(|s| s.as_str()));
        prop_assert_eq!(id1, id2, "EventId must be deterministic for identical inputs");
    }

    #[test]
    fn event_id_different_inputs_differ(
        parts_a in proptest::collection::vec("[a-zA-Z0-9]{1,20}", 2..4),
        parts_b in proptest::collection::vec("[a-zA-Z0-9]{1,20}", 2..4),
    ) {
        prop_assume!(parts_a != parts_b);
        let id_a = shiplog_ids::EventId::from_parts(parts_a.iter().map(|s| s.as_str()));
        let id_b = shiplog_ids::EventId::from_parts(parts_b.iter().map(|s| s.as_str()));
        prop_assert_ne!(id_a, id_b, "Different inputs should (virtually always) produce different IDs");
    }
}

// ============================================================================
// EventPayload tagged serialization preserves discriminant
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]
    #[test]
    fn event_payload_tagged_roundtrip(payload in shiplog_testkit::proptest::strategy_event_payload()) {
        let json = serde_json::to_string(&payload).unwrap();
        let back: EventPayload = serde_json::from_str(&json).unwrap();
        // Check the tag discriminant survived
        match (&payload, &back) {
            (EventPayload::PullRequest(_), EventPayload::PullRequest(_)) => {},
            (EventPayload::Review(_), EventPayload::Review(_)) => {},
            (EventPayload::Manual(_), EventPayload::Manual(_)) => {},
            _ => prop_assert!(false, "payload discriminant changed after roundtrip"),
        }
    }
}

// ============================================================================
// TimeWindow – since is always before until after strategy generation
// ============================================================================

proptest! {
    #[test]
    fn time_window_since_before_until(tw in shiplog_testkit::proptest::strategy_time_window()) {
        prop_assert!(tw.since <= tw.until, "TimeWindow since must be <= until");
    }

    #[test]
    fn time_window_contains_since(
        since in shiplog_testkit::proptest::strategy_naive_date(),
        days in 1u64..365u64,
    ) {
        let until = since.checked_add_days(chrono::Days::new(days)).unwrap();
        let tw = TimeWindow { since, until };
        prop_assert!(tw.contains(since), "TimeWindow must contain its own since date");
    }

    #[test]
    fn time_window_excludes_until(
        since in shiplog_testkit::proptest::strategy_naive_date(),
        days in 1u64..365u64,
    ) {
        let until = since.checked_add_days(chrono::Days::new(days)).unwrap();
        let tw = TimeWindow { since, until };
        prop_assert!(!tw.contains(until), "TimeWindow must exclude its own until date");
    }
}

// ============================================================================
// ManualDate roundtrip (Single and Range)
// ============================================================================

#[test]
fn manual_date_single_roundtrip() {
    let d = ManualDate::Single(chrono::NaiveDate::from_ymd_opt(2025, 6, 15).unwrap());
    let json = serde_json::to_string(&d).unwrap();
    let back: ManualDate = serde_json::from_str(&json).unwrap();
    assert_eq!(d, back);
}

#[test]
fn manual_date_range_roundtrip() {
    let d = ManualDate::Range {
        start: chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        end: chrono::NaiveDate::from_ymd_opt(2025, 3, 31).unwrap(),
    };
    let json = serde_json::to_string(&d).unwrap();
    let back: ManualDate = serde_json::from_str(&json).unwrap();
    assert_eq!(d, back);
}

// ============================================================================
// Completeness coverage
// ============================================================================

#[test]
fn completeness_all_variants_roundtrip() {
    use shiplog_schema::coverage::Completeness;
    let variants = [
        Completeness::Complete,
        Completeness::Partial,
        Completeness::Unknown,
    ];
    for v in &variants {
        let json = serde_json::to_string(v).unwrap();
        let back: Completeness = serde_json::from_str(&json).unwrap();
        assert_eq!(v, &back, "Completeness roundtrip failed for {json}");
    }
}
