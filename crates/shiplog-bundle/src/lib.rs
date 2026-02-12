use anyhow::{Context, Result};
use chrono::Utc;
use sha2::{Digest, Sha256};
use shiplog_ids::RunId;
use shiplog_schema::bundle::{BundleManifest, FileChecksum};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub fn write_bundle_manifest(out_dir: &Path, run_id: &RunId) -> Result<BundleManifest> {
    let mut files = Vec::new();

    for path in walk_files(out_dir)? {
        // don't checksum the manifest itself if it already exists
        if path.file_name().and_then(|s| s.to_str()) == Some("bundle.manifest.json") {
            continue;
        }

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
            } else {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}
