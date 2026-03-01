//! Snapshot tests for shiplog-redaction-profile serialization formats.

use shiplog_redaction_profile::RedactionProfile;

#[test]
fn snapshot_json_internal() {
    let json = serde_json::to_string_pretty(&RedactionProfile::Internal).unwrap();
    insta::assert_snapshot!("json_internal", json);
}

#[test]
fn snapshot_json_manager() {
    let json = serde_json::to_string_pretty(&RedactionProfile::Manager).unwrap();
    insta::assert_snapshot!("json_manager", json);
}

#[test]
fn snapshot_json_public() {
    let json = serde_json::to_string_pretty(&RedactionProfile::Public).unwrap();
    insta::assert_snapshot!("json_public", json);
}

#[test]
fn snapshot_display_all_variants() {
    let variants = [
        RedactionProfile::Internal,
        RedactionProfile::Manager,
        RedactionProfile::Public,
    ];
    let display: Vec<String> = variants.iter().map(|v| v.to_string()).collect();
    insta::assert_snapshot!("display_all_variants", display.join("\n"));
}

#[test]
fn snapshot_json_roundtrip_all_variants() {
    let variants = [
        RedactionProfile::Internal,
        RedactionProfile::Manager,
        RedactionProfile::Public,
    ];
    let roundtrips: Vec<String> = variants
        .iter()
        .map(|v| {
            let json = serde_json::to_string(v).unwrap();
            let decoded: RedactionProfile = serde_json::from_str(&json).unwrap();
            format!("{} -> {} -> {}", v.as_str(), json, decoded.as_str())
        })
        .collect();
    insta::assert_snapshot!("json_roundtrip_all", roundtrips.join("\n"));
}
