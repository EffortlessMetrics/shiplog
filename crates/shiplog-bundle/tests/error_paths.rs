//! Error-path and failure-mode tests for the bundle writer.
//!
//! Exercises invalid file paths, missing directories, empty directories,
//! and sha256 on non-existent files.

use shiplog_bundle::{write_bundle_manifest, write_zip};
use shiplog_ids::RunId;
use shiplog_schema::bundle::BundleProfile;
use std::fs::File;
use std::path::Path;

// ---------------------------------------------------------------------------
// sha256_file on non-existent file (tested indirectly via manifest)
// ---------------------------------------------------------------------------

#[test]
fn write_bundle_manifest_on_nonexistent_dir_errors() {
    let nonexistent = Path::new("H:\\this\\path\\does\\not\\exist\\at\\all");
    let run_id = RunId::now("test");

    let result = write_bundle_manifest(nonexistent, &run_id, &BundleProfile::Internal);
    assert!(result.is_err(), "should fail on nonexistent directory");
}

#[test]
fn write_bundle_manifest_on_empty_dir_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let run_id = RunId::now("test");

    let result = write_bundle_manifest(dir.path(), &run_id, &BundleProfile::Internal);
    assert!(result.is_ok(), "empty dir should succeed with zero files");
    let manifest = result.unwrap();
    assert!(
        manifest.files.is_empty(),
        "no files should be in the manifest"
    );
}

#[test]
fn write_bundle_manifest_on_empty_dir_manager_profile_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let run_id = RunId::now("test");

    let manifest =
        write_bundle_manifest(dir.path(), &run_id, &BundleProfile::Manager).unwrap();
    assert!(manifest.files.is_empty());
    assert_eq!(manifest.profile, BundleProfile::Manager);
}

// ---------------------------------------------------------------------------
// write_zip error paths
// ---------------------------------------------------------------------------

#[test]
fn write_zip_to_invalid_path_errors() {
    let dir = tempfile::tempdir().unwrap();
    // Write a file so the dir isn't empty
    std::fs::write(dir.path().join("packet.md"), "# Test").unwrap();

    // Try to write zip to a path with null bytes (invalid)
    let invalid_zip = dir.path().join("\0bad.zip");
    let result = write_zip(dir.path(), &invalid_zip, &BundleProfile::Internal);
    assert!(result.is_err(), "null byte path should fail");
}

#[test]
fn write_zip_on_nonexistent_source_dir_errors() {
    let dir = tempfile::tempdir().unwrap();
    let zip_path = dir.path().join("output.zip");
    let nonexistent = Path::new("H:\\no\\such\\dir\\for\\zip");

    let result = write_zip(nonexistent, &zip_path, &BundleProfile::Internal);
    assert!(result.is_err(), "non-existent source dir should fail");
}

#[test]
fn write_zip_empty_dir_produces_valid_archive() {
    let src_dir = tempfile::tempdir().unwrap();
    let out_dir = tempfile::tempdir().unwrap();
    let zip_path = out_dir.path().join("empty.zip");

    let result = write_zip(src_dir.path(), &zip_path, &BundleProfile::Internal);
    assert!(result.is_ok(), "empty dir should produce valid zip");

    // Verify the zip is valid and contains no entries
    let file = File::open(&zip_path).unwrap();
    let archive = zip::ZipArchive::new(file).unwrap();
    assert_eq!(archive.len(), 0, "zip should have 0 entries");
}

// ---------------------------------------------------------------------------
// Profile filtering edge cases
// ---------------------------------------------------------------------------

#[test]
fn manager_profile_on_dir_without_profiles_subdir_returns_empty() {
    let dir = tempfile::tempdir().unwrap();
    // Only write top-level files, no profiles/ subdirectory
    std::fs::write(dir.path().join("packet.md"), "# Top").unwrap();
    std::fs::write(dir.path().join("coverage.manifest.json"), "{}").unwrap();

    let run_id = RunId::now("test");
    let manifest =
        write_bundle_manifest(dir.path(), &run_id, &BundleProfile::Manager).unwrap();

    // Manager profile requires profiles/manager/packet.md — the top-level
    // packet.md doesn't match, but coverage.manifest.json does.
    let paths: Vec<&str> = manifest.files.iter().map(|f| f.path.as_str()).collect();
    assert!(
        paths.contains(&"coverage.manifest.json"),
        "coverage should be included"
    );
    assert!(
        !paths.contains(&"packet.md"),
        "top-level packet should not be in manager profile"
    );
}

#[test]
fn public_profile_on_dir_without_profiles_subdir_returns_only_coverage() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("packet.md"), "# Top").unwrap();
    std::fs::write(dir.path().join("coverage.manifest.json"), "{}").unwrap();
    std::fs::write(dir.path().join("ledger.events.jsonl"), "").unwrap();

    let run_id = RunId::now("test");
    let manifest =
        write_bundle_manifest(dir.path(), &run_id, &BundleProfile::Public).unwrap();

    let paths: Vec<&str> = manifest.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths.len(), 1, "only coverage should be included");
    assert!(paths.contains(&"coverage.manifest.json"));
}

// ---------------------------------------------------------------------------
// Bundle manifest contains correct checksums
// ---------------------------------------------------------------------------

#[test]
fn manifest_checksums_are_deterministic() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("packet.md"), "hello world").unwrap();

    let run_id = RunId::now("test");
    let m1 =
        write_bundle_manifest(dir.path(), &run_id, &BundleProfile::Internal).unwrap();
    // Remove the manifest file so it doesn't interfere with second run
    std::fs::remove_file(dir.path().join("bundle.manifest.json")).unwrap();
    let m2 =
        write_bundle_manifest(dir.path(), &run_id, &BundleProfile::Internal).unwrap();

    // Checksums should be identical for the same content
    assert_eq!(m1.files.len(), m2.files.len());
    for (f1, f2) in m1.files.iter().zip(m2.files.iter()) {
        assert_eq!(f1.sha256, f2.sha256, "checksums should be deterministic");
        assert_eq!(f1.bytes, f2.bytes);
    }
}
