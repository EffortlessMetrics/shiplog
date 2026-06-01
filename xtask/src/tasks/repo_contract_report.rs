//! `rtk cargo xtask repo-contract-report`
//!
//! Writes a compact repo-contract inspection report for humans and agents.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::Command;

const REPORT_COMMAND: &str = "rtk cargo xtask repo-contract-report";
const SWARM_REPO: &str = "EffortlessMetrics/shiplog-swarm";
const SOURCE_REPO: &str = "EffortlessMetrics/shiplog";
const SWARM_BRANCH: &str = "main";
const SOURCE_BRANCH: &str = "main";
const ROUTED_WORKFLOW: &str = "EM CI Routed Shiplog Rust";
const SWARM_REQUIRED_CHECK: &str = "Shiplog Rust Small Result";
const DISALLOWED_BRANCH_PROTECTION_CHECKS: &[&str] = &[
    "Route Shiplog Rust Small",
    "Shiplog Rust Small on CPX42",
    "Shiplog Rust Small on CX43",
    "Shiplog Rust Small on CX53",
    "Shiplog Rust Small on GitHub Hosted",
];

#[derive(Debug, Deserialize)]
struct DocArtifactsPolicy {
    #[serde(default)]
    artifact: Vec<Artifact>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Artifact {
    id: String,
    kind: String,
    path: String,
    status: String,
    owner: String,
    #[serde(default)]
    milestone: Option<String>,
    #[serde(default)]
    linked_proposal: Option<String>,
    #[serde(default)]
    linked_spec: Option<String>,
    #[serde(default)]
    linked_specs: Vec<String>,
    #[serde(default)]
    linked_adr: Option<String>,
    #[serde(default)]
    linked_adrs: Vec<String>,
    #[serde(default)]
    linked_plan: Option<String>,
    #[serde(default)]
    policy_impact: Vec<String>,
    #[serde(default)]
    superseded_by: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ActiveGoal {
    id: String,
    title: String,
    status: String,
    owner: String,
    created: String,
    objective: String,
    #[serde(default)]
    end_state: Vec<String>,
    #[serde(default)]
    work_item: Vec<WorkItem>,
}

#[derive(Debug, Deserialize, Serialize)]
struct WorkItem {
    id: String,
    status: String,
    #[serde(default)]
    proposal: Option<String>,
    #[serde(default)]
    spec: Option<String>,
    #[serde(default)]
    adr: Option<String>,
    plan: String,
    #[serde(default)]
    commands: Vec<String>,
    #[serde(default)]
    receipts: Vec<String>,
    #[serde(default)]
    blocked_by: Option<toml::Value>,
    #[serde(default)]
    superseded_by: Option<String>,
}

#[derive(Debug, Serialize)]
struct ActiveGoalReport {
    id: String,
    title: String,
    status: String,
    owner: String,
    created: String,
    objective: String,
    end_state: Vec<String>,
    active_work_items: Vec<String>,
    ready_work_items: Vec<String>,
}

#[derive(Debug, Serialize)]
struct SupportTierClaim {
    surface: String,
    tier: String,
    claim: String,
    proof_command: String,
    notes: String,
}

#[derive(Debug, Serialize)]
struct GraphEdge {
    from: String,
    to: String,
    relation: String,
}

#[derive(Debug, Serialize)]
struct RepoContractReport {
    schema_version: u32,
    generated_by: String,
    outputs: Vec<String>,
    active_goal: ActiveGoalReport,
    git_topology: GitTopologyReport,
    local_checkout: LocalCheckoutReport,
    remote_branch_hygiene: RemoteBranchHygieneReport,
    remote_queue_hygiene: RemoteQueueHygieneReport,
    routed_ci_health: RoutedCiHealthReport,
    promotion_pr_contract: PromotionPrContractReport,
    branch_protection_contract: BranchProtectionContractReport,
    receipt_freshness: ReceiptFreshnessReport,
    artifacts: Vec<Artifact>,
    work_items: Vec<WorkItem>,
    support_tiers: Vec<SupportTierClaim>,
    edges: Vec<GraphEdge>,
}

struct RepoInspections {
    git_topology: GitTopologyReport,
    local_checkout: LocalCheckoutReport,
    remote_branch_hygiene: RemoteBranchHygieneReport,
    remote_queue_hygiene: RemoteQueueHygieneReport,
    routed_ci_health: RoutedCiHealthReport,
    promotion_pr_contract: PromotionPrContractReport,
    branch_protection_contract: BranchProtectionContractReport,
    receipt_freshness: ReceiptFreshnessReport,
}

#[derive(Debug, Serialize)]
struct GitTopologyReport {
    source_ref: String,
    swarm_ref: String,
    source_head: Option<String>,
    swarm_head: Option<String>,
    merge_base: Option<String>,
    trees_aligned: Option<bool>,
    source_ahead: Vec<String>,
    source_ahead_classification: String,
    source_ahead_promotion_merges: Vec<String>,
    source_ahead_other_commits: Vec<String>,
    swarm_ahead: Vec<String>,
    status: String,
    notes: Vec<String>,
    next_actions: Vec<String>,
}

#[derive(Debug, Serialize)]
struct LocalCheckoutReport {
    branch_summary: Option<String>,
    clean: Option<bool>,
    status_entries: Vec<String>,
    local_branch_count: usize,
    local_merged_cleanup_candidates: Vec<String>,
    local_merged_cleanup_review_commands: Vec<String>,
    protected_local_branches: Vec<String>,
    status: String,
    notes: Vec<String>,
    next_actions: Vec<String>,
}

#[derive(Debug, Serialize)]
struct RemoteBranchHygieneReport {
    status: String,
    source_remote: String,
    swarm_remote: String,
    source_cleanup_candidates: Vec<String>,
    swarm_cleanup_candidates: Vec<String>,
    source_merged_cleanup_candidates: Vec<String>,
    source_review_cleanup_candidates: Vec<String>,
    swarm_merged_cleanup_candidates: Vec<String>,
    swarm_review_cleanup_candidates: Vec<String>,
    source_merged_cleanup_review_commands: Vec<String>,
    source_review_cleanup_review_commands: Vec<String>,
    swarm_merged_cleanup_review_commands: Vec<String>,
    swarm_review_cleanup_review_commands: Vec<String>,
    source_open_pr_branches: Vec<String>,
    swarm_open_pr_branches: Vec<String>,
    protected_branches: Vec<String>,
    notes: Vec<String>,
    next_actions: Vec<String>,
}

#[derive(Debug, Serialize)]
struct RemoteQueueHygieneReport {
    status: String,
    source_repo: String,
    swarm_repo: String,
    source_open_prs: Vec<String>,
    swarm_open_prs: Vec<String>,
    source_open_issues: Vec<String>,
    swarm_open_issues: Vec<String>,
    notes: Vec<String>,
    next_actions: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct RoutedCiRunReport {
    id: Option<u64>,
    status: String,
    conclusion: String,
    display_title: String,
    event: String,
    head_branch: String,
    created_at: String,
    updated_at: String,
    url: String,
    success: bool,
}

#[derive(Debug, Serialize)]
struct RoutedCiHealthReport {
    status: String,
    workflow: String,
    source_repo: String,
    source_branch: String,
    source_latest_run: Option<RoutedCiRunReport>,
    swarm_repo: String,
    swarm_branch: String,
    swarm_latest_run: Option<RoutedCiRunReport>,
    notes: Vec<String>,
    next_actions: Vec<String>,
}

#[derive(Debug, Serialize)]
struct PromotionPrContractReport {
    status: String,
    source_repo: String,
    latest_promotion_pr: Option<String>,
    latest_promotion_url: Option<String>,
    expected_title: Option<String>,
    actual_title: Option<String>,
    state: Option<String>,
    state_ok: Option<bool>,
    merge_commit: Option<String>,
    expected_source_head: Option<String>,
    merge_commit_ok: Option<bool>,
    expected_swarm_head: Option<String>,
    title_ok: Option<bool>,
    body_mentions_swarm_head: Option<bool>,
    body_mentions_included_swarm_prs: Option<bool>,
    body_mentions_swarm_proof: Option<bool>,
    body_mentions_merge_method: Option<bool>,
    failed_checks: Vec<String>,
    notes: Vec<String>,
    next_actions: Vec<String>,
}

#[derive(Debug, Serialize)]
struct BranchProtectionContractReport {
    status: String,
    repo: String,
    branch: String,
    expected_required_status_checks: Vec<String>,
    actual_required_status_checks: Vec<String>,
    required_status_checks_ok: Option<bool>,
    strict_required_status_checks: Option<bool>,
    strict_required_status_checks_ok: Option<bool>,
    enforce_admins: Option<bool>,
    allow_force_pushes: Option<bool>,
    allow_force_pushes_ok: Option<bool>,
    allow_deletions: Option<bool>,
    allow_deletions_ok: Option<bool>,
    disallowed_required_checks: Vec<String>,
    notes: Vec<String>,
    next_actions: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ReceiptFreshnessReport {
    status: String,
    latest_source_promotion_merge: Option<String>,
    latest_source_promotion_receipt: Option<String>,
    latest_source_receipt_in_active_goal: Option<bool>,
    latest_source_receipt_in_plan: Option<bool>,
    latest_swarm_head: Option<String>,
    latest_swarm_receipt: Option<String>,
    latest_swarm_receipt_in_active_goal: Option<bool>,
    latest_swarm_receipt_in_plan: Option<bool>,
    missing_active_goal_receipts: Vec<String>,
    missing_plan_receipts: Vec<String>,
    notes: Vec<String>,
    next_actions: Vec<String>,
}

pub fn run(workspace_root: &Path) -> Result<()> {
    let artifacts = load_doc_artifacts(workspace_root)?;
    let goal = load_active_goal(workspace_root)?;
    let support_tiers = load_support_tiers(workspace_root)?;
    let git_topology = inspect_git_topology(workspace_root);
    let local_checkout = inspect_local_checkout(workspace_root);
    let remote_queue_hygiene = inspect_remote_queue_hygiene(workspace_root);
    let remote_branch_hygiene =
        inspect_remote_branch_hygiene(workspace_root, &remote_queue_hygiene);
    let routed_ci_health = inspect_routed_ci_health(workspace_root);
    let promotion_pr_contract = inspect_promotion_pr_contract(workspace_root, &git_topology);
    let branch_protection_contract = inspect_branch_protection_contract(workspace_root);
    let receipt_freshness = inspect_receipt_freshness(workspace_root, &goal, &git_topology);

    let output_dir = workspace_root.join("target").join("source-of-truth");
    fs::create_dir_all(&output_dir).with_context(|| format!("create {}", output_dir.display()))?;

    let graph_json = output_dir.join("graph.json");
    let graph_md = output_dir.join("graph.md");

    let inspections = RepoInspections {
        git_topology,
        local_checkout,
        remote_branch_hygiene,
        remote_queue_hygiene,
        routed_ci_health,
        promotion_pr_contract,
        branch_protection_contract,
        receipt_freshness,
    };
    let report = build_report(
        &artifacts,
        goal,
        support_tiers,
        inspections,
        vec![
            display_path(workspace_root, &graph_json),
            display_path(workspace_root, &graph_md),
        ],
    );

    let json = serde_json::to_string_pretty(&report).context("serialize repo contract report")?;
    fs::write(&graph_json, format!("{json}\n"))
        .with_context(|| format!("write {}", graph_json.display()))?;
    fs::write(&graph_md, render_markdown(&report))
        .with_context(|| format!("write {}", graph_md.display()))?;

    println!(
        "repo-contract-report: wrote report artifacts {} and {}",
        display_path(workspace_root, &graph_json),
        display_path(workspace_root, &graph_md)
    );

    Ok(())
}

fn load_doc_artifacts(workspace_root: &Path) -> Result<Vec<Artifact>> {
    let path = workspace_root.join("policy").join("doc-artifacts.toml");
    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let policy: DocArtifactsPolicy =
        toml::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    Ok(policy.artifact)
}

fn load_active_goal(workspace_root: &Path) -> Result<ActiveGoal> {
    let path = workspace_root
        .join(".codex")
        .join("goals")
        .join("active.toml");
    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("parse {}", path.display()))
}

fn load_support_tiers(workspace_root: &Path) -> Result<Vec<SupportTierClaim>> {
    let path = workspace_root
        .join("docs")
        .join("status")
        .join("SUPPORT_TIERS.md");
    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    Ok(parse_support_tier_claims(&text))
}

fn build_report(
    artifacts: &[Artifact],
    goal: ActiveGoal,
    support_tiers: Vec<SupportTierClaim>,
    inspections: RepoInspections,
    outputs: Vec<String>,
) -> RepoContractReport {
    let active_work_items = goal
        .work_item
        .iter()
        .filter(|item| item.status == "active")
        .map(|item| item.id.clone())
        .collect();
    let ready_work_items = goal
        .work_item
        .iter()
        .filter(|item| item.status == "ready")
        .map(|item| item.id.clone())
        .collect();

    let mut edges = Vec::new();
    for artifact in artifacts {
        push_artifact_edges(artifact, &mut edges);
    }
    for item in &goal.work_item {
        push_work_item_edges(item, &mut edges);
    }
    for claim in &support_tiers {
        if !claim.proof_command.trim().is_empty() {
            edges.push(GraphEdge {
                from: format!("support-tier:{}", claim.surface),
                to: claim.proof_command.clone(),
                relation: "proved_by".to_string(),
            });
        }
    }

    RepoContractReport {
        schema_version: 1,
        generated_by: REPORT_COMMAND.to_string(),
        outputs,
        active_goal: ActiveGoalReport {
            id: goal.id,
            title: goal.title,
            status: goal.status,
            owner: goal.owner,
            created: goal.created,
            objective: goal.objective,
            end_state: goal.end_state,
            active_work_items,
            ready_work_items,
        },
        git_topology: inspections.git_topology,
        local_checkout: inspections.local_checkout,
        remote_branch_hygiene: inspections.remote_branch_hygiene,
        remote_queue_hygiene: inspections.remote_queue_hygiene,
        routed_ci_health: inspections.routed_ci_health,
        promotion_pr_contract: inspections.promotion_pr_contract,
        branch_protection_contract: inspections.branch_protection_contract,
        receipt_freshness: inspections.receipt_freshness,
        artifacts: artifacts.to_vec(),
        work_items: goal.work_item,
        support_tiers,
        edges,
    }
}

fn inspect_git_topology(workspace_root: &Path) -> GitTopologyReport {
    const SOURCE_REF: &str = "origin/main";
    const SWARM_REF: &str = "swarm/main";

    let mut notes = Vec::new();
    let source_head = git_line(workspace_root, &["rev-parse", SOURCE_REF], &mut notes);
    let swarm_head = git_line(workspace_root, &["rev-parse", SWARM_REF], &mut notes);
    let merge_base = git_line(
        workspace_root,
        &["merge-base", SOURCE_REF, SWARM_REF],
        &mut notes,
    );
    let source_ahead = git_lines(
        workspace_root,
        &["log", "--oneline", &format!("{SWARM_REF}..{SOURCE_REF}")],
        &mut notes,
    );
    let swarm_ahead = git_lines(
        workspace_root,
        &["log", "--oneline", &format!("{SOURCE_REF}..{SWARM_REF}")],
        &mut notes,
    );
    let source_ahead_summary = classify_source_ahead(&source_ahead);
    let trees_aligned = git_trees_aligned(workspace_root, SOURCE_REF, SWARM_REF, &mut notes);
    let status = match (
        source_head.as_ref(),
        swarm_head.as_ref(),
        merge_base.as_ref(),
        trees_aligned,
        source_ahead.is_empty(),
        swarm_ahead.is_empty(),
    ) {
        (Some(source), Some(swarm), Some(_), Some(true), true, true) if source == swarm => {
            "identical".to_string()
        }
        (Some(_), Some(_), Some(_), Some(true), _, _) => "tree-aligned".to_string(),
        (Some(_), Some(_), Some(_), Some(false), false, false) => "diverged".to_string(),
        (Some(_), Some(_), Some(_), Some(false), true, false) => "swarm-ahead".to_string(),
        (Some(_), Some(_), Some(_), Some(false), false, true) => "source-ahead".to_string(),
        (Some(_), Some(_), Some(_), Some(false), true, true) => "tree-drift".to_string(),
        _ => "unavailable".to_string(),
    };

    let next_actions = topology_next_actions(
        &status,
        trees_aligned,
        &source_ahead_summary.classification,
        &source_ahead_summary.other_commits,
        &swarm_ahead,
    );

    GitTopologyReport {
        source_ref: SOURCE_REF.to_string(),
        swarm_ref: SWARM_REF.to_string(),
        source_head,
        swarm_head,
        merge_base,
        trees_aligned,
        source_ahead,
        source_ahead_classification: source_ahead_summary.classification,
        source_ahead_promotion_merges: source_ahead_summary.promotion_merges,
        source_ahead_other_commits: source_ahead_summary.other_commits,
        swarm_ahead,
        status,
        notes,
        next_actions,
    }
}

fn inspect_local_checkout(workspace_root: &Path) -> LocalCheckoutReport {
    let mut notes = Vec::new();
    let status_lines = git_lines(
        workspace_root,
        &["status", "--short", "--branch"],
        &mut notes,
    );
    let local_branch_lines = git_lines(
        workspace_root,
        &["branch", "--format=%(refname:short)"],
        &mut notes,
    );
    let source_merged_lines = git_lines(
        workspace_root,
        &[
            "branch",
            "--merged",
            "origin/main",
            "--format=%(refname:short)",
        ],
        &mut notes,
    );
    let swarm_merged_lines = git_lines(
        workspace_root,
        &[
            "branch",
            "--merged",
            "swarm/main",
            "--format=%(refname:short)",
        ],
        &mut notes,
    );
    local_checkout_from_status_and_branch_lines(
        status_lines,
        local_branch_lines,
        source_merged_lines,
        swarm_merged_lines,
        notes,
    )
}

#[cfg(test)]
fn local_checkout_from_status_lines(lines: Vec<String>, notes: Vec<String>) -> LocalCheckoutReport {
    local_checkout_from_status_and_branch_lines(lines, Vec::new(), Vec::new(), Vec::new(), notes)
}

fn local_checkout_from_status_and_branch_lines(
    lines: Vec<String>,
    local_branch_lines: Vec<String>,
    source_merged_lines: Vec<String>,
    swarm_merged_lines: Vec<String>,
    notes: Vec<String>,
) -> LocalCheckoutReport {
    let branch_summary = lines
        .first()
        .and_then(|line| line.strip_prefix("## "))
        .map(ToOwned::to_owned);
    let status_entries = lines
        .into_iter()
        .skip(usize::from(branch_summary.is_some()))
        .collect::<Vec<_>>();
    let clean = branch_summary
        .as_ref()
        .map(|_| status_entries.iter().all(|entry| entry.trim().is_empty()));
    let local_branch_count = local_branch_lines.len();
    let current_branch = current_branch_name(branch_summary.as_deref());
    let source_merged = local_branch_set(source_merged_lines);
    let swarm_merged = local_branch_set(swarm_merged_lines);
    let mut protected_local_branches = Vec::new();
    let mut local_merged_cleanup_candidates = Vec::new();

    for branch in local_branch_lines {
        if is_protected_local_branch(&branch) {
            protected_local_branches.push(branch);
        } else if Some(branch.as_str()) == current_branch.as_deref() {
            continue;
        } else if source_merged.contains(&branch) || swarm_merged.contains(&branch) {
            local_merged_cleanup_candidates.push(branch);
        }
    }

    protected_local_branches.sort();
    local_merged_cleanup_candidates.sort();
    let local_merged_cleanup_review_commands =
        local_merged_cleanup_review_commands(&local_merged_cleanup_candidates);
    let status = match (clean, local_merged_cleanup_candidates.is_empty()) {
        (Some(true), true) => "clean",
        (Some(true), false) => "review-needed",
        (Some(false), _) => "dirty",
        (None, _) => "unavailable",
    }
    .to_string();
    let next_actions = local_checkout_next_actions(clean, local_merged_cleanup_candidates.len());

    LocalCheckoutReport {
        branch_summary,
        clean,
        status_entries,
        local_branch_count,
        local_merged_cleanup_candidates,
        local_merged_cleanup_review_commands,
        protected_local_branches,
        status,
        notes,
        next_actions,
    }
}

fn current_branch_name(branch_summary: Option<&str>) -> Option<String> {
    let summary = branch_summary?;
    if summary.starts_with("HEAD ") || summary == "HEAD" {
        return None;
    }
    Some(summary.split("...").next().unwrap_or(summary).to_string())
}

fn local_branch_set(lines: Vec<String>) -> BTreeSet<String> {
    lines
        .into_iter()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect()
}

fn is_protected_local_branch(branch: &str) -> bool {
    branch == "main" || branch.starts_with("release/")
}

fn local_merged_cleanup_review_commands(candidates: &[String]) -> Vec<String> {
    candidates
        .iter()
        .map(|branch| {
            format!(
                "rtk gh pr list --repo {SWARM_REPO} --state all --head {branch} --limit 10 && rtk gh pr list --repo {SOURCE_REPO} --state all --head {branch} --limit 10 && rtk git log --oneline --max-count 3 {branch}"
            )
        })
        .collect()
}

fn local_checkout_next_actions(
    clean: Option<bool>,
    local_merged_cleanup_count: usize,
) -> Vec<String> {
    match clean {
        Some(true) if local_merged_cleanup_count == 0 => vec![
            "Local checkout is clean; continue with the active source-of-truth work item."
                .to_string(),
        ],
        Some(true) => vec![format!(
            "Review {local_merged_cleanup_count} local branch(es) already merged into source or swarm before deleting with `rtk git branch -d <branch>`."
        )],
        Some(false) => vec![
            "Inspect `rtk git status`, keep only scoped changes, and leave no dirty or untracked files before handoff."
                .to_string(),
        ],
        None => vec![
            "Run from a Git checkout with `origin` and `swarm` remotes available, then rerun `rtk cargo xtask repo-contract-report`."
                .to_string(),
        ],
    }
}

fn inspect_remote_branch_hygiene(
    workspace_root: &Path,
    remote_queue_hygiene: &RemoteQueueHygieneReport,
) -> RemoteBranchHygieneReport {
    const SOURCE_REMOTE: &str = "origin";
    const SWARM_REMOTE: &str = "swarm";
    const SOURCE_REF: &str = "origin/main";
    const SWARM_REF: &str = "swarm/main";

    let mut notes = Vec::new();
    let lines = git_lines(
        workspace_root,
        &["branch", "-r", "--format=%(refname:short)"],
        &mut notes,
    );
    let source_merged_lines = git_lines(
        workspace_root,
        &[
            "branch",
            "-r",
            "--merged",
            SOURCE_REF,
            "--format=%(refname:short)",
        ],
        &mut notes,
    );
    let swarm_merged_lines = git_lines(
        workspace_root,
        &[
            "branch",
            "-r",
            "--merged",
            SWARM_REF,
            "--format=%(refname:short)",
        ],
        &mut notes,
    );
    remote_branch_hygiene_from_lines_with_merged(
        lines,
        source_merged_lines,
        swarm_merged_lines,
        notes,
        SOURCE_REMOTE,
        SWARM_REMOTE,
        open_pr_branch_set(SOURCE_REMOTE, &remote_queue_hygiene.source_open_prs),
        open_pr_branch_set(SWARM_REMOTE, &remote_queue_hygiene.swarm_open_prs),
    )
}

#[cfg(test)]
fn remote_branch_hygiene_from_lines(
    lines: Vec<String>,
    notes: Vec<String>,
    source_remote: &str,
    swarm_remote: &str,
) -> RemoteBranchHygieneReport {
    remote_branch_hygiene_from_lines_with_merged(
        lines,
        Vec::new(),
        Vec::new(),
        notes,
        source_remote,
        swarm_remote,
        BTreeSet::new(),
        BTreeSet::new(),
    )
}

fn remote_branch_hygiene_from_lines_with_merged(
    lines: Vec<String>,
    source_merged_lines: Vec<String>,
    swarm_merged_lines: Vec<String>,
    notes: Vec<String>,
    source_remote: &str,
    swarm_remote: &str,
    source_open_pr_branches: BTreeSet<String>,
    swarm_open_pr_branches: BTreeSet<String>,
) -> RemoteBranchHygieneReport {
    let mut source_cleanup_candidates = Vec::new();
    let mut swarm_cleanup_candidates = Vec::new();
    let mut source_open_pr_branch_matches = Vec::new();
    let mut swarm_open_pr_branch_matches = Vec::new();
    let mut protected_branches = Vec::new();
    let source_merged = remote_branch_set(source_merged_lines);
    let swarm_merged = remote_branch_set(swarm_merged_lines);

    for line in lines {
        let branch = line.trim();
        if branch.is_empty() || branch == source_remote || branch == swarm_remote {
            continue;
        }
        if is_protected_remote_branch(branch, source_remote, swarm_remote) {
            protected_branches.push(branch.to_string());
        } else if source_open_pr_branches.contains(branch) {
            source_open_pr_branch_matches.push(branch.to_string());
        } else if swarm_open_pr_branches.contains(branch) {
            swarm_open_pr_branch_matches.push(branch.to_string());
        } else if branch.starts_with(&format!("{source_remote}/")) {
            source_cleanup_candidates.push(branch.to_string());
        } else if branch.starts_with(&format!("{swarm_remote}/")) {
            swarm_cleanup_candidates.push(branch.to_string());
        }
    }

    source_cleanup_candidates.sort();
    swarm_cleanup_candidates.sort();
    source_open_pr_branch_matches.sort();
    swarm_open_pr_branch_matches.sort();
    protected_branches.sort();
    let (source_merged_cleanup_candidates, source_review_cleanup_candidates) =
        partition_merged_candidates(&source_cleanup_candidates, &source_merged);
    let (swarm_merged_cleanup_candidates, swarm_review_cleanup_candidates) =
        partition_merged_candidates(&swarm_cleanup_candidates, &swarm_merged);
    let source_merged_cleanup_review_commands = remote_cleanup_review_commands(
        &source_merged_cleanup_candidates,
        source_remote,
        "EffortlessMetrics/shiplog",
    );
    let source_review_cleanup_review_commands = remote_cleanup_review_commands(
        &source_review_cleanup_candidates,
        source_remote,
        "EffortlessMetrics/shiplog",
    );
    let swarm_merged_cleanup_review_commands = remote_cleanup_review_commands(
        &swarm_merged_cleanup_candidates,
        swarm_remote,
        "EffortlessMetrics/shiplog-swarm",
    );
    let swarm_review_cleanup_review_commands = remote_cleanup_review_commands(
        &swarm_review_cleanup_candidates,
        swarm_remote,
        "EffortlessMetrics/shiplog-swarm",
    );

    let status = if !notes.is_empty() {
        "unavailable"
    } else if source_cleanup_candidates.is_empty() && swarm_cleanup_candidates.is_empty() {
        "clean"
    } else {
        "review-needed"
    }
    .to_string();
    let next_actions = remote_branch_hygiene_next_actions(
        &status,
        source_cleanup_candidates.len(),
        swarm_cleanup_candidates.len(),
        source_merged_cleanup_candidates.len(),
        source_review_cleanup_candidates.len(),
        swarm_merged_cleanup_candidates.len(),
        swarm_review_cleanup_candidates.len(),
    );

    RemoteBranchHygieneReport {
        status,
        source_remote: source_remote.to_string(),
        swarm_remote: swarm_remote.to_string(),
        source_cleanup_candidates,
        swarm_cleanup_candidates,
        source_merged_cleanup_candidates,
        source_review_cleanup_candidates,
        swarm_merged_cleanup_candidates,
        swarm_review_cleanup_candidates,
        source_merged_cleanup_review_commands,
        source_review_cleanup_review_commands,
        swarm_merged_cleanup_review_commands,
        swarm_review_cleanup_review_commands,
        source_open_pr_branches: source_open_pr_branch_matches,
        swarm_open_pr_branches: swarm_open_pr_branch_matches,
        protected_branches,
        notes,
        next_actions,
    }
}

fn remote_cleanup_review_commands(candidates: &[String], remote: &str, repo: &str) -> Vec<String> {
    candidates
        .iter()
        .filter_map(|candidate| remote_branch_head(candidate, remote).map(|head| (candidate, head)))
        .map(|(candidate, head)| {
            format!(
                "rtk gh pr list --repo {repo} --state all --head {head} --limit 10 && rtk git log --oneline --max-count 3 {candidate}"
            )
        })
        .collect()
}

fn remote_branch_head<'a>(branch: &'a str, remote: &str) -> Option<&'a str> {
    branch.strip_prefix(&format!("{remote}/"))
}

fn remote_branch_set(lines: Vec<String>) -> BTreeSet<String> {
    lines
        .into_iter()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect()
}

fn open_pr_branch_set(remote: &str, prs: &[String]) -> BTreeSet<String> {
    prs.iter()
        .filter_map(|pr| formatted_pr_head(pr))
        .map(|head| format!("{remote}/{head}"))
        .collect()
}

fn formatted_pr_head(pr: &str) -> Option<&str> {
    let marker = "head=";
    let start = pr.find(marker)? + marker.len();
    let rest = &pr[start..];
    let end = rest
        .find(", base=")
        .or_else(|| rest.find(']'))
        .unwrap_or(rest.len());
    let head = rest[..end].trim();
    if head.is_empty() || head == "-" {
        None
    } else {
        Some(head)
    }
}

fn partition_merged_candidates(
    candidates: &[String],
    merged_branches: &BTreeSet<String>,
) -> (Vec<String>, Vec<String>) {
    candidates
        .iter()
        .cloned()
        .partition(|candidate| merged_branches.contains(candidate))
}

fn is_protected_remote_branch(branch: &str, source_remote: &str, swarm_remote: &str) -> bool {
    branch == format!("{source_remote}/main")
        || branch == format!("{swarm_remote}/main")
        || branch.starts_with(&format!("{source_remote}/release/"))
}

fn remote_branch_hygiene_next_actions(
    status: &str,
    source_count: usize,
    swarm_count: usize,
    source_merged_count: usize,
    source_review_count: usize,
    swarm_merged_count: usize,
    swarm_review_count: usize,
) -> Vec<String> {
    match status {
        "clean" => vec!["Remote branch hygiene is clean.".to_string()],
        "review-needed" => vec![format!(
            "Review {source_count} source and {swarm_count} swarm remote cleanup candidate(s); start with {source_merged_count} source and {swarm_merged_count} swarm candidate(s) already merged into their repo trunk, then inspect {source_review_count} source and {swarm_review_count} swarm unmerged candidate(s). Delete only after confirming no open PR, release need, or preserved follow-up value."
        )],
        _ => vec![
            "Run from a Git checkout with `origin` and `swarm` remotes available, then rerun `rtk cargo xtask repo-contract-report`."
                .to_string(),
        ],
    }
}

fn inspect_remote_queue_hygiene(workspace_root: &Path) -> RemoteQueueHygieneReport {
    let mut notes = Vec::new();

    if !workspace_root.join(".git").exists() {
        notes
            .push("not a Git checkout; live PR and issue queue inspection was skipped".to_string());
        return remote_queue_hygiene_from_parts(
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            notes,
        );
    }

    let source_open_prs = github_open_prs(SOURCE_REPO, &mut notes);
    let swarm_open_prs = github_open_prs(SWARM_REPO, &mut notes);
    let source_open_issues = github_open_issues(SOURCE_REPO, &mut notes);
    let swarm_open_issues = github_open_issues(SWARM_REPO, &mut notes);

    remote_queue_hygiene_from_parts(
        source_open_prs,
        swarm_open_prs,
        source_open_issues,
        swarm_open_issues,
        notes,
    )
}

fn remote_queue_hygiene_from_parts(
    source_open_prs: Vec<String>,
    swarm_open_prs: Vec<String>,
    source_open_issues: Vec<String>,
    swarm_open_issues: Vec<String>,
    notes: Vec<String>,
) -> RemoteQueueHygieneReport {
    let status = if !notes.is_empty() {
        "unavailable"
    } else if source_open_prs.is_empty()
        && swarm_open_prs.is_empty()
        && source_open_issues.is_empty()
        && swarm_open_issues.is_empty()
    {
        "clean"
    } else {
        "review-needed"
    }
    .to_string();
    let next_actions = remote_queue_hygiene_next_actions(
        &status,
        source_open_prs.len(),
        swarm_open_prs.len(),
        source_open_issues.len(),
        swarm_open_issues.len(),
    );

    RemoteQueueHygieneReport {
        status,
        source_repo: SOURCE_REPO.to_string(),
        swarm_repo: SWARM_REPO.to_string(),
        source_open_prs,
        swarm_open_prs,
        source_open_issues,
        swarm_open_issues,
        notes,
        next_actions,
    }
}

fn github_open_prs(repo: &str, notes: &mut Vec<String>) -> Vec<String> {
    let args = [
        "pr",
        "list",
        "--repo",
        repo,
        "--state",
        "open",
        "--limit",
        "100",
        "--json",
        "number,title,isDraft,headRefName,baseRefName",
    ];
    gh_json_array(&args, notes)
        .into_iter()
        .map(|value| format_github_pr(repo, &value))
        .collect()
}

fn github_open_issues(repo: &str, notes: &mut Vec<String>) -> Vec<String> {
    let args = [
        "issue",
        "list",
        "--repo",
        repo,
        "--state",
        "open",
        "--limit",
        "100",
        "--json",
        "number,title,labels",
    ];
    gh_json_array(&args, notes)
        .into_iter()
        .map(|value| format_github_issue(repo, &value))
        .collect()
}

fn gh_json_array(args: &[&str], notes: &mut Vec<String>) -> Vec<serde_json::Value> {
    match Command::new("gh").args(args).output() {
        Ok(output) if output.status.success() => {
            match serde_json::from_slice::<Vec<serde_json::Value>>(&output.stdout) {
                Ok(values) => values,
                Err(err) => {
                    notes.push(format!(
                        "gh {} returned invalid JSON: {err}",
                        args.join(" ")
                    ));
                    Vec::new()
                }
            }
        }
        Ok(output) => {
            let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
            notes.push(format!(
                "gh {} failed: {}",
                args.join(" "),
                if detail.is_empty() {
                    output.status.to_string()
                } else {
                    detail
                }
            ));
            Vec::new()
        }
        Err(err) => {
            notes.push(format!("gh {} failed: {err}", args.join(" ")));
            Vec::new()
        }
    }
}

fn gh_json_object(args: &[&str], notes: &mut Vec<String>) -> Option<serde_json::Value> {
    match Command::new("gh").args(args).output() {
        Ok(output) if output.status.success() => {
            match serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                Ok(value) => Some(value),
                Err(err) => {
                    notes.push(format!(
                        "gh {} returned invalid JSON: {err}",
                        args.join(" ")
                    ));
                    None
                }
            }
        }
        Ok(output) => {
            let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
            notes.push(format!(
                "gh {} failed: {}",
                args.join(" "),
                if detail.is_empty() {
                    output.status.to_string()
                } else {
                    detail
                }
            ));
            None
        }
        Err(err) => {
            notes.push(format!("gh {} failed: {err}", args.join(" ")));
            None
        }
    }
}

fn format_github_pr(repo: &str, value: &serde_json::Value) -> String {
    let number = json_u64(value, "number");
    let title = json_str(value, "title");
    let draft = value
        .get("isDraft")
        .and_then(serde_json::Value::as_bool)
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    let head = json_str(value, "headRefName");
    let base = json_str(value, "baseRefName");

    format!("{repo}#{number} {title} [draft={draft}, head={head}, base={base}]")
}

fn format_github_issue(repo: &str, value: &serde_json::Value) -> String {
    let number = json_u64(value, "number");
    let title = json_str(value, "title");
    let labels = value
        .get("labels")
        .and_then(serde_json::Value::as_array)
        .map(|labels| {
            labels
                .iter()
                .filter_map(|label| label.get("name"))
                .filter_map(serde_json::Value::as_str)
                .collect::<Vec<_>>()
                .join(",")
        })
        .filter(|labels| !labels.is_empty())
        .unwrap_or_else(|| "-".to_string());

    format!("{repo}#{number} {title} [labels={labels}]")
}

fn json_u64(value: &serde_json::Value, key: &str) -> String {
    value
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .map(|value| value.to_string())
        .unwrap_or_else(|| "?".to_string())
}

fn json_str(value: &serde_json::Value, key: &str) -> String {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("-")
        .to_string()
}

fn json_string_opt(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn json_pointer_string_opt(value: &serde_json::Value, pointer: &str) -> Option<String> {
    value
        .pointer(pointer)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn remote_queue_hygiene_next_actions(
    status: &str,
    source_prs: usize,
    swarm_prs: usize,
    source_issues: usize,
    swarm_issues: usize,
) -> Vec<String> {
    match status {
        "clean" => vec!["Remote PR and issue queues are empty.".to_string()],
        "review-needed" => vec![format!(
            "Review {source_prs} source PR(s), {swarm_prs} swarm PR(s), {source_issues} source issue(s), and {swarm_issues} swarm issue(s); merge, close, defer, or convert each item before treating the queue as boring."
        )],
        _ => vec![
            "Verify queues with `rtk gh pr list` and `rtk gh issue list` for both source and swarm repos."
                .to_string(),
        ],
    }
}

fn inspect_routed_ci_health(workspace_root: &Path) -> RoutedCiHealthReport {
    let mut notes = Vec::new();

    if !workspace_root.join(".git").exists() {
        notes.push("not a Git checkout; live routed CI inspection was skipped".to_string());
        return routed_ci_health_from_parts(None, None, notes);
    }

    let source_latest_run = github_latest_routed_ci_run(SOURCE_REPO, SOURCE_BRANCH, &mut notes);
    let swarm_latest_run = github_latest_routed_ci_run(SWARM_REPO, SWARM_BRANCH, &mut notes);

    routed_ci_health_from_parts(source_latest_run, swarm_latest_run, notes)
}

fn github_latest_routed_ci_run(
    repo: &str,
    branch: &str,
    notes: &mut Vec<String>,
) -> Option<RoutedCiRunReport> {
    let args = [
        "run",
        "list",
        "--repo",
        repo,
        "--workflow",
        ROUTED_WORKFLOW,
        "--branch",
        branch,
        "--limit",
        "1",
        "--json",
        "databaseId,status,conclusion,displayTitle,event,headBranch,url,createdAt,updatedAt",
    ];
    gh_json_array(&args, notes)
        .into_iter()
        .next()
        .map(routed_ci_run_from_json)
}

fn routed_ci_run_from_json(value: serde_json::Value) -> RoutedCiRunReport {
    let status = json_str(&value, "status");
    let conclusion = json_str(&value, "conclusion");
    let success = status == "completed" && conclusion == "success";

    RoutedCiRunReport {
        id: value.get("databaseId").and_then(serde_json::Value::as_u64),
        status,
        conclusion,
        display_title: json_str(&value, "displayTitle"),
        event: json_str(&value, "event"),
        head_branch: json_str(&value, "headBranch"),
        created_at: json_str(&value, "createdAt"),
        updated_at: json_str(&value, "updatedAt"),
        url: json_str(&value, "url"),
        success,
    }
}

fn routed_ci_health_from_parts(
    source_latest_run: Option<RoutedCiRunReport>,
    swarm_latest_run: Option<RoutedCiRunReport>,
    notes: Vec<String>,
) -> RoutedCiHealthReport {
    let source_ok = source_latest_run.as_ref().is_some_and(|run| run.success);
    let swarm_ok = swarm_latest_run.as_ref().is_some_and(|run| run.success);
    let status = if !notes.is_empty() {
        "unavailable"
    } else if source_ok && swarm_ok {
        "green"
    } else {
        "attention-needed"
    }
    .to_string();
    let next_actions = routed_ci_health_next_actions(&status, source_ok, swarm_ok);

    RoutedCiHealthReport {
        status,
        workflow: ROUTED_WORKFLOW.to_string(),
        source_repo: SOURCE_REPO.to_string(),
        source_branch: SOURCE_BRANCH.to_string(),
        source_latest_run,
        swarm_repo: SWARM_REPO.to_string(),
        swarm_branch: SWARM_BRANCH.to_string(),
        swarm_latest_run,
        notes,
        next_actions,
    }
}

fn routed_ci_health_next_actions(status: &str, source_ok: bool, swarm_ok: bool) -> Vec<String> {
    match status {
        "green" => vec![
            "Latest routed CI runs on source and swarm main completed successfully.".to_string(),
        ],
        "attention-needed" => {
            let mut missing = Vec::new();
            if !source_ok {
                missing.push("source main");
            }
            if !swarm_ok {
                missing.push("swarm main");
            }
            vec![format!(
                "Inspect the latest `{ROUTED_WORKFLOW}` run on {} before treating the control plane as green.",
                missing.join(" and ")
            )]
        }
        _ => vec![format!(
            "Verify routed CI with `rtk gh run list --repo {SOURCE_REPO} --workflow \"{ROUTED_WORKFLOW}\" --branch {SOURCE_BRANCH} --limit 3` and the matching swarm command."
        )],
    }
}

fn routed_ci_run_summary(run: &Option<RoutedCiRunReport>) -> String {
    match run {
        Some(run) => {
            let id = run
                .id
                .map(|value| value.to_string())
                .unwrap_or_else(|| "?".to_string());
            format!(
                "#{id} {}/{} [{} on {}] {}",
                run.conclusion, run.status, run.event, run.head_branch, run.display_title
            )
        }
        None => "-".to_string(),
    }
}

fn inspect_promotion_pr_contract(
    workspace_root: &Path,
    git_topology: &GitTopologyReport,
) -> PromotionPrContractReport {
    let mut notes = Vec::new();

    if !workspace_root.join(".git").exists() {
        notes.push("not a Git checkout; live promotion PR inspection was skipped".to_string());
        return promotion_pr_contract_from_parts(PromotionPrContractParts {
            latest_promotion_pr: None,
            latest_promotion_url: None,
            expected_title: None,
            actual_title: None,
            state: None,
            merge_commit: None,
            expected_source_head: git_topology.source_head.clone(),
            expected_swarm_head: git_topology.swarm_head.clone(),
            notes,
        });
    }

    let latest_promotion_merge = git_topology.source_ahead_promotion_merges.first();
    let Some(pr_number) =
        latest_promotion_merge.and_then(|merge| extract_merge_pull_request_number(merge))
    else {
        notes.push("no source promotion merge commit was found".to_string());
        return promotion_pr_contract_from_parts(PromotionPrContractParts {
            latest_promotion_pr: None,
            latest_promotion_url: None,
            expected_title: None,
            actual_title: None,
            state: None,
            merge_commit: None,
            expected_source_head: git_topology.source_head.clone(),
            expected_swarm_head: git_topology.swarm_head.clone(),
            notes,
        });
    };

    let pr_number_text = pr_number.to_string();
    let args = [
        "pr",
        "view",
        pr_number_text.as_str(),
        "--repo",
        SOURCE_REPO,
        "--json",
        "title,body,state,mergeCommit,url",
    ];

    let Some(value) = gh_json_object(&args, &mut notes) else {
        return promotion_pr_contract_from_parts(PromotionPrContractParts {
            latest_promotion_pr: Some(github_receipt(SOURCE_REPO, pr_number)),
            latest_promotion_url: None,
            expected_title: expected_promotion_title(git_topology.swarm_head.as_deref()),
            actual_title: None,
            state: None,
            merge_commit: None,
            expected_source_head: git_topology.source_head.clone(),
            expected_swarm_head: git_topology.swarm_head.clone(),
            notes,
        });
    };

    promotion_pr_contract_from_json(
        pr_number,
        git_topology.source_head.clone(),
        git_topology.swarm_head.clone(),
        value,
        notes,
    )
}

struct PromotionPrContractParts {
    latest_promotion_pr: Option<String>,
    latest_promotion_url: Option<String>,
    expected_title: Option<String>,
    actual_title: Option<String>,
    state: Option<String>,
    merge_commit: Option<String>,
    expected_source_head: Option<String>,
    expected_swarm_head: Option<String>,
    notes: Vec<String>,
}

fn promotion_pr_contract_from_json(
    pr_number: u64,
    expected_source_head: Option<String>,
    expected_swarm_head: Option<String>,
    value: serde_json::Value,
    notes: Vec<String>,
) -> PromotionPrContractReport {
    let body = json_string_opt(&value, "body").unwrap_or_default();
    let parts = PromotionPrContractParts {
        latest_promotion_pr: Some(github_receipt(SOURCE_REPO, pr_number)),
        latest_promotion_url: json_string_opt(&value, "url"),
        expected_title: expected_promotion_title(expected_swarm_head.as_deref()),
        actual_title: json_string_opt(&value, "title"),
        state: json_string_opt(&value, "state"),
        merge_commit: json_pointer_string_opt(&value, "/mergeCommit/oid"),
        expected_source_head,
        expected_swarm_head,
        notes,
    };

    promotion_pr_contract_from_parts_with_body(parts, &body)
}

fn promotion_pr_contract_from_parts(parts: PromotionPrContractParts) -> PromotionPrContractReport {
    promotion_pr_contract_from_parts_with_body(parts, "")
}

fn promotion_pr_contract_from_parts_with_body(
    parts: PromotionPrContractParts,
    body: &str,
) -> PromotionPrContractReport {
    let title_ok = match (&parts.expected_title, &parts.actual_title) {
        (Some(expected), Some(actual)) => Some(actual == expected),
        (None, None) if parts.latest_promotion_pr.is_none() => None,
        _ => Some(false),
    };
    let state_ok = parts.state.as_ref().map(|state| state == "MERGED");
    let merge_commit_ok = match (&parts.merge_commit, &parts.expected_source_head) {
        (Some(actual), Some(expected)) => Some(actual == expected),
        (None, None) if parts.latest_promotion_pr.is_none() => None,
        _ => Some(false),
    };
    let body_mentions_swarm_head = parts.expected_swarm_head.as_ref().map(|head| {
        let short = short_sha(head);
        body.contains(head) || body.contains(&short)
    });
    let body_mentions_included_swarm_prs = Some(
        body.contains("Included swarm PRs") && body.contains("EffortlessMetrics/shiplog-swarm#"),
    );
    let body_mentions_swarm_proof =
        Some(body.contains("Swarm proof") && body.contains(SWARM_REQUIRED_CHECK));
    let lower_body = body.to_ascii_lowercase();
    let body_mentions_merge_method =
        Some(lower_body.contains("regular merge commit") && lower_body.contains("do not squash"));
    let failed_checks = promotion_pr_contract_failed_checks(PromotionPrContractCheckInputs {
        title_ok,
        expected_title: parts.expected_title.as_deref(),
        actual_title: parts.actual_title.as_deref(),
        state_ok,
        state: parts.state.as_deref(),
        merge_commit_ok,
        merge_commit: parts.merge_commit.as_deref(),
        expected_source_head: parts.expected_source_head.as_deref(),
        body_mentions_swarm_head,
        expected_swarm_head: parts.expected_swarm_head.as_deref(),
        body_mentions_included_swarm_prs,
        body_mentions_swarm_proof,
        body_mentions_merge_method,
    });

    let checks = [
        title_ok,
        state_ok,
        merge_commit_ok,
        body_mentions_swarm_head,
        body_mentions_included_swarm_prs,
        body_mentions_swarm_proof,
        body_mentions_merge_method,
    ];
    let status = if !parts.notes.is_empty() {
        "unavailable"
    } else if parts.latest_promotion_pr.is_none() {
        "not-applicable"
    } else if checks.iter().all(|check| *check == Some(true)) {
        "aligned"
    } else {
        "drift"
    }
    .to_string();
    let next_actions = promotion_pr_contract_next_actions(&status, &failed_checks);

    PromotionPrContractReport {
        status,
        source_repo: SOURCE_REPO.to_string(),
        latest_promotion_pr: parts.latest_promotion_pr,
        latest_promotion_url: parts.latest_promotion_url,
        expected_title: parts.expected_title,
        actual_title: parts.actual_title,
        state: parts.state,
        state_ok,
        merge_commit: parts.merge_commit,
        expected_source_head: parts.expected_source_head,
        merge_commit_ok,
        expected_swarm_head: parts.expected_swarm_head,
        title_ok,
        body_mentions_swarm_head,
        body_mentions_included_swarm_prs,
        body_mentions_swarm_proof,
        body_mentions_merge_method,
        failed_checks,
        notes: parts.notes,
        next_actions,
    }
}

struct PromotionPrContractCheckInputs<'a> {
    title_ok: Option<bool>,
    expected_title: Option<&'a str>,
    actual_title: Option<&'a str>,
    state_ok: Option<bool>,
    state: Option<&'a str>,
    merge_commit_ok: Option<bool>,
    merge_commit: Option<&'a str>,
    expected_source_head: Option<&'a str>,
    body_mentions_swarm_head: Option<bool>,
    expected_swarm_head: Option<&'a str>,
    body_mentions_included_swarm_prs: Option<bool>,
    body_mentions_swarm_proof: Option<bool>,
    body_mentions_merge_method: Option<bool>,
}

fn promotion_pr_contract_failed_checks(inputs: PromotionPrContractCheckInputs<'_>) -> Vec<String> {
    let mut failed = Vec::new();

    if inputs.title_ok == Some(false) {
        failed.push(format!(
            "title mismatch: expected `{}`, actual `{}`",
            inputs.expected_title.unwrap_or("-"),
            inputs.actual_title.unwrap_or("-")
        ));
    }
    if inputs.state_ok == Some(false) {
        failed.push(format!(
            "state is not `MERGED`: actual `{}`",
            inputs.state.unwrap_or("-")
        ));
    }
    if inputs.merge_commit_ok == Some(false) {
        failed.push(format!(
            "merge commit mismatch: expected source head `{}`, actual `{}`",
            inputs.expected_source_head.unwrap_or("-"),
            inputs.merge_commit.unwrap_or("-")
        ));
    }
    if inputs.body_mentions_swarm_head == Some(false) {
        failed.push(format!(
            "body missing swarm head SHA `{}`",
            inputs.expected_swarm_head.unwrap_or("-")
        ));
    }
    if inputs.body_mentions_included_swarm_prs == Some(false) {
        failed.push(
            "body missing `Included swarm PRs` with `EffortlessMetrics/shiplog-swarm#...`"
                .to_string(),
        );
    }
    if inputs.body_mentions_swarm_proof == Some(false) {
        failed.push(format!(
            "body missing `Swarm proof` with `{SWARM_REQUIRED_CHECK}`"
        ));
    }
    if inputs.body_mentions_merge_method == Some(false) {
        failed.push(
            "body missing merge-method boundary: include `regular merge commit` and `do not squash`"
                .to_string(),
        );
    }

    failed
}

fn expected_promotion_title(swarm_head: Option<&str>) -> Option<String> {
    swarm_head.map(|head| {
        format!(
            "merge(swarm): promote shiplog-swarm through {}",
            short_sha(head)
        )
    })
}

fn short_sha(value: &str) -> String {
    value.chars().take(7).collect()
}

fn promotion_pr_contract_next_actions(status: &str, failed_checks: &[String]) -> Vec<String> {
    match status {
        "aligned" => vec![
            "Latest source promotion PR records the swarm head, included swarm PRs, proof, and merge-commit boundary."
                .to_string(),
        ],
        "drift" if failed_checks.is_empty() => {
            vec![
                "Inspect the latest source promotion PR body and title before relying on it as a traceability receipt."
                    .to_string(),
            ]
        }
        "drift" => vec![format!(
            "Repair the latest source promotion PR receipt before relying on it: {}.",
            failed_checks.join("; ")
        )],
        "not-applicable" => vec![
            "No source promotion PR is available yet; create one after the next proven swarm change lands."
                .to_string(),
        ],
        _ => vec![
            "Verify the latest source promotion PR with `rtk gh pr view <number> --repo EffortlessMetrics/shiplog --json title,body,state,mergeCommit,url`."
                .to_string(),
        ],
    }
}

fn inspect_branch_protection_contract(workspace_root: &Path) -> BranchProtectionContractReport {
    let mut notes = Vec::new();
    let expected_required_status_checks = vec![SWARM_REQUIRED_CHECK.to_string()];

    if !workspace_root.join(".git").exists() {
        notes.push("not a Git checkout; live branch protection inspection was skipped".to_string());
        return branch_protection_contract_from_parts(
            expected_required_status_checks,
            Vec::new(),
            None,
            None,
            None,
            None,
            notes,
        );
    }

    let path = format!("repos/{SWARM_REPO}/branches/{SWARM_BRANCH}/protection");
    match Command::new("gh").args(["api", &path]).output() {
        Ok(output) if output.status.success() => {
            match serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                Ok(value) => {
                    branch_protection_contract_from_json(expected_required_status_checks, value)
                }
                Err(err) => {
                    notes.push(format!("gh api {path} returned invalid JSON: {err}"));
                    branch_protection_contract_from_parts(
                        expected_required_status_checks,
                        Vec::new(),
                        None,
                        None,
                        None,
                        None,
                        notes,
                    )
                }
            }
        }
        Ok(output) => {
            let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
            notes.push(format!(
                "gh api {path} failed: {}",
                if detail.is_empty() {
                    output.status.to_string()
                } else {
                    detail
                }
            ));
            branch_protection_contract_from_parts(
                expected_required_status_checks,
                Vec::new(),
                None,
                None,
                None,
                None,
                notes,
            )
        }
        Err(err) => {
            notes.push(format!("gh api {path} failed: {err}"));
            branch_protection_contract_from_parts(
                expected_required_status_checks,
                Vec::new(),
                None,
                None,
                None,
                None,
                notes,
            )
        }
    }
}

fn branch_protection_contract_from_json(
    expected_required_status_checks: Vec<String>,
    value: serde_json::Value,
) -> BranchProtectionContractReport {
    let actual_required_status_checks = value
        .pointer("/required_status_checks/contexts")
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let strict_required_status_checks = value
        .pointer("/required_status_checks/strict")
        .and_then(serde_json::Value::as_bool);
    let enforce_admins = value
        .pointer("/enforce_admins/enabled")
        .and_then(serde_json::Value::as_bool);
    let allow_force_pushes = value
        .pointer("/allow_force_pushes/enabled")
        .and_then(serde_json::Value::as_bool);
    let allow_deletions = value
        .pointer("/allow_deletions/enabled")
        .and_then(serde_json::Value::as_bool);

    branch_protection_contract_from_parts(
        expected_required_status_checks,
        actual_required_status_checks,
        strict_required_status_checks,
        enforce_admins,
        allow_force_pushes,
        allow_deletions,
        Vec::new(),
    )
}

fn branch_protection_contract_from_parts(
    expected_required_status_checks: Vec<String>,
    actual_required_status_checks: Vec<String>,
    strict_required_status_checks: Option<bool>,
    enforce_admins: Option<bool>,
    allow_force_pushes: Option<bool>,
    allow_deletions: Option<bool>,
    notes: Vec<String>,
) -> BranchProtectionContractReport {
    let required_status_checks_ok = if notes.is_empty() {
        Some(same_string_set(
            &expected_required_status_checks,
            &actual_required_status_checks,
        ))
    } else {
        None
    };
    let strict_required_status_checks_ok = strict_required_status_checks;
    let allow_force_pushes_ok = allow_force_pushes.map(|value| !value);
    let allow_deletions_ok = allow_deletions.map(|value| !value);
    let disallowed_required_checks = actual_required_status_checks
        .iter()
        .filter(|check| DISALLOWED_BRANCH_PROTECTION_CHECKS.contains(&check.as_str()))
        .cloned()
        .collect::<Vec<_>>();

    let status = if !notes.is_empty() {
        "unavailable"
    } else if required_status_checks_ok == Some(true)
        && strict_required_status_checks_ok == Some(true)
        && allow_force_pushes_ok == Some(true)
        && allow_deletions_ok == Some(true)
        && disallowed_required_checks.is_empty()
    {
        "aligned"
    } else {
        "drift"
    }
    .to_string();
    let next_actions = branch_protection_next_actions(&status);

    BranchProtectionContractReport {
        status,
        repo: SWARM_REPO.to_string(),
        branch: SWARM_BRANCH.to_string(),
        expected_required_status_checks,
        actual_required_status_checks,
        required_status_checks_ok,
        strict_required_status_checks,
        strict_required_status_checks_ok,
        enforce_admins,
        allow_force_pushes,
        allow_force_pushes_ok,
        allow_deletions,
        allow_deletions_ok,
        disallowed_required_checks,
        notes,
        next_actions,
    }
}

fn same_string_set(expected: &[String], actual: &[String]) -> bool {
    expected.iter().collect::<BTreeSet<_>>() == actual.iter().collect::<BTreeSet<_>>()
}

fn branch_protection_next_actions(status: &str) -> Vec<String> {
    match status {
        "aligned" => vec![
            "Branch protection requires only `Shiplog Rust Small Result`; continue normal swarm PRs."
                .to_string(),
        ],
        "drift" => vec![format!(
            "Repair `{SWARM_REPO}/{SWARM_BRANCH}` branch protection so the only required status check is `{SWARM_REQUIRED_CHECK}` and conditional implementation jobs are not required."
        )],
        _ => vec![format!(
            "Verify live branch protection with `rtk gh api repos/{SWARM_REPO}/branches/{SWARM_BRANCH}/protection`."
        )],
    }
}

fn inspect_receipt_freshness(
    workspace_root: &Path,
    goal: &ActiveGoal,
    git_topology: &GitTopologyReport,
) -> ReceiptFreshnessReport {
    let mut notes = Vec::new();
    let plan_text = load_plan_texts(workspace_root, goal, &mut notes);

    let latest_source_promotion_merge = git_topology.source_ahead_promotion_merges.first().cloned();
    let latest_source_promotion_receipt = latest_source_promotion_merge
        .as_deref()
        .and_then(extract_merge_pull_request_number)
        .map(|number| github_receipt(SOURCE_REPO, number));

    let latest_swarm_head = git_line(
        workspace_root,
        &["log", "--oneline", "-1", "swarm/main"],
        &mut notes,
    );
    let latest_swarm_receipt = latest_swarm_head
        .as_deref()
        .and_then(extract_parenthesized_pull_request_number)
        .map(|number| github_receipt(SWARM_REPO, number));

    let latest_source_receipt_in_active_goal =
        receipt_presence_in_goal(goal, latest_source_promotion_receipt.as_deref());
    let latest_source_receipt_in_plan =
        receipt_presence_in_text(&plan_text, latest_source_promotion_receipt.as_deref());
    let latest_swarm_receipt_in_active_goal =
        receipt_presence_in_goal(goal, latest_swarm_receipt.as_deref());
    let latest_swarm_receipt_in_plan =
        receipt_presence_in_text(&plan_text, latest_swarm_receipt.as_deref());

    let mut missing_active_goal_receipts = Vec::new();
    push_missing_receipt(
        &mut missing_active_goal_receipts,
        latest_source_promotion_receipt.as_deref(),
        latest_source_receipt_in_active_goal,
    );
    push_missing_receipt(
        &mut missing_active_goal_receipts,
        latest_swarm_receipt.as_deref(),
        latest_swarm_receipt_in_active_goal,
    );

    let mut missing_plan_receipts = Vec::new();
    push_missing_receipt(
        &mut missing_plan_receipts,
        latest_source_promotion_receipt.as_deref(),
        latest_source_receipt_in_plan,
    );
    push_missing_receipt(
        &mut missing_plan_receipts,
        latest_swarm_receipt.as_deref(),
        latest_swarm_receipt_in_plan,
    );

    let mut required = Vec::new();
    if let Some(value) = latest_source_receipt_in_active_goal {
        required.push(value);
    } else {
        notes.push(
            "latest source promotion PR could not be inferred from source promotion commits"
                .to_string(),
        );
    }
    if let Some(value) = latest_source_receipt_in_plan {
        required.push(value);
    }
    if let Some(value) = latest_swarm_receipt_in_active_goal {
        required.push(value);
    } else {
        notes.push("latest swarm PR could not be inferred from swarm/main head".to_string());
    }
    if let Some(value) = latest_swarm_receipt_in_plan {
        required.push(value);
    }

    let status = receipt_freshness_status(&required, latest_swarm_head.as_deref()).to_string();
    if status == "pending-next-substantive-pr" {
        notes.push(
            "latest swarm head is a receipt refresh; self-referential receipts should be carried by the next substantive swarm PR"
                .to_string(),
        );
    }

    let next_actions = receipt_freshness_next_actions(&status);

    ReceiptFreshnessReport {
        status,
        latest_source_promotion_merge,
        latest_source_promotion_receipt,
        latest_source_receipt_in_active_goal,
        latest_source_receipt_in_plan,
        latest_swarm_head,
        latest_swarm_receipt,
        latest_swarm_receipt_in_active_goal,
        latest_swarm_receipt_in_plan,
        missing_active_goal_receipts,
        missing_plan_receipts,
        notes,
        next_actions,
    }
}

fn push_missing_receipt(
    missing_receipts: &mut Vec<String>,
    receipt: Option<&str>,
    present: Option<bool>,
) {
    if present == Some(false)
        && let Some(receipt) = receipt
    {
        missing_receipts.push(receipt.to_string());
    }
}

fn receipt_freshness_status(required: &[bool], latest_swarm_head: Option<&str>) -> &'static str {
    if required.is_empty() {
        "unavailable"
    } else if required.iter().all(|present| *present) {
        "current"
    } else if is_receipt_refresh_head(latest_swarm_head) {
        "pending-next-substantive-pr"
    } else {
        "stale"
    }
}

fn is_receipt_refresh_head(latest_swarm_head: Option<&str>) -> bool {
    let Some(latest_swarm_head) = latest_swarm_head else {
        return false;
    };
    let subject = latest_swarm_head
        .split_once(' ')
        .map(|(_, subject)| subject)
        .unwrap_or(latest_swarm_head)
        .to_ascii_lowercase();
    (subject.contains("refresh") || subject.contains("refreshed")) && subject.contains("receipt")
}

fn load_plan_texts(workspace_root: &Path, goal: &ActiveGoal, notes: &mut Vec<String>) -> String {
    let mut text = String::new();
    for plan in goal
        .work_item
        .iter()
        .map(|item| item.plan.trim())
        .filter(|plan| !plan.is_empty())
    {
        match fs::read_to_string(workspace_root.join(plan)) {
            Ok(plan_text) => {
                text.push_str(&plan_text);
                text.push('\n');
            }
            Err(err) => notes.push(format!("read {plan} failed: {err}")),
        }
    }
    text
}

fn receipt_presence_in_goal(goal: &ActiveGoal, receipt: Option<&str>) -> Option<bool> {
    let receipt = receipt?;
    Some(
        goal.work_item
            .iter()
            .any(|item| item.receipts.iter().any(|value| value == receipt)),
    )
}

fn receipt_presence_in_text(text: &str, receipt: Option<&str>) -> Option<bool> {
    receipt.map(|receipt| text.contains(receipt))
}

fn github_receipt(repo: &str, number: u64) -> String {
    format!("{repo}#{number}")
}

fn extract_merge_pull_request_number(commit: &str) -> Option<u64> {
    let subject = commit
        .split_once(' ')
        .map(|(_, subject)| subject)
        .unwrap_or(commit);
    let rest = subject.strip_prefix("Merge pull request #")?;
    parse_leading_number(rest)
}

fn extract_parenthesized_pull_request_number(commit: &str) -> Option<u64> {
    let start = commit.rfind("(#")?;
    let rest = &commit[start + 2..];
    let end = rest.find(')')?;
    rest[..end].parse().ok()
}

fn parse_leading_number(text: &str) -> Option<u64> {
    let digits = text
        .chars()
        .take_while(|value| value.is_ascii_digit())
        .collect::<String>();
    digits.parse().ok()
}

fn receipt_freshness_next_actions(status: &str) -> Vec<String> {
    match status {
        "current" => vec![
            "Latest completed source promotion and swarm PR receipts are recorded in the active goal and plan."
                .to_string(),
        ],
        "stale" => vec![
            "Record the latest completed source promotion and swarm PR receipts in `.codex/goals/active.toml` and `plans/shiplog-swarm/implementation-plan.md` during the next substantive swarm PR."
                .to_string(),
        ],
        "pending-next-substantive-pr" => vec![
            "The latest swarm change is itself a receipt refresh; carry these self-referential receipts in the next substantive swarm PR instead of opening another receipt-only loop."
                .to_string(),
        ],
        _ => vec![
            "Verify source promotion and swarm PR subjects, then refresh receipt records manually if needed."
                .to_string(),
        ],
    }
}

struct SourceAheadSummary {
    classification: String,
    promotion_merges: Vec<String>,
    other_commits: Vec<String>,
}

fn classify_source_ahead(commits: &[String]) -> SourceAheadSummary {
    let mut promotion_merges = Vec::new();
    let mut other_commits = Vec::new();

    for commit in commits {
        if is_source_promotion_merge(commit) {
            promotion_merges.push(commit.clone());
        } else {
            other_commits.push(commit.clone());
        }
    }

    let classification = match (
        promotion_merges.is_empty(),
        other_commits.is_empty(),
        commits.is_empty(),
    ) {
        (_, _, true) => "none",
        (false, true, false) => "promotion-merge-only",
        (true, false, false) => "non-promotion",
        (false, false, false) => "mixed",
        (true, true, false) => "none",
    }
    .to_string();

    SourceAheadSummary {
        classification,
        promotion_merges,
        other_commits,
    }
}

fn is_source_promotion_merge(commit: &str) -> bool {
    let subject = commit
        .split_once(' ')
        .map(|(_, subject)| subject)
        .unwrap_or(commit);

    (subject.starts_with("Merge pull request #") && subject.contains("promote/swarm-"))
        || subject.starts_with("merge(swarm): promote shiplog-swarm through ")
}

fn topology_next_actions(
    status: &str,
    trees_aligned: Option<bool>,
    source_ahead_classification: &str,
    source_ahead_other_commits: &[String],
    swarm_ahead: &[String],
) -> Vec<String> {
    let mut actions = Vec::new();

    if !source_ahead_other_commits.is_empty() {
        actions.push(
            "Pause promotion cadence and reconcile source-only non-promotion commits before new swarm work lands."
                .to_string(),
        );
    }

    if !swarm_ahead.is_empty() {
        actions.push(
            "Open a source promotion PR from `swarm/main` and merge it with a regular merge commit after checks pass."
                .to_string(),
        );
    }

    match (status, trees_aligned, source_ahead_classification) {
        ("identical", _, _) => actions.push(
            "Continue normal development in `EffortlessMetrics/shiplog-swarm` with a focused PR."
                .to_string(),
        ),
        ("tree-aligned", Some(true), "promotion-merge-only") if swarm_ahead.is_empty() => {
            actions.push(
                "Continue normal development in `EffortlessMetrics/shiplog-swarm`; no source promotion is pending."
                    .to_string(),
            );
        }
        ("tree-aligned", Some(true), "none") if swarm_ahead.is_empty() => actions.push(
            "Continue normal development in `EffortlessMetrics/shiplog-swarm`; source and swarm trees are aligned."
                .to_string(),
        ),
        ("diverged", _, _) | ("tree-drift", Some(false), _) => actions.push(
            "Stop normal promotion and inspect the source/swarm diff before merging more work."
                .to_string(),
        ),
        ("swarm-ahead", _, _) if !swarm_ahead.is_empty() => {}
        ("source-ahead", _, "promotion-merge-only") => actions.push(
            "No swarm promotion is pending; source is ahead only by promotion merge commits."
                .to_string(),
        ),
        ("unavailable", _, _) => actions.push(
            "Fetch `origin` and `swarm`, verify both refs exist, and rerun `rtk cargo xtask repo-contract-report`."
                .to_string(),
        ),
        _ => {
            if actions.is_empty() {
                actions.push(
                    "Inspect source/swarm topology before choosing the next promotion or development step."
                        .to_string(),
                );
            }
        }
    }

    actions
}

fn git_line(workspace_root: &Path, args: &[&str], notes: &mut Vec<String>) -> Option<String> {
    git_lines(workspace_root, args, notes).into_iter().next()
}

fn git_lines(workspace_root: &Path, args: &[&str], notes: &mut Vec<String>) -> Vec<String> {
    match Command::new("git")
        .arg("-C")
        .arg(workspace_root)
        .args(args)
        .output()
    {
        Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToOwned::to_owned)
            .collect(),
        Ok(output) => {
            let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
            notes.push(format!(
                "git {} failed: {}",
                args.join(" "),
                if detail.is_empty() {
                    output.status.to_string()
                } else {
                    detail
                }
            ));
            Vec::new()
        }
        Err(err) => {
            notes.push(format!("git {} failed: {err}", args.join(" ")));
            Vec::new()
        }
    }
}

fn git_trees_aligned(
    workspace_root: &Path,
    source_ref: &str,
    swarm_ref: &str,
    notes: &mut Vec<String>,
) -> Option<bool> {
    match Command::new("git")
        .arg("-C")
        .arg(workspace_root)
        .args(["diff", "--quiet", source_ref, swarm_ref])
        .status()
    {
        Ok(status) if status.success() => Some(true),
        Ok(status) if status.code() == Some(1) => Some(false),
        Ok(status) => {
            notes.push(format!(
                "git diff --quiet {source_ref} {swarm_ref} failed: {status}"
            ));
            None
        }
        Err(err) => {
            notes.push(format!(
                "git diff --quiet {source_ref} {swarm_ref} failed: {err}"
            ));
            None
        }
    }
}

fn push_artifact_edges(artifact: &Artifact, edges: &mut Vec<GraphEdge>) {
    push_optional_edge(
        &artifact.id,
        artifact.linked_proposal.as_deref(),
        "linked_proposal",
        edges,
    );
    push_optional_edge(
        &artifact.id,
        artifact.linked_spec.as_deref(),
        "linked_spec",
        edges,
    );
    for linked in &artifact.linked_specs {
        edges.push(GraphEdge {
            from: artifact.id.clone(),
            to: linked.clone(),
            relation: "linked_spec".to_string(),
        });
    }
    push_optional_edge(
        &artifact.id,
        artifact.linked_adr.as_deref(),
        "linked_adr",
        edges,
    );
    for linked in &artifact.linked_adrs {
        edges.push(GraphEdge {
            from: artifact.id.clone(),
            to: linked.clone(),
            relation: "linked_adr".to_string(),
        });
    }
    push_optional_edge(
        &artifact.id,
        artifact.linked_plan.as_deref(),
        "linked_plan",
        edges,
    );
    push_optional_edge(
        &artifact.id,
        artifact.superseded_by.as_deref(),
        "superseded_by",
        edges,
    );
    for policy in &artifact.policy_impact {
        edges.push(GraphEdge {
            from: artifact.id.clone(),
            to: policy.clone(),
            relation: "policy_impact".to_string(),
        });
    }
}

fn push_work_item_edges(item: &WorkItem, edges: &mut Vec<GraphEdge>) {
    let from = format!("work-item:{}", item.id);
    push_optional_edge(&from, item.proposal.as_deref(), "proposal", edges);
    push_optional_edge(&from, item.spec.as_deref(), "spec", edges);
    push_optional_edge(&from, item.adr.as_deref(), "adr", edges);
    edges.push(GraphEdge {
        from,
        to: item.plan.clone(),
        relation: "plan".to_string(),
    });
}

fn push_optional_edge(from: &str, to: Option<&str>, relation: &str, edges: &mut Vec<GraphEdge>) {
    let Some(to) = to else {
        return;
    };
    if to.trim().is_empty() || to == "none" {
        return;
    }
    edges.push(GraphEdge {
        from: from.to_string(),
        to: to.to_string(),
        relation: relation.to_string(),
    });
}

fn parse_support_tier_claims(text: &str) -> Vec<SupportTierClaim> {
    let mut claims = Vec::new();
    let mut in_claim_map = false;

    for line in text.lines() {
        if line.trim() == "## Claim map" {
            in_claim_map = true;
            continue;
        }
        if !in_claim_map {
            continue;
        }
        if line.starts_with("## ") && line.trim() != "## Claim map" {
            break;
        }
        if line.trim().is_empty()
            || line.trim() == "| Surface | Tier | Claim | Proof command | Notes |"
            || is_table_separator(line)
            || !line.trim_start().starts_with('|')
        {
            continue;
        }

        let cells = table_cells(line);
        if cells.len() == 5 {
            claims.push(SupportTierClaim {
                surface: cells[0].to_string(),
                tier: cells[1].to_string(),
                claim: cells[2].to_string(),
                proof_command: cells[3].to_string(),
                notes: cells[4].to_string(),
            });
        }
    }

    claims
}

fn render_markdown(report: &RepoContractReport) -> String {
    let mut out = String::new();
    out.push_str("# Repo contract report\n\n");
    out.push_str(&format!("Generated by `{REPORT_COMMAND}`.\n\n"));

    out.push_str("## Active goal\n\n");
    out.push_str("| Field | Value |\n|---|---|\n");
    push_row(&mut out, "ID", &report.active_goal.id);
    push_row(&mut out, "Title", &report.active_goal.title);
    push_row(&mut out, "Status", &report.active_goal.status);
    push_row(&mut out, "Owner", &report.active_goal.owner);
    push_row(&mut out, "Created", &report.active_goal.created);
    push_row(
        &mut out,
        "Active work items",
        &join_or_dash(&report.active_goal.active_work_items),
    );
    push_row(
        &mut out,
        "Ready work items",
        &join_or_dash(&report.active_goal.ready_work_items),
    );
    out.push_str("\n### Objective\n\n");
    out.push_str(&md(report.active_goal.objective.trim()));
    out.push('\n');
    push_markdown_bullets(&mut out, "End state", &report.active_goal.end_state);

    out.push_str("\n## Git topology\n\n");
    out.push_str("| Field | Value |\n|---|---|\n");
    push_row(&mut out, "Status", &report.git_topology.status);
    push_row(&mut out, "Source ref", &report.git_topology.source_ref);
    push_row(&mut out, "Swarm ref", &report.git_topology.swarm_ref);
    push_row(
        &mut out,
        "Source head",
        report.git_topology.source_head.as_deref().unwrap_or("-"),
    );
    push_row(
        &mut out,
        "Swarm head",
        report.git_topology.swarm_head.as_deref().unwrap_or("-"),
    );
    push_row(
        &mut out,
        "Merge base",
        report.git_topology.merge_base.as_deref().unwrap_or("-"),
    );
    push_row(
        &mut out,
        "Trees aligned",
        &report
            .git_topology
            .trees_aligned
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string()),
    );
    push_row(
        &mut out,
        "Source ahead",
        &format!("{} commit(s)", report.git_topology.source_ahead.len()),
    );
    push_row(
        &mut out,
        "Source ahead classification",
        &report.git_topology.source_ahead_classification,
    );
    push_row(
        &mut out,
        "Source promotion merges",
        &format!(
            "{} commit(s)",
            report.git_topology.source_ahead_promotion_merges.len()
        ),
    );
    push_row(
        &mut out,
        "Source other commits",
        &format!(
            "{} commit(s)",
            report.git_topology.source_ahead_other_commits.len()
        ),
    );
    push_row(
        &mut out,
        "Swarm ahead",
        &format!("{} commit(s)", report.git_topology.swarm_ahead.len()),
    );
    push_row(
        &mut out,
        "Notes",
        &format!("{} note(s)", report.git_topology.notes.len()),
    );
    push_row(
        &mut out,
        "Next actions",
        &format!("{} action(s)", report.git_topology.next_actions.len()),
    );
    push_markdown_list(
        &mut out,
        "Source ahead non-promotion commits",
        &report.git_topology.source_ahead_other_commits,
    );
    push_markdown_list_limited(
        &mut out,
        "Source promotion merge commits",
        &report.git_topology.source_ahead_promotion_merges,
        12,
    );
    push_markdown_list(
        &mut out,
        "Swarm ahead commits",
        &report.git_topology.swarm_ahead,
    );
    push_markdown_list(&mut out, "Git topology notes", &report.git_topology.notes);
    push_markdown_bullets(
        &mut out,
        "Git topology next actions",
        &report.git_topology.next_actions,
    );

    out.push_str("\n## Local checkout\n\n");
    out.push_str("| Field | Value |\n|---|---|\n");
    push_row(&mut out, "Status", &report.local_checkout.status);
    push_row(
        &mut out,
        "Branch",
        report
            .local_checkout
            .branch_summary
            .as_deref()
            .unwrap_or("-"),
    );
    push_row(
        &mut out,
        "Clean",
        &report
            .local_checkout
            .clean
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string()),
    );
    push_row(
        &mut out,
        "Status entries",
        &format!("{} item(s)", report.local_checkout.status_entries.len()),
    );
    push_row(
        &mut out,
        "Local branches",
        &format!("{} branch(es)", report.local_checkout.local_branch_count),
    );
    push_row(
        &mut out,
        "Local merged cleanup candidates",
        &format!(
            "{} branch(es)",
            report.local_checkout.local_merged_cleanup_candidates.len()
        ),
    );
    push_row(
        &mut out,
        "Protected local branches",
        &format!(
            "{} branch(es)",
            report.local_checkout.protected_local_branches.len()
        ),
    );
    push_row(
        &mut out,
        "Notes",
        &format!("{} note(s)", report.local_checkout.notes.len()),
    );
    push_row(
        &mut out,
        "Next actions",
        &format!("{} action(s)", report.local_checkout.next_actions.len()),
    );
    push_markdown_list(
        &mut out,
        "Local checkout status entries",
        &report.local_checkout.status_entries,
    );
    push_markdown_list_limited(
        &mut out,
        "Local merged cleanup candidate branches",
        &report.local_checkout.local_merged_cleanup_candidates,
        20,
    );
    push_markdown_list_limited(
        &mut out,
        "Local merged cleanup review commands",
        &report.local_checkout.local_merged_cleanup_review_commands,
        10,
    );
    push_markdown_list_limited(
        &mut out,
        "Protected local branches",
        &report.local_checkout.protected_local_branches,
        20,
    );
    push_markdown_list(
        &mut out,
        "Local checkout notes",
        &report.local_checkout.notes,
    );
    push_markdown_bullets(
        &mut out,
        "Local checkout next actions",
        &report.local_checkout.next_actions,
    );

    out.push_str("\n## Remote branch hygiene\n\n");
    out.push_str("| Field | Value |\n|---|---|\n");
    push_row(&mut out, "Status", &report.remote_branch_hygiene.status);
    push_row(
        &mut out,
        "Source remote",
        &report.remote_branch_hygiene.source_remote,
    );
    push_row(
        &mut out,
        "Swarm remote",
        &report.remote_branch_hygiene.swarm_remote,
    );
    push_row(
        &mut out,
        "Source cleanup candidates",
        &format!(
            "{} branch(es)",
            report.remote_branch_hygiene.source_cleanup_candidates.len()
        ),
    );
    push_row(
        &mut out,
        "Swarm cleanup candidates",
        &format!(
            "{} branch(es)",
            report.remote_branch_hygiene.swarm_cleanup_candidates.len()
        ),
    );
    push_row(
        &mut out,
        "Source merged cleanup candidates",
        &format!(
            "{} branch(es)",
            report
                .remote_branch_hygiene
                .source_merged_cleanup_candidates
                .len()
        ),
    );
    push_row(
        &mut out,
        "Source review cleanup candidates",
        &format!(
            "{} branch(es)",
            report
                .remote_branch_hygiene
                .source_review_cleanup_candidates
                .len()
        ),
    );
    push_row(
        &mut out,
        "Swarm merged cleanup candidates",
        &format!(
            "{} branch(es)",
            report
                .remote_branch_hygiene
                .swarm_merged_cleanup_candidates
                .len()
        ),
    );
    push_row(
        &mut out,
        "Swarm review cleanup candidates",
        &format!(
            "{} branch(es)",
            report
                .remote_branch_hygiene
                .swarm_review_cleanup_candidates
                .len()
        ),
    );
    push_row(
        &mut out,
        "Source open PR-backed branches",
        &format!(
            "{} branch(es)",
            report.remote_branch_hygiene.source_open_pr_branches.len()
        ),
    );
    push_row(
        &mut out,
        "Swarm open PR-backed branches",
        &format!(
            "{} branch(es)",
            report.remote_branch_hygiene.swarm_open_pr_branches.len()
        ),
    );
    push_row(
        &mut out,
        "Protected branches",
        &format!(
            "{} branch(es)",
            report.remote_branch_hygiene.protected_branches.len()
        ),
    );
    push_row(
        &mut out,
        "Notes",
        &format!("{} note(s)", report.remote_branch_hygiene.notes.len()),
    );
    push_row(
        &mut out,
        "Next actions",
        &format!(
            "{} action(s)",
            report.remote_branch_hygiene.next_actions.len()
        ),
    );
    push_markdown_list_limited(
        &mut out,
        "Source cleanup candidate branches",
        &report.remote_branch_hygiene.source_cleanup_candidates,
        20,
    );
    push_markdown_list_limited(
        &mut out,
        "Swarm cleanup candidate branches",
        &report.remote_branch_hygiene.swarm_cleanup_candidates,
        20,
    );
    push_markdown_list_limited(
        &mut out,
        "Source merged cleanup candidate branches",
        &report
            .remote_branch_hygiene
            .source_merged_cleanup_candidates,
        20,
    );
    push_markdown_list_limited(
        &mut out,
        "Source merged cleanup review commands",
        &report
            .remote_branch_hygiene
            .source_merged_cleanup_review_commands,
        10,
    );
    push_markdown_list_limited(
        &mut out,
        "Source review cleanup candidate branches",
        &report
            .remote_branch_hygiene
            .source_review_cleanup_candidates,
        20,
    );
    push_markdown_list_limited(
        &mut out,
        "Source review cleanup review commands",
        &report
            .remote_branch_hygiene
            .source_review_cleanup_review_commands,
        10,
    );
    push_markdown_list_limited(
        &mut out,
        "Swarm merged cleanup candidate branches",
        &report.remote_branch_hygiene.swarm_merged_cleanup_candidates,
        20,
    );
    push_markdown_list_limited(
        &mut out,
        "Swarm merged cleanup review commands",
        &report
            .remote_branch_hygiene
            .swarm_merged_cleanup_review_commands,
        10,
    );
    push_markdown_list_limited(
        &mut out,
        "Swarm review cleanup candidate branches",
        &report.remote_branch_hygiene.swarm_review_cleanup_candidates,
        20,
    );
    push_markdown_list_limited(
        &mut out,
        "Swarm review cleanup review commands",
        &report
            .remote_branch_hygiene
            .swarm_review_cleanup_review_commands,
        10,
    );
    push_markdown_list_limited(
        &mut out,
        "Source open PR-backed branches",
        &report.remote_branch_hygiene.source_open_pr_branches,
        20,
    );
    push_markdown_list_limited(
        &mut out,
        "Swarm open PR-backed branches",
        &report.remote_branch_hygiene.swarm_open_pr_branches,
        20,
    );
    push_markdown_list_limited(
        &mut out,
        "Protected remote branches",
        &report.remote_branch_hygiene.protected_branches,
        20,
    );
    push_markdown_list(
        &mut out,
        "Remote branch hygiene notes",
        &report.remote_branch_hygiene.notes,
    );
    push_markdown_bullets(
        &mut out,
        "Remote branch hygiene next actions",
        &report.remote_branch_hygiene.next_actions,
    );

    out.push_str("\n## Remote queue hygiene\n\n");
    out.push_str("| Field | Value |\n|---|---|\n");
    push_row(&mut out, "Status", &report.remote_queue_hygiene.status);
    push_row(
        &mut out,
        "Source repo",
        &report.remote_queue_hygiene.source_repo,
    );
    push_row(
        &mut out,
        "Swarm repo",
        &report.remote_queue_hygiene.swarm_repo,
    );
    push_row(
        &mut out,
        "Source open PRs",
        &format!(
            "{} item(s)",
            report.remote_queue_hygiene.source_open_prs.len()
        ),
    );
    push_row(
        &mut out,
        "Swarm open PRs",
        &format!(
            "{} item(s)",
            report.remote_queue_hygiene.swarm_open_prs.len()
        ),
    );
    push_row(
        &mut out,
        "Source open issues",
        &format!(
            "{} item(s)",
            report.remote_queue_hygiene.source_open_issues.len()
        ),
    );
    push_row(
        &mut out,
        "Swarm open issues",
        &format!(
            "{} item(s)",
            report.remote_queue_hygiene.swarm_open_issues.len()
        ),
    );
    push_row(
        &mut out,
        "Notes",
        &format!("{} note(s)", report.remote_queue_hygiene.notes.len()),
    );
    push_row(
        &mut out,
        "Next actions",
        &format!(
            "{} action(s)",
            report.remote_queue_hygiene.next_actions.len()
        ),
    );
    push_markdown_list(
        &mut out,
        "Source open PRs",
        &report.remote_queue_hygiene.source_open_prs,
    );
    push_markdown_list(
        &mut out,
        "Swarm open PRs",
        &report.remote_queue_hygiene.swarm_open_prs,
    );
    push_markdown_list(
        &mut out,
        "Source open issues",
        &report.remote_queue_hygiene.source_open_issues,
    );
    push_markdown_list(
        &mut out,
        "Swarm open issues",
        &report.remote_queue_hygiene.swarm_open_issues,
    );
    push_markdown_list(
        &mut out,
        "Remote queue hygiene notes",
        &report.remote_queue_hygiene.notes,
    );
    push_markdown_bullets(
        &mut out,
        "Remote queue hygiene next actions",
        &report.remote_queue_hygiene.next_actions,
    );

    out.push_str("\n## Routed CI health\n\n");
    out.push_str("| Field | Value |\n|---|---|\n");
    push_row(&mut out, "Status", &report.routed_ci_health.status);
    push_row(&mut out, "Workflow", &report.routed_ci_health.workflow);
    push_row(
        &mut out,
        "Source repo",
        &report.routed_ci_health.source_repo,
    );
    push_row(
        &mut out,
        "Source branch",
        &report.routed_ci_health.source_branch,
    );
    push_row(
        &mut out,
        "Source latest run",
        &routed_ci_run_summary(&report.routed_ci_health.source_latest_run),
    );
    push_row(
        &mut out,
        "Source latest run OK",
        &report
            .routed_ci_health
            .source_latest_run
            .as_ref()
            .map(|run| run.success.to_string())
            .unwrap_or_else(|| "-".to_string()),
    );
    push_row(&mut out, "Swarm repo", &report.routed_ci_health.swarm_repo);
    push_row(
        &mut out,
        "Swarm branch",
        &report.routed_ci_health.swarm_branch,
    );
    push_row(
        &mut out,
        "Swarm latest run",
        &routed_ci_run_summary(&report.routed_ci_health.swarm_latest_run),
    );
    push_row(
        &mut out,
        "Swarm latest run OK",
        &report
            .routed_ci_health
            .swarm_latest_run
            .as_ref()
            .map(|run| run.success.to_string())
            .unwrap_or_else(|| "-".to_string()),
    );
    push_row(
        &mut out,
        "Notes",
        &format!("{} note(s)", report.routed_ci_health.notes.len()),
    );
    push_row(
        &mut out,
        "Next actions",
        &format!("{} action(s)", report.routed_ci_health.next_actions.len()),
    );
    push_markdown_list(
        &mut out,
        "Routed CI health notes",
        &report.routed_ci_health.notes,
    );
    push_markdown_bullets(
        &mut out,
        "Routed CI health next actions",
        &report.routed_ci_health.next_actions,
    );

    out.push_str("\n## Promotion PR contract\n\n");
    out.push_str("| Field | Value |\n|---|---|\n");
    push_row(&mut out, "Status", &report.promotion_pr_contract.status);
    push_row(
        &mut out,
        "Source repo",
        &report.promotion_pr_contract.source_repo,
    );
    push_row(
        &mut out,
        "Latest promotion PR",
        report
            .promotion_pr_contract
            .latest_promotion_pr
            .as_deref()
            .unwrap_or("-"),
    );
    push_row(
        &mut out,
        "Latest promotion URL",
        report
            .promotion_pr_contract
            .latest_promotion_url
            .as_deref()
            .unwrap_or("-"),
    );
    push_row(
        &mut out,
        "Expected title",
        report
            .promotion_pr_contract
            .expected_title
            .as_deref()
            .unwrap_or("-"),
    );
    push_row(
        &mut out,
        "Actual title",
        report
            .promotion_pr_contract
            .actual_title
            .as_deref()
            .unwrap_or("-"),
    );
    push_row(
        &mut out,
        "Title OK",
        &bool_opt(&report.promotion_pr_contract.title_ok),
    );
    push_row(
        &mut out,
        "State",
        report.promotion_pr_contract.state.as_deref().unwrap_or("-"),
    );
    push_row(
        &mut out,
        "State OK",
        &bool_opt(&report.promotion_pr_contract.state_ok),
    );
    push_row(
        &mut out,
        "Merge commit",
        report
            .promotion_pr_contract
            .merge_commit
            .as_deref()
            .unwrap_or("-"),
    );
    push_row(
        &mut out,
        "Expected source head",
        report
            .promotion_pr_contract
            .expected_source_head
            .as_deref()
            .unwrap_or("-"),
    );
    push_row(
        &mut out,
        "Merge commit OK",
        &bool_opt(&report.promotion_pr_contract.merge_commit_ok),
    );
    push_row(
        &mut out,
        "Expected swarm head",
        report
            .promotion_pr_contract
            .expected_swarm_head
            .as_deref()
            .unwrap_or("-"),
    );
    push_row(
        &mut out,
        "Body mentions swarm head",
        &bool_opt(&report.promotion_pr_contract.body_mentions_swarm_head),
    );
    push_row(
        &mut out,
        "Body mentions included swarm PRs",
        &bool_opt(
            &report
                .promotion_pr_contract
                .body_mentions_included_swarm_prs,
        ),
    );
    push_row(
        &mut out,
        "Body mentions swarm proof",
        &bool_opt(&report.promotion_pr_contract.body_mentions_swarm_proof),
    );
    push_row(
        &mut out,
        "Body mentions merge method",
        &bool_opt(&report.promotion_pr_contract.body_mentions_merge_method),
    );
    push_row(
        &mut out,
        "Failed checks",
        &format!(
            "{} check(s)",
            report.promotion_pr_contract.failed_checks.len()
        ),
    );
    push_row(
        &mut out,
        "Notes",
        &format!("{} note(s)", report.promotion_pr_contract.notes.len()),
    );
    push_row(
        &mut out,
        "Next actions",
        &format!(
            "{} action(s)",
            report.promotion_pr_contract.next_actions.len()
        ),
    );
    push_markdown_list(
        &mut out,
        "Promotion PR contract failed checks",
        &report.promotion_pr_contract.failed_checks,
    );
    push_markdown_list(
        &mut out,
        "Promotion PR contract notes",
        &report.promotion_pr_contract.notes,
    );
    push_markdown_bullets(
        &mut out,
        "Promotion PR contract next actions",
        &report.promotion_pr_contract.next_actions,
    );

    out.push_str("\n## Branch protection contract\n\n");
    out.push_str("| Field | Value |\n|---|---|\n");
    push_row(
        &mut out,
        "Status",
        &report.branch_protection_contract.status,
    );
    push_row(&mut out, "Repo", &report.branch_protection_contract.repo);
    push_row(
        &mut out,
        "Branch",
        &report.branch_protection_contract.branch,
    );
    push_row(
        &mut out,
        "Expected required checks",
        &join_or_dash(
            &report
                .branch_protection_contract
                .expected_required_status_checks,
        ),
    );
    push_row(
        &mut out,
        "Actual required checks",
        &join_or_dash(
            &report
                .branch_protection_contract
                .actual_required_status_checks,
        ),
    );
    push_row(
        &mut out,
        "Required checks OK",
        &bool_opt(&report.branch_protection_contract.required_status_checks_ok),
    );
    push_row(
        &mut out,
        "Strict required checks",
        &bool_opt(
            &report
                .branch_protection_contract
                .strict_required_status_checks,
        ),
    );
    push_row(
        &mut out,
        "Strict required checks OK",
        &bool_opt(
            &report
                .branch_protection_contract
                .strict_required_status_checks_ok,
        ),
    );
    push_row(
        &mut out,
        "Enforce admins",
        &bool_opt(&report.branch_protection_contract.enforce_admins),
    );
    push_row(
        &mut out,
        "Allow force pushes",
        &bool_opt(&report.branch_protection_contract.allow_force_pushes),
    );
    push_row(
        &mut out,
        "Allow force pushes OK",
        &bool_opt(&report.branch_protection_contract.allow_force_pushes_ok),
    );
    push_row(
        &mut out,
        "Allow deletions",
        &bool_opt(&report.branch_protection_contract.allow_deletions),
    );
    push_row(
        &mut out,
        "Allow deletions OK",
        &bool_opt(&report.branch_protection_contract.allow_deletions_ok),
    );
    push_row(
        &mut out,
        "Disallowed required checks",
        &format!(
            "{} check(s)",
            report
                .branch_protection_contract
                .disallowed_required_checks
                .len()
        ),
    );
    push_row(
        &mut out,
        "Notes",
        &format!("{} note(s)", report.branch_protection_contract.notes.len()),
    );
    push_row(
        &mut out,
        "Next actions",
        &format!(
            "{} action(s)",
            report.branch_protection_contract.next_actions.len()
        ),
    );
    push_markdown_list(
        &mut out,
        "Branch protection actual required checks",
        &report
            .branch_protection_contract
            .actual_required_status_checks,
    );
    push_markdown_list(
        &mut out,
        "Branch protection disallowed required checks",
        &report.branch_protection_contract.disallowed_required_checks,
    );
    push_markdown_list(
        &mut out,
        "Branch protection notes",
        &report.branch_protection_contract.notes,
    );
    push_markdown_bullets(
        &mut out,
        "Branch protection next actions",
        &report.branch_protection_contract.next_actions,
    );

    out.push_str("\n## Receipt freshness\n\n");
    out.push_str("| Field | Value |\n|---|---|\n");
    push_row(&mut out, "Status", &report.receipt_freshness.status);
    push_row(
        &mut out,
        "Latest source promotion merge",
        report
            .receipt_freshness
            .latest_source_promotion_merge
            .as_deref()
            .unwrap_or("-"),
    );
    push_row(
        &mut out,
        "Latest source promotion receipt",
        report
            .receipt_freshness
            .latest_source_promotion_receipt
            .as_deref()
            .unwrap_or("-"),
    );
    push_row(
        &mut out,
        "Source receipt in active goal",
        &bool_opt(
            &report
                .receipt_freshness
                .latest_source_receipt_in_active_goal,
        ),
    );
    push_row(
        &mut out,
        "Source receipt in plan",
        &bool_opt(&report.receipt_freshness.latest_source_receipt_in_plan),
    );
    push_row(
        &mut out,
        "Latest swarm head",
        report
            .receipt_freshness
            .latest_swarm_head
            .as_deref()
            .unwrap_or("-"),
    );
    push_row(
        &mut out,
        "Latest swarm receipt",
        report
            .receipt_freshness
            .latest_swarm_receipt
            .as_deref()
            .unwrap_or("-"),
    );
    push_row(
        &mut out,
        "Swarm receipt in active goal",
        &bool_opt(&report.receipt_freshness.latest_swarm_receipt_in_active_goal),
    );
    push_row(
        &mut out,
        "Swarm receipt in plan",
        &bool_opt(&report.receipt_freshness.latest_swarm_receipt_in_plan),
    );
    push_row(
        &mut out,
        "Missing active goal receipts",
        &format!(
            "{} item(s)",
            report.receipt_freshness.missing_active_goal_receipts.len()
        ),
    );
    push_row(
        &mut out,
        "Missing plan receipts",
        &format!(
            "{} item(s)",
            report.receipt_freshness.missing_plan_receipts.len()
        ),
    );
    push_row(
        &mut out,
        "Notes",
        &format!("{} note(s)", report.receipt_freshness.notes.len()),
    );
    push_row(
        &mut out,
        "Next actions",
        &format!("{} action(s)", report.receipt_freshness.next_actions.len()),
    );
    push_markdown_list(
        &mut out,
        "Receipt freshness notes",
        &report.receipt_freshness.notes,
    );
    push_markdown_list(
        &mut out,
        "Missing active goal receipts",
        &report.receipt_freshness.missing_active_goal_receipts,
    );
    push_markdown_list(
        &mut out,
        "Missing plan receipts",
        &report.receipt_freshness.missing_plan_receipts,
    );
    push_markdown_bullets(
        &mut out,
        "Receipt freshness next actions",
        &report.receipt_freshness.next_actions,
    );

    out.push_str("\n## Work items\n\n");
    out.push_str("| ID | Status | Proposal | Spec | Plan | Receipts |\n");
    out.push_str("|---|---|---|---|---|---|\n");
    for item in &report.work_items {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            md(&item.id),
            md(&item.status),
            md_opt(&item.proposal),
            md_opt(&item.spec),
            md(&item.plan),
            md(&receipt_summary(&item.receipts))
        ));
    }

    push_work_item_proof_commands(&mut out, &report.work_items);

    out.push_str("\n## Artifacts\n\n");
    out.push_str("| ID | Kind | Status | Owner | Path |\n");
    out.push_str("|---|---|---|---|---|\n");
    for artifact in &report.artifacts {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            md(&artifact.id),
            md(&artifact.kind),
            md(&artifact.status),
            md(&artifact.owner),
            md(&artifact.path)
        ));
    }

    out.push_str("\n## Support tiers\n\n");
    out.push_str("| Surface | Tier | Proof command |\n");
    out.push_str("|---|---|---|\n");
    for claim in &report.support_tiers {
        out.push_str(&format!(
            "| {} | {} | {} |\n",
            md(&claim.surface),
            md(&claim.tier),
            md(&claim.proof_command)
        ));
    }

    out.push_str("\n## Edges\n\n");
    out.push_str("| From | Relation | To |\n");
    out.push_str("|---|---|---|\n");
    for edge in &report.edges {
        out.push_str(&format!(
            "| {} | {} | {} |\n",
            md(&edge.from),
            md(&edge.relation),
            md(&edge.to)
        ));
    }

    out
}

fn push_row(out: &mut String, key: &str, value: &str) {
    out.push_str(&format!("| {} | {} |\n", md(key), md(value)));
}

fn push_markdown_list(out: &mut String, title: &str, values: &[String]) {
    if values.is_empty() {
        return;
    }
    out.push_str(&format!("\n### {title}\n\n"));
    for value in values {
        out.push_str(&format!("- `{}`\n", md(value)));
    }
}

fn push_markdown_list_limited(out: &mut String, title: &str, values: &[String], limit: usize) {
    if values.is_empty() {
        return;
    }

    out.push_str(&format!("\n### {title}\n\n"));
    let shown = values.len().min(limit);
    for value in values.iter().take(shown) {
        out.push_str(&format!("- `{}`\n", md(value)));
    }
    let hidden = values.len().saturating_sub(shown);
    if hidden > 0 {
        out.push_str(&format!(
            "- ... {hidden} earlier item(s) omitted from Markdown; see `graph.json` for the full list.\n"
        ));
    }
}

fn push_markdown_bullets(out: &mut String, title: &str, values: &[String]) {
    if values.is_empty() {
        return;
    }
    out.push_str(&format!("\n### {title}\n\n"));
    for value in values {
        out.push_str(&format!("- {}\n", md(value.trim())));
    }
}

fn push_work_item_proof_commands(out: &mut String, work_items: &[WorkItem]) {
    let items_with_commands = work_items
        .iter()
        .filter(|item| {
            item.commands
                .iter()
                .any(|command| !command.trim().is_empty())
        })
        .collect::<Vec<_>>();
    if items_with_commands.is_empty() {
        return;
    }

    out.push_str("\n## Work item proof commands\n\n");
    for item in items_with_commands {
        out.push_str(&format!("### {}\n\n", md(&item.id)));
        out.push_str("```bash\n");
        for command in &item.commands {
            let command = command.trim();
            if !command.is_empty() {
                out.push_str(command);
                out.push('\n');
            }
        }
        out.push_str("```\n\n");
    }
}

fn table_cells(line: &str) -> Vec<&str> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(str::trim)
        .collect()
}

fn is_table_separator(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with('|')
        && trimmed
            .trim_matches('|')
            .split('|')
            .all(|cell| cell.trim().chars().all(|c| c == '-' || c == ':'))
}

fn join_or_dash(values: &[String]) -> String {
    if values.is_empty() {
        "-".to_string()
    } else {
        values.join(", ")
    }
}

fn receipt_summary(receipts: &[String]) -> String {
    const LIMIT: usize = 12;

    if receipts.is_empty() {
        return "-".to_string();
    }

    let proof_receipts = receipts
        .iter()
        .filter(|receipt| !receipt.contains(" closed as "))
        .collect::<Vec<_>>();
    let summary_source = if proof_receipts.is_empty() {
        receipts.iter().collect::<Vec<_>>()
    } else {
        proof_receipts
    };
    let start = summary_source.len().saturating_sub(LIMIT);
    let selected = summary_source[start..]
        .iter()
        .map(|receipt| receipt.as_str())
        .collect::<Vec<_>>();
    let hidden = receipts.len().saturating_sub(selected.len());
    let mut summary = selected.join(", ");
    if hidden > 0 {
        summary.push_str(&format!(" (+{hidden} earlier/other)"));
    }
    summary
}

fn md_opt(value: &Option<String>) -> String {
    md(value.as_deref().unwrap_or("-"))
}

fn bool_opt(value: &Option<bool>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn md(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

fn display_path(workspace_root: &Path, path: &Path) -> String {
    path.strip_prefix(workspace_root)
        .unwrap_or(path)
        .display()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    #[test]
    fn writes_json_and_markdown_reports() {
        let dir = tempdir().unwrap();
        write(
            &dir.path().join("policy/doc-artifacts.toml"),
            r#"
schema_version = 1
policy = "doc-artifacts"
owner = "repo-infra"
status = "advisory"

[[artifact]]
id = "SHIPLOG-PROP-0008"
kind = "proposal"
path = "docs/proposals/SHIPLOG-PROP-0008-source-of-truth-stack.md"
status = "proposed"
owner = "repo-infra"

[[artifact]]
id = "SHIPLOG-SPEC-0010"
kind = "spec"
path = "docs/specs/SHIPLOG-SPEC-0010-source-of-truth-stack.md"
status = "proposed"
owner = "repo-infra"
linked_proposal = "SHIPLOG-PROP-0008"

[[artifact]]
id = "SHIPLOG-PLAN-0010"
kind = "plan"
path = "plans/0.10.0/implementation-plan.md"
status = "active"
owner = "codex"
linked_proposal = "SHIPLOG-PROP-0008"
linked_spec = "SHIPLOG-SPEC-0010"
"#,
        );
        write(
            &dir.path().join(".codex/goals/active.toml"),
            r#"
id = "shiplog-source-of-truth-stack"
title = "Shiplog source-of-truth stack rollout"
status = "active"
owner = "codex"
created = "2026-05-20"
objective = "Keep repo source-of-truth artifacts linked."
end_state = ["Artifacts are linked."]

[[work_item]]
id = "repo-contract-report"
status = "active"
proposal = "SHIPLOG-PROP-0008"
spec = "SHIPLOG-SPEC-0010"
plan = "plans/0.10.0/implementation-plan.md"
commands = ["rtk cargo xtask repo-contract-report", "rtk git diff --check"]
receipts = [
  "receipt-01",
  "receipt-02",
  "receipt-03 closed as completed by receipt-04",
  "receipt-04",
  "receipt-05",
  "receipt-06",
  "receipt-07",
  "receipt-08",
  "receipt-09",
  "receipt-10",
  "receipt-11",
  "receipt-12",
  "receipt-13",
  "receipt-14",
]
"#,
        );
        write(
            &dir.path().join("docs/status/SUPPORT_TIERS.md"),
            r#"# Support tiers

## Claim map

| Surface | Tier | Claim | Proof command | Notes |
|---|---|---|---|---|
| Repo contract report | Stabilizing | Agents can inspect repo contract state. | `rtk cargo xtask repo-contract-report` | Writes inspection reports. |
"#,
        );

        run(dir.path()).unwrap();

        let graph_json = dir.path().join("target/source-of-truth/graph.json");
        let graph_md = dir.path().join("target/source-of-truth/graph.md");
        assert!(graph_json.is_file());
        assert!(graph_md.is_file());
        let json = fs::read_to_string(graph_json).unwrap();
        assert!(json.contains("\"repo-contract-report\""));
        assert!(json.contains("\"git_topology\""));
        assert!(json.contains("\"local_checkout\""));
        assert!(json.contains("\"local_merged_cleanup_candidates\""));
        assert!(json.contains("\"remote_branch_hygiene\""));
        assert!(json.contains("\"source_merged_cleanup_candidates\""));
        assert!(json.contains("\"swarm_review_cleanup_candidates\""));
        assert!(json.contains("\"source_merged_cleanup_review_commands\""));
        assert!(json.contains("\"remote_queue_hygiene\""));
        assert!(json.contains("\"routed_ci_health\""));
        assert!(json.contains("\"promotion_pr_contract\""));
        assert!(json.contains("\"branch_protection_contract\""));
        assert!(json.contains("\"receipt_freshness\""));
        let markdown = fs::read_to_string(graph_md).unwrap();
        assert!(markdown.contains("# Repo contract report"));
        assert!(markdown.contains("Keep repo source-of-truth artifacts linked."));
        assert!(markdown.contains("### End state"));
        assert!(markdown.contains("Artifacts are linked."));
        assert!(markdown.contains("Repo contract report"));
        assert!(markdown.contains("receipt-14 (+2 earlier/other)"));
        assert!(!markdown.contains("receipt-01"));
        assert!(markdown.contains("## Work item proof commands"));
        assert!(markdown.contains("rtk cargo xtask repo-contract-report"));
        assert!(markdown.contains("## Git topology"));
        assert!(markdown.contains("Git topology next actions"));
        assert!(markdown.contains("## Local checkout"));
        assert!(markdown.contains("Local merged cleanup candidates"));
        assert!(markdown.contains("## Remote branch hygiene"));
        assert!(markdown.contains("Source merged cleanup candidates"));
        assert!(markdown.contains("Swarm review cleanup candidates"));
        assert!(markdown.contains("## Remote queue hygiene"));
        assert!(markdown.contains("## Routed CI health"));
        assert!(markdown.contains("## Promotion PR contract"));
        assert!(markdown.contains("## Branch protection contract"));
        assert!(markdown.contains("## Receipt freshness"));
    }

    #[test]
    fn remote_queue_hygiene_reports_clean_when_queues_are_empty() {
        let report = remote_queue_hygiene_from_parts(
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );

        assert_eq!(report.status, "clean");
        assert!(
            report
                .next_actions
                .iter()
                .any(|action| action.contains("queues are empty"))
        );
    }

    #[test]
    fn remote_queue_hygiene_reports_open_work() {
        let report = remote_queue_hygiene_from_parts(
            vec!["EffortlessMetrics/shiplog#1 source PR".to_string()],
            vec!["EffortlessMetrics/shiplog-swarm#2 swarm PR".to_string()],
            vec!["EffortlessMetrics/shiplog#3 source issue".to_string()],
            vec!["EffortlessMetrics/shiplog-swarm#4 swarm issue".to_string()],
            Vec::new(),
        );

        assert_eq!(report.status, "review-needed");
        assert!(
            report
                .next_actions
                .iter()
                .any(|action| action.contains("1 source PR(s)"))
        );
        assert_eq!(report.source_open_prs.len(), 1);
        assert_eq!(report.swarm_open_issues.len(), 1);
    }

    #[test]
    fn formats_open_pr_and_issue_receipts() {
        let pr = format_github_pr(
            SWARM_REPO,
            &serde_json::json!({
                "number": 118,
                "title": "docs: align claims",
                "isDraft": false,
                "headRefName": "docs/claims",
                "baseRefName": "main"
            }),
        );
        let issue = format_github_issue(
            SOURCE_REPO,
            &serde_json::json!({
                "number": 12,
                "title": "queue cleanup",
                "labels": [{"name": "release"}]
            }),
        );

        assert_eq!(
            pr,
            "EffortlessMetrics/shiplog-swarm#118 docs: align claims [draft=false, head=docs/claims, base=main]"
        );
        assert_eq!(
            issue,
            "EffortlessMetrics/shiplog#12 queue cleanup [labels=release]"
        );
    }

    #[test]
    fn routed_ci_health_reports_green_for_successful_latest_runs() {
        let source = routed_ci_run_from_json(serde_json::json!({
            "databaseId": 26731901009u64,
            "status": "completed",
            "conclusion": "success",
            "displayTitle": "Merge pull request #555",
            "event": "push",
            "headBranch": "main",
            "url": "https://example.test/source-run",
            "createdAt": "2026-06-01T02:24:47Z",
            "updatedAt": "2026-06-01T02:33:18Z"
        }));
        let swarm = routed_ci_run_from_json(serde_json::json!({
            "databaseId": 26731460914u64,
            "status": "completed",
            "conclusion": "success",
            "displayTitle": "xtask: report remote queue hygiene (#119)",
            "event": "push",
            "headBranch": "main",
            "url": "https://example.test/swarm-run",
            "createdAt": "2026-06-01T02:07:39Z",
            "updatedAt": "2026-06-01T02:16:10Z"
        }));

        let report = routed_ci_health_from_parts(Some(source), Some(swarm), Vec::new());

        assert_eq!(report.status, "green");
        assert!(report.source_latest_run.as_ref().unwrap().success);
        assert!(report.swarm_latest_run.as_ref().unwrap().success);
        assert!(
            report
                .next_actions
                .iter()
                .any(|action| action.contains("completed successfully"))
        );
    }

    #[test]
    fn routed_ci_health_reports_attention_for_failed_latest_run() {
        let source = routed_ci_run_from_json(serde_json::json!({
            "databaseId": 1u64,
            "status": "completed",
            "conclusion": "failure",
            "displayTitle": "Merge pull request #555",
            "event": "push",
            "headBranch": "main"
        }));
        let swarm = routed_ci_run_from_json(serde_json::json!({
            "databaseId": 2u64,
            "status": "completed",
            "conclusion": "success",
            "displayTitle": "xtask: report queue hygiene (#119)",
            "event": "push",
            "headBranch": "main"
        }));

        let report = routed_ci_health_from_parts(Some(source), Some(swarm), Vec::new());

        assert_eq!(report.status, "attention-needed");
        assert!(!report.source_latest_run.as_ref().unwrap().success);
        assert!(
            report
                .next_actions
                .iter()
                .any(|action| action.contains("source main"))
        );
    }

    #[test]
    fn routed_ci_health_reports_unavailable_when_gh_inspection_fails() {
        let report = routed_ci_health_from_parts(
            None,
            None,
            vec!["gh run list failed: authentication required".to_string()],
        );

        assert_eq!(report.status, "unavailable");
        assert!(
            report
                .next_actions
                .iter()
                .any(|action| action.contains("rtk gh run list"))
        );
    }

    #[test]
    fn formats_routed_ci_run_summary() {
        let run = routed_ci_run_from_json(serde_json::json!({
            "databaseId": 26731460914u64,
            "status": "completed",
            "conclusion": "success",
            "displayTitle": "xtask: report remote queue hygiene (#119)",
            "event": "push",
            "headBranch": "main"
        }));

        assert_eq!(
            routed_ci_run_summary(&Some(run)),
            "#26731460914 success/completed [push on main] xtask: report remote queue hygiene (#119)"
        );
    }

    #[test]
    fn promotion_pr_contract_reports_aligned_when_body_has_required_receipts() {
        let body = r#"
Promotes shiplog-swarm/main through 491dd34b4f3e2fb1c7588679d6832c09f6257924.

Merge this PR with a regular merge commit; do not squash.

## Included swarm PRs
- EffortlessMetrics/shiplog-swarm#120

## Swarm proof
- Shiplog Rust Small Result passed.
"#;
        let report = promotion_pr_contract_from_json(
            556,
            Some("474bf93ad7f120d173136f474e8e912b08005798".to_string()),
            Some("491dd34b4f3e2fb1c7588679d6832c09f6257924".to_string()),
            serde_json::json!({
                "title": "merge(swarm): promote shiplog-swarm through 491dd34",
                "body": body,
                "state": "MERGED",
                "mergeCommit": {"oid": "474bf93ad7f120d173136f474e8e912b08005798"},
                "url": "https://github.com/EffortlessMetrics/shiplog/pull/556"
            }),
            Vec::new(),
        );

        assert_eq!(report.status, "aligned");
        assert_eq!(
            report.latest_promotion_pr.as_deref(),
            Some("EffortlessMetrics/shiplog#556")
        );
        assert_eq!(report.title_ok, Some(true));
        assert_eq!(report.state_ok, Some(true));
        assert_eq!(report.merge_commit_ok, Some(true));
        assert_eq!(report.body_mentions_swarm_head, Some(true));
        assert_eq!(report.body_mentions_included_swarm_prs, Some(true));
        assert_eq!(report.body_mentions_swarm_proof, Some(true));
        assert_eq!(report.body_mentions_merge_method, Some(true));
        assert!(report.failed_checks.is_empty());
    }

    #[test]
    fn promotion_pr_contract_reports_drift_with_failed_checks_when_body_omits_receipts() {
        let report = promotion_pr_contract_from_json(
            556,
            Some("474bf93ad7f120d173136f474e8e912b08005798".to_string()),
            Some("491dd34b4f3e2fb1c7588679d6832c09f6257924".to_string()),
            serde_json::json!({
                "title": "merge(swarm): promote shiplog-swarm through 491dd34",
                "body": "Promotes shiplog-swarm/main through 491dd34.",
                "state": "MERGED",
                "mergeCommit": {"oid": "474bf93ad7f120d173136f474e8e912b08005798"}
            }),
            Vec::new(),
        );

        assert_eq!(report.status, "drift");
        assert_eq!(report.body_mentions_included_swarm_prs, Some(false));
        assert_eq!(report.body_mentions_swarm_proof, Some(false));
        assert_eq!(report.body_mentions_merge_method, Some(false));
        assert_eq!(
            report.failed_checks,
            vec![
                "body missing `Included swarm PRs` with `EffortlessMetrics/shiplog-swarm#...`",
                "body missing `Swarm proof` with `Shiplog Rust Small Result`",
                "body missing merge-method boundary: include `regular merge commit` and `do not squash`",
            ]
        );
        assert!(
            report
                .next_actions
                .iter()
                .any(|action| action.contains("Included swarm PRs"))
        );
    }

    #[test]
    fn promotion_pr_contract_reports_unavailable_when_gh_fails() {
        let report = promotion_pr_contract_from_parts(PromotionPrContractParts {
            latest_promotion_pr: Some("EffortlessMetrics/shiplog#556".to_string()),
            latest_promotion_url: None,
            expected_title: Some("merge(swarm): promote shiplog-swarm through 491dd34".to_string()),
            actual_title: None,
            state: None,
            merge_commit: None,
            expected_source_head: Some("474bf93ad7f120d173136f474e8e912b08005798".to_string()),
            expected_swarm_head: Some("491dd34b4f3e2fb1c7588679d6832c09f6257924".to_string()),
            notes: vec!["gh pr view failed: authentication required".to_string()],
        });

        assert_eq!(report.status, "unavailable");
        assert!(
            report
                .next_actions
                .iter()
                .any(|action| action.contains("rtk gh pr view"))
        );
    }

    #[test]
    fn branch_protection_contract_accepts_normalized_result_only() {
        let report = branch_protection_contract_from_json(
            vec![SWARM_REQUIRED_CHECK.to_string()],
            serde_json::json!({
                "required_status_checks": {
                    "contexts": [SWARM_REQUIRED_CHECK],
                    "strict": true
                },
                "enforce_admins": { "enabled": false },
                "allow_force_pushes": { "enabled": false },
                "allow_deletions": { "enabled": false }
            }),
        );

        assert_eq!(report.status, "aligned");
        assert_eq!(report.required_status_checks_ok, Some(true));
        assert!(report.disallowed_required_checks.is_empty());
        assert_eq!(report.allow_force_pushes_ok, Some(true));
        assert_eq!(report.allow_deletions_ok, Some(true));
    }

    #[test]
    fn branch_protection_contract_flags_conditional_required_jobs() {
        let report = branch_protection_contract_from_json(
            vec![SWARM_REQUIRED_CHECK.to_string()],
            serde_json::json!({
                "required_status_checks": {
                    "contexts": [
                        SWARM_REQUIRED_CHECK,
                        "Shiplog Rust Small on CX43"
                    ],
                    "strict": true
                },
                "enforce_admins": { "enabled": false },
                "allow_force_pushes": { "enabled": false },
                "allow_deletions": { "enabled": false }
            }),
        );

        assert_eq!(report.status, "drift");
        assert_eq!(report.required_status_checks_ok, Some(false));
        assert_eq!(
            report.disallowed_required_checks,
            vec!["Shiplog Rust Small on CX43"]
        );
        assert!(
            report
                .next_actions
                .iter()
                .any(|action| action.contains("only required status check"))
        );
    }

    #[test]
    fn limits_long_markdown_lists_without_losing_latest_items() {
        let values = (1..=14)
            .map(|index| format!("commit-{index:02}"))
            .collect::<Vec<_>>();
        let mut markdown = String::new();

        push_markdown_list_limited(&mut markdown, "Source promotion merge commits", &values, 12);

        assert!(markdown.contains("commit-01"));
        assert!(markdown.contains("commit-12"));
        assert!(!markdown.contains("commit-13"));
        assert!(markdown.contains("2 earlier item(s) omitted"));
        assert!(markdown.contains("graph.json"));
    }

    #[test]
    fn classifies_source_ahead_promotion_merges() {
        let commits = vec![
            "84485cc Merge pull request #499 from EffortlessMetrics/promote/swarm-20260522-8c50281"
                .to_string(),
            "ecdd4d9 Merge pull request #498 from EffortlessMetrics/promote/swarm-20260522-cbcd866"
                .to_string(),
        ];

        let summary = classify_source_ahead(&commits);

        assert_eq!(summary.classification, "promotion-merge-only");
        assert_eq!(summary.promotion_merges, commits);
        assert!(summary.other_commits.is_empty());
    }

    #[test]
    fn classifies_explicit_source_promotion_subjects() {
        let commits =
            vec!["1a35a90 merge(swarm): promote shiplog-swarm through f4fc2d5".to_string()];

        let summary = classify_source_ahead(&commits);

        assert_eq!(summary.classification, "promotion-merge-only");
        assert_eq!(summary.promotion_merges, commits);
        assert!(summary.other_commits.is_empty());
    }

    #[test]
    fn classifies_source_ahead_mixed_commits() {
        let promotion =
            "84485cc Merge pull request #499 from EffortlessMetrics/promote/swarm-20260522-8c50281"
                .to_string();
        let source_only = "abc1234 docs: source-only release note".to_string();
        let commits = vec![promotion.clone(), source_only.clone()];

        let summary = classify_source_ahead(&commits);

        assert_eq!(summary.classification, "mixed");
        assert_eq!(summary.promotion_merges, vec![promotion]);
        assert_eq!(summary.other_commits, vec![source_only]);
    }

    #[test]
    fn next_actions_continue_when_tree_aligned_after_promotions() {
        let actions =
            topology_next_actions("tree-aligned", Some(true), "promotion-merge-only", &[], &[]);

        assert_eq!(
            actions,
            vec![
                "Continue normal development in `EffortlessMetrics/shiplog-swarm`; no source promotion is pending."
            ]
        );
    }

    #[test]
    fn next_actions_promote_when_swarm_is_ahead() {
        let actions = topology_next_actions(
            "swarm-ahead",
            Some(false),
            "none",
            &[],
            &["abc1234 docs: next swarm change".to_string()],
        );

        assert!(
            actions
                .iter()
                .any(|action| { action.contains("Open a source promotion PR from `swarm/main`") })
        );
    }

    #[test]
    fn next_actions_pause_on_source_only_drift() {
        let actions = topology_next_actions(
            "source-ahead",
            Some(false),
            "non-promotion",
            &["abc1234 docs: source-only release note".to_string()],
            &[],
        );

        assert!(
            actions
                .iter()
                .any(|action| action.contains("source-only non-promotion commits"))
        );
    }

    #[test]
    fn local_checkout_reports_clean_branch() {
        let report =
            local_checkout_from_status_lines(vec!["## main...origin/main".to_string()], Vec::new());

        assert_eq!(report.status, "clean");
        assert_eq!(report.branch_summary.as_deref(), Some("main...origin/main"));
        assert_eq!(report.clean, Some(true));
        assert!(report.status_entries.is_empty());
        assert!(report.local_merged_cleanup_candidates.is_empty());
        assert!(
            report
                .next_actions
                .iter()
                .any(|action| action.contains("Local checkout is clean"))
        );
    }

    #[test]
    fn local_checkout_reports_dirty_entries() {
        let report = local_checkout_from_status_lines(
            vec![
                "## xtask/report-local-checkout...swarm/main".to_string(),
                " M xtask/src/tasks/repo_contract_report.rs".to_string(),
                "?? scratch.txt".to_string(),
            ],
            Vec::new(),
        );

        assert_eq!(report.status, "dirty");
        assert_eq!(report.clean, Some(false));
        assert_eq!(report.status_entries.len(), 2);
        assert!(
            report
                .next_actions
                .iter()
                .any(|action| action.contains("rtk git status"))
        );
    }

    #[test]
    fn local_checkout_reports_merged_local_branch_candidates() {
        let report = local_checkout_from_status_and_branch_lines(
            vec!["## codex/current...swarm/main".to_string()],
            vec![
                "codex/current".to_string(),
                "codex/done".to_string(),
                "main".to_string(),
                "release/v0.9.0".to_string(),
            ],
            vec!["main".to_string(), "codex/done".to_string()],
            vec!["codex/current".to_string()],
            Vec::new(),
        );

        assert_eq!(report.status, "review-needed");
        assert_eq!(report.clean, Some(true));
        assert_eq!(report.local_branch_count, 4);
        assert_eq!(report.local_merged_cleanup_candidates, vec!["codex/done"]);
        assert_eq!(
            report.protected_local_branches,
            vec!["main", "release/v0.9.0"]
        );
        assert_eq!(
            report.local_merged_cleanup_review_commands,
            vec![
                "rtk gh pr list --repo EffortlessMetrics/shiplog-swarm --state all --head codex/done --limit 10 && rtk gh pr list --repo EffortlessMetrics/shiplog --state all --head codex/done --limit 10 && rtk git log --oneline --max-count 3 codex/done"
            ]
        );
        assert!(
            report
                .next_actions
                .iter()
                .any(|action| { action.contains("Review 1 local branch(es) already merged") })
        );
    }

    #[test]
    fn local_checkout_reports_unavailable_without_branch() {
        let report = local_checkout_from_status_lines(
            Vec::new(),
            vec!["git status --short --branch failed".to_string()],
        );

        assert_eq!(report.status, "unavailable");
        assert_eq!(report.clean, None);
        assert_eq!(report.notes.len(), 1);
        assert!(
            report
                .next_actions
                .iter()
                .any(|action| action.contains("Git checkout"))
        );
    }

    #[test]
    fn remote_branch_hygiene_keeps_release_branches_protected() {
        let report = remote_branch_hygiene_from_lines(
            vec![
                "origin".to_string(),
                "origin/main".to_string(),
                "origin/release/v0.9.0".to_string(),
                "origin/promote/swarm-20260531-1046ae2".to_string(),
                "origin/feat/stale-source-work".to_string(),
                "swarm".to_string(),
                "swarm/main".to_string(),
                "swarm/codex/stale-agent-branch".to_string(),
            ],
            Vec::new(),
            "origin",
            "swarm",
        );

        assert_eq!(report.status, "review-needed");
        assert_eq!(
            report.protected_branches,
            vec!["origin/main", "origin/release/v0.9.0", "swarm/main"]
        );
        assert_eq!(
            report.source_cleanup_candidates,
            vec![
                "origin/feat/stale-source-work",
                "origin/promote/swarm-20260531-1046ae2",
            ]
        );
        assert!(report.source_merged_cleanup_candidates.is_empty());
        assert_eq!(
            report.source_review_cleanup_candidates,
            report.source_cleanup_candidates
        );
        assert_eq!(
            report.source_review_cleanup_review_commands,
            vec![
                "rtk gh pr list --repo EffortlessMetrics/shiplog --state all --head feat/stale-source-work --limit 10 && rtk git log --oneline --max-count 3 origin/feat/stale-source-work",
                "rtk gh pr list --repo EffortlessMetrics/shiplog --state all --head promote/swarm-20260531-1046ae2 --limit 10 && rtk git log --oneline --max-count 3 origin/promote/swarm-20260531-1046ae2"
            ]
        );
        assert_eq!(
            report.swarm_cleanup_candidates,
            vec!["swarm/codex/stale-agent-branch"]
        );
        assert!(report.swarm_merged_cleanup_candidates.is_empty());
        assert_eq!(
            report.swarm_review_cleanup_candidates,
            report.swarm_cleanup_candidates
        );
        assert_eq!(
            report.swarm_review_cleanup_review_commands,
            vec![
                "rtk gh pr list --repo EffortlessMetrics/shiplog-swarm --state all --head codex/stale-agent-branch --limit 10 && rtk git log --oneline --max-count 3 swarm/codex/stale-agent-branch"
            ]
        );
        assert!(
            report
                .next_actions
                .iter()
                .any(|action| action.contains("preserved follow-up value"))
        );
    }

    #[test]
    fn remote_branch_hygiene_splits_merged_and_review_candidates() {
        let report = remote_branch_hygiene_from_lines_with_merged(
            vec![
                "origin/main".to_string(),
                "origin/promote/swarm-20260531-1046ae2".to_string(),
                "origin/feat/stale-source-work".to_string(),
                "swarm/main".to_string(),
                "swarm/codex/stale-agent-branch".to_string(),
                "swarm/codex/unmerged-agent-branch".to_string(),
            ],
            vec!["origin/promote/swarm-20260531-1046ae2".to_string()],
            vec!["swarm/codex/stale-agent-branch".to_string()],
            Vec::new(),
            "origin",
            "swarm",
            BTreeSet::new(),
            BTreeSet::new(),
        );

        assert_eq!(
            report.source_merged_cleanup_candidates,
            vec!["origin/promote/swarm-20260531-1046ae2"]
        );
        assert_eq!(
            report.source_review_cleanup_candidates,
            vec!["origin/feat/stale-source-work"]
        );
        assert_eq!(
            report.swarm_merged_cleanup_candidates,
            vec!["swarm/codex/stale-agent-branch"]
        );
        assert_eq!(
            report.swarm_review_cleanup_candidates,
            vec!["swarm/codex/unmerged-agent-branch"]
        );
        assert_eq!(
            report.source_merged_cleanup_review_commands,
            vec![
                "rtk gh pr list --repo EffortlessMetrics/shiplog --state all --head promote/swarm-20260531-1046ae2 --limit 10 && rtk git log --oneline --max-count 3 origin/promote/swarm-20260531-1046ae2"
            ]
        );
        assert_eq!(
            report.source_review_cleanup_review_commands,
            vec![
                "rtk gh pr list --repo EffortlessMetrics/shiplog --state all --head feat/stale-source-work --limit 10 && rtk git log --oneline --max-count 3 origin/feat/stale-source-work"
            ]
        );
        assert_eq!(
            report.swarm_merged_cleanup_review_commands,
            vec![
                "rtk gh pr list --repo EffortlessMetrics/shiplog-swarm --state all --head codex/stale-agent-branch --limit 10 && rtk git log --oneline --max-count 3 swarm/codex/stale-agent-branch"
            ]
        );
        assert_eq!(
            report.swarm_review_cleanup_review_commands,
            vec![
                "rtk gh pr list --repo EffortlessMetrics/shiplog-swarm --state all --head codex/unmerged-agent-branch --limit 10 && rtk git log --oneline --max-count 3 swarm/codex/unmerged-agent-branch"
            ]
        );
        assert!(report.next_actions.iter().any(|action| {
            action.contains("start with 1 source and 1 swarm candidate(s) already merged")
        }));
        assert!(
            report.next_actions.iter().any(|action| {
                action.contains("inspect 1 source and 1 swarm unmerged candidate")
            })
        );
    }

    #[test]
    fn remote_branch_hygiene_excludes_open_pr_heads_from_cleanup_candidates() {
        let report = remote_branch_hygiene_from_lines_with_merged(
            vec![
                "origin/main".to_string(),
                "swarm/main".to_string(),
                "swarm/codex/open-agent-branch".to_string(),
                "swarm/codex/stale-agent-branch".to_string(),
            ],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            "origin",
            "swarm",
            BTreeSet::new(),
            BTreeSet::from(["swarm/codex/open-agent-branch".to_string()]),
        );

        assert_eq!(
            report.swarm_open_pr_branches,
            vec!["swarm/codex/open-agent-branch"]
        );
        assert_eq!(
            report.swarm_cleanup_candidates,
            vec!["swarm/codex/stale-agent-branch"]
        );
        assert_eq!(
            report.swarm_review_cleanup_candidates,
            vec!["swarm/codex/stale-agent-branch"]
        );
    }

    #[test]
    fn remote_branch_hygiene_reports_clean_when_only_protected_refs_exist() {
        let report = remote_branch_hygiene_from_lines(
            vec![
                "origin".to_string(),
                "origin/main".to_string(),
                "origin/release/v0.9.0".to_string(),
                "swarm".to_string(),
                "swarm/main".to_string(),
            ],
            Vec::new(),
            "origin",
            "swarm",
        );

        assert_eq!(report.status, "clean");
        assert!(report.source_cleanup_candidates.is_empty());
        assert!(report.swarm_cleanup_candidates.is_empty());
    }

    #[test]
    fn extracts_source_promotion_pr_from_merge_subject() {
        let commit =
            "e4ac1c7 Merge pull request #519 from EffortlessMetrics/promote/swarm-20260523-6147ab3";

        assert_eq!(extract_merge_pull_request_number(commit), Some(519));
    }

    #[test]
    fn extracts_swarm_pr_from_squash_subject() {
        let commit = "6147ab3 xtask: report local checkout state (#76)";

        assert_eq!(extract_parenthesized_pull_request_number(commit), Some(76));
    }

    #[test]
    fn receipt_freshness_next_actions_explain_stale_receipts() {
        let actions = receipt_freshness_next_actions("stale");

        assert!(
            actions
                .iter()
                .any(|action| action.contains(".codex/goals/active.toml"))
        );
        assert!(
            actions
                .iter()
                .any(|action| action.contains("plans/shiplog-swarm/implementation-plan.md"))
        );
    }

    #[test]
    fn receipt_freshness_defers_self_referential_receipt_refreshes() {
        let status = receipt_freshness_status(
            &[true, false, true, false],
            Some("37ad2c5 docs(swarm): refresh promotion receipts (#88)"),
        );

        assert_eq!(status, "pending-next-substantive-pr");
        let actions = receipt_freshness_next_actions(status);
        assert!(
            actions
                .iter()
                .any(|action| action.contains("receipt-only loop"))
        );
    }

    #[test]
    fn receipt_freshness_defers_scoped_promotion_receipt_refreshes() {
        let status = receipt_freshness_status(
            &[true, false, true, false],
            Some("b046873 docs(swarm): refresh native-deps promotion receipts (#104)"),
        );

        assert_eq!(status, "pending-next-substantive-pr");
    }

    #[test]
    fn receipt_freshness_defers_scoped_receipt_refreshes_without_promotion_word() {
        let status = receipt_freshness_status(
            &[true, false, true, false],
            Some("e86bcde docs(swarm): refresh branch cleanup receipts (#114)"),
        );

        assert_eq!(status, "pending-next-substantive-pr");
    }

    #[test]
    fn receipt_freshness_keeps_substantive_missing_receipts_stale() {
        let status = receipt_freshness_status(
            &[true, false, true, false],
            Some("ae20816 ci: allow hosted fallback on non-PR routes (#87)"),
        );

        assert_eq!(status, "stale");
    }

    #[test]
    fn push_missing_receipt_records_only_known_absent_receipts() {
        let mut missing = Vec::new();

        push_missing_receipt(
            &mut missing,
            Some("EffortlessMetrics/shiplog#520"),
            Some(false),
        );
        push_missing_receipt(
            &mut missing,
            Some("EffortlessMetrics/shiplog-swarm#77"),
            Some(true),
        );
        push_missing_receipt(&mut missing, None, Some(false));
        push_missing_receipt(&mut missing, Some("unknown"), None);

        assert_eq!(missing, vec!["EffortlessMetrics/shiplog#520"]);
    }
}
