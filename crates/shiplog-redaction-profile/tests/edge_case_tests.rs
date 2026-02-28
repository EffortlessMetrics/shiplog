//! Edge case tests for shiplog-redaction-profile.

use shiplog_redaction_profile::RedactionProfile;
use std::collections::{HashMap, HashSet};

// ============================================================================
// Unicode and special character inputs
// ============================================================================

#[test]
fn unicode_input_defaults_to_public() {
    let cases = [
        "日本語",
        "中文",
        "한국어",
        "العربية",
        "emoji: 🔒🔓🛡️",
        "Ünïcödé",
        "café",
        "\u{200B}",         // zero-width space
        "\u{FEFF}",         // byte order mark
        "in\u{200B}ternal", // "internal" with zero-width space
    ];
    for input in cases {
        assert_eq!(
            RedactionProfile::from_profile_str(input),
            RedactionProfile::Public,
            "Unicode input {input:?} should default to Public"
        );
    }
}

#[test]
fn null_and_whitespace_inputs_default_to_public() {
    let cases = ["", " ", "\t", "\n", "\r\n", "   internal   ", " manager "];
    for input in cases {
        assert_eq!(
            RedactionProfile::from_profile_str(input),
            RedactionProfile::Public,
            "Whitespace input {input:?} should default to Public"
        );
    }
}

// ============================================================================
// Very long strings
// ============================================================================

#[test]
fn very_long_string_defaults_to_public() {
    let long = "a".repeat(100_000);
    assert_eq!(
        RedactionProfile::from_profile_str(&long),
        RedactionProfile::Public
    );
}

#[test]
fn long_prefix_match_does_not_succeed() {
    // "internal" followed by extra chars should not match
    let prefixed = format!("internal{}", "x".repeat(10_000));
    assert_eq!(
        RedactionProfile::from_profile_str(&prefixed),
        RedactionProfile::Public
    );
}

// ============================================================================
// All three profiles produce distinct outputs
// ============================================================================

#[test]
fn all_profiles_have_distinct_as_str_values() {
    let strs: HashSet<&str> = [
        RedactionProfile::Internal,
        RedactionProfile::Manager,
        RedactionProfile::Public,
    ]
    .iter()
    .map(|p| p.as_str())
    .collect();
    assert_eq!(strs.len(), 3);
}

#[test]
fn all_profiles_have_distinct_display_values() {
    let displays: HashSet<String> = [
        RedactionProfile::Internal,
        RedactionProfile::Manager,
        RedactionProfile::Public,
    ]
    .iter()
    .map(|p| p.to_string())
    .collect();
    assert_eq!(displays.len(), 3);
}

#[test]
fn all_profiles_have_distinct_serde_json_values() {
    let jsons: HashSet<String> = [
        RedactionProfile::Internal,
        RedactionProfile::Manager,
        RedactionProfile::Public,
    ]
    .iter()
    .map(|p| serde_json::to_string(p).unwrap())
    .collect();
    assert_eq!(jsons.len(), 3);
}

// ============================================================================
// Profile comparison and ordering
// ============================================================================

#[test]
fn profiles_are_equal_to_themselves() {
    assert_eq!(RedactionProfile::Internal, RedactionProfile::Internal);
    assert_eq!(RedactionProfile::Manager, RedactionProfile::Manager);
    assert_eq!(RedactionProfile::Public, RedactionProfile::Public);
}

#[test]
fn different_profiles_are_not_equal() {
    assert_ne!(RedactionProfile::Internal, RedactionProfile::Manager);
    assert_ne!(RedactionProfile::Internal, RedactionProfile::Public);
    assert_ne!(RedactionProfile::Manager, RedactionProfile::Public);
}

#[test]
fn profiles_work_as_hashmap_keys() {
    let mut map = HashMap::new();
    map.insert(RedactionProfile::Internal, "internal-value");
    map.insert(RedactionProfile::Manager, "manager-value");
    map.insert(RedactionProfile::Public, "public-value");

    assert_eq!(map.len(), 3);
    assert_eq!(map[&RedactionProfile::Internal], "internal-value");
    assert_eq!(map[&RedactionProfile::Manager], "manager-value");
    assert_eq!(map[&RedactionProfile::Public], "public-value");
}

#[test]
fn profile_dedup_in_hashset_works() {
    let mut hset = HashSet::new();
    hset.insert(RedactionProfile::Public);
    hset.insert(RedactionProfile::Public);
    hset.insert(RedactionProfile::Manager);
    hset.insert(RedactionProfile::Manager);
    hset.insert(RedactionProfile::Internal);
    hset.insert(RedactionProfile::Internal);
    assert_eq!(hset.len(), 3);
}

// ============================================================================
// Serde edge cases
// ============================================================================

#[test]
fn serde_bincode_style_roundtrip() {
    // Test serializing to JSON Value and back
    for profile in [
        RedactionProfile::Internal,
        RedactionProfile::Manager,
        RedactionProfile::Public,
    ] {
        let value = serde_json::to_value(profile).unwrap();
        let decoded: RedactionProfile = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, profile);
    }
}

#[test]
fn serde_from_string_literal_json() {
    let internal: RedactionProfile = serde_json::from_str("\"Internal\"").unwrap();
    assert_eq!(internal, RedactionProfile::Internal);

    let manager: RedactionProfile = serde_json::from_str("\"Manager\"").unwrap();
    assert_eq!(manager, RedactionProfile::Manager);

    let public: RedactionProfile = serde_json::from_str("\"Public\"").unwrap();
    assert_eq!(public, RedactionProfile::Public);
}

#[test]
fn serde_rejects_lowercase_json_variant() {
    // Serde default derives use PascalCase for enum variant names.
    // "internal" (lowercase) should fail to deserialize.
    let result: Result<RedactionProfile, _> = serde_json::from_str("\"internal\"");
    assert!(
        result.is_err(),
        "lowercase 'internal' should not deserialize via serde"
    );
}

#[test]
fn serde_rejects_invalid_json_types() {
    let result: Result<RedactionProfile, _> = serde_json::from_str("42");
    assert!(result.is_err());

    let result: Result<RedactionProfile, _> = serde_json::from_str("null");
    assert!(result.is_err());

    let result: Result<RedactionProfile, _> = serde_json::from_str("true");
    assert!(result.is_err());
}

// ============================================================================
// Copy semantics
// ============================================================================

#[test]
fn copy_semantics_preserved() {
    let a = RedactionProfile::Manager;
    let b = a; // Copy
    let c = a; // Still valid because Copy
    assert_eq!(a, b);
    assert_eq!(b, c);
}

// ============================================================================
// from_profile_str vs FromStr consistency
// ============================================================================

#[test]
fn from_profile_str_and_from_str_agree_on_edge_cases() {
    let edge_cases = [
        "",
        "internal",
        "manager",
        "public",
        "INTERNAL",
        "Manager",
        "PUBLIC",
        " ",
        "\0",
        "internal\0",
        "🔒",
    ];
    for input in edge_cases {
        let from_profile = RedactionProfile::from_profile_str(input);
        let from_str: RedactionProfile = input.parse().unwrap();
        assert_eq!(from_profile, from_str, "Mismatch for input {input:?}");
    }
}

// ============================================================================
// Canonical roundtrip stability
// ============================================================================

#[test]
fn double_parse_roundtrip_is_stable() {
    for input in ["internal", "manager", "public", "unknown", "", "🔒"] {
        let first = RedactionProfile::from_profile_str(input);
        let second = RedactionProfile::from_profile_str(first.as_str());
        let third = RedactionProfile::from_profile_str(second.as_str());
        assert_eq!(first, second);
        assert_eq!(second, third);
    }
}
