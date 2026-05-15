//! PR-scoped RIPR evidence helpers.
//!
//! These commands intentionally write only to `target/ripr/**`. Public README
//! badges use `cargo xtask badges`, which asks RIPR for repo-scoped badge
//! evidence instead of reusing diff-scoped PR artifacts.

use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const PR_DIR: &str = "target/ripr/pr";
const REVIEW_DIR: &str = "target/ripr/review";

#[derive(Debug, Clone)]
pub struct RiprArgs {
    pub check: bool,
    pub base: Option<String>,
    pub head: Option<String>,
}

pub fn pr(workspace_root: &Path, args: RiprArgs) -> Result<()> {
    let out_dir = workspace_root.join(PR_DIR);
    if args.check {
        return check_pr_contract(&out_dir);
    }

    fs::create_dir_all(&out_dir).context("create RIPR PR evidence directory")?;
    let base = resolve_base(args.base.as_deref());
    let head = resolve_head(args.head.as_deref());
    verify_git_ref(workspace_root, &base)
        .with_context(|| format!("resolve RIPR base ref {base:?}"))?;
    verify_git_ref(workspace_root, &head)
        .with_context(|| format!("resolve RIPR head ref {head:?}"))?;

    let ripr_bin = ripr_bin();
    let json_out = out_dir.join("repo-exposure.json");
    let md_out = out_dir.join("repo-exposure.md");

    run_ripr(
        workspace_root,
        &ripr_bin,
        [
            "check".to_string(),
            "--root".to_string(),
            workspace_root.display().to_string(),
            "--base".to_string(),
            base,
            "--head".to_string(),
            head,
            "--format".to_string(),
            "repo-exposure-json".to_string(),
            "--out".to_string(),
            json_out.display().to_string(),
        ],
        "RIPR PR JSON evidence",
    )?;

    run_ripr(
        workspace_root,
        &ripr_bin,
        [
            "check".to_string(),
            "--root".to_string(),
            workspace_root.display().to_string(),
            "--base".to_string(),
            resolve_base(args.base.as_deref()),
            "--head".to_string(),
            resolve_head(args.head.as_deref()),
            "--format".to_string(),
            "repo-exposure-md".to_string(),
            "--out".to_string(),
            md_out.display().to_string(),
        ],
        "RIPR PR Markdown evidence",
    )?;

    check_pr_contract(&out_dir)
}

pub fn review_comments(workspace_root: &Path, args: RiprArgs) -> Result<()> {
    let out_dir = workspace_root.join(REVIEW_DIR);
    if args.check {
        return check_review_contract(&out_dir);
    }

    fs::create_dir_all(&out_dir).context("create RIPR review guidance directory")?;
    let base = resolve_base(args.base.as_deref());
    let head = resolve_head(args.head.as_deref());
    verify_git_ref(workspace_root, &base)
        .with_context(|| format!("resolve RIPR base ref {base:?}"))?;
    verify_git_ref(workspace_root, &head)
        .with_context(|| format!("resolve RIPR head ref {head:?}"))?;

    let json_out = out_dir.join("comments.json");
    run_ripr(
        workspace_root,
        &ripr_bin(),
        [
            "review-comments".to_string(),
            "--root".to_string(),
            workspace_root.display().to_string(),
            "--base".to_string(),
            base,
            "--head".to_string(),
            head,
            "--out".to_string(),
            json_out.display().to_string(),
        ],
        "RIPR review guidance",
    )?;

    check_review_contract(&out_dir)
}

fn ripr_bin() -> String {
    std::env::var("RIPR_BIN").unwrap_or_else(|_| "ripr".to_string())
}

fn resolve_base(explicit: Option<&str>) -> String {
    explicit
        .map(ToOwned::to_owned)
        .or_else(|| {
            std::env::var("GITHUB_BASE_SHA")
                .ok()
                .filter(|s| !s.is_empty())
        })
        .or_else(|| {
            std::env::var("GITHUB_BASE_REF")
                .ok()
                .filter(|s| !s.is_empty())
                .map(|s| format!("origin/{s}"))
        })
        .unwrap_or_else(|| "origin/main".to_string())
}

fn resolve_head(explicit: Option<&str>) -> String {
    explicit
        .map(ToOwned::to_owned)
        .or_else(|| {
            std::env::var("GITHUB_HEAD_SHA")
                .ok()
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| "HEAD".to_string())
}

fn verify_git_ref(workspace_root: &Path, reference: &str) -> Result<()> {
    let output = Command::new("git")
        .args(["rev-parse", "--verify", &format!("{reference}^{{commit}}")])
        .current_dir(workspace_root)
        .output()
        .context("invoke git rev-parse")?;

    if !output.status.success() {
        bail!(
            "git could not resolve {reference:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

fn run_ripr<I>(workspace_root: &Path, ripr_bin: &str, args: I, label: &str) -> Result<()>
where
    I: IntoIterator<Item = String>,
{
    let args: Vec<String> = args.into_iter().collect();
    let output = Command::new(ripr_bin)
        .args(&args)
        .current_dir(workspace_root)
        .output()
        .with_context(|| format!("invoke {ripr_bin} for {label}"))?;

    if !output.status.success() {
        bail!(
            "{ripr_bin} {label} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

fn check_pr_contract(out_dir: &Path) -> Result<()> {
    check_json(&out_dir.join("repo-exposure.json"))?;
    check_non_empty(&out_dir.join("repo-exposure.md"))?;
    println!("ripr-pr: output contract is intact");
    Ok(())
}

fn check_review_contract(out_dir: &Path) -> Result<()> {
    check_json(&out_dir.join("comments.json"))?;
    check_non_empty(&out_dir.join("comments.md"))?;
    println!("ripr-review-comments: output contract is intact");
    Ok(())
}

fn check_json(path: &Path) -> Result<()> {
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    if bytes.is_empty() {
        bail!("{} is empty", path.display());
    }
    let _: serde_json::Value = serde_json::from_slice(&bytes)
        .with_context(|| format!("parse {} as JSON", path.display()))?;
    Ok(())
}

fn check_non_empty(path: &Path) -> Result<()> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    if text.trim().is_empty() {
        bail!("{} is empty", path.display());
    }
    Ok(())
}

#[allow(dead_code)]
fn target_path(workspace_root: &Path, rel: &str) -> PathBuf {
    workspace_root.join(rel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_refs_win() {
        assert_eq!(resolve_base(Some("base-sha")), "base-sha");
        assert_eq!(resolve_head(Some("head-sha")), "head-sha");
    }
}
