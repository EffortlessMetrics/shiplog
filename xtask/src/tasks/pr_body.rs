//! `cargo xtask pr-body`
//!
//! Generates a draft pull request body from the active source-of-truth work
//! item. This is a derived artifact only; it does not create a PR or mutate
//! source documentation.

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

pub struct PrBodyInputs {
    pub workspace_root: PathBuf,
    pub work_item: String,
    pub output: PathBuf,
}

#[derive(Debug, Deserialize)]
struct ActiveGoal {
    #[serde(default)]
    work_item: Vec<WorkItem>,
}

#[derive(Debug, Deserialize)]
struct WorkItem {
    id: String,
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
}

#[derive(Debug, Deserialize)]
struct DocArtifactsPolicy {
    #[serde(default)]
    artifact: Vec<Artifact>,
}

#[derive(Clone, Debug, Deserialize)]
struct Artifact {
    id: String,
    kind: String,
    path: String,
    #[serde(default)]
    policy_impact: Vec<String>,
}

#[derive(Debug)]
struct PlanItem {
    metadata: BTreeMap<String, String>,
    sections: BTreeMap<String, String>,
}

#[derive(Debug, Default)]
struct SpecImpact {
    support_tier: Option<String>,
    policy: Vec<String>,
}

pub fn run(inputs: PrBodyInputs) -> Result<()> {
    let goal = load_active_goal(&inputs.workspace_root)?;
    let artifacts = load_doc_artifacts(&inputs.workspace_root)?;
    let artifact_by_id = artifacts
        .iter()
        .map(|artifact| (artifact.id.as_str(), artifact))
        .collect::<BTreeMap<_, _>>();

    let work_item = goal
        .work_item
        .iter()
        .find(|item| item.id == inputs.work_item)
        .with_context(|| format!("pr-body: missing work item {:?}", inputs.work_item))?;

    if work_item.plan.trim().is_empty() {
        bail!("pr-body: work item {:?} has no plan path", work_item.id);
    }
    if !relative_repo_path(&work_item.plan) {
        bail!(
            "pr-body: work item {:?} plan {:?} must be a relative repo path",
            work_item.id,
            work_item.plan
        );
    }

    let plan_path = inputs.workspace_root.join(&work_item.plan);
    let plan_text =
        fs::read_to_string(&plan_path).with_context(|| format!("read {}", plan_path.display()))?;
    let plan_section = extract_work_item_section(&plan_text, &work_item.id).with_context(|| {
        format!(
            "pr-body: work item {:?} is not listed in {:?}",
            work_item.id, work_item.plan
        )
    })?;
    let plan_item = parse_plan_item(&plan_section);

    let proposal = resolve_optional_artifact(
        &artifact_by_id,
        work_item.proposal.as_deref(),
        "proposal",
        "proposal",
    )?;
    let spec =
        resolve_optional_artifact(&artifact_by_id, work_item.spec.as_deref(), "spec", "spec")?;
    let adr = resolve_optional_artifact(&artifact_by_id, work_item.adr.as_deref(), "adr", "ADR")?;

    let spec_impact = match spec {
        Some(spec_artifact) => {
            let spec_path = inputs.workspace_root.join(&spec_artifact.path);
            let spec_text = fs::read_to_string(&spec_path)
                .with_context(|| format!("read {}", spec_path.display()))?;
            parse_spec_impact(&spec_text)
        }
        None => SpecImpact::default(),
    };

    let policy_impact = merged_policy_impact(&spec_impact, spec);
    let output_path = resolve_output_path(&inputs.workspace_root, &inputs.output);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }

    let markdown = render_pr_body(
        work_item,
        &plan_item,
        proposal,
        spec,
        adr,
        &spec_impact,
        &policy_impact,
    );
    fs::write(&output_path, markdown)
        .with_context(|| format!("write {}", output_path.display()))?;

    println!(
        "pr-body: wrote {}",
        display_path(&inputs.workspace_root, &output_path)
    );
    Ok(())
}

fn load_active_goal(workspace_root: &Path) -> Result<ActiveGoal> {
    let path = workspace_root
        .join(".codex")
        .join("goals")
        .join("active.toml");
    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("parse {}", path.display()))
}

fn load_doc_artifacts(workspace_root: &Path) -> Result<Vec<Artifact>> {
    let path = workspace_root.join("policy").join("doc-artifacts.toml");
    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let policy: DocArtifactsPolicy =
        toml::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    Ok(policy.artifact)
}

fn resolve_optional_artifact<'a>(
    artifact_by_id: &'a BTreeMap<&str, &'a Artifact>,
    id: Option<&str>,
    expected_kind: &str,
    label: &str,
) -> Result<Option<&'a Artifact>> {
    let Some(id) = non_empty_non_none(id) else {
        return Ok(None);
    };
    let artifact = artifact_by_id
        .get(id)
        .copied()
        .with_context(|| format!("pr-body: {label} artifact {id:?} is not ledgered"))?;
    if artifact.kind != expected_kind {
        bail!(
            "pr-body: {label} artifact {:?} has kind {:?}, expected {:?}",
            artifact.id,
            artifact.kind,
            expected_kind
        );
    }
    Ok(Some(artifact))
}

fn extract_work_item_section(plan_text: &str, work_item_id: &str) -> Option<String> {
    let heading = format!("## Work item: {work_item_id}");
    let mut in_section = false;
    let mut lines = Vec::new();

    for line in plan_text.lines() {
        if line.trim() == heading {
            in_section = true;
            continue;
        }
        if in_section && line.starts_with("## Work item: ") {
            break;
        }
        if in_section {
            lines.push(line);
        }
    }

    in_section.then(|| lines.join("\n"))
}

fn parse_plan_item(section: &str) -> PlanItem {
    let mut metadata = BTreeMap::new();
    let mut sections = BTreeMap::new();
    let mut current_section: Option<String> = None;
    let mut current_lines = Vec::new();

    for line in section.lines() {
        if let Some(name) = line.strip_prefix("### ") {
            if let Some(section_name) = current_section.replace(name.trim().to_string()) {
                sections.insert(section_name, current_lines.join("\n").trim().to_string());
                current_lines.clear();
            }
            continue;
        }

        if current_section.is_some() {
            current_lines.push(line);
        } else if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim();
            if !key.is_empty() {
                metadata.insert(key.to_string(), value.to_string());
            }
        }
    }

    if let Some(section_name) = current_section {
        sections.insert(section_name, current_lines.join("\n").trim().to_string());
    }

    PlanItem { metadata, sections }
}

fn parse_spec_impact(spec_text: &str) -> SpecImpact {
    let mut impact = SpecImpact::default();
    let mut lines = spec_text.lines().peekable();

    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("Support-tier impact:") {
            let value = value.trim();
            if !value.is_empty() {
                impact.support_tier = Some(value.to_string());
            }
            continue;
        }
        if trimmed == "Policy impact:" {
            while let Some(next) = lines.peek() {
                let next = next.trim();
                if next.is_empty() || next.starts_with("## ") {
                    break;
                }
                if let Some(item) = next.strip_prefix("- ") {
                    let item = item.trim();
                    if !item.is_empty() {
                        impact.policy.push(item.to_string());
                    }
                }
                lines.next();
            }
        }
    }

    impact
}

fn merged_policy_impact(spec_impact: &SpecImpact, spec: Option<&Artifact>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();

    for value in &spec_impact.policy {
        if seen.insert(value.as_str()) {
            out.push(value.clone());
        }
    }
    if let Some(spec) = spec {
        for value in &spec.policy_impact {
            if seen.insert(value.as_str()) {
                out.push(value.clone());
            }
        }
    }

    out
}

fn render_pr_body(
    work_item: &WorkItem,
    plan_item: &PlanItem,
    proposal: Option<&Artifact>,
    spec: Option<&Artifact>,
    adr: Option<&Artifact>,
    spec_impact: &SpecImpact,
    policy_impact: &[String],
) -> String {
    let mut out = String::new();
    out.push_str("## Summary\n\n");
    out.push_str(section_or_fallback(
        plan_item,
        "Goal",
        "Generated from the linked source-of-truth work item.",
    ));
    out.push_str("\n\n");

    out.push_str("## Links\n\n");
    out.push_str(&format!(
        "Proposal: {}\n",
        artifact_link(proposal, work_item.proposal.as_deref())
    ));
    out.push_str(&format!(
        "Spec: {}\n",
        artifact_link(spec, work_item.spec.as_deref())
    ));
    out.push_str(&format!(
        "ADR: {}\n",
        artifact_link(adr, work_item.adr.as_deref())
    ));
    out.push_str(&format!(
        "Plan item: `{}` in `{}`\n",
        work_item.id, work_item.plan
    ));
    if let Some(branch) = plan_item.metadata.get("Branch").filter(|s| !s.is_empty()) {
        out.push_str(&format!("Branch: `{branch}`\n"));
    }
    out.push('\n');

    push_named_section(
        &mut out,
        "Scope",
        section_or_first_fallback(
            plan_item,
            &["Production delta", "Goal"],
            "No scope recorded.",
        ),
    );
    push_named_section(
        &mut out,
        "Non-goals",
        section_or_first_fallback(
            plan_item,
            &["Non-goals", "Claim boundary"],
            "No non-goals recorded.",
        ),
    );
    push_named_section(
        &mut out,
        "Acceptance",
        section_or_fallback(plan_item, "Acceptance", "No acceptance criteria recorded."),
    );

    out.push_str("## Support-tier impact\n\n");
    match spec_impact.support_tier.as_deref() {
        Some(value) => out.push_str(&format!("{value}\n\n")),
        None => out.push_str("none recorded\n\n"),
    }

    out.push_str("## Policy impact\n\n");
    if policy_impact.is_empty() {
        out.push_str("none recorded\n\n");
    } else {
        for policy in policy_impact {
            out.push_str(&format!("- `{policy}`\n"));
        }
        out.push('\n');
    }

    push_receipt_refs(&mut out, work_item);

    out.push_str("## Proof\n\n");
    if !work_item.commands.is_empty() {
        out.push_str("```bash\n");
        for command in &work_item.commands {
            let command = command.trim();
            if !command.is_empty() {
                out.push_str(command);
                out.push('\n');
            }
        }
        out.push_str("```\n\n");
    } else {
        let proof = section_or_fallback(plan_item, "Proof commands", "No proof commands recorded.");
        if proof.trim_start().starts_with("```") {
            out.push_str(proof.trim());
            out.push_str("\n\n");
        } else {
            out.push_str("```bash\n");
            out.push_str(proof.trim());
            out.push_str("\n```\n\n");
        }
    }

    push_named_section(
        &mut out,
        "Claim boundary",
        section_or_fallback(plan_item, "Claim boundary", "No claim boundary recorded."),
    );
    push_named_section(
        &mut out,
        "Rollback",
        section_or_fallback(plan_item, "Rollback", "No rollback recorded."),
    );

    out
}

fn push_receipt_refs(out: &mut String, work_item: &WorkItem) {
    const MAX_RECEIPTS: usize = 12;

    let receipts = work_item
        .receipts
        .iter()
        .map(|receipt| receipt.trim())
        .filter(|receipt| !receipt.is_empty())
        .collect::<Vec<_>>();
    if receipts.is_empty() {
        return;
    }
    let actionable_receipts = receipts
        .iter()
        .copied()
        .filter(|receipt| is_compact_pr_receipt_ref(receipt))
        .collect::<Vec<_>>();
    let display_receipts = if actionable_receipts.is_empty() {
        receipts.as_slice()
    } else {
        actionable_receipts.as_slice()
    };

    out.push_str("## Existing receipts\n\n");
    if actionable_receipts.is_empty() && receipts.len() > MAX_RECEIPTS {
        out.push_str(&format!(
            "Showing last {MAX_RECEIPTS} of {} recorded receipt refs in manifest order.\n\n",
            receipts.len()
        ));
    } else if !actionable_receipts.is_empty() && actionable_receipts.len() > MAX_RECEIPTS {
        out.push_str(&format!(
            "Showing last {MAX_RECEIPTS} of {} compact PR receipt refs ({} total recorded receipts).\n\n",
            actionable_receipts.len(),
            receipts.len()
        ));
    } else if !actionable_receipts.is_empty() && actionable_receipts.len() < receipts.len() {
        out.push_str(&format!(
            "Showing {} compact PR receipt refs from {} total recorded receipts.\n\n",
            actionable_receipts.len(),
            receipts.len()
        ));
    }

    let start = display_receipts.len().saturating_sub(MAX_RECEIPTS);
    for receipt in display_receipts.iter().skip(start) {
        out.push_str(&format!("- `{receipt}`\n"));
    }
    out.push('\n');
}

fn is_compact_pr_receipt_ref(receipt: &str) -> bool {
    let receipt = receipt.trim();
    !receipt.contains(char::is_whitespace)
        && receipt.contains('/')
        && receipt.rsplit_once('#').is_some_and(|(_, number)| {
            !number.is_empty() && number.chars().all(|c| c.is_ascii_digit())
        })
}

fn push_named_section(out: &mut String, title: &str, content: &str) {
    out.push_str(&format!("## {title}\n\n"));
    out.push_str(content.trim());
    out.push_str("\n\n");
}

fn section_or_fallback<'a>(plan_item: &'a PlanItem, name: &str, fallback: &'a str) -> &'a str {
    section_or_first_fallback(plan_item, &[name], fallback)
}

fn section_or_first_fallback<'a>(
    plan_item: &'a PlanItem,
    names: &[&str],
    fallback: &'a str,
) -> &'a str {
    for name in names {
        if let Some(value) = plan_item
            .sections
            .get(*name)
            .map(String::as_str)
            .filter(|value| !value.trim().is_empty())
        {
            return value;
        }
    }

    fallback
}

fn artifact_link(artifact: Option<&Artifact>, fallback_id: Option<&str>) -> String {
    if let Some(artifact) = artifact {
        format!("`{}` (`{}`)", artifact.id, artifact.path)
    } else {
        match non_empty_non_none(fallback_id) {
            Some(id) => format!("`{id}` (not resolved)"),
            None => "none".to_string(),
        }
    }
}

fn resolve_output_path(workspace_root: &Path, output: &Path) -> PathBuf {
    if output.is_absolute() {
        output.to_path_buf()
    } else {
        workspace_root.join(output)
    }
}

fn display_path(workspace_root: &Path, path: &Path) -> String {
    path.strip_prefix(workspace_root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn relative_repo_path(path: &str) -> bool {
    let path = Path::new(path);
    !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn non_empty_non_none(value: Option<&str>) -> Option<&str> {
    value.and_then(|s| {
        let s = s.trim();
        if s.is_empty() || s == "none" {
            None
        } else {
            Some(s)
        }
    })
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

    fn fixture_workspace() -> tempfile::TempDir {
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
policy_impact = ["policy/doc-artifacts.toml"]

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
schema_version = 1
id = "shiplog-source-of-truth-stack"
title = "Shiplog source-of-truth stack rollout"
status = "active"
owner = "codex"
created = "2026-05-20"
objective = "Keep repo source-of-truth artifacts linked."
end_state = ["Artifacts are linked."]

[[work_item]]
id = "pr-body-generator"
status = "active"
proposal = "SHIPLOG-PROP-0008"
spec = "SHIPLOG-SPEC-0010"
plan = "plans/0.10.0/implementation-plan.md"
commands = ["rtk cargo xtask pr-body --work-item pr-body-generator", "rtk git diff --check"]
receipts = ["EffortlessMetrics/shiplog-swarm#36", "EffortlessMetrics/shiplog#479"]
"#,
        );
        write(
            &dir.path().join("plans/0.10.0/implementation-plan.md"),
            r#"# 0.10.0 Source-of-Truth Rollout Plan

Plan artifact: SHIPLOG-PLAN-0010

## Work item: pr-body-generator

Status: active
Linked proposal: SHIPLOG-PROP-0008
Linked spec: SHIPLOG-SPEC-0010
Linked ADR: none
Branch: infra/pr-body-generator

### Goal

Add a repo-native PR body generator.

### Production delta

`xtask` command, tests, and docs.

### Non-goals

No GitHub API calls and no PR creation.

### Acceptance

The generated body includes links, proof, and receipt refs.

### Proof commands

```bash
cargo xtask stale-plan-proof
git stale-plan-proof
```

### Rollback

Revert the generator PR.

### Claim boundary

This generates drafts only.
"#,
        );
        write(
            &dir.path()
                .join("docs/specs/SHIPLOG-SPEC-0010-source-of-truth-stack.md"),
            r#"# SHIPLOG-SPEC-0010: Source-of-truth stack contract

Support-tier impact: stabilizing
Policy impact:
- policy/doc-artifacts.toml
- .codex/goals/active.toml

## Problem
"#,
        );
        write(
            &dir.path()
                .join("docs/proposals/SHIPLOG-PROP-0008-source-of-truth-stack.md"),
            "# SHIPLOG-PROP-0008\n",
        );
        dir
    }

    #[test]
    fn writes_pr_body_from_active_work_item() {
        let dir = fixture_workspace();
        run(PrBodyInputs {
            workspace_root: dir.path().to_path_buf(),
            work_item: "pr-body-generator".to_string(),
            output: PathBuf::from("target/source-of-truth/pr-body.md"),
        })
        .unwrap();

        let body = fs::read_to_string(dir.path().join("target/source-of-truth/pr-body.md"))
            .expect("read generated body");
        assert!(body.contains("## Summary"));
        assert!(body.contains("Proposal: `SHIPLOG-PROP-0008`"));
        assert!(body.contains("Spec: `SHIPLOG-SPEC-0010`"));
        assert!(body.contains("No GitHub API calls"));
        assert!(body.contains("## Acceptance"));
        assert!(body.contains("The generated body includes links, proof, and receipt refs."));
        assert!(body.contains("Support-tier impact"));
        assert!(body.contains("stabilizing"));
        assert!(body.contains("## Existing receipts"));
        assert!(body.contains("EffortlessMetrics/shiplog-swarm#36"));
        assert!(body.contains("EffortlessMetrics/shiplog#479"));
        assert!(body.contains("rtk cargo xtask pr-body --work-item pr-body-generator"));
        assert!(!body.contains("cargo xtask stale-plan-proof"));
        assert!(body.contains("This generates drafts only."));
    }

    #[test]
    fn existing_receipts_prefer_compact_pr_refs_over_closure_notes() {
        let work_item = WorkItem {
            id: "promotion-cadence".to_string(),
            proposal: None,
            spec: None,
            adr: None,
            plan: "plans/shiplog-swarm/implementation-plan.md".to_string(),
            commands: Vec::new(),
            receipts: vec![
                "EffortlessMetrics/shiplog-swarm#132".to_string(),
                "EffortlessMetrics/shiplog#569".to_string(),
                "EffortlessMetrics/shiplog-swarm#133".to_string(),
                "EffortlessMetrics/shiplog#570".to_string(),
                "EffortlessMetrics/shiplog#205 closed as completed by #206/#208".to_string(),
            ],
        };
        let mut body = String::new();

        push_receipt_refs(&mut body, &work_item);

        assert!(body.contains("4 compact PR receipt refs from 5 total recorded receipts"));
        assert!(body.contains("EffortlessMetrics/shiplog-swarm#133"));
        assert!(body.contains("EffortlessMetrics/shiplog#570"));
        assert!(!body.contains("closed as completed"));
    }

    #[test]
    fn compact_pr_receipt_refs_require_numeric_issue_suffix_and_no_narrative() {
        assert!(is_compact_pr_receipt_ref(
            "EffortlessMetrics/shiplog-swarm#134"
        ));
        assert!(!is_compact_pr_receipt_ref(
            "EffortlessMetrics/shiplog#205 closed as completed by #206/#208"
        ));
        assert!(!is_compact_pr_receipt_ref(
            "EffortlessMetrics/shiplog#closed"
        ));
    }

    #[test]
    fn section_lookup_uses_ordered_fallback_sections() {
        let plan_item = PlanItem {
            metadata: BTreeMap::new(),
            sections: BTreeMap::from([
                ("Goal".to_string(), "Keep the PR useful.".to_string()),
                (
                    "Claim boundary".to_string(),
                    "This does not move release authority.".to_string(),
                ),
            ]),
        };

        assert_eq!(
            section_or_first_fallback(
                &plan_item,
                &["Production delta", "Goal"],
                "No scope recorded.",
            ),
            "Keep the PR useful."
        );
        assert_eq!(
            section_or_first_fallback(
                &plan_item,
                &["Non-goals", "Claim boundary"],
                "No non-goals recorded.",
            ),
            "This does not move release authority."
        );
    }

    #[test]
    fn missing_work_item_fails_clearly() {
        let dir = fixture_workspace();
        let err = run(PrBodyInputs {
            workspace_root: dir.path().to_path_buf(),
            work_item: "missing".to_string(),
            output: PathBuf::from("target/source-of-truth/pr-body.md"),
        })
        .unwrap_err();

        assert!(err.to_string().contains("missing work item"));
    }

    #[test]
    fn receipt_summary_limits_compact_pr_receipt_refs() {
        let mut out = String::new();
        let work_item = WorkItem {
            id: "receipts".to_string(),
            proposal: None,
            spec: None,
            adr: None,
            plan: "plans/test.md".to_string(),
            commands: Vec::new(),
            receipts: (1..=13)
                .map(|number| format!("EffortlessMetrics/shiplog#{number}"))
                .collect(),
        };

        push_receipt_refs(&mut out, &work_item);

        assert!(out.contains("Showing last 12 of 13 compact PR receipt refs"));
        assert!(!out.contains("Showing latest"));
        assert!(!out.contains("- `EffortlessMetrics/shiplog#1`\n"));
        assert!(out.contains("- `EffortlessMetrics/shiplog#13`\n"));
    }
}
