//! End-to-end regressions for direct proof-command enforcement.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

fn xtask() -> Command {
    Command::cargo_bin("xtask").expect("xtask binary not built")
}

fn fixture(status: &str, command: &str) -> tempfile::TempDir {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();

    fs::create_dir_all(root.join("policy")).expect("create policy dir");
    fs::create_dir_all(root.join(".codex/goals")).expect("create goal dir");
    fs::create_dir_all(root.join("plans/current")).expect("create plan dir");

    fs::write(
        root.join("policy/doc-artifacts.toml"),
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
status = "accepted"
owner = "repo-infra"
linked_proposal = "SHIPLOG-PROP-0008"

[[artifact]]
id = "SHIPLOG-PLAN-0010"
kind = "plan"
path = "plans/current/implementation-plan.md"
status = "active"
owner = "repo-infra"
linked_proposal = "SHIPLOG-PROP-0008"
linked_spec = "SHIPLOG-SPEC-0010"
"#,
    )
    .expect("write artifact policy");

    fs::write(
        root.join("plans/current/implementation-plan.md"),
        "SHIPLOG-PLAN-0010\n\n## Work item: direct-command-check\n",
    )
    .expect("write plan");

    fs::write(
        root.join(".codex/goals/active.toml"),
        format!(
            r#"
schema_version = 1

id = "shiplog-direct-command-contract"
title = "Shiplog direct command contract"
status = "active"
owner = "repo-infra"
created = "2026-07-24"
objective = "Keep proof commands direct."
end_state = ["Proof commands are direct."]

[[work_item]]
id = "direct-command-check"
status = "{status}"
proposal = "SHIPLOG-PROP-0008"
spec = "SHIPLOG-SPEC-0010"
plan = "plans/current/implementation-plan.md"
commands = [{command:?}]
"#
        ),
    )
    .expect("write active goal");

    dir
}

fn assert_wrapper_rejected(status: &str, command: &str) {
    let dir = fixture(status, command);

    xtask()
        .args(["--workspace-root", dir.path().to_str().expect("utf-8 path")])
        .arg("check-goals")
        .assert()
        .failure()
        .stderr(predicate::str::contains("[goal-work-item-command-wrapper]"))
        .stderr(predicate::str::contains("direct-command-check"));
}

#[test]
fn active_work_item_reports_wrapper_diagnostic() {
    assert_wrapper_rejected("active", "rtk git diff --check");
}

#[test]
fn ready_work_item_reports_wrapper_diagnostic() {
    assert_wrapper_rejected("ready", "rtk cargo xtask check-goals");
}

#[test]
fn tab_separated_wrapper_reports_wrapper_diagnostic() {
    assert_wrapper_rejected("active", "rtk\tgit diff --check");
}
