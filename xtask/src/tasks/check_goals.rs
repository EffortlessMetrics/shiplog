//! `cargo xtask check-goals`
//!
//! Validates `.codex/goals/active.toml` plus archived goal manifests, the
//! repo-local execution-state manifests that tell agents which
//! source-of-truth work is current and what has already closed.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Deserialize)]
struct ActiveGoal {
    schema_version: u32,
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

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
struct DocArtifactsPolicy {
    #[serde(default)]
    artifact: Vec<ArtifactRef>,
}

#[derive(Debug, Deserialize)]
struct ArtifactRef {
    id: String,
    kind: String,
    path: String,
}

pub fn run(workspace_root: &Path) -> Result<()> {
    let active_path = workspace_root
        .join(".codex")
        .join("goals")
        .join("active.toml");
    let goal = load_goal(&active_path)?;

    let artifacts = load_doc_artifacts(workspace_root)?;
    let by_id = artifact_by_id(&artifacts);
    let by_plan_path = plan_artifact_by_path(&artifacts);

    let mut findings = Vec::new();
    validate_no_legacy_active_goal(workspace_root, &mut findings);
    validate_goal_shape(&goal, &mut findings);
    validate_work_items(
        &goal,
        workspace_root,
        &by_id,
        &by_plan_path,
        true,
        &mut findings,
    );
    let archive_count =
        validate_archived_goals(workspace_root, &by_id, &by_plan_path, &mut findings)?;

    if findings.is_empty() {
        println!(
            "check-goals: active goal {:?} has {} work item(s) linked and valid; {archive_count} archived goal manifest(s) linked and valid.",
            goal.id,
            goal.work_item.len()
        );
        return Ok(());
    }

    for finding in &findings {
        eprintln!("  {finding}");
    }
    Err(anyhow::anyhow!(
        "check-goals: {} finding(s)",
        findings.len()
    ))
}

fn validate_no_legacy_active_goal(workspace_root: &Path, findings: &mut Vec<String>) {
    let legacy_path = workspace_root
        .join(".shiplog")
        .join("goals")
        .join("active.toml");
    if legacy_path.exists() {
        findings.push(format!(
            "[goal-legacy-active-manifest] {} exists; active execution state must live in .codex/goals/active.toml",
            legacy_path.display()
        ));
    }
}

fn load_goal(path: &Path) -> Result<ActiveGoal> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("parse {}", path.display()))
}

fn validate_archived_goals(
    workspace_root: &Path,
    by_id: &BTreeMap<&str, &ArtifactRef>,
    by_plan_path: &BTreeMap<&str, &ArtifactRef>,
    findings: &mut Vec<String>,
) -> Result<usize> {
    let archive_dir = workspace_root.join(".codex").join("goals").join("archive");
    if !archive_dir.exists() {
        return Ok(0);
    }
    let mut paths = archive_goal_paths(&archive_dir)?;
    paths.sort();

    for path in &paths {
        match load_goal(path) {
            Ok(goal) => {
                validate_goal_shape(&goal, findings);
                validate_archive_goal_shape(path, &goal, findings);
                validate_work_items(&goal, workspace_root, by_id, by_plan_path, false, findings);
            }
            Err(err) => findings.push(format!(
                "[goal-archive-invalid] {}: {err:#}",
                path.display()
            )),
        }
    }

    Ok(paths.len())
}

fn archive_goal_paths(archive_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(archive_dir).with_context(|| {
        format!(
            "read archived goal manifest directory {}",
            archive_dir.display()
        )
    })? {
        let entry = entry.with_context(|| {
            format!(
                "read archived goal manifest entry in {}",
                archive_dir.display()
            )
        })?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) == Some("toml") {
            paths.push(path);
        }
    }
    Ok(paths)
}

fn validate_archive_goal_shape(path: &Path, goal: &ActiveGoal, findings: &mut Vec<String>) {
    if goal.status != "archived" {
        findings.push(format!(
            "[goal-archive-status] {} goal {} status {:?} is not archived",
            path.display(),
            goal.id,
            goal.status
        ));
    }
    for item in &goal.work_item {
        if matches!(item.status.as_str(), "active" | "ready") {
            findings.push(format!(
                "[goal-archive-open-work-item] {} goal {} work item {} is still {}",
                path.display(),
                goal.id,
                item.id,
                item.status
            ));
        }
    }
}

fn load_doc_artifacts(workspace_root: &Path) -> Result<Vec<ArtifactRef>> {
    let path = workspace_root.join("policy").join("doc-artifacts.toml");
    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let policy: DocArtifactsPolicy =
        toml::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    Ok(policy.artifact)
}

fn artifact_by_id(artifacts: &[ArtifactRef]) -> BTreeMap<&str, &ArtifactRef> {
    artifacts
        .iter()
        .map(|artifact| (artifact.id.as_str(), artifact))
        .collect()
}

fn plan_artifact_by_path(artifacts: &[ArtifactRef]) -> BTreeMap<&str, &ArtifactRef> {
    artifacts
        .iter()
        .filter(|artifact| artifact.kind == "plan")
        .map(|artifact| (artifact.path.as_str(), artifact))
        .collect()
}

fn validate_goal_shape(goal: &ActiveGoal, findings: &mut Vec<String>) {
    if goal.schema_version != 1 {
        findings.push(format!(
            "[goal-invalid-schema-version] {} schema_version {} is not 1",
            goal.id, goal.schema_version
        ));
    }
    for (field, value) in [
        ("id", goal.id.as_str()),
        ("title", goal.title.as_str()),
        ("status", goal.status.as_str()),
        ("owner", goal.owner.as_str()),
        ("created", goal.created.as_str()),
        ("objective", goal.objective.as_str()),
    ] {
        if value.trim().is_empty() {
            findings.push(format!("[goal-empty-{field}] active goal {field} is empty"));
        }
    }
    if !matches!(
        goal.status.as_str(),
        "active" | "blocked" | "done" | "superseded" | "archived"
    ) {
        findings.push(format!(
            "[goal-invalid-status] {} status {:?} is not active/blocked/done/superseded/archived",
            goal.id, goal.status
        ));
    }
    if goal.status == "active" && goal.work_item.is_empty() {
        findings.push(format!(
            "[goal-active-without-work-items] {} is active but has no work_item entries",
            goal.id
        ));
    }
    if goal.end_state.iter().all(|item| item.trim().is_empty()) {
        findings.push(format!(
            "[goal-empty-end-state] {} has no non-empty end_state entries",
            goal.id
        ));
    }
}

fn validate_work_items(
    goal: &ActiveGoal,
    workspace_root: &Path,
    by_id: &BTreeMap<&str, &ArtifactRef>,
    by_plan_path: &BTreeMap<&str, &ArtifactRef>,
    enforce_rtk_commands: bool,
    findings: &mut Vec<String>,
) {
    let mut seen = BTreeSet::new();
    let mut active_count = 0usize;

    for item in &goal.work_item {
        if !seen.insert(item.id.as_str()) {
            findings.push(format!(
                "[goal-work-item-duplicate-id] duplicate work item id {:?}",
                item.id
            ));
            continue;
        }
        if item.id.trim().is_empty() {
            findings.push("[goal-work-item-empty-id] work item id is empty".to_string());
        }
        if !valid_work_status(&item.status) {
            findings.push(format!(
                "[goal-work-item-invalid-status] {} status {:?} is not ready/active/blocked/done/superseded",
                item.id, item.status
            ));
        }
        if item.status == "active" {
            active_count += 1;
        }

        validate_required_artifact_link(
            &item.id,
            "proposal",
            item.proposal.as_deref(),
            "proposal",
            by_id,
            findings,
        );
        validate_required_artifact_link(
            &item.id,
            "spec",
            item.spec.as_deref(),
            "spec",
            by_id,
            findings,
        );
        if let Some(adr) = non_empty_non_none(item.adr.as_deref()) {
            validate_required_artifact_link(&item.id, "adr", Some(adr), "adr", by_id, findings);
        }
        validate_plan_link(item, workspace_root, by_plan_path, findings);
        validate_status_requirements(item, findings);
        if enforce_rtk_commands {
            validate_current_command_prefixes(item, findings);
        }
    }

    if active_count > 1 {
        findings.push(format!(
            "[goal-multiple-active-work-items] {} has {active_count} active work items",
            goal.id
        ));
    }
}

fn validate_current_command_prefixes(item: &WorkItem, findings: &mut Vec<String>) {
    if !matches!(item.status.as_str(), "ready" | "active") {
        return;
    }

    for command in &item.commands {
        let command = command.trim();
        if command.is_empty() || command.starts_with("rtk ") {
            continue;
        }
        findings.push(format!(
            "[goal-work-item-command-missing-rtk] {} command {:?} must start with \"rtk \"",
            item.id, command
        ));
    }
}

fn validate_required_artifact_link(
    item_id: &str,
    field: &str,
    linked_id: Option<&str>,
    expected_kind: &str,
    by_id: &BTreeMap<&str, &ArtifactRef>,
    findings: &mut Vec<String>,
) {
    let Some(linked_id) = non_empty_non_none(linked_id) else {
        findings.push(format!(
            "[goal-work-item-missing-{field}] {item_id} has no {field} reference"
        ));
        return;
    };
    match by_id.get(linked_id) {
        Some(target) if target.kind == expected_kind => {}
        Some(target) => findings.push(format!(
            "[goal-work-item-link-kind-mismatch] {item_id} {field} {linked_id:?} points to kind {:?}, expected {expected_kind:?}",
            target.kind
        )),
        None => findings.push(format!(
            "[goal-work-item-unresolved-link] {item_id} {field} references missing artifact {linked_id:?}"
        )),
    }
}

fn validate_plan_link(
    item: &WorkItem,
    workspace_root: &Path,
    by_plan_path: &BTreeMap<&str, &ArtifactRef>,
    findings: &mut Vec<String>,
) {
    if item.plan.trim().is_empty() {
        findings.push(format!(
            "[goal-work-item-empty-plan] {} has an empty plan reference",
            item.id
        ));
        return;
    }
    if !relative_repo_path(&item.plan) {
        findings.push(format!(
            "[goal-work-item-unsafe-plan] {} plan {:?} must be a relative repo path",
            item.id, item.plan
        ));
        return;
    }
    if !workspace_root.join(&item.plan).is_file() {
        findings.push(format!(
            "[goal-work-item-missing-plan-file] {} plan {:?} is not a readable file",
            item.id, item.plan
        ));
    } else if let Ok(contents) = fs::read_to_string(workspace_root.join(&item.plan)) {
        let heading = format!("## Work item: {}", item.id);
        if !contents.contains(&heading) {
            findings.push(format!(
                "[goal-work-item-not-in-plan] {} is not listed as {:?} in {:?}",
                item.id, heading, item.plan
            ));
        }
    }
    if !by_plan_path.contains_key(item.plan.as_str()) {
        findings.push(format!(
            "[goal-work-item-plan-not-ledgered] {} plan {:?} is not a plan artifact in policy/doc-artifacts.toml",
            item.id, item.plan
        ));
    }
}

fn validate_status_requirements(item: &WorkItem, findings: &mut Vec<String>) {
    match item.status.as_str() {
        "ready" | "active" if !has_non_empty_strings(&item.commands) => findings.push(format!(
            "[goal-work-item-missing-proof-commands] {} is {} but has no proof commands",
            item.id, item.status
        )),
        "blocked" if !value_has_content(item.blocked_by.as_ref()) => findings.push(format!(
            "[goal-work-item-blocked-without-blocker] {} is blocked but has no blocked_by",
            item.id
        )),
        "done"
            if !has_non_empty_strings(&item.commands) && !has_non_empty_strings(&item.receipts) =>
        {
            findings.push(format!(
                "[goal-work-item-done-without-proof] {} is done but has no proof commands or receipt refs",
                item.id
            ));
        }
        "superseded"
            if item
                .superseded_by
                .as_deref()
                .is_none_or(|s| s.trim().is_empty()) =>
        {
            findings.push(format!(
                "[goal-work-item-superseded-without-replacement] {} is superseded but has no superseded_by",
                item.id
            ));
        }
        _ => {}
    }
}

fn valid_work_status(status: &str) -> bool {
    matches!(
        status,
        "ready" | "active" | "blocked" | "done" | "superseded"
    )
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

fn has_non_empty_strings(values: &[String]) -> bool {
    values.iter().any(|value| !value.trim().is_empty())
}

fn value_has_content(value: Option<&toml::Value>) -> bool {
    match value {
        Some(toml::Value::String(s)) => !s.trim().is_empty(),
        Some(toml::Value::Array(values)) => values.iter().any(|value| match value {
            toml::Value::String(s) => !s.trim().is_empty(),
            _ => true,
        }),
        Some(_) => true,
        None => false,
    }
}

fn relative_repo_path(path: &str) -> bool {
    let path = Path::new(path);
    !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    const DOC_ARTIFACTS: &str = r#"
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
id = "SHIPLOG-ADR-0010"
kind = "adr"
path = "docs/adr/SHIPLOG-ADR-0010-example.md"
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
"#;

    const ACTIVE_GOAL: &str = r#"
schema_version = 1

id = "shiplog-source-of-truth-stack"
title = "Shiplog source-of-truth stack rollout"
status = "active"
owner = "codex"
created = "2026-05-20"

objective = "Keep repo source-of-truth artifacts linked."
end_state = ["Artifacts are linked."]

[[work_item]]
id = "active-goal-checker"
status = "active"
proposal = "SHIPLOG-PROP-0008"
spec = "SHIPLOG-SPEC-0010"
plan = "plans/0.10.0/implementation-plan.md"
commands = ["rtk cargo xtask check-goals", "rtk git diff --check"]
"#;

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    fn fixture(active_goal: &str, doc_artifacts: &str) -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        write(
            &dir.path().join("policy").join("doc-artifacts.toml"),
            doc_artifacts,
        );
        write(
            &dir.path().join(".codex").join("goals").join("active.toml"),
            active_goal,
        );
        write(
            &dir.path().join("plans/0.10.0/implementation-plan.md"),
            "Plan artifact: SHIPLOG-PLAN-0010\n\n## Work item: active-goal-checker\n",
        );
        dir
    }

    #[test]
    fn linked_active_goal_passes() {
        let dir = fixture(ACTIVE_GOAL, DOC_ARTIFACTS);
        run(dir.path()).expect("valid active goal should pass");
    }

    #[test]
    fn legacy_shiplog_active_goal_is_rejected() {
        let dir = fixture(ACTIVE_GOAL, DOC_ARTIFACTS);
        write(
            &dir.path()
                .join(".shiplog")
                .join("goals")
                .join("active.toml"),
            ACTIVE_GOAL,
        );

        let mut findings = Vec::new();
        validate_no_legacy_active_goal(dir.path(), &mut findings);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].contains("[goal-legacy-active-manifest]"));

        let err = run(dir.path()).unwrap_err();

        assert!(err.to_string().contains("1 finding"));
    }

    #[test]
    fn linked_archived_goal_passes() {
        let dir = fixture(ACTIVE_GOAL, DOC_ARTIFACTS);
        write(
            &dir.path()
                .join(".codex/goals/archive/2026-05-22-source-of-truth.toml"),
            &ACTIVE_GOAL
                .replacen("status = \"active\"", "status = \"archived\"", 1)
                .replace(
                    "status = \"active\"\nproposal",
                    "status = \"done\"\nproposal",
                ),
        );
        run(dir.path()).expect("valid active and archived goals should pass");
    }

    #[test]
    fn archived_goal_commands_are_historical_receipts() {
        let dir = fixture(ACTIVE_GOAL, DOC_ARTIFACTS);
        write(
            &dir.path()
                .join(".codex/goals/archive/2026-05-22-source-of-truth.toml"),
            &ACTIVE_GOAL
                .replacen("status = \"active\"", "status = \"archived\"", 1)
                .replace(
                    "status = \"active\"\nproposal",
                    "status = \"done\"\nproposal",
                )
                .replace("rtk git diff --check", "git diff --check"),
        );
        run(dir.path()).expect("archived historical command receipts should pass");
    }

    #[test]
    fn archived_goal_must_have_archived_status() {
        let dir = fixture(ACTIVE_GOAL, DOC_ARTIFACTS);
        write(
            &dir.path()
                .join(".codex/goals/archive/2026-05-22-source-of-truth.toml"),
            &ACTIVE_GOAL.replacen(
                "status = \"active\"\nproposal",
                "status = \"done\"\nproposal",
                1,
            ),
        );
        let err = run(dir.path()).unwrap_err();
        assert!(err.to_string().contains("1 finding"));
    }

    #[test]
    fn archived_goal_cannot_have_open_work_item() {
        let dir = fixture(ACTIVE_GOAL, DOC_ARTIFACTS);
        write(
            &dir.path()
                .join(".codex/goals/archive/2026-05-22-source-of-truth.toml"),
            &ACTIVE_GOAL.replacen("status = \"active\"", "status = \"archived\"", 1),
        );
        let err = run(dir.path()).unwrap_err();
        assert!(err.to_string().contains("1 finding"));
    }

    #[test]
    fn multiple_active_work_items_are_finding() {
        let active_goal = ACTIVE_GOAL.replace(
            "commands = [\"rtk cargo xtask check-goals\", \"rtk git diff --check\"]",
            "commands = [\"rtk cargo xtask check-goals\", \"rtk git diff --check\"]\n\n[[work_item]]\nid = \"second-active\"\nstatus = \"active\"\nproposal = \"SHIPLOG-PROP-0008\"\nspec = \"SHIPLOG-SPEC-0010\"\nplan = \"plans/0.10.0/implementation-plan.md\"\ncommands = [\"rtk git diff --check\"]",
        );
        let dir = fixture(&active_goal, DOC_ARTIFACTS);
        let err = run(dir.path()).unwrap_err();
        assert!(err.to_string().contains("2 finding"));
    }

    #[test]
    fn ready_work_item_without_commands_is_finding() {
        let active_goal = ACTIVE_GOAL
            .replace(
                "status = \"active\"\nproposal",
                "status = \"ready\"\nproposal",
            )
            .replace(
                "commands = [\"rtk cargo xtask check-goals\", \"rtk git diff --check\"]",
                "",
            );
        let dir = fixture(&active_goal, DOC_ARTIFACTS);
        let err = run(dir.path()).unwrap_err();
        assert!(err.to_string().contains("1 finding"));
    }

    #[test]
    fn blocked_work_item_without_blocker_is_finding() {
        let active_goal = ACTIVE_GOAL.replace(
            "status = \"active\"\nproposal",
            "status = \"blocked\"\nproposal",
        );
        let dir = fixture(&active_goal, DOC_ARTIFACTS);
        let err = run(dir.path()).unwrap_err();
        assert!(err.to_string().contains("1 finding"));
    }

    #[test]
    fn done_work_item_without_proof_is_finding() {
        let active_goal = ACTIVE_GOAL
            .replace(
                "status = \"active\"\nproposal",
                "status = \"done\"\nproposal",
            )
            .replace(
                "commands = [\"rtk cargo xtask check-goals\", \"rtk git diff --check\"]",
                "",
            );
        let dir = fixture(&active_goal, DOC_ARTIFACTS);
        let err = run(dir.path()).unwrap_err();
        assert!(err.to_string().contains("1 finding"));
    }

    #[test]
    fn active_work_item_commands_must_use_rtk_prefix() {
        let active_goal = ACTIVE_GOAL.replace("rtk git diff --check", "git diff --check");
        let dir = fixture(&active_goal, DOC_ARTIFACTS);
        let err = run(dir.path()).unwrap_err();
        assert!(err.to_string().contains("1 finding"));
    }

    #[test]
    fn missing_artifact_reference_is_finding() {
        let active_goal = ACTIVE_GOAL.replace("SHIPLOG-SPEC-0010", "SHIPLOG-SPEC-9999");
        let dir = fixture(&active_goal, DOC_ARTIFACTS);
        let err = run(dir.path()).unwrap_err();
        assert!(err.to_string().contains("1 finding"));
    }

    #[test]
    fn plan_must_be_a_ledgered_plan_artifact() {
        let active_goal =
            ACTIVE_GOAL.replace("plans/0.10.0/implementation-plan.md", "plans/missing.md");
        let dir = fixture(&active_goal, DOC_ARTIFACTS);
        let err = run(dir.path()).unwrap_err();
        assert!(err.to_string().contains("2 finding"));
    }

    #[test]
    fn work_item_must_be_listed_in_plan() {
        let dir = tempdir().unwrap();
        write(
            &dir.path().join("policy").join("doc-artifacts.toml"),
            DOC_ARTIFACTS,
        );
        write(
            &dir.path().join(".codex").join("goals").join("active.toml"),
            ACTIVE_GOAL,
        );
        write(
            &dir.path().join("plans/0.10.0/implementation-plan.md"),
            "Plan artifact: SHIPLOG-PLAN-0010\n",
        );
        let err = run(dir.path()).unwrap_err();
        assert!(err.to_string().contains("1 finding"));
    }
}
