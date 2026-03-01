//! Property tests for shiplog-redact
//!
//! This module contains property-based tests for redaction invariants
//! (privacy guarantees across profiles).

use proptest::prelude::*;
use shiplog_ports::Redactor;
use shiplog_schema::event::EventPayload;
use shiplog_testkit::proptest::*;

// ============================================================================
// Public Profile Redaction Tests
// ============================================================================

proptest! {
    // Public profile strips sensitive data from all event kinds.
    #[test]
    fn prop_public_profile_redacts_sensitive_fields(
        events in strategy_event_vec(20),
        key_bytes in proptest::collection::vec(any::<u8>(), 1..32)
    ) {
        let r = shiplog_redact::DeterministicRedactor::new(&key_bytes);
        let redacted = r.redact_events(&events, "public").unwrap();

        prop_assert_eq!(events.len(), redacted.len());

        for (orig, red) in events.iter().zip(redacted.iter()) {
            // Links and source URLs are always stripped in public mode.
            prop_assert!(red.links.is_empty());
            prop_assert!(red.source.url.is_none());

            // Repo name should be aliased.
            prop_assert_ne!(&red.repo.full_name, &orig.repo.full_name);

            match &red.payload {
                EventPayload::PullRequest(pr) => {
                    prop_assert_eq!(&pr.title, "[redacted]");
                    prop_assert!(pr.touched_paths_hint.is_empty());
                }
                EventPayload::Review(rev) => {
                    prop_assert_eq!(&rev.pull_title, "[redacted]");
                }
                EventPayload::Manual(man) => {
                    prop_assert_eq!(&man.title, "[redacted]");
                    prop_assert!(man.description.is_none());
                    prop_assert!(man.impact.is_none());
                }
            }
        }
    }

    // Public workstream redaction aliases titles, removes summaries, and drops repo tag.
    #[test]
    fn prop_public_workstreams_strip_sensitive_fields(
        ws_file in strategy_workstreams_file(),
        key_bytes in proptest::collection::vec(any::<u8>(), 1..32)
    ) {
        let r = shiplog_redact::DeterministicRedactor::new(&key_bytes);
        let redacted = r.redact_workstreams(&ws_file, "public").unwrap();

        prop_assert_eq!(ws_file.workstreams.len(), redacted.workstreams.len());

        for (orig, red) in ws_file.workstreams.iter().zip(redacted.workstreams.iter()) {
            prop_assert!(red.summary.is_none());
            prop_assert!(!red.tags.contains(&"repo".to_string()));
            if !orig.title.is_empty() {
                prop_assert_ne!(&red.title, &orig.title);
            }
        }
    }
}

// ============================================================================
// Manager and Internal Profile Tests
// ============================================================================

proptest! {
    // Manager profile keeps useful context but removes sensitive detail fields.
    #[test]
    fn prop_manager_profile_keeps_context_strips_details(
        events in strategy_event_vec(20),
        key_bytes in proptest::collection::vec(any::<u8>(), 1..32)
    ) {
        let r = shiplog_redact::DeterministicRedactor::new(&key_bytes);
        let redacted = r.redact_events(&events, "manager").unwrap();

        prop_assert_eq!(events.len(), redacted.len());

        for (orig, red) in events.iter().zip(redacted.iter()) {
            // Links are stripped for manager profile too.
            prop_assert!(red.links.is_empty());

            match (&orig.payload, &red.payload) {
                (EventPayload::PullRequest(orig_pr), EventPayload::PullRequest(red_pr)) => {
                    prop_assert_eq!(&orig_pr.title, &red_pr.title);
                    prop_assert!(red_pr.touched_paths_hint.is_empty());
                }
                (EventPayload::Manual(orig_m), EventPayload::Manual(red_m)) => {
                    prop_assert_eq!(&orig_m.title, &red_m.title);
                    prop_assert!(red_m.description.is_none());
                    prop_assert!(red_m.impact.is_none());
                }
                _ => {}
            }
        }
    }

    // Internal profile is identity.
    #[test]
    fn prop_internal_profile_is_identity(
        events in strategy_event_vec(20),
        key_bytes in proptest::collection::vec(any::<u8>(), 1..32)
    ) {
        let r = shiplog_redact::DeterministicRedactor::new(&key_bytes);
        let redacted = r.redact_events(&events, "internal").unwrap();

        prop_assert_eq!(events, redacted);
    }

    // Same key produces deterministic public redaction output.
    #[test]
    fn prop_public_redaction_is_deterministic_for_same_key(
        events in strategy_event_vec(20),
        key_bytes in proptest::collection::vec(any::<u8>(), 1..32)
    ) {
        let r1 = shiplog_redact::DeterministicRedactor::new(&key_bytes);
        let r2 = shiplog_redact::DeterministicRedactor::new(&key_bytes);

        let out1 = r1.redact_events(&events, "public").unwrap();
        let out2 = r2.redact_events(&events, "public").unwrap();

        prop_assert_eq!(out1, out2);
    }
}

// ============================================================================
// Advanced Redaction Invariants
// ============================================================================

proptest! {
    // Double public redaction still preserves all privacy properties.
    #[test]
    fn prop_double_public_redaction_still_private(
        events in strategy_event_vec(10),
        key_bytes in proptest::collection::vec(any::<u8>(), 1..32)
    ) {
        let r = shiplog_redact::DeterministicRedactor::new(&key_bytes);
        let once = r.redact_events(&events, "public").unwrap();
        let twice = r.redact_events(&once, "public").unwrap();

        // Privacy properties still hold after double redaction.
        for red in &twice {
            prop_assert!(red.links.is_empty());
            prop_assert!(red.source.url.is_none());
            match &red.payload {
                EventPayload::PullRequest(pr) => {
                    prop_assert_eq!(&pr.title, "[redacted]");
                    prop_assert!(pr.touched_paths_hint.is_empty());
                }
                EventPayload::Review(rev) => {
                    prop_assert_eq!(&rev.pull_title, "[redacted]");
                }
                EventPayload::Manual(man) => {
                    prop_assert_eq!(&man.title, "[redacted]");
                    prop_assert!(man.description.is_none());
                    prop_assert!(man.impact.is_none());
                }
            }
        }
    }

    // Double manager redaction still preserves privacy properties.
    #[test]
    fn prop_double_manager_redaction_still_private(
        events in strategy_event_vec(10),
        key_bytes in proptest::collection::vec(any::<u8>(), 1..32)
    ) {
        let r = shiplog_redact::DeterministicRedactor::new(&key_bytes);
        let once = r.redact_events(&events, "manager").unwrap();
        let twice = r.redact_events(&once, "manager").unwrap();

        for red in &twice {
            prop_assert!(red.links.is_empty());
            match &red.payload {
                EventPayload::PullRequest(pr) => {
                    prop_assert!(pr.touched_paths_hint.is_empty());
                }
                EventPayload::Manual(man) => {
                    prop_assert!(man.description.is_none());
                    prop_assert!(man.impact.is_none());
                }
                _ => {}
            }
        }
    }

    // Redaction preserves event count across any profile.
    #[test]
    fn prop_redaction_preserves_event_count(
        events in strategy_event_vec(20),
        key_bytes in proptest::collection::vec(any::<u8>(), 1..32),
        profile in prop_oneof![Just("internal"), Just("manager"), Just("public")]
    ) {
        let r = shiplog_redact::DeterministicRedactor::new(&key_bytes);
        let redacted = r.redact_events(&events, profile).unwrap();
        prop_assert_eq!(events.len(), redacted.len());
    }

    // Double public workstream redaction still strips sensitive fields.
    #[test]
    fn prop_double_public_workstream_redaction_still_private(
        ws_file in strategy_workstreams_file(),
        key_bytes in proptest::collection::vec(any::<u8>(), 1..32)
    ) {
        let r = shiplog_redact::DeterministicRedactor::new(&key_bytes);
        let once = r.redact_workstreams(&ws_file, "public").unwrap();
        let twice = r.redact_workstreams(&once, "public").unwrap();

        for red in &twice.workstreams {
            prop_assert!(red.summary.is_none());
            prop_assert!(!red.tags.contains(&"repo".to_string()));
        }
    }

    // Redacted public output never contains original repo name.
    #[test]
    fn prop_public_redacted_output_never_contains_original_repo(
        events in strategy_event_vec(10),
        key_bytes in proptest::collection::vec(any::<u8>(), 1..32)
    ) {
        let r = shiplog_redact::DeterministicRedactor::new(&key_bytes);
        let redacted = r.redact_events(&events, "public").unwrap();
        let redacted_json = serde_json::to_string(&redacted).unwrap();

        for orig in &events {
            let orig_repo = &orig.repo.full_name;
            if !orig_repo.is_empty() {
                prop_assert!(
                    !redacted_json.contains(orig_repo),
                    "Redacted JSON should not contain original repo name '{}'", orig_repo
                );
            }
        }
    }

    // Redacted public output never contains original PR titles.
    #[test]
    fn prop_public_redacted_output_never_contains_original_titles(
        events in strategy_event_vec(10),
        key_bytes in proptest::collection::vec(any::<u8>(), 1..32)
    ) {
        let r = shiplog_redact::DeterministicRedactor::new(&key_bytes);
        let redacted = r.redact_events(&events, "public").unwrap();
        let redacted_json = serde_json::to_string(&redacted).unwrap();

        for orig in &events {
            let title = match &orig.payload {
                EventPayload::PullRequest(pr) => &pr.title,
                EventPayload::Review(rev) => &rev.pull_title,
                EventPayload::Manual(man) => &man.title,
            };
            // Only check non-trivial titles (avoid matching "[redacted]" or very short strings)
            if title.len() > 5 && title != "[redacted]" {
                prop_assert!(
                    !redacted_json.contains(title),
                    "Redacted JSON should not contain original title '{}'", title
                );
            }
        }
    }

    // Different keys produce different redacted outputs (for non-empty events).
    #[test]
    fn prop_different_keys_produce_different_outputs(
        events in proptest::collection::vec(shiplog_testkit::proptest::strategy_event_envelope(), 1..5),
        key_a in proptest::collection::vec(any::<u8>(), 1..32),
        key_b in proptest::collection::vec(any::<u8>(), 1..32)
    ) {
        prop_assume!(key_a != key_b);
        let r_a = shiplog_redact::DeterministicRedactor::new(&key_a);
        let r_b = shiplog_redact::DeterministicRedactor::new(&key_b);

        let out_a = r_a.redact_events(&events, "public").unwrap();
        let out_b = r_b.redact_events(&events, "public").unwrap();

        // At least one repo alias should differ since keys differ.
        let any_repo_differs = out_a.iter().zip(out_b.iter())
            .any(|(a, b)| a.repo.full_name != b.repo.full_name);
        prop_assert!(any_repo_differs, "Different keys should produce different repo aliases");
    }
}

// ============================================================================
// Additional Redaction Invariant Tests
// ============================================================================

proptest! {
    // Public redacted events preserve their EventId (structural identity stable through redaction).
    #[test]
    fn prop_public_redaction_preserves_event_ids(
        events in strategy_event_vec(10),
        key_bytes in proptest::collection::vec(any::<u8>(), 1..32)
    ) {
        let r = shiplog_redact::DeterministicRedactor::new(&key_bytes);
        let redacted = r.redact_events(&events, "public").unwrap();

        for (orig, red) in events.iter().zip(redacted.iter()) {
            prop_assert_eq!(&orig.id, &red.id, "EventId must be stable through public redaction");
        }
    }

    // Manager redaction is deterministic with the same key.
    #[test]
    fn prop_manager_redaction_deterministic(
        events in strategy_event_vec(15),
        key_bytes in proptest::collection::vec(any::<u8>(), 1..32)
    ) {
        let r1 = shiplog_redact::DeterministicRedactor::new(&key_bytes);
        let r2 = shiplog_redact::DeterministicRedactor::new(&key_bytes);

        let out1 = r1.redact_events(&events, "manager").unwrap();
        let out2 = r2.redact_events(&events, "manager").unwrap();

        prop_assert_eq!(out1, out2);
    }

    // Redaction preserves event ordering (position-stable).
    #[test]
    fn prop_redaction_preserves_event_order(
        events in strategy_event_vec(20),
        key_bytes in proptest::collection::vec(any::<u8>(), 1..32),
        profile in prop_oneof![Just("internal"), Just("manager"), Just("public")]
    ) {
        let r = shiplog_redact::DeterministicRedactor::new(&key_bytes);
        let redacted = r.redact_events(&events, profile).unwrap();

        prop_assert_eq!(events.len(), redacted.len());
        for (orig, red) in events.iter().zip(redacted.iter()) {
            // Event kind must be preserved regardless of profile.
            prop_assert_eq!(&orig.kind, &red.kind);
        }
    }
}
