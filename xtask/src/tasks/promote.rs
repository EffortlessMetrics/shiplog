//! The promote command verifies an exact swarm head and prepares the source
//! promotion branch. It deliberately stops before creating or merging a PR.

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use super::promotion_body;

const SWARM_REPO: &str = "EffortlessMetrics/shiplog-swarm";
const SOURCE_REPO: &str = "EffortlessMetrics/shiplog";
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
    pub allow_historical: bool,
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

#[derive(Debug, Deserialize)]
struct RunJobs {
    jobs: Vec<JobReceipt>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct JobReceipt {
    #[serde(rename = "databaseId")]
    database_id: u64,
    name: String,
    status: String,
    conclusion: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PromotionState {
    schema_version: u32,
    latest_promotion: LatestPromotion,
    pending: PendingPromotion,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LatestPromotion {
    status: String,
    disposition: String,
    source_promotion_pr: String,
    source_merge_sha: String,
    promoted_swarm_head: String,
    source_governance: Vec<String>,
    source_post_merge_proof: String,
    included_swarm_prs: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PendingPromotion {
    swarm_pr_range: Vec<String>,
    deferred_receipt_carry: Vec<String>,
}

trait PromotePort {
    fn git_output(&self, workspace_root: &Path, args: &[&str]) -> Result<String>;
    fn git_status(&self, workspace_root: &Path, args: &[&str]) -> Result<()>;
    fn gh_output(&self, args: &[&str]) -> Result<Vec<u8>>;
}

struct SystemPort;

/// Machine-readable plan/receipt for the prepared promotion. Emitted for agents
/// and `repo-contract-report`; deterministic for a given repository state.
#[derive(Debug, Serialize)]
struct PromotePlan {
    swarm_head: String,
    source_ref: String,
    source_head: String,
    merge_base: String,
    branch: String,
    required_check: String,
    ci_run_id: u64,
    ci_job: JobReceipt,
    last_promoted_swarm_head: String,
    included_swarm_prs: Vec<String>,
    source_pr: Option<String>,
    dry_run: bool,
    next_actions: Vec<String>,
    planned_mutations: Vec<PlannedMutation>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum PlannedMutation {
    WriteReceipt {
        path: String,
    },
    WritePromotionBody {
        path: String,
    },
    PushBranch {
        remote: String,
        ref_name: String,
        refspec: String,
        current_target: Option<String>,
        disposition: MutationDisposition,
    },
    CreateOrUpdatePullRequest {
        repository: String,
        base: String,
        head: String,
        disposition: MutationDisposition,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
enum MutationDisposition {
    Required,
    AlreadyCurrent,
    Deferred,
}

pub fn run(inputs: PromoteInputs) -> Result<()> {
    run_with_port(&SystemPort, inputs)
}

fn run_with_port(port: &impl PromotePort, inputs: PromoteInputs) -> Result<()> {
    let stdout = io::stdout();
    run_with_port_to(port, inputs, &mut stdout.lock())
}

fn run_with_port_to(
    port: &impl PromotePort,
    inputs: PromoteInputs,
    output: &mut dyn Write,
) -> Result<()> {
    let state = load_promotion_state(&inputs.workspace_root)?;
    let swarm_sha = port
        .git_output(
            &inputs.workspace_root,
            &["rev-parse", &format!("{}^{{commit}}", inputs.swarm_sha)],
        )
        .context("promote: resolve --swarm-sha")?;
    let swarm_ref_sha = port
        .git_output(
            &inputs.workspace_root,
            &["rev-parse", &format!("{}^{{commit}}", inputs.swarm_ref)],
        )
        .with_context(|| format!("promote: resolve {}", inputs.swarm_ref))?;

    if !inputs.allow_historical && swarm_sha != swarm_ref_sha {
        bail!(
            "promote: requested swarm head {swarm_sha} must equal current {} {swarm_ref_sha}; pass --allow-historical to plan an older reachable head",
            inputs.swarm_ref
        );
    }
    ensure_ancestor_with_port(
        port,
        &inputs.workspace_root,
        &swarm_sha,
        &swarm_ref_sha,
        "the requested swarm head must be reachable from the swarm ref",
    )?;
    let source_head = port.git_output(
        &inputs.workspace_root,
        &["rev-parse", &format!("{}^{{commit}}", inputs.source_ref)],
    )?;
    let governance_commits = approved_governance_commits(port, &state.latest_promotion)?;
    let promotion_merge = find_latest_promotion_merge(
        port,
        &inputs.workspace_root,
        &source_head,
        &state.latest_promotion.promoted_swarm_head,
        &governance_commits,
    )?;
    ensure_ancestor_with_port(
        port,
        &inputs.workspace_root,
        &state.latest_promotion.promoted_swarm_head,
        &swarm_sha,
        "last promoted swarm head must be an ancestor of the requested swarm head",
    )?;
    let (receipt, job) = green_swarm_receipt(port, &swarm_sha)?;

    let branch = inputs
        .branch
        .unwrap_or_else(|| format!("promote/swarm-current-{}", &swarm_sha[..12]));
    let existing = port.git_output(
        &inputs.workspace_root,
        &[
            "ls-remote",
            &inputs.source_remote,
            &format!("refs/heads/{branch}"),
        ],
    )?;
    let existing_sha = existing.split_whitespace().next().unwrap_or_default();
    if !existing_sha.is_empty() {
        ensure_ancestor_with_port(
            port,
            &inputs.workspace_root,
            existing_sha,
            &swarm_sha,
            "existing promotion branch is not fast-forwardable to the requested swarm head",
        )?;
    }

    let merge_base = port
        .git_output(
            &inputs.workspace_root,
            &["merge-base", &promotion_merge, &swarm_sha],
        )
        .with_context(|| {
            format!(
                "promote: determine merge base between promotion checkpoint {promotion_merge} and swarm head {swarm_sha}"
            )
        })?;
    if merge_base.is_empty() {
        bail!("promote: merge-base returned no commit for the promotion plan");
    }
    let included_swarm_prs = included_swarm_prs(
        port,
        &inputs.workspace_root,
        &state.latest_promotion.promoted_swarm_head,
        &swarm_sha,
    )?;

    let next_actions = vec![
        format!(
            "Push {swarm_sha}:refs/heads/{branch} to {}.",
            inputs.source_remote
        ),
        "Open a regular-merge source promotion PR from the branch; do not squash.".to_string(),
        "After merge, run `cargo xtask repo-contract-report`.".to_string(),
    ];
    let receipt_path = receipt_path_for_output(&inputs.output);
    let planned_mutations = vec![
        PlannedMutation::WriteReceipt {
            path: portable_display(&inputs.workspace_root, &receipt_path),
        },
        PlannedMutation::WritePromotionBody {
            path: portable_display(&inputs.workspace_root, &inputs.output),
        },
        PlannedMutation::PushBranch {
            remote: inputs.source_remote.clone(),
            ref_name: format!("refs/heads/{branch}"),
            refspec: format!("{swarm_sha}:refs/heads/{branch}"),
            current_target: (!existing_sha.is_empty()).then(|| existing_sha.to_string()),
            disposition: if existing_sha == swarm_sha {
                MutationDisposition::AlreadyCurrent
            } else {
                MutationDisposition::Required
            },
        },
        PlannedMutation::CreateOrUpdatePullRequest {
            repository: SOURCE_REPO.to_string(),
            base: "main".to_string(),
            head: branch.clone(),
            disposition: MutationDisposition::Deferred,
        },
    ];
    let plan = PromotePlan {
        swarm_head: swarm_sha.clone(),
        source_ref: inputs.source_ref.clone(),
        source_head,
        merge_base,
        branch: branch.clone(),
        required_check: REQUIRED_RESULT.to_string(),
        ci_run_id: receipt.database_id,
        ci_job: job,
        last_promoted_swarm_head: state.latest_promotion.promoted_swarm_head,
        included_swarm_prs: included_swarm_prs.clone(),
        source_pr: None,
        dry_run: inputs.dry_run,
        next_actions,
        planned_mutations,
    };
    if inputs.dry_run {
        let json = serde_json::to_string_pretty(&plan).context("promote: serialize plan")?;
        writeln!(output, "{json}").context("promote: write dry-run plan")?;
        return Ok(());
    }

    writeln!(output, "promote: swarm head {swarm_sha}")?;
    writeln!(
        output,
        "promote: green {REQUIRED_RESULT} run {}",
        receipt.database_id
    )?;
    writeln!(output, "promote: source ref {}", inputs.source_ref)?;
    writeln!(output, "promote: branch {branch}")?;
    writeln!(
        output,
        "promote: included swarm PRs since {}: {}",
        plan.last_promoted_swarm_head,
        if included_swarm_prs.is_empty() {
            "(none)".to_string()
        } else {
            included_swarm_prs.join(", ")
        }
    )?;

    let receipt_path = write_plan_receipt(&inputs.workspace_root, &inputs.output, &plan)?;
    writeln!(
        output,
        "promote: wrote plan receipt {}",
        display_path(&inputs.workspace_root, &receipt_path)
    )?;

    if existing_sha != swarm_sha {
        port.git_status(
            &inputs.workspace_root,
            &[
                "push",
                &inputs.source_remote,
                &format!("{swarm_sha}:refs/heads/{branch}"),
            ],
        )
        .with_context(|| format!("promote: push {branch}"))?;
    } else {
        writeln!(output, "promote: branch already points at requested head")?;
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

    writeln!(output, "promote: open a regular merge PR; do not squash")?;
    writeln!(
        output,
        "promote: after merge run cargo xtask repo-contract-report"
    )?;
    Ok(())
}

/// Enumerate the swarm PRs squash-merged between the last promoted source head
/// and the requested swarm head, inferred from `source_ref..swarm_sha`.
fn included_swarm_prs(
    port: &impl PromotePort,
    workspace_root: &Path,
    last_promoted_swarm_head: &str,
    swarm_sha: &str,
) -> Result<Vec<String>> {
    let log = port
        .git_output(
            workspace_root,
            &[
                "log",
                "--no-merges",
                "--reverse",
                "--format=%s",
                &format!("{last_promoted_swarm_head}..{swarm_sha}"),
            ],
        )
        .with_context(|| {
            format!("promote: enumerate swarm PRs {last_promoted_swarm_head}..{swarm_sha}")
        })?;
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
    let subject = subject.trim_end();
    let start = subject.rfind("(#")?;
    let number = subject[start + 2..].strip_suffix(')')?;
    number.parse().ok()
}

/// Write the machine-readable promotion plan next to the generated body and
/// return its path. `output` is the generated-body file path (the same value
/// `promotion_body` writes to); the receipt is placed in that file's parent
/// directory. Dry-run reports this exact target but returns before calling this
/// writer.
fn write_plan_receipt(workspace_root: &Path, output: &Path, plan: &PromotePlan) -> Result<PathBuf> {
    let receipt_path = receipt_path_for_output(output);
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

fn receipt_path_for_output(output: &Path) -> PathBuf {
    output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .join("promote-receipt.json")
}

fn load_promotion_state(workspace_root: &Path) -> Result<PromotionState> {
    let path = workspace_root.join("plans/shiplog-swarm/promotion-state.toml");
    let text =
        fs::read_to_string(&path).with_context(|| format!("promote: read {}", path.display()))?;
    let state: PromotionState =
        toml::from_str(&text).with_context(|| format!("promote: parse {}", path.display()))?;
    if state.schema_version != 1
        || state.latest_promotion.status != "completed"
        || state.latest_promotion.promoted_swarm_head.len() != 40
    {
        bail!("promote: promotion state does not describe a completed promotion");
    }
    if state.latest_promotion.source_promotion_pr.is_empty()
        || state.latest_promotion.disposition.is_empty()
    {
        bail!("promote: completed promotion state is missing source identity");
    }
    let _recorded_receipts = (
        &state.latest_promotion.source_merge_sha,
        &state.latest_promotion.source_post_merge_proof,
        &state.latest_promotion.included_swarm_prs,
        &state.pending.swarm_pr_range,
        &state.pending.deferred_receipt_carry,
    );
    Ok(state)
}

#[derive(Debug, Deserialize)]
struct PullRequestReceipt {
    state: String,
    #[serde(rename = "mergeCommit")]
    merge_commit: Option<CommitOid>,
}

#[derive(Debug, Deserialize)]
struct CommitOid {
    oid: String,
}

fn approved_governance_commits(
    port: &impl PromotePort,
    promotion: &LatestPromotion,
) -> Result<BTreeSet<String>> {
    let mut commits = BTreeSet::new();
    for receipt in &promotion.source_governance {
        let (repo, number) = receipt
            .rsplit_once('#')
            .with_context(|| format!("promote: malformed source governance receipt {receipt:?}"))?;
        if repo != "EffortlessMetrics/shiplog" || number.parse::<u64>().is_err() {
            bail!("promote: malformed source governance receipt {receipt:?}");
        }
        let output = port.gh_output(&[
            "pr",
            "view",
            number,
            "--repo",
            repo,
            "--json",
            "state,mergeCommit",
        ])?;
        let pr: PullRequestReceipt = serde_json::from_slice(&output)
            .with_context(|| format!("promote: parse source governance PR {receipt}"))?;
        if pr.state != "MERGED" {
            bail!("promote: source governance PR {receipt} is not merged");
        }
        let commit = pr.merge_commit.with_context(|| {
            format!("promote: source governance PR {receipt} has no merge commit")
        })?;
        commits.insert(commit.oid);
    }
    Ok(commits)
}

fn find_latest_promotion_merge(
    port: &impl PromotePort,
    workspace_root: &Path,
    source_head: &str,
    promoted_swarm_head: &str,
    governance_commits: &BTreeSet<String>,
) -> Result<String> {
    let mut cursor = source_head.to_string();
    loop {
        let parents = port.git_output(workspace_root, &["show", "-s", "--format=%P", &cursor])?;
        let parents: Vec<_> = parents.split_whitespace().collect();
        if governance_commits.contains(&cursor) {
            let first = parents.first().with_context(|| {
                format!("promote: approved source governance commit {cursor} has no parent")
            })?;
            cursor = (*first).to_string();
            continue;
        }
        match parents.as_slice() {
            [_first, second] if *second == promoted_swarm_head => return Ok(cursor),
            [_first, _second] => bail!(
                "promote: source commit {cursor} is an unexpected merge, not the recorded regular promotion checkpoint"
            ),
            [..] => bail!(
                "promote: unapproved source divergence at {cursor}; only recorded source governance may follow the latest promotion merge"
            ),
        }
    }
}

fn green_swarm_receipt(
    port: &impl PromotePort,
    swarm_sha: &str,
) -> Result<(RunReceipt, JobReceipt)> {
    let output = port.gh_output(&[
        "run",
        "list",
        "--repo",
        SWARM_REPO,
        "--workflow",
        ROUTED_WORKFLOW,
        "--commit",
        swarm_sha,
        "--json",
        "databaseId,headSha,status,conclusion",
    ])?;
    let runs: Vec<RunReceipt> =
        serde_json::from_slice(&output).context("promote: parse exact-head swarm workflow JSON")?;
    let run = runs
        .into_iter()
        .find(|run| {
            run.head_sha == swarm_sha
                && run.status == "completed"
                && run.conclusion.as_deref() == Some("success")
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "promote: no completed successful {REQUIRED_RESULT} run for {swarm_sha}"
            )
        })?;
    let run_id = run.database_id.to_string();
    let output = port.gh_output(&[
        "run", "view", &run_id, "--repo", SWARM_REPO, "--json", "jobs",
    ])?;
    let jobs: RunJobs =
        serde_json::from_slice(&output).context("promote: parse terminal aggregate job JSON")?;
    let job = jobs
        .jobs
        .into_iter()
        .find(|job| job.name == REQUIRED_RESULT)
        .with_context(|| {
            format!(
                "promote: workflow run {} lacks {REQUIRED_RESULT}",
                run.database_id
            )
        })?;
    if job.status != "completed" || job.conclusion.as_deref() != Some("success") {
        bail!(
            "promote: terminal {REQUIRED_RESULT} job in run {} is not successful",
            run.database_id
        );
    }
    Ok((run, job))
}

fn ensure_ancestor_with_port(
    port: &impl PromotePort,
    workspace_root: &Path,
    older: &str,
    newer: &str,
    message: &str,
) -> Result<()> {
    if port
        .git_status(
            workspace_root,
            &["merge-base", "--is-ancestor", older, newer],
        )
        .is_err()
    {
        bail!("promote: {message}: {older} is not an ancestor of {newer}");
    }
    Ok(())
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

impl PromotePort for SystemPort {
    fn git_output(&self, workspace_root: &Path, args: &[&str]) -> Result<String> {
        git_output(workspace_root, args)
    }

    fn git_status(&self, workspace_root: &Path, args: &[&str]) -> Result<()> {
        git_status(workspace_root, args)
    }

    fn gh_output(&self, args: &[&str]) -> Result<Vec<u8>> {
        let output = Command::new("gh")
            .args(args)
            .output()
            .with_context(|| format!("promote: run gh {}", args.join(" ")))?;
        if !output.status.success() {
            bail!(
                "gh {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Ok(output.stdout)
    }
}

fn display_path(workspace_root: &Path, path: &Path) -> String {
    path.strip_prefix(workspace_root)
        .map(|relative| relative.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

fn portable_display(workspace_root: &Path, path: &Path) -> String {
    display_path(workspace_root, path).replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::ensure;
    use std::cell::RefCell;
    use std::collections::VecDeque;

    struct StubPort {
        gh: RefCell<VecDeque<std::result::Result<Vec<u8>, String>>>,
        gh_calls: RefCell<Vec<Vec<String>>>,
        remote_target: Option<String>,
        fail_merge_base: bool,
    }

    impl PromotePort for StubPort {
        fn git_output(&self, workspace_root: &Path, args: &[&str]) -> Result<String> {
            if args.first() == Some(&"ls-remote") {
                return Ok(self
                    .remote_target
                    .as_ref()
                    .map(|target| format!("{target}\t{}", args.last().unwrap_or(&"")))
                    .unwrap_or_default());
            }
            if self.fail_merge_base && args.first() == Some(&"merge-base") {
                bail!("stub merge-base failure");
            }
            SystemPort.git_output(workspace_root, args)
        }

        fn git_status(&self, workspace_root: &Path, args: &[&str]) -> Result<()> {
            SystemPort.git_status(workspace_root, args)
        }

        fn gh_output(&self, args: &[&str]) -> Result<Vec<u8>> {
            self.gh_calls
                .borrow_mut()
                .push(args.iter().map(|arg| (*arg).to_string()).collect());
            match self.gh.borrow_mut().pop_front() {
                Some(Ok(output)) => Ok(output),
                Some(Err(message)) => bail!("stub gh: {message}"),
                None => bail!("stub gh response queue exhausted"),
            }
        }
    }

    struct GitFixture {
        dir: tempfile::TempDir,
        promoted: String,
        current: String,
        governance: String,
    }

    fn fixture_git() -> Result<GitFixture> {
        fixture_git_with_governance_shape(false)
    }

    fn fixture_git_with_merge_governance() -> Result<GitFixture> {
        fixture_git_with_governance_shape(true)
    }

    fn fixture_git_with_governance_shape(merge_governance: bool) -> Result<GitFixture> {
        let dir = tempfile::tempdir()?;
        git_fixture(dir.path(), &["init", "--initial-branch=base"])?;
        git_fixture(dir.path(), &["config", "user.email", "test@example.com"])?;
        git_fixture(dir.path(), &["config", "user.name", "Promotion Test"])?;
        fs::write(dir.path().join("base.txt"), "base\n")?;
        git_fixture(dir.path(), &["add", "base.txt"])?;
        git_fixture(dir.path(), &["commit", "-m", "base"])?;
        git_fixture(dir.path(), &["switch", "-c", "promoted"])?;
        fs::write(dir.path().join("promoted.txt"), "promoted\n")?;
        git_fixture(dir.path(), &["add", "promoted.txt"])?;
        git_fixture(dir.path(), &["commit", "-m", "feat: promoted (#238)"])?;
        let promoted = git_fixture(dir.path(), &["rev-parse", "HEAD"])?;
        git_fixture(dir.path(), &["switch", "-c", "swarm"])?;
        fs::write(dir.path().join("current.txt"), "current\n")?;
        git_fixture(dir.path(), &["add", "current.txt"])?;
        git_fixture(dir.path(), &["commit", "-m", "feat: current (#255)"])?;
        let current = git_fixture(dir.path(), &["rev-parse", "HEAD"])?;
        git_fixture(dir.path(), &["switch", "base"])?;
        git_fixture(dir.path(), &["switch", "-c", "source"])?;
        git_fixture(
            dir.path(),
            &["merge", "--no-ff", "promoted", "-m", "Merge promotion #655"],
        )?;
        if merge_governance {
            git_fixture(dir.path(), &["switch", "-c", "governance-side"])?;
            fs::write(dir.path().join("governance.txt"), "approved\n")?;
            git_fixture(dir.path(), &["add", "governance.txt"])?;
            git_fixture(dir.path(), &["commit", "-m", "chore: governance payload"])?;
            git_fixture(dir.path(), &["switch", "source"])?;
            git_fixture(
                dir.path(),
                &[
                    "merge",
                    "--no-ff",
                    "governance-side",
                    "-m",
                    "chore: governance (#656)",
                ],
            )?;
        } else {
            fs::write(dir.path().join("governance.txt"), "approved\n")?;
            git_fixture(dir.path(), &["add", "governance.txt"])?;
            git_fixture(dir.path(), &["commit", "-m", "chore: governance (#656)"])?;
        }
        let governance = git_fixture(dir.path(), &["rev-parse", "HEAD"])?;
        fs::create_dir_all(dir.path().join("plans/shiplog-swarm"))?;
        fs::write(
            dir.path().join("plans/shiplog-swarm/promotion-state.toml"),
            format!(
                "schema_version = 1\n[latest_promotion]\nstatus = \"completed\"\ndisposition = \"completed-with-governance\"\nsource_promotion_pr = \"EffortlessMetrics/shiplog#655\"\nsource_merge_sha = \"\"\npromoted_swarm_head = \"{promoted}\"\nsource_governance = [\"EffortlessMetrics/shiplog#656\"]\nsource_post_merge_proof = \"\"\nincluded_swarm_prs = [\"EffortlessMetrics/shiplog-swarm#238\"]\n[pending]\nswarm_pr_range = []\ndeferred_receipt_carry = []\n"
            ),
        )?;
        Ok(GitFixture {
            dir,
            promoted,
            current,
            governance,
        })
    }

    fn git_fixture(workspace_root: &Path, args: &[&str]) -> Result<String> {
        git_output(workspace_root, args)
    }

    fn stub_port(fixture: &GitFixture, run_success: bool, job_success: bool) -> StubPort {
        stub_port_for_head(fixture, &fixture.current, run_success, job_success)
    }

    fn stub_port_for_head(
        fixture: &GitFixture,
        head: &str,
        run_success: bool,
        job_success: bool,
    ) -> StubPort {
        let run_conclusion = if run_success { "success" } else { "failure" };
        let job_conclusion = if job_success { "success" } else { "failure" };
        StubPort {
            gh: RefCell::new(VecDeque::from([
                Ok(format!(
                    "{{\"state\":\"MERGED\",\"mergeCommit\":{{\"oid\":\"{}\"}}}}",
                    fixture.governance
                )
                .into_bytes()),
                Ok(format!(
                    "[{{\"databaseId\":42,\"headSha\":\"{}\",\"status\":\"completed\",\"conclusion\":\"{run_conclusion}\"}}]",
                    head
                )
                .into_bytes()),
                Ok(format!(
                    "{{\"jobs\":[{{\"databaseId\":84,\"name\":\"{REQUIRED_RESULT}\",\"status\":\"completed\",\"conclusion\":\"{job_conclusion}\"}}]}}"
                )
                .into_bytes()),
            ])),
            gh_calls: RefCell::new(Vec::new()),
            remote_target: None,
            fail_merge_base: false,
        }
    }

    fn fixture_inputs(fixture: &GitFixture) -> PromoteInputs {
        PromoteInputs {
            workspace_root: fixture.dir.path().to_path_buf(),
            swarm_sha: fixture.current.clone(),
            dry_run: true,
            source_ref: "source".to_string(),
            swarm_ref: "swarm".to_string(),
            source_remote: "origin".to_string(),
            branch: None,
            output: PathBuf::from("target/source-of-truth/promotion-body.md"),
            allow_historical: false,
        }
    }

    #[test]
    fn planner_accepts_current_source_governance_and_stays_read_only() -> Result<()> {
        let fixture = fixture_git()?;
        let port = stub_port(&fixture, true, true);
        let inputs = fixture_inputs(&fixture);
        let target = fixture.dir.path().join("target");
        let mut output = Vec::new();
        run_with_port_to(&port, inputs, &mut output)?;
        let plan: serde_json::Value = serde_json::from_slice(&output)?;
        let branch = format!("promote/swarm-current-{}", &fixture.current[..12]);
        ensure!(
            plan["planned_mutations"]
                == serde_json::json!([
                    {
                        "kind": "write-receipt",
                        "path": "target/source-of-truth/promote-receipt.json"
                    },
                    {
                        "kind": "write-promotion-body",
                        "path": "target/source-of-truth/promotion-body.md"
                    },
                    {
                        "kind": "push-branch",
                        "remote": "origin",
                        "ref_name": format!("refs/heads/{branch}"),
                        "refspec": format!("{}:refs/heads/{branch}", fixture.current),
                        "current_target": null,
                        "disposition": "required"
                    },
                    {
                        "kind": "create-or-update-pull-request",
                        "repository": "EffortlessMetrics/shiplog",
                        "base": "main",
                        "head": branch,
                        "disposition": "deferred"
                    }
                ])
        );
        ensure!(!target.exists());
        Ok(())
    }

    #[test]
    fn planner_follows_first_parent_of_approved_merge_governance() -> Result<()> {
        let fixture = fixture_git_with_merge_governance()?;
        let port = stub_port(&fixture, true, true);
        let mut output = Vec::new();
        run_with_port_to(&port, fixture_inputs(&fixture), &mut output)?;
        let plan: serde_json::Value = serde_json::from_slice(&output)?;
        ensure!(plan["last_promoted_swarm_head"] == fixture.promoted);
        Ok(())
    }

    #[test]
    fn planner_records_already_current_branch_target() -> Result<()> {
        let fixture = fixture_git()?;
        let mut port = stub_port(&fixture, true, true);
        port.remote_target = Some(fixture.current.clone());
        let mut output = Vec::new();
        run_with_port_to(&port, fixture_inputs(&fixture), &mut output)?;
        let plan: serde_json::Value = serde_json::from_slice(&output)?;
        let push = &plan["planned_mutations"][2];
        ensure!(push["current_target"] == fixture.current);
        ensure!(push["disposition"] == "already-current");
        ensure!(!fixture.dir.path().join("target").exists());
        Ok(())
    }

    #[test]
    fn planner_rejects_stale_head_without_historical_opt_in() -> Result<()> {
        let fixture = fixture_git()?;
        let port = stub_port(&fixture, true, true);
        let mut inputs = fixture_inputs(&fixture);
        inputs.swarm_sha = fixture.promoted.clone();
        let error = run_with_port(&port, inputs)
            .err()
            .context("expected stale rejection")?;
        ensure!(error.to_string().contains("--allow-historical"));
        Ok(())
    }

    #[test]
    fn planner_allows_reachable_historical_head_with_explicit_opt_in() -> Result<()> {
        let fixture = fixture_git()?;
        let port = stub_port_for_head(&fixture, &fixture.promoted, true, true);
        let mut inputs = fixture_inputs(&fixture);
        inputs.swarm_sha = fixture.promoted.clone();
        inputs.allow_historical = true;
        run_with_port(&port, inputs)?;
        let calls = port.gh_calls.borrow();
        let run_list = calls
            .iter()
            .find(|args| args.starts_with(&["run".to_string(), "list".to_string()]))
            .context("expected exact-head run-list call")?;
        ensure!(
            run_list
                .windows(2)
                .any(|pair| pair == ["--commit", fixture.promoted.as_str()])
        );
        ensure!(!run_list.iter().any(|arg| arg == "--branch"));
        Ok(())
    }

    #[test]
    fn planner_rejects_ungreen_workflow_and_failed_terminal_job() -> Result<()> {
        let fixture = fixture_git()?;
        let error = run_with_port(&stub_port(&fixture, false, true), fixture_inputs(&fixture))
            .err()
            .context("expected workflow rejection")?;
        ensure!(error.to_string().contains("no completed successful"));

        let fixture = fixture_git()?;
        let error = run_with_port(&stub_port(&fixture, true, false), fixture_inputs(&fixture))
            .err()
            .context("expected aggregate rejection")?;
        ensure!(error.to_string().contains("terminal"));
        Ok(())
    }

    #[test]
    fn planner_rejects_malformed_github_json() -> Result<()> {
        let fixture = fixture_git()?;
        let port = StubPort {
            gh: RefCell::new(VecDeque::from([Ok(b"not-json".to_vec())])),
            gh_calls: RefCell::new(Vec::new()),
            remote_target: None,
            fail_merge_base: false,
        };
        let error = run_with_port(&port, fixture_inputs(&fixture))
            .err()
            .context("expected malformed JSON rejection")?;
        ensure!(error.to_string().contains("parse source governance PR"));
        Ok(())
    }

    #[test]
    fn planner_rejects_unapproved_source_divergence() -> Result<()> {
        let fixture = fixture_git()?;
        fs::write(fixture.dir.path().join("divergence.txt"), "unapproved\n")?;
        git_fixture(fixture.dir.path(), &["add", "divergence.txt"])?;
        git_fixture(
            fixture.dir.path(),
            &["commit", "-m", "fix: source product drift"],
        )?;
        let error = run_with_port(&stub_port(&fixture, true, true), fixture_inputs(&fixture))
            .err()
            .context("expected divergence rejection")?;
        ensure!(error.to_string().contains("unapproved source divergence"));
        Ok(())
    }

    #[test]
    fn planner_propagates_merge_base_failure_before_output_or_mutation() -> Result<()> {
        let fixture = fixture_git()?;
        let mut port = stub_port(&fixture, true, true);
        port.fail_merge_base = true;
        let target = fixture.dir.path().join("target");
        let mut output = Vec::new();
        let error = run_with_port_to(&port, fixture_inputs(&fixture), &mut output)
            .err()
            .context("expected merge-base rejection")?;
        ensure!(error.to_string().contains("determine merge base"));
        ensure!(output.is_empty());
        ensure!(!target.exists());
        Ok(())
    }

    #[test]
    fn planner_ignores_non_terminal_squash_markers() -> Result<()> {
        let fixture = fixture_git()?;
        git_fixture(fixture.dir.path(), &["switch", "swarm"])?;
        fs::write(fixture.dir.path().join("inline.txt"), "inline\n")?;
        git_fixture(fixture.dir.path(), &["add", "inline.txt"])?;
        git_fixture(
            fixture.dir.path(),
            &["commit", "-m", "fix: mention (#777) inline text"],
        )?;
        fs::write(fixture.dir.path().join("garbage.txt"), "garbage\n")?;
        git_fixture(fixture.dir.path(), &["add", "garbage.txt"])?;
        git_fixture(
            fixture.dir.path(),
            &["commit", "-m", "fix: almost terminal (#778) garbage"],
        )?;
        let head = git_fixture(fixture.dir.path(), &["rev-parse", "HEAD"])?;
        let receipts =
            included_swarm_prs(&SystemPort, fixture.dir.path(), &fixture.promoted, &head)?;
        ensure!(receipts == ["EffortlessMetrics/shiplog-swarm#255"]);
        Ok(())
    }

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
        assert_eq!(
            extract_trailing_pr_number("fix: valid (#346)   "),
            Some(346)
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
        assert_eq!(
            extract_trailing_pr_number("fix: inline (#5) text continues"),
            None
        );
        assert_eq!(extract_trailing_pr_number("fix: trailing (#5)."), None);
        assert_eq!(extract_trailing_pr_number("fix: marker (#5) garbage"), None);
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
            merge_base: "df611d5".to_string(),
            branch: "promote/swarm-current-c4fdba223d1c".to_string(),
            required_check: REQUIRED_RESULT.to_string(),
            ci_run_id: 1234,
            ci_job: JobReceipt {
                database_id: 5678,
                name: REQUIRED_RESULT.to_string(),
                status: "completed".to_string(),
                conclusion: Some("success".to_string()),
            },
            last_promoted_swarm_head: "141b118da0890e9984ec0c5f0b7f9e3e1c07b3ea".to_string(),
            included_swarm_prs: vec!["EffortlessMetrics/shiplog-swarm#238".to_string()],
            source_pr: None,
            dry_run: true,
            next_actions: vec!["Open a regular-merge source promotion PR.".to_string()],
            planned_mutations: vec![PlannedMutation::WriteReceipt {
                path: "target/source-of-truth/promote-receipt.json".to_string(),
            }],
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
