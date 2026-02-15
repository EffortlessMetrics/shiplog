use anyhow::{Context, Result};
use chrono::Utc;
use sha2::{Digest, Sha256};
use shiplog_ids::RunId;
use shiplog_schema::bundle::{BundleManifest, FileChecksum};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Files excluded from bundles. `redaction.aliases.json` contains
/// plaintext-to-alias mappings that would defeat redaction.
/// `bundle.manifest.json` is excluded because it is written *after*
/// the file walk and must not checksum itself.
const EXCLUDED_FILENAMES: &[&str] = &["redaction.aliases.json", "bundle.manifest.json"];

pub fn write_bundle_manifest(out_dir: &Path, run_id: &RunId) -> Result<BundleManifest> {
    let mut files = Vec::new();

    for path in walk_files(out_dir)? {
        let bytes = std::fs::metadata(&path)?.len();
        let sha256 = sha256_file(&path)?;
        let rel = path
            .strip_prefix(out_dir)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();

        files.push(FileChecksum {
            path: rel,
            sha256,
            bytes,
        });
    }

    let manifest = BundleManifest {
        run_id: run_id.clone(),
        generated_at: Utc::now(),
        files,
    };

    let text = serde_json::to_string_pretty(&manifest)?;
    std::fs::write(out_dir.join("bundle.manifest.json"), text)?;
    Ok(manifest)
}

pub fn write_zip(out_dir: &Path, zip_path: &Path) -> Result<()> {
    let file = File::create(zip_path).with_context(|| format!("create zip {zip_path:?}"))?;
    let mut zip = zip::ZipWriter::new(file);
    let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);

    for path in walk_files(out_dir)? {
        let rel = path
            .strip_prefix(out_dir)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();

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

fn walk_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(p) = stack.pop() {
        for entry in std::fs::read_dir(&p)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                if !EXCLUDED_FILENAMES.contains(&name) {
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

    #[test]
    fn walk_files_excludes_redaction_aliases() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("packet.md"), "# Packet").unwrap();
        std::fs::write(dir.path().join("redaction.aliases.json"), "{}").unwrap();
        std::fs::write(dir.path().join("ledger.events.jsonl"), "").unwrap();

        let files = walk_files(dir.path()).unwrap();
        let names: Vec<&str> = files
            .iter()
            .filter_map(|p| p.file_name().and_then(|s| s.to_str()))
            .collect();

        assert!(names.contains(&"packet.md"));
        assert!(names.contains(&"ledger.events.jsonl"));
        assert!(
            !names.contains(&"redaction.aliases.json"),
            "redaction.aliases.json should be excluded from walk_files"
        );
    }

    #[test]
    fn bundle_manifest_excludes_redaction_aliases() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("packet.md"), "# Packet").unwrap();
        std::fs::write(
            dir.path().join("redaction.aliases.json"),
            r#"{"version":1,"entries":{}}"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("ledger.events.jsonl"), "").unwrap();

        let run_id = shiplog_ids::RunId::now("test");
        let manifest = write_bundle_manifest(dir.path(), &run_id).unwrap();
        let paths: Vec<&str> = manifest.files.iter().map(|f| f.path.as_str()).collect();

        assert!(
            !paths.iter().any(|p| p.contains("redaction.aliases.json")),
            "redaction.aliases.json should not appear in bundle manifest"
        );
        assert!(
            !paths.iter().any(|p| p.contains("bundle.manifest.json")),
            "bundle.manifest.json should not appear in bundle manifest"
        );
        assert!(
            paths.iter().any(|p| p.contains("packet.md")),
            "packet.md should appear in bundle manifest"
        );
    }

    #[test]
    fn zip_excludes_redaction_aliases() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("packet.md"), "# Packet").unwrap();
        std::fs::write(
            dir.path().join("redaction.aliases.json"),
            r#"{"version":1,"entries":{}}"#,
        )
        .unwrap();

        let zip_path = dir.path().join("test.zip");
        write_zip(dir.path(), &zip_path).unwrap();

        let file = File::open(&zip_path).unwrap();
        let archive = zip::ZipArchive::new(file).unwrap();
        let names: Vec<String> = (0..archive.len())
            .map(|i| archive.name_for_index(i).unwrap().to_string())
            .collect();

        assert!(
            names.iter().any(|n| n.contains("packet.md")),
            "packet.md should be in zip"
        );
        assert!(
            !names.iter().any(|n| n.contains("redaction.aliases.json")),
            "redaction.aliases.json should not be in zip"
        );
    }
}
