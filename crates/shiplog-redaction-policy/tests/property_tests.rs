//! Property tests for shiplog-redaction-policy.

use proptest::prelude::*;
use shiplog_redaction_policy::{
    RedactionProfile, redact_event_with_aliases, redact_events_with_aliases,
    redact_workstream_with_aliases, redact_workstreams_with_aliases,
};
use shiplog_schema::event::EventPayload;
use shiplog_testkit::proptest::*;

fn test_alias(kind: &str, value: &str) -> String {
    let mut acc = 0xcbf29ce484222325u64;
    for byte in kind.bytes().chain(value.bytes()) {
        acc ^= u64::from(byte);
        acc = acc.wrapping_mul(0x100000001b3);
    }
    format!("{kind}-{acc:016x}-redacted")
}

proptest! {
    #[test]
    fn prop_public_profile_redacts_sensitive_fields(events in strategy_event_vec(20)) {
        let redacted = redact_events_with_aliases(&events, RedactionProfile::Public, &test_alias);
        prop_assert_eq!(events.len(), redacted.len());

        for (original, projected) in events.iter().zip(redacted.iter()) {
            prop_assert!(projected.links.is_empty());
            prop_assert!(projected.source.url.is_none());

            let expected_repo_alias = test_alias("repo", &original.repo.full_name);
            prop_assert_eq!(&projected.repo.full_name, &expected_repo_alias);

            match (&original.payload, &projected.payload) {
                (EventPayload::PullRequest(_), EventPayload::PullRequest(pr)) => {
                    prop_assert_eq!(&pr.title, "[redacted]");
                    prop_assert!(pr.touched_paths_hint.is_empty());
                }
                (EventPayload::Review(_), EventPayload::Review(review)) => {
                    prop_assert_eq!(&review.pull_title, "[redacted]");
                }
                (EventPayload::Manual(_), EventPayload::Manual(manual)) => {
                    prop_assert_eq!(&manual.title, "[redacted]");
                    prop_assert!(manual.description.is_none());
                    prop_assert!(manual.impact.is_none());
                }
                _ => prop_assert!(false, "payload variant changed during redaction"),
            }
        }
    }

    #[test]
    fn prop_manager_profile_preserves_context_strips_detail_fields(events in strategy_event_vec(20)) {
        let redacted = redact_events_with_aliases(&events, RedactionProfile::Manager, &test_alias);
        prop_assert_eq!(events.len(), redacted.len());

        for (original, projected) in events.iter().zip(redacted.iter()) {
            prop_assert!(projected.links.is_empty());
            prop_assert_eq!(&projected.repo.full_name, &original.repo.full_name);
            prop_assert_eq!(&projected.source.url, &original.source.url);

            match (&original.payload, &projected.payload) {
                (EventPayload::PullRequest(orig_pr), EventPayload::PullRequest(red_pr)) => {
                    prop_assert_eq!(&red_pr.title, &orig_pr.title);
                    prop_assert!(red_pr.touched_paths_hint.is_empty());
                }
                (EventPayload::Review(orig_review), EventPayload::Review(red_review)) => {
                    prop_assert_eq!(red_review, orig_review);
                }
                (EventPayload::Manual(orig_manual), EventPayload::Manual(red_manual)) => {
                    prop_assert_eq!(&red_manual.title, &orig_manual.title);
                    prop_assert!(red_manual.description.is_none());
                    prop_assert!(red_manual.impact.is_none());
                }
                _ => prop_assert!(false, "payload variant changed during redaction"),
            }
        }
    }

    #[test]
    fn prop_internal_profile_is_identity_for_events(events in strategy_event_vec(20)) {
        let redacted = redact_events_with_aliases(&events, RedactionProfile::Internal, &test_alias);
        prop_assert_eq!(redacted, events);
    }

    #[test]
    fn prop_public_workstreams_alias_titles_strip_summary_and_repo_tag(ws_file in strategy_workstreams_file()) {
        let redacted = redact_workstreams_with_aliases(&ws_file, RedactionProfile::Public, &test_alias);
        prop_assert_eq!(ws_file.workstreams.len(), redacted.workstreams.len());

        for (original, projected) in ws_file.workstreams.iter().zip(redacted.workstreams.iter()) {
            let expected_title_alias = test_alias("ws", &original.title);
            prop_assert_eq!(&projected.title, &expected_title_alias);
            prop_assert!(projected.summary.is_none());
            prop_assert!(!projected.tags.contains(&"repo".to_string()));
        }
    }

    #[test]
    fn prop_internal_workstreams_identity(ws_file in strategy_workstreams_file()) {
        let redacted = redact_workstreams_with_aliases(&ws_file, RedactionProfile::Internal, &test_alias);
        prop_assert_eq!(redacted, ws_file);
    }

    #[test]
    fn prop_redaction_preserves_event_count(events in strategy_event_vec(20)) {
        for profile in [RedactionProfile::Internal, RedactionProfile::Manager, RedactionProfile::Public] {
            let redacted = redact_events_with_aliases(&events, profile, &test_alias);
            prop_assert_eq!(events.len(), redacted.len(), "event count changed for profile {:?}", profile);
        }
    }

    #[test]
    fn prop_redaction_preserves_event_id(events in strategy_event_vec(20)) {
        for profile in [RedactionProfile::Internal, RedactionProfile::Manager, RedactionProfile::Public] {
            let redacted = redact_events_with_aliases(&events, profile, &test_alias);
            for (orig, red) in events.iter().zip(redacted.iter()) {
                prop_assert_eq!(&orig.id, &red.id, "event ID changed for profile {:?}", profile);
            }
        }
    }

    #[test]
    fn prop_redaction_preserves_event_kind(events in strategy_event_vec(20)) {
        for profile in [RedactionProfile::Internal, RedactionProfile::Manager, RedactionProfile::Public] {
            let redacted = redact_events_with_aliases(&events, profile, &test_alias);
            for (orig, red) in events.iter().zip(redacted.iter()) {
                prop_assert_eq!(&orig.kind, &red.kind, "event kind changed for profile {:?}", profile);
            }
        }
    }

    #[test]
    fn prop_public_is_stricter_than_manager(event in strategy_event_envelope()) {
        let manager = redact_event_with_aliases(event.clone(), RedactionProfile::Manager, &test_alias);
        let public = redact_event_with_aliases(event.clone(), RedactionProfile::Public, &test_alias);

        // Public always clears source URL; manager may keep it
        prop_assert!(public.source.url.is_none());
        // Public always aliases repo; manager keeps original
        prop_assert_ne!(&public.repo.full_name, &event.repo.full_name);
        prop_assert_eq!(&manager.repo.full_name, &event.repo.full_name);
    }

    #[test]
    fn prop_manager_workstream_keeps_title_and_tags(ws in strategy_workstream()) {
        let redacted = redact_workstream_with_aliases(ws.clone(), RedactionProfile::Manager, &test_alias);
        prop_assert_eq!(&redacted.title, &ws.title);
        prop_assert!(redacted.summary.is_none());
        prop_assert_eq!(&redacted.tags, &ws.tags);
    }
}
