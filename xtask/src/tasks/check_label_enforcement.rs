//! `cargo xtask check-label-enforcement`
//!
//! Compare every label declared in `policy/ci-budget.toml [labels]` to the
//! labels actually consulted by workflow job-level `if:` blocks in
//! `.github/workflows/*.yml`. Distinguish enforced labels (consulted by at
//! least one workflow) from declared-but-not-enforced labels (listed in
//! `[labels_enforcement].declared_only`). Surface drift when a label moves
//! between categories without an explicit policy update.
//!
//! Three categories of drift this checker catches:
//!
//! 1. **Label declared, neither enforced nor declared-only.** A label was
//!    added to `[labels]` but no workflow consults it and policy doesn't
//!    acknowledge that it is currently aspirational. Either wire it into
//!    a workflow or add it to `declared_only`.
//!
//! 2. **Label declared-only AND consulted by a workflow.** A workflow
//!    started consuming a label that policy still claims is aspirational.
//!    Remove the label from `declared_only` so the auditor-facing
//!    classification stays honest.
//!
//! 3. **Label in `declared_only` but not in `[labels]`.** Defensive check:
//!    `declared_only` should only reference labels that are actually
//!    declared in policy. A stray entry suggests stale policy state.
//!
//! See [`docs/ci/labels.md`](../../docs/ci/labels.md) for the human-
//! readable label catalogue. The labels themselves are the rendered values
//! of `[labels]` (e.g. `ripr_force = "ripr"` → label name `"ripr"`).

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use crate::policy;
use crate::tasks::file_policy::Mode;

#[derive(Debug, Deserialize)]
struct CiBudgetPolicy {
    #[serde(default)]
    labels: BTreeMap<String, String>,
    #[serde(default)]
    labels_enforcement: LabelsEnforcement,
}

#[derive(Debug, Default, Deserialize)]
struct LabelsEnforcement {
    #[serde(default)]
    declared_only: Vec<String>,
}

pub fn run(workspace_root: &Path, mode: Mode) -> Result<()> {
    let policy_dir = policy::policy_dir(workspace_root);
    let budget_path = policy_dir.join("ci-budget.toml");
    let text = fs::read_to_string(&budget_path)
        .with_context(|| format!("read {}", budget_path.display()))?;
    let budget: CiBudgetPolicy =
        toml::from_str(&text).with_context(|| format!("parse {}", budget_path.display()))?;

    let declared_labels: BTreeSet<String> = budget.labels.values().cloned().collect();
    let declared_only: BTreeSet<String> = budget
        .labels_enforcement
        .declared_only
        .iter()
        .cloned()
        .collect();

    let workflows_dir = workspace_root.join(".github").join("workflows");
    let enforced_labels = scan_workflows_for_labels(&workflows_dir, &declared_labels)?;

    let mut findings: Vec<String> = Vec::new();

    for label in &declared_labels {
        let consulted = enforced_labels.contains(label);
        let is_declared_only = declared_only.contains(label);

        match (consulted, is_declared_only) {
            (false, false) => findings.push(format!(
                "[label-undocumented] {label:?} is declared in [labels] but neither consulted by any workflow `if:` block nor listed in [labels_enforcement].declared_only — add it to declared_only or wire it into a workflow"
            )),
            (true, true) => findings.push(format!(
                "[label-promoted] {label:?} is listed in [labels_enforcement].declared_only but IS consulted by a workflow `if:` block — remove it from declared_only (policy classification is stale)"
            )),
            _ => {}
        }
    }

    for entry in &declared_only {
        if !declared_labels.contains(entry) {
            findings.push(format!(
                "[label-stray-declared-only] {entry:?} is listed in [labels_enforcement].declared_only but is not declared in [labels] — remove the stray entry"
            ));
        }
    }

    if findings.is_empty() {
        println!("check-label-enforcement: no findings.");
        return Ok(());
    }

    for f in &findings {
        eprintln!("  {f}");
    }
    println!("check-label-enforcement: {} finding(s).", findings.len());

    match mode {
        Mode::Advisory => {
            println!("(advisory mode: not failing)");
            Ok(())
        }
        Mode::BlockingAllowlist => Err(anyhow::anyhow!(
            "check-label-enforcement: {} finding(s) (blocking-allowlist mode)",
            findings.len()
        )),
    }
}

/// Scan every `.yml` workflow under `dir` for `contains(...labels.*.name,
/// '<label>')` references and return the set of declared labels found.
///
/// Tolerates both `'…'` and `"…"` quoting around the label literal because
/// either is valid YAML. Whitespace inside the `contains(...)` is permitted
/// but not aggressively normalised — workflow YAML in this repo uses the
/// canonical `contains(github.event.pull_request.labels.*.name, '…')`
/// shape, so a naive substring match is sufficient.
fn scan_workflows_for_labels(dir: &Path, declared: &BTreeSet<String>) -> Result<BTreeSet<String>> {
    let mut found = BTreeSet::new();
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(found),
        Err(err) => {
            return Err(anyhow::Error::new(err)).with_context(|| format!("read {}", dir.display()));
        }
    };
    for entry in entries {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("yml") {
            continue;
        }
        let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        for label in declared {
            let single = format!("contains(github.event.pull_request.labels.*.name, '{label}')");
            let double = format!("contains(github.event.pull_request.labels.*.name, \"{label}\")");
            if text.contains(&single) || text.contains(&double) {
                found.insert(label.clone());
            }
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    fn fixture_workspace(budget: &str, workflows: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        write(&dir.path().join("policy").join("ci-budget.toml"), budget);
        for (name, body) in workflows {
            write(
                &dir.path().join(".github").join("workflows").join(name),
                body,
            );
        }
        dir
    }

    const BASE_HEADER: &str = r#"
schema_version = 1
policy = "ci-budget"
owner = "test"
status = "advisory"
"#;

    #[test]
    fn label_consulted_by_workflow_is_enforced() {
        let budget = format!(
            "{BASE_HEADER}[labels]\nbdd = \"bdd\"\n\n[labels_enforcement]\ndeclared_only = []\n"
        );
        let workflow =
            "jobs:\n  smoke:\n    if: contains(github.event.pull_request.labels.*.name, 'bdd')\n";
        let dir = fixture_workspace(&budget, &[("bdd-testing.yml", workflow)]);

        run(dir.path(), Mode::BlockingAllowlist).expect("consistent fixture should pass");
    }

    #[test]
    fn label_in_declared_only_and_not_in_workflow_is_consistent() {
        let budget = format!(
            "{BASE_HEADER}[labels]\nripr_force = \"ripr\"\n\n[labels_enforcement]\ndeclared_only = [\"ripr\"]\n"
        );
        let dir = fixture_workspace(&budget, &[]);
        run(dir.path(), Mode::BlockingAllowlist).expect("declared-only-no-workflow is consistent");
    }

    #[test]
    fn label_not_consulted_and_not_declared_only_is_finding() {
        let budget = format!(
            "{BASE_HEADER}[labels]\nrelease_check = \"release-check\"\n\n[labels_enforcement]\ndeclared_only = []\n"
        );
        let dir = fixture_workspace(&budget, &[]);
        let err = run(dir.path(), Mode::BlockingAllowlist).unwrap_err();
        assert!(err.to_string().contains("1 finding"));
    }

    #[test]
    fn declared_only_label_consulted_by_workflow_is_finding() {
        let budget = format!(
            "{BASE_HEADER}[labels]\nripr_force = \"ripr\"\n\n[labels_enforcement]\ndeclared_only = [\"ripr\"]\n"
        );
        let workflow = "jobs:\n  advisory:\n    if: contains(github.event.pull_request.labels.*.name, 'ripr')\n";
        let dir = fixture_workspace(&budget, &[("ripr.yml", workflow)]);
        let err = run(dir.path(), Mode::BlockingAllowlist).unwrap_err();
        assert!(err.to_string().contains("1 finding"));
    }

    #[test]
    fn stray_declared_only_entry_is_finding() {
        let budget = format!(
            "{BASE_HEADER}[labels]\nbdd = \"bdd\"\n\n[labels_enforcement]\ndeclared_only = [\"ghost-label\"]\n"
        );
        let workflow =
            "jobs:\n  smoke:\n    if: contains(github.event.pull_request.labels.*.name, 'bdd')\n";
        let dir = fixture_workspace(&budget, &[("bdd.yml", workflow)]);
        let err = run(dir.path(), Mode::BlockingAllowlist).unwrap_err();
        assert!(err.to_string().contains("1 finding"));
    }

    #[test]
    fn advisory_mode_reports_but_does_not_fail() {
        let budget = format!(
            "{BASE_HEADER}[labels]\nrelease_check = \"release-check\"\n\n[labels_enforcement]\ndeclared_only = []\n"
        );
        let dir = fixture_workspace(&budget, &[]);
        run(dir.path(), Mode::Advisory).expect("advisory mode should not fail on findings");
    }

    #[test]
    fn double_quoted_label_in_workflow_is_detected() {
        let budget = format!(
            "{BASE_HEADER}[labels]\nfuzz = \"fuzz\"\n\n[labels_enforcement]\ndeclared_only = []\n"
        );
        let workflow = "jobs:\n  smoke:\n    if: contains(github.event.pull_request.labels.*.name, \"fuzz\")\n";
        let dir = fixture_workspace(&budget, &[("fuzz.yml", workflow)]);
        run(dir.path(), Mode::BlockingAllowlist).expect("double-quoted label should be detected");
    }

    #[test]
    fn full_ci_consulted_alongside_specific_label_is_enforced() {
        let budget = format!(
            "{BASE_HEADER}[labels]\nfull_ci = \"full-ci\"\nbdd = \"bdd\"\n\n[labels_enforcement]\ndeclared_only = []\n"
        );
        let workflow = "jobs:\n  smoke:\n    if: |\n      contains(github.event.pull_request.labels.*.name, 'bdd') ||\n      contains(github.event.pull_request.labels.*.name, 'full-ci')\n";
        let dir = fixture_workspace(&budget, &[("bdd.yml", workflow)]);
        run(dir.path(), Mode::BlockingAllowlist).expect("both labels enforced");
    }
}
