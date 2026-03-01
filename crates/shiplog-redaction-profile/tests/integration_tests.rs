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

#[test]
fn all_variants_have_distinct_as_str() {
    let variants = [
        RedactionProfile::Internal,
        RedactionProfile::Manager,
        RedactionProfile::Public,
    ];
    let strs: Vec<&str> = variants.iter().map(|v| v.as_str()).collect();
    let unique: std::collections::HashSet<&str> = strs.iter().copied().collect();
    assert_eq!(
        strs.len(),
        unique.len(),
        "all as_str() values must be unique"
    );
}

#[test]
fn clone_and_copy_produce_equal_values() {
    let original = RedactionProfile::Manager;
    let cloned = original;
    let copied = original;
    assert_eq!(original, cloned);
    assert_eq!(original, copied);
}

#[test]
fn debug_format_is_meaningful() {
    let debug = format!("{:?}", RedactionProfile::Internal);
    assert!(
        debug.contains("Internal"),
        "debug format should contain variant name"
    );
}

#[test]
fn hash_set_deduplication_works() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(RedactionProfile::Public);
    set.insert(RedactionProfile::Public);
    set.insert(RedactionProfile::Manager);
    assert_eq!(set.len(), 2);
}

#[test]
fn from_str_is_infallible() {
    // FromStr::Err is Infallible, so parse never fails
    let result: Result<RedactionProfile, _> = "anything".parse();
    assert!(result.is_ok());
}

#[test]
fn case_sensitive_parsing() {
    // Only lowercase matches
    assert_eq!(
        RedactionProfile::from_profile_str("Internal"),
        RedactionProfile::Public
    );
    assert_eq!(
        RedactionProfile::from_profile_str("MANAGER"),
        RedactionProfile::Public
    );
    assert_eq!(
        RedactionProfile::from_profile_str("PUBLIC"),
        RedactionProfile::Public
    );
}

#[test]
fn serde_json_array_roundtrip() {
    let profiles = vec![
        RedactionProfile::Internal,
        RedactionProfile::Manager,
        RedactionProfile::Public,
    ];
    let json = serde_json::to_string(&profiles).expect("serialize");
    let decoded: Vec<RedactionProfile> = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(profiles, decoded);
}
