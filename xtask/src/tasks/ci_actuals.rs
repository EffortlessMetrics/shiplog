//! `cargo xtask ci actuals`
//!
//! Reads a GitHub Actions jobs API response (e.g. captured via
//! `gh api repos/{owner}/{repo}/actions/runs/{run_id}/jobs`),
//! joins each job to a lane in `policy/ci-lanes.toml`, computes
//! actual_seconds + actual_lem from the job timings + the runner
//! multiplier in `policy/ci-budget.toml`, and emits
//! `target/ci/ci-actuals.json` against
//! [`contracts/schemas/ci-actuals.v1.schema.json`](../../contracts/schemas/ci-actuals.v1.schema.json).
//!
//! See [`docs/ci/ci-actuals.md`](../../docs/ci/ci-actuals.md) for the
//! human reference.
//!
//! Always advisory: prints the drift summary; never exits non-zero on
//! drift. Hard enforcement is a follow-up release decision.

use anyhow::{Context, Result};
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::policy;

// ────────────────────────────────────────────────────────────────────────
// Inputs
// ────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ActualsInputs {
    pub workspace_root: PathBuf,
    pub run_id: String,
    pub workflow_name: String,
    pub head_sha: String,
    pub pr_number: Option<u32>,
    pub jobs_input: PathBuf,
    pub output: PathBuf,
}

// ────────────────────────────────────────────────────────────────────────
// GitHub Actions jobs API shape (subset of fields we consume)
// ────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct JobsResponse {
    jobs: Vec<JobInput>,
}

#[derive(Debug, Deserialize)]
struct JobInput {
    name: String,
    #[serde(default)]
    labels: Vec<String>,
    started_at: Option<String>,
    completed_at: Option<String>,
    conclusion: Option<String>,
    status: String,
}

// ────────────────────────────────────────────────────────────────────────
// Policy structs (only the fields we consume)
// ────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct CiBudgetPolicy {
    runner_multipliers: BTreeMap<String, f64>,
}

#[derive(Debug, Deserialize)]
struct CiLanesPolicy {
    lane: BTreeMap<String, LaneDef>,
}

#[derive(Debug, Deserialize)]
struct LaneDef {
    // `runner` exists in the lane TOML but actuals look up the multiplier from
    // the per-job runner label reported by the GHA API, not from policy. Field
    // intentionally not read here.
    base_lem: f64,
    #[serde(default)]
    workflow_name: Option<String>,
    #[serde(default)]
    job_name: Option<String>,
}

// ────────────────────────────────────────────────────────────────────────
// Output structs (mirror of ci-actuals.v1.schema.json)
// ────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct CiActuals {
    schema_version: u32,
    repo: String,
    run_id: String,
    workflow: String,
    head_sha: String,
    pr_number: Option<u32>,
    lanes: Vec<LaneActual>,
    totals: Totals,
}

#[derive(Debug, Serialize)]
struct LaneActual {
    lane_id: String,
    workflow: String,
    job: String,
    runner: String,
    runner_multiplier: f64,
    estimated_lem: f64,
    started_at: Option<String>,
    completed_at: Option<String>,
    actual_seconds: f64,
    actual_lem: f64,
    cache_hit: Option<bool>,
    cache_key: Option<String>,
    conclusion: String,
    skip_reason: Option<String>,
    skip_detail: Option<String>,
}

#[derive(Debug, Serialize)]
struct Totals {
    estimated_lem: f64,
    actual_lem: f64,
    drift_pct: f64,
}

// ────────────────────────────────────────────────────────────────────────
// Entry point
// ────────────────────────────────────────────────────────────────────────

pub fn run(inputs: ActualsInputs) -> Result<()> {
    let policy_dir = policy::policy_dir(&inputs.workspace_root);
    let budget: CiBudgetPolicy = load_policy(&policy_dir, "ci-budget.toml")?;
    let lanes: CiLanesPolicy = load_policy(&policy_dir, "ci-lanes.toml")?;

    let jobs_text = fs::read_to_string(&inputs.jobs_input)
        .with_context(|| format!("read jobs input {}", inputs.jobs_input.display()))?;
    let jobs_response: JobsResponse =
        serde_json::from_str(&jobs_text).context("parse jobs JSON")?;

    let mut lane_actuals = Vec::new();
    for job in &jobs_response.jobs {
        let actual = build_actual(job, &inputs.workflow_name, &lanes.lane, &budget);
        lane_actuals.push(actual);
    }

    let estimated_total: f64 = lane_actuals
        .iter()
        .filter(|a| a.conclusion != "skipped")
        .map(|a| a.estimated_lem)
        .sum();
    let actual_total: f64 = lane_actuals.iter().map(|a| a.actual_lem).sum();
    let drift_pct = if estimated_total > 0.0 {
        (actual_total - estimated_total) / estimated_total * 100.0
    } else {
        0.0
    };

    let actuals = CiActuals {
        schema_version: 1,
        repo: "shiplog".to_string(),
        run_id: inputs.run_id,
        workflow: inputs.workflow_name,
        head_sha: inputs.head_sha,
        pr_number: inputs.pr_number,
        lanes: lane_actuals,
        totals: Totals {
            estimated_lem: estimated_total,
            actual_lem: actual_total,
            drift_pct,
        },
    };

    write_actuals_json(&inputs.output, &actuals)?;

    if let Ok(summary_path) = std::env::var("GITHUB_STEP_SUMMARY") {
        write_step_summary(&PathBuf::from(summary_path), &actuals)?;
    }

    println!("Wrote CI actuals to {}", inputs.output.display());
    println!(
        "  jobs: {}  estimated: {:.1} LEM  actual: {:.1} LEM  drift: {:+.1}%",
        actuals.lanes.len(),
        actuals.totals.estimated_lem,
        actuals.totals.actual_lem,
        actuals.totals.drift_pct
    );

    Ok(())
}

// ────────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────────

fn load_policy<T: for<'de> Deserialize<'de>>(policy_dir: &Path, file: &str) -> Result<T> {
    let path = policy_dir.join(file);
    let text =
        fs::read_to_string(&path).with_context(|| format!("read policy {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("parse policy {}", path.display()))
}

fn build_actual(
    job: &JobInput,
    workflow_name: &str,
    lanes: &BTreeMap<String, LaneDef>,
    budget: &CiBudgetPolicy,
) -> LaneActual {
    let conclusion_raw = job
        .conclusion
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(job.status.as_str());
    let conclusion = normalize_conclusion(conclusion_raw);

    let lane_match = match_lane(workflow_name, &job.name, lanes);
    let runner_label = job.labels.first().cloned().unwrap_or_default();
    let runner_multiplier = lookup_multiplier(&runner_label, &budget.runner_multipliers);
    let actual_seconds = compute_seconds(job.started_at.as_deref(), job.completed_at.as_deref());
    let actual_lem = (actual_seconds / 60.0) * runner_multiplier;
    let (lane_id, estimated_lem) = match &lane_match {
        Some((id, lane)) => (format!("lane.{id}"), lane.base_lem),
        None => ("lane.unknown".to_string(), 0.0),
    };
    let (skip_reason, skip_detail) = if conclusion == "skipped" {
        (
            Some("nightly-only".to_string()),
            Some(format!(
                "GitHub Actions reported skip for job {:?} in workflow {workflow_name:?}",
                job.name
            )),
        )
    } else {
        (None, None)
    };

    LaneActual {
        lane_id,
        workflow: workflow_name.to_string(),
        job: job.name.clone(),
        runner: runner_label,
        runner_multiplier,
        estimated_lem,
        started_at: job.started_at.clone(),
        completed_at: job.completed_at.clone(),
        actual_seconds,
        actual_lem,
        cache_hit: None,
        cache_key: None,
        conclusion: conclusion.to_string(),
        skip_reason,
        skip_detail,
    }
}

/// Match a job (workflow_name, job_name) to a lane in the policy. Prefer
/// lanes with both `workflow_name` and `job_name` set; fall back to lanes
/// with `workflow_name` matching but no `job_name` (workflow-wide
/// catch-all).
fn match_lane<'a>(
    workflow_name: &str,
    job_name: &str,
    lanes: &'a BTreeMap<String, LaneDef>,
) -> Option<(&'a String, &'a LaneDef)> {
    // First pass: exact (workflow_name, job_name) match.
    for (id, lane) in lanes {
        if let Some(wf) = &lane.workflow_name
            && let Some(jn) = &lane.job_name
            && wf == workflow_name
            && jn == job_name
        {
            return Some((id, lane));
        }
    }
    // Second pass: workflow_name match with no job_name set (catch-all).
    for (id, lane) in lanes {
        if let Some(wf) = &lane.workflow_name
            && wf == workflow_name
            && lane.job_name.is_none()
        {
            return Some((id, lane));
        }
    }
    None
}

fn lookup_multiplier(runner_label: &str, multipliers: &BTreeMap<String, f64>) -> f64 {
    // Multipliers are TOML keys like `ubuntu_24_04`; runner labels from the
    // GitHub API are `ubuntu-24.04` / `ubuntu-latest` / etc. Try both forms.
    multipliers
        .get(runner_label)
        .copied()
        .or_else(|| {
            multipliers
                .get(&runner_label.replace(['-', '.'], "_"))
                .copied()
        })
        .unwrap_or(1.0)
}

fn normalize_conclusion(raw: &str) -> &'static str {
    match raw {
        "success" => "success",
        "failure" => "failure",
        "skipped" => "skipped",
        "cancelled" => "cancelled",
        "timed_out" => "timed_out",
        // GitHub may also return "neutral" / "action_required" / "in_progress" / null;
        // normalize to "failure" so the schema's enum stays satisfied while making
        // the unusual case visible to a reviewer.
        _ => "failure",
    }
}

fn compute_seconds(started_at: Option<&str>, completed_at: Option<&str>) -> f64 {
    let (Some(start), Some(end)) = (started_at, completed_at) else {
        return 0.0;
    };
    let Ok(start_dt) = DateTime::parse_from_rfc3339(start) else {
        return 0.0;
    };
    let Ok(end_dt) = DateTime::parse_from_rfc3339(end) else {
        return 0.0;
    };
    let secs = (end_dt - start_dt).num_seconds();
    if secs < 0 { 0.0 } else { secs as f64 }
}

fn write_actuals_json(output: &Path, actuals: &CiActuals) -> Result<()> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create dir {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(actuals).context("serialize CI actuals to JSON")?;
    fs::write(output, json + "\n").with_context(|| format!("write {}", output.display()))?;
    Ok(())
}

fn write_step_summary(summary_path: &Path, actuals: &CiActuals) -> Result<()> {
    use std::fmt::Write;
    let mut s = String::new();
    let _ = writeln!(s, "## CI Actuals — `{}`", actuals.workflow);
    let _ = writeln!(s);
    let _ = writeln!(
        s,
        "Run `{}` · head `{}` · pr `{}`",
        actuals.run_id,
        &actuals.head_sha[..7.min(actuals.head_sha.len())],
        actuals.pr_number.map_or("—".to_string(), |n| n.to_string())
    );
    let _ = writeln!(s);
    let _ = writeln!(
        s,
        "**Estimated:** {:.1} LEM  ·  **Actual:** {:.1} LEM  ·  **Drift:** {:+.1}%",
        actuals.totals.estimated_lem, actuals.totals.actual_lem, actuals.totals.drift_pct
    );
    let _ = writeln!(s);
    let _ = writeln!(s, "### Per-job actuals ({})", actuals.lanes.len());
    let _ = writeln!(s);
    let _ = writeln!(
        s,
        "| lane | job | runner | est. LEM | actual LEM | drift | seconds | conclusion |"
    );
    let _ = writeln!(s, "|---|---|---|---:|---:|---:|---:|---|");
    for la in &actuals.lanes {
        let drift = if la.estimated_lem > 0.0 {
            format!(
                "{:+.0}%",
                (la.actual_lem - la.estimated_lem) / la.estimated_lem * 100.0
            )
        } else {
            "—".to_string()
        };
        let _ = writeln!(
            s,
            "| `{}` | `{}` | `{}` | {:.1} | {:.1} | {} | {:.0} | {} |",
            la.lane_id,
            la.job,
            la.runner,
            la.estimated_lem,
            la.actual_lem,
            drift,
            la.actual_seconds,
            la.conclusion
        );
    }
    fs::write(summary_path, s)
        .with_context(|| format!("write GitHub step summary {}", summary_path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    // Test-side mirror of the production CiActuals to verify it round-trips
    // through serde_json. Fields are present-but-unused so the deserialization
    // itself exercises the schema.
    #[allow(dead_code)]
    #[derive(Debug, Deserialize)]
    struct ActualsReadback {
        schema_version: u32,
        run_id: String,
        workflow: String,
        head_sha: String,
        pr_number: Option<u32>,
        lanes: Vec<LaneReadback>,
        totals: TotalsReadback,
    }
    #[allow(dead_code)]
    #[derive(Debug, Deserialize)]
    struct LaneReadback {
        lane_id: String,
        workflow: String,
        job: String,
        runner: String,
        runner_multiplier: f64,
        estimated_lem: f64,
        actual_seconds: f64,
        actual_lem: f64,
        cache_hit: Option<bool>,
        cache_key: Option<String>,
        conclusion: String,
        skip_reason: Option<String>,
    }
    #[allow(dead_code)]
    #[derive(Debug, Deserialize)]
    struct TotalsReadback {
        estimated_lem: f64,
        actual_lem: f64,
        drift_pct: f64,
    }

    fn write(p: &Path, content: &str) {
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(p, content).unwrap();
    }

    fn fixture_workspace() -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        let policy = dir.path().join("policy");
        fs::create_dir_all(&policy).unwrap();
        write(
            &policy.join("ci-budget.toml"),
            r#"
schema_version = 1
policy = "ci-budget"
owner = "x"
status = "advisory"

[budget]
preferred_default_lem = 25
default_limit_lem = 35
elevated_limit_lem = 75
hard_limit_lem = 125

[runner_multipliers]
ubuntu_latest = 1.0
ubuntu_24_04 = 1.0
windows_latest = 2.0

[labels]
full_ci = "full-ci"
"#,
        );
        write(
            &policy.join("ci-lanes.toml"),
            r#"
schema_version = 1
policy = "ci-lanes"
owner = "x"
status = "advisory"

[lane.ci_check]
description = "Primary"
intent = "Rust correctness"
runner = "ubuntu_24_04"
base_lem = 12
default_pr = true
blocking = true
workflow = ".github/workflows/ci.yml"
workflow_name = "CI"
job_name = "Check (ubuntu-latest)"

[lane.ci_check_windows]
description = "Windows"
intent = "Rust correctness (Windows parity)"
runner = "windows_latest"
base_lem = 36
default_pr = true
blocking = true
workflow = ".github/workflows/ci.yml"
workflow_name = "CI"
job_name = "Check (windows-latest)"

[lane.bdd]
description = "BDD"
intent = "user-flow"
runner = "ubuntu_24_04"
base_lem = 32
default_pr = true
blocking = false
workflow = ".github/workflows/bdd-testing.yml"
workflow_name = "BDD Testing"
"#,
        );
        dir
    }

    fn write_jobs(dir: &Path, jobs_json: &str) -> PathBuf {
        let path = dir.join("jobs.json");
        fs::write(&path, jobs_json).unwrap();
        path
    }

    fn run_actuals(workspace_root: &Path, jobs_json: &str, workflow_name: &str) -> ActualsReadback {
        let input = write_jobs(workspace_root, jobs_json);
        let output = workspace_root.join("actuals.json");
        run(ActualsInputs {
            workspace_root: workspace_root.to_path_buf(),
            run_id: "999".to_string(),
            workflow_name: workflow_name.to_string(),
            head_sha: "a".repeat(40),
            pr_number: Some(148),
            jobs_input: input,
            output: output.clone(),
        })
        .expect("actuals should succeed");
        let json = fs::read_to_string(&output).unwrap();
        serde_json::from_str(&json).expect("parseable actuals")
    }

    #[test]
    fn maps_jobs_to_lanes_with_workflow_and_job_name() {
        let dir = fixture_workspace();
        let actuals = run_actuals(
            dir.path(),
            r#"{
                "jobs": [
                    {
                        "name": "Check (ubuntu-latest)",
                        "labels": ["ubuntu-latest"],
                        "started_at": "2026-05-10T01:32:37Z",
                        "completed_at": "2026-05-10T01:37:52Z",
                        "conclusion": "success",
                        "status": "completed"
                    },
                    {
                        "name": "Check (windows-latest)",
                        "labels": ["windows-latest"],
                        "started_at": "2026-05-10T01:32:37Z",
                        "completed_at": "2026-05-10T01:50:37Z",
                        "conclusion": "success",
                        "status": "completed"
                    }
                ]
            }"#,
            "CI",
        );
        assert_eq!(actuals.schema_version, 1);
        assert_eq!(actuals.lanes.len(), 2);

        let ubuntu = actuals
            .lanes
            .iter()
            .find(|l| l.job == "Check (ubuntu-latest)")
            .unwrap();
        assert_eq!(ubuntu.lane_id, "lane.ci_check");
        assert_eq!(ubuntu.runner_multiplier, 1.0);
        // 5min15sec = 315 seconds = 5.25 min × 1.0 = 5.25 LEM
        assert!(ubuntu.actual_seconds > 314.0 && ubuntu.actual_seconds < 316.0);
        assert!(ubuntu.actual_lem > 5.2 && ubuntu.actual_lem < 5.3);
        assert_eq!(ubuntu.estimated_lem, 12.0);
        assert_eq!(ubuntu.conclusion, "success");
        assert_eq!(ubuntu.skip_reason, None);

        let win = actuals
            .lanes
            .iter()
            .find(|l| l.job == "Check (windows-latest)")
            .unwrap();
        assert_eq!(win.lane_id, "lane.ci_check_windows");
        assert_eq!(win.runner_multiplier, 2.0);
        // 18 min × 2.0 = 36 LEM
        assert!(win.actual_lem > 35.5 && win.actual_lem < 36.5);
    }

    #[test]
    fn falls_back_to_workflow_catchall_lane_when_job_name_unspecified() {
        let dir = fixture_workspace();
        let actuals = run_actuals(
            dir.path(),
            r#"{
                "jobs": [
                    {
                        "name": "BDD & Property Tests",
                        "labels": ["ubuntu-latest"],
                        "started_at": "2026-05-10T01:32:43Z",
                        "completed_at": "2026-05-10T01:33:46Z",
                        "conclusion": "success",
                        "status": "completed"
                    },
                    {
                        "name": "Ingest Crate Tests",
                        "labels": ["ubuntu-latest"],
                        "started_at": "2026-05-10T01:32:37Z",
                        "completed_at": "2026-05-10T01:33:40Z",
                        "conclusion": "success",
                        "status": "completed"
                    }
                ]
            }"#,
            "BDD Testing",
        );
        // Both jobs map to lane.bdd via the workflow-catchall (no job_name in lane).
        assert_eq!(actuals.lanes.len(), 2);
        assert!(actuals.lanes.iter().all(|l| l.lane_id == "lane.bdd"));
    }

    #[test]
    fn unknown_workflow_yields_lane_unknown() {
        let dir = fixture_workspace();
        let actuals = run_actuals(
            dir.path(),
            r#"{
                "jobs": [
                    {
                        "name": "Mystery Job",
                        "labels": ["ubuntu-latest"],
                        "started_at": "2026-05-10T01:32:37Z",
                        "completed_at": "2026-05-10T01:33:00Z",
                        "conclusion": "success",
                        "status": "completed"
                    }
                ]
            }"#,
            "Some Other Workflow",
        );
        assert_eq!(actuals.lanes.len(), 1);
        assert_eq!(actuals.lanes[0].lane_id, "lane.unknown");
        assert_eq!(actuals.lanes[0].estimated_lem, 0.0);
    }

    #[test]
    fn skipped_job_emits_skip_reason() {
        let dir = fixture_workspace();
        let actuals = run_actuals(
            dir.path(),
            r#"{
                "jobs": [
                    {
                        "name": "Check (ubuntu-latest)",
                        "labels": ["ubuntu-latest"],
                        "started_at": null,
                        "completed_at": null,
                        "conclusion": "skipped",
                        "status": "completed"
                    }
                ]
            }"#,
            "CI",
        );
        let lane = &actuals.lanes[0];
        assert_eq!(lane.conclusion, "skipped");
        assert!(lane.skip_reason.is_some());
        assert_eq!(lane.actual_seconds, 0.0);
        assert_eq!(lane.actual_lem, 0.0);
    }

    #[test]
    fn neutral_or_unexpected_conclusion_normalises_to_failure() {
        let dir = fixture_workspace();
        let actuals = run_actuals(
            dir.path(),
            r#"{
                "jobs": [
                    {
                        "name": "Check (ubuntu-latest)",
                        "labels": ["ubuntu-latest"],
                        "started_at": "2026-05-10T01:32:37Z",
                        "completed_at": "2026-05-10T01:37:52Z",
                        "conclusion": "neutral",
                        "status": "completed"
                    }
                ]
            }"#,
            "CI",
        );
        assert_eq!(actuals.lanes[0].conclusion, "failure");
    }

    #[test]
    fn totals_compute_drift_relative_to_estimated() {
        let dir = fixture_workspace();
        let actuals = run_actuals(
            dir.path(),
            r#"{
                "jobs": [
                    {
                        "name": "Check (ubuntu-latest)",
                        "labels": ["ubuntu-latest"],
                        "started_at": "2026-05-10T01:32:37Z",
                        "completed_at": "2026-05-10T01:37:52Z",
                        "conclusion": "success",
                        "status": "completed"
                    }
                ]
            }"#,
            "CI",
        );
        // estimated 12, actual ~5.25 → drift roughly -56%
        assert!(actuals.totals.estimated_lem > 11.9 && actuals.totals.estimated_lem < 12.1);
        assert!(actuals.totals.actual_lem > 5.2 && actuals.totals.actual_lem < 5.3);
        assert!(actuals.totals.drift_pct < -50.0 && actuals.totals.drift_pct > -60.0);
    }

    #[test]
    fn skipped_jobs_excluded_from_estimated_total() {
        let dir = fixture_workspace();
        let actuals = run_actuals(
            dir.path(),
            r#"{
                "jobs": [
                    {
                        "name": "Check (ubuntu-latest)",
                        "labels": ["ubuntu-latest"],
                        "started_at": "2026-05-10T01:32:37Z",
                        "completed_at": "2026-05-10T01:37:52Z",
                        "conclusion": "success",
                        "status": "completed"
                    },
                    {
                        "name": "Check (windows-latest)",
                        "labels": ["windows-latest"],
                        "started_at": null,
                        "completed_at": null,
                        "conclusion": "skipped",
                        "status": "completed"
                    }
                ]
            }"#,
            "CI",
        );
        // Only the success job (estimated 12) contributes to estimated_total.
        assert!(actuals.totals.estimated_lem > 11.9 && actuals.totals.estimated_lem < 12.1);
    }
}
