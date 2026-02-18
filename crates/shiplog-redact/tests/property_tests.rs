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
