//! `cargo xtask ci plan`
//!
//! Emits a CI Plan against
//! [`contracts/schemas/ci-plan.v1.schema.json`](../../contracts/schemas/ci-plan.v1.schema.json).
//! See [`docs/ci/ci-plan-json.md`](../../docs/ci/ci-plan-json.md) for the
//! human reference.
//!
//! Reads `policy/ci-budget.toml`, `policy/ci-lanes.toml`, and
//! `policy/ci-risk-packs.toml`. Walks the changed files between two SHAs
//! (or accepts an explicit `--changed-files` list for testing), matches
//! risk packs by glob, selects lanes from defaults + label matches +
//! risk-pack routing, and computes the LEM forecast.
//!
//! Always advisory: writes the plan and the GitHub step summary, never
//! exits non-zero on budget violations. Hard enforcement is a follow-up
//! release decision after PR #148 records actuals.

use anyhow::{Context, Result, anyhow};
use globset::Glob;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::policy;

// ────────────────────────────────────────────────────────────────────────
// Inputs
// ────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PlanInputs {
    pub workspace_root: PathBuf,
    pub base_ref: Option<String>,
    pub head_ref: Option<String>,
    pub pr_number: Option<u32>,
    pub labels: Vec<String>,
    pub changed_files_override: Option<Vec<String>>,
    pub output: PathBuf,
}

// ────────────────────────────────────────────────────────────────────────
// Policy structs (only the fields we consume)
// ────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct CiBudgetPolicy {
    budget: BudgetTiers,
    runner_multipliers: BTreeMap<String, f64>,
    labels: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct BudgetTiers {
    preferred_default_lem: u32,
    default_limit_lem: u32,
    elevated_limit_lem: u32,
    hard_limit_lem: u32,
}

#[derive(Debug, Deserialize)]
struct CiLanesPolicy {
    lane: BTreeMap<String, LaneDef>,
}

#[derive(Debug, Deserialize)]
struct LaneDef {
    runner: String,
    base_lem: f64,
    default_pr: bool,
    #[serde(default)]
    workflow: Option<String>,
    #[serde(default)]
    labels: Option<Vec<String>>,
    #[serde(default)]
    schedule: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CiRiskPacksPolicy {
    risk_pack: Vec<RiskPackDef>,
}

#[derive(Debug, Deserialize)]
struct RiskPackDef {
    id: String,
    paths: Vec<String>,
    selected_lanes: Vec<String>,
}

// ────────────────────────────────────────────────────────────────────────
// Output structs (mirror of ci-plan.v1.schema.json)
// ────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct CiPlan {
    schema_version: u32,
    repo: String,
    base_sha: String,
    head_sha: String,
    pr_number: Option<u32>,
    labels: Vec<String>,
    changed: ChangedSet,
    selection: LaneSelection,
    budget: BudgetPlan,
    warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ChangedSet {
    files: Vec<String>,
    areas: Vec<String>,
    crates: Vec<String>,
}

#[derive(Debug, Serialize)]
struct LaneSelection {
    risk_packs: Vec<RiskPackMatch>,
    lanes: Vec<LaneSelected>,
    skipped_lanes: Vec<LaneSkipped>,
}

#[derive(Debug, Serialize)]
struct RiskPackMatch {
    id: String,
    matched_paths: Vec<String>,
}

#[derive(Debug, Serialize)]
struct LaneSelected {
    id: String,
    selected_by: String,
}

#[derive(Debug, Serialize)]
struct LaneSkipped {
    id: String,
    skip_reason: String,
    skip_detail: Option<String>,
}

#[derive(Debug, Serialize)]
struct BudgetPlan {
    estimated_lem: f64,
    band: String,
    default_limit_lem: u32,
    elevated_limit_lem: u32,
    hard_limit_lem: u32,
    ack_required: Option<String>,
    ack_present: bool,
}

// ────────────────────────────────────────────────────────────────────────
// Entry point
// ────────────────────────────────────────────────────────────────────────

pub fn run(inputs: PlanInputs) -> Result<()> {
    let policy_dir = policy::policy_dir(&inputs.workspace_root);
    let budget_policy: CiBudgetPolicy = load_policy(&policy_dir, "ci-budget.toml")?;
    let lanes_policy: CiLanesPolicy = load_policy(&policy_dir, "ci-lanes.toml")?;
    let risk_packs_policy: CiRiskPacksPolicy = load_policy(&policy_dir, "ci-risk-packs.toml")?;

    // Filter empty labels (e.g. when `--labels ""` is passed from the workflow
    // because the PR has no labels). The schema requires labels[].minLength = 1.
    let labels: Vec<String> = inputs
        .labels
        .iter()
        .filter(|l| !l.is_empty())
        .cloned()
        .collect();
    let changed_files = resolve_changed_files(&inputs)?;
    let (base_sha, head_sha) = resolve_shas(&inputs)?;

    let matched_packs = match_risk_packs(&risk_packs_policy.risk_pack, &changed_files)?;
    let pack_selected_lanes: BTreeSet<String> = matched_packs
        .iter()
        .flat_map(|m| m.selected_lanes.iter().cloned())
        .collect();

    let mut selected = Vec::new();
    let mut skipped = Vec::new();
    let label_set: BTreeSet<&str> = labels.iter().map(String::as_str).collect();

    for (id, lane) in &lanes_policy.lane {
        let lane_id = format!("lane.{id}");
        let selected_by =
            pick_selected_by(id, lane, &label_set, &pack_selected_lanes, &matched_packs);
        match selected_by {
            Some(reason) => selected.push(LaneSelected {
                id: lane_id,
                selected_by: reason,
            }),
            None => {
                let (reason, detail) = pick_skip_reason(lane, &label_set);
                skipped.push(LaneSkipped {
                    id: lane_id,
                    skip_reason: reason,
                    skip_detail: detail,
                });
            }
        }
    }

    let estimated_lem = compute_lem(
        &selected,
        &lanes_policy.lane,
        &budget_policy.runner_multipliers,
    );
    let band = derive_band(estimated_lem, &budget_policy.budget);
    let (ack_required, ack_present) = derive_ack(&band, &label_set, &budget_policy.labels);

    let warnings = build_warnings(
        &band,
        ack_required.as_deref(),
        ack_present,
        &selected,
        estimated_lem,
    );

    let plan = CiPlan {
        schema_version: 1,
        repo: "shiplog".to_string(),
        base_sha,
        head_sha,
        pr_number: inputs.pr_number,
        labels,
        changed: build_changed_set(&changed_files),
        selection: LaneSelection {
            risk_packs: matched_packs
                .into_iter()
                .map(|m| RiskPackMatch {
                    id: m.id,
                    matched_paths: m.matched_paths,
                })
                .collect(),
            lanes: selected,
            skipped_lanes: skipped,
        },
        budget: BudgetPlan {
            estimated_lem,
            band,
            default_limit_lem: budget_policy.budget.default_limit_lem,
            elevated_limit_lem: budget_policy.budget.elevated_limit_lem,
            hard_limit_lem: budget_policy.budget.hard_limit_lem,
            ack_required,
            ack_present,
        },
        warnings,
    };

    write_plan_json(&inputs.output, &plan)?;
    if let Ok(summary_path) = std::env::var("GITHUB_STEP_SUMMARY") {
        write_step_summary(&PathBuf::from(summary_path), &plan)?;
    }
    print_console_summary(&plan, &inputs.output);
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

fn resolve_changed_files(inputs: &PlanInputs) -> Result<Vec<String>> {
    if let Some(files) = &inputs.changed_files_override {
        return Ok(files.clone());
    }
    let base = inputs.base_ref.as_deref().unwrap_or("main");
    let head = inputs.head_ref.as_deref().unwrap_or("HEAD");
    let output = Command::new("git")
        .args(["diff", "--name-only", &format!("{base}..{head}")])
        .current_dir(&inputs.workspace_root)
        .output()
        .with_context(|| format!("git diff --name-only {base}..{head}"))?;
    if !output.status.success() {
        return Err(anyhow!(
            "git diff {base}..{head} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8(output.stdout)
        .context("git diff output not valid UTF-8")?
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect())
}

fn resolve_shas(inputs: &PlanInputs) -> Result<(String, String)> {
    if inputs.changed_files_override.is_some() {
        // Tests pass synthetic SHAs through base_ref/head_ref directly.
        let base = inputs.base_ref.clone().unwrap_or_else(|| "0".repeat(40));
        let head = inputs.head_ref.clone().unwrap_or_else(|| "0".repeat(40));
        return Ok((base, head));
    }
    let base_ref = inputs.base_ref.as_deref().unwrap_or("main");
    let head_ref = inputs.head_ref.as_deref().unwrap_or("HEAD");
    let base = git_rev_parse(&inputs.workspace_root, base_ref)?;
    let head = git_rev_parse(&inputs.workspace_root, head_ref)?;
    Ok((base, head))
}

fn git_rev_parse(workspace_root: &Path, gitref: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", gitref])
        .current_dir(workspace_root)
        .output()
        .with_context(|| format!("git rev-parse {gitref}"))?;
    if !output.status.success() {
        return Err(anyhow!(
            "git rev-parse {gitref} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let sha = String::from_utf8(output.stdout)
        .context("git rev-parse output not valid UTF-8")?
        .trim()
        .to_string();
    if sha.len() != 40 || !sha.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(anyhow!(
            "expected 40-char hex SHA from git rev-parse {gitref}, got {sha:?}"
        ));
    }
    Ok(sha)
}

#[derive(Debug)]
struct MatchedPack {
    id: String,
    matched_paths: Vec<String>,
    selected_lanes: Vec<String>,
}

fn match_risk_packs(packs: &[RiskPackDef], changed_files: &[String]) -> Result<Vec<MatchedPack>> {
    let mut out = Vec::new();
    for pack in packs {
        let mut matched = Vec::new();
        let matchers: Result<Vec<_>> = pack
            .paths
            .iter()
            .map(|p| {
                Glob::new(p)
                    .map(|g| g.compile_matcher())
                    .map_err(|e| anyhow!("bad glob in risk_pack {}: {}: {}", pack.id, p, e))
            })
            .collect();
        let matchers = matchers?;
        for file in changed_files {
            if matchers.iter().any(|m| m.is_match(file)) {
                matched.push(file.clone());
            }
        }
        if !matched.is_empty() {
            out.push(MatchedPack {
                id: pack.id.clone(),
                matched_paths: matched,
                selected_lanes: pack.selected_lanes.clone(),
            });
        }
    }
    Ok(out)
}

fn pick_selected_by(
    id: &str,
    lane: &LaneDef,
    labels: &BTreeSet<&str>,
    pack_selected: &BTreeSet<String>,
    matched_packs: &[MatchedPack],
) -> Option<String> {
    if lane.default_pr {
        return Some("default_pr".to_string());
    }
    if let Some(lane_labels) = &lane.labels {
        for ll in lane_labels {
            if labels.contains(ll.as_str()) {
                return Some(format!("label:{ll}"));
            }
        }
    }
    if pack_selected.contains(id) {
        let pack_id = matched_packs
            .iter()
            .find(|p| p.selected_lanes.iter().any(|sl| sl == id))
            .map_or_else(|| "unknown".to_string(), |p| p.id.clone());
        return Some(format!("risk_pack:{pack_id}"));
    }
    None
}

fn pick_skip_reason(lane: &LaneDef, labels: &BTreeSet<&str>) -> (String, Option<String>) {
    if lane.schedule.is_some() {
        return ("nightly-only".to_string(), None);
    }
    if lane
        .workflow
        .as_deref()
        .is_some_and(|w| w.contains("release.yml"))
    {
        return ("release-only".to_string(), None);
    }
    if let Some(lane_labels) = &lane.labels
        && !lane_labels.is_empty()
    {
        let detail = format!(
            "lane requires one of {:?}; current labels {:?}",
            lane_labels,
            labels.iter().collect::<Vec<_>>()
        );
        return ("label-absent".to_string(), Some(detail));
    }
    ("no-matching-risk-pack".to_string(), None)
}

fn compute_lem(
    selected: &[LaneSelected],
    lanes: &BTreeMap<String, LaneDef>,
    multipliers: &BTreeMap<String, f64>,
) -> f64 {
    let mut total = 0.0_f64;
    for sel in selected {
        // sel.id is "lane.X"; look up "X" in lanes
        let key = sel.id.strip_prefix("lane.").unwrap_or(&sel.id);
        let Some(lane) = lanes.get(key) else { continue };
        let mult = multipliers.get(&lane.runner).copied().unwrap_or(1.0);
        total += lane.base_lem * mult;
    }
    total
}

fn derive_band(estimated_lem: f64, tiers: &BudgetTiers) -> String {
    let est = estimated_lem.ceil() as u32;
    if est <= tiers.preferred_default_lem {
        "preferred".to_string()
    } else if est <= tiers.default_limit_lem {
        "default".to_string()
    } else if est <= tiers.elevated_limit_lem {
        "elevated".to_string()
    } else {
        "hard".to_string()
    }
}

fn derive_ack(
    band: &str,
    labels: &BTreeSet<&str>,
    label_aliases: &BTreeMap<String, String>,
) -> (Option<String>, bool) {
    let ack_label = match band {
        "preferred" | "default" => return (None, false),
        "elevated" => label_aliases
            .get("budget_ack")
            .cloned()
            .unwrap_or_else(|| "ci-budget-ack".to_string()),
        "hard" => label_aliases
            .get("budget_override")
            .cloned()
            .unwrap_or_else(|| "ci-budget-override".to_string()),
        _ => return (None, false),
    };
    let present = labels.contains(ack_label.as_str())
        || labels.contains(
            label_aliases
                .get("full_ci")
                .map_or("full-ci", String::as_str),
        );
    (Some(ack_label), present)
}

fn build_warnings(
    band: &str,
    ack_required: Option<&str>,
    ack_present: bool,
    selected: &[LaneSelected],
    estimated_lem: f64,
) -> Vec<String> {
    let mut warnings = Vec::new();
    if let Some(label) = ack_required
        && !ack_present
    {
        warnings.push(format!(
            "PR is in band {band:?} (~{estimated_lem:.0} LEM); label {label:?} expected (advisory: not enforced in v0.5.0)"
        ));
    }
    if band == "hard" {
        warnings.push(format!(
            "PR estimated cost ~{:.0} LEM falls in the hard tier ({} selected lanes); consider whether the spend is justified",
            estimated_lem,
            selected.len()
        ));
    }
    warnings
}

fn build_changed_set(files: &[String]) -> ChangedSet {
    let mut areas = BTreeSet::new();
    let mut crates = BTreeSet::new();
    for f in files {
        if let Some(area) = top_area(f) {
            areas.insert(area);
        }
        if let Some(crate_name) = workspace_crate_of(f) {
            crates.insert(crate_name);
        }
    }
    ChangedSet {
        files: files.to_vec(),
        areas: areas.into_iter().collect(),
        crates: crates.into_iter().collect(),
    }
}

fn top_area(path: &str) -> Option<String> {
    let normalised = path.replace('\\', "/");
    let first = normalised.split('/').next()?;
    let area = match first {
        "docs" => "docs",
        "crates" => "crates",
        "apps" => "apps",
        "scripts" => "scripts",
        "policy" => "policy",
        ".github" => "workflows",
        "contracts" => "contracts",
        "fuzz" => "fuzz",
        "tests" => "tests",
        "examples" => "examples",
        _ => "root",
    };
    Some(area.to_string())
}

fn workspace_crate_of(path: &str) -> Option<String> {
    let normalised = path.replace('\\', "/");
    let mut parts = normalised.split('/');
    let first = parts.next()?;
    if first == "crates" || first == "apps" {
        parts.next().map(String::from)
    } else if first == "xtask" {
        Some("xtask".to_string())
    } else {
        None
    }
}

fn write_plan_json(output: &Path, plan: &CiPlan) -> Result<()> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create dir {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(plan).context("serialize CI plan to JSON")?;
    fs::write(output, json + "\n").with_context(|| format!("write {}", output.display()))?;
    Ok(())
}

fn write_step_summary(summary_path: &Path, plan: &CiPlan) -> Result<()> {
    use std::fmt::Write;
    let mut s = String::new();
    let _ = writeln!(s, "## CI Plan");
    let _ = writeln!(s);
    let _ = writeln!(
        s,
        "- **Repo:** {}  ·  **Base:** `{}`  ·  **Head:** `{}`",
        plan.repo,
        &plan.base_sha[..7.min(plan.base_sha.len())],
        &plan.head_sha[..7.min(plan.head_sha.len())]
    );
    if let Some(n) = plan.pr_number {
        let _ = writeln!(s, "- **PR:** #{n}");
    }
    let labels = if plan.labels.is_empty() {
        "(none)".to_string()
    } else {
        plan.labels.join(", ")
    };
    let _ = writeln!(s, "- **Labels:** {labels}");
    let _ = writeln!(
        s,
        "- **Changed:** {} files / {} areas / {} crates",
        plan.changed.files.len(),
        plan.changed.areas.len(),
        plan.changed.crates.len()
    );
    let _ = writeln!(s);
    let _ = writeln!(
        s,
        "### Budget — `{:.1}` LEM (band: **{}**)",
        plan.budget.estimated_lem, plan.budget.band
    );
    let _ = writeln!(s);
    let _ = writeln!(
        s,
        "default ≤ {} · elevated ≤ {} · hard ≤ {}",
        plan.budget.default_limit_lem, plan.budget.elevated_limit_lem, plan.budget.hard_limit_lem
    );
    if let Some(label) = &plan.budget.ack_required {
        let mark = if plan.budget.ack_present {
            "✅"
        } else {
            "⚠️"
        };
        let _ = writeln!(s);
        let _ = writeln!(
            s,
            "{mark} ack label: `{label}` (present: {})",
            plan.budget.ack_present
        );
    }
    let _ = writeln!(s);
    let _ = writeln!(s, "### Selected lanes ({})", plan.selection.lanes.len());
    let _ = writeln!(s);
    for lane in &plan.selection.lanes {
        let _ = writeln!(s, "- `{}` ← {}", lane.id, lane.selected_by);
    }
    let _ = writeln!(s);
    let _ = writeln!(
        s,
        "### Skipped lanes ({})",
        plan.selection.skipped_lanes.len()
    );
    let _ = writeln!(s);
    for skip in &plan.selection.skipped_lanes {
        let _ = writeln!(s, "- `{}` — {}", skip.id, skip.skip_reason);
    }
    if !plan.selection.risk_packs.is_empty() {
        let _ = writeln!(s);
        let _ = writeln!(s, "### Matched risk packs");
        let _ = writeln!(s);
        for pack in &plan.selection.risk_packs {
            let _ = writeln!(
                s,
                "- `{}` ({} files matched)",
                pack.id,
                pack.matched_paths.len()
            );
        }
    }
    if !plan.warnings.is_empty() {
        let _ = writeln!(s);
        let _ = writeln!(s, "### Warnings");
        let _ = writeln!(s);
        for w in &plan.warnings {
            let _ = writeln!(s, "- {w}");
        }
    }

    fs::write(summary_path, s)
        .with_context(|| format!("write GitHub step summary {}", summary_path.display()))?;
    Ok(())
}

fn print_console_summary(plan: &CiPlan, output: &Path) {
    println!("Wrote CI plan to {}", output.display());
    println!(
        "  selected lanes: {}    skipped lanes: {}    estimated LEM: {:.1} (band: {})",
        plan.selection.lanes.len(),
        plan.selection.skipped_lanes.len(),
        plan.budget.estimated_lem,
        plan.budget.band
    );
    if !plan.warnings.is_empty() {
        println!("  warnings: {}", plan.warnings.len());
        for w in &plan.warnings {
            println!("    - {w}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    // Test-side mirror of the production CiPlan so we can `serde_json::from_str` it
    // without exposing a Deserialize impl on the production type (which is
    // serialize-only by design).
    #[derive(Debug, Deserialize)]
    struct PlanReadback {
        schema_version: u32,
        changed: ChangedReadback,
        selection: SelectionReadback,
        budget: BudgetReadback,
        warnings: Vec<String>,
    }
    #[derive(Debug, Deserialize)]
    struct ChangedReadback {
        files: Vec<String>,
        areas: Vec<String>,
        crates: Vec<String>,
    }
    #[derive(Debug, Deserialize)]
    struct SelectionReadback {
        risk_packs: Vec<RiskPackReadback>,
        lanes: Vec<LaneReadback>,
        skipped_lanes: Vec<SkippedReadback>,
    }
    #[derive(Debug, Deserialize)]
    struct RiskPackReadback {
        id: String,
    }
    #[derive(Debug, Deserialize)]
    struct LaneReadback {
        id: String,
        selected_by: String,
    }
    #[derive(Debug, Deserialize)]
    struct SkippedReadback {
        id: String,
        skip_reason: String,
    }
    #[derive(Debug, Deserialize)]
    struct BudgetReadback {
        estimated_lem: f64,
        band: String,
        ack_required: Option<String>,
        ack_present: bool,
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
ubuntu_24_04 = 1.0
windows_latest = 2.0
external_ai_review = 4.0

[labels]
full_ci = "full-ci"
budget_ack = "ci-budget-ack"
budget_override = "ci-budget-override"
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

[lane.coverage]
description = "Codecov"
intent = "execution-surface measurement"
runner = "ubuntu_24_04"
base_lem = 45
default_pr = false
blocking = false
workflow = ".github/workflows/coverage.yml"
labels = ["coverage", "full-ci"]

[lane.fuzz_extended]
description = "Extended fuzz"
intent = "parser robustness (deep)"
runner = "ubuntu_24_04"
base_lem = 540
default_pr = false
blocking = false
workflow = ".github/workflows/fuzzing.yml"
schedule = "daily"

[lane.release_preflight]
description = "release preflight"
intent = "publish readiness"
runner = "ubuntu_24_04"
base_lem = 8
default_pr = false
blocking = true
workflow = ".github/workflows/release.yml"

[lane.mutation_targeted]
description = "Targeted mutation"
intent = "test-strength evidence (scoped)"
runner = "ubuntu_24_04"
base_lem = 45
default_pr = false
blocking = false
workflow = ".github/workflows/mutation-testing.yml"
labels = ["mutation"]
"#,
        );
        write(
            &policy.join("ci-risk-packs.toml"),
            r#"
schema_version = 1
policy = "ci-risk-packs"
owner = "x"
status = "advisory"

[[risk_pack]]
id = "redaction-privacy"
description = "redaction"
paths = ["crates/shiplog-redact/**"]
labels = ["mutation"]
selected_lanes = ["mutation_targeted"]

[[risk_pack]]
id = "docs-only"
description = "docs"
paths = ["docs/**", "*.md"]
labels = []
selected_lanes = []
"#,
        );
        dir
    }

    fn run_plan(workspace_root: &Path, files: Vec<&str>, labels: Vec<&str>) -> PlanReadback {
        let output = workspace_root.join("plan.json");
        run(PlanInputs {
            workspace_root: workspace_root.to_path_buf(),
            base_ref: Some("a".repeat(40)),
            head_ref: Some("b".repeat(40)),
            pr_number: Some(42),
            labels: labels.into_iter().map(String::from).collect(),
            changed_files_override: Some(files.into_iter().map(String::from).collect()),
            output: output.clone(),
        })
        .expect("plan should succeed");
        let json = fs::read_to_string(&output).expect("plan json");
        serde_json::from_str(&json).expect("parseable plan")
    }

    #[test]
    fn docs_only_pr_selects_only_default_pr_lanes() {
        let dir = fixture_workspace();
        let plan = run_plan(dir.path(), vec!["docs/foo.md", "README.md"], vec![]);
        assert_eq!(plan.schema_version, 1);
        assert_eq!(plan.changed.files.len(), 2);
        assert!(plan.changed.areas.contains(&"docs".to_string()));
        assert_eq!(plan.changed.crates.len(), 0);
        let selected: Vec<&str> = plan.selection.lanes.iter().map(|l| l.id.as_str()).collect();
        assert_eq!(selected, vec!["lane.ci_check"]);
        // ci_check is default_pr at 12 LEM => preferred band, no ack
        assert!(plan.budget.estimated_lem > 11.0 && plan.budget.estimated_lem < 13.0);
        assert_eq!(plan.budget.band, "preferred");
        assert_eq!(plan.budget.ack_required, None);
        assert!(!plan.budget.ack_present);
    }

    #[test]
    fn redaction_change_routes_targeted_mutation() {
        let dir = fixture_workspace();
        let plan = run_plan(dir.path(), vec!["crates/shiplog-redact/src/lib.rs"], vec![]);
        let pack_ids: Vec<&str> = plan
            .selection
            .risk_packs
            .iter()
            .map(|p| p.id.as_str())
            .collect();
        assert!(pack_ids.contains(&"redaction-privacy"));
        let selected: Vec<&str> = plan.selection.lanes.iter().map(|l| l.id.as_str()).collect();
        assert!(selected.contains(&"lane.ci_check"));
        assert!(selected.contains(&"lane.mutation_targeted"));
        let mut_select = plan
            .selection
            .lanes
            .iter()
            .find(|l| l.id == "lane.mutation_targeted")
            .unwrap();
        assert_eq!(mut_select.selected_by, "risk_pack:redaction-privacy");
    }

    #[test]
    fn coverage_label_selects_coverage_lane() {
        let dir = fixture_workspace();
        let plan = run_plan(dir.path(), vec!["src/foo.rs"], vec!["coverage"]);
        let selected: Vec<&str> = plan.selection.lanes.iter().map(|l| l.id.as_str()).collect();
        assert!(selected.contains(&"lane.coverage"));
        let cov = plan
            .selection
            .lanes
            .iter()
            .find(|l| l.id == "lane.coverage")
            .unwrap();
        assert_eq!(cov.selected_by, "label:coverage");
    }

    #[test]
    fn extended_fuzz_skipped_as_nightly_only() {
        let dir = fixture_workspace();
        let plan = run_plan(dir.path(), vec!["docs/foo.md"], vec![]);
        let skip = plan
            .selection
            .skipped_lanes
            .iter()
            .find(|s| s.id == "lane.fuzz_extended")
            .expect("fuzz_extended should be skipped");
        assert_eq!(skip.skip_reason, "nightly-only");
    }

    #[test]
    fn release_preflight_skipped_as_release_only() {
        let dir = fixture_workspace();
        let plan = run_plan(dir.path(), vec!["docs/foo.md"], vec![]);
        let skip = plan
            .selection
            .skipped_lanes
            .iter()
            .find(|s| s.id == "lane.release_preflight")
            .expect("release_preflight should be skipped");
        assert_eq!(skip.skip_reason, "release-only");
    }

    #[test]
    fn mutation_skipped_as_label_absent_when_no_risk_pack_match() {
        let dir = fixture_workspace();
        let plan = run_plan(dir.path(), vec!["docs/foo.md"], vec![]);
        let skip = plan
            .selection
            .skipped_lanes
            .iter()
            .find(|s| s.id == "lane.mutation_targeted")
            .expect("mutation_targeted should be skipped");
        assert_eq!(skip.skip_reason, "label-absent");
    }

    #[test]
    fn budget_band_climbs_with_added_label_lanes() {
        let dir = fixture_workspace();
        let plan = run_plan(dir.path(), vec!["src/foo.rs"], vec!["coverage"]);
        // ci_check 12 + coverage 45 = 57 LEM => elevated
        assert!(plan.budget.estimated_lem > 56.0 && plan.budget.estimated_lem < 58.0);
        assert_eq!(plan.budget.band, "elevated");
        assert_eq!(plan.budget.ack_required.as_deref(), Some("ci-budget-ack"));
        assert!(!plan.budget.ack_present);
        assert!(!plan.warnings.is_empty());
    }

    #[test]
    fn ack_present_when_budget_label_applied() {
        let dir = fixture_workspace();
        let plan = run_plan(
            dir.path(),
            vec!["src/foo.rs"],
            vec!["coverage", "ci-budget-ack"],
        );
        assert!(plan.budget.ack_present);
        assert!(
            plan.warnings.iter().all(|w| !w.contains("expected")),
            "no missing-ack warning, got: {:?}",
            plan.warnings
        );
    }
}
