//! Format stability tests for shiplog-bundle.
//!
//! Complements `snapshot.rs` with direct BundleManifest JSON/YAML snapshots
//! (no filesystem dependency), minimal variants, and edge-case values.

use shiplog_ids::RunId;
use shiplog_schema::bundle::{BundleManifest, BundleProfile, FileChecksum};

fn ts() -> chrono::DateTime<chrono::Utc> {
    chrono::TimeZone::with_ymd_and_hms(&chrono::Utc, 2025, 6, 1, 12, 0, 0).unwrap()
}

// ============================================================================
// Fully populated BundleManifest
// ============================================================================

#[test]
fn bundle_manifest_full_json() {
    let manifest = BundleManifest {
        run_id: RunId("stability_test_run".into()),
        generated_at: ts(),
        profile: BundleProfile::Internal,
        files: vec![
            FileChecksum {
                path: "packet.md".into(),
                sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into(),
                bytes: 8192,
            },
            FileChecksum {
                path: "ledger.events.jsonl".into(),
                sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".into(),
                bytes: 1024,
            },
            FileChecksum {
                path: "coverage.manifest.json".into(),
                sha256: "d7a8fbb307d7809469ca9abcb0082e4f8d5651e46d3cdb762d02d0bf37c9e592".into(),
                bytes: 512,
            },
            FileChecksum {
                path: "profiles/manager/packet.md".into(),
                sha256: "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08".into(),
                bytes: 4096,
            },
        ],
    };
    insta::assert_json_snapshot!(manifest);
}

#[test]
fn bundle_manifest_full_yaml() {
    let manifest = BundleManifest {
        run_id: RunId("yaml_test_run".into()),
        generated_at: ts(),
        profile: BundleProfile::Manager,
        files: vec![
            FileChecksum {
                path: "profiles/manager/packet.md".into(),
                sha256: "abc123".into(),
                bytes: 2048,
            },
            FileChecksum {
                path: "coverage.manifest.json".into(),
                sha256: "def456".into(),
                bytes: 256,
            },
        ],
    };
    insta::assert_yaml_snapshot!(manifest);
}

// ============================================================================
// Minimal BundleManifest
// ============================================================================

#[test]
fn bundle_manifest_empty_files() {
    let manifest = BundleManifest {
        run_id: RunId("empty".into()),
        generated_at: ts(),
        profile: BundleProfile::Internal,
        files: vec![],
    };
    insta::assert_json_snapshot!(manifest);
}

#[test]
fn bundle_manifest_public_profile() {
    let manifest = BundleManifest {
        run_id: RunId("public_test".into()),
        generated_at: ts(),
        profile: BundleProfile::Public,
        files: vec![FileChecksum {
            path: "profiles/public/packet.md".into(),
            sha256: "aabbcc".into(),
            bytes: 100,
        }],
    };
    insta::assert_json_snapshot!(manifest);
}

// ============================================================================
// Edge cases
// ============================================================================

#[test]
fn bundle_manifest_edge_cases() {
    let manifest = BundleManifest {
        run_id: RunId("".into()),
        generated_at: ts(),
        profile: BundleProfile::Internal,
        files: vec![
            FileChecksum {
                path: "".into(),
                sha256: "".into(),
                bytes: 0,
            },
            FileChecksum {
                path: "deeply/nested/path/to/file with spaces.txt".into(),
                sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".into(),
                bytes: u64::MAX,
            },
        ],
    };
    insta::assert_json_snapshot!(manifest);
}

#[test]
fn file_checksum_json_format() {
    let fc = FileChecksum {
        path: "packet.md".into(),
        sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into(),
        bytes: 4096,
    };
    insta::assert_json_snapshot!(fc);
}

#[test]
fn bundle_profile_all_variants_json() {
    let profiles = vec![
        BundleProfile::Internal,
        BundleProfile::Manager,
        BundleProfile::Public,
    ];
    insta::assert_json_snapshot!(profiles);
}
