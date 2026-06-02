//! `cargo xtask check-support-tiers`
//!
//! Validates `docs/status/SUPPORT_TIERS.md`, the repo-local map from
//! product/governance claims to copyable proof commands.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug)]
struct ClaimRow {
    line: usize,
    surface: String,
    tier: String,
    claim: String,
    proof_command: String,
    notes: String,
}

#[derive(Debug, Deserialize)]
struct ActiveGoal {
    id: String,
    #[serde(default)]
    work_item: Vec<ActiveGoalWorkItem>,
}

#[derive(Debug, Deserialize)]
struct ActiveGoalWorkItem {
    id: String,
}

pub fn run(workspace_root: &Path) -> Result<()> {
    let path = workspace_root
        .join("docs")
        .join("status")
        .join("SUPPORT_TIERS.md");
    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;

    let mut findings = Vec::new();
    let rows = parse_claim_rows(&text, &mut findings);

    for row in &rows {
        validate_claim_row(workspace_root, row, &mut findings);
    }

    if findings.is_empty() {
        println!(
            "check-support-tiers: {} support-tier claim(s) linked and valid.",
            rows.len()
        );
        return Ok(());
    }

    for finding in &findings {
        eprintln!("  {finding}");
    }
    Err(anyhow::anyhow!(
        "check-support-tiers: {} finding(s)",
        findings.len()
    ))
}

fn parse_claim_rows(text: &str, findings: &mut Vec<String>) -> Vec<ClaimRow> {
    let mut rows = Vec::new();
    let mut in_claim_map = false;
    let mut saw_header = false;

    for (index, line) in text.lines().enumerate() {
        let line_no = index + 1;
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
        if line.trim().is_empty() {
            continue;
        }
        if is_table_separator(line) {
            continue;
        }
        if line.trim() == "| Surface | Tier | Claim | Proof command | Notes |" {
            saw_header = true;
            continue;
        }
        if !line.trim_start().starts_with('|') {
            continue;
        }
        let cells = table_cells(line);
        if cells.len() != 5 {
            findings.push(format!(
                "[support-tier-row-shape] line {line_no} has {} cell(s), expected 5",
                cells.len()
            ));
            continue;
        }
        rows.push(ClaimRow {
            line: line_no,
            surface: cells[0].to_string(),
            tier: cells[1].to_string(),
            claim: cells[2].to_string(),
            proof_command: cells[3].to_string(),
            notes: cells[4].to_string(),
        });
    }

    if !saw_header {
        findings
            .push("[support-tier-missing-claim-map] missing claim map table header".to_string());
    }
    if rows.is_empty() {
        findings
            .push("[support-tier-empty-claim-map] no support-tier claim rows found".to_string());
    }

    rows
}

fn validate_claim_row(workspace_root: &Path, row: &ClaimRow, findings: &mut Vec<String>) {
    for (field, value) in [
        ("surface", row.surface.as_str()),
        ("tier", row.tier.as_str()),
        ("claim", row.claim.as_str()),
        ("notes", row.notes.as_str()),
    ] {
        if value.trim().is_empty() {
            findings.push(format!(
                "[support-tier-empty-{field}] line {} has empty {field}",
                row.line
            ));
        }
    }

    if !valid_tier(&row.tier) {
        findings.push(format!(
            "[support-tier-invalid-tier] line {} tier {:?} is not Stable/Stabilizing/Experimental/Advisory/Not supported",
            row.line, row.tier
        ));
    }

    if requires_proof(&row.tier) {
        if proof_is_empty(&row.proof_command) {
            findings.push(format!(
                "[support-tier-missing-proof] line {} {:?} claim has no proof command",
                row.line, row.surface
            ));
        }
        let commands = backtick_commands(&row.proof_command);
        if commands.is_empty() {
            findings.push(format!(
                "[support-tier-proof-not-command] line {} proof must contain a backticked command",
                row.line
            ));
        }
        for command in commands {
            validate_proof_command(workspace_root, row.line, command, findings);
        }
    }
}

fn validate_proof_command(
    workspace_root: &Path,
    line: usize,
    command: &str,
    findings: &mut Vec<String>,
) {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.len() < 4 || parts[0] != "rtk" || parts[1] != "cargo" || parts[2] != "xtask" {
        findings.push(format!(
            "[support-tier-proof-unsupported-command] line {line} proof command {command:?} must start with `rtk cargo xtask`"
        ));
        return;
    }
    let subcommand = parts[3];
    if !known_xtask_subcommand(subcommand) {
        findings.push(format!(
            "[support-tier-proof-unknown-xtask] line {line} proof command {command:?} references unknown xtask subcommand {subcommand:?}"
        ));
        return;
    }

    match subcommand {
        "pr-body" => validate_pr_body_proof(workspace_root, line, command, &parts, findings),
        "promotion-body" => {
            validate_target_output_proof("promotion-body", line, command, &parts, findings)
        }
        "closeout" => validate_closeout_proof(workspace_root, line, command, &parts, findings),
        _ => {}
    }
}

fn validate_pr_body_proof(
    workspace_root: &Path,
    line: usize,
    command: &str,
    parts: &[&str],
    findings: &mut Vec<String>,
) {
    let Some(work_item) = flag_value(parts, "--work-item") else {
        findings.push(format!(
            "[support-tier-pr-body-missing-work-item] line {line} proof command {command:?} must include `--work-item <id>`"
        ));
        return;
    };

    let Some(goal) = load_active_goal(workspace_root, line, findings) else {
        return;
    };
    if !goal.work_item.iter().any(|item| item.id == work_item) {
        findings.push(format!(
            "[support-tier-pr-body-unknown-work-item] line {line} proof command {command:?} references work item {work_item:?}, but `.codex/goals/active.toml` does not contain it"
        ));
    }
}

fn validate_closeout_proof(
    workspace_root: &Path,
    line: usize,
    command: &str,
    parts: &[&str],
    findings: &mut Vec<String>,
) {
    let Some(goal_id) = flag_value(parts, "--goal") else {
        findings.push(format!(
            "[support-tier-closeout-missing-goal] line {line} proof command {command:?} must include `--goal <id>`"
        ));
        return;
    };

    let Some(goal) = load_active_goal(workspace_root, line, findings) else {
        return;
    };
    if goal.id != goal_id {
        findings.push(format!(
            "[support-tier-closeout-goal-mismatch] line {line} proof command {command:?} references goal {goal_id:?}, but `.codex/goals/active.toml` is {:?}",
            goal.id
        ));
    }

    for flag in ["--handoff-output", "--archive-output"] {
        let Some(output) = flag_value(parts, flag) else {
            findings.push(format!(
                "[support-tier-closeout-source-writing-proof] line {line} proof command {command:?} must include `{flag} target/...` so support-tier proof does not write source closeout artifacts"
            ));
            continue;
        };
        if !output.starts_with("target/") && !output.starts_with("target\\") {
            findings.push(format!(
                "[support-tier-closeout-source-writing-proof] line {line} proof command {command:?} uses `{flag} {output}`; support-tier closeout proof must write under `target/`"
            ));
        }
    }
}

fn validate_target_output_proof(
    label: &str,
    line: usize,
    command: &str,
    parts: &[&str],
    findings: &mut Vec<String>,
) {
    let Some(output) = flag_value(parts, "--output") else {
        findings.push(format!(
            "[support-tier-{label}-missing-output] line {line} proof command {command:?} must include `--output target/...`"
        ));
        return;
    };
    if !output.starts_with("target/") && !output.starts_with("target\\") {
        findings.push(format!(
            "[support-tier-{label}-source-writing-proof] line {line} proof command {command:?} uses `--output {output}`; proof must write under `target/`"
        ));
    }
}

fn load_active_goal(
    workspace_root: &Path,
    line: usize,
    findings: &mut Vec<String>,
) -> Option<ActiveGoal> {
    let path = workspace_root
        .join(".codex")
        .join("goals")
        .join("active.toml");
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) => {
            findings.push(format!(
                "[support-tier-active-goal-unreadable] line {line} could not read {}: {err}",
                path.display()
            ));
            return None;
        }
    };
    match toml::from_str(&text) {
        Ok(goal) => Some(goal),
        Err(err) => {
            findings.push(format!(
                "[support-tier-active-goal-invalid] line {line} could not parse {}: {err}",
                path.display()
            ));
            None
        }
    }
}

fn flag_value<'a>(parts: &'a [&str], flag: &str) -> Option<&'a str> {
    for (index, part) in parts.iter().enumerate() {
        if *part == flag {
            return parts
                .get(index + 1)
                .copied()
                .filter(|value| !value.starts_with("--"));
        }
        if let Some(value) = part.strip_prefix(&format!("{flag}=")) {
            return (!value.is_empty()).then_some(value);
        }
    }
    None
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

fn valid_tier(tier: &str) -> bool {
    matches!(
        tier,
        "Stable" | "Stabilizing" | "Experimental" | "Advisory" | "Not supported"
    )
}

fn requires_proof(tier: &str) -> bool {
    matches!(tier, "Stable" | "Stabilizing")
}

fn proof_is_empty(proof: &str) -> bool {
    let proof = proof.trim();
    let normalized = proof.to_ascii_lowercase();
    proof.is_empty() || matches!(normalized.as_str(), "none" | "n/a" | "tbd")
}

fn backtick_commands(cell: &str) -> Vec<&str> {
    let mut commands = Vec::new();
    let mut remainder = cell;
    while let Some(start) = remainder.find('`') {
        let after_start = &remainder[start + 1..];
        let Some(end) = after_start.find('`') else {
            break;
        };
        let command = after_start[..end].trim();
        if !command.is_empty() {
            commands.push(command);
        }
        remainder = &after_start[end + 1..];
    }
    commands
}

fn known_xtask_subcommand(command: &str) -> bool {
    matches!(
        command,
        "check-policy-schemas"
            | "check-policy-ledgers"
            | "check-doc-artifacts"
            | "check-goals"
            | "check-support-tiers"
            | "package-boundary"
            | "package-version"
            | "policy-report"
            | "repo-contract-report"
            | "pr-body"
            | "promotion-body"
            | "closeout"
            | "ci"
            | "check-file-policy"
            | "check-generated"
            | "check-executable-files"
            | "check-workflows"
            | "check-dependency-surfaces"
            | "check-process-policy"
            | "check-network-policy"
            | "check-lint-policy"
            | "check-clippy-exceptions"
            | "no-panic"
            | "check-no-panic-family"
            | "check-label-enforcement"
            | "check-risk-pack-integrity"
            | "check-lane-mappings"
            | "check-actuals-coverage"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write_support_tiers(body: &str) -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        let path = dir.path().join("docs").join("status");
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("SUPPORT_TIERS.md"), body).unwrap();
        dir
    }

    fn write_active_goal(dir: &tempfile::TempDir) {
        let path = dir.path().join(".codex").join("goals");
        fs::create_dir_all(&path).unwrap();
        fs::write(
            path.join("active.toml"),
            r#"
schema_version = 1
id = "shiplog-swarm-control-plane"
title = "Shiplog swarm development control plane"
status = "active"
owner = "codex"
created = "2026-05-22"
objective = "Keep normal shiplog development moving through swarm."
end_state = ["Promotion remains boring."]

[[work_item]]
id = "promotion-cadence"
status = "active"
plan = "plans/shiplog-swarm/implementation-plan.md"
commands = ["rtk cargo xtask repo-contract-report"]
"#,
        )
        .unwrap();
    }

    fn valid_doc() -> String {
        r#"# Support tiers

## Tier definitions

| Tier | Meaning |
|---|---|
| Stable | Supported. |
| Stabilizing | Maturing. |

## Claim map

| Surface | Tier | Claim | Proof command | Notes |
|---|---|---|---|---|
| Document artifact links | Stabilizing | Proposal/spec/ADR/plan artifacts are linked. | `rtk cargo xtask check-doc-artifacts` | Dedicated checker. |
| Policy ledgers | Stable | Policy files parse. | `rtk cargo xtask check-policy-ledgers` | Required proof. |
"#
        .to_string()
    }

    #[test]
    fn valid_support_tiers_pass() {
        let dir = write_support_tiers(&valid_doc());
        run(dir.path()).expect("valid support tiers should pass");
    }

    #[test]
    fn invalid_tier_is_finding() {
        let doc = valid_doc().replace("| Stable | Policy files", "| Strong | Policy files");
        let dir = write_support_tiers(&doc);
        let err = run(dir.path()).unwrap_err();
        assert!(err.to_string().contains("1 finding"));
    }

    #[test]
    fn stable_claim_without_proof_is_finding() {
        let doc = valid_doc().replace("`rtk cargo xtask check-policy-ledgers`", "TBD");
        let dir = write_support_tiers(&doc);
        let err = run(dir.path()).unwrap_err();
        assert!(err.to_string().contains("2 finding"));
    }

    #[test]
    fn unknown_xtask_proof_is_finding() {
        let doc = valid_doc().replace(
            "`rtk cargo xtask check-policy-ledgers`",
            "`rtk cargo xtask made-up-check`",
        );
        let dir = write_support_tiers(&doc);
        let err = run(dir.path()).unwrap_err();
        assert!(err.to_string().contains("1 finding"));
    }

    #[test]
    fn bare_cargo_xtask_proof_is_finding() {
        let doc = valid_doc().replace("`rtk cargo xtask", "`cargo xtask");
        let dir = write_support_tiers(&doc);
        let err = run(dir.path()).unwrap_err();
        assert!(err.to_string().contains("2 finding"));
    }

    #[test]
    fn source_of_truth_generator_proofs_validate_active_goal_refs() {
        let doc = r#"# Support tiers

## Claim map

| Surface | Tier | Claim | Proof command | Notes |
|---|---|---|---|---|
| PR body generator | Stabilizing | Agents can draft PR bodies. | `rtk cargo xtask pr-body --work-item promotion-cadence --output target/source-of-truth/pr-body.md` | Derived draft. |
| Promotion body generator | Stabilizing | Agents can draft promotion bodies. | `rtk cargo xtask promotion-body --output target/source-of-truth/promotion-body.md` | Derived draft. |
| Closeout generator | Stabilizing | Agents can draft closeouts. | `rtk cargo xtask closeout --goal shiplog-swarm-control-plane --handoff-output target/source-of-truth/closeout.md --archive-output target/source-of-truth/active-goal-archive.toml` | Derived draft. |
"#;
        let dir = write_support_tiers(doc);
        write_active_goal(&dir);
        run(dir.path()).expect("generator support-tier proofs should pass");
    }

    #[test]
    fn stale_pr_body_work_item_is_finding() {
        let doc = valid_doc().replace(
            "`rtk cargo xtask check-policy-ledgers`",
            "`rtk cargo xtask pr-body --work-item pr-body-generator`",
        );
        let dir = write_support_tiers(&doc);
        write_active_goal(&dir);

        let err = run(dir.path()).unwrap_err();
        assert!(err.to_string().contains("1 finding"));
    }

    #[test]
    fn stale_closeout_goal_is_finding() {
        let doc = valid_doc().replace(
            "`rtk cargo xtask check-policy-ledgers`",
            "`rtk cargo xtask closeout --goal shiplog-source-of-truth-stack --handoff-output target/source-of-truth/closeout.md --archive-output target/source-of-truth/active-goal-archive.toml`",
        );
        let dir = write_support_tiers(&doc);
        write_active_goal(&dir);

        let err = run(dir.path()).unwrap_err();
        assert!(err.to_string().contains("1 finding"));
    }

    #[test]
    fn source_writing_closeout_proof_is_finding() {
        let doc = valid_doc().replace(
            "`rtk cargo xtask check-policy-ledgers`",
            "`rtk cargo xtask closeout --goal shiplog-swarm-control-plane`",
        );
        let dir = write_support_tiers(&doc);
        write_active_goal(&dir);

        let err = run(dir.path()).unwrap_err();
        assert!(err.to_string().contains("2 finding"));
    }

    #[test]
    fn source_writing_promotion_body_proof_is_finding() {
        let doc = valid_doc().replace(
            "`rtk cargo xtask check-policy-ledgers`",
            "`rtk cargo xtask promotion-body --output docs/promotion-body.md`",
        );
        let dir = write_support_tiers(&doc);

        let err = run(dir.path()).unwrap_err();
        assert!(err.to_string().contains("1 finding"));
    }
}
