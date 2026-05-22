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
fn check_goals_validates_archived_goal_manifests() {
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
        (
            ".codex/goals/archive/2026-05-22-source-of-truth.toml",
            r#"
schema_version = 1

id = "shiplog-source-of-truth-stack"
title = "Shiplog source-of-truth stack rollout"
status = "archived"
owner = "codex"
created = "2026-05-20"

objective = "Keep repo source-of-truth artifacts linked."
end_state = ["Artifacts are linked."]

[[work_item]]
id = "active-goal-checker"
status = "done"
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
        .stdout(predicate::str::contains(
            "1 archived goal manifest(s) linked and valid",
        ));
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
fn check_support_tiers_passes_for_valid_claim_map() {
    let dir = fixture_workspace(&[(
        "docs/status/SUPPORT_TIERS.md",
        r#"# Support tiers

## Claim map

| Surface | Tier | Claim | Proof command | Notes |
|---|---|---|---|---|
| Document artifact links | Stabilizing | Proposal/spec/ADR/plan artifacts are linked. | `cargo xtask check-doc-artifacts` | Dedicated checker. |
| Policy ledgers | Stable | Policy files parse. | `cargo xtask check-policy-schemas` | Required proof. |
"#,
    )]);

    xtask()
        .args(["--workspace-root", dir.path().to_str().expect("utf-8 path")])
        .arg("check-support-tiers")
        .assert()
        .success()
        .stdout(predicate::str::contains("linked and valid"));
}

#[test]
fn check_support_tiers_fails_on_unknown_xtask_proof() {
    let dir = fixture_workspace(&[(
        "docs/status/SUPPORT_TIERS.md",
        r#"# Support tiers

## Claim map

| Surface | Tier | Claim | Proof command | Notes |
|---|---|---|---|---|
| Policy ledgers | Stable | Policy files parse. | `cargo xtask made-up-check` | Required proof. |
"#,
    )]);

    xtask()
        .args(["--workspace-root", dir.path().to_str().expect("utf-8 path")])
        .arg("check-support-tiers")
        .assert()
        .failure()
        .stderr(predicate::str::contains("support-tier-proof-unknown-xtask"));
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
fn repo_contract_report_writes_graph_outputs() {
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
            ".codex/goals/active.toml",
            r#"
id = "shiplog-source-of-truth-stack"
title = "Shiplog source-of-truth stack rollout"
status = "active"
owner = "codex"
created = "2026-05-20"
objective = "Keep repo source-of-truth artifacts linked."
end_state = ["Artifacts are linked."]

[[work_item]]
id = "repo-contract-report"
status = "active"
proposal = "SHIPLOG-PROP-0008"
spec = "SHIPLOG-SPEC-0010"
plan = "plans/0.10.0/implementation-plan.md"
commands = ["cargo xtask repo-contract-report", "git diff --check"]
"#,
        ),
        (
            "docs/status/SUPPORT_TIERS.md",
            r#"# Support tiers

## Claim map

| Surface | Tier | Claim | Proof command | Notes |
|---|---|---|---|---|
| Repo contract report | Stabilizing | Agents can inspect the graph. | `cargo xtask repo-contract-report` | Writes graph reports. |
"#,
        ),
    ]);

    xtask()
        .args(["--workspace-root", dir.path().to_str().expect("utf-8 path")])
        .arg("repo-contract-report")
        .assert()
        .success()
        .stdout(predicate::str::contains("repo-contract-report: wrote"));

    let graph_json = dir.path().join("target/source-of-truth/graph.json");
    let graph_md = dir.path().join("target/source-of-truth/graph.md");
    assert!(graph_json.is_file());
    assert!(graph_md.is_file());
    assert!(
        fs::read_to_string(graph_json)
            .expect("read graph json")
            .contains("\"repo-contract-report\"")
    );
    assert!(
        fs::read_to_string(graph_md)
            .expect("read graph md")
            .contains("# Source-of-truth graph")
    );
}

#[test]
fn pr_body_writes_body_from_active_work_item() {
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
policy_impact = ["policy/doc-artifacts.toml"]

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
id = "pr-body-generator"
status = "active"
proposal = "SHIPLOG-PROP-0008"
spec = "SHIPLOG-SPEC-0010"
plan = "plans/0.10.0/implementation-plan.md"
commands = ["cargo xtask pr-body --work-item pr-body-generator", "git diff --check"]
"#,
        ),
        (
            "plans/0.10.0/implementation-plan.md",
            r#"# 0.10.0 Source-of-Truth Rollout Plan

Plan artifact: SHIPLOG-PLAN-0010

## Work item: pr-body-generator

Status: active
Linked proposal: SHIPLOG-PROP-0008
Linked spec: SHIPLOG-SPEC-0010
Linked ADR: none
Branch: infra/pr-body-generator

### Goal

Add a repo-native PR body generator.

### Production delta

`xtask` command, tests, and docs.

### Non-goals

No GitHub API calls and no PR creation.

### Proof commands

```bash
cargo xtask pr-body --work-item pr-body-generator
git diff --check
```

### Rollback

Revert the generator PR.

### Claim boundary

This generates drafts only.
"#,
        ),
        (
            "docs/specs/SHIPLOG-SPEC-0010-source-of-truth-stack.md",
            r#"# SHIPLOG-SPEC-0010: Source-of-truth stack contract

Support-tier impact: stabilizing
Policy impact:
- policy/doc-artifacts.toml
- .codex/goals/active.toml
"#,
        ),
        (
            "docs/proposals/SHIPLOG-PROP-0008-source-of-truth-stack.md",
            "# SHIPLOG-PROP-0008\n",
        ),
    ]);

    xtask()
        .args(["--workspace-root", dir.path().to_str().expect("utf-8 path")])
        .args(["pr-body", "--work-item", "pr-body-generator"])
        .assert()
        .success()
        .stdout(predicate::str::contains("pr-body: wrote"));

    let body = fs::read_to_string(dir.path().join("target/source-of-truth/pr-body.md"))
        .expect("read generated body");
    assert!(body.contains("## Summary"));
    assert!(body.contains("Proposal: `SHIPLOG-PROP-0008`"));
    assert!(body.contains("Spec: `SHIPLOG-SPEC-0010`"));
    assert!(body.contains("No GitHub API calls"));
    assert!(body.contains("## Proof"));
    assert!(body.contains("This generates drafts only."));
}

#[test]
fn pr_body_fails_on_missing_work_item() {
    let dir = fixture_workspace(&[
        (
            "policy/doc-artifacts.toml",
            r#"
schema_version = 1
policy = "doc-artifacts"
owner = "repo-infra"
status = "advisory"
"#,
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
"#,
        ),
    ]);

    xtask()
        .args(["--workspace-root", dir.path().to_str().expect("utf-8 path")])
        .args(["pr-body", "--work-item", "missing"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("missing work item"));
}

#[test]
fn closeout_writes_handoff_and_archive() {
    let dir = fixture_workspace(&[
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
id = "pr-body-generator"
status = "done"
proposal = "SHIPLOG-PROP-0008"
spec = "SHIPLOG-SPEC-0010"
plan = "plans/0.10.0/implementation-plan.md"
commands = ["cargo xtask pr-body --work-item pr-body-generator", "git diff --check"]
receipts = ["EffortlessMetrics/shiplog-swarm#36", "EffortlessMetrics/shiplog#479"]
"#,
        ),
        (
            "plans/0.10.0/implementation-plan.md",
            r#"# 0.10.0 Source-of-Truth Rollout Plan

## Work item: pr-body-generator

### Claim boundary

This generates PR body drafts only.
"#,
        ),
    ]);

    xtask()
        .args(["--workspace-root", dir.path().to_str().expect("utf-8 path")])
        .args([
            "closeout",
            "--goal",
            "shiplog-source-of-truth-stack",
            "--date",
            "2026-05-22",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("closeout: wrote"));

    let handoff = fs::read_to_string(
        dir.path()
            .join("docs/handoffs/2026-05-22-shiplog-source-of-truth-stack-closeout.md"),
    )
    .expect("read closeout handoff");
    assert!(handoff.contains("## Landed work items"));
    assert!(handoff.contains("pr-body-generator"));
    assert!(handoff.contains("EffortlessMetrics/shiplog-swarm#36"));
    assert!(handoff.contains("This generates PR body drafts only."));

    let archive = fs::read_to_string(
        dir.path()
            .join(".codex/goals/archive/2026-05-22-shiplog-source-of-truth-stack.toml"),
    )
    .expect("read archived goal");
    assert!(archive.contains("status = \"archived\""));
}

#[test]
fn closeout_fails_on_wrong_goal() {
    let dir = fixture_workspace(&[(
        ".codex/goals/active.toml",
        r#"
schema_version = 1
id = "shiplog-source-of-truth-stack"
title = "Shiplog source-of-truth stack rollout"
status = "active"
owner = "codex"
created = "2026-05-20"
objective = "Keep repo source-of-truth artifacts linked."
"#,
    )]);

    xtask()
        .args(["--workspace-root", dir.path().to_str().expect("utf-8 path")])
        .args(["closeout", "--goal", "other-goal", "--date", "2026-05-22"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not match requested goal"));
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
