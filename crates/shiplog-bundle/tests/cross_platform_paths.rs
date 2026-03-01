//! Cross-platform path handling tests for shiplog-bundle.
//!
//! Ensures bundle manifests and zip archives handle paths with spaces,
//! unicode, and special characters correctly on all platforms.

use anyhow::Result;
use shiplog_bundle::{write_bundle_manifest, write_zip};
use shiplog_ids::RunId;
use shiplog_output_layout::*;
use shiplog_schema::bundle::{BundleManifest, BundleProfile};
use std::fs;
use std::io::Read;
use std::path::Path;

fn make_test_dir(dir: &Path) {
    fs::write(dir.join(FILE_PACKET_MD), "# Packet\ncontent").unwrap();
    fs::write(dir.join(FILE_LEDGER_EVENTS_JSONL), r#"{"id":"evt1"}"#).unwrap();
    fs::write(
        dir.join(FILE_COVERAGE_MANIFEST_JSON),
        r#"{"total":10,"covered":8}"#,
    )
    .unwrap();

    let mgr = dir.join(DIR_PROFILES).join(PROFILE_MANAGER);
    fs::create_dir_all(&mgr).unwrap();
    fs::write(mgr.join(FILE_PACKET_MD), "# Manager Packet").unwrap();

    let pub_dir = dir.join(DIR_PROFILES).join(PROFILE_PUBLIC);
    fs::create_dir_all(&pub_dir).unwrap();
    fs::write(pub_dir.join(FILE_PACKET_MD), "# Public Packet").unwrap();
}

// ---------------------------------------------------------------------------
// Manifest with special path characters
// ---------------------------------------------------------------------------

#[test]
fn manifest_with_spaces_in_parent_path() -> Result<()> {
    let root = tempfile::tempdir()?;
    let dir = root.path().join("my output dir");
    fs::create_dir_all(&dir)?;
    make_test_dir(&dir);

    let run_id = RunId("spaces-test".into());
    let manifest = write_bundle_manifest(&dir, &run_id, &BundleProfile::Internal)?;

    assert!(!manifest.files.is_empty());
    // All relative paths in manifest should use forward slashes (no backslashes)
    for f in &manifest.files {
        assert!(
            !f.path.contains('\\'),
            "manifest path should not contain backslash: {}",
            f.path
        );
    }

    Ok(())
}

#[test]
fn manifest_with_unicode_in_parent_path() -> Result<()> {
    let root = tempfile::tempdir()?;
    let dir = root.path().join("出力ディレクトリ");
    fs::create_dir_all(&dir)?;
    make_test_dir(&dir);

    let run_id = RunId("unicode-test".into());
    let manifest = write_bundle_manifest(&dir, &run_id, &BundleProfile::Internal)?;

    assert!(!manifest.files.is_empty());
    // Verify the manifest JSON can be round-tripped
    let json = fs::read_to_string(dir.join(FILE_BUNDLE_MANIFEST_JSON))?;
    let parsed: BundleManifest = serde_json::from_str(&json)?;
    assert_eq!(parsed.files.len(), manifest.files.len());

    Ok(())
}

#[test]
fn manifest_with_special_chars_in_parent_path() -> Result<()> {
    let root = tempfile::tempdir()?;
    let dir = root.path().join("project (v2.0-beta)");
    fs::create_dir_all(&dir)?;
    make_test_dir(&dir);

    let run_id = RunId("special-chars-test".into());
    let manifest = write_bundle_manifest(&dir, &run_id, &BundleProfile::Internal)?;

    assert!(!manifest.files.is_empty());
    // Relative paths should only reference files inside the dir, not the parent path
    for f in &manifest.files {
        assert!(
            !f.path.contains("project (v2.0-beta)"),
            "relative path should not contain parent dir name: {}",
            f.path
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Zip with special path characters
// ---------------------------------------------------------------------------

#[test]
fn zip_with_spaces_in_paths() -> Result<()> {
    let root = tempfile::tempdir()?;
    let dir = root.path().join("zip source dir");
    fs::create_dir_all(&dir)?;
    make_test_dir(&dir);

    let zip_path = root.path().join("output archive.zip");
    write_zip(&dir, &zip_path, &BundleProfile::Internal)?;

    let file = fs::File::open(&zip_path)?;
    let archive = zip::ZipArchive::new(file)?;
    assert!(!archive.is_empty(), "zip should not be empty");

    // All zip entry names should use forward slashes
    for i in 0..archive.len() {
        let name = archive.name_for_index(i).unwrap().to_string();
        assert!(
            !name.contains('\\'),
            "zip entry should not contain backslash: {}",
            name
        );
    }

    Ok(())
}

#[test]
fn zip_with_unicode_in_paths() -> Result<()> {
    let root = tempfile::tempdir()?;
    let dir = root.path().join("données_de_sortie");
    fs::create_dir_all(&dir)?;
    make_test_dir(&dir);

    let zip_path = root.path().join("archive_données.zip");
    write_zip(&dir, &zip_path, &BundleProfile::Internal)?;

    let file = fs::File::open(&zip_path)?;
    let archive = zip::ZipArchive::new(file)?;
    assert!(!archive.is_empty());

    Ok(())
}

#[test]
fn zip_round_trip_with_spaces_preserves_content() -> Result<()> {
    let root = tempfile::tempdir()?;
    let dir = root.path().join("round trip dir");
    fs::create_dir_all(&dir)?;
    make_test_dir(&dir);

    let zip_path = root.path().join("round trip.zip");
    write_zip(&dir, &zip_path, &BundleProfile::Internal)?;

    let file = fs::File::open(&zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    // Verify packet.md content survives round-trip
    let mut entry = archive.by_name(FILE_PACKET_MD)?;
    let mut content = String::new();
    entry.read_to_string(&mut content)?;
    assert_eq!(content, "# Packet\ncontent");

    Ok(())
}

// ---------------------------------------------------------------------------
// Manager/public profile zip with special paths
// ---------------------------------------------------------------------------

#[test]
fn manager_zip_with_spaces_in_path() -> Result<()> {
    let root = tempfile::tempdir()?;
    let dir = root.path().join("manager zip dir");
    fs::create_dir_all(&dir)?;
    make_test_dir(&dir);

    let zip_path = root.path().join("manager output.zip");
    write_zip(&dir, &zip_path, &BundleProfile::Manager)?;

    let file = fs::File::open(&zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    assert_eq!(archive.len(), 2, "manager zip should have 2 files");

    // Verify manager packet can be read via forward-slash path
    let manager_path = format!("{DIR_PROFILES}/{PROFILE_MANAGER}/{FILE_PACKET_MD}");
    let mut entry = archive.by_name(&manager_path)?;
    let mut content = String::new();
    entry.read_to_string(&mut content)?;
    assert_eq!(content, "# Manager Packet");

    Ok(())
}

// ---------------------------------------------------------------------------
// Manifest paths are always forward-slash normalized
// ---------------------------------------------------------------------------

#[test]
fn manifest_paths_use_forward_slashes_for_profile_entries() -> Result<()> {
    let dir = tempfile::tempdir()?;
    make_test_dir(dir.path());

    let run_id = RunId("fwd-slash-test".into());
    let manifest = write_bundle_manifest(dir.path(), &run_id, &BundleProfile::Internal)?;

    for f in &manifest.files {
        assert!(
            !f.path.contains('\\'),
            "manifest path contains backslash: {}",
            f.path
        );
        // Profile paths should use forward slashes
        if f.path.contains(DIR_PROFILES) {
            assert!(
                f.path.starts_with(&format!("{DIR_PROFILES}/")),
                "profile path not forward-slash prefixed: {}",
                f.path
            );
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Checksums are stable regardless of platform path style
// ---------------------------------------------------------------------------

#[test]
fn checksums_are_correct_with_unicode_parent_path() -> Result<()> {
    let root = tempfile::tempdir()?;
    let dir = root.path().join("チェックサム");
    fs::create_dir_all(&dir)?;
    make_test_dir(&dir);

    let run_id = RunId("checksum-unicode".into());
    let manifest = write_bundle_manifest(&dir, &run_id, &BundleProfile::Internal)?;

    for f in &manifest.files {
        let full_path = dir.join(&f.path);
        let bytes = fs::read(&full_path)?;

        use sha2::{Digest, Sha256};
        let digest = hex::encode(Sha256::digest(&bytes));
        assert_eq!(
            digest, f.sha256,
            "SHA-256 mismatch for {} in unicode path",
            f.path
        );
    }

    Ok(())
}
