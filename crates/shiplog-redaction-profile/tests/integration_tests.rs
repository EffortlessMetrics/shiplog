//! Integration tests for shiplog-redaction-profile public API.

use shiplog_redaction_profile::RedactionProfile;

#[test]
fn serde_roundtrip_preserves_profile() {
    for profile in [
        RedactionProfile::Internal,
        RedactionProfile::Manager,
        RedactionProfile::Public,
    ] {
        let json = serde_json::to_string(&profile).expect("serialize profile");
        let decoded: RedactionProfile = serde_json::from_str(&json).expect("deserialize profile");
        assert_eq!(decoded, profile);
    }
}

#[test]
fn parse_and_display_agree_for_known_profiles() {
    for raw in ["internal", "manager", "public"] {
        let parsed: RedactionProfile = raw.parse().expect("infallible parse");
        assert_eq!(parsed.as_str(), raw);
        assert_eq!(parsed.to_string(), raw);
    }
}

#[test]
fn parse_defaults_unknown_to_public() {
    let parsed: RedactionProfile = "something-else".parse().expect("infallible parse");
    assert_eq!(parsed, RedactionProfile::Public);
}
