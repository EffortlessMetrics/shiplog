//! `cargo xtask check-actuals-coverage`
//!
//! Fourth proactive guard after #182 / #183 / #184. Cross-check that
//! every workflow declared as a lane `workflow_name` in
//! `policy/ci-lanes.toml` is subscribed by `ci-actuals.yml`'s
//! `on.workflow_run.workflows` list, and vice versa.
//!
//! This guards the #169 defect class directly: a workflow exists, runs
//! on every PR, costs real runner time, but its actuals are never
//! collected because it's missing from the `workflow_run` trigger list.
//! The forecast/actual loop then under-counts PR-fast spend.
//!
//! Two finding kinds (in `--mode blocking-allowlist`):
//!
//! 1. `[actuals-not-subscribed]` — A lane in `ci-lanes.toml` declares
//!    `workflow_name = "X"` but "X" is not in `ci-actuals.yml`'s
//!    `workflow_run.workflows`. Either add the subscription or
//!    explicitly list "X" in `[actuals_exemptions].not_subscribed`.
//!
//! 2. `[actuals-orphan-subscription]` — `ci-actuals.yml` subscribes
//!    to workflow "Y" but no lane in `ci-lanes.toml` declares
//!    `workflow_name = "Y"`. Either drop the subscription or add a
//!    lane (otherwise every job in workflow "Y" categorises as
//!    `lane.unknown`).
//!
//! Exemptions are encoded in a new `[actuals_exemptions]` section of
//! `policy/ci-lanes.toml`. The seed entry is `"CI Actuals"` itself —
//! subscribing the actuals emitter to its own completions would create
//! a feedback loop.
//!
//! See [`docs/ci/ci-actuals.md`](../../docs/ci/ci-actuals.md) for the
//! human-readable explanation of how the subscription list is consumed.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use crate::policy;
use crate::tasks::file_policy::Mode;

#[derive(Debug, Deserialize)]
struct LanesPolicy {
    #[serde(default)]
    lane: BTreeMap<String, LaneDef>,
    #[serde(default)]
    actuals_exemptions: ActualsExemptions,
}

#[derive(Debug, Deserialize)]
struct LaneDef {
    #[serde(default)]
    workflow_name: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct ActualsExemptions {
    #[serde(default)]
    not_subscribed: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ActualsYaml {
    on: OnTriggers,
}

#[derive(Debug, Deserialize)]
struct OnTriggers {
    workflow_run: WorkflowRunTrigger,
}

#[derive(Debug, Deserialize)]
struct WorkflowRunTrigger {
    workflows: Vec<String>,
}

pub fn run(workspace_root: &Path, mode: Mode) -> Result<()> {
    let policy_dir = policy::policy_dir(workspace_root);
    let lanes_path = policy_dir.join("ci-lanes.toml");
    let lanes: LanesPolicy = toml::from_str(
        &fs::read_to_string(&lanes_path)
            .with_context(|| format!("read {}", lanes_path.display()))?,
    )
    .with_context(|| format!("parse {}", lanes_path.display()))?;

    let actuals_path = workspace_root
        .join(".github")
        .join("workflows")
        .join("ci-actuals.yml");
    let actuals: ActualsYaml = serde_yaml::from_str(
        &fs::read_to_string(&actuals_path)
            .with_context(|| format!("read {}", actuals_path.display()))?,
    )
    .with_context(|| format!("parse {}", actuals_path.display()))?;

    let subscribed: BTreeSet<String> = actuals.on.workflow_run.workflows.iter().cloned().collect();
    let exemptions: BTreeSet<String> = lanes
        .actuals_exemptions
        .not_subscribed
        .iter()
        .cloned()
        .collect();
    let declared_workflows: BTreeSet<String> = lanes
        .lane
        .values()
        .filter_map(|l| l.workflow_name.clone())
        .collect();

    let mut findings: Vec<String> = Vec::new();

    for wf in &declared_workflows {
        if subscribed.contains(wf) || exemptions.contains(wf) {
            continue;
        }
        findings.push(format!(
            "[actuals-not-subscribed] workflow {wf:?} is declared as a lane workflow_name in policy/ci-lanes.toml but is not subscribed by ci-actuals.yml's workflow_run.workflows list — add it there or list it in [actuals_exemptions].not_subscribed"
        ));
    }

    for wf in &subscribed {
        if declared_workflows.contains(wf) || exemptions.contains(wf) {
            continue;
        }
        findings.push(format!(
            "[actuals-orphan-subscription] ci-actuals.yml subscribes to workflow {wf:?} but no lane in policy/ci-lanes.toml declares it as workflow_name — jobs in that workflow will categorise as lane.unknown. Either drop the subscription or add a matching lane."
        ));
    }

    for wf in &exemptions {
        if subscribed.contains(wf) {
            findings.push(format!(
                "[actuals-exemption-stale] workflow {wf:?} is listed in [actuals_exemptions].not_subscribed but IS subscribed by ci-actuals.yml — remove the exemption (or stop subscribing)"
            ));
        }
    }

    if findings.is_empty() {
        println!("check-actuals-coverage: no findings.");
        return Ok(());
    }
    for f in &findings {
        eprintln!("  {f}");
    }
    println!("check-actuals-coverage: {} finding(s).", findings.len());

    match mode {
        Mode::Advisory => {
            println!("(advisory mode: not failing)");
            Ok(())
        }
        Mode::BlockingAllowlist => Err(anyhow::anyhow!(
            "check-actuals-coverage: {} finding(s) (blocking-allowlist mode)",
            findings.len()
        )),
    }
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

    fn fixture(lanes: &str, actuals: &str) -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        write(&dir.path().join("policy").join("ci-lanes.toml"), lanes);
        write(
            &dir.path()
                .join(".github")
                .join("workflows")
                .join("ci-actuals.yml"),
            actuals,
        );
        dir
    }

    const LANES_HEADER: &str = r#"
schema_version = 1
policy = "ci-lanes"
owner = "test"
status = "advisory"
"#;

    fn actuals_yaml(workflows: &[&str]) -> String {
        let mut s = String::from("name: CI Actuals\non:\n  workflow_run:\n    workflows:\n");
        for w in workflows {
            s.push_str(&format!("      - {w:?}\n"));
        }
        s.push_str("    types: [completed]\njobs:\n  collect:\n    runs-on: ubuntu-latest\n    steps: []\n");
        s
    }

    #[test]
    fn fully_consistent_coverage_passes() {
        let lanes = format!(
            "{LANES_HEADER}[lane.ci_check]\nworkflow_name = \"CI\"\n\n[lane.bdd]\nworkflow_name = \"BDD Testing\"\n\n[actuals_exemptions]\nnot_subscribed = [\"CI Actuals\"]\n"
        );
        let actuals = actuals_yaml(&["CI", "BDD Testing"]);
        let dir = fixture(&lanes, &actuals);
        run(dir.path(), Mode::BlockingAllowlist).expect("consistent fixture should pass");
    }

    #[test]
    fn declared_lane_missing_subscription_is_finding() {
        let lanes = format!(
            "{LANES_HEADER}[lane.ci_check]\nworkflow_name = \"CI\"\n\n[lane.bdd_smoke]\nworkflow_name = \"BDD Smoke\"\n\n[actuals_exemptions]\nnot_subscribed = []\n"
        );
        // BDD Smoke declared but ci-actuals subscribes only CI
        let actuals = actuals_yaml(&["CI"]);
        let dir = fixture(&lanes, &actuals);
        let err = run(dir.path(), Mode::BlockingAllowlist).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("1 finding"), "expected 1 finding, got: {msg}");
    }

    #[test]
    fn subscribed_workflow_without_lane_is_finding() {
        let lanes = format!(
            "{LANES_HEADER}[lane.ci_check]\nworkflow_name = \"CI\"\n\n[actuals_exemptions]\nnot_subscribed = []\n"
        );
        // ci-actuals subscribes to OrphanWorkflow which no lane declares
        let actuals = actuals_yaml(&["CI", "OrphanWorkflow"]);
        let dir = fixture(&lanes, &actuals);
        let err = run(dir.path(), Mode::BlockingAllowlist).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("1 finding"), "expected 1 finding, got: {msg}");
    }

    #[test]
    fn exemption_satisfies_both_directions() {
        // CI Actuals is declared as a lane workflow_name AND exempt; it
        // should not appear in either finding direction even when not
        // subscribed.
        let lanes = format!(
            "{LANES_HEADER}[lane.ci_actuals]\nworkflow_name = \"CI Actuals\"\n\n[actuals_exemptions]\nnot_subscribed = [\"CI Actuals\"]\n"
        );
        let actuals = actuals_yaml(&[]);
        let dir = fixture(&lanes, &actuals);
        run(dir.path(), Mode::BlockingAllowlist)
            .expect("exempt lane should pass even when not subscribed");
    }

    #[test]
    fn stale_exemption_for_subscribed_workflow_is_finding() {
        // Workflow is subscribed but also listed as exempt — likely
        // means someone added a subscription without removing the
        // exemption. Flag it.
        let lanes = format!(
            "{LANES_HEADER}[lane.ci_check]\nworkflow_name = \"CI\"\n\n[lane.bdd]\nworkflow_name = \"BDD Testing\"\n\n[actuals_exemptions]\nnot_subscribed = [\"BDD Testing\", \"CI Actuals\"]\n"
        );
        let actuals = actuals_yaml(&["CI", "BDD Testing"]);
        let dir = fixture(&lanes, &actuals);
        let err = run(dir.path(), Mode::BlockingAllowlist).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("1 finding"), "expected 1 finding, got: {msg}");
    }

    #[test]
    fn advisory_mode_reports_but_does_not_fail() {
        let lanes = format!(
            "{LANES_HEADER}[lane.bdd_smoke]\nworkflow_name = \"BDD Smoke\"\n\n[actuals_exemptions]\nnot_subscribed = []\n"
        );
        let actuals = actuals_yaml(&["CI"]);
        let dir = fixture(&lanes, &actuals);
        run(dir.path(), Mode::Advisory).expect("advisory should report but not fail");
    }

    #[test]
    fn no_workflow_run_workflows_yields_orphan_findings_only_via_subscriptions() {
        // Empty subscription list + one declared lane → 1 finding
        // (declared-not-subscribed; no orphan possible).
        let lanes = format!(
            "{LANES_HEADER}[lane.ci_check]\nworkflow_name = \"CI\"\n\n[actuals_exemptions]\nnot_subscribed = []\n"
        );
        let actuals = actuals_yaml(&[]);
        let dir = fixture(&lanes, &actuals);
        let err = run(dir.path(), Mode::BlockingAllowlist).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("1 finding"), "expected 1 finding, got: {msg}");
    }

    #[test]
    fn multiple_findings_accumulate() {
        let lanes = format!(
            "{LANES_HEADER}[lane.ci_check]\nworkflow_name = \"CI\"\n\n[lane.bdd_smoke]\nworkflow_name = \"BDD Smoke\"\n\n[actuals_exemptions]\nnot_subscribed = []\n"
        );
        // CI declared but not subscribed (1 finding);
        // OrphanA, OrphanB subscribed but no lane (2 findings).
        let actuals = actuals_yaml(&["OrphanA", "OrphanB"]);
        let dir = fixture(&lanes, &actuals);
        let err = run(dir.path(), Mode::BlockingAllowlist).unwrap_err();
        let msg = err.to_string();
        // declared-not-subscribed: CI, BDD Smoke (both) → 2
        // orphan-subscription: OrphanA, OrphanB → 2
        // total: 4
        assert!(msg.contains("4 finding"), "expected 4 findings, got: {msg}");
    }
}
