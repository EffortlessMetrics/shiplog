//! PR-scoped RIPR evidence producers.
//!
//! These commands write diff-scoped evidence under `target/ripr/`. They do not
//! publish comments and are advisory by default.

use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const RIPR_PR_JSON: &str = "target/ripr/pr/repo-exposure.json";
const RIPR_PR_MD: &str = "target/ripr/pr/repo-exposure.md";
const RIPR_REVIEW_JSON: &str = "target/ripr/review/comments.json";
const RIPR_REVIEW_MD: &str = "target/ripr/review/comments.md";

pub fn pr(workspace_root: &Path, check: bool) -> Result<()> {
    if check {
        return check_contract(workspace_root, &[RIPR_PR_JSON, RIPR_PR_MD]);
    }

    let ripr_bin = ripr_bin();
    let out_json = workspace_root.join(RIPR_PR_JSON);
    let out_md = workspace_root.join(RIPR_PR_MD);
    ensure_parent(&out_json)?;

    run_ripr_check_format(workspace_root, &ripr_bin, "repo-exposure-json", &out_json)?;
    run_ripr_check_format(workspace_root, &ripr_bin, "repo-exposure-md", &out_md)?;

    check_contract(workspace_root, &[RIPR_PR_JSON, RIPR_PR_MD])?;
    println!("ripr-pr: wrote target/ripr/pr repo exposure evidence");
    Ok(())
}

pub fn review_comments(workspace_root: &Path, check: bool) -> Result<()> {
    if check {
        return check_contract(workspace_root, &[RIPR_REVIEW_JSON, RIPR_REVIEW_MD]);
    }

    let ripr_bin = ripr_bin();
    let out_json = workspace_root.join(RIPR_REVIEW_JSON);
    ensure_parent(&out_json)?;

    let output = Command::new(&ripr_bin)
        .arg("review-comments")
        .arg("--root")
        .arg(workspace_root)
        .arg("--base")
        .arg(base_ref())
        .arg("--head")
        .arg(head_ref())
        .arg("--out")
        .arg(&out_json)
        .current_dir(workspace_root)
        .output()
        .with_context(|| format!("invoke {ripr_bin} for PR review guidance"))?;

    if !output.status.success() {
        bail!(
            "{ripr_bin} review-comments failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    check_contract(workspace_root, &[RIPR_REVIEW_JSON, RIPR_REVIEW_MD])?;
    println!("ripr-review-comments: wrote target/ripr/review guidance");
    Ok(())
}

fn run_ripr_check_format(
    workspace_root: &Path,
    ripr_bin: &str,
    format: &str,
    output_path: &Path,
) -> Result<()> {
    let output = Command::new(ripr_bin)
        .arg("check")
        .arg("--root")
        .arg(workspace_root)
        .arg("--base")
        .arg(base_ref())
        .arg("--format")
        .arg(format)
        .current_dir(workspace_root)
        .output()
        .with_context(|| format!("invoke {ripr_bin} for PR-scoped repo exposure {format}"))?;

    if !output.status.success() {
        bail!(
            "{ripr_bin} PR evidence ({format}) failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fs::write(output_path, output.stdout)
        .with_context(|| format!("write {}", output_path.display()))
}

fn check_contract(workspace_root: &Path, required: &[&str]) -> Result<()> {
    for rel in required {
        let path = workspace_root.join(rel);
        if !path.exists() {
            bail!("missing RIPR contract file: {rel}");
        }
        let metadata = fs::metadata(&path).with_context(|| format!("stat {rel}"))?;
        if metadata.len() == 0 {
            bail!("empty RIPR contract file: {rel}");
        }
        if rel.ends_with(".json") {
            let text = fs::read_to_string(&path).with_context(|| format!("read {rel}"))?;
            let parsed: Value = serde_json::from_str(&text)
                .with_context(|| format!("parse RIPR JSON contract {rel}"))?;
            if !parsed.is_object() {
                bail!("RIPR JSON contract {rel} must be an object");
            }
        }
    }
    println!("ripr: output contract is intact");
    Ok(())
}

fn ensure_parent(path: &Path) -> Result<()> {
    let parent = path
        .parent()
        .with_context(|| format!("resolve parent for {}", path.display()))?;
    fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))
}

fn ripr_bin() -> String {
    std::env::var("RIPR_BIN").unwrap_or_else(|_| "ripr".to_string())
}

fn base_ref() -> String {
    std::env::var("RIPR_BASE")
        .or_else(|_| std::env::var("GITHUB_BASE_REF").map(|base| format!("origin/{base}")))
        .unwrap_or_else(|_| "origin/main".to_string())
}

fn head_ref() -> String {
    std::env::var("RIPR_HEAD")
        .or_else(|_| std::env::var("GITHUB_HEAD_SHA"))
        .unwrap_or_else(|_| "HEAD".to_string())
}

#[allow(dead_code)]
fn workspace_relative(path: &Path, workspace_root: &Path) -> PathBuf {
    path.strip_prefix(workspace_root)
        .unwrap_or(path)
        .to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn check_contract_accepts_json_and_markdown() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("target/ripr/pr")).unwrap();
        fs::write(root.join(RIPR_PR_JSON), "{\"schema_version\":1}\n").unwrap();
        fs::write(root.join(RIPR_PR_MD), "# RIPR\n").unwrap();

        check_contract(root, &[RIPR_PR_JSON, RIPR_PR_MD]).unwrap();
    }

    #[test]
    fn check_contract_rejects_missing_file() {
        let dir = tempdir().unwrap();
        assert!(check_contract(dir.path(), &[RIPR_PR_JSON]).is_err());
    }
}
