//! Property tests for shiplog-redaction-projector.

use proptest::prelude::*;
use shiplog_redaction_policy::{redact_events_with_aliases, redact_workstreams_with_aliases};
use shiplog_redaction_projector::{
    parse_profile, project_events_with_aliases, project_workstreams_with_aliases,
};
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
}
