//! PR-scoped RIPR evidence wrappers.

use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;

const RIPR_PR_DIR: &str = "target/ripr/pr";
const RIPR_REVIEW_DIR: &str = "target/ripr/review";

pub fn pr(workspace_root: &Path, check: bool) -> Result<()> {
    if check {
        return check_pr_contract(workspace_root);
    }
    let out_dir = workspace_root.join(RIPR_PR_DIR);
    fs::create_dir_all(&out_dir).context("create target/ripr/pr")?;
    run_ripr(
        workspace_root,
        &[
            "check",
            "--root",
            ".",
            "--format",
            "repo-exposure-json",
            "--out",
            "target/ripr/pr/repo-exposure.json",
        ],
    )?;
    run_ripr(
        workspace_root,
        &[
            "check",
            "--root",
            ".",
            "--format",
            "repo-exposure-md",
            "--out",
            "target/ripr/pr/repo-exposure.md",
        ],
    )?;
    check_pr_contract(workspace_root)
}

pub fn review_comments(workspace_root: &Path, check: bool) -> Result<()> {
    if check {
        return check_review_contract(workspace_root);
    }
    fs::create_dir_all(workspace_root.join(RIPR_REVIEW_DIR))
        .context("create target/ripr/review")?;
    run_ripr(
        workspace_root,
        &[
            "review-comments",
            "--root",
            ".",
            "--base",
            "origin/main",
            "--head",
            "HEAD",
            "--out",
            "target/ripr/review/comments.json",
        ],
    )?;
    check_review_contract(workspace_root)
}

fn run_ripr(workspace_root: &Path, args: &[&str]) -> Result<()> {
    let ripr_bin = std::env::var("RIPR_BIN").unwrap_or_else(|_| "ripr".to_string());
    let output = Command::new(&ripr_bin)
        .args(args)
        .current_dir(workspace_root)
        .output()
        .with_context(|| format!("invoke {ripr_bin}"))?;
    if !output.status.success() {
        bail!(
            "{ripr_bin} {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

fn check_pr_contract(workspace_root: &Path) -> Result<()> {
    read_json(&workspace_root.join(RIPR_PR_DIR).join("repo-exposure.json"))?;
    read_non_empty(&workspace_root.join(RIPR_PR_DIR).join("repo-exposure.md"))?;
    println!("ripr-pr: output contract is intact");
    Ok(())
}

fn check_review_contract(workspace_root: &Path) -> Result<()> {
    read_json(&workspace_root.join(RIPR_REVIEW_DIR).join("comments.json"))?;
    read_non_empty(&workspace_root.join(RIPR_REVIEW_DIR).join("comments.md"))?;
    println!("ripr-review-comments: output contract is intact");
    Ok(())
}

fn read_json(path: &Path) -> Result<Value> {
    let text = read_non_empty(path)?;
    serde_json::from_str(&text).with_context(|| format!("parse JSON {}", path.display()))
}

fn read_non_empty(path: &Path) -> Result<String> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    if text.trim().is_empty() {
        bail!("{} is empty", path.display());
    }
    Ok(text)
}
