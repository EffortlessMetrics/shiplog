//! Property tests for shiplog-redaction-projector.

use proptest::prelude::*;
use shiplog_redaction_policy::{redact_events_with_aliases, redact_workstreams_with_aliases};
use shiplog_redaction_projector::{
    parse_profile, project_events_with_aliases, project_workstreams_with_aliases,
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
    fn prop_event_projection_matches_policy_dispatch(
        events in strategy_event_vec(20),
        profile in ".*"
    ) {
        let actual = project_events_with_aliases(&events, &profile, &test_alias);
        let expected = redact_events_with_aliases(&events, parse_profile(&profile), &test_alias);

        prop_assert_eq!(actual, expected);
    }

    #[test]
    fn prop_workstream_projection_matches_policy_dispatch(
        workstreams in strategy_workstreams_file(),
        profile in ".*"
    ) {
        let actual = project_workstreams_with_aliases(&workstreams, &profile, &test_alias);
        let expected =
            redact_workstreams_with_aliases(&workstreams, parse_profile(&profile), &test_alias);

        prop_assert_eq!(actual, expected);
    }

    #[test]
    fn prop_projection_is_canonicalized_by_parse_profile(
        events in strategy_event_vec(20),
        profile in ".*"
    ) {
        let canonical = parse_profile(&profile).as_str();
        let projected_raw = project_events_with_aliases(&events, &profile, &test_alias);
        let projected_canonical = project_events_with_aliases(&events, canonical, &test_alias);

        prop_assert_eq!(projected_raw, projected_canonical);
    }

    #[test]
    fn prop_projector_preserves_event_ids_and_kinds(
        events in strategy_event_vec(20),
        profile in prop_oneof![Just("internal"), Just("manager"), Just("public"), Just("unknown")]
    ) {
        let projected = project_events_with_aliases(&events, profile, &test_alias);
        prop_assert_eq!(events.len(), projected.len());
        for (orig, proj) in events.iter().zip(projected.iter()) {
            prop_assert_eq!(&orig.id, &proj.id);
            prop_assert_eq!(&orig.kind, &proj.kind);
        }
    }

    #[test]
    fn prop_projector_public_clears_links_and_source_url(
        events in strategy_event_vec(20),
    ) {
        let projected = project_events_with_aliases(&events, "public", &test_alias);
        for event in &projected {
            prop_assert!(event.links.is_empty());
            prop_assert!(event.source.url.is_none());
        }
    }

    #[test]
    fn prop_projector_internal_is_identity(
        events in strategy_event_vec(20),
    ) {
        let projected = project_events_with_aliases(&events, "internal", &test_alias);
        prop_assert_eq!(projected, events);
    }

    #[test]
    fn prop_projector_workstream_count_preserved(
        workstreams in strategy_workstreams_file(),
        profile in prop_oneof![Just("internal"), Just("manager"), Just("public")]
    ) {
        let projected = project_workstreams_with_aliases(&workstreams, profile, &test_alias);
        prop_assert_eq!(workstreams.workstreams.len(), projected.workstreams.len());
    }

    #[test]
    fn prop_projector_public_events_all_titled_redacted(
        events in strategy_event_vec(20),
    ) {
        let projected = project_events_with_aliases(&events, "public", &test_alias);
        for event in &projected {
            match &event.payload {
                EventPayload::PullRequest(pr) => prop_assert_eq!(&pr.title, "[redacted]"),
                EventPayload::Review(r) => prop_assert_eq!(&r.pull_title, "[redacted]"),
                EventPayload::Manual(m) => prop_assert_eq!(&m.title, "[redacted]"),
            }
        }
    }
}
