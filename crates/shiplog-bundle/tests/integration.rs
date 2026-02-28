use anyhow::Result;
use shiplog_bundle::{write_bundle_manifest, write_zip};
use shiplog_ids::RunId;
use shiplog_output_layout::*;
use shiplog_schema::bundle::{BundleManifest, BundleProfile};
use std::collections::HashMap;
use std::fs;
use std::io::Read;

fn make_test_dir(dir: &std::path::Path) {
    fs::write(dir.join(FILE_PACKET_MD), "# Packet\nSome content here.").unwrap();
    fs::write(
        dir.join(FILE_LEDGER_EVENTS_JSONL),
        r#"{"id":"evt1","kind":"pr"}"#,
    )
    .unwrap();
    fs::write(
        dir.join(FILE_COVERAGE_MANIFEST_JSON),
        r#"{"total":10,"covered":8}"#,
    )
    .unwrap();
    fs::write(
        dir.join(FILE_REDACTION_ALIASES_JSON),
        r#"{"version":1,"entries":{"secret":"alias1"}}"#,
    )
    .unwrap();

    let mgr = dir.join(DIR_PROFILES).join(PROFILE_MANAGER);
    fs::create_dir_all(&mgr).unwrap();
    fs::write(mgr.join(FILE_PACKET_MD), "# Manager Packet").unwrap();

    let pub_dir = dir.join(DIR_PROFILES).join(PROFILE_PUBLIC);
    fs::create_dir_all(&pub_dir).unwrap();
    fs::write(pub_dir.join(FILE_PACKET_MD), "# Public Packet").unwrap();
}

#[test]
fn zip_round_trip_internal() -> Result<()> {
    let dir = tempfile::tempdir()?;
    make_test_dir(dir.path());

    let zip_path = dir.path().join("internal.zip");
    write_zip(dir.path(), &zip_path, &BundleProfile::Internal)?;

    let file = fs::File::open(&zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    // Read back each file and verify content matches
    let mut zip_contents: HashMap<String, Vec<u8>> = HashMap::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf)?;
        zip_contents.insert(name, buf);
    }

    // packet.md should match
    assert_eq!(
        zip_contents.get(FILE_PACKET_MD).map(|b| b.as_slice()),
        Some(b"# Packet\nSome content here." as &[u8]),
        "packet.md content should round-trip through zip"
    );

    // coverage.manifest.json should match
    assert_eq!(
        zip_contents
            .get(FILE_COVERAGE_MANIFEST_JSON)
            .map(|b| b.as_slice()),
        Some(br#"{"total":10,"covered":8}"# as &[u8]),
    );

    // redaction.aliases.json must NOT be present
    assert!(
        !zip_contents.contains_key(FILE_REDACTION_ALIASES_JSON),
        "redaction.aliases.json must not be in zip"
    );

    Ok(())
}

#[test]
fn zip_round_trip_manager() -> Result<()> {
    let dir = tempfile::tempdir()?;
    make_test_dir(dir.path());

    let zip_path = dir.path().join("manager.zip");
    write_zip(dir.path(), &zip_path, &BundleProfile::Manager)?;

    let file = fs::File::open(&zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    assert_eq!(archive.len(), 2, "manager zip should have exactly 2 files");

    let mut names: Vec<String> = (0..archive.len())
        .map(|i| archive.name_for_index(i).unwrap().to_string())
        .collect();
    names.sort();

    assert!(names.contains(&FILE_COVERAGE_MANIFEST_JSON.to_string()));
    assert!(names.contains(&format!(
        "{DIR_PROFILES}/{PROFILE_MANAGER}/{FILE_PACKET_MD}"
    )));

    // Verify manager packet content
    let manager_path = format!("{DIR_PROFILES}/{PROFILE_MANAGER}/{FILE_PACKET_MD}");
    let mut entry = archive.by_name(&manager_path)?;
    let mut content = String::new();
    entry.read_to_string(&mut content)?;
    assert_eq!(content, "# Manager Packet");

    Ok(())
}

#[test]
fn zip_round_trip_public() -> Result<()> {
    let dir = tempfile::tempdir()?;
    make_test_dir(dir.path());

    let zip_path = dir.path().join("public.zip");
    write_zip(dir.path(), &zip_path, &BundleProfile::Public)?;

    let file = fs::File::open(&zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    assert_eq!(archive.len(), 2, "public zip should have exactly 2 files");

    let public_path = format!("{DIR_PROFILES}/{PROFILE_PUBLIC}/{FILE_PACKET_MD}");
    let mut entry = archive.by_name(&public_path)?;
    let mut content = String::new();
    entry.read_to_string(&mut content)?;
    assert_eq!(content, "# Public Packet");

    Ok(())
}

#[test]
fn manifest_json_round_trip() -> Result<()> {
    let dir = tempfile::tempdir()?;
    make_test_dir(dir.path());

    let run_id = RunId("round-trip-test".into());
    let manifest = write_bundle_manifest(dir.path(), &run_id, &BundleProfile::Internal)?;

    // Read the written JSON back and deserialize
    let json = fs::read_to_string(dir.path().join(FILE_BUNDLE_MANIFEST_JSON))?;
    let parsed: BundleManifest = serde_json::from_str(&json)?;

    assert_eq!(parsed.run_id, manifest.run_id);
    assert_eq!(parsed.profile, manifest.profile);
    assert_eq!(parsed.files.len(), manifest.files.len());
    for (original, parsed) in manifest.files.iter().zip(parsed.files.iter()) {
        assert_eq!(original.path, parsed.path);
        assert_eq!(original.sha256, parsed.sha256);
        assert_eq!(original.bytes, parsed.bytes);
    }

    Ok(())
}

#[test]
fn manifest_checksums_verified_against_sha256() -> Result<()> {
    let dir = tempfile::tempdir()?;
    make_test_dir(dir.path());

    let run_id = RunId("verify-test".into());
    let manifest = write_bundle_manifest(dir.path(), &run_id, &BundleProfile::Internal)?;

    // Independently compute SHA-256 for each file and compare
    for f in &manifest.files {
        let full_path = dir.path().join(&f.path);
        let bytes = fs::read(&full_path)?;

        use sha2::{Digest, Sha256};
        let digest = hex::encode(Sha256::digest(&bytes));

        assert_eq!(digest, f.sha256, "SHA-256 mismatch for {}", f.path);
        assert_eq!(
            bytes.len() as u64,
            f.bytes,
            "byte count mismatch for {}",
            f.path
        );
    }

    Ok(())
}

#[test]
fn zip_contents_match_manifest_files() -> Result<()> {
    for profile in [
        BundleProfile::Internal,
        BundleProfile::Manager,
        BundleProfile::Public,
    ] {
        let dir = tempfile::tempdir()?;
        let zip_dir = tempfile::tempdir()?;
        make_test_dir(dir.path());

        let run_id = RunId("match-test".into());
        let manifest = write_bundle_manifest(dir.path(), &run_id, &profile)?;

        let zip_path = zip_dir.path().join(format!("{:?}.zip", profile));
        write_zip(dir.path(), &zip_path, &profile)?;

        let file = fs::File::open(&zip_path)?;
        let archive = zip::ZipArchive::new(file)?;

        let mut zip_names: Vec<String> = (0..archive.len())
            .map(|i| archive.name_for_index(i).unwrap().to_string())
            .collect();
        zip_names.sort();

        let mut manifest_paths: Vec<String> =
            manifest.files.iter().map(|f| f.path.clone()).collect();
        manifest_paths.sort();

        assert_eq!(
            zip_names, manifest_paths,
            "zip entries should match manifest files for {:?}",
            profile
        );
    }

    Ok(())
}

#[test]
fn manifest_excludes_self() -> Result<()> {
    let dir = tempfile::tempdir()?;
    make_test_dir(dir.path());

    let run_id = RunId("self-exclude-test".into());

    // Write manifest twice to ensure first run's manifest.json doesn't appear in second
    write_bundle_manifest(dir.path(), &run_id, &BundleProfile::Internal)?;
    let manifest = write_bundle_manifest(dir.path(), &run_id, &BundleProfile::Internal)?;

    let paths: Vec<&str> = manifest.files.iter().map(|f| f.path.as_str()).collect();
    assert!(
        !paths.contains(&FILE_BUNDLE_MANIFEST_JSON),
        "bundle.manifest.json should never appear in its own manifest"
    );

    Ok(())
}

#[test]
fn empty_dir_produces_empty_manifest() -> Result<()> {
    let dir = tempfile::tempdir()?;

    let run_id = RunId("empty-test".into());
    let manifest = write_bundle_manifest(dir.path(), &run_id, &BundleProfile::Internal)?;

    assert!(
        manifest.files.is_empty(),
        "empty directory should produce empty manifest"
    );
    assert_eq!(manifest.profile, BundleProfile::Internal);

    Ok(())
}

#[test]
fn empty_dir_produces_empty_zip() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let zip_dir = tempfile::tempdir()?;

    let zip_path = zip_dir.path().join("empty.zip");
    write_zip(dir.path(), &zip_path, &BundleProfile::Internal)?;

    let file = fs::File::open(&zip_path)?;
    let archive = zip::ZipArchive::new(file)?;
    assert_eq!(archive.len(), 0, "empty dir should produce empty zip");

    Ok(())
}
