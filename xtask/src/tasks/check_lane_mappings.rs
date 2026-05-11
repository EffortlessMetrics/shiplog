//! `cargo xtask check-lane-mappings`
//!
//! Third proactive guard after #182 + #183. Verify that every lane in
//! `policy/ci-lanes.toml` whose definition declares a `workflow`,
//! `workflow_name`, or `job_name` resolves to a real workflow file and
//! (where applicable) a real job inside that file.
//!
//! This is the structural complement to `check-risk-pack-integrity`:
//! the risk-pack check ensures policy references point at real lanes;
//! this check ensures real lanes point at real workflow/job artefacts.
//! Together they remove the two ways a lane reference can be silently
//! wrong.
//!
//! Concrete classes of regression this catches:
//!
//! 1. **Stale `workflow` path** — a workflow YAML was renamed or moved
//!    and the lane policy still references the old path.
//! 2. **Stale `workflow_name`** — the workflow's top-level `name:`
//!    field was changed and the lane policy still references the old
//!    display name. CI Actuals looks up lanes by `workflow_name`, so
//!    a mismatch silently demotes every job in the workflow to
//!    `lane.unknown` (the bug #167 fixed at the *consumer* end; this
//!    guard prevents the *policy* end from drifting back).
//! 3. **Stale `job_name`** — the workflow's job display name was
//!    renamed and the lane policy still points at the old name.
//! 4. **Missing `workflow_name`/`job_name`** — a non-catchall lane
//!    declared `workflow` but forgot the structural fields; CI Actuals
//!    can't match jobs to it. (Like the `lane.ripr_advisory` shape
//!    #169 fixed.)
//!
//! Catchall lanes — those with `workflow_name` set but `job_name`
//! intentionally omitted — are accepted. This matches the comment
//! convention in `policy/ci-lanes.toml` ("No job_name set:
//! workflow-catchall applies to all N jobs").
//!
//! See [`docs/ci/ci-lane-map.md`](../../docs/ci/ci-lane-map.md) for the
//! human-readable lane catalogue and the structure each entry honours.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::policy;
use crate::tasks::file_policy::Mode;

#[derive(Debug, Deserialize)]
struct LanesPolicy {
    #[serde(default)]
    lane: BTreeMap<String, LaneDef>,
}

#[derive(Debug, Deserialize)]
struct LaneDef {
    #[serde(default)]
    workflow: Option<String>,
    #[serde(default)]
    workflow_name: Option<String>,
    #[serde(default)]
    job_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WorkflowYaml {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    jobs: BTreeMap<String, JobDef>,
}

#[derive(Debug, Deserialize)]
struct JobDef {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    strategy: Option<Strategy>,
}

#[derive(Debug, Deserialize)]
struct Strategy {
    #[serde(default)]
    matrix: Option<serde_yaml::Value>,
}

pub fn run(workspace_root: &Path, mode: Mode) -> Result<()> {
    let policy_dir = policy::policy_dir(workspace_root);
    let lanes_path = policy_dir.join("ci-lanes.toml");
    let lanes_text = fs::read_to_string(&lanes_path)
        .with_context(|| format!("read {}", lanes_path.display()))?;
    let lanes: LanesPolicy =
        toml::from_str(&lanes_text).with_context(|| format!("parse {}", lanes_path.display()))?;

    let mut findings: Vec<String> = Vec::new();

    for (lane_id, lane) in &lanes.lane {
        // Skip lanes that declare none of workflow / workflow_name / job_name —
        // they're metadata-only entries (e.g. lanes that exist purely for
        // forecast-grouping). A future tick can decide whether those should
        // be required to point at a workflow.
        if lane.workflow.is_none() && lane.workflow_name.is_none() && lane.job_name.is_none() {
            continue;
        }

        // `job_name` without `workflow_name` is unmatchable; CI Actuals
        // categorises by (workflow_name, job_name). Flag it.
        if lane.job_name.is_some() && lane.workflow_name.is_none() {
            findings.push(format!(
                "[lane-missing-workflow-name] lane.{lane_id} declares job_name {:?} without workflow_name — CI Actuals can't match this lane",
                lane.job_name.as_deref().unwrap_or("")
            ));
        }

        let Some(workflow_rel) = lane.workflow.as_deref() else {
            // Lane has workflow_name but no workflow path. Allowed
            // historically; can't structurally verify the YAML. Skip.
            continue;
        };

        let workflow_abs = workspace_root.join(workflow_rel);
        if !workflow_abs.exists() {
            findings.push(format!(
                "[lane-missing-workflow-file] lane.{lane_id} references workflow {workflow_rel:?} which does not exist on disk"
            ));
            continue;
        }

        let parsed = match load_workflow(&workflow_abs) {
            Ok(parsed) => parsed,
            Err(err) => {
                findings.push(format!(
                    "[lane-workflow-parse-failed] lane.{lane_id} references workflow {workflow_rel:?} but YAML failed to parse: {err}"
                ));
                continue;
            }
        };

        if let Some(declared_workflow_name) = lane.workflow_name.as_deref() {
            match parsed.name.as_deref() {
                Some(actual) if actual == declared_workflow_name => {}
                Some(actual) => findings.push(format!(
                    "[lane-workflow-name-mismatch] lane.{lane_id} declares workflow_name {declared_workflow_name:?} but {workflow_rel:?} top-level name is {actual:?}"
                )),
                None => findings.push(format!(
                    "[lane-workflow-name-missing-in-yaml] lane.{lane_id} declares workflow_name {declared_workflow_name:?} but {workflow_rel:?} has no top-level name field"
                )),
            }
        }

        if let Some(declared_job_name) = lane.job_name.as_deref() {
            let display_names = collect_job_display_names(&parsed.jobs);
            if !display_names.contains(declared_job_name) {
                let known: Vec<&str> = display_names.iter().map(|s| s.as_str()).collect();
                findings.push(format!(
                    "[lane-unknown-job-name] lane.{lane_id} declares job_name {declared_job_name:?} but {workflow_rel:?} jobs map to display names {known:?}"
                ));
            }
        }
    }

    if findings.is_empty() {
        println!("check-lane-mappings: no findings.");
        return Ok(());
    }
    for f in &findings {
        eprintln!("  {f}");
    }
    println!("check-lane-mappings: {} finding(s).", findings.len());

    match mode {
        Mode::Advisory => {
            println!("(advisory mode: not failing)");
            Ok(())
        }
        Mode::BlockingAllowlist => Err(anyhow::anyhow!(
            "check-lane-mappings: {} finding(s) (blocking-allowlist mode)",
            findings.len()
        )),
    }
}

fn load_workflow(path: &PathBuf) -> Result<WorkflowYaml> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    serde_yaml::from_str(&text).with_context(|| format!("parse YAML at {}", path.display()))
}

/// Collect the GitHub Actions UI display names for every job in `jobs`,
/// expanding `${{ matrix.<var> }}` references using each job's
/// `strategy.matrix` declaration. Supports both simple list form
/// (`matrix.os: [ubuntu-latest, windows-latest]`) and `matrix.include`
/// (the value of `<var>` is pulled from each include entry).
///
/// A job's base display name is `jobs.<id>.name` when set, otherwise the
/// job id. Matrix expansion produces one display name per Cartesian-product
/// combination of the referenced matrix variables.
fn collect_job_display_names(jobs: &BTreeMap<String, JobDef>) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for (id, def) in jobs {
        let template = def.name.clone().unwrap_or_else(|| id.clone());
        let matrix = def.strategy.as_ref().and_then(|s| s.matrix.as_ref());
        out.extend(expand_job_template(&template, matrix));
    }
    out
}

/// Expand `${{ matrix.<var> }}` references in `template` against the
/// optional `matrix` mapping. Returns the literal template (single element)
/// when there's no matrix or when expansion cannot resolve all variables.
fn expand_job_template(template: &str, matrix: Option<&serde_yaml::Value>) -> Vec<String> {
    static MATRIX_REF: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let re =
        MATRIX_REF.get_or_init(|| regex::Regex::new(r"\$\{\{\s*matrix\.(\w+)\s*\}\}").unwrap());

    let vars_used: Vec<String> = re
        .captures_iter(template)
        .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
        .fold(Vec::new(), |mut acc, v| {
            if !acc.contains(&v) {
                acc.push(v);
            }
            acc
        });

    if vars_used.is_empty() {
        return vec![template.to_string()];
    }
    let Some(matrix) = matrix.and_then(|m| m.as_mapping()) else {
        return vec![template.to_string()];
    };

    let mut value_sets: Vec<Vec<String>> = Vec::new();
    for var in &vars_used {
        let values = collect_matrix_values(matrix, var);
        if values.is_empty() {
            return vec![template.to_string()];
        }
        value_sets.push(values);
    }

    let mut combos: Vec<Vec<String>> = vec![Vec::new()];
    for set in &value_sets {
        let mut next = Vec::with_capacity(combos.len() * set.len());
        for combo in &combos {
            for v in set {
                let mut extended = combo.clone();
                extended.push(v.clone());
                next.push(extended);
            }
        }
        combos = next;
    }

    combos
        .into_iter()
        .map(|combo| {
            let mut result = template.to_string();
            for (var, val) in vars_used.iter().zip(combo.iter()) {
                let pattern_re = regex::Regex::new(&format!(
                    r"\$\{{\{{\s*matrix\.{}\s*\}}\}}",
                    regex::escape(var)
                ))
                .expect("matrix substitution regex compiles");
                result = pattern_re.replace_all(&result, val.as_str()).into_owned();
            }
            result
        })
        .collect()
}

/// Pull values for `var` from a matrix mapping. Tries direct list form
/// (`matrix.os: [ubuntu-latest, ...]`) first, then falls back to scanning
/// `matrix.include[].var`. Numbers and booleans are coerced to strings.
fn collect_matrix_values(matrix: &serde_yaml::Mapping, var: &str) -> Vec<String> {
    if let Some(serde_yaml::Value::Sequence(seq)) =
        matrix.get(serde_yaml::Value::String(var.to_string()))
    {
        return seq.iter().filter_map(yaml_scalar_to_string).collect();
    }
    if let Some(serde_yaml::Value::Sequence(includes)) =
        matrix.get(serde_yaml::Value::String("include".to_string()))
    {
        return includes
            .iter()
            .filter_map(|item| {
                item.as_mapping()
                    .and_then(|m| m.get(serde_yaml::Value::String(var.to_string())))
                    .and_then(yaml_scalar_to_string)
            })
            .collect();
    }
    Vec::new()
}

fn yaml_scalar_to_string(v: &serde_yaml::Value) -> Option<String> {
    match v {
        serde_yaml::Value::String(s) => Some(s.clone()),
        serde_yaml::Value::Number(n) => Some(n.to_string()),
        serde_yaml::Value::Bool(b) => Some(b.to_string()),
        _ => None,
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

    fn fixture(lanes: &str, workflows: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        write(&dir.path().join("policy").join("ci-lanes.toml"), lanes);
        for (name, body) in workflows {
            write(
                &dir.path().join(".github").join("workflows").join(name),
                body,
            );
        }
        dir
    }

    const LANES_HEADER: &str = r#"
schema_version = 1
policy = "ci-lanes"
owner = "test"
status = "advisory"
"#;

    #[test]
    fn consistent_lane_with_workflow_and_job_passes() {
        let lanes = format!(
            "{LANES_HEADER}[lane.ci_check]\nworkflow = \".github/workflows/ci.yml\"\nworkflow_name = \"CI\"\njob_name = \"Check (ubuntu-latest)\"\n"
        );
        let workflow = "name: CI\non: push\njobs:\n  check:\n    name: Check (ubuntu-latest)\n    runs-on: ubuntu-latest\n    steps: []\n";
        let dir = fixture(&lanes, &[("ci.yml", workflow)]);
        run(dir.path(), Mode::BlockingAllowlist).expect("consistent fixture should pass");
    }

    #[test]
    fn job_id_with_no_explicit_name_is_matched_by_id() {
        let lanes = format!(
            "{LANES_HEADER}[lane.ci_deny]\nworkflow = \".github/workflows/ci.yml\"\nworkflow_name = \"CI\"\njob_name = \"deny\"\n"
        );
        let workflow =
            "name: CI\non: push\njobs:\n  deny:\n    runs-on: ubuntu-latest\n    steps: []\n";
        let dir = fixture(&lanes, &[("ci.yml", workflow)]);
        run(dir.path(), Mode::BlockingAllowlist).expect("job id should match when name is absent");
    }

    #[test]
    fn missing_workflow_file_is_finding() {
        let lanes = format!(
            "{LANES_HEADER}[lane.x]\nworkflow = \".github/workflows/missing.yml\"\nworkflow_name = \"X\"\n"
        );
        let dir = fixture(&lanes, &[]);
        let err = run(dir.path(), Mode::BlockingAllowlist).unwrap_err();
        assert!(err.to_string().contains("1 finding"));
    }

    #[test]
    fn workflow_name_mismatch_is_finding() {
        let lanes = format!(
            "{LANES_HEADER}[lane.ci_check]\nworkflow = \".github/workflows/ci.yml\"\nworkflow_name = \"CI\"\n"
        );
        let workflow = "name: Continuous Integration\non: push\njobs:\n  check:\n    runs-on: ubuntu-latest\n    steps: []\n";
        let dir = fixture(&lanes, &[("ci.yml", workflow)]);
        let err = run(dir.path(), Mode::BlockingAllowlist).unwrap_err();
        assert!(err.to_string().contains("1 finding"));
    }

    #[test]
    fn unknown_job_name_is_finding() {
        let lanes = format!(
            "{LANES_HEADER}[lane.ci_check]\nworkflow = \".github/workflows/ci.yml\"\nworkflow_name = \"CI\"\njob_name = \"GhostJob\"\n"
        );
        let workflow = "name: CI\non: push\njobs:\n  check:\n    name: Check (ubuntu-latest)\n    runs-on: ubuntu-latest\n    steps: []\n";
        let dir = fixture(&lanes, &[("ci.yml", workflow)]);
        let err = run(dir.path(), Mode::BlockingAllowlist).unwrap_err();
        assert!(err.to_string().contains("1 finding"));
    }

    #[test]
    fn job_name_without_workflow_name_is_finding() {
        let lanes = format!(
            "{LANES_HEADER}[lane.x]\nworkflow = \".github/workflows/ci.yml\"\njob_name = \"Orphan\"\n"
        );
        let workflow = "name: CI\non: push\njobs:\n  check:\n    name: Orphan\n    runs-on: ubuntu-latest\n    steps: []\n";
        let dir = fixture(&lanes, &[("ci.yml", workflow)]);
        let err = run(dir.path(), Mode::BlockingAllowlist).unwrap_err();
        assert!(err.to_string().contains("1 finding"));
    }

    #[test]
    fn workflow_catchall_lane_is_accepted() {
        // Lane declares workflow_name but intentionally omits job_name —
        // the catchall pattern from policy/ci-lanes.toml.
        let lanes = format!(
            "{LANES_HEADER}[lane.bdd]\nworkflow = \".github/workflows/bdd-testing.yml\"\nworkflow_name = \"BDD Testing\"\n"
        );
        let workflow = "name: BDD Testing\non: push\njobs:\n  one:\n    name: BDD One\n    runs-on: ubuntu-latest\n    steps: []\n  two:\n    name: BDD Two\n    runs-on: ubuntu-latest\n    steps: []\n";
        let dir = fixture(&lanes, &[("bdd-testing.yml", workflow)]);
        run(dir.path(), Mode::BlockingAllowlist).expect("catchall lane is allowed");
    }

    #[test]
    fn metadata_only_lane_is_skipped() {
        // Lane declares no workflow/workflow_name/job_name — purely
        // metadata (used for forecast grouping). Skip without findings.
        let lanes = format!(
            "{LANES_HEADER}[lane.metadata_only]\ndescription = \"...\"\nrunner = \"ubuntu_24_04\"\nbase_lem = 0\n"
        );
        let dir = fixture(&lanes, &[]);
        run(dir.path(), Mode::BlockingAllowlist).expect("metadata-only lane is skipped");
    }

    #[test]
    fn advisory_mode_reports_but_does_not_fail() {
        let lanes = format!(
            "{LANES_HEADER}[lane.x]\nworkflow = \".github/workflows/missing.yml\"\nworkflow_name = \"X\"\n"
        );
        let dir = fixture(&lanes, &[]);
        run(dir.path(), Mode::Advisory).expect("advisory mode should report but not fail");
    }

    #[test]
    fn matrix_job_expansion_resolves_per_leg_display_name() {
        let lanes = format!(
            "{LANES_HEADER}[lane.ci_check]\nworkflow = \".github/workflows/ci.yml\"\nworkflow_name = \"CI\"\njob_name = \"Check (windows-latest)\"\n"
        );
        let workflow = "name: CI\non: push\njobs:\n  check:\n    name: Check (${{ matrix.os }})\n    strategy:\n      matrix:\n        os: [ubuntu-latest, windows-latest]\n    runs-on: ${{ matrix.os }}\n    steps: []\n";
        let dir = fixture(&lanes, &[("ci.yml", workflow)]);
        run(dir.path(), Mode::BlockingAllowlist)
            .expect("matrix expansion should produce Check (windows-latest)");
    }

    #[test]
    fn matrix_include_form_expansion_finds_per_leg_names() {
        let lanes = format!(
            "{LANES_HEADER}[lane.release_x86]\nworkflow = \".github/workflows/release.yml\"\nworkflow_name = \"Release\"\njob_name = \"Build x86_64-unknown-linux-gnu\"\n"
        );
        let workflow = "name: Release\non: push\njobs:\n  build-binary:\n    name: Build ${{ matrix.target }}\n    strategy:\n      matrix:\n        include:\n          - os: ubuntu-latest\n            target: x86_64-unknown-linux-gnu\n          - os: macos-latest\n            target: aarch64-apple-darwin\n    runs-on: ${{ matrix.os }}\n    steps: []\n";
        let dir = fixture(&lanes, &[("release.yml", workflow)]);
        run(dir.path(), Mode::BlockingAllowlist)
            .expect("matrix.include form should resolve per-leg names");
    }

    #[test]
    fn malformed_workflow_yaml_is_finding_not_panic() {
        let lanes = format!(
            "{LANES_HEADER}[lane.x]\nworkflow = \".github/workflows/broken.yml\"\nworkflow_name = \"Broken\"\n"
        );
        let workflow = "name: \"Broken\"\njobs:\n  unclosed: [\n";
        let dir = fixture(&lanes, &[("broken.yml", workflow)]);
        let err = run(dir.path(), Mode::BlockingAllowlist).unwrap_err();
        assert!(err.to_string().contains("1 finding"));
    }
}
