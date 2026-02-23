//! Property tests for shiplog-redaction-profile.

use proptest::prelude::*;
use shiplog_redaction_profile::RedactionProfile;

proptest! {
    #[test]
    fn parse_then_render_is_always_canonical(input in ".*") {
        let parsed = RedactionProfile::from_profile_str(&input);
        let rendered = parsed.as_str();

        prop_assert!(matches!(rendered, "internal" | "manager" | "public"));
    }

    #[test]
    fn unknown_inputs_default_to_public(input in ".*") {
        prop_assume!(input != "internal");
        prop_assume!(input != "manager");

        let parsed = RedactionProfile::from_profile_str(&input);
        prop_assert_eq!(parsed, RedactionProfile::Public);
    }

    #[test]
    fn canonical_roundtrip_is_idempotent(input in ".*") {
        let parsed = RedactionProfile::from_profile_str(&input);
        let roundtrip = RedactionProfile::from_profile_str(parsed.as_str());

        prop_assert_eq!(parsed, roundtrip);
    }
}
