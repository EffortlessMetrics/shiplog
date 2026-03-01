use shiplog_bundle::write_bundle_manifest;
use shiplog_ids::RunId;
use shiplog_output_layout::*;
use shiplog_schema::bundle::BundleProfile;
use std::fs;

fn make_test_dir(dir: &std::path::Path) {
    fs::write(dir.join(FILE_PACKET_MD), "# Packet").unwrap();
    fs::write(dir.join(FILE_LEDGER_EVENTS_JSONL), "").unwrap();
    fs::write(dir.join(FILE_COVERAGE_MANIFEST_JSON), "{}").unwrap();
    fs::write(
        dir.join(FILE_REDACTION_ALIASES_JSON),
        r#"{"version":1,"entries":{}}"#,
    )
    .unwrap();

    let mgr = dir.join(DIR_PROFILES).join(PROFILE_MANAGER);
    fs::create_dir_all(&mgr).unwrap();
    fs::write(mgr.join(FILE_PACKET_MD), "# Manager").unwrap();

    let pub_dir = dir.join(DIR_PROFILES).join(PROFILE_PUBLIC);
    fs::create_dir_all(&pub_dir).unwrap();
    fs::write(pub_dir.join(FILE_PACKET_MD), "# Public").unwrap();
}

/// Redact non-deterministic fields from serialized manifest JSON.
fn redact_manifest_json(json: &str) -> String {
    let mut v: serde_json::Value = serde_json::from_str(json).unwrap();
    v["generated_at"] = serde_json::Value::String("[TIMESTAMP]".into());
    v["run_id"] = serde_json::Value::String("[RUN_ID]".into());
    serde_json::to_string_pretty(&v).unwrap()
}

#[test]
fn snapshot_internal_manifest_format() {
    let dir = tempfile::tempdir().unwrap();
    make_test_dir(dir.path());

    let run_id = RunId("snapshot-test".into());
    write_bundle_manifest(dir.path(), &run_id, &BundleProfile::Internal).unwrap();

    let raw = fs::read_to_string(dir.path().join(FILE_BUNDLE_MANIFEST_JSON)).unwrap();
    let redacted = redact_manifest_json(&raw);
    insta::assert_snapshot!("internal_manifest_format", redacted);
}

#[test]
fn snapshot_manager_manifest_format() {
    let dir = tempfile::tempdir().unwrap();
    make_test_dir(dir.path());

    let run_id = RunId("snapshot-test".into());
    write_bundle_manifest(dir.path(), &run_id, &BundleProfile::Manager).unwrap();

    let raw = fs::read_to_string(dir.path().join(FILE_BUNDLE_MANIFEST_JSON)).unwrap();
    let redacted = redact_manifest_json(&raw);
    insta::assert_snapshot!("manager_manifest_format", redacted);
}

#[test]
fn snapshot_public_manifest_format() {
    let dir = tempfile::tempdir().unwrap();
    make_test_dir(dir.path());

    let run_id = RunId("snapshot-test".into());
    write_bundle_manifest(dir.path(), &run_id, &BundleProfile::Public).unwrap();

    let raw = fs::read_to_string(dir.path().join(FILE_BUNDLE_MANIFEST_JSON)).unwrap();
    let redacted = redact_manifest_json(&raw);
    insta::assert_snapshot!("public_manifest_format", redacted);
}

#[test]
fn snapshot_checksum_format() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(FILE_PACKET_MD), "hello world").unwrap();

    let run_id = RunId("checksum-test".into());
    let manifest = write_bundle_manifest(dir.path(), &run_id, &BundleProfile::Internal).unwrap();

    // Snapshot just the file checksum entries (deterministic content)
    let checksums: Vec<String> = manifest
        .files
        .iter()
        .map(|f| format!("{}  {} ({} bytes)", f.sha256, f.path, f.bytes))
        .collect();
    insta::assert_snapshot!("checksum_format", checksums.join("\n"));
}
