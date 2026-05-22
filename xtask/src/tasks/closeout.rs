//! `cargo xtask closeout`
//!
//! Generates source-of-truth closeout artifacts from the active goal manifest.
//! This command writes repo-local documentation/archive files only; it does not
//! inspect GitHub or mutate provider state.

use anyhow::{Context, Result, bail};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

pub struct CloseoutInputs {
    pub workspace_root: PathBuf,
    pub goal: String,
    pub date: Option<String>,
    pub handoff_output: Option<PathBuf>,
    pub archive_output: Option<PathBuf>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ActiveGoal {
    schema_version: u32,
    id: String,
    title: String,
    status: String,
    owner: String,
    created: String,
    objective: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    end_state: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    work_item: Vec<WorkItem>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct WorkItem {
    id: String,
    status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    proposal: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    spec: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    adr: Option<String>,
    plan: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    commands: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    receipts: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    blocked_by: Option<toml::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    superseded_by: Option<String>,
}

#[derive(Debug, Default)]
struct PlanItem {
    sections: BTreeMap<String, String>,
}

pub fn run(inputs: CloseoutInputs) -> Result<()> {
    let goal = load_active_goal(&inputs.workspace_root)?;
    if goal.id != inputs.goal {
        bail!(
            "closeout: active goal {:?} does not match requested goal {:?}",
            goal.id,
            inputs.goal
        );
    }

    let date = inputs
        .date
        .unwrap_or_else(|| Utc::now().date_naive().to_string());
    validate_date(&date)?;

    let plan_items = load_plan_items(&inputs.workspace_root, &goal)?;

    let handoff_path = resolve_output_path(
        &inputs.workspace_root,
        inputs.handoff_output.as_deref(),
        &format!("docs/handoffs/{date}-{}-closeout.md", goal.id),
    );
    let archive_path = resolve_output_path(
        &inputs.workspace_root,
        inputs.archive_output.as_deref(),
        &format!(".codex/goals/archive/{date}-{}.toml", goal.id),
    );

    write_with_parent(
        &handoff_path,
        render_closeout(&goal, &date, &plan_items).as_bytes(),
    )?;

    let mut archived_goal = goal.clone();
    archived_goal.status = "archived".to_string();
    let toml = toml::to_string_pretty(&archived_goal).context("serialize archived goal")?;
    write_with_parent(&archive_path, toml.as_bytes())?;

    println!(
        "closeout: wrote {} and {}",
        display_path(&inputs.workspace_root, &handoff_path),
        display_path(&inputs.workspace_root, &archive_path)
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

fn load_plan_items(workspace_root: &Path, goal: &ActiveGoal) -> Result<BTreeMap<String, PlanItem>> {
    let mut plan_text_by_path = BTreeMap::new();
    let mut out = BTreeMap::new();

    for item in &goal.work_item {
        if item.plan.trim().is_empty() {
            continue;
        }
        if !relative_repo_path(&item.plan) {
            bail!(
                "closeout: work item {:?} plan {:?} must be a relative repo path",
                item.id,
                item.plan
            );
        }
        let plan_text = match plan_text_by_path.get(&item.plan) {
            Some(text) => text,
            None => {
                let path = workspace_root.join(&item.plan);
                let text = fs::read_to_string(&path)
                    .with_context(|| format!("read {}", path.display()))?;
                plan_text_by_path.insert(item.plan.clone(), text);
                plan_text_by_path
                    .get(&item.plan)
                    .expect("inserted plan text")
            }
        };
        if let Some(section) = extract_work_item_section(plan_text, &item.id) {
            out.insert(item.id.clone(), parse_plan_item(&section));
        }
    }

    Ok(out)
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
        }
    }

    if let Some(section_name) = current_section {
        sections.insert(section_name, current_lines.join("\n").trim().to_string());
    }

    PlanItem { sections }
}

fn render_closeout(
    goal: &ActiveGoal,
    date: &str,
    plan_items: &BTreeMap<String, PlanItem>,
) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {} closeout\n\n", goal.title));
    out.push_str(&format!("Date: {date}\n"));
    out.push_str(&format!("Goal: `{}`\n", goal.id));
    out.push_str(&format!("Status: `{}`\n", goal.status));
    out.push_str(&format!("Owner: `{}`\n", goal.owner));
    out.push_str(&format!("Created: `{}`\n\n", goal.created));

    out.push_str("## Objective\n\n");
    out.push_str(goal.objective.trim());
    out.push_str("\n\n");

    out.push_str("## End state\n\n");
    if goal.end_state.is_empty() {
        out.push_str("No end-state entries recorded.\n\n");
    } else {
        for item in &goal.end_state {
            out.push_str(&format!("- {}\n", item.trim()));
        }
        out.push('\n');
    }

    out.push_str("## Landed work items\n\n");
    let landed = goal
        .work_item
        .iter()
        .filter(|item| matches!(item.status.as_str(), "done" | "superseded"))
        .collect::<Vec<_>>();
    if landed.is_empty() {
        out.push_str("No done or superseded work items recorded.\n\n");
    } else {
        out.push_str("| Work item | Status | Proposal | Spec | Plan |\n");
        out.push_str("|---|---|---|---|---|\n");
        for item in landed {
            out.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                md(&item.id),
                md(&item.status),
                md_opt(&item.proposal),
                md_opt(&item.spec),
                md(&item.plan)
            ));
        }
        out.push('\n');
    }

    out.push_str("## Proof commands\n\n");
    for item in &goal.work_item {
        if item.commands.is_empty() {
            continue;
        }
        out.push_str(&format!("### {}\n\n", item.id));
        out.push_str("```bash\n");
        for command in &item.commands {
            out.push_str(command.trim());
            out.push('\n');
        }
        out.push_str("```\n\n");
    }

    out.push_str("## Receipts\n\n");
    let items_with_receipts = goal
        .work_item
        .iter()
        .filter(|item| !item.receipts.is_empty())
        .collect::<Vec<_>>();
    if items_with_receipts.is_empty() {
        out.push_str("No receipt refs recorded in the active goal manifest.\n\n");
    } else {
        for item in items_with_receipts {
            out.push_str(&format!("### {}\n\n", item.id));
            for receipt in &item.receipts {
                out.push_str(&format!("- `{}`\n", receipt.trim()));
            }
            out.push('\n');
        }
    }

    out.push_str("## Claim boundaries\n\n");
    for item in &goal.work_item {
        let boundary = plan_items
            .get(&item.id)
            .and_then(|plan_item| plan_item.sections.get("Claim boundary"))
            .map(String::as_str)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("No claim boundary recorded in the linked plan item.");
        out.push_str(&format!("### {}\n\n", item.id));
        out.push_str(boundary.trim());
        out.push_str("\n\n");
    }

    out.push_str("## Remaining work\n\n");
    let remaining = goal
        .work_item
        .iter()
        .filter(|item| !matches!(item.status.as_str(), "done" | "superseded"))
        .collect::<Vec<_>>();
    if remaining.is_empty() {
        out.push_str("No active, ready, or blocked work items remain in this goal manifest.\n\n");
    } else {
        out.push_str("| Work item | Status | Blocked by |\n");
        out.push_str("|---|---|---|\n");
        for item in remaining {
            out.push_str(&format!(
                "| {} | {} | {} |\n",
                md(&item.id),
                md(&item.status),
                md(&blocked_by_display(item.blocked_by.as_ref()))
            ));
        }
        out.push('\n');
    }

    out.push_str("## Generated boundary\n\n");
    out.push_str(
        "This closeout is generated from `.codex/goals/active.toml` and linked \
         plan text. It does not inspect GitHub state, infer merged PRs, replace \
         release handoffs, mutate provider records, or prove runtime product \
         behavior.\n",
    );

    out
}

fn validate_date(date: &str) -> Result<()> {
    let valid = date.len() == 10
        && date.chars().enumerate().all(|(idx, ch)| {
            if idx == 4 || idx == 7 {
                ch == '-'
            } else {
                ch.is_ascii_digit()
            }
        });
    if valid {
        Ok(())
    } else {
        bail!("closeout: --date must use YYYY-MM-DD format, got {date:?}")
    }
}

fn resolve_output_path(workspace_root: &Path, output: Option<&Path>, default: &str) -> PathBuf {
    let output = output.unwrap_or_else(|| Path::new(default));
    if output.is_absolute() {
        output.to_path_buf()
    } else {
        workspace_root.join(output)
    }
}

fn write_with_parent(path: &Path, content: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    fs::write(path, content).with_context(|| format!("write {}", path.display()))
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

fn md_opt(value: &Option<String>) -> String {
    md(value.as_deref().unwrap_or("-"))
}

fn md(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

fn blocked_by_display(value: Option<&toml::Value>) -> String {
    match value {
        Some(toml::Value::String(s)) => s.clone(),
        Some(toml::Value::Array(values)) => values
            .iter()
            .map(|value| match value {
                toml::Value::String(s) => s.clone(),
                other => other.to_string(),
            })
            .collect::<Vec<_>>()
            .join(", "),
        Some(other) => other.to_string(),
        None => "-".to_string(),
    }
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
status = "done"
proposal = "SHIPLOG-PROP-0008"
spec = "SHIPLOG-SPEC-0010"
plan = "plans/0.10.0/implementation-plan.md"
commands = ["cargo xtask pr-body --work-item pr-body-generator", "git diff --check"]
receipts = ["EffortlessMetrics/shiplog-swarm#36", "EffortlessMetrics/shiplog#479"]

[[work_item]]
id = "closeout-generator"
status = "active"
proposal = "SHIPLOG-PROP-0008"
spec = "SHIPLOG-SPEC-0010"
plan = "plans/0.10.0/implementation-plan.md"
commands = ["cargo xtask closeout --goal shiplog-source-of-truth-stack", "git diff --check"]
"#,
        );
        write(
            &dir.path().join("plans/0.10.0/implementation-plan.md"),
            r#"# 0.10.0 Source-of-Truth Rollout Plan

## Work item: pr-body-generator

### Claim boundary

This generates PR body drafts only.

## Work item: closeout-generator

### Claim boundary

This generates closeout drafts only.
"#,
        );
        dir
    }

    #[test]
    fn writes_closeout_and_archive() {
        let dir = fixture_workspace();
        run(CloseoutInputs {
            workspace_root: dir.path().to_path_buf(),
            goal: "shiplog-source-of-truth-stack".to_string(),
            date: Some("2026-05-22".to_string()),
            handoff_output: None,
            archive_output: None,
        })
        .unwrap();

        let handoff = fs::read_to_string(
            dir.path()
                .join("docs/handoffs/2026-05-22-shiplog-source-of-truth-stack-closeout.md"),
        )
        .expect("read generated handoff");
        assert!(handoff.contains("# Shiplog source-of-truth stack rollout closeout"));
        assert!(handoff.contains("## Landed work items"));
        assert!(handoff.contains("pr-body-generator"));
        assert!(handoff.contains("EffortlessMetrics/shiplog-swarm#36"));
        assert!(handoff.contains("This generates PR body drafts only."));
        assert!(handoff.contains("closeout-generator"));

        let archive = fs::read_to_string(
            dir.path()
                .join(".codex/goals/archive/2026-05-22-shiplog-source-of-truth-stack.toml"),
        )
        .expect("read generated archive");
        assert!(archive.contains("status = \"archived\""));
        assert!(archive.contains("id = \"shiplog-source-of-truth-stack\""));
    }

    #[test]
    fn missing_goal_fails_clearly() {
        let dir = fixture_workspace();
        let err = run(CloseoutInputs {
            workspace_root: dir.path().to_path_buf(),
            goal: "other-goal".to_string(),
            date: Some("2026-05-22".to_string()),
            handoff_output: None,
            archive_output: None,
        })
        .unwrap_err();

        assert!(err.to_string().contains("does not match requested goal"));
    }
}
