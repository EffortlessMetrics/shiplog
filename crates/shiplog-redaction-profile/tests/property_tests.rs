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

    #[test]
    fn serde_json_roundtrip_preserves_identity(input in ".*") {
        let parsed = RedactionProfile::from_profile_str(&input);
        let json = serde_json::to_string(&parsed).unwrap();
        let decoded: RedactionProfile = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(parsed, decoded);
    }

    #[test]
    fn from_str_matches_from_profile_str(input in ".*") {
        let a = RedactionProfile::from_profile_str(&input);
        let b: RedactionProfile = input.parse().unwrap();
        prop_assert_eq!(a, b);
    }

    #[test]
    fn display_matches_as_str(input in ".*") {
        let parsed = RedactionProfile::from_profile_str(&input);
        prop_assert_eq!(parsed.to_string(), parsed.as_str());
    }

    #[test]
    fn hash_eq_consistency(a_str in ".*", b_str in ".*") {
        use std::collections::HashSet;
        let a = RedactionProfile::from_profile_str(&a_str);
        let b = RedactionProfile::from_profile_str(&b_str);
        if a == b {
            let mut set = HashSet::new();
            set.insert(a);
            prop_assert!(set.contains(&b));
        }
    }

    #[test]
    fn only_three_possible_outputs(input in ".*") {
        let parsed = RedactionProfile::from_profile_str(&input);
        prop_assert!(
            parsed == RedactionProfile::Internal
            || parsed == RedactionProfile::Manager
            || parsed == RedactionProfile::Public
        );
    }
}
