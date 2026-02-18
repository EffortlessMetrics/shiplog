//! Property tests for shiplog-redact
//!
//! This module contains property-based tests for redaction invariants
//! (privacy guarantees across profiles).

use proptest::prelude::*;
use shiplog_redact::{DeterministicRedactor, RedactionProfile};
use shiplog_schema::event::*;
use shiplog_testkit::proptest::*;

// ============================================================================
// Redaction Determinism Tests
// ============================================================================

proptest! {
    /// Test that aliases are stable for same key
    #[test]
    fn prop_aliases_are_stable_for_same_key(
        kind in "[a-z]{1,10}",
        value in "[a-zA-Z0-9_-]{1,100}",
        key_bytes in proptest::collection::vec(any::<u8>(), 1..32)
    ) {
        let r = DeterministicRedactor::new(&key_bytes);
        let a1 = r.alias(&kind, &value);
        let a2 = r.alias(&kind, &value);
        prop_assert_eq!(a1, a2);
    }

    /// Test that different keys produce different aliases
    #[test]
    fn prop_different_keys_produce_different_aliases(
        kind in "[a-z]{1,10}",
        value in "[a-zA-Z0-9_-]{1,50}",
        key1 in proptest::collection::vec(any::<u8>(), 1..32),
        key2 in proptest::collection::vec(any::<u8>(), 1..32)
    ) {
        prop_assume!(key1 != key2);
        let r1 = DeterministicRedactor::new(&key1);
        let r2 = DeterministicRedactor::new(&key2);
        let a1 = r1.alias(&kind, &value);
        let a2 = r2.alias(&kind, &value);
        prop_assert_ne!(a1, a2);
    }
}

// ============================================================================
// Public Profile Redaction Tests
// ============================================================================

proptest! {
    /// Test that public profile redacts all sensitive data
    #[test]
    fn prop_public_profile_redacts_all_sensitive_data(
        events in strategy_event_vec(20),
        key_bytes in proptest::collection::vec(any::<u8>(), 1..32)
    ) {
        let r = DeterministicRedactor::new(&key_bytes);
        let redacted = r.redact_events(&events, &RedactionProfile::Public).unwrap();

        for (orig, red) in events.iter().zip(redacted.iter()) {
            // Check titles are redacted
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

            // Check links are removed
            prop_assert!(red.links.is_empty());

            // Check source URL is removed
            prop_assert!(red.source.url.is_none());

            // Check repo is aliased (not equal to original)
            prop_assert_ne!(red.repo.full_name, orig.repo.full_name);
        }
    }

    /// Test that sensitive strings never leak in JSON output
    #[test]
    fn prop_sensitive_strings_redacted(
        title in "[a-zA-Z0-9_-]{10,100}",
        repo in r"[a-z0-9_-]+/[a-z0-9_-]+",
        path_hint in "[a-zA-Z0-9_/-]{5,50}",
        link_url in "https://[a-z0-9.-]+/[a-z0-9._/-]+",
        key_bytes in proptest::collection::vec(any::<u8>(), 1..32)
    ) {
        let r = DeterministicRedactor::new(&key_bytes);

        let ev = EventEnvelope {
            id: shiplog_ids::EventId::from_parts(["test", "1"]),
            kind: EventKind::PullRequest,
            occurred_at: chrono::Utc::now(),
            actor: Actor { login: "user".into(), id: None },
            repo: RepoRef {
                full_name: repo.clone(),
                html_url: Some(link_url.clone()),
                visibility: RepoVisibility::Private,
            },
            payload: EventPayload::PullRequest(PullRequestEvent {
                number: 1,
                title: title.clone(),
                state: PullRequestState::Merged,
                created_at: chrono::Utc::now(),
                merged_at: Some(chrono::Utc::now()),
                additions: Some(100),
                deletions: Some(20),
                changed_files: Some(5),
                touched_paths_hint: vec![path_hint.clone()],
                window: None,
            }),
            tags: vec!["repo".to_string()],
            links: vec![Link {
                label: "pr".to_string(),
                url: link_url.clone(),
            }],
            source: SourceRef {
                system: SourceSystem::Github,
                url: Some(link_url.clone()),
                opaque_id: None,
            },
        };

        let redacted = r.redact_events(&[ev], &RedactionProfile::Public).unwrap();
        let json = serde_json::to_string(&redacted).unwrap();

        // Check that sensitive strings are not in JSON
        prop_assert!(!json.contains(&title));
        prop_assert!(!json.contains(&repo));
        prop_assert!(!json.contains(&path_hint));
        prop_assert!(!json.contains(&link_url));
    }

    /// Test that "repo" tag is removed in public profile
    #[test]
    fn prop_repo_tag_removed_in_public(
        events in strategy_event_vec(20),
        key_bytes in proptest::collection::vec(any::<u8>(), 1..32)
    ) {
        let r = DeterministicRedactor::new(&key_bytes);
        let redacted = r.redact_events(&events, &RedactionProfile::Public).unwrap();

        for red in redacted.iter() {
            prop_assert!(!red.tags.contains(&"repo".to_string()));
        }
    }
}

// ============================================================================
// Manager Profile Redaction Tests
// ============================================================================

proptest! {
    /// Test that manager profile preserves titles but removes paths
    #[test]
    fn prop_manager_profile_preserves_titles_removes_paths(
        events in strategy_event_vec(20),
        key_bytes in proptest::collection::vec(any::<u8>(), 1..32)
    ) {
        let r = DeterministicRedactor::new(&key_bytes);
        let redacted = r.redact_events(&events, &RedactionProfile::Manager).unwrap();

        for (orig, red) in events.iter().zip(redacted.iter()) {
            match (&orig.payload, &red.payload) {
                (EventPayload::PullRequest(orig_pr), EventPayload::PullRequest(red_pr)) => {
                    // Title preserved
                    prop_assert_eq!(orig_pr.title, red_pr.title);
                    // Paths removed
                    prop_assert!(red_pr.touched_paths_hint.is_empty());
                }
                (EventPayload::Manual(orig_m), EventPayload::Manual(red_m)) => {
                    // Title preserved
                    prop_assert_eq!(orig_m.title, red_m.title);
                    // Description removed
                    prop_assert!(red_m.description.is_none());
                    prop_assert!(red_m.impact.is_none());
                }
                _ => {}
            }

            // Links removed
            prop_assert!(red.links.is_empty());
        }
    }
}

// ============================================================================
// Internal Profile Tests
// ============================================================================

proptest! {
    /// Test that internal profile preserves all data
    #[test]
    fn prop_internal_profile_preserves_all_data(
        events in strategy_event_vec(20),
        key_bytes in proptest::collection::vec(any::<u8>(), 1..32)
    ) {
        let r = DeterministicRedactor::new(&key_bytes);
        let redacted = r.redact_events(&events, &RedactionProfile::Internal).unwrap();

        prop_assert_eq!(events.len(), redacted.len());
        for (orig, red) in events.iter().zip(redacted.iter()) {
            prop_assert_eq!(orig.id, red.id);
            prop_assert_eq!(orig.kind, red.kind);
            prop_assert_eq!(orig.actor.login, red.actor.login);
            prop_assert_eq!(orig.repo.full_name, red.repo.full_name);
        }
    }
}

// ============================================================================
// Structural Preservation Tests
// ============================================================================

proptest! {
    /// Test that event envelope structure remains valid after redaction
    #[test]
    fn prop_structural_preservation(
        events in strategy_event_vec(20),
        key_bytes in proptest::collection::vec(any::<u8>(), 1..32)
    ) {
        let r = DeterministicRedactor::new(&key_bytes);

        for profile in [RedactionProfile::Internal, RedactionProfile::Manager, RedactionProfile::Public] {
            let redacted = r.redact_events(&events, &profile).unwrap();

            // Check that all events have valid structure
            for red in redacted.iter() {
                prop_assert!(!red.id.to_string().is_empty());
                prop_assert!(!red.actor.login.is_empty());
                prop_assert!(!red.repo.full_name.is_empty());
            }
        }
    }

    /// Test that receipt counts and event relationships are maintained
    #[test]
    fn prop_receipt_preservation(
        events in strategy_event_vec(20),
        key_bytes in proptest::collection::vec(any::<u8>(), 1..32)
    ) {
        let r = DeterministicRedactor::new(&key_bytes);

        for profile in [RedactionProfile::Internal, RedactionProfile::Manager] {
            let redacted = r.redact_events(&events, &profile).unwrap();

            // Check that event count is preserved
            prop_assert_eq!(events.len(), redacted.len());

            // Check that event IDs are preserved
            for (orig, red) in events.iter().zip(redacted.iter()) {
                prop_assert_eq!(orig.id, red.id);
            }
        }
    }
}
