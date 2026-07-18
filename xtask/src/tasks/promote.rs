//! The promote command verifies an exact swarm head and prepares the source
//! promotion branch. It deliberately stops before creating or merging a PR.

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
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

/// Machine-readable plan/receipt for the prepared promotion. Emitted for agents
/// and `repo-contract-report`; deterministic for a given repository state.
#[derive(Debug, Serialize)]
struct PromotePlan {
    swarm_head: String,
    source_ref: String,
    source_head: String,
    merge_base: Option<String>,
    branch: String,
    required_check: String,
    ci_run_id: u64,
    included_swarm_prs: Vec<String>,
    dry_run: bool,
    next_actions: Vec<String>,
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

    let source_head = git_output(
        &inputs.workspace_root,
        &["rev-parse", &format!("{}^{{commit}}", inputs.source_ref)],
    )
    .with_context(|| format!("promote: resolve {}", inputs.source_ref))?;
    let merge_base = git_output(
        &inputs.workspace_root,
        &["merge-base", &inputs.source_ref, &swarm_sha],
    )
    .ok();
    let included_swarm_prs =
        included_swarm_prs(&inputs.workspace_root, &inputs.source_ref, &swarm_sha)?;

    println!("promote: swarm head {swarm_sha}");
    println!(
        "promote: green {REQUIRED_RESULT} run {}",
        receipt.database_id
    );
    println!("promote: source ref {}", inputs.source_ref);
    println!("promote: branch {branch}");
    println!(
        "promote: included swarm PRs since {}: {}",
        inputs.source_ref,
        if included_swarm_prs.is_empty() {
            "(none)".to_string()
        } else {
            included_swarm_prs.join(", ")
        }
    );

    let next_actions = vec![
        format!(
            "Push {swarm_sha}:refs/heads/{branch} to {}.",
            inputs.source_remote
        ),
        "Open a regular-merge source promotion PR from the branch; do not squash.".to_string(),
        "After merge, run `cargo xtask repo-contract-report`.".to_string(),
    ];
    let plan = PromotePlan {
        swarm_head: swarm_sha.clone(),
        source_ref: inputs.source_ref.clone(),
        source_head,
        merge_base,
        branch: branch.clone(),
        required_check: REQUIRED_RESULT.to_string(),
        ci_run_id: receipt.database_id,
        included_swarm_prs: included_swarm_prs.clone(),
        dry_run: inputs.dry_run,
        next_actions,
    };
    let receipt_path = write_plan_receipt(&inputs.workspace_root, &inputs.output, &plan)?;
    println!(
        "promote: wrote plan receipt {}",
        display_path(&inputs.workspace_root, &receipt_path)
    );

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
        included_swarm_prs,
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

/// Enumerate the swarm PRs squash-merged between the last promoted source head
/// and the requested swarm head, inferred from `source_ref..swarm_sha`.
fn included_swarm_prs(
    workspace_root: &Path,
    source_ref: &str,
    swarm_sha: &str,
) -> Result<Vec<String>> {
    let log = git_output(
        workspace_root,
        &[
            "log",
            "--no-merges",
            "--format=%s",
            &format!("{source_ref}..{swarm_sha}"),
        ],
    )
    .with_context(|| format!("promote: enumerate swarm PRs {source_ref}..{swarm_sha}"))?;
    Ok(extract_swarm_pr_receipts(log.lines()))
}

/// Extract `owner/repo#N` receipts from squash-merge commit subjects, keeping
/// first-seen order and de-duplicating.
fn extract_swarm_pr_receipts<'a>(subjects: impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut receipts = Vec::new();
    for subject in subjects {
        if let Some(number) = extract_trailing_pr_number(subject)
            && seen.insert(number)
        {
            receipts.push(format!("{SWARM_REPO}#{number}"));
        }
    }
    receipts
}

/// Parse the trailing `(#N)` PR number from a squash-merge commit subject.
fn extract_trailing_pr_number(subject: &str) -> Option<u64> {
    let start = subject.rfind("(#")?;
    let rest = &subject[start + 2..];
    let end = rest.find(')')?;
    rest[..end].parse().ok()
}

/// Write the machine-readable promotion plan next to the generated body and
/// return its path. `output` is the generated-body file path (the same value
/// `promotion_body` writes to); the receipt is placed in that file's parent
/// directory. This is a build artifact under `target/`, not a tracked
/// mutation, so it is emitted in `--dry-run` too.
fn write_plan_receipt(workspace_root: &Path, output: &Path, plan: &PromotePlan) -> Result<PathBuf> {
    let receipt_path = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .join("promote-receipt.json");
    let absolute = if receipt_path.is_absolute() {
        receipt_path.clone()
    } else {
        workspace_root.join(&receipt_path)
    };
    if let Some(parent) = absolute.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("promote: create {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(plan).context("promote: serialize plan receipt")?;
    fs::write(&absolute, format!("{json}\n"))
        .with_context(|| format!("promote: write {}", absolute.display()))?;
    Ok(absolute)
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
    use super::*;

    #[test]
    fn branch_name_is_stable_for_a_head() {
        let sha = "0123456789abcdef0123456789abcdef01234567";
        assert_eq!(
            format!("promote/swarm-current-{}", &sha[..12]),
            "promote/swarm-current-0123456789ab"
        );
    }

    #[test]
    fn extract_trailing_pr_number_parses_squash_subject() {
        assert_eq!(
            extract_trailing_pr_number("feat(xtask): add idempotent swarm promotion prep (#238)"),
            Some(238)
        );
        // Uses the trailing marker, not an inline reference.
        assert_eq!(
            extract_trailing_pr_number("fix: follow up on (#12) with the real fix (#345)"),
            Some(345)
        );
    }

    #[test]
    fn extract_trailing_pr_number_rejects_subjects_without_marker() {
        assert_eq!(extract_trailing_pr_number("chore: no pr marker"), None);
        assert_eq!(extract_trailing_pr_number("weird (#notanumber)"), None);
        assert_eq!(
            extract_trailing_pr_number("open paren (#5 but no close"),
            None
        );
    }

    #[test]
    fn extract_swarm_pr_receipts_formats_dedups_and_keeps_order() {
        let subjects = [
            "fix(ci): make auxiliary smoke lanes deterministic (#253)",
            "fix(control-plane): classify source-only governance (#251)",
            "deps: bump clap (#248)",
            "docs: touch-up with no pr marker",
            // Duplicate number is de-duplicated.
            "revert: re-land classify governance (#251)",
        ];
        let receipts = extract_swarm_pr_receipts(subjects.into_iter());
        assert_eq!(
            receipts,
            vec![
                "EffortlessMetrics/shiplog-swarm#253".to_string(),
                "EffortlessMetrics/shiplog-swarm#251".to_string(),
                "EffortlessMetrics/shiplog-swarm#248".to_string(),
            ]
        );
    }

    #[test]
    fn extract_swarm_pr_receipts_empty_for_no_prs() {
        let subjects = ["chore: no marker", "another plain subject"];
        assert!(extract_swarm_pr_receipts(subjects.into_iter()).is_empty());
    }

    #[test]
    fn plan_receipt_serializes_expected_fields_and_is_deterministic() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("target/source-of-truth/promotion-body.md");
        let plan = PromotePlan {
            swarm_head: "c4fdba223d1c5c5b99a95b159ab8123d83d4b842".to_string(),
            source_ref: "origin/main".to_string(),
            source_head: "ee4c7e0b628e4495f3044397b0566fe06f1e567c".to_string(),
            merge_base: Some("df611d5".to_string()),
            branch: "promote/swarm-current-c4fdba223d1c".to_string(),
            required_check: REQUIRED_RESULT.to_string(),
            ci_run_id: 1234,
            included_swarm_prs: vec!["EffortlessMetrics/shiplog-swarm#238".to_string()],
            dry_run: true,
            next_actions: vec!["Open a regular-merge source promotion PR.".to_string()],
        };
        let path = write_plan_receipt(dir.path(), &output, &plan).unwrap();
        assert_eq!(
            path,
            dir.path()
                .join("target/source-of-truth/promote-receipt.json")
        );
        let first = std::fs::read_to_string(&path).unwrap();
        assert!(first.contains("\"swarm_head\""));
        assert!(first.contains("\"source_head\""));
        assert!(first.contains("\"merge_base\""));
        assert!(first.contains("\"included_swarm_prs\""));
        assert!(first.contains("\"ci_run_id\": 1234"));
        assert!(first.contains("\"branch\""));
        assert!(first.contains("\"next_actions\""));
        assert!(first.contains("EffortlessMetrics/shiplog-swarm#238"));
        // Deterministic for the same plan.
        let second_path = write_plan_receipt(dir.path(), &output, &plan).unwrap();
        assert_eq!(first, std::fs::read_to_string(second_path).unwrap());
    }
}
