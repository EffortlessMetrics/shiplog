//! Integration tests for the `cargo xtask` CLI.
//!
//! These exercise the binary end-to-end against a synthetic workspace
//! root so they do not depend on the real `policy/` ledger state.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

fn xtask() -> Command {
    Command::cargo_bin("xtask").expect("xtask binary not built")
}

fn fixture_workspace(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempdir().expect("tempdir");
    let policy = dir.path().join("policy");
    fs::create_dir_all(&policy).expect("create policy dir");
    for (name, content) in files {
        let path = if name.contains('/') || name.contains('\\') {
            dir.path().join(name)
        } else {
            policy.join(name)
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create fixture parent");
        }
        fs::write(path, content).expect("write fixture");
    }
    dir
}

#[test]
fn check_policy_schemas_passes_for_well_formed_fixture() {
    let dir = fixture_workspace(&[(
        "ci-budget.toml",
        "schema_version = 1\npolicy = \"ci-budget\"\nowner = \"EffortlessMetrics\"\nstatus = \"advisory\"\n",
    )]);
    xtask()
        .args(["--workspace-root", dir.path().to_str().expect("utf-8 path")])
        .arg("check-policy-schemas")
        .assert()
        .success()
        .stdout(predicate::str::contains("well-formed"));
}

#[test]
fn check_policy_schemas_fails_on_status_typo() {
    let dir = fixture_workspace(&[(
        "ci-budget.toml",
        "schema_version = 1\npolicy = \"ci-budget\"\nowner = \"EffortlessMetrics\"\nstatus = \"advisroy\"\n",
    )]);
    xtask()
        .args(["--workspace-root", dir.path().to_str().expect("utf-8 path")])
        .arg("check-policy-schemas")
        .assert()
        .failure()
        .stderr(predicate::str::contains("status = \"advisroy\""));
}

#[test]
fn check_doc_artifacts_passes_for_linked_fixture() {
    let dir = fixture_workspace(&[
        (
            "policy/doc-artifacts.toml",
            r#"
schema_version = 1
policy = "doc-artifacts"
owner = "repo-infra"
status = "advisory"

[[artifact]]
id = "SHIPLOG-PROP-0001"
kind = "proposal"
path = "docs/proposals/SHIPLOG-PROP-0001-example.md"
status = "proposed"
owner = "repo-infra"

[[artifact]]
id = "SHIPLOG-SPEC-0001"
kind = "spec"
path = "docs/specs/SHIPLOG-SPEC-0001-example.md"
status = "accepted"
owner = "repo-infra"
linked_proposal = "SHIPLOG-PROP-0001"
"#,
        ),
        (
            "docs/proposals/SHIPLOG-PROP-0001-example.md",
            "# SHIPLOG-PROP-0001\n",
        ),
        (
            "docs/specs/SHIPLOG-SPEC-0001-example.md",
            "# SHIPLOG-SPEC-0001\n",
        ),
    ]);

    xtask()
        .args(["--workspace-root", dir.path().to_str().expect("utf-8 path")])
        .arg("check-doc-artifacts")
        .assert()
        .success()
        .stdout(predicate::str::contains("linked and valid"));
}

#[test]
fn check_doc_artifacts_fails_on_missing_file() {
    let dir = fixture_workspace(&[(
        "policy/doc-artifacts.toml",
        r#"
schema_version = 1
policy = "doc-artifacts"
owner = "repo-infra"
status = "advisory"

[[artifact]]
id = "SHIPLOG-PROP-0001"
kind = "proposal"
path = "docs/proposals/SHIPLOG-PROP-0001-example.md"
status = "proposed"
owner = "repo-infra"
"#,
    )]);

    xtask()
        .args(["--workspace-root", dir.path().to_str().expect("utf-8 path")])
        .arg("check-doc-artifacts")
        .assert()
        .failure()
        .stderr(predicate::str::contains("doc-artifact-missing-file"));
}

#[test]
fn check_goals_passes_for_linked_active_goal() {
    let dir = fixture_workspace(&[
        (
            "policy/doc-artifacts.toml",
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

[[artifact]]
id = "SHIPLOG-PLAN-0010"
kind = "plan"
path = "plans/0.10.0/implementation-plan.md"
status = "active"
owner = "codex"
linked_proposal = "SHIPLOG-PROP-0008"
linked_spec = "SHIPLOG-SPEC-0010"
"#,
        ),
        (
            "plans/0.10.0/implementation-plan.md",
            "SHIPLOG-PLAN-0010\n\n## Work item: active-goal-checker\n",
        ),
        (
            ".codex/goals/active.toml",
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
id = "active-goal-checker"
status = "active"
proposal = "SHIPLOG-PROP-0008"
spec = "SHIPLOG-SPEC-0010"
plan = "plans/0.10.0/implementation-plan.md"
commands = ["cargo xtask check-goals", "git diff --check"]
"#,
        ),
    ]);

    xtask()
        .args(["--workspace-root", dir.path().to_str().expect("utf-8 path")])
        .arg("check-goals")
        .assert()
        .success()
        .stdout(predicate::str::contains("linked and valid"));
}

#[test]
fn check_goals_fails_on_multiple_active_items() {
    let dir = fixture_workspace(&[
        (
            "policy/doc-artifacts.toml",
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

[[artifact]]
id = "SHIPLOG-PLAN-0010"
kind = "plan"
path = "plans/0.10.0/implementation-plan.md"
status = "active"
owner = "codex"
linked_proposal = "SHIPLOG-PROP-0008"
linked_spec = "SHIPLOG-SPEC-0010"
"#,
        ),
        (
            "plans/0.10.0/implementation-plan.md",
            "SHIPLOG-PLAN-0010\n\n## Work item: one\n\n## Work item: two\n",
        ),
        (
            ".codex/goals/active.toml",
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
id = "one"
status = "active"
proposal = "SHIPLOG-PROP-0008"
spec = "SHIPLOG-SPEC-0010"
plan = "plans/0.10.0/implementation-plan.md"
commands = ["git diff --check"]

[[work_item]]
id = "two"
status = "active"
proposal = "SHIPLOG-PROP-0008"
spec = "SHIPLOG-SPEC-0010"
plan = "plans/0.10.0/implementation-plan.md"
commands = ["git diff --check"]
"#,
        ),
    ]);

    xtask()
        .args(["--workspace-root", dir.path().to_str().expect("utf-8 path")])
        .arg("check-goals")
        .assert()
        .failure()
        .stderr(predicate::str::contains("goal-multiple-active-work-items"));
}

#[test]
fn policy_report_lists_loaded_ledgers() {
    let dir = fixture_workspace(&[
        (
            "ci-budget.toml",
            "schema_version = 1\npolicy = \"ci-budget\"\nowner = \"EffortlessMetrics\"\nstatus = \"advisory\"\n",
        ),
        (
            "ci-lanes.toml",
            "schema_version = 1\npolicy = \"ci-lanes\"\nowner = \"EffortlessMetrics\"\nstatus = \"advisory\"\n",
        ),
    ]);
    xtask()
        .args(["--workspace-root", dir.path().to_str().expect("utf-8 path")])
        .arg("policy-report")
        .assert()
        .success()
        .stdout(predicate::str::contains("ci-budget.toml"))
        .stdout(predicate::str::contains("ci-lanes.toml"));
}

#[test]
fn policy_report_empty_dir_is_ok() {
    let dir = tempdir().expect("tempdir");
    fs::create_dir_all(dir.path().join("policy")).expect("create policy dir");
    xtask()
        .args(["--workspace-root", dir.path().to_str().expect("utf-8 path")])
        .arg("policy-report")
        .assert()
        .success()
        .stdout(predicate::str::contains("0 ledger(s)"));
}

#[test]
fn workspace_root_can_come_from_env() {
    let dir = fixture_workspace(&[(
        "ci-budget.toml",
        "schema_version = 1\npolicy = \"ci-budget\"\nowner = \"EffortlessMetrics\"\nstatus = \"advisory\"\n",
    )]);
    xtask()
        .env(
            "SHIPLOG_XTASK_WORKSPACE_ROOT",
            dir.path().to_str().expect("utf-8 path"),
        )
        .arg("check-policy-schemas")
        .assert()
        .success();
}
