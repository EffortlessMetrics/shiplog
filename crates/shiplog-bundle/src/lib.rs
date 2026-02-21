//! Bundle writer for shiplog run outputs.
//!
//! Generates `bundle.manifest.json` (file checksums + sizes) and builds
//! profile-scoped zip archives for `internal`, `manager`, and `public` handoff.

use anyhow::{Context, Result};
use chrono::Utc;
use sha2::{Digest, Sha256};
use shiplog_ids::RunId;
use shiplog_output_layout::{
    DIR_PROFILES, FILE_BUNDLE_MANIFEST_JSON, FILE_COVERAGE_MANIFEST_JSON, FILE_PACKET_MD,
    FILE_REDACTION_ALIASES_JSON, PROFILE_MANAGER, PROFILE_PUBLIC,
};
use shiplog_schema::bundle::{BundleManifest, BundleProfile, FileChecksum};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Files excluded from bundles regardless of profile. `redaction.aliases.json`
/// contains plaintext-to-alias mappings that would defeat redaction.
/// `bundle.manifest.json` is excluded because it is written *after*
/// the file walk and must not checksum itself.
const ALWAYS_EXCLUDED: &[&str] = &[FILE_REDACTION_ALIASES_JSON, FILE_BUNDLE_MANIFEST_JSON];

/// Decide whether `rel_path` (forward-slash normalised, relative to the run
/// directory) should be included in a bundle for the given profile.
fn is_scoped_include(rel_path: &str, profile: &BundleProfile) -> bool {
    match profile {
        BundleProfile::Internal => true,
        BundleProfile::Manager => {
            rel_path == format!("{DIR_PROFILES}/{PROFILE_MANAGER}/{FILE_PACKET_MD}")
                || rel_path == FILE_COVERAGE_MANIFEST_JSON
        }
        BundleProfile::Public => {
            rel_path == format!("{DIR_PROFILES}/{PROFILE_PUBLIC}/{FILE_PACKET_MD}")
                || rel_path == FILE_COVERAGE_MANIFEST_JSON
        }
    }
}

pub fn write_bundle_manifest(
    out_dir: &Path,
    run_id: &RunId,
    profile: &BundleProfile,
) -> Result<BundleManifest> {
    let mut files = Vec::new();

    for path in walk_files(out_dir, profile)? {
        let bytes = std::fs::metadata(&path)?.len();
        let sha256 = sha256_file(&path)?;
        let rel = path
            .strip_prefix(out_dir)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");

        files.push(FileChecksum {
            path: rel,
            sha256,
            bytes,
        });
    }

    let manifest = BundleManifest {
        run_id: run_id.clone(),
        generated_at: Utc::now(),
        profile: profile.clone(),
        files,
    };

    let text = serde_json::to_string_pretty(&manifest)?;
    std::fs::write(out_dir.join(FILE_BUNDLE_MANIFEST_JSON), text)?;
    Ok(manifest)
}

pub fn write_zip(out_dir: &Path, zip_path: &Path, profile: &BundleProfile) -> Result<()> {
    let file = File::create(zip_path).with_context(|| format!("create zip {zip_path:?}"))?;
    let mut zip = zip::ZipWriter::new(file);
    let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);

    for path in walk_files(out_dir, profile)? {
        let rel = path
            .strip_prefix(out_dir)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");

        zip.start_file(rel, opts)?;
        let mut f = File::open(&path)?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;
        zip.write_all(&buf)?;
    }

    zip.finish()?;
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String> {
    let mut f = File::open(path)?;
    let mut h = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex::encode(h.finalize()))
}

fn walk_files(root: &Path, profile: &BundleProfile) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(p) = stack.pop() {
        for entry in std::fs::read_dir(&p)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                if ALWAYS_EXCLUDED.contains(&name) {
                    continue;
                }
                // Normalise backslashes to forward slashes for cross-platform matching
                let rel = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                if is_scoped_include(&rel, profile) {
                    out.push(path);
                }
            } else {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use shiplog_output_layout::FILE_LEDGER_EVENTS_JSONL;

    /// Helper: create a minimal run directory for testing.
    fn make_test_dir(dir: &Path) {
        std::fs::write(dir.join(FILE_PACKET_MD), "# Packet").unwrap();
        std::fs::write(dir.join(FILE_LEDGER_EVENTS_JSONL), "").unwrap();
        std::fs::write(dir.join(FILE_COVERAGE_MANIFEST_JSON), "{}").unwrap();
        std::fs::write(
            dir.join(FILE_REDACTION_ALIASES_JSON),
            r#"{"version":1,"entries":{}}"#,
        )
        .unwrap();

        let mgr = dir.join(DIR_PROFILES).join(PROFILE_MANAGER);
        std::fs::create_dir_all(&mgr).unwrap();
        std::fs::write(mgr.join(FILE_PACKET_MD), "# Manager").unwrap();

        let pub_dir = dir.join(DIR_PROFILES).join(PROFILE_PUBLIC);
        std::fs::create_dir_all(&pub_dir).unwrap();
        std::fs::write(pub_dir.join(FILE_PACKET_MD), "# Public").unwrap();
    }

    fn file_names(files: &[PathBuf]) -> Vec<String> {
        files
            .iter()
            .filter_map(|p| p.file_name().and_then(|s| s.to_str()).map(String::from))
            .collect()
    }

    fn rel_paths(root: &Path, files: &[PathBuf]) -> Vec<String> {
        files
            .iter()
            .map(|p| {
                p.strip_prefix(root)
                    .unwrap_or(p)
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect()
    }

    #[test]
    fn walk_files_excludes_redaction_aliases() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(FILE_PACKET_MD), "# Packet").unwrap();
        std::fs::write(dir.path().join(FILE_REDACTION_ALIASES_JSON), "{}").unwrap();
        std::fs::write(dir.path().join(FILE_LEDGER_EVENTS_JSONL), "").unwrap();

        let files = walk_files(dir.path(), &BundleProfile::Internal).unwrap();
        let names = file_names(&files);

        assert!(names.contains(&FILE_PACKET_MD.to_string()));
        assert!(names.contains(&FILE_LEDGER_EVENTS_JSONL.to_string()));
        assert!(
            !names.contains(&FILE_REDACTION_ALIASES_JSON.to_string()),
            "redaction.aliases.json should be excluded from walk_files"
        );
    }

    #[test]
    fn bundle_manifest_excludes_redaction_aliases() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(FILE_PACKET_MD), "# Packet").unwrap();
        std::fs::write(
            dir.path().join(FILE_REDACTION_ALIASES_JSON),
            r#"{"version":1,"entries":{}}"#,
        )
        .unwrap();
        std::fs::write(dir.path().join(FILE_LEDGER_EVENTS_JSONL), "").unwrap();

        let run_id = shiplog_ids::RunId::now("test");
        let manifest =
            write_bundle_manifest(dir.path(), &run_id, &BundleProfile::Internal).unwrap();
        let paths: Vec<&str> = manifest.files.iter().map(|f| f.path.as_str()).collect();

        assert!(
            !paths
                .iter()
                .any(|p| p.contains(FILE_REDACTION_ALIASES_JSON)),
            "redaction.aliases.json should not appear in bundle manifest"
        );
        assert!(
            !paths.iter().any(|p| p.contains(FILE_BUNDLE_MANIFEST_JSON)),
            "bundle.manifest.json should not appear in bundle manifest"
        );
        assert!(
            paths.iter().any(|p| p.contains(FILE_PACKET_MD)),
            "packet.md should appear in bundle manifest"
        );
    }

    #[test]
    fn zip_excludes_redaction_aliases() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(FILE_PACKET_MD), "# Packet").unwrap();
        std::fs::write(
            dir.path().join(FILE_REDACTION_ALIASES_JSON),
            r#"{"version":1,"entries":{}}"#,
        )
        .unwrap();

        let zip_path = dir.path().join("test.zip");
        write_zip(dir.path(), &zip_path, &BundleProfile::Internal).unwrap();

        let file = File::open(&zip_path).unwrap();
        let archive = zip::ZipArchive::new(file).unwrap();
        let names: Vec<String> = (0..archive.len())
            .map(|i| archive.name_for_index(i).unwrap().to_string())
            .collect();

        assert!(
            names.iter().any(|n| n.contains(FILE_PACKET_MD)),
            "packet.md should be in zip"
        );
        assert!(
            !names
                .iter()
                .any(|n| n.contains(FILE_REDACTION_ALIASES_JSON)),
            "redaction.aliases.json should not be in zip"
        );
    }

    #[test]
    fn manager_profile_includes_only_manager_packet_and_coverage() {
        let dir = tempfile::tempdir().unwrap();
        make_test_dir(dir.path());

        let files = walk_files(dir.path(), &BundleProfile::Manager).unwrap();
        let rels = rel_paths(dir.path(), &files);

        assert!(rels.contains(&FILE_COVERAGE_MANIFEST_JSON.to_string()));
        assert!(rels.contains(&format!(
            "{DIR_PROFILES}/{PROFILE_MANAGER}/{FILE_PACKET_MD}"
        )));
        assert!(!rels.contains(&FILE_PACKET_MD.to_string()));
        assert!(!rels.contains(&FILE_LEDGER_EVENTS_JSONL.to_string()));
        assert!(!rels.contains(&format!("{DIR_PROFILES}/{PROFILE_PUBLIC}/{FILE_PACKET_MD}")));
        assert_eq!(rels.len(), 2);
    }

    #[test]
    fn public_profile_includes_only_public_packet_and_coverage() {
        let dir = tempfile::tempdir().unwrap();
        make_test_dir(dir.path());

        let files = walk_files(dir.path(), &BundleProfile::Public).unwrap();
        let rels = rel_paths(dir.path(), &files);

        assert!(rels.contains(&FILE_COVERAGE_MANIFEST_JSON.to_string()));
        assert!(rels.contains(&format!("{DIR_PROFILES}/{PROFILE_PUBLIC}/{FILE_PACKET_MD}")));
        assert!(!rels.contains(&FILE_PACKET_MD.to_string()));
        assert!(!rels.contains(&format!(
            "{DIR_PROFILES}/{PROFILE_MANAGER}/{FILE_PACKET_MD}"
        )));
        assert_eq!(rels.len(), 2);
    }

    #[test]
    fn all_profiles_exclude_aliases() {
        let dir = tempfile::tempdir().unwrap();
        make_test_dir(dir.path());

        for profile in [
            BundleProfile::Internal,
            BundleProfile::Manager,
            BundleProfile::Public,
        ] {
            let files = walk_files(dir.path(), &profile).unwrap();
            let names = file_names(&files);
            assert!(
                !names.contains(&FILE_REDACTION_ALIASES_JSON.to_string()),
                "aliases leaked in {profile:?}"
            );
        }
    }

    #[test]
    fn manifest_respects_profile() {
        let dir = tempfile::tempdir().unwrap();
        make_test_dir(dir.path());

        let run_id = shiplog_ids::RunId::now("test");
        let manifest = write_bundle_manifest(dir.path(), &run_id, &BundleProfile::Manager).unwrap();

        assert_eq!(manifest.profile, BundleProfile::Manager);
        assert_eq!(manifest.files.len(), 2);
    }

    #[test]
    fn zip_respects_profile() {
        let dir = tempfile::tempdir().unwrap();
        make_test_dir(dir.path());

        let zip_path = dir.path().join("test.zip");
        write_zip(dir.path(), &zip_path, &BundleProfile::Public).unwrap();

        let file = File::open(&zip_path).unwrap();
        let archive = zip::ZipArchive::new(file).unwrap();
        let names: Vec<String> = (0..archive.len())
            .map(|i| archive.name_for_index(i).unwrap().to_string())
            .collect();

        assert_eq!(names.len(), 2, "public zip should have exactly 2 files");
        assert!(
            names
                .iter()
                .any(|n| n.contains(&format!("{DIR_PROFILES}/{PROFILE_PUBLIC}/{FILE_PACKET_MD}")))
        );
        assert!(
            names
                .iter()
                .any(|n| n.contains(FILE_COVERAGE_MANIFEST_JSON))
        );
    }
}
