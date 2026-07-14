//! The promote command verifies an exact swarm head and prepares the source
//! promotion branch. It deliberately stops before creating or merging a PR.

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::promotion_body;

const SWARM_REPO: &str = "EffortlessMetrics/shiplog-swarm";
const ROUTED_WORKFLOW: &str = "EM CI Routed Shiplog Rust";
const REQUIRED_RESULT: &str = "Shiplog Rust Small Result";

pub struct PromoteInputs {
    pub workspace_root: PathBuf,
    pub swarm_sha: String,
    pub dry_run: bool,
    pub source_ref: String,
    pub swarm_ref: String,
    pub source_remote: String,
    pub branch: Option<String>,
    pub output: PathBuf,
}

#[derive(Debug, Deserialize)]
struct RunReceipt {
    #[serde(rename = "databaseId")]
    database_id: u64,
    #[serde(rename = "headSha")]
    head_sha: String,
    status: String,
    conclusion: Option<String>,
}

pub fn run(inputs: PromoteInputs) -> Result<()> {
    let swarm_sha = git_output(
        &inputs.workspace_root,
        &["rev-parse", &format!("{}^{{commit}}", inputs.swarm_sha)],
    )
    .context("promote: resolve --swarm-sha")?;
    let swarm_ref_sha = git_output(
        &inputs.workspace_root,
        &["rev-parse", &format!("{}^{{commit}}", inputs.swarm_ref)],
    )
    .with_context(|| format!("promote: resolve {}", inputs.swarm_ref))?;

    ensure_shared_promotion_base(&inputs.workspace_root, &inputs.source_ref, &swarm_sha)?;
    ensure_ancestor(
        &inputs.workspace_root,
        &swarm_sha,
        &swarm_ref_sha,
        "the requested swarm head must be reachable from the swarm ref",
    )?;
    let receipt = green_swarm_receipt(&swarm_sha)?;

    let branch = inputs
        .branch
        .unwrap_or_else(|| format!("promote/swarm-current-{}", &swarm_sha[..12]));
    let existing = git_output(
        &inputs.workspace_root,
        &[
            "ls-remote",
            &inputs.source_remote,
            &format!("refs/heads/{branch}"),
        ],
    )?;
    let existing_sha = existing.split_whitespace().next().unwrap_or_default();
    if !existing_sha.is_empty() {
        ensure_ancestor(
            &inputs.workspace_root,
            existing_sha,
            &swarm_sha,
            "existing promotion branch is not fast-forwardable to the requested swarm head",
        )?;
    }

    println!("promote: swarm head {swarm_sha}");
    println!(
        "promote: green {REQUIRED_RESULT} run {}",
        receipt.database_id
    );
    println!("promote: source ref {}", inputs.source_ref);
    println!("promote: branch {branch}");
    if inputs.dry_run {
        println!("promote: dry-run; would push {swarm_sha}:refs/heads/{branch}");
        println!(
            "promote: dry-run; would generate {}",
            display_path(&inputs.workspace_root, &inputs.output)
        );
        return Ok(());
    }

    if existing_sha != swarm_sha {
        git_status(
            &inputs.workspace_root,
            &[
                "push",
                &inputs.source_remote,
                &format!("{swarm_sha}:refs/heads/{branch}"),
            ],
        )
        .with_context(|| format!("promote: push {branch}"))?;
    } else {
        println!("promote: branch already points at requested head");
    }

    promotion_body::run(promotion_body::PromotionBodyInputs {
        workspace_root: inputs.workspace_root.clone(),
        source_ref: inputs.source_ref,
        swarm_ref: inputs.swarm_ref,
        swarm_head: Some(swarm_sha),
        included_swarm_prs: Vec::new(),
        swarm_pr_run: None,
        swarm_main_run: Some(receipt.database_id.to_string()),
        source_pr_run: None,
        source_post_merge_run: None,
        output: inputs.output,
    })?;

    println!("promote: open a regular merge PR; do not squash");
    println!("promote: after merge run cargo xtask repo-contract-report");
    Ok(())
}

fn green_swarm_receipt(swarm_sha: &str) -> Result<RunReceipt> {
    let output = Command::new("gh")
        .args([
            "run",
            "list",
            "--repo",
            SWARM_REPO,
            "--workflow",
            ROUTED_WORKFLOW,
            "--branch",
            "main",
            "--limit",
            "20",
            "--json",
            "databaseId,headSha,status,conclusion",
        ])
        .output()
        .context("promote: run gh to inspect swarm CI")?;
    if !output.status.success() {
        bail!(
            "promote: gh run list failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let runs: Vec<RunReceipt> =
        serde_json::from_slice(&output.stdout).context("promote: parse swarm CI JSON")?;
    runs.into_iter()
        .find(|run| {
            run.head_sha == swarm_sha
                && run.status == "completed"
                && run.conclusion.as_deref() == Some("success")
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "promote: no completed successful {REQUIRED_RESULT} run for {swarm_sha}"
            )
        })
}

fn ensure_ancestor(workspace_root: &Path, older: &str, newer: &str, message: &str) -> Result<()> {
    let status = Command::new("git")
        .args(["merge-base", "--is-ancestor", older, newer])
        .current_dir(workspace_root)
        .status()
        .with_context(|| format!("promote: verify ancestry {older}..{newer}"))?;
    if !status.success() {
        bail!("promote: {message}: {older} is not an ancestor of {newer}");
    }
    Ok(())
}

fn ensure_shared_promotion_base(
    workspace_root: &Path,
    source_ref: &str,
    swarm_sha: &str,
) -> Result<()> {
    let direct = Command::new("git")
        .args(["merge-base", "--is-ancestor", source_ref, swarm_sha])
        .current_dir(workspace_root)
        .status()
        .with_context(|| format!("promote: verify ancestry {source_ref}..{swarm_sha}"))?;
    if direct.success() {
        return Ok(());
    }

    let parents = git_output(workspace_root, &["show", "-s", "--format=%P", source_ref])?;
    let mut parent_iter = parents.split_whitespace();
    let Some(_source_parent) = parent_iter.next() else {
        bail!(
            "promote: {source_ref} is not an ancestor of {swarm_sha} and is not a two-parent promotion merge"
        );
    };
    let Some(swarm_parent) = parent_iter.next() else {
        bail!(
            "promote: {source_ref} is not an ancestor of {swarm_sha} and is not a two-parent promotion merge"
        );
    };
    if parent_iter.next().is_some() {
        bail!(
            "promote: {source_ref} is not an ancestor of {swarm_sha} and is not a two-parent promotion merge"
        );
    }
    ensure_ancestor(
        workspace_root,
        swarm_parent,
        swarm_sha,
        "source promotion second parent must be an ancestor of the swarm head",
    )
}

fn git_output(workspace_root: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(workspace_root)
        .output()
        .with_context(|| format!("promote: run git {}", args.join(" ")))?;
    if !output.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn git_status(workspace_root: &Path, args: &[&str]) -> Result<()> {
    let status = Command::new("git")
        .args(args)
        .current_dir(workspace_root)
        .status()
        .with_context(|| format!("promote: run git {}", args.join(" ")))?;
    if !status.success() {
        bail!("git {} failed", args.join(" "));
    }
    Ok(())
}

fn display_path(workspace_root: &Path, path: &Path) -> String {
    path.strip_prefix(workspace_root)
        .map(|relative| relative.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn branch_name_is_stable_for_a_head() {
        let sha = "0123456789abcdef0123456789abcdef01234567";
        assert_eq!(
            format!("promote/swarm-current-{}", &sha[..12]),
            "promote/swarm-current-0123456789ab"
        );
    }
}
