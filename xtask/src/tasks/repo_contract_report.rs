//! `cargo xtask repo-contract-report`
//!
//! Writes a compact source-of-truth graph report for humans and agents.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;

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
    artifacts: Vec<Artifact>,
    work_items: Vec<WorkItem>,
    support_tiers: Vec<SupportTierClaim>,
    edges: Vec<GraphEdge>,
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
    swarm_ahead: Vec<String>,
    status: String,
    notes: Vec<String>,
}

pub fn run(workspace_root: &Path) -> Result<()> {
    let artifacts = load_doc_artifacts(workspace_root)?;
    let goal = load_active_goal(workspace_root)?;
    let support_tiers = load_support_tiers(workspace_root)?;
    let git_topology = inspect_git_topology(workspace_root);

    let output_dir = workspace_root.join("target").join("source-of-truth");
    fs::create_dir_all(&output_dir).with_context(|| format!("create {}", output_dir.display()))?;

    let graph_json = output_dir.join("graph.json");
    let graph_md = output_dir.join("graph.md");

    let report = build_report(
        &artifacts,
        goal,
        support_tiers,
        git_topology,
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
        "repo-contract-report: wrote {} and {}",
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
    git_topology: GitTopologyReport,
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
        generated_by: "cargo xtask repo-contract-report".to_string(),
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
        git_topology,
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
        &[
            "log",
            "--oneline",
            "--max-count=20",
            &format!("{SWARM_REF}..{SOURCE_REF}"),
        ],
        &mut notes,
    );
    let swarm_ahead = git_lines(
        workspace_root,
        &[
            "log",
            "--oneline",
            "--max-count=20",
            &format!("{SOURCE_REF}..{SWARM_REF}"),
        ],
        &mut notes,
    );
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

    GitTopologyReport {
        source_ref: SOURCE_REF.to_string(),
        swarm_ref: SWARM_REF.to_string(),
        source_head,
        swarm_head,
        merge_base,
        trees_aligned,
        source_ahead,
        swarm_ahead,
        status,
        notes,
    }
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
    out.push_str("# Source-of-truth graph\n\n");
    out.push_str("Generated by `cargo xtask repo-contract-report`.\n\n");

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
        "Swarm ahead",
        &format!("{} commit(s)", report.git_topology.swarm_ahead.len()),
    );
    push_row(
        &mut out,
        "Notes",
        &format!("{} note(s)", report.git_topology.notes.len()),
    );
    push_markdown_list(
        &mut out,
        "Source ahead commits",
        &report.git_topology.source_ahead,
    );
    push_markdown_list(
        &mut out,
        "Swarm ahead commits",
        &report.git_topology.swarm_ahead,
    );
    push_markdown_list(&mut out, "Git topology notes", &report.git_topology.notes);

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
            md(&join_or_dash(&item.receipts))
        ));
    }

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

fn md_opt(value: &Option<String>) -> String {
    md(value.as_deref().unwrap_or("-"))
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
commands = ["cargo xtask repo-contract-report", "git diff --check"]
"#,
        );
        write(
            &dir.path().join("docs/status/SUPPORT_TIERS.md"),
            r#"# Support tiers

## Claim map

| Surface | Tier | Claim | Proof command | Notes |
|---|---|---|---|---|
| Repo contract report | Stabilizing | Agents can inspect the graph. | `cargo xtask repo-contract-report` | Writes graph reports. |
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
        let markdown = fs::read_to_string(graph_md).unwrap();
        assert!(markdown.contains("# Source-of-truth graph"));
        assert!(markdown.contains("Repo contract report"));
        assert!(markdown.contains("## Git topology"));
    }
}
