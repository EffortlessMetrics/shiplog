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
        fs::write(policy.join(name), content).expect("write fixture");
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
