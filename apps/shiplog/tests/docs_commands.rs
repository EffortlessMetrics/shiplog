//! Fixture-safe command tests for documented review-cycle workflows.

use assert_cmd::Command;
use predicates::prelude::*;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn shiplog_cmd() -> Command {
    Command::from_std(std::process::Command::new(env!("CARGO_BIN_EXE_shiplog")))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn example_config(name: &str) -> PathBuf {
    repo_root().join("examples/configs").join(name)
}

#[test]
fn config_reference_documents_current_surface() {
    let doc_path = repo_root().join("docs/config-reference.md");
    let doc = std::fs::read_to_string(&doc_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", doc_path.display()));

    for needle in [
        "shiplog config validate --config shiplog.toml",
        "shiplog config explain --config shiplog.toml",
        "shiplog doctor --config shiplog.toml",
        "[shiplog]",
        "config_version = 1",
        "implicit v1",
        "[defaults]",
        "defaults.out",
        "defaults.window",
        "last-6-months",
        "last-quarter",
        "year:2025",
        "internal",
        "manager",
        "public",
        "[sources.github]",
        "me = false",
        "GITHUB_TOKEN",
        "api_base",
        "no_details",
        "[sources.gitlab]",
        "GITLAB_TOKEN",
        "opened",
        "merged",
        "closed",
        "[sources.jira]",
        "auth_user_env",
        "JIRA_TOKEN",
        "assignee JQL value",
        "[sources.linear]",
        "user_id",
        "LINEAR_API_KEY",
        "backlog",
        "cancelled",
        "[sources.git]",
        "include_merges",
        "[sources.json]",
        "coverage",
        "[sources.manual]",
        "manual_events.yaml",
        "[redaction]",
        "SHIPLOG_REDACT_KEY",
        "config validate",
        "doctor",
    ] {
        assert!(
            doc.contains(needle),
            "config reference should mention {needle:?}"
        );
    }
}

#[test]
fn documented_help_commands_stay_available() {
    shiplog_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("collect"))
        .stdout(predicate::str::contains("render"))
        .stdout(predicate::str::contains("workstreams"))
        .stdout(predicate::str::contains("journal"))
        .stdout(predicate::str::contains("cache"))
        .stdout(predicate::str::contains("identify"));

    shiplog_cmd()
        .args(["collect", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("multi"))
        .stdout(predicate::str::contains("github"))
        .stdout(predicate::str::contains("gitlab"))
        .stdout(predicate::str::contains("jira"))
        .stdout(predicate::str::contains("linear"))
        .stdout(predicate::str::contains("git"))
        .stdout(predicate::str::contains("json"))
        .stdout(predicate::str::contains("manual"));

    shiplog_cmd()
        .args(["render", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--latest"))
        .stdout(predicate::str::contains("--mode"))
        .stdout(predicate::str::contains("--receipt-limit"))
        .stdout(predicate::str::contains("--appendix"))
        .stdout(predicate::str::contains("--redact-key"));

    shiplog_cmd()
        .args(["review", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("weekly"))
        .stdout(predicate::str::contains("--latest"))
        .stdout(predicate::str::contains("--run"))
        .stdout(predicate::str::contains("--strict"));

    shiplog_cmd()
        .args(["review", "weekly", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--latest"))
        .stdout(predicate::str::contains("--run"))
        .stdout(predicate::str::contains("--strict"));

    shiplog_cmd()
        .args(["workstreams", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("rename"))
        .stdout(predicate::str::contains("move"))
        .stdout(predicate::str::contains("split"))
        .stdout(predicate::str::contains("receipts"));

    shiplog_cmd()
        .args(["journal", "add", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--date"))
        .stdout(predicate::str::contains("--title"))
        .stdout(predicate::str::contains("--workstream"))
        .stdout(predicate::str::contains("--receipt"));

    shiplog_cmd()
        .args(["cache", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("stats"))
        .stdout(predicate::str::contains("inspect"))
        .stdout(predicate::str::contains("clean"));

    shiplog_cmd()
        .args(["identify", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("jira"))
        .stdout(predicate::str::contains("linear"));

    shiplog_cmd()
        .args(["identify", "jira", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--instance"))
        .stdout(predicate::str::contains("--auth-user"))
        .stdout(predicate::str::contains("JIRA_TOKEN"));

    shiplog_cmd()
        .args(["identify", "linear", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--api-key"))
        .stdout(predicate::str::contains("LINEAR_API_KEY"));
}

#[test]
fn review_cycle_fixture_commands_execute_without_network() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");
    let local_config = example_config("local-git-json-manual.toml");

    shiplog_cmd()
        .current_dir(repo_root())
        .args([
            "init",
            "--source",
            "json",
            "--source",
            "manual",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Would write shiplog.toml"));

    let journal_path = tmp.path().join("manual_events.yaml");
    shiplog_cmd()
        .args([
            "journal",
            "add",
            "--events",
            journal_path.to_str().unwrap(),
            "--date",
            "2026-05-08",
            "--title",
            "Documented fixture rehearsal",
            "--workstream",
            "Docs",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Would add manual event"));

    shiplog_cmd()
        .current_dir(repo_root())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args([
            "config",
            "validate",
            "--config",
            local_config.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Config valid"));

    shiplog_cmd()
        .current_dir(repo_root())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args([
            "config",
            "explain",
            "--config",
            local_config.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Enabled sources:"));

    shiplog_cmd()
        .current_dir(repo_root())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["doctor", "--config", local_config.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Config: ok"))
        .stdout(predicate::str::contains("Manual: ok"));

    shiplog_cmd()
        .current_dir(repo_root())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args([
            "collect",
            "--out",
            out.to_str().unwrap(),
            "multi",
            "--config",
            local_config.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Collected configured sources:"))
        .stdout(predicate::str::contains("- json: success"))
        .stdout(predicate::str::contains("- manual: success"))
        .stdout(predicate::str::contains("Merged and wrote:"));

    shiplog_cmd()
        .args(["runs", "list", "--out", out.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Count: 1"))
        .stdout(predicate::str::contains("events:"));

    shiplog_cmd()
        .args([
            "runs",
            "show",
            "--out",
            out.to_str().unwrap(),
            "--run",
            "latest",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Run:"))
        .stdout(predicate::str::contains("Sources:"));

    shiplog_cmd()
        .args([
            "runs",
            "compare",
            "--out",
            out.to_str().unwrap(),
            "--from",
            "latest",
            "--to",
            "latest",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Compare:"))
        .stdout(predicate::str::contains("Events:"));

    shiplog_cmd()
        .args([
            "review",
            "weekly",
            "--out",
            out.to_str().unwrap(),
            "--latest",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Weekly review:"))
        .stdout(predicate::str::contains("New evidence:"))
        .stdout(predicate::str::contains("Evidence debt:"));

    shiplog_cmd()
        .args([
            "workstreams",
            "list",
            "--out",
            out.to_str().unwrap(),
            "--run",
            "latest",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Count:"));

    shiplog_cmd()
        .args([
            "workstreams",
            "validate",
            "--out",
            out.to_str().unwrap(),
            "--run",
            "latest",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Workstreams valid"));

    shiplog_cmd()
        .args([
            "render",
            "--out",
            out.to_str().unwrap(),
            "--latest",
            "--mode",
            "scaffold",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Rendered from existing events"));

    shiplog_cmd()
        .args([
            "render",
            "--out",
            out.to_str().unwrap(),
            "--latest",
            "--receipt-limit",
            "3",
            "--appendix",
            "summary",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Rendered from existing events"));

    shiplog_cmd()
        .args([
            "open",
            "packet",
            "--out",
            out.to_str().unwrap(),
            "--latest",
            "--print-path",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("packet.md"));

    shiplog_cmd()
        .args(["cache", "stats", "--out", out.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cache root:"));
}

#[test]
fn share_profile_docs_fail_closed_without_key() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");
    let local_config = example_config("local-git-json-manual.toml");

    shiplog_cmd()
        .current_dir(repo_root())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args([
            "collect",
            "--out",
            out.to_str().unwrap(),
            "multi",
            "--config",
            local_config.to_str().unwrap(),
        ])
        .assert()
        .success();

    shiplog_cmd()
        .env_remove("SHIPLOG_REDACT_KEY")
        .args([
            "render",
            "--out",
            out.to_str().unwrap(),
            "--latest",
            "--bundle-profile",
            "manager",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "manager profile requires --redact-key or SHIPLOG_REDACT_KEY",
        ));
}
