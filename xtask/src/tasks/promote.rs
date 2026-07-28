//! The promote command verifies an exact swarm head and prepares the source
//! promotion branch. It deliberately stops before creating or merging a PR.

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::id as process_id;
use std::time::{SystemTime, UNIX_EPOCH};

use super::promotion_body;

const SWARM_REPO: &str = "EffortlessMetrics/shiplog-swarm";
const SOURCE_REPO: &str = "EffortlessMetrics/shiplog";
const ROUTED_WORKFLOW: &str = "EM CI Routed Shiplog Rust";
const REQUIRED_RESULT: &str = "Shiplog Rust Small Result";
const SOURCE_ONLY_PATH_POLICY: &str = "policy/source-only-paths.toml";
/// Trailers giving the overlay commit machine-readable identity. The promotion
/// checkpoint's second parent is the overlay, not the swarm head, so closeout
/// needs the overlay to state which swarm head it carries.
const OVERLAY_SOURCE_TRAILER: &str = "Shiplog-Source-Head:";
const OVERLAY_SWARM_TRAILER: &str = "Shiplog-Swarm-Head:";
/// Content hash of the per-path resolution plan the overlay was built from.
/// Verify-only recomputes this id from the exact source parent, swarm head,
/// policy, and active evidence before accepting a modern overlay checkpoint.
const OVERLAY_PLAN_TRAILER: &str = "Shiplog-Resolution-Plan:";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceOnlyPathsPolicy {
    schema_version: u32,
    policy: String,
    #[serde(default)]
    owner: String,
    status: String,
    #[serde(default)]
    allow: Vec<SourceOnlyPathEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceOnlyPathEntry {
    path: String,
    #[serde(default)]
    owner: String,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    classification: String,
    #[serde(default)]
    created: String,
    #[serde(default)]
    review_after: String,
}

pub struct PromoteInputs {
    pub workspace_root: PathBuf,
    pub swarm_sha: String,
    pub dry_run: bool,
    pub source_ref: String,
    pub swarm_ref: String,
    pub source_remote: String,
    pub output: PathBuf,
    pub allow_historical: bool,
    pub verify_only: bool,
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
    /// Reuses the canonical type rather than restating it: this struct is
    /// `deny_unknown_fields`, so a field added to the manifest and not mirrored
    /// here makes `promote` reject the manifest outright.
    #[serde(default)]
    transition: Vec<super::promotion_state::Transition>,
    #[serde(default)]
    source_authority: Vec<super::promotion_state::SourceAuthorityDecision>,
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
    /// `git patch-id --stable` over a patch supplied on stdin.
    fn git_patch_id(&self, patch: &str) -> Result<String> {
        super::transition::system_patch_id(patch)
    }
    fn git_output_with_env(
        &self,
        workspace_root: &Path,
        args: &[&str],
        _env: &[(&str, &str)],
    ) -> Result<String> {
        self.git_output(workspace_root, args)
    }
}

struct SystemPort;

/// Result of building the source overlay commit.
#[derive(Debug)]
struct PreparedOverlay {
    sha: String,
    /// Overlay residue that cleanup could not remove. Recorded in the receipt so
    /// a partially cleaned workspace is visible rather than silently inherited
    /// by the next promotion.
    cleanup_warnings: Vec<String>,
}

/// Machine-readable plan/receipt for the prepared promotion. Emitted for agents
/// and `repo-contract-report`; deterministic for a given repository state.
#[derive(Debug, Serialize)]
struct PromotePlan {
    swarm_head: String,
    prepared_overlay_sha: String,
    /// Every differing path with the exact blobs it was resolved against, the
    /// effect the overlay applied, and the basis that entitled it.
    path_decisions: Vec<PathDecision>,
    /// Paths the overlay resolved in source's favour, for quick reading.
    overlay_source_paths: Vec<String>,
    /// Differing paths that kept swarm content.
    overlay_swarm_path_count: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    overlay_cleanup_warnings: Vec<String>,
    source_ref: String,
    source_head: String,
    merge_base: String,
    branch: String,
    required_check: String,
    ci_run_id: u64,
    ci_job: JobReceipt,
    last_promoted_swarm_head: String,
    included_swarm_prs: Vec<String>,
    source_pr: Option<SourcePullRequest>,
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
        action: PullRequestAction,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
enum MutationDisposition {
    Required,
    AlreadyCurrent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum PullRequestAction {
    Create,
    Update,
    AlreadyCurrent,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct SourcePullRequest {
    number: u64,
    url: String,
    #[serde(rename = "headRefName")]
    head_ref_name: String,
    #[serde(rename = "headRefOid")]
    head_ref_oid: String,
    #[serde(rename = "baseRefName")]
    base_ref_name: String,
    #[serde(rename = "headRepository")]
    head_repository: RepositoryIdentity,
    #[serde(rename = "headRepositoryOwner")]
    head_repository_owner: RepositoryOwner,
    title: String,
    body: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RepositoryIdentity {
    #[serde(rename = "nameWithOwner")]
    #[serde(default)]
    name_with_owner: String,
    #[serde(default)]
    name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RepositoryOwner {
    login: String,
}

#[derive(Debug, Deserialize)]
struct CompareReceipt {
    status: String,
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

    if inputs.verify_only {
        return run_verify_only(port, &inputs, &state, &swarm_sha, &swarm_ref_sha, output);
    }

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
    let source_only_paths = load_source_only_paths(&inputs.workspace_root)?;
    let transition_authority = derive_promotion_transition_authority(
        port,
        &inputs.workspace_root,
        &source_head,
        &swarm_sha,
        &source_only_paths,
        &state,
    )?;
    // Commits the ancestry walk may step over: approved governance, plus source
    // merges an active transition receipt accounts for. Both are recorded
    // evidence; anything else following the promotion merge is unapproved.
    let mut recorded_commits = approved_governance_commits(port, &state.latest_promotion)?;
    recorded_commits.extend(transition_authority.source_commits.iter().cloned());
    find_latest_promotion_merge(
        port,
        &inputs.workspace_root,
        &source_head,
        &state.latest_promotion.promoted_swarm_head,
        &recorded_commits,
    )?;
    ensure_ancestor_with_port(
        port,
        &inputs.workspace_root,
        &state.latest_promotion.promoted_swarm_head,
        &swarm_sha,
        "last promoted swarm head must be an ancestor of the requested swarm head",
    )?;
    let (receipt, job) = green_swarm_receipt(port, &swarm_sha)?;

    let branch = format!("promote/swarm-current-{}", &swarm_sha[..12]);
    // Resolved before the overlay is built, so a promotion that cannot be
    // resolved never materializes a commit, and so overlay construction and
    // acceptance are driven by the same decision.
    let (overlay_plan, merge_base) = compute_resolution_plan(
        port,
        &inputs.workspace_root,
        &source_head,
        &swarm_sha,
        &source_only_paths,
        &transition_authority,
        output,
    )?;
    let take_source = overlay_plan.take_source();
    let plan_id = resolution_plan_id(&overlay_plan)?;
    let PreparedOverlay {
        sha: prepared_overlay_sha,
        cleanup_warnings: overlay_cleanup_warnings,
    } = prepare_source_overlay(
        port,
        &inputs.workspace_root,
        &source_head,
        &swarm_sha,
        &take_source,
        &plan_id,
        // A planning-only run must not leave the overlay commit in the object
        // database; an executing run has to keep it so the push has something to
        // send.
        inputs.dry_run,
    )?;
    let existing = port.git_output(
        &inputs.workspace_root,
        &[
            "ls-remote",
            &inputs.source_remote,
            &format!("refs/heads/{branch}"),
        ],
    )?;
    let existing_sha = existing.split_whitespace().next().unwrap_or_default();
    if !existing_sha.is_empty() && existing_sha != prepared_overlay_sha {
        ensure_remote_fast_forward(port, existing_sha, &source_head, &prepared_overlay_sha)?;
    }

    let included_swarm_prs = included_swarm_prs(
        port,
        &inputs.workspace_root,
        &state.latest_promotion.promoted_swarm_head,
        &swarm_sha,
    )?;
    let body_inputs = promotion_body::PromotionBodyInputs {
        workspace_root: inputs.workspace_root.clone(),
        source_ref: inputs.source_ref.clone(),
        swarm_ref: inputs.swarm_ref.clone(),
        swarm_head: Some(swarm_sha.clone()),
        included_swarm_prs: included_swarm_prs.clone(),
        swarm_pr_run: None,
        swarm_main_run: Some(receipt.database_id.to_string()),
        source_pr_run: None,
        source_post_merge_run: None,
        output: inputs.output.clone(),
    };
    let promotion_body = promotion_body::render(&body_inputs)?;
    let title = format!(
        "merge(swarm): promote shiplog-swarm through {}",
        &swarm_sha[..12]
    );
    let source_pr = discover_source_pr(port, &branch)?;
    if let Some(pr) = source_pr.as_ref()
        && (existing_sha.is_empty() || pr.head_ref_oid != existing_sha)
    {
        bail!(
            "promote: open source PR #{} head {} does not match remote branch target {:?}",
            pr.number,
            pr.head_ref_oid,
            existing_sha
        );
    }
    let pr_action = match source_pr.as_ref() {
        None => PullRequestAction::Create,
        Some(pr) if pr.title == title && pr.body == promotion_body => {
            PullRequestAction::AlreadyCurrent
        }
        Some(_) => PullRequestAction::Update,
    };

    let next_actions = vec![
        format!(
            "Push {prepared_overlay_sha}:refs/heads/{branch} to {}.",
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
            refspec: format!("{prepared_overlay_sha}:refs/heads/{branch}"),
            current_target: (!existing_sha.is_empty()).then(|| existing_sha.to_string()),
            disposition: if existing_sha == prepared_overlay_sha {
                MutationDisposition::AlreadyCurrent
            } else {
                MutationDisposition::Required
            },
        },
        PlannedMutation::CreateOrUpdatePullRequest {
            repository: SOURCE_REPO.to_string(),
            base: "main".to_string(),
            head: branch.clone(),
            action: pr_action,
        },
    ];
    let mut plan = PromotePlan {
        swarm_head: swarm_sha.clone(),
        prepared_overlay_sha: prepared_overlay_sha.clone(),
        overlay_source_paths: take_source.clone(),
        overlay_swarm_path_count: overlay_plan.take_swarm_count(),
        path_decisions: overlay_plan.decisions.clone(),
        overlay_cleanup_warnings,
        source_ref: inputs.source_ref.clone(),
        source_head,
        merge_base,
        branch: branch.clone(),
        required_check: REQUIRED_RESULT.to_string(),
        ci_run_id: receipt.database_id,
        ci_job: job,
        last_promoted_swarm_head: state.latest_promotion.promoted_swarm_head,
        included_swarm_prs: included_swarm_prs.clone(),
        source_pr: source_pr.clone(),
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
    writeln!(output, "promote: prepared overlay {prepared_overlay_sha}")?;
    for warning in &plan.overlay_cleanup_warnings {
        writeln!(output, "promote: warning: {warning}")?;
    }
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

    let body_path =
        promotion_body::write_rendered(&inputs.workspace_root, &inputs.output, &promotion_body)?;

    if existing_sha != prepared_overlay_sha {
        // Lease against exactly the target observed while planning, so a branch
        // someone else moved in between is rejected instead of overwritten. An
        // empty expectation asserts the ref does not exist yet.
        let lease = format!("--force-with-lease=refs/heads/{branch}:{existing_sha}");
        port.git_status(
            &inputs.workspace_root,
            &[
                "push",
                &lease,
                &inputs.source_remote,
                &format!("{prepared_overlay_sha}:refs/heads/{branch}"),
            ],
        )
        .with_context(|| format!("promote: push {branch}"))?;
    } else {
        writeln!(output, "promote: branch already points at requested head")?;
    }

    let executed_pr = execute_source_pr(
        port,
        pr_action,
        source_pr.as_ref(),
        &branch,
        &prepared_overlay_sha,
        &title,
        &promotion_body,
        &body_path,
    )?;
    plan.source_pr = Some(executed_pr);
    let receipt_path = write_plan_receipt(&inputs.workspace_root, &inputs.output, &plan)?;
    writeln!(
        output,
        "promote: wrote plan receipt {}",
        display_path(&inputs.workspace_root, &receipt_path)
    )?;

    writeln!(output, "promote: open a regular merge PR; do not squash")?;
    writeln!(
        output,
        "promote: after merge run cargo xtask repo-contract-report"
    )?;
    Ok(())
}

/// Machine-readable receipt for `--verify-only`: confirms that an exact swarm
/// head already landed on the source ref as a regular-merge promotion
/// checkpoint. Emitted only on success (a failed verification bails before this
/// is constructed and exits non-zero). Deterministic for a given repository
/// state; writes nothing.
#[derive(Debug, Serialize)]
struct PromoteVerification {
    mode: &'static str,
    swarm_head: String,
    source_ref: String,
    source_head: String,
    last_promoted_swarm_head: String,
    landed_merge: String,
    /// Which shape carried the swarm head. Overlay checkpoints additionally
    /// proved their parent and recorded source head match the merge.
    checkpoint_shape: CheckpointShape,
    /// Plan id the overlay recorded, when it is an overlay checkpoint. Modern
    /// overlays are checked against the exact plan derived from their source
    /// parent, swarm head, policy, and active evidence.
    #[serde(skip_serializing_if = "Option::is_none")]
    resolution_plan_id: Option<String>,
    included_swarm_prs: Vec<String>,
    checks: Vec<String>,
    next_actions: Vec<String>,
}

/// Read-only post-merge verification. Confirms the requested swarm head is
/// reachable from the swarm ref and has landed on the source ref as a
/// regular-merge (two-parent) checkpoint whose second parent is the exact swarm
/// head. Modern overlay checkpoints also rederive and compare the exact
/// evidence-bearing resolution plan used to build the overlay. Fails closed if
/// the promotion has not landed, was squash-merged, or its active evidence no
/// longer matches. Performs no ref, PR, or file mutation; checking source
/// post-merge CI and whether the source tip carries unapproved divergence
/// remains the job of `repo-contract-report`.
fn run_verify_only(
    port: &impl PromotePort,
    inputs: &PromoteInputs,
    state: &PromotionState,
    swarm_sha: &str,
    swarm_ref_sha: &str,
    output: &mut dyn Write,
) -> Result<()> {
    ensure_ancestor_with_port(
        port,
        &inputs.workspace_root,
        swarm_sha,
        swarm_ref_sha,
        "the verified swarm head must be reachable from the swarm ref",
    )?;
    let source_head = port
        .git_output(
            &inputs.workspace_root,
            &["rev-parse", &format!("{}^{{commit}}", inputs.source_ref)],
        )
        .with_context(|| format!("promote: resolve {}", inputs.source_ref))?;
    let landed_merge = find_regular_merge_landing(
        port,
        &inputs.workspace_root,
        &source_head,
        swarm_sha,
    )?
    .with_context(|| {
        format!(
            "promote: verify-only could not confirm a regular-merge checkpoint landing swarm head {swarm_sha} on {} (the promotion is unlanded or was squash-merged)",
            inputs.source_ref
        )
    })?;
    let resolution_check = match (
        landed_merge.shape,
        landed_merge.resolution_plan_id.as_deref(),
    ) {
        (CheckpointShape::Overlay, Some(recorded_plan_id)) => {
            let source_only_paths = load_source_only_paths(&inputs.workspace_root)?;
            let transition_authority = derive_promotion_transition_authority(
                port,
                &inputs.workspace_root,
                &landed_merge.source_parent,
                swarm_sha,
                &source_only_paths,
                state,
            )?;
            let (plan, _) = compute_resolution_plan(
                port,
                &inputs.workspace_root,
                &landed_merge.source_parent,
                swarm_sha,
                &source_only_paths,
                &transition_authority,
                output,
            )?;
            let derived_plan_id = resolution_plan_id(&plan)?;
            ensure!(
                recorded_plan_id == derived_plan_id,
                "promote: verified overlay {} records resolution plan {recorded_plan_id}, but the current exact evidence derives {derived_plan_id}",
                landed_merge.merge_sha
            );
            format!(
                "overlay resolution plan {recorded_plan_id} matches the exact persisted promotion evidence"
            )
        }
        (CheckpointShape::Overlay, None) => {
            "legacy overlay has no resolution-plan evidence; modern per-path decision proof was not claimed"
                .to_string()
        }
        (CheckpointShape::RawSwarmHead, _) => {
            "raw swarm-head checkpoint predates overlay resolution-plan evidence".to_string()
        }
    };
    let included_swarm_prs = included_swarm_prs(
        port,
        &inputs.workspace_root,
        &state.latest_promotion.promoted_swarm_head,
        swarm_sha,
    )?;
    let shape_check = match landed_merge.shape {
        CheckpointShape::RawSwarmHead => format!(
            "regular-merge checkpoint {} has swarm head {swarm_sha} directly as its second parent",
            landed_merge.merge_sha
        ),
        CheckpointShape::Overlay => format!(
            "regular-merge checkpoint {} merges an overlay parented on {} that records source head {} and swarm head {swarm_sha}",
            landed_merge.merge_sha, landed_merge.source_parent, landed_merge.source_parent
        ),
    };
    let verification = PromoteVerification {
        mode: "verify-only",
        swarm_head: swarm_sha.to_string(),
        source_ref: inputs.source_ref.clone(),
        source_head,
        last_promoted_swarm_head: state.latest_promotion.promoted_swarm_head.clone(),
        landed_merge: landed_merge.merge_sha.clone(),
        checkpoint_shape: landed_merge.shape,
        resolution_plan_id: landed_merge.resolution_plan_id.clone(),
        included_swarm_prs,
        checks: vec![
            format!("swarm head {swarm_sha} is reachable from {}", inputs.swarm_ref),
            shape_check,
            resolution_check,
        ],
        next_actions: vec![
            "Run `cargo xtask repo-contract-report` to check source post-merge CI and topology alignment.".to_string(),
        ],
    };
    let json =
        serde_json::to_string_pretty(&verification).context("promote: serialize verification")?;
    writeln!(output, "{json}").context("promote: write verification receipt")?;
    Ok(())
}

/// Search the source history reachable from `source_head` for a regular
/// two-parent merge whose second parent is exactly `swarm_sha` — the shape a
/// non-squashed promotion checkpoint produces. Returns the newest such merge, or
/// `None` when the head has not landed as a regular merge (unlanded or squashed,
/// where `swarm_sha` is not a merged-in parent anywhere in source history).
/// Later commits on top of the checkpoint do not hide the landing.
/// How a promotion checkpoint carries its swarm head.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum CheckpointShape {
    /// Predates overlays: the swarm head was pushed and merged directly.
    RawSwarmHead,
    /// The second parent is an overlay commit recording its own inputs.
    Overlay,
}

/// A regular-merge promotion checkpoint on the source ref.
///
/// One parser serves both the planner, which walks back to the previous
/// checkpoint, and closeout, which confirms a requested head landed. They
/// previously implemented partial and divergent topology rules, so the first
/// overlay promotion satisfied closeout while the next planning run rejected it.
#[derive(Clone, Debug)]
struct PromotionCheckpoint {
    merge_sha: String,
    source_parent: String,
    /// Swarm head this checkpoint landed, whichever shape carried it.
    swarm_head: String,
    resolution_plan_id: Option<String>,
    shape: CheckpointShape,
}

/// Interpret a two-parent merge as a promotion checkpoint.
///
/// A second parent with no swarm trailer is the legacy shape and is taken at
/// face value. A second parent that claims to be an overlay must prove it: its
/// own parent and its recorded source head must both be the merge's first
/// parent, and it must carry a well-formed resolution plan id. Without the
/// parent proof, any commit carrying a copied trailer would satisfy
/// verification.
fn parse_promotion_checkpoint(
    port: &impl PromotePort,
    workspace_root: &Path,
    merge_sha: &str,
    first_parent: &str,
    second_parent: &str,
) -> Result<PromotionCheckpoint> {
    let message = port
        .git_output(
            workspace_root,
            &["show", "-s", "--format=%B", second_parent],
        )
        .with_context(|| format!("promote: read commit message for {second_parent}"))?;

    let Some(swarm_head) = overlay_trailer(&message, OVERLAY_SWARM_TRAILER) else {
        return Ok(PromotionCheckpoint {
            merge_sha: merge_sha.to_string(),
            source_parent: first_parent.to_string(),
            swarm_head: second_parent.to_string(),
            resolution_plan_id: None,
            shape: CheckpointShape::RawSwarmHead,
        });
    };

    let recorded_source = overlay_trailer(&message, OVERLAY_SOURCE_TRAILER).with_context(|| {
        format!(
            "promote: overlay {second_parent} records a swarm head but no {OVERLAY_SOURCE_TRAILER} trailer"
        )
    })?;
    if recorded_source != first_parent {
        bail!(
            "promote: overlay {second_parent} records source head {recorded_source}, but checkpoint {merge_sha} merges onto {first_parent}"
        );
    }
    let overlay_parents = port
        .git_output(
            workspace_root,
            &["show", "-s", "--format=%P", second_parent],
        )
        .with_context(|| format!("promote: read parents of overlay {second_parent}"))?;
    let overlay_parents = overlay_parents.split_whitespace().collect::<Vec<_>>();
    if overlay_parents.as_slice() != [first_parent] {
        bail!(
            "promote: overlay {second_parent} must have exactly one parent {first_parent}, found {}",
            overlay_parents.join(" ")
        );
    }
    // Overlays produced by #278 predate the resolution-plan trailer. Keep
    // those already-valid checkpoints verifiable after #279 lands, while
    // rejecting a malformed trailer whenever a newer overlay supplies one.
    let resolution_plan_id = overlay_trailer(&message, OVERLAY_PLAN_TRAILER);
    if let Some(plan_id) = &resolution_plan_id
        && (!matches!(plan_id.len(), 40 | 64) || !plan_id.chars().all(|c| c.is_ascii_hexdigit()))
    {
        bail!(
            "promote: overlay {second_parent} records a malformed resolution plan id {plan_id:?}"
        );
    }
    Ok(PromotionCheckpoint {
        merge_sha: merge_sha.to_string(),
        source_parent: first_parent.to_string(),
        swarm_head,
        resolution_plan_id,
        shape: CheckpointShape::Overlay,
    })
}

/// Every two-parent merge reachable from `source_head`, newest first.
///
/// Octopus merges are excluded by the arity filter: they are not the required
/// two-parent promotion topology even when a swarm head happens to be a parent.
fn two_parent_merges(
    port: &impl PromotePort,
    workspace_root: &Path,
    source_head: &str,
) -> Result<Vec<(String, String, String)>> {
    let output = port
        .git_output(
            workspace_root,
            &["rev-list", "--merges", "--parents", source_head],
        )
        .with_context(|| {
            format!("promote: enumerate merges reachable from source head {source_head}")
        })?;
    Ok(two_parent_merges_from_rev_list(&output))
}

/// `(merge, first parent, second parent)` for every merge with exactly two
/// parents.
fn two_parent_merges_from_rev_list(output: &str) -> Vec<(String, String, String)> {
    output
        .lines()
        .filter_map(|line| {
            // A promotion checkpoint is exactly "<merge> <parent1> <parent2>".
            let fields = line.split_whitespace().collect::<Vec<_>>();
            (fields.len() == 3).then(|| {
                (
                    fields[0].to_string(),
                    fields[1].to_string(),
                    fields[2].to_string(),
                )
            })
        })
        .collect()
}

/// Locate the checkpoint that landed `swarm_sha`, in either shape.
fn find_regular_merge_landing(
    port: &impl PromotePort,
    workspace_root: &Path,
    source_head: &str,
    swarm_sha: &str,
) -> Result<Option<PromotionCheckpoint>> {
    for (merge, first, second) in two_parent_merges(port, workspace_root, source_head)? {
        let checkpoint = parse_promotion_checkpoint(port, workspace_root, &merge, &first, &second)?;
        if checkpoint.swarm_head == swarm_sha {
            return Ok(Some(checkpoint));
        }
    }
    Ok(None)
}

/// Value of a single-line overlay trailer, if present.
fn overlay_trailer(message: &str, trailer: &str) -> Option<String> {
    message.lines().rev().find_map(|line| {
        line.trim()
            .strip_prefix(trailer)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
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

/// Confirm the existing promotion branch can be fast-forwarded to the overlay
/// this run prepared.
///
/// The comparison deliberately runs against `source_head`, not against the
/// prepared overlay. The overlay is a local commit that has not been pushed, so
/// the source repository cannot compare it: asking the compare API about an
/// object it does not have fails outright. The substitution is exact rather than
/// approximate, because the overlay is a single new commit whose parent is
/// `source_head` (or is `source_head` itself when the trees already match). Its
/// ancestors are therefore `source_head`'s ancestors plus itself, so
/// `current_target` is an ancestor of the overlay exactly when it is an ancestor
/// of `source_head`. Callers skip this check when the branch already points at
/// the prepared overlay.
fn ensure_remote_fast_forward(
    port: &impl PromotePort,
    current_target: &str,
    source_head: &str,
    requested_target: &str,
) -> Result<()> {
    let comparison = format!("{current_target}...{source_head}");
    let output = port
        .gh_output(&["api", &format!("repos/{SOURCE_REPO}/compare/{comparison}")])
        .with_context(|| {
            format!(
                "promote: compare remote branch target {current_target} to source head {source_head} in swarm authority"
            )
        })?;
    let receipt: CompareReceipt = serde_json::from_slice(&output)
        .context("promote: parse source branch ancestry comparison")?;
    if !matches!(receipt.status.as_str(), "ahead" | "identical") {
        bail!(
            "promote: existing promotion branch target {current_target} is not fast-forwardable to {requested_target} (compare status {:?} against source head {source_head})",
            receipt.status
        );
    }
    Ok(())
}

fn discover_source_pr(port: &impl PromotePort, branch: &str) -> Result<Option<SourcePullRequest>> {
    let output = port.gh_output(&[
        "pr",
        "list",
        "--repo",
        SOURCE_REPO,
        "--state",
        "open",
        "--head",
        branch,
        "--json",
        "number,url,headRefName,headRefOid,baseRefName,headRepository,headRepositoryOwner,title,body",
    ])?;
    let mut prs: Vec<SourcePullRequest> =
        serde_json::from_slice(&output).context("promote: parse canonical source PR list")?;
    if prs.len() > 1 {
        bail!("promote: multiple open source PRs use deterministic branch {branch:?}");
    }
    let pr = prs.pop();
    if let Some(pr) = pr.as_ref()
        && !promote_source_pr_matches_identity(pr, branch)
    {
        bail!(
            "promote: open source PR #{} is incompatible with deterministic {branch:?} -> main identity",
            pr.number
        );
    }
    Ok(pr)
}

fn promote_source_pr_matches_identity(pr: &SourcePullRequest, branch: &str) -> bool {
    if pr.head_ref_name != branch {
        return false;
    }
    if pr.base_ref_name != "main" {
        return false;
    }
    if pr.head_repository_owner.login != "EffortlessMetrics" {
        return false;
    }
    if !pr.head_repository.name_with_owner.is_empty() {
        return pr.head_repository.name_with_owner == SOURCE_REPO;
    }
    !pr.head_repository.name.is_empty() && pr.head_repository.name == "shiplog"
}

fn execute_source_pr(
    port: &impl PromotePort,
    action: PullRequestAction,
    existing: Option<&SourcePullRequest>,
    branch: &str,
    prepared_overlay_sha: &str,
    title: &str,
    body: &str,
    body_path: &Path,
) -> Result<SourcePullRequest> {
    let body_path = body_path
        .to_str()
        .context("promote: promotion body path is not UTF-8")?;
    match action {
        PullRequestAction::Create => {
            let output = port.gh_output(&[
                "pr",
                "create",
                "--repo",
                SOURCE_REPO,
                "--base",
                "main",
                "--head",
                branch,
                "--title",
                title,
                "--body-file",
                body_path,
            ])?;
            let url = String::from_utf8(output)
                .context("promote: source PR create output is not UTF-8")?
                .trim()
                .to_string();
            let number = url
                .rsplit('/')
                .next()
                .and_then(|value| value.parse::<u64>().ok())
                .with_context(|| format!("promote: parse created source PR URL {url:?}"))?;
            Ok(SourcePullRequest {
                number,
                url,
                head_ref_name: branch.to_string(),
                head_ref_oid: prepared_overlay_sha.to_string(),
                base_ref_name: "main".to_string(),
                head_repository: RepositoryIdentity {
                    name_with_owner: SOURCE_REPO.to_string(),
                    name: "shiplog".to_string(),
                },
                head_repository_owner: RepositoryOwner {
                    login: "EffortlessMetrics".to_string(),
                },
                title: title.to_string(),
                body: body.to_string(),
            })
        }
        PullRequestAction::Update => {
            let pr = existing.context("promote: update action lacks existing source PR")?;
            port.gh_output(&[
                "pr",
                "edit",
                &pr.number.to_string(),
                "--repo",
                SOURCE_REPO,
                "--title",
                title,
                "--body-file",
                body_path,
            ])?;
            let mut updated = pr.clone();
            updated.head_ref_oid = prepared_overlay_sha.to_string();
            updated.title = title.to_string();
            updated.body = body.to_string();
            Ok(updated)
        }
        PullRequestAction::AlreadyCurrent => existing
            .cloned()
            .context("promote: already-current action lacks source PR"),
    }
}

fn load_source_only_paths(workspace_root: &Path) -> Result<Vec<String>> {
    let path = workspace_root.join(SOURCE_ONLY_PATH_POLICY);
    let text =
        fs::read_to_string(&path).with_context(|| format!("promote: read {}", path.display()))?;
    let policy: SourceOnlyPathsPolicy =
        toml::from_str(&text).with_context(|| format!("promote: parse {}", path.display()))?;
    let _ = &policy.owner;
    if policy.schema_version != 1
        || policy.policy != "source-only-paths"
        || policy.status != "blocking"
    {
        bail!(
            "{} requires schema_version=1, policy=source-only-paths, status=blocking",
            path.display()
        );
    }
    let mut allow = Vec::new();
    let mut seen = BTreeSet::new();
    for entry in policy.allow {
        let _ = (
            &entry.owner,
            &entry.reason,
            &entry.classification,
            &entry.created,
            &entry.review_after,
        );
        let normalized = normalized_source_only_path(&entry.path)?;
        if seen.insert(normalized.clone()) {
            allow.push(normalized);
        }
    }
    Ok(allow)
}

/// Derive the evidence-bearing authority used by both planning and
/// post-merge verification. Keeping this in one helper prevents verify-only
/// from silently using a weaker policy/evidence interpretation than overlay
/// construction.
fn derive_promotion_transition_authority(
    port: &impl PromotePort,
    workspace_root: &Path,
    source_head: &str,
    swarm_sha: &str,
    source_only_paths: &[String],
    state: &PromotionState,
) -> Result<super::transition::TransitionAuthority> {
    let refs = super::transition::TransitionRefs {
        source_repo: SOURCE_REPO,
        swarm_repo: SWARM_REPO,
        source_target: source_head,
        swarm_target: swarm_sha,
    };
    let mut authority =
        super::transition::derive_authority(port, workspace_root, &refs, &state.transition)?;
    authority.source_authority = super::transition::derive_source_authority(
        port,
        workspace_root,
        &refs,
        source_only_paths,
        &state.source_authority,
    )?;
    Ok(authority)
}

/// Build and validate the exact per-path plan used by both promotion planning
/// and modern post-merge verification. The output sink is shared so a blocked
/// verification reports the same machine-readable repair plan as a blocked
/// promotion run.
fn compute_resolution_plan(
    port: &impl PromotePort,
    workspace_root: &Path,
    source_head: &str,
    swarm_sha: &str,
    source_only_paths: &[String],
    transition_authority: &super::transition::TransitionAuthority,
    output: &mut dyn Write,
) -> Result<(OverlayPlan, String)> {
    let merge_base = port
        .git_output(workspace_root, &["merge-base", source_head, swarm_sha])
        .with_context(|| {
            format!(
                "promote: determine merge base between source head {source_head} and swarm head {swarm_sha}"
            )
        })?;
    if merge_base.is_empty() {
        bail!("promote: merge-base returned no commit for the promotion plan");
    }
    let plan = plan_path_resolutions(
        port,
        workspace_root,
        source_head,
        swarm_sha,
        &merge_base,
        source_only_paths,
        transition_authority,
    )?;
    ensure_no_blocked_paths(&plan, output)?;
    Ok((plan, merge_base))
}

fn normalized_source_only_path(path: &str) -> Result<String> {
    let path = path.trim();
    if path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        bail!("source-only path must be a normalized repository-relative path: {path:?}");
    }
    Ok(path.to_string())
}

/// What the overlay does with one differing path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum PathEffect {
    /// The overlay restores the source content, including its absence.
    TakeSource,
    /// The overlay keeps the swarm content.
    TakeSwarm,
    /// The promotion is refused.
    Block,
}

/// Why an effect was earned.
///
/// Recorded alongside the effect so the receipt answers both questions a reader
/// has: what happened to this path, and what entitled it. An effect without a
/// basis is indistinguishable from a guess.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum ResolutionBasis {
    /// Listed in `policy/source-only-paths.toml`. The only basis that may select
    /// source content.
    SourceOnlyPolicy,
    /// Ordinary product content: swarm changed it and source did not.
    SwarmProductChange,
    /// A receipt proves the two changes are reconciled.
    ResolvedTransition {
        source_pr: String,
        swarm_chain: Vec<String>,
        disposition: String,
    },
    /// An explicit reviewed decision discards a source-only transition change
    /// and keeps the exact swarm tree entry for this bounded promotion.
    DiscardedSource {
        source_pr: String,
        decision_receipt: String,
        decision_merge_sha: String,
        reason: String,
    },
    /// An explicit reviewed decision keeps the exact source tree entry for a
    /// source-authoritative path that swarm changed during this promotion.
    KeptSource {
        decision_receipt: String,
        decision_merge_sha: String,
        reason: String,
    },
    /// Swarm changed a path the policy reserves to source, so the overlay would
    /// revert it.
    SwarmChangedSourceAuthoritative,
    /// Only source changed it, so keeping swarm content would revert it.
    SourceChangeNotInSwarm {
        #[serde(skip_serializing_if = "Option::is_none")]
        source_pr: Option<String>,
    },
    /// Both repositories changed it with no receipt proving reconciliation.
    UnprovenTwoSided,
}

impl ResolutionBasis {
    /// Operator-facing explanation for a refused path.
    fn refusal(&self, path: &str) -> String {
        match self {
            Self::SwarmChangedSourceAuthoritative => format!(
                "{path}: swarm changed a source-authoritative path; move it out of {SOURCE_ONLY_PATH_POLICY} or drop the swarm change"
            ),
            Self::SourceChangeNotInSwarm {
                source_pr: Some(receipt),
            } => format!(
                "{path}: swarm does not carry {receipt}'s change; port it to swarm and record it as equivalent or superseded, or make the path source-authoritative in {SOURCE_ONLY_PATH_POLICY}"
            ),
            Self::SourceChangeNotInSwarm { source_pr: None } => format!(
                "{path}: only source changed this, so promoting would revert it; port the change to swarm and record it as equivalent or superseded, or make the path source-authoritative in {SOURCE_ONLY_PATH_POLICY}"
            ),
            Self::UnprovenTwoSided => format!(
                "{path}: both repositories changed this path with no receipt proving they are reconciled"
            ),
            Self::SourceOnlyPolicy
            | Self::SwarmProductChange
            | Self::ResolvedTransition { .. }
            | Self::DiscardedSource { .. }
            | Self::KeptSource { .. } => {
                format!("{path}: resolved")
            }
        }
    }
}

/// One path's exact state, the effect the overlay applies, and why.
#[derive(Clone, Debug, Serialize)]
struct PathDecision {
    path: String,
    /// Blob at the exact source head, or absent when the path is not there.
    #[serde(skip_serializing_if = "Option::is_none")]
    source_blob: Option<String>,
    /// Blob at the exact swarm head, or absent when the path is not there.
    #[serde(skip_serializing_if = "Option::is_none")]
    swarm_blob: Option<String>,
    source_changed: bool,
    swarm_changed: bool,
    effect: PathEffect,
    basis: ResolutionBasis,
}

/// Per-path plan for one promotion.
///
/// Acceptance and content used to be decided independently: alignment consulted
/// the policy list plus transition receipts, while overlay construction
/// consulted the policy list alone. Every disagreement between them silently
/// dropped one side's work. Both now come from these decisions.
///
/// Paths identical in both trees are absent by construction, since the plan is
/// built from the diff between them.
#[derive(Debug, Default, Serialize)]
struct OverlayPlan {
    decisions: Vec<PathDecision>,
}

impl OverlayPlan {
    /// Paths the overlay must restore from source.
    fn take_source(&self) -> Vec<String> {
        self.decisions
            .iter()
            .filter(|decision| decision.effect == PathEffect::TakeSource)
            .map(|decision| decision.path.clone())
            .collect()
    }

    fn blocked(&self) -> Vec<&PathDecision> {
        self.decisions
            .iter()
            .filter(|decision| decision.effect == PathEffect::Block)
            .collect()
    }

    fn take_swarm_count(&self) -> usize {
        self.decisions
            .iter()
            .filter(|decision| decision.effect == PathEffect::TakeSwarm)
            .count()
    }
}

/// Decide what happens to every path that differs between source and swarm.
///
/// Only `policy/source-only-paths.toml` can select source content. A transition
/// A resolved transition or an explicit discard decision may permit the overlay
/// to keep swarm content on a path both sides changed, but can never
/// manufacture source authority. That preserves swarm as the product mutation
/// authority while making exceptional source abandonment explicit.
fn plan_path_resolutions(
    port: &impl PromotePort,
    workspace_root: &Path,
    source_head: &str,
    swarm_sha: &str,
    merge_base: &str,
    source_only_paths: &[String],
    transition_authority: &super::transition::TransitionAuthority,
) -> Result<OverlayPlan> {
    for path in transition_authority.source_authority.keys() {
        if transition_authority.discard_source.contains_key(path) {
            bail!("path {path} has both discard_source and source_authority decisions");
        }
    }
    let differing = git_diff_names(port, workspace_root, source_head, swarm_sha)?;
    let source_changed = git_diff_names_set(port, workspace_root, merge_base, source_head)?;
    let swarm_changed = git_diff_names_set(port, workspace_root, merge_base, swarm_sha)?;
    let source_blobs = tree_blobs(port, workspace_root, source_head)?;
    let swarm_blobs = tree_blobs(port, workspace_root, swarm_sha)?;
    let source_authoritative = source_only_paths
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();

    let mut decisions = Vec::new();
    for path in differing {
        let touched_by_source = source_changed.contains(path.as_str());
        let touched_by_swarm = swarm_changed.contains(path.as_str());
        let (effect, basis) = if let Some(decision) = transition_authority
            .discard_source
            .get(path.as_str())
            .filter(|_| touched_by_source)
        {
            (
                PathEffect::TakeSwarm,
                ResolutionBasis::DiscardedSource {
                    source_pr: decision.source_pr.clone(),
                    decision_receipt: decision.decision_receipt.clone(),
                    decision_merge_sha: decision.decision_merge_sha.clone(),
                    reason: decision.reason.clone(),
                },
            )
        } else if let Some(decision) = transition_authority
            .source_authority
            .get(path.as_str())
            .filter(|_| touched_by_swarm)
        {
            (
                PathEffect::TakeSource,
                ResolutionBasis::KeptSource {
                    decision_receipt: decision.decision_receipt.clone(),
                    decision_merge_sha: decision.decision_merge_sha.clone(),
                    reason: decision.reason.clone(),
                },
            )
        } else if source_authoritative.contains(path.as_str()) {
            if touched_by_swarm {
                (
                    PathEffect::Block,
                    ResolutionBasis::SwarmChangedSourceAuthoritative,
                )
            } else {
                (PathEffect::TakeSource, ResolutionBasis::SourceOnlyPolicy)
            }
        } else if let Some(receipt) = transition_authority
            .two_sided
            .get(path.as_str())
            .filter(|_| touched_by_source && touched_by_swarm)
        {
            (
                PathEffect::TakeSwarm,
                ResolutionBasis::ResolvedTransition {
                    source_pr: receipt.source_pr.clone(),
                    swarm_chain: receipt.swarm_chain.clone(),
                    disposition: receipt.disposition.to_string(),
                },
            )
        } else if touched_by_source && !touched_by_swarm {
            (
                PathEffect::Block,
                ResolutionBasis::SourceChangeNotInSwarm {
                    source_pr: transition_authority
                        .awaiting_swarm
                        .get(path.as_str())
                        .cloned(),
                },
            )
        } else if touched_by_source && touched_by_swarm {
            (PathEffect::Block, ResolutionBasis::UnprovenTwoSided)
        } else {
            (PathEffect::TakeSwarm, ResolutionBasis::SwarmProductChange)
        };
        decisions.push(PathDecision {
            source_blob: source_blobs.get(path.as_str()).cloned(),
            swarm_blob: swarm_blobs.get(path.as_str()).cloned(),
            path,
            source_changed: touched_by_source,
            swarm_changed: touched_by_swarm,
            effect,
            basis,
        });
    }
    Ok(OverlayPlan { decisions })
}

/// Refuse a plan that cannot be applied without discarding work.
///
/// Emits the machine-readable unresolved decisions so a caller gets an exact
/// repair queue instead of a generic divergence message.
fn ensure_no_blocked_paths(plan: &OverlayPlan, output: &mut dyn Write) -> Result<()> {
    let blocked = plan.blocked();
    if blocked.is_empty() {
        return Ok(());
    }
    let unresolved = OverlayPlan {
        decisions: blocked.iter().map(|decision| (*decision).clone()).collect(),
    };
    let json = serde_json::to_string_pretty(&unresolved)
        .context("promote: serialize unresolved path plan")?;
    writeln!(output, "{json}").context("promote: write unresolved path plan")?;
    bail!(
        "promote: cannot resolve {} path(s) without discarding work:\n  {}",
        blocked.len(),
        blocked
            .iter()
            .map(|decision| decision.basis.refusal(&decision.path))
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}

/// `path -> blob oid` for every file in `revision`.
///
/// Read in one call per side rather than per path: a promotion can touch
/// hundreds of files, and the blobs bind a decision to the exact tree state it
/// was made against.
fn tree_blobs(
    port: &impl PromotePort,
    workspace_root: &Path,
    revision: &str,
) -> Result<BTreeMap<String, String>> {
    // NUL-delimited: path identity is part of the promotion authority contract,
    // so a path containing whitespace, a tab, or a quote must not be able to
    // reshape the parse.
    let output = port
        .git_output(workspace_root, &["ls-tree", "-rz", "--full-tree", revision])
        .with_context(|| format!("promote: read tree of {revision}"))?;
    Ok(parse_tree_blobs(&output))
}

/// Parse `git ls-tree -rz --full-tree` output into `path -> blob oid`.
fn parse_tree_blobs(listing: &str) -> BTreeMap<String, String> {
    listing
        .split('\0')
        .filter(|entry| !entry.is_empty())
        .filter_map(|entry| {
            // "<mode> <type> <oid>\t<path>"
            let (meta, path) = entry.split_once('\t')?;
            Some((
                path.to_string(),
                meta.split_whitespace().nth(2)?.to_string(),
            ))
        })
        .collect()
}

fn git_diff_names(
    port: &impl PromotePort,
    workspace_root: &Path,
    left: &str,
    right: &str,
) -> Result<Vec<String>> {
    // NUL-delimited, and renames disabled. A path containing whitespace or a
    // quote must not reshape the parse, and a rename reported as one entry would
    // hide what is really a deletion plus an addition, each of which needs its
    // own resolution.
    let output = port.git_output(
        workspace_root,
        &["diff", "--name-only", "--no-renames", "-z", left, right],
    )?;
    Ok(output
        .split('\0')
        .filter(|entry| !entry.is_empty())
        .map(str::to_string)
        .collect())
}

fn git_diff_names_set(
    port: &impl PromotePort,
    workspace_root: &Path,
    left: &str,
    right: &str,
) -> Result<BTreeSet<String>> {
    Ok(git_diff_names(port, workspace_root, left, right)?
        .into_iter()
        .collect())
}

/// Content hash of a resolution plan, via `git hash-object`.
///
/// Uses git's own object hashing rather than adding a digest dependency. The plan
/// is fully determined by the promotion inputs, so a planning run and the
/// executing run that follows it produce the same id.
fn resolution_plan_id(plan: &OverlayPlan) -> Result<String> {
    let json = serde_json::to_string(plan).context("promote: serialize resolution plan")?;
    super::transition::git_hash_object(&json)
}

/// Build the source overlay commit: the swarm tree applied onto source head with
/// policy-approved source-only paths taken from source.
///
/// `isolate_objects` redirects newly written git objects into a throwaway store
/// that cleanup deletes, so a planning-only run leaves the repository's object
/// database untouched. The overlay commit is fully deterministic (fixed author,
/// committer, and dates), so the sha reported from an isolated run is the same
/// sha a later executing run materializes for real.
fn prepare_source_overlay(
    port: &impl PromotePort,
    workspace_root: &Path,
    source_head: &str,
    swarm_sha: &str,
    take_source: &[String],
    plan_id: &str,
    isolate_objects: bool,
) -> Result<PreparedOverlay> {
    let workspace = OverlayWorkspace::claim(
        port,
        workspace_root,
        source_head,
        swarm_sha,
        isolate_objects,
    )?;
    let prepared = (|| -> Result<String> {
        let overlay_root = workspace.path();
        let env = workspace.git_env();
        let git = |args: &[&str]| port.git_output_with_env(overlay_root, args, &env);
        // `read-tree -u --reset` makes the worktree exactly the swarm tree.
        // `checkout <swarm> -- .` copies swarm's files over but never removes a
        // file swarm deleted, so the overlay silently retained it and the
        // promotion dropped that deletion.
        git(&["read-tree", "-u", "--reset", swarm_sha])?;
        for path in take_source {
            let pathspec = literal_pathspec(path);
            // A resolved source path must match the source tree exactly, which
            // includes its absence. Restoring only when source still has the
            // path would let a file source deliberately deleted survive in the
            // overlay via the swarm copy; the post-construction blob check below
            // is what now catches such a reintroduction.
            if tree_has_path_with_env(port, overlay_root, source_head, path.as_str(), &env)? {
                git(&["checkout", source_head, "--", &pathspec]).with_context(|| {
                    format!(
                        "promote: restore resolved source path {path} while applying {swarm_sha}"
                    )
                })?;
            } else {
                git(&["rm", "-r", "--force", "--ignore-unmatch", "--", &pathspec]).with_context(
                    || {
                        format!(
                            "promote: honour source deletion of resolved path {path} while applying {swarm_sha}"
                        )
                    },
                )?;
            }
        }
        git(&["add", "-A"])?;
        let source_date = git(&["show", "-s", "--format=%cI", source_head])
            .with_context(|| format!("promote: read committer date for source {source_head}"))?;
        let commit_env = [
            ("GIT_AUTHOR_NAME", "shiplog-promote[bot]"),
            (
                "GIT_AUTHOR_EMAIL",
                "shiplog-promote[bot]@users.noreply.github.com",
            ),
            ("GIT_COMMITTER_NAME", "shiplog-promote[bot]"),
            (
                "GIT_COMMITTER_EMAIL",
                "shiplog-promote[bot]@users.noreply.github.com",
            ),
            ("GIT_AUTHOR_DATE", source_date.as_str()),
            ("GIT_COMMITTER_DATE", source_date.as_str()),
        ];
        // Assembled with explicit newlines: a multi-line literal would carry its
        // own source indentation into the commit body and mangle the trailers.
        let message = format!(
            "chore(promote): overlay source with swarm {short}\n\n{OVERLAY_SOURCE_TRAILER} {source_head}\n{OVERLAY_SWARM_TRAILER} {swarm_sha}\n{OVERLAY_PLAN_TRAILER} {plan_id}\n",
            short = &swarm_sha[..12]
        );
        let mut commit_env: Vec<(&str, &str)> = commit_env.to_vec();
        commit_env.extend(env.iter().copied());
        // The overlay is the transaction checkpoint, not merely a carrier for
        // changed paths. Preserve it even when source and swarm already have
        // identical trees.
        port.git_output_with_env(
            overlay_root,
            &["commit", "--allow-empty", "--no-gpg-sign", "-m", &message],
            &commit_env,
        )?;
        let overlay_sha = git(&["rev-parse", "HEAD"])?;
        // The invariant that closes the silent-discard class. Every path in any of
        // the three trees is checked by blob, not by name: a resolved path must
        // hold the exact source blob (or be absent exactly as source has it), and
        // every other path must hold the exact swarm blob. Comparing which names
        // differ would accept a resolved path holding wrong content, since its
        // name is permitted to differ. This fully specifies the overlay tree, so
        // anything dropped, reverted, or reintroduced surfaces here no matter
        // which step got it wrong.
        let read_blobs = |revision: &str| -> Result<BTreeMap<String, String>> {
            let listing = git(&["ls-tree", "-rz", "--full-tree", revision])?;
            Ok(parse_tree_blobs(&listing))
        };
        let overlay_blobs = read_blobs(&overlay_sha)?;
        let source_blobs = read_blobs(source_head)?;
        let swarm_blobs = read_blobs(swarm_sha)?;
        let resolved_to_source = take_source.iter().cloned().collect::<BTreeSet<_>>();
        let mut paths = BTreeSet::new();
        paths.extend(overlay_blobs.keys().cloned());
        paths.extend(source_blobs.keys().cloned());
        paths.extend(swarm_blobs.keys().cloned());
        for path in paths {
            let expected = if resolved_to_source.contains(&path) {
                source_blobs.get(&path)
            } else {
                swarm_blobs.get(&path)
            };
            let actual = overlay_blobs.get(&path);
            if actual != expected {
                let side = if resolved_to_source.contains(&path) {
                    "source"
                } else {
                    "swarm"
                };
                bail!(
                    "promote: overlay {overlay_sha} holds {actual:?} at {path}, but the resolution plan requires the {side} blob {expected:?}"
                );
            }
        }
        Ok(overlay_sha)
    })();
    let cleanup_warnings = workspace.release();
    match prepared {
        Ok(sha) => Ok(PreparedOverlay {
            sha,
            cleanup_warnings,
        }),
        Err(error) => {
            // A run that failed mid-overlay is the case most likely to leave
            // residue, so surface the warnings here: there is no receipt to
            // carry them on this path.
            for warning in &cleanup_warnings {
                eprintln!("promote: warning: {warning}");
            }
            Err(error)
        }
    }
}

/// A promotion overlay worktree that this process exclusively owns.
///
/// Overlay worktrees live under a shared `target/promotion-overlay` parent that
/// concurrent promotions and every other `target/` consumer also use, so the
/// parent is never removed and only the exact child claimed here is ever
/// deleted. Cleanup runs on every exit path, including `?` returns and panics,
/// because an abandoned overlay leaves residue in two independent places: a
/// directory git no longer tracks, and a worktree registration whose directory
/// is gone. Removing the registration, removing the directory, and pruning are
/// therefore attempted independently rather than short-circuiting on the first
/// success.
struct OverlayWorkspace<'a, P: PromotePort> {
    port: &'a P,
    workspace_root: &'a Path,
    path: PathBuf,
    /// Throwaway object store for a planning-only run, held as a sibling of the
    /// worktree so `git add -A` never sees it. `None` when the overlay must
    /// persist because the run will push it.
    object_dir: Option<PathBuf>,
    /// Environment redirecting object writes into `object_dir`, with the real
    /// object directory as an alternate so reads still resolve.
    env: Vec<(String, String)>,
    warnings: Vec<String>,
    released: bool,
}

impl<'a, P: PromotePort> OverlayWorkspace<'a, P> {
    /// Claim a uniquely named overlay directory and register it as a detached
    /// worktree at `source_head`.
    ///
    /// The name carries the swarm sha for diagnosability plus a pid and a
    /// time-derived nonce, and `fs::create_dir` is what actually establishes
    /// exclusivity: it fails when the name is taken, so a collision retries
    /// instead of adopting a directory another promotion may still be using.
    /// The pid alone is not enough, because pids are reused after a crash that
    /// left residue behind.
    fn claim(
        port: &'a P,
        workspace_root: &'a Path,
        source_head: &str,
        swarm_sha: &str,
        isolate_objects: bool,
    ) -> Result<Self> {
        let parent = workspace_root.join("target").join("promotion-overlay");
        fs::create_dir_all(&parent).with_context(|| {
            format!(
                "promote: create overlay parent directory {}",
                parent.display()
            )
        })?;
        let pid = process_id();
        let mut path = None;
        for attempt in 0..64u32 {
            let candidate = parent.join(format!(
                "{}-{pid}-{:x}",
                &swarm_sha[..12],
                overlay_nonce(attempt)
            ));
            match fs::create_dir(&candidate) {
                Ok(()) => {
                    path = Some(candidate);
                    break;
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(error).with_context(|| {
                        format!("promote: claim overlay workspace {}", candidate.display())
                    });
                }
            }
        }
        let path = path.with_context(|| {
            format!(
                "promote: could not claim a unique overlay workspace under {}",
                parent.display()
            )
        })?;
        let (object_dir, env) = if isolate_objects {
            // A sibling of the worktree, never a child: anything inside the
            // worktree would be picked up by `git add -A`. Named by appending
            // rather than via `with_extension`, which would instead replace part
            // of the claimed name if the name format ever gained a dot.
            let mut object_name = path
                .file_name()
                .context("promote: overlay workspace has no file name")?
                .to_os_string();
            object_name.push(".objects");
            let object_dir = parent.join(object_name);
            for sub in ["", "info", "pack"] {
                fs::create_dir_all(object_dir.join(sub)).with_context(|| {
                    format!(
                        "promote: create isolated object store {}",
                        object_dir.display()
                    )
                })?;
            }
            let real_objects = port
                .git_output(
                    workspace_root,
                    &[
                        "rev-parse",
                        "--path-format=absolute",
                        "--git-path",
                        "objects",
                    ],
                )
                .context("promote: resolve repository object directory")?;
            let env = vec![
                (
                    "GIT_OBJECT_DIRECTORY".to_string(),
                    object_dir.to_string_lossy().into_owned(),
                ),
                ("GIT_ALTERNATE_OBJECT_DIRECTORIES".to_string(), real_objects),
            ];
            (Some(object_dir), env)
        } else {
            (None, Vec::new())
        };
        let workspace = Self {
            port,
            workspace_root,
            path,
            object_dir,
            env,
            warnings: Vec::new(),
            released: false,
        };
        let path_arg = workspace.path_arg();
        port.git_output(
            workspace_root,
            &["worktree", "add", "--detach", &path_arg, source_head],
        )
        .with_context(|| format!("promote: add overlay worktree at {path_arg}"))?;
        Ok(workspace)
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn path_arg(&self) -> String {
        self.path.to_string_lossy().into_owned()
    }

    /// Environment for git calls inside this overlay. Empty when objects are not
    /// isolated, so callers can pass it unconditionally.
    fn git_env(&self) -> Vec<(&str, &str)> {
        self.env
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
            .collect()
    }

    /// Clean up and surface any residue that survived, so the caller can record
    /// it in the receipt instead of leaving the workspace in an unknown state.
    fn release(mut self) -> Vec<String> {
        self.cleanup();
        std::mem::take(&mut self.warnings)
    }

    fn cleanup(&mut self) {
        if self.released {
            return;
        }
        self.released = true;
        let path_arg = self.path_arg();
        // Deregistration failing is not itself a problem: the directory may have
        // been claimed but never registered, or already removed by hand. What
        // matters is the end state, which the directory check and the prune below
        // establish independently.
        let _ = self.port.git_output(
            self.workspace_root,
            &["worktree", "remove", "--force", &path_arg],
        );
        if let Err(error) = fs::remove_dir_all(&self.path)
            && self.path.exists()
        {
            self.warnings.push(format!(
                "failed to remove overlay workspace {path_arg}: {error}"
            ));
        }
        if let Some(object_dir) = &self.object_dir
            && let Err(error) = fs::remove_dir_all(object_dir)
            && object_dir.exists()
        {
            self.warnings.push(format!(
                "failed to remove isolated object store {}: {error}",
                object_dir.display()
            ));
        }
        if let Err(error) = self
            .port
            .git_output(self.workspace_root, &["worktree", "prune"])
        {
            self.warnings.push(format!(
                "failed to prune stale worktree registrations: {error}"
            ));
        }
    }
}

impl<P: PromotePort> Drop for OverlayWorkspace<'_, P> {
    fn drop(&mut self) {
        self.cleanup();
        for warning in &self.warnings {
            eprintln!("promote: warning: {warning}");
        }
    }
}

/// Nonce for overlay directory names. Mixed with the attempt counter so a
/// same-nanosecond retry picks a different name.
fn overlay_nonce(attempt: u32) -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos() as u64)
        .unwrap_or_default();
    nanos.wrapping_add(u64::from(attempt).wrapping_mul(0x9E37_79B9_7F4A_7C15))
}

fn tree_has_path_with_env(
    port: &impl PromotePort,
    workspace_root: &Path,
    revision: &str,
    path: &str,
    env: &[(&str, &str)],
) -> Result<bool> {
    let pathspec = literal_pathspec(path);
    let output = port
        .git_output_with_env(
            workspace_root,
            &["ls-tree", "-r", "--name-only", revision, "--", &pathspec],
            env,
        )
        .with_context(|| format!("promote: inspect path {path} in {revision}"))?;
    let output = output.trim().to_string();
    Ok(output.lines().any(|line| line == path))
}

fn literal_pathspec(path: &str) -> String {
    format!(":(literal){path}")
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
        || state.latest_promotion.source_merge_sha.is_empty()
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
            [first, second] => {
                // The same parser closeout uses. Matching the raw swarm head here
                // meant the next promotion rejected the checkpoint the previous
                // one created, because an overlay checkpoint's second parent is
                // the overlay, not the swarm head.
                let checkpoint =
                    parse_promotion_checkpoint(port, workspace_root, &cursor, first, second)?;
                if checkpoint.swarm_head == promoted_swarm_head {
                    return Ok(cursor);
                }
                bail!(
                    "promote: source commit {cursor} lands swarm head {}, not the recorded promotion checkpoint for {promoted_swarm_head}",
                    checkpoint.swarm_head
                )
            }
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
    git_output_with_env(workspace_root, args, &[])
}

/// Bridge so every promote port can also serve the transition evidence checks.
impl<P: PromotePort> super::transition::TransitionPort for P {
    fn git_output(&self, workspace_root: &Path, args: &[&str]) -> Result<String> {
        PromotePort::git_output(self, workspace_root, args)
    }

    fn gh_output(&self, args: &[&str]) -> Result<Vec<u8>> {
        PromotePort::gh_output(self, args)
    }

    fn git_patch_id(&self, patch: &str) -> Result<String> {
        PromotePort::git_patch_id(self, patch)
    }
}

fn git_output_with_env(
    workspace_root: &Path,
    args: &[&str],
    env: &[(&str, &str)],
) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(workspace_root)
        .envs(env.iter().map(|(key, value)| (*key, *value)))
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

    fn git_output_with_env(
        &self,
        workspace_root: &Path,
        args: &[&str],
        env: &[(&str, &str)],
    ) -> Result<String> {
        git_output_with_env(workspace_root, args, env)
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
        git_mutations: RefCell<Vec<Vec<String>>>,
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

        fn git_output_with_env(
            &self,
            workspace_root: &Path,
            args: &[&str],
            env: &[(&str, &str)],
        ) -> Result<String> {
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
            SystemPort.git_output_with_env(workspace_root, args, env)
        }

        fn git_status(&self, workspace_root: &Path, args: &[&str]) -> Result<()> {
            if args.first() == Some(&"push") {
                self.git_mutations
                    .borrow_mut()
                    .push(args.iter().map(|arg| (*arg).to_string()).collect());
                return Ok(());
            }
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
        fs::create_dir_all(dir.path().join("policy"))?;
        fs::write(
            dir.path().join("policy/source-only-paths.toml"),
            "schema_version = 1\npolicy = \"source-only-paths\"\nowner = \"repo-infra/release\"\nstatus = \"blocking\"\n\n[[allow]]\npath = \"promoted.txt\"\nowner = \"repo-infra/release\"\nreason = \"fixture source-only path\"\nclassification = \"release-governance\"\ncreated = \"2026-07-14\"\nreview_after = \"2027-01-14\"\n\n[[allow]]\npath = \"governance.txt\"\nowner = \"repo-infra/release\"\nreason = \"fixture source-only path\"\nclassification = \"release-governance\"\ncreated = \"2026-07-14\"\nreview_after = \"2027-01-14\"\n",
        )?;
        fs::create_dir_all(dir.path().join("plans/shiplog-swarm"))?;
        fs::write(
            dir.path().join("plans/shiplog-swarm/promotion-state.toml"),
            format!(
                "schema_version = 1\n[latest_promotion]\nstatus = \"completed\"\ndisposition = \"completed-with-governance\"\nsource_promotion_pr = \"EffortlessMetrics/shiplog#655\"\nsource_merge_sha = \"160d430f1a5af338537e35ff98b8ddda14d4673c\"\npromoted_swarm_head = \"{promoted}\"\nsource_governance = [\"EffortlessMetrics/shiplog#656\"]\nsource_post_merge_proof = \"\"\nincluded_swarm_prs = [\"EffortlessMetrics/shiplog-swarm#238\"]\n[pending]\nswarm_pr_range = []\ndeferred_receipt_carry = []\n"
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

    /// Advance the fixture `source` branch by landing the swarm head (`current`)
    /// as a regular two-parent merge, mirroring a completed promotion. Returns
    /// the landed merge commit sha.
    fn advance_source_with_regular_merge(fixture: &GitFixture) -> Result<String> {
        git_fixture(fixture.dir.path(), &["switch", "source"])?;
        git_fixture(
            fixture.dir.path(),
            &["merge", "--no-ff", "swarm", "-m", "Merge promotion #700"],
        )?;
        git_fixture(fixture.dir.path(), &["rev-parse", "HEAD"])
    }

    /// Advance the fixture `source` branch by landing the swarm head as a
    /// single-parent (squash-shaped) commit instead of a regular merge.
    fn advance_source_with_squash(fixture: &GitFixture) -> Result<String> {
        git_fixture(fixture.dir.path(), &["switch", "source"])?;
        fs::write(fixture.dir.path().join("squashed.txt"), "squashed\n")?;
        git_fixture(fixture.dir.path(), &["add", "squashed.txt"])?;
        git_fixture(
            fixture.dir.path(),
            &["commit", "-m", "feat: squashed promotion (#700)"],
        )?;
        git_fixture(fixture.dir.path(), &["rev-parse", "HEAD"])
    }

    fn verify_only_inputs(fixture: &GitFixture) -> PromoteInputs {
        let mut inputs = fixture_inputs(fixture);
        inputs.dry_run = false;
        inputs.verify_only = true;
        inputs
    }

    #[test]
    fn verify_only_confirms_regular_merge_landed_with_expected_ancestry() -> Result<()> {
        let fixture = fixture_git()?;
        let landed = advance_source_with_regular_merge(&fixture)?;
        let port = stub_port(&fixture, true, true);
        let mut output = Vec::new();
        run_with_port_to(&port, verify_only_inputs(&fixture), &mut output)?;
        let receipt: serde_json::Value = serde_json::from_slice(&output)?;
        ensure!(receipt["mode"] == "verify-only");
        ensure!(receipt["landed_merge"] == landed);
        ensure!(receipt["swarm_head"] == fixture.current);
        ensure!(receipt["last_promoted_swarm_head"] == fixture.promoted);
        ensure!(
            receipt["included_swarm_prs"]
                == serde_json::json!(["EffortlessMetrics/shiplog-swarm#255"])
        );
        ensure!(receipt["checks"].as_array().is_some_and(|checks| {
            checks.iter().any(|check| {
                check.as_str().is_some_and(|text| {
                    text == "raw swarm-head checkpoint predates overlay resolution-plan evidence"
                })
            })
        }));
        // Read-only: no ref push, no PR create/edit, no on-disk artifacts.
        ensure!(port.git_mutations.borrow().is_empty());
        ensure!(!port.gh_calls.borrow().iter().any(|call| {
            call.get(1)
                .is_some_and(|action| action == "create" || action == "edit")
        }));
        ensure!(!fixture.dir.path().join("target").exists());
        Ok(())
    }

    #[test]
    fn verify_only_rejects_unlanded_promotion() -> Result<()> {
        // Source still points at the previous checkpoint; the swarm head has
        // not landed as a merge whose second parent is the requested head.
        let fixture = fixture_git()?;
        let port = stub_port(&fixture, true, true);
        let error = run_with_port_to(&port, verify_only_inputs(&fixture), &mut Vec::new())
            .expect_err("unlanded promotion must fail closed");
        ensure!(error.chain().any(|cause| {
            cause
                .to_string()
                .contains("could not confirm a regular-merge checkpoint")
        }));
        ensure!(port.git_mutations.borrow().is_empty());
        Ok(())
    }

    #[test]
    fn verify_only_rejects_squash_shaped_landing() -> Result<()> {
        let fixture = fixture_git()?;
        advance_source_with_squash(&fixture)?;
        let port = stub_port(&fixture, true, true);
        let error = run_with_port_to(&port, verify_only_inputs(&fixture), &mut Vec::new())
            .expect_err("squash-shaped landing must fail closed");
        ensure!(error.chain().any(|cause| {
            cause
                .to_string()
                .contains("could not confirm a regular-merge checkpoint")
        }));
        ensure!(port.git_mutations.borrow().is_empty());
        Ok(())
    }

    #[test]
    fn verify_only_confirms_landing_despite_later_source_commits() -> Result<()> {
        // A genuine regular-merge landing must still verify after unrelated
        // commits land on source before promotion-state.toml is updated.
        let fixture = fixture_git()?;
        let landed = advance_source_with_regular_merge(&fixture)?;
        git_fixture(fixture.dir.path(), &["switch", "source"])?;
        fs::write(fixture.dir.path().join("later.txt"), "later\n")?;
        git_fixture(fixture.dir.path(), &["add", "later.txt"])?;
        git_fixture(
            fixture.dir.path(),
            &["commit", "-m", "chore: later source commit (#701)"],
        )?;
        let port = stub_port(&fixture, true, true);
        let mut output = Vec::new();
        run_with_port_to(&port, verify_only_inputs(&fixture), &mut output)?;
        let receipt: serde_json::Value = serde_json::from_slice(&output)?;
        // The landing is still recognized as the earlier merge, not the tip.
        ensure!(receipt["landed_merge"] == landed);
        ensure!(receipt["source_head"] != landed);
        ensure!(port.git_mutations.borrow().is_empty());
        Ok(())
    }

    #[test]
    fn verify_only_recomputes_modern_overlay_resolution_plan() -> Result<()> {
        let fixture = fixture_git()?;
        let root = fixture.dir.path();
        let source_only_paths = load_source_only_paths(root)?;
        let merge_base = git_fixture(root, &["merge-base", &fixture.governance, &fixture.current])?;
        let plan = resolve_paths(
            root,
            &fixture.governance,
            &fixture.current,
            &merge_base,
            &source_only_paths,
            &super::super::transition::TransitionAuthority::default(),
        )?;
        let plan_id = resolution_plan_id(&plan)?;
        let overlay = prepare_source_overlay(
            &SystemPort,
            root,
            &fixture.governance,
            &fixture.current,
            &plan.take_source(),
            &plan_id,
            false,
        )?;
        git_fixture(root, &["switch", "source"])?;
        git_fixture(
            root,
            &[
                "merge",
                "--no-ff",
                "-m",
                "merge(swarm): promote through verified overlay",
                &overlay.sha,
            ],
        )?;
        let landed = git_fixture(root, &["rev-parse", "HEAD"])?;

        let port = stub_port(&fixture, true, true);
        let mut output = Vec::new();
        run_with_port_to(&port, verify_only_inputs(&fixture), &mut output)?;
        let receipt: serde_json::Value = serde_json::from_slice(&output)?;
        ensure!(receipt["landed_merge"] == landed);
        ensure!(receipt["checkpoint_shape"] == "overlay");
        ensure!(receipt["resolution_plan_id"] == plan_id);
        ensure!(receipt["checks"].as_array().is_some_and(|checks| {
            checks.iter().any(|check| {
                check.as_str().is_some_and(|text| {
                    text.contains("matches the exact persisted promotion evidence")
                })
            })
        }));
        ensure!(port.git_mutations.borrow().is_empty());
        Ok(())
    }

    #[test]
    fn verify_only_rejects_changed_resolution_evidence() -> Result<()> {
        let fixture = fixture_git()?;
        let root = fixture.dir.path();
        let source_only_paths = load_source_only_paths(root)?;
        let merge_base = git_fixture(root, &["merge-base", &fixture.governance, &fixture.current])?;
        let plan = resolve_paths(
            root,
            &fixture.governance,
            &fixture.current,
            &merge_base,
            &source_only_paths,
            &super::super::transition::TransitionAuthority::default(),
        )?;
        let plan_id = resolution_plan_id(&plan)?;
        let overlay = prepare_source_overlay(
            &SystemPort,
            root,
            &fixture.governance,
            &fixture.current,
            &plan.take_source(),
            &plan_id,
            false,
        )?;
        git_fixture(root, &["switch", "source"])?;
        git_fixture(
            root,
            &[
                "merge",
                "--no-ff",
                "-m",
                "merge(swarm): promote through verified overlay",
                &overlay.sha,
            ],
        )?;
        let policy = root.join(SOURCE_ONLY_PATH_POLICY);
        let policy_text = fs::read_to_string(&policy)?;
        fs::write(
            &policy,
            policy_text.replace(
                "path = \"governance.txt\"",
                "path = \"changed-governance.txt\"",
            ),
        )?;

        let port = stub_port(&fixture, true, true);
        let error = run_with_port_to(&port, verify_only_inputs(&fixture), &mut Vec::new())
            .expect_err("changed resolution evidence must fail closed");
        let message = error.to_string();
        ensure!(
            message.contains("cannot resolve") || message.contains("resolution plan"),
            "unexpected changed-evidence error: {message}"
        );
        ensure!(port.git_mutations.borrow().is_empty());
        Ok(())
    }

    #[test]
    fn verify_only_rejects_octopus_merge_shaped_landing() -> Result<()> {
        let output = "\
merge-new source-parent requested-swarm-head unrelated-parent
merge-old source-parent another-swarm-head
";
        let merges = two_parent_merges_from_rev_list(output);
        ensure!(
            !merges
                .iter()
                .any(|(_, _, second)| second == "requested-swarm-head"),
            "octopus merge must not satisfy the two-parent promotion contract"
        );
        ensure!(merges.len() == 1 && merges[0].0 == "merge-old");
        ensure!(merges[0].1 == "source-parent" && merges[0].2 == "another-swarm-head");
        Ok(())
    }

    /// A resolved receipt reconciles a divergence both sides made. It must not
    /// keep applying once the shape changes: a later source-only change to an
    /// already-reconciled path would otherwise be waved through as take-swarm and
    /// silently reverted, which is the exact failure the unified planner exists
    /// to remove.
    #[test]
    fn a_stale_resolved_receipt_does_not_cover_a_later_source_only_change() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = dir.path();
        git_fixture(root, &["init", "--initial-branch=main"])?;
        git_fixture(root, &["config", "user.email", "test@example.com"])?;
        git_fixture(root, &["config", "user.name", "Promotion Test"])?;
        fs::write(root.join("shared.toml"), "base\n")?;
        git_fixture(root, &["add", "-A"])?;
        git_fixture(root, &["commit", "-m", "base"])?;
        let base = git_fixture(root, &["rev-parse", "HEAD"])?;

        git_fixture(root, &["switch", "-c", "swarm"])?;
        fs::write(root.join("unrelated.txt"), "swarm\n")?;
        git_fixture(root, &["add", "-A"])?;
        git_fixture(root, &["commit", "-m", "feat: unrelated swarm work"])?;
        let swarm_sha = git_fixture(root, &["rev-parse", "HEAD"])?;

        // Only source changes the already-reconciled path this time.
        git_fixture(root, &["switch", "main"])?;
        fs::write(root.join("shared.toml"), "later source change\n")?;
        git_fixture(root, &["add", "-A"])?;
        git_fixture(root, &["commit", "-m", "fix: later source change"])?;
        let source_head = git_fixture(root, &["rev-parse", "HEAD"])?;

        // A receipt that legitimately reconciled shared.toml previously.
        let mut authority = super::super::transition::TransitionAuthority::default();
        authority.two_sided.insert(
            "shared.toml".to_string(),
            super::super::transition::ResolvedReceipt {
                source_pr: "EffortlessMetrics/shiplog#657".to_string(),
                swarm_chain: vec!["EffortlessMetrics/shiplog-swarm#269".to_string()],
                disposition: super::super::promotion_state::TransitionDisposition::Equivalent,
            },
        );

        let error = resolve_paths(root, &source_head, &swarm_sha, &base, &[], &authority)
            .expect_err("a stale receipt must not cover a later source-only change");
        ensure!(
            error.to_string().contains("would revert it"),
            "unexpected error: {error}"
        );
        Ok(())
    }

    /// The construction proof checks content, not just which names differ. A
    /// resolved path holding the wrong blob would pass a name-set comparison
    /// because its name is permitted to differ from swarm.
    #[test]
    fn construction_is_verified_by_blob_not_by_path_name() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = dir.path();
        git_fixture(root, &["init", "--initial-branch=main"])?;
        git_fixture(root, &["config", "user.email", "test@example.com"])?;
        git_fixture(root, &["config", "user.name", "Promotion Test"])?;
        fs::write(root.join("owned.toml"), "source version\n")?;
        fs::write(root.join("product.txt"), "base\n")?;
        git_fixture(root, &["add", "-A"])?;
        git_fixture(root, &["commit", "-m", "base"])?;
        let source_head = git_fixture(root, &["rev-parse", "HEAD"])?;

        git_fixture(root, &["switch", "-c", "swarm"])?;
        fs::write(root.join("owned.toml"), "swarm version\n")?;
        fs::write(root.join("product.txt"), "swarm\n")?;
        git_fixture(root, &["add", "-A"])?;
        git_fixture(root, &["commit", "-m", "feat: swarm changes both"])?;
        let swarm_sha = git_fixture(root, &["rev-parse", "HEAD"])?;

        // Resolve owned.toml to source; the overlay must hold source's exact blob
        // there and swarm's exact blob everywhere else.
        let overlay = prepare_source_overlay(
            &SystemPort,
            root,
            &source_head,
            &swarm_sha,
            &["owned.toml".to_string()],
            "1234567890abcdef1234567890abcdef12345678",
            false,
        )?;
        let owned = git_fixture(root, &["show", &format!("{}:owned.toml", overlay.sha)])?;
        ensure!(
            owned.trim() == "source version",
            "resolved path should hold the source blob, got {owned:?}"
        );
        let product = git_fixture(root, &["show", &format!("{}:product.txt", overlay.sha)])?;
        ensure!(
            product.trim() == "swarm",
            "unresolved path should hold the swarm blob, got {product:?}"
        );
        Ok(())
    }

    /// A file swarm deleted must not survive in the overlay. Construction used to
    /// copy the swarm tree over the source worktree, which never removed a path
    /// swarm had deleted, so the promotion silently dropped that deletion. Found
    /// by the post-construction invariant rather than by a test.
    #[test]
    fn overlay_honours_a_deletion_made_in_swarm() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = dir.path();
        git_fixture(root, &["init", "--initial-branch=main"])?;
        git_fixture(root, &["config", "user.email", "test@example.com"])?;
        git_fixture(root, &["config", "user.name", "Promotion Test"])?;
        fs::write(root.join("keep.txt"), "keep\n")?;
        fs::write(root.join("removed-by-swarm.txt"), "doomed\n")?;
        git_fixture(root, &["add", "-A"])?;
        git_fixture(root, &["commit", "-m", "base"])?;
        let source_head = git_fixture(root, &["rev-parse", "HEAD"])?;

        git_fixture(root, &["switch", "-c", "swarm"])?;
        git_fixture(root, &["rm", "--quiet", "removed-by-swarm.txt"])?;
        git_fixture(root, &["commit", "-m", "chore: swarm removes a file"])?;
        let swarm_sha = git_fixture(root, &["rev-parse", "HEAD"])?;

        let overlay = prepare_source_overlay(
            &SystemPort,
            root,
            &source_head,
            &swarm_sha,
            &[],
            "1234567890abcdef1234567890abcdef12345678",
            false,
        )?;
        let listed = git_fixture(root, &["ls-tree", "-r", "--name-only", &overlay.sha])?;
        ensure!(
            !listed.lines().any(|line| line == "removed-by-swarm.txt"),
            "overlay retained a file swarm deleted: {listed}"
        );
        ensure!(listed.lines().any(|line| line == "keep.txt"));
        Ok(())
    }

    /// A refused plan emits the unresolved decisions as JSON, so a caller gets an
    /// exact repair queue with the blobs and basis rather than prose to parse.
    #[test]
    fn a_refused_plan_emits_the_unresolved_decisions() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = dir.path();
        git_fixture(root, &["init", "--initial-branch=main"])?;
        git_fixture(root, &["config", "user.email", "test@example.com"])?;
        git_fixture(root, &["config", "user.name", "Promotion Test"])?;
        fs::write(root.join("product.txt"), "base\n")?;
        git_fixture(root, &["add", "-A"])?;
        git_fixture(root, &["commit", "-m", "base"])?;
        let base = git_fixture(root, &["rev-parse", "HEAD"])?;
        git_fixture(root, &["switch", "-c", "swarm"])?;
        fs::write(root.join("product.txt"), "swarm\n")?;
        git_fixture(root, &["add", "-A"])?;
        git_fixture(root, &["commit", "-m", "feat: swarm edit"])?;
        let swarm_sha = git_fixture(root, &["rev-parse", "HEAD"])?;
        git_fixture(root, &["switch", "main"])?;
        fs::write(root.join("product.txt"), "source\n")?;
        git_fixture(root, &["add", "-A"])?;
        git_fixture(root, &["commit", "-m", "fix: source edit"])?;
        let source_head = git_fixture(root, &["rev-parse", "HEAD"])?;

        let plan = plan_path_resolutions(
            &SystemPort,
            root,
            &source_head,
            &swarm_sha,
            &base,
            &[],
            &super::super::transition::TransitionAuthority::default(),
        )?;
        let mut emitted = Vec::new();
        let error = ensure_no_blocked_paths(&plan, &mut emitted)
            .expect_err("two-sided divergence with no receipt must be refused");
        ensure!(error.to_string().contains("both repositories changed"));

        let reported: serde_json::Value = serde_json::from_slice(&emitted)?;
        let decision = &reported["decisions"][0];
        ensure!(decision["path"] == "product.txt");
        ensure!(decision["effect"] == "block");
        ensure!(decision["basis"]["kind"] == "unproven-two-sided");
        ensure!(decision["source_changed"] == true && decision["swarm_changed"] == true);
        // The exact blobs bind the decision to the trees it was made against.
        ensure!(decision["source_blob"].is_string() && decision["swarm_blob"].is_string());
        ensure!(decision["source_blob"] != decision["swarm_blob"]);
        Ok(())
    }

    #[test]
    fn discard_source_resolution_selects_swarm_content_for_source_only_drift() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = dir.path();
        git_fixture(root, &["init", "--initial-branch=main"])?;
        git_fixture(root, &["config", "user.email", "test@example.com"])?;
        git_fixture(root, &["config", "user.name", "Promotion Test"])?;
        fs::write(root.join("governance.md"), "base\n")?;
        git_fixture(root, &["add", "-A"])?;
        git_fixture(root, &["commit", "-m", "base"])?;
        let base = git_fixture(root, &["rev-parse", "HEAD"])?;

        git_fixture(root, &["switch", "-c", "swarm"])?;
        let swarm_sha = git_fixture(root, &["rev-parse", "HEAD"])?;
        git_fixture(root, &["switch", "main"])?;
        fs::write(root.join("governance.md"), "source transition\n")?;
        git_fixture(root, &["add", "-A"])?;
        git_fixture(root, &["commit", "-m", "chore: source transition copy"])?;
        let source_head = git_fixture(root, &["rev-parse", "HEAD"])?;

        let mut authority = super::super::transition::TransitionAuthority::default();
        authority.discard_source.insert(
            "governance.md".to_string(),
            super::super::transition::DiscardSourceDecision {
                source_pr: "EffortlessMetrics/shiplog#674".to_string(),
                decision_receipt: "EffortlessMetrics/shiplog-swarm#242".to_string(),
                decision_merge_sha: "7d0a1b2c3d4e5f60718293a4b5c6d7e8f9a0b1c2".to_string(),
                reason: "Reviewed swarm state supersedes the source transition copy.".to_string(),
            },
        );

        let plan = plan_path_resolutions(
            &SystemPort,
            root,
            &source_head,
            &swarm_sha,
            &base,
            &[],
            &authority,
        )?;
        ensure!(plan.decisions.len() == 1);
        let decision = &plan.decisions[0];
        ensure!(decision.effect == PathEffect::TakeSwarm);
        ensure!(matches!(
            decision.basis,
            ResolutionBasis::DiscardedSource { .. }
        ));
        ensure_no_blocked_paths(&plan, &mut Vec::new())?;
        Ok(())
    }

    #[test]
    fn discard_source_resolution_selects_swarm_content_for_two_sided_drift() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = dir.path();
        git_fixture(root, &["init", "--initial-branch=main"])?;
        git_fixture(root, &["config", "user.email", "test@example.com"])?;
        git_fixture(root, &["config", "user.name", "Promotion Test"])?;
        fs::write(root.join("governance.md"), "base\n")?;
        git_fixture(root, &["add", "-A"])?;
        git_fixture(root, &["commit", "-m", "base"])?;
        let base = git_fixture(root, &["rev-parse", "HEAD"])?;

        git_fixture(root, &["switch", "-c", "swarm"])?;
        fs::write(root.join("governance.md"), "swarm product\n")?;
        git_fixture(root, &["add", "-A"])?;
        git_fixture(root, &["commit", "-m", "swarm product change"])?;
        let swarm_sha = git_fixture(root, &["rev-parse", "HEAD"])?;
        git_fixture(root, &["switch", "main"])?;
        fs::write(root.join("governance.md"), "source transition\n")?;
        git_fixture(root, &["add", "-A"])?;
        git_fixture(root, &["commit", "-m", "source transition copy"])?;
        let source_head = git_fixture(root, &["rev-parse", "HEAD"])?;

        let mut authority = super::super::transition::TransitionAuthority::default();
        authority.discard_source.insert(
            "governance.md".to_string(),
            super::super::transition::DiscardSourceDecision {
                source_pr: "EffortlessMetrics/shiplog#674".to_string(),
                decision_receipt: "EffortlessMetrics/shiplog-swarm#242".to_string(),
                decision_merge_sha: "7d0a1b2c3d4e5f60718293a4b5c6d7e8f9a0b1c2".to_string(),
                reason: "Reviewed swarm state supersedes the source transition copy.".to_string(),
            },
        );

        let plan = plan_path_resolutions(
            &SystemPort,
            root,
            &source_head,
            &swarm_sha,
            &base,
            &[],
            &authority,
        )?;
        ensure!(plan.decisions.len() == 1);
        let decision = &plan.decisions[0];
        ensure!(decision.effect == PathEffect::TakeSwarm);
        ensure!(decision.source_changed && decision.swarm_changed);
        ensure!(matches!(
            decision.basis,
            ResolutionBasis::DiscardedSource { .. }
        ));
        ensure_no_blocked_paths(&plan, &mut Vec::new())?;
        Ok(())
    }

    /// The overlay records the plan it was built from, so closeout can bind a
    /// landed overlay to the exact per-path decisions behind it.
    #[test]
    fn overlay_records_the_resolution_plan_it_was_built_from() -> Result<()> {
        let fixture = fixture_git()?;
        let root = fixture.dir.path();
        // Materialized for real: a planning run isolates its objects, so the sha
        // it reports is deliberately unresolvable afterwards.
        let overlay = prepare_source_overlay(
            &SystemPort,
            root,
            &fixture.governance,
            &fixture.current,
            &[],
            "1234567890abcdef1234567890abcdef12345678",
            false,
        )?;
        let message = git_fixture(root, &["show", "-s", "--format=%B", &overlay.sha])?;

        let recorded = overlay_trailer(&message, OVERLAY_PLAN_TRAILER)
            .context("overlay must record its resolution plan")?;
        ensure!(recorded == "1234567890abcdef1234567890abcdef12345678");
        // Trailers are unindented and separately readable.
        ensure!(
            message.contains(&format!("\n{OVERLAY_PLAN_TRAILER} {recorded}")),
            "plan trailer should sit at the start of its own line: {message:?}"
        );
        ensure!(
            overlay_trailer(&message, OVERLAY_SWARM_TRAILER).as_deref()
                == Some(fixture.current.as_str())
        );
        Ok(())
    }

    /// An unchanged source/swarm tree still needs a durable overlay checkpoint.
    /// The isolated planning commit and the later materialized commit must be
    /// identical so dry-run remains a faithful preview of execution.
    #[test]
    fn identical_tree_overlay_is_deterministic_and_reusable() -> Result<()> {
        let fixture = fixture_git()?;
        let root = fixture.dir.path();
        let plan_id = "1234567890abcdef1234567890abcdef12345678";

        let planned = prepare_source_overlay(
            &SystemPort,
            root,
            &fixture.governance,
            &fixture.governance,
            &[],
            plan_id,
            true,
        )?;
        let executed = prepare_source_overlay(
            &SystemPort,
            root,
            &fixture.governance,
            &fixture.governance,
            &[],
            plan_id,
            false,
        )?;
        ensure!(
            planned.sha == executed.sha,
            "dry-run and execution must materialize the same empty overlay"
        );
        ensure!(
            executed.sha != fixture.governance,
            "identical trees still require an overlay checkpoint"
        );
        let source_date = git_fixture(root, &["show", "-s", "--format=%cI", &fixture.governance])?;
        ensure!(
            git_fixture(root, &["show", "-s", "--format=%aI", &executed.sha])? == source_date,
            "overlay author date must derive from the exact source target"
        );
        ensure!(
            git_fixture(root, &["show", "-s", "--format=%cI", &executed.sha])? == source_date,
            "overlay committer date must derive from the exact source target"
        );
        ensure!(
            git_fixture(root, &["show", "-s", "--format=%P", &executed.sha])? == fixture.governance,
            "the empty overlay must have the source head as its only parent"
        );
        ensure!(
            git_fixture(
                root,
                &[
                    "diff-tree",
                    "--no-commit-id",
                    "--name-only",
                    "-r",
                    &executed.sha
                ],
            )?
            .is_empty(),
            "identical-tree overlay must be empty"
        );

        let message = git_fixture(root, &["show", "-s", "--format=%B", &executed.sha])?;
        ensure!(
            overlay_trailer(&message, OVERLAY_SOURCE_TRAILER).as_deref()
                == Some(fixture.governance.as_str())
        );
        ensure!(
            overlay_trailer(&message, OVERLAY_SWARM_TRAILER).as_deref()
                == Some(fixture.governance.as_str())
        );
        ensure!(overlay_trailer(&message, OVERLAY_PLAN_TRAILER).as_deref() == Some(plan_id));

        git_fixture(root, &["switch", "--detach", &fixture.governance])?;
        git_fixture(
            root,
            &[
                "merge",
                "--no-ff",
                "-m",
                "merge(swarm): promote identical tree",
                &executed.sha,
            ],
        )?;
        let landed = git_fixture(root, &["rev-parse", "HEAD"])?;

        fs::write(root.join("later-source.txt"), "later\n")?;
        git_fixture(root, &["add", "later-source.txt"])?;
        git_fixture(root, &["commit", "-m", "chore: source governance (#700)"])?;
        let later_source_head = git_fixture(root, &["rev-parse", "HEAD"])?;
        let governance = later_source_head.clone();

        let found = find_latest_promotion_merge(
            &SystemPort,
            root,
            &later_source_head,
            &fixture.governance,
            &BTreeSet::from([governance]),
        )?;
        ensure!(
            found == landed,
            "next planning run must recognize the identical-tree checkpoint"
        );
        Ok(())
    }

    #[test]
    fn overlay_checkpoint_rejects_extra_overlay_parent() -> Result<()> {
        let fixture = fixture_git()?;
        let root = fixture.dir.path();
        let tree = format!("{}^{{tree}}", fixture.governance);
        let message = format!(
            "chore(promote): malformed overlay\n\n{OVERLAY_SOURCE_TRAILER} {}\n{OVERLAY_SWARM_TRAILER} {}\n",
            fixture.governance, fixture.current
        );
        let overlay = git_fixture(
            root,
            &[
                "commit-tree",
                &tree,
                "-p",
                &fixture.governance,
                "-p",
                &fixture.promoted,
                "-m",
                &message,
            ],
        )?;

        git_fixture(root, &["switch", "--detach", &fixture.governance])?;
        git_fixture(
            root,
            &[
                "merge",
                "--no-ff",
                "-m",
                "merge: malformed overlay",
                &overlay,
            ],
        )?;
        let merged = git_fixture(root, &["rev-parse", "HEAD"])?;

        let error = find_regular_merge_landing(&SystemPort, root, &merged, &fixture.current)
            .expect_err("an overlay with extra parents must be rejected");
        ensure!(
            error.to_string().contains("must have exactly one parent"),
            "unexpected error: {error}"
        );
        Ok(())
    }

    #[test]
    fn overlay_treats_metacharacter_paths_literally() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = dir.path();
        git_fixture(root, &["init", "--initial-branch=main"])?;
        git_fixture(root, &["config", "user.email", "test@example.com"])?;
        git_fixture(root, &["config", "user.name", "Promotion Test"])?;

        fs::write(root.join("literal0.txt"), "base\n")?;
        git_fixture(root, &["add", "literal0.txt"])?;
        git_fixture(root, &["commit", "-m", "base"])?;
        git_fixture(root, &["switch", "-c", "swarm"])?;
        fs::write(root.join("literal0.txt"), "swarm\n")?;
        git_fixture(root, &["add", "literal0.txt"])?;
        git_fixture(root, &["commit", "-m", "swarm sibling"])?;
        let swarm_sha = git_fixture(root, &["rev-parse", "HEAD"])?;
        git_fixture(root, &["switch", "main"])?;
        fs::write(root.join("literal[0].txt"), "source\n")?;
        git_fixture(root, &["add", "literal[0].txt"])?;
        git_fixture(root, &["commit", "-m", "source exact path"])?;
        let source_head = git_fixture(root, &["rev-parse", "HEAD"])?;

        let overlay = prepare_source_overlay(
            &SystemPort,
            root,
            &source_head,
            &swarm_sha,
            &["literal[0].txt".to_string()],
            "1234567890abcdef1234567890abcdef12345678",
            false,
        )?;
        let exact_object = format!("{}:literal[0].txt", overlay.sha);
        let sibling_object = format!("{}:literal0.txt", overlay.sha);
        ensure!(
            git_fixture(root, &["show", &exact_object])? == "source",
            "literal policy path must restore the exact bracketed file"
        );
        ensure!(
            git_fixture(root, &["show", &sibling_object])? == "swarm",
            "glob-matching sibling must retain swarm content"
        );
        Ok(())
    }

    /// The mirror of the source-authoritative case. Outside the policy list the
    /// overlay keeps swarm content, so a path only source changed would be
    /// reverted. `missing_in_swarm` used to approve exactly this, discarding the
    /// source change the receipt existed to record.
    #[test]
    fn resolution_blocks_a_source_change_swarm_does_not_carry() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = dir.path();
        git_fixture(root, &["init", "--initial-branch=main"])?;
        git_fixture(root, &["config", "user.email", "test@example.com"])?;
        git_fixture(root, &["config", "user.name", "Promotion Test"])?;
        fs::write(root.join("product.txt"), "base\n")?;
        git_fixture(root, &["add", "-A"])?;
        git_fixture(root, &["commit", "-m", "base"])?;
        let base = git_fixture(root, &["rev-parse", "HEAD"])?;

        // Swarm moves on without touching product.txt.
        git_fixture(root, &["switch", "-c", "swarm"])?;
        fs::write(root.join("other.txt"), "swarm\n")?;
        git_fixture(root, &["add", "-A"])?;
        git_fixture(root, &["commit", "-m", "feat: unrelated swarm work"])?;
        let swarm_sha = git_fixture(root, &["rev-parse", "HEAD"])?;

        // Source alone changes product.txt.
        git_fixture(root, &["switch", "main"])?;
        fs::write(root.join("product.txt"), "source hotfix\n")?;
        git_fixture(root, &["add", "-A"])?;
        git_fixture(root, &["commit", "-m", "fix: source hotfix"])?;
        let source_head = git_fixture(root, &["rev-parse", "HEAD"])?;

        // With no receipt at all.
        let error = resolve_paths(
            root,
            &source_head,
            &swarm_sha,
            &base,
            &[],
            &super::super::transition::TransitionAuthority::default(),
        )
        .expect_err("a source-only change swarm lacks must block");
        ensure!(
            error.to_string().contains("would revert it"),
            "unexpected error: {error}"
        );

        // A `missing_in_swarm` receipt does not authorize it either; it only
        // makes the refusal name the receipt and say what to do.
        let mut authority = super::super::transition::TransitionAuthority::default();
        authority.awaiting_swarm.insert(
            "product.txt".to_string(),
            "EffortlessMetrics/shiplog#657".to_string(),
        );
        let error = resolve_paths(root, &source_head, &swarm_sha, &base, &[], &authority)
            .expect_err("missing_in_swarm must not authorize a revert");
        let message = error.to_string();
        ensure!(
            message.contains("does not carry EffortlessMetrics/shiplog#657's change"),
            "refusal should name the receipt: {message}"
        );
        ensure!(
            message.contains("port it to swarm"),
            "refusal should say what to do: {message}"
        );

        // Making the path source-authoritative is the resolution, and then the
        // overlay restores it rather than reverting it.
        let plan = resolve_paths(
            root,
            &source_head,
            &swarm_sha,
            &base,
            &["product.txt".to_string()],
            &authority,
        )?;
        ensure!(plan.take_source() == vec!["product.txt".to_string()]);
        Ok(())
    }

    /// Overlay construction resolves a policy-listed source-only path in
    /// source's favour, so a swarm change to one would be reverted while the
    /// promotion still claimed to carry that swarm commit. The alignment check
    /// previously examined only source-only and two-sided changes, so a
    /// swarm-only change to such a path passed silently.
    #[test]
    fn alignment_rejects_a_swarm_change_to_a_source_authoritative_path() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = dir.path();
        git_fixture(root, &["init", "--initial-branch=main"])?;
        git_fixture(root, &["config", "user.email", "test@example.com"])?;
        git_fixture(root, &["config", "user.name", "Promotion Test"])?;
        fs::write(root.join("owned-by-source.toml"), "source\n")?;
        git_fixture(root, &["add", "-A"])?;
        git_fixture(root, &["commit", "-m", "base"])?;
        let base = git_fixture(root, &["rev-parse", "HEAD"])?;

        // Swarm alone edits a path the policy marks source-authoritative.
        git_fixture(root, &["switch", "-c", "swarm"])?;
        fs::write(root.join("owned-by-source.toml"), "swarm edit\n")?;
        git_fixture(root, &["add", "-A"])?;
        git_fixture(
            root,
            &["commit", "-m", "feat: swarm edits source-owned path"],
        )?;
        let swarm_sha = git_fixture(root, &["rev-parse", "HEAD"])?;

        let error = resolve_paths(
            root,
            &base,
            &swarm_sha,
            &base,
            &["owned-by-source.toml".to_string()],
            &super::super::transition::TransitionAuthority::default(),
        )
        .expect_err("a swarm change to a source-authoritative path must be rejected");
        ensure!(
            error
                .to_string()
                .contains("swarm changed a source-authoritative path"),
            "unexpected error: {error}"
        );
        Ok(())
    }

    #[test]
    fn bounded_source_authority_keeps_source_after_reviewed_decision() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = dir.path();
        git_fixture(root, &["init", "--initial-branch=main"])?;
        git_fixture(root, &["config", "user.email", "test@example.com"])?;
        git_fixture(root, &["config", "user.name", "Promotion Test"])?;
        fs::write(root.join("owned-by-source.toml"), "source\n")?;
        git_fixture(root, &["add", "-A"])?;
        git_fixture(root, &["commit", "-m", "base"])?;
        let base = git_fixture(root, &["rev-parse", "HEAD"])?;

        git_fixture(root, &["switch", "-c", "swarm"])?;
        fs::write(root.join("owned-by-source.toml"), "swarm edit\n")?;
        git_fixture(root, &["add", "-A"])?;
        git_fixture(
            root,
            &["commit", "-m", "feat: swarm edits source-owned path"],
        )?;
        let swarm_sha = git_fixture(root, &["rev-parse", "HEAD"])?;

        let path = "owned-by-source.toml";
        let mut authority = super::super::transition::TransitionAuthority::default();
        authority.source_authority.insert(
            path.to_string(),
            super::super::promotion_state::SourceAuthorityDecision {
                path: path.to_string(),
                source_target: base.clone(),
                swarm_target: swarm_sha.clone(),
                decision_receipt: "EffortlessMetrics/shiplog-swarm#319".to_string(),
                decision_merge_sha: "7d0a1b2c3d4e5f60718293a4b5c6d7e8f9a0b1c2".to_string(),
                reason: "Source release automation remains authoritative.".to_string(),
                consumed_by: String::new(),
                source_tree_entry: None,
                swarm_tree_entry: None,
            },
        );

        let plan = resolve_paths(
            root,
            &base,
            &swarm_sha,
            &base,
            &[path.to_string()],
            &authority,
        )?;
        assert_eq!(plan.take_source(), vec![path.to_string()]);
        assert!(plan.blocked().is_empty());
        Ok(())
    }

    /// The defect that made overlay promotions a one-shot: the planner walks back
    /// to the previous checkpoint with its own matcher, which required the raw
    /// swarm head as second parent. After the first overlay promotion landed, the
    /// next planning run rejected the checkpoint the previous run had created.
    /// Both now use one parser.
    #[test]
    fn the_planner_recognises_a_checkpoint_the_previous_promotion_created() -> Result<()> {
        let fixture = fixture_git()?;
        let root = fixture.dir.path();
        let source_only_paths = load_source_only_paths(root)?;

        // First promotion: build the overlay and land it as a regular merge.
        let overlay = prepare_source_overlay(
            &SystemPort,
            root,
            &fixture.governance,
            &fixture.current,
            &source_only_paths,
            "1234567890abcdef1234567890abcdef12345678",
            false,
        )?;
        git_fixture(root, &["switch", "--detach", &fixture.governance])?;
        git_fixture(
            root,
            &[
                "merge",
                "--no-ff",
                "-m",
                "merge(swarm): promote through overlay",
                &overlay.sha,
            ],
        )?;
        let landed = git_fixture(root, &["rev-parse", "HEAD"])?;

        // The planner, walking back from a later source head, must accept it.
        fs::write(root.join("later-source.txt"), "later\n")?;
        git_fixture(root, &["add", "-A"])?;
        git_fixture(root, &["commit", "-m", "chore: source governance (#700)"])?;
        let later_source_head = git_fixture(root, &["rev-parse", "HEAD"])?;
        let governance = git_fixture(root, &["rev-parse", "HEAD"])?;

        let found = find_latest_promotion_merge(
            &SystemPort,
            root,
            &later_source_head,
            &fixture.current,
            &BTreeSet::from([governance]),
        )?;
        ensure!(
            found == landed,
            "planner must recognise the overlay checkpoint it created: expected {landed}, got {found}"
        );
        Ok(())
    }

    /// The overlay must prove it belongs to the checkpoint. Without the parent
    /// and source-head proof, any commit carrying a copied trailer would satisfy
    /// verification.
    #[test]
    fn a_copied_swarm_trailer_does_not_make_a_checkpoint() -> Result<()> {
        let fixture = fixture_git()?;
        let root = fixture.dir.path();

        // A commit unrelated to the checkpoint, carrying a copied trailer that
        // claims the swarm head but parented somewhere else entirely.
        git_fixture(root, &["switch", "--detach", &fixture.promoted])?;
        fs::write(root.join("impostor.txt"), "impostor\n")?;
        git_fixture(root, &["add", "-A"])?;
        let message = format!(
            "chore(promote): overlay source with swarm {}\n\n{OVERLAY_SOURCE_TRAILER} {}\n{OVERLAY_SWARM_TRAILER} {}\n",
            &fixture.current[..12],
            fixture.governance,
            fixture.current
        );
        git_fixture(root, &["commit", "--no-gpg-sign", "-m", &message])?;
        let impostor = git_fixture(root, &["rev-parse", "HEAD"])?;

        git_fixture(root, &["switch", "--detach", &fixture.governance])?;
        git_fixture(
            root,
            &["merge", "--no-ff", "-m", "merge: impostor", &impostor],
        )?;
        let merged = git_fixture(root, &["rev-parse", "HEAD"])?;

        let error = find_regular_merge_landing(&SystemPort, root, &merged, &fixture.current)
            .expect_err("a commit whose parent is not the checkpoint's must be rejected");
        let message = error.to_string();
        ensure!(
            message.contains("must have exactly one parent"),
            "unexpected error: {message}"
        );
        Ok(())
    }

    /// #278 could create valid overlay checkpoints before #279 added the
    /// resolution-plan trailer. Those checkpoints must remain verifiable after
    /// the planner learns to record the newer receipt correlation value.
    #[test]
    fn legacy_overlay_without_resolution_plan_remains_verifiable() -> Result<()> {
        let fixture = fixture_git()?;
        let root = fixture.dir.path();

        git_fixture(root, &["switch", "--detach", &fixture.governance])?;
        fs::write(root.join("legacy-overlay.txt"), "legacy\n")?;
        git_fixture(root, &["add", "legacy-overlay.txt"])?;
        let message = format!(
            "chore(promote): legacy overlay {}\n\n{OVERLAY_SOURCE_TRAILER} {}\n{OVERLAY_SWARM_TRAILER} {}\n",
            &fixture.current[..12],
            fixture.governance,
            fixture.current
        );
        git_fixture(root, &["commit", "--no-gpg-sign", "-m", &message])?;
        let overlay = git_fixture(root, &["rev-parse", "HEAD"])?;

        git_fixture(root, &["switch", "source"])?;
        git_fixture(
            root,
            &["merge", "--no-ff", "-m", "merge: legacy overlay", &overlay],
        )?;
        let landed = git_fixture(root, &["rev-parse", "HEAD"])?;

        let found = find_regular_merge_landing(&SystemPort, root, &landed, &fixture.current)?
            .context("legacy overlay checkpoint must remain verifiable")?;
        ensure!(found.shape == CheckpointShape::Overlay);
        ensure!(
            found.resolution_plan_id.is_none(),
            "legacy overlay must not invent a resolution plan id"
        );
        let port = stub_port(&fixture, true, true);
        let mut output = Vec::new();
        run_with_port_to(&port, verify_only_inputs(&fixture), &mut output)?;
        let receipt: serde_json::Value = serde_json::from_slice(&output)?;
        ensure!(receipt["checkpoint_shape"] == "overlay");
        ensure!(receipt["resolution_plan_id"].is_null());
        ensure!(receipt["checks"].as_array().is_some_and(|checks| {
            checks.iter().any(|check| {
                check.as_str().is_some_and(|text| {
                    text == "legacy overlay has no resolution-plan evidence; modern per-path decision proof was not claimed"
                })
            })
        }));
        Ok(())
    }

    /// Promotions push an overlay commit, so the checkpoint's second parent is
    /// the overlay rather than the swarm head. Closeout has to recognise that
    /// shape or every promotion this tool creates fails its own verification.
    #[test]
    fn verify_only_recognises_an_overlay_second_parent() -> Result<()> {
        let fixture = fixture_git()?;
        let root = fixture.dir.path();
        let source_only_paths = load_source_only_paths(root)?;

        // Build the overlay exactly as promotion does, then land it as a
        // regular merge the way the source promotion PR would.
        let overlay = prepare_source_overlay(
            &SystemPort,
            root,
            &fixture.governance,
            &fixture.current,
            &source_only_paths,
            "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
            false,
        )?;
        ensure!(
            overlay.sha != fixture.current,
            "overlay must be its own commit"
        );
        git_fixture(root, &["switch", "--detach", &fixture.governance])?;
        git_fixture(
            root,
            &[
                "merge",
                "--no-ff",
                "-m",
                "merge(swarm): promote through overlay",
                &overlay.sha,
            ],
        )?;
        let landed = git_fixture(root, &["rev-parse", "HEAD"])?;

        let found = find_regular_merge_landing(&SystemPort, root, &landed, &fixture.current)?
            .context("overlay-parented checkpoint must be recognised")?;
        ensure!(found.shape == CheckpointShape::Overlay);
        ensure!(
            found.merge_sha == landed,
            "expected {landed} to be the landing, got {}",
            found.merge_sha
        );
        ensure!(
            found.resolution_plan_id.as_deref()
                == Some("1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"),
            "SHA-256 resolution plan ids must remain verifiable"
        );

        // The overlay carries the identity that makes this possible.
        let message = git_fixture(root, &["show", "-s", "--format=%B", &overlay.sha])?;
        ensure!(
            overlay_trailer(&message, OVERLAY_SWARM_TRAILER).as_deref()
                == Some(fixture.current.as_str()),
            "overlay must record the swarm head it carries: {message}"
        );
        ensure!(
            overlay_trailer(&message, OVERLAY_SOURCE_TRAILER).as_deref()
                == Some(fixture.governance.as_str())
        );

        // An unrelated swarm head must not match that checkpoint.
        ensure!(
            find_regular_merge_landing(
                &SystemPort,
                root,
                &landed,
                "0000000000000000000000000000000000000000"
            )?
            .is_none(),
            "a checkpoint must only satisfy the swarm head it actually carries"
        );
        Ok(())
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
                Ok(b"[]".to_vec()),
            ])),
            gh_calls: RefCell::new(Vec::new()),
            git_mutations: RefCell::new(Vec::new()),
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
            output: PathBuf::from("target/source-of-truth/promotion-body.md"),
            allow_historical: false,
            verify_only: false,
        }
    }

    /// The overlay sha the command itself produces, learned from a throwaway
    /// planning run.
    ///
    /// Derived from the real pipeline rather than recomputed, so it cannot drift
    /// from what the command does: the overlay commit now also carries the
    /// resolution plan id, which depends on inputs only the command resolves.
    /// The run is read-only and asserts it left no residue. The returned sha is
    /// therefore not resolvable locally: planning isolates its objects, so this is
    /// an identifier to compare, not a commit to read.
    fn fixture_overlay_sha(fixture: &GitFixture) -> Result<String> {
        let port = stub_port(fixture, true, true);
        let mut output = Vec::new();
        run_with_port_to(&port, fixture_inputs(fixture), &mut output)?;
        let plan: serde_json::Value = serde_json::from_slice(&output)?;
        let overlay = plan["prepared_overlay_sha"]
            .as_str()
            .context("planning run did not report a prepared overlay")?
            .to_string();
        ensure!(
            overlay_children(fixture.dir.path())?.is_empty(),
            "overlay parent still holds worktrees after cleanup"
        );
        ensure!(
            port.git_mutations.borrow().is_empty(),
            "the probing run must stay read-only"
        );
        Ok(overlay)
    }

    /// Plan and refuse exactly as the command does, discarding the machine
    /// readable plan the refusal emits.
    fn resolve_paths(
        root: &Path,
        source_head: &str,
        swarm_sha: &str,
        merge_base: &str,
        source_only_paths: &[String],
        authority: &super::super::transition::TransitionAuthority,
    ) -> Result<OverlayPlan> {
        let plan = plan_path_resolutions(
            &SystemPort,
            root,
            source_head,
            swarm_sha,
            merge_base,
            source_only_paths,
            authority,
        )?;
        ensure_no_blocked_paths(&plan, &mut Vec::new())?;
        Ok(plan)
    }

    /// Overlay worktree directories currently present under the shared parent.
    fn overlay_children(workspace_root: &Path) -> Result<Vec<PathBuf>> {
        let parent = workspace_root.join("target").join("promotion-overlay");
        if !parent.exists() {
            return Ok(Vec::new());
        }
        let mut children = Vec::new();
        for entry in fs::read_dir(&parent)? {
            children.push(entry?.path());
        }
        children.sort();
        Ok(children)
    }

    fn replace_pr_list(port: &StubPort, prs: serde_json::Value) -> Result<()> {
        let mut responses = port.gh.borrow_mut();
        let _previous = responses
            .pop_back()
            .context("expected default PR-list response")?;
        responses.push_back(Ok(serde_json::to_vec(&prs)?));
        Ok(())
    }

    fn recorded_pr(receipt: &serde_json::Value) -> Result<serde_json::Value> {
        let pr = receipt["source_pr"].clone();
        ensure!(!pr.is_null());
        Ok(pr)
    }

    #[test]
    fn planner_accepts_current_source_governance_and_stays_read_only() -> Result<()> {
        let fixture = fixture_git()?;
        let port = stub_port(&fixture, true, true);
        let inputs = fixture_inputs(&fixture);
        let overlay = fixture_overlay_sha(&fixture)?;
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
                        "refspec": format!("{overlay}:refs/heads/{branch}"),
                        "current_target": null,
                        "disposition": "required"
                    },
                    {
                        "kind": "create-or-update-pull-request",
                        "repository": "EffortlessMetrics/shiplog",
                        "base": "main",
                        "head": branch,
                        "action": "create"
                    }
                ])
        );
        ensure!(overlay_children(fixture.dir.path())?.is_empty());
        ensure!(port.git_mutations.borrow().is_empty());
        ensure!(!port.gh_calls.borrow().iter().any(|call| {
            call.get(1)
                .is_some_and(|action| action == "create" || action == "edit")
        }));
        Ok(())
    }

    #[test]
    fn planner_allows_missing_source_only_paths_during_overlay() -> Result<()> {
        let fixture = fixture_git()?;
        let policy_path = fixture.dir.path().join("policy/source-only-paths.toml");
        let mut policy = fs::read_to_string(&policy_path)?;
        policy.push_str(
            "\n[[allow]]\npath = \"not-present-in-source.txt\"\nowner = \"repo-infra/release\"\nreason = \"missing path should be ignored\"\nclassification = \"release-governance\"\ncreated = \"2026-07-23\"\nreview_after = \"2027-01-23\"\n",
        );
        fs::write(&policy_path, policy)?;

        let source_only_paths = load_source_only_paths(fixture.dir.path())?;
        ensure!(
            source_only_paths
                .iter()
                .any(|path| path == "not-present-in-source.txt")
        );
        let overlay = prepare_source_overlay(
            &SystemPort,
            fixture.dir.path(),
            &fixture.governance,
            &fixture.current,
            &source_only_paths,
            "1234567890abcdef1234567890abcdef12345678",
            false,
        )?;
        ensure!(overlay.cleanup_warnings.is_empty());
        ensure!(overlay_children(fixture.dir.path())?.is_empty());
        Ok(())
    }

    /// The overlay parent is shared with every other `target/` consumer, so
    /// preparing an overlay must not disturb unrelated build artifacts. An
    /// earlier implementation recursively removed `target/` on each run, which
    /// deleted dependency files out from under a concurrent Cargo build.
    #[test]
    fn overlay_preparation_preserves_unrelated_target_contents() -> Result<()> {
        let fixture = fixture_git()?;
        let root = fixture.dir.path();
        let artifact = root.join("target/debug/build-artifact.bin");
        fs::create_dir_all(artifact.parent().context("artifact parent")?)?;
        fs::write(&artifact, b"cargo output")?;

        let _overlay = fixture_overlay_sha(&fixture)?;

        ensure!(artifact.exists(), "overlay preparation deleted target/");
        ensure!(fs::read(&artifact)? == b"cargo output");
        ensure!(
            root.join("target/promotion-overlay").exists(),
            "shared overlay parent should survive cleanup"
        );
        Ok(())
    }

    /// Two overlay preparations must not be able to delete each other's active
    /// worktree. Holding one workspace open while a second is claimed and
    /// released proves the shared parent is never cleared wholesale.
    #[test]
    fn concurrent_overlay_workspaces_do_not_delete_each_other() -> Result<()> {
        let fixture = fixture_git()?;
        let root = fixture.dir.path();

        let first = OverlayWorkspace::claim(
            &SystemPort,
            root,
            &fixture.governance,
            &fixture.current,
            false,
        )?;
        let first_path = first.path().to_path_buf();

        let second = OverlayWorkspace::claim(
            &SystemPort,
            root,
            &fixture.governance,
            &fixture.current,
            false,
        )?;
        let second_path = second.path().to_path_buf();
        ensure!(first_path != second_path, "overlay paths must be unique");
        ensure!(
            first_path.exists(),
            "claiming a second overlay removed the first"
        );

        let warnings = second.release();
        ensure!(
            warnings.is_empty(),
            "unexpected cleanup residue: {warnings:?}"
        );
        ensure!(!second_path.exists());
        ensure!(
            first_path.exists(),
            "releasing the second overlay removed the first"
        );

        let warnings = first.release();
        ensure!(
            warnings.is_empty(),
            "unexpected cleanup residue: {warnings:?}"
        );
        ensure!(!first_path.exists());
        Ok(())
    }

    /// A registered worktree whose directory was removed by hand left the repo
    /// unusable before cleanup pruned registrations. Cleanup must reconcile it.
    #[test]
    fn overlay_cleanup_reconciles_stale_registered_worktree() -> Result<()> {
        let fixture = fixture_git()?;
        let root = fixture.dir.path();
        let stale = OverlayWorkspace::claim(
            &SystemPort,
            root,
            &fixture.governance,
            &fixture.current,
            false,
        )?;
        let stale_path = stale.path().to_path_buf();
        std::mem::forget(stale);
        fs::remove_dir_all(&stale_path)?;
        ensure!(
            git_fixture(root, &["worktree", "list", "--porcelain"])?.contains("prunable"),
            "fixture should present a prunable registration"
        );

        let _overlay = fixture_overlay_sha(&fixture)?;

        let listed = git_fixture(root, &["worktree", "list", "--porcelain"])?;
        ensure!(
            !listed.contains("prunable"),
            "cleanup left stale worktree registrations: {listed}"
        );
        Ok(())
    }

    /// A leftover directory git no longer tracks must not block a later
    /// promotion, and must not be adopted as this run's workspace.
    #[test]
    fn overlay_preparation_tolerates_stale_unregistered_directory() -> Result<()> {
        let fixture = fixture_git()?;
        let root = fixture.dir.path();
        let orphan = root
            .join("target/promotion-overlay")
            .join(format!("{}-0-orphan", &fixture.current[..12]));
        fs::create_dir_all(&orphan)?;
        fs::write(orphan.join("leftover.txt"), b"residue")?;

        let workspace = OverlayWorkspace::claim(
            &SystemPort,
            root,
            &fixture.governance,
            &fixture.current,
            false,
        )?;
        ensure!(
            workspace.path() != orphan,
            "claim adopted a stale directory instead of a fresh one"
        );
        let warnings = workspace.release();
        ensure!(
            warnings.is_empty(),
            "unexpected cleanup residue: {warnings:?}"
        );
        ensure!(
            orphan.exists(),
            "cleanup removed a directory this run did not own"
        );
        Ok(())
    }

    /// Failure after the worktree is registered must still deregister and
    /// remove it, or the next run inherits an unusable repository.
    #[test]
    fn overlay_preparation_cleans_up_after_failure() -> Result<()> {
        let fixture = fixture_git()?;
        let root = fixture.dir.path();
        let before = overlay_children(root)?;

        let source_only_paths = load_source_only_paths(root)?;
        let error = prepare_source_overlay(
            &SystemPort,
            root,
            &fixture.governance,
            "0000000000000000000000000000000000000000",
            &source_only_paths,
            "1234567890abcdef1234567890abcdef12345678",
            false,
        )
        .expect_err("overlay against a missing swarm sha should fail");
        ensure!(!error.to_string().is_empty());

        ensure!(
            overlay_children(root)? == before,
            "failed overlay left a workspace behind"
        );
        let listed = git_fixture(root, &["worktree", "list", "--porcelain"])?;
        ensure!(
            !listed.contains("prunable"),
            "failed overlay left a stale registration: {listed}"
        );
        Ok(())
    }

    /// Sequential preparations are idempotent: the same inputs yield the same
    /// overlay commit and leave no residue behind.
    #[test]
    fn repeated_overlay_preparation_is_idempotent() -> Result<()> {
        let fixture = fixture_git()?;
        let first = fixture_overlay_sha(&fixture)?;
        let second = fixture_overlay_sha(&fixture)?;
        let third = fixture_overlay_sha(&fixture)?;
        ensure!(
            first == second && second == third,
            "overlay sha drifted across runs: {first} {second} {third}"
        );
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
        let overlay = fixture_overlay_sha(&fixture)?;
        let mut port = stub_port(&fixture, true, true);
        port.remote_target = Some(overlay.clone());
        let mut output = Vec::new();
        run_with_port_to(&port, fixture_inputs(&fixture), &mut output)?;
        let plan: serde_json::Value = serde_json::from_slice(&output)?;
        let push = &plan["planned_mutations"][2];
        ensure!(push["current_target"] == serde_json::json!(overlay));
        ensure!(push["disposition"] == "already-current");
        ensure!(overlay_children(fixture.dir.path())?.is_empty());
        Ok(())
    }

    #[test]
    fn execution_creates_once_then_exact_rerun_is_a_noop() -> Result<()> {
        let fixture = fixture_git()?;
        let port = stub_port(&fixture, true, true);
        let overlay = fixture_overlay_sha(&fixture)?;
        port.gh.borrow_mut().push_back(Ok(
            b"https://github.com/EffortlessMetrics/shiplog/pull/700\n".to_vec(),
        ));
        let mut inputs = fixture_inputs(&fixture);
        inputs.dry_run = false;
        let mut output = Vec::new();
        run_with_port_to(&port, inputs, &mut output)?;
        ensure!(port.git_mutations.borrow().len() == 1);
        // The push must be leased against the target observed while planning, so
        // a branch moved by someone else in between is rejected, not overwritten.
        // Here no branch existed yet, so the lease asserts exactly that.
        let branch = format!("promote/swarm-current-{}", &fixture.current[..12]);
        ensure!(
            port.git_mutations.borrow()[0]
                == vec![
                    "push".to_string(),
                    format!("--force-with-lease=refs/heads/{branch}:"),
                    "origin".to_string(),
                    format!("{overlay}:refs/heads/{branch}"),
                ],
            "unexpected push invocation: {:?}",
            port.git_mutations.borrow()[0]
        );
        ensure!(
            port.gh_calls
                .borrow()
                .iter()
                .any(|call| { call.starts_with(&["pr".to_string(), "create".to_string()]) })
        );
        let receipt_path = fixture
            .dir
            .path()
            .join("target/source-of-truth/promote-receipt.json");
        let first: serde_json::Value = serde_json::from_str(&fs::read_to_string(&receipt_path)?)?;
        ensure!(first["source_pr"]["number"] == 700);
        ensure!(first["planned_mutations"][3]["action"] == "create");
        let promotion_body = fs::read_to_string(
            fixture
                .dir
                .path()
                .join("target/source-of-truth/promotion-body.md"),
        )?;
        ensure!(promotion_body.contains("## Rollback"));
        ensure!(promotion_body.contains("pause further promotions"));
        ensure!(promotion_body.contains("reconcile the source/swarm divergence"));
        ensure!(promotion_body.contains("This tool does not perform rollback"));

        let mut rerun = stub_port(&fixture, true, true);
        rerun.remote_target = Some(overlay);
        replace_pr_list(&rerun, serde_json::json!([recorded_pr(&first)?]))?;
        let mut inputs = fixture_inputs(&fixture);
        inputs.dry_run = false;
        run_with_port_to(&rerun, inputs, &mut Vec::new())?;
        ensure!(rerun.git_mutations.borrow().is_empty());
        ensure!(!rerun.gh_calls.borrow().iter().any(|call| {
            call.get(1)
                .is_some_and(|action| action == "create" || action == "edit")
        }));
        let second: serde_json::Value = serde_json::from_str(&fs::read_to_string(&receipt_path)?)?;
        ensure!(second["source_pr"]["number"] == 700);
        ensure!(second["planned_mutations"][2]["disposition"] == "already-current");
        ensure!(second["planned_mutations"][3]["action"] == "already-current");
        Ok(())
    }

    #[test]
    fn execution_updates_one_compatible_stale_pr() -> Result<()> {
        let fixture = fixture_git()?;
        let overlay = fixture_overlay_sha(&fixture)?;
        let mut port = stub_port(&fixture, true, true);
        port.remote_target = Some(overlay.clone());
        let branch = format!("promote/swarm-current-{}", &fixture.current[..12]);
        replace_pr_list(
            &port,
            serde_json::json!([{
                "number": 701,
                "url": "https://github.com/EffortlessMetrics/shiplog/pull/701",
                "headRefName": branch,
                "headRefOid": overlay,
                "baseRefName": "main",
                "headRepository": {"nameWithOwner": "EffortlessMetrics/shiplog"},
                "headRepositoryOwner": {"login": "EffortlessMetrics"},
                "title": "stale title",
                "body": "stale body"
            }]),
        )?;
        port.gh.borrow_mut().push_back(Ok(Vec::new()));
        let mut inputs = fixture_inputs(&fixture);
        inputs.dry_run = false;
        run_with_port_to(&port, inputs, &mut Vec::new())?;
        ensure!(port.git_mutations.borrow().is_empty());
        ensure!(port.gh_calls.borrow().iter().any(|call| {
            call.starts_with(&["pr".to_string(), "edit".to_string()])
                && call.get(2).is_some_and(|number| number == "701")
        }));
        let receipt: serde_json::Value = serde_json::from_str(&fs::read_to_string(
            fixture
                .dir
                .path()
                .join("target/source-of-truth/promote-receipt.json"),
        )?)?;
        ensure!(receipt["planned_mutations"][3]["action"] == "update");
        Ok(())
    }

    #[test]
    fn planner_rejects_duplicate_or_wrong_base_source_prs() -> Result<()> {
        let fixture = fixture_git()?;
        let overlay = fixture_overlay_sha(&fixture)?;
        let mut port = stub_port(&fixture, true, true);
        port.remote_target = Some(overlay.clone());
        let branch = format!("promote/swarm-current-{}", &fixture.current[..12]);
        let candidate = serde_json::json!({
            "number": 702,
            "url": "https://github.com/EffortlessMetrics/shiplog/pull/702",
            "headRefName": branch,
            "headRefOid": overlay.clone(),
            "baseRefName": "main",
            "headRepository": {"nameWithOwner": "EffortlessMetrics/shiplog"},
            "headRepositoryOwner": {"login": "EffortlessMetrics"},
            "title": "title",
            "body": "body"
        });
        replace_pr_list(&port, serde_json::json!([candidate.clone(), candidate]))?;
        let error = run_with_port(&port, fixture_inputs(&fixture))
            .err()
            .context("expected duplicate rejection")?;
        ensure!(error.to_string().contains("multiple open source PRs"));

        let fixture = fixture_git()?;
        let overlay = fixture_overlay_sha(&fixture)?;
        let mut port = stub_port(&fixture, true, true);
        port.remote_target = Some(overlay.clone());
        let branch = format!("promote/swarm-current-{}", &fixture.current[..12]);
        replace_pr_list(
            &port,
            serde_json::json!([{
                "number": 703,
                "url": "https://github.com/EffortlessMetrics/shiplog/pull/703",
                "headRefName": branch,
                "headRefOid": overlay,
                "baseRefName": "release",
                "headRepository": {"nameWithOwner": "EffortlessMetrics/shiplog"},
                "headRepositoryOwner": {"login": "EffortlessMetrics"},
                "title": "title",
                "body": "body"
            }]),
        )?;
        let error = run_with_port(&port, fixture_inputs(&fixture))
            .err()
            .context("expected base rejection")?;
        ensure!(error.to_string().contains("incompatible"));
        Ok(())
    }

    #[test]
    fn planner_rejects_fork_pr_with_matching_branch_base_and_oid() -> Result<()> {
        let fixture = fixture_git()?;
        let overlay = fixture_overlay_sha(&fixture)?;
        let mut port = stub_port(&fixture, true, true);
        port.remote_target = Some(overlay.clone());
        let branch = format!("promote/swarm-current-{}", &fixture.current[..12]);
        replace_pr_list(
            &port,
            serde_json::json!([{
                "number": 704,
                "url": "https://github.com/EffortlessMetrics/shiplog/pull/704",
                "headRefName": branch,
                "headRefOid": overlay,
                "baseRefName": "main",
                "headRepository": {"nameWithOwner": "fork-owner/shiplog"},
                "headRepositoryOwner": {"login": "fork-owner"},
                "title": "matching title is irrelevant",
                "body": "matching body is irrelevant"
            }]),
        )?;
        let error = run_with_port(&port, fixture_inputs(&fixture))
            .err()
            .context("expected fork identity rejection")?;
        ensure!(error.to_string().contains("incompatible"));
        ensure!(port.git_mutations.borrow().is_empty());
        ensure!(!port.gh_calls.borrow().iter().any(|call| {
            call.get(1)
                .is_some_and(|action| action == "create" || action == "edit")
        }));
        ensure!(overlay_children(fixture.dir.path())?.is_empty());
        Ok(())
    }

    #[test]
    fn planner_rejects_non_fast_forward_remote_without_mutation() -> Result<()> {
        let fixture = fixture_git()?;
        let mut port = stub_port(&fixture, true, true);
        port.remote_target = Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string());
        let mut responses = port.gh.borrow_mut();
        let _previous = responses.pop_back().context("expected PR-list response")?;
        responses.push_back(Ok(b"{\"status\":\"diverged\"}".to_vec()));
        drop(responses);
        let error = run_with_port(&port, fixture_inputs(&fixture))
            .err()
            .context("expected non-fast-forward rejection")?;
        ensure!(error.to_string().contains("not fast-forwardable"));
        // The comparison must name the existing remote target and the source
        // head. Comparing against the prepared overlay asks the source
        // repository about a local commit it has never received, which fails
        // outright rather than yielding a fast-forward decision.
        let expected = format!(
            "repos/EffortlessMetrics/shiplog/compare/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa...{}",
            fixture.governance
        );
        let compared = port
            .gh_calls
            .borrow()
            .iter()
            .filter_map(|call| call.get(1).cloned())
            .find(|path| path.starts_with("repos/EffortlessMetrics/shiplog/compare/"))
            .context("expected a compare call")?;
        ensure!(
            compared == expected,
            "compared {compared} but expected {expected}"
        );
        ensure!(port.git_mutations.borrow().is_empty());
        ensure!(overlay_children(fixture.dir.path())?.is_empty());
        Ok(())
    }

    #[test]
    fn planner_rejects_remote_head_absent_from_swarm_authority() -> Result<()> {
        let fixture = fixture_git()?;
        let mut port = stub_port(&fixture, true, true);
        port.remote_target = Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string());
        let mut responses = port.gh.borrow_mut();
        let _previous = responses.pop_back().context("expected PR-list response")?;
        responses.push_back(Err("HTTP 404 comparison commit not found".to_string()));
        drop(responses);
        let error = run_with_port(&port, fixture_inputs(&fixture))
            .err()
            .context("expected absent remote-head rejection")?;
        ensure!(error.to_string().contains("in swarm authority"));
        ensure!(port.git_mutations.borrow().is_empty());
        ensure!(overlay_children(fixture.dir.path())?.is_empty());
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
            git_mutations: RefCell::new(Vec::new()),
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
        let mut output = Vec::new();
        let error = run_with_port_to(&port, fixture_inputs(&fixture), &mut output)
            .err()
            .context("expected merge-base rejection")?;
        ensure!(error.to_string().contains("determine merge base"));
        ensure!(output.is_empty());
        ensure!(overlay_children(fixture.dir.path())?.is_empty());
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

    /// Every loose object, ref, and worktree registration in the repository,
    /// so a planning-only run can be proven not to have written to any of them.
    fn repository_state(root: &Path) -> Result<(Vec<String>, String, String)> {
        let mut objects = Vec::new();
        let objects_root = root.join(".git").join("objects");
        let mut stack = vec![objects_root.clone()];
        while let Some(dir) = stack.pop() {
            if !dir.exists() {
                continue;
            }
            for entry in fs::read_dir(&dir)? {
                let path = entry?.path();
                if path.is_dir() {
                    stack.push(path);
                } else {
                    objects.push(path.to_string_lossy().into_owned());
                }
            }
        }
        objects.sort();
        let refs = git_fixture(root, &["show-ref"]).unwrap_or_default();
        let worktrees = git_fixture(root, &["worktree", "list", "--porcelain"])?;
        Ok((objects, refs, worktrees))
    }

    /// `--dry-run` must mutate nothing. The overlay is a real commit, so building
    /// it in the repository wrote a tree, a commit, and blobs into
    /// `.git/objects` that cleanup could not undo. It is now built in a
    /// throwaway object store, and the reported sha still matches what an
    /// executing run materializes because the commit is fully deterministic.
    #[test]
    fn dry_run_reports_overlay_without_writing_to_the_object_database() -> Result<()> {
        let fixture = fixture_git()?;
        let root = fixture.dir.path();
        let source_only_paths = load_source_only_paths(root)?;

        let before = repository_state(root)?;
        let planned = prepare_source_overlay(
            &SystemPort,
            root,
            &fixture.governance,
            &fixture.current,
            &source_only_paths,
            "1234567890abcdef1234567890abcdef12345678",
            true,
        )?;
        let after = repository_state(root)?;

        ensure!(
            planned.cleanup_warnings.is_empty(),
            "cleanup residue: {:?}",
            planned.cleanup_warnings
        );
        ensure!(
            before.0 == after.0,
            "dry run wrote {} new object file(s) into .git/objects",
            after.0.len().saturating_sub(before.0.len())
        );
        ensure!(before.1 == after.1, "dry run changed refs");
        ensure!(
            before.2 == after.2,
            "dry run left worktree registrations changed:\n{}",
            after.2
        );
        ensure!(overlay_children(root)?.is_empty());
        // The planned sha is not resolvable, because its objects are gone.
        ensure!(
            git_fixture(root, &["cat-file", "-e", &planned.sha]).is_err(),
            "dry-run overlay {} survived in the object database",
            planned.sha
        );

        // Executing for real produces the same sha and keeps it resolvable.
        let executed = prepare_source_overlay(
            &SystemPort,
            root,
            &fixture.governance,
            &fixture.current,
            &source_only_paths,
            "1234567890abcdef1234567890abcdef12345678",
            false,
        )?;
        ensure!(
            executed.sha == planned.sha,
            "dry run reported {} but execution produced {}",
            planned.sha,
            executed.sha
        );
        git_fixture(root, &["cat-file", "-e", &executed.sha])?;
        Ok(())
    }

    /// A source-authoritative path must match source exactly, including when
    /// source deleted it. Restoring only when source still had the path let the
    /// swarm copy survive, and `ensure_source_only_alignment` approves that
    /// difference, so nothing downstream caught the reintroduction.
    #[test]
    fn overlay_honours_source_deletion_of_source_only_path() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let root = dir.path();
        git_fixture(root, &["init", "--initial-branch=main"])?;
        git_fixture(root, &["config", "user.email", "test@example.com"])?;
        git_fixture(root, &["config", "user.name", "Promotion Test"])?;
        fs::write(root.join("product.txt"), "v1\n")?;
        fs::write(root.join("retired.toml"), "retired\n")?;
        git_fixture(root, &["add", "-A"])?;
        git_fixture(root, &["commit", "-m", "base"])?;

        // Swarm still carries the retired path and moves the product forward.
        git_fixture(root, &["switch", "-c", "swarm"])?;
        fs::write(root.join("product.txt"), "v2\n")?;
        git_fixture(root, &["add", "-A"])?;
        git_fixture(root, &["commit", "-m", "feat: swarm change (#300)"])?;
        let swarm_sha = git_fixture(root, &["rev-parse", "HEAD"])?;

        // Source deliberately deletes the retired path.
        git_fixture(root, &["switch", "main"])?;
        git_fixture(root, &["rm", "--quiet", "retired.toml"])?;
        git_fixture(root, &["commit", "-m", "chore: retire source-only path"])?;
        let source_head = git_fixture(root, &["rev-parse", "HEAD"])?;

        let overlay = prepare_source_overlay(
            &SystemPort,
            root,
            &source_head,
            &swarm_sha,
            &["retired.toml".to_string()],
            "1234567890abcdef1234567890abcdef12345678",
            false,
        )?;
        ensure!(overlay.cleanup_warnings.is_empty());

        let listed = git_fixture(root, &["ls-tree", "-r", "--name-only", &overlay.sha])?;
        ensure!(
            !listed.lines().any(|line| line == "retired.toml"),
            "overlay reintroduced a path source deleted: {listed}"
        );
        let product = git_fixture(root, &["show", &format!("{}:product.txt", overlay.sha)])?;
        ensure!(
            product.trim() == "v2",
            "overlay lost swarm product content: {product:?}"
        );
        Ok(())
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
            prepared_overlay_sha: "abcdeffedcba9876543210fedcba1234567890abcd".to_string(),
            path_decisions: Vec::new(),
            overlay_source_paths: vec!["policy/automation-authority.toml".to_string()],
            overlay_swarm_path_count: 3,
            overlay_cleanup_warnings: Vec::new(),
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
