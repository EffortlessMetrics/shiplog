//! `cargo xtask check-risk-pack-integrity`
//!
//! Cross-check referential integrity of `policy/ci-risk-packs.toml`
//! against the lane policy and label policy that it references:
//!
//! - Every value in a `[[risk_pack]].selected_lanes` array must resolve
//!   to a `[lane.*]` table in `policy/ci-lanes.toml`. A risk pack that
//!   references a lane that no longer exists is dead routing weight —
//!   the PR plan happily forecasts it but no workflow runs it, and the
//!   broken reference silently survives every other policy check.
//!
//! - Every value in a `[[risk_pack]].labels` array must resolve to a
//!   label declared in `policy/ci-budget.toml [labels]`. A risk pack
//!   that auto-applies a label nobody else knows about (or that was
//!   removed) drifts the same way.
//!
//! This is the second proactive guard added after #182 (which compared
//! `[labels]` against workflow `if:` consumption). The first guard
//! prevented "documented but not enforced." This guard prevents
//! "referenced but doesn't exist."
//!
//! Three concrete classes of regression this catches:
//!
//! 1. Lane rename in `ci-lanes.toml` without updating `selected_lanes`
//!    in `ci-risk-packs.toml`.
//! 2. Label rename in `ci-budget.toml [labels]` without updating the
//!    risk-pack `labels` array.
//! 3. Removing a lane (e.g. PR #164 dropped `lane.ci_msrv`) without
//!    pruning the corresponding `selected_lanes` reference.
//!
//! See [`docs/ci/risk-packs.md`](../../docs/ci/risk-packs.md) for the
//! human-readable risk-pack catalogue.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use toml::Value;

use crate::policy;
use crate::tasks::file_policy::Mode;

#[derive(Debug, Deserialize)]
struct RiskPacksPolicy {
    #[serde(default, rename = "risk_pack")]
    risk_packs: Vec<RiskPack>,
}

#[derive(Debug, Deserialize)]
struct RiskPack {
    id: String,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    selected_lanes: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct LanesPolicy {
    #[serde(default)]
    lane: BTreeMap<String, Value>,
}

#[derive(Debug, Deserialize)]
struct BudgetPolicy {
    #[serde(default)]
    labels: BTreeMap<String, String>,
}

pub fn run(workspace_root: &Path, mode: Mode) -> Result<()> {
    let policy_dir = policy::policy_dir(workspace_root);

    let lanes: LanesPolicy = load_toml(&policy_dir.join("ci-lanes.toml"))?;
    let known_lanes: BTreeSet<String> = lanes.lane.keys().cloned().collect();

    let budget: BudgetPolicy = load_toml(&policy_dir.join("ci-budget.toml"))?;
    let known_labels: BTreeSet<String> = budget.labels.values().cloned().collect();

    let risk_packs: RiskPacksPolicy = load_toml(&policy_dir.join("ci-risk-packs.toml"))?;

    let mut findings: Vec<String> = Vec::new();

    for pack in &risk_packs.risk_packs {
        for lane in &pack.selected_lanes {
            if !known_lanes.contains(lane) {
                findings.push(format!(
                    "[risk-pack-unknown-lane] risk_pack id={:?} selects lane {:?} which is not declared as [lane.{}] in policy/ci-lanes.toml — rename the entry, add the lane, or drop the reference",
                    pack.id, lane, lane
                ));
            }
        }
        for label in &pack.labels {
            if !known_labels.contains(label) {
                findings.push(format!(
                    "[risk-pack-unknown-label] risk_pack id={:?} declares label {:?} which is not declared in policy/ci-budget.toml [labels] — add the label to ci-budget.toml or drop the reference",
                    pack.id, label
                ));
            }
        }
    }

    if findings.is_empty() {
        println!("check-risk-pack-integrity: no findings.");
        return Ok(());
    }

    for f in &findings {
        eprintln!("  {f}");
    }
    println!("check-risk-pack-integrity: {} finding(s).", findings.len());

    match mode {
        Mode::Advisory => {
            println!("(advisory mode: not failing)");
            Ok(())
        }
        Mode::BlockingAllowlist => Err(anyhow::anyhow!(
            "check-risk-pack-integrity: {} finding(s) (blocking-allowlist mode)",
            findings.len()
        )),
    }
}

fn load_toml<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("parse {}", path.display()))
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

    fn fixture(lanes: &str, budget: &str, risk_packs: &str) -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        write(&dir.path().join("policy").join("ci-lanes.toml"), lanes);
        write(&dir.path().join("policy").join("ci-budget.toml"), budget);
        write(
            &dir.path().join("policy").join("ci-risk-packs.toml"),
            risk_packs,
        );
        dir
    }

    const LANES_HEADER: &str = r#"
schema_version = 1
policy = "ci-lanes"
owner = "test"
status = "advisory"
"#;

    const BUDGET_HEADER: &str = r#"
schema_version = 1
policy = "ci-budget"
owner = "test"
status = "advisory"
"#;

    const RISK_HEADER: &str = r#"
schema_version = 1
policy = "ci-risk-packs"
owner = "test"
status = "advisory"
"#;

    #[test]
    fn consistent_fixture_passes() {
        let lanes =
            format!("{LANES_HEADER}[lane.ci_check]\nbase_lem = 12\n\n[lane.bdd]\nbase_lem = 32\n");
        let budget = format!("{BUDGET_HEADER}[labels]\nbdd = \"bdd\"\n");
        let risk = format!(
            "{RISK_HEADER}[[risk_pack]]\nid = \"x\"\nlabels = [\"bdd\"]\nselected_lanes = [\"ci_check\", \"bdd\"]\n"
        );
        let dir = fixture(&lanes, &budget, &risk);
        run(dir.path(), Mode::BlockingAllowlist).expect("consistent fixture should pass");
    }

    #[test]
    fn unknown_lane_reference_is_finding() {
        let lanes = format!("{LANES_HEADER}[lane.ci_check]\nbase_lem = 12\n");
        let budget = format!("{BUDGET_HEADER}[labels]\n");
        let risk =
            format!("{RISK_HEADER}[[risk_pack]]\nid = \"x\"\nselected_lanes = [\"ci_msrv\"]\n");
        let dir = fixture(&lanes, &budget, &risk);
        let err = run(dir.path(), Mode::BlockingAllowlist).unwrap_err();
        assert!(err.to_string().contains("1 finding"));
    }

    #[test]
    fn unknown_label_reference_is_finding() {
        let lanes = format!("{LANES_HEADER}[lane.ci_check]\nbase_lem = 12\n");
        let budget = format!("{BUDGET_HEADER}[labels]\nbdd = \"bdd\"\n");
        let risk = format!(
            "{RISK_HEADER}[[risk_pack]]\nid = \"x\"\nlabels = [\"undeclared-label\"]\nselected_lanes = [\"ci_check\"]\n"
        );
        let dir = fixture(&lanes, &budget, &risk);
        let err = run(dir.path(), Mode::BlockingAllowlist).unwrap_err();
        assert!(err.to_string().contains("1 finding"));
    }

    #[test]
    fn lane_and_label_findings_accumulate() {
        let lanes = format!("{LANES_HEADER}[lane.ci_check]\nbase_lem = 12\n");
        let budget = format!("{BUDGET_HEADER}[labels]\nbdd = \"bdd\"\n");
        let risk = format!(
            "{RISK_HEADER}[[risk_pack]]\nid = \"x\"\nlabels = [\"undeclared\"]\nselected_lanes = [\"undeclared_lane\"]\n"
        );
        let dir = fixture(&lanes, &budget, &risk);
        let err = run(dir.path(), Mode::BlockingAllowlist).unwrap_err();
        assert!(err.to_string().contains("2 finding"));
    }

    #[test]
    fn empty_risk_pack_list_passes() {
        let lanes = format!("{LANES_HEADER}[lane.ci_check]\nbase_lem = 12\n");
        let budget = format!("{BUDGET_HEADER}[labels]\n");
        let risk = RISK_HEADER.to_string();
        let dir = fixture(&lanes, &budget, &risk);
        run(dir.path(), Mode::BlockingAllowlist).expect("empty list is consistent");
    }

    #[test]
    fn advisory_mode_reports_but_does_not_fail() {
        let lanes = format!("{LANES_HEADER}[lane.ci_check]\nbase_lem = 12\n");
        let budget = format!("{BUDGET_HEADER}[labels]\n");
        let risk =
            format!("{RISK_HEADER}[[risk_pack]]\nid = \"x\"\nselected_lanes = [\"ghost\"]\n");
        let dir = fixture(&lanes, &budget, &risk);
        run(dir.path(), Mode::Advisory).expect("advisory should report but not fail");
    }

    #[test]
    fn multiple_risk_packs_attribute_findings_correctly() {
        let lanes = format!("{LANES_HEADER}[lane.ci_check]\nbase_lem = 12\n");
        let budget = format!("{BUDGET_HEADER}[labels]\n");
        let risk = format!(
            "{RISK_HEADER}[[risk_pack]]\nid = \"a\"\nselected_lanes = [\"ghost_a\"]\n\n[[risk_pack]]\nid = \"b\"\nselected_lanes = [\"ghost_b\"]\n"
        );
        let dir = fixture(&lanes, &budget, &risk);
        let err = run(dir.path(), Mode::BlockingAllowlist).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("2 finding"));
    }
}
