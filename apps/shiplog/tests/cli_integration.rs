//! Comprehensive CLI integration tests using `assert_cmd` and `predicates`.

use assert_cmd::Command;
use chrono::Duration;
use predicates::prelude::*;
use shiplog_cache::ApiCache;
use shiplog_schema::workstream::WorkstreamsFile;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use tempfile::TempDir;

fn shiplog_cmd() -> Command {
    Command::from_std(std::process::Command::new(env!("CARGO_BIN_EXE_shiplog")))
}

fn fixture_dir() -> PathBuf {
    repo_root().join("examples/fixture")
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn example_config(name: &str) -> PathBuf {
    repo_root().join("examples/configs").join(name)
}

fn seed_github_cache(cache_dir: &Path) -> PathBuf {
    std::fs::create_dir_all(cache_dir).unwrap();
    let path = cache_dir.join("github-api-cache.db");
    let cache = ApiCache::open(&path).unwrap();
    cache
        .set("fresh", &serde_json::json!({ "ok": true }))
        .unwrap();
    cache
        .set_with_ttl(
            "expired",
            &serde_json::json!({ "expired": true }),
            Duration::seconds(-1),
        )
        .unwrap();
    path
}

/// Run `collect json` into `tmp` and return the run directory path.
fn collect_json_into(tmp: &Path) -> PathBuf {
    let fixtures = fixture_dir();
    shiplog_cmd()
        .args([
            "collect",
            "--out",
            tmp.to_str().unwrap(),
            "json",
            "--events",
            fixtures.join("ledger.events.jsonl").to_str().unwrap(),
            "--coverage",
            fixtures.join("coverage.manifest.json").to_str().unwrap(),
        ])
        .assert()
        .success();
    tmp.join("run_fixture")
}

/// Run `collect manual` into `tmp` and return the run directory path.
fn collect_manual_into(tmp: &Path) -> PathBuf {
    let manual_events = tmp.join("manual_events.yaml");
    write_manual_events(&manual_events);

    shiplog_cmd()
        .args([
            "collect",
            "--out",
            tmp.to_str().unwrap(),
            "manual",
            "--events",
            manual_events.to_str().unwrap(),
            "--user",
            "octo",
            "--since",
            "2025-01-01",
            "--until",
            "2025-04-01",
        ])
        .assert()
        .success();
    first_run_dir(tmp)
}

fn write_manual_events(path: &Path) {
    std::fs::write(
        path,
        r#"version: 1
generated_at: 2026-01-01T00:00:00Z
events:
  - id: incident-followup
    type: Incident
    date: 2025-02-15
    title: Manual incident follow-up
    description: Verified the rollback procedure with support.
    workstream: Platform Reliability
    tags:
      - reliability
    receipts:
      - label: incident doc
        url: https://example.invalid/incidents/42
    impact: Reduced repeated escalation during review window.
"#,
    )
    .unwrap();
}

fn git_available() -> bool {
    StdCommand::new("git")
        .arg("--version")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn run_git(repo: &Path, args: &[&str]) {
    let output = StdCommand::new("git")
        .current_dir(repo)
        .args(args)
        .output()
        .unwrap_or_else(|err| panic!("failed to run git {args:?}: {err}"));
    assert!(
        output.status.success(),
        "git {args:?} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn run_git_commit(repo: &Path) {
    let output = StdCommand::new("git")
        .current_dir(repo)
        .env("GIT_AUTHOR_DATE", "2025-01-15T12:00:00+00:00")
        .env("GIT_COMMITTER_DATE", "2025-01-15T12:00:00+00:00")
        .args(["commit", "-m", "initial commit"])
        .output()
        .unwrap_or_else(|err| panic!("failed to run git commit: {err}"));
    assert!(
        output.status.success(),
        "git commit failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn create_local_git_repo() -> Option<TempDir> {
    if !git_available() {
        return None;
    }

    let tmp = TempDir::new().unwrap();
    run_git(tmp.path(), &["init"]);
    run_git(tmp.path(), &["config", "user.name", "Shiplog Test"]);
    run_git(tmp.path(), &["config", "user.email", "shiplog@example.com"]);

    std::fs::write(tmp.path().join("README.md"), "# fixture\n").unwrap();
    run_git(tmp.path(), &["add", "README.md"]);
    run_git_commit(tmp.path());

    Some(tmp)
}

fn first_run_dir(out: &Path) -> PathBuf {
    std::fs::read_dir(out)
        .unwrap()
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .find(|path| path.join("ledger.events.jsonl").exists())
        .expect("expected a shiplog run directory")
}

// ── 1. --version flag ──────────────────────────────────────────────────────

#[test]
fn version_flag_returns_version_string() {
    shiplog_cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("shiplog"))
        .stdout(predicate::str::contains("."));
}

// ── 2. --help shows all subcommands ────────────────────────────────────────

#[test]
fn help_shows_all_subcommands() {
    shiplog_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("init"))
        .stdout(predicate::str::contains("doctor"))
        .stdout(predicate::str::contains("config"))
        .stdout(predicate::str::contains("cache"))
        .stdout(predicate::str::contains("collect"))
        .stdout(predicate::str::contains("render"))
        .stdout(predicate::str::contains("refresh"))
        .stdout(predicate::str::contains("workstreams"))
        .stdout(predicate::str::contains("runs"))
        .stdout(predicate::str::contains("open"))
        .stdout(predicate::str::contains("merge"))
        .stdout(predicate::str::contains("import"))
        .stdout(predicate::str::contains("run"));
}

#[test]
fn init_help_shows_options() {
    shiplog_cmd()
        .args(["init", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--source"))
        .stdout(predicate::str::contains("--dry-run"))
        .stdout(predicate::str::contains("--force"));
}

#[test]
fn doctor_help_shows_options() {
    shiplog_cmd()
        .args(["doctor", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--config"))
        .stdout(predicate::str::contains("--source"));
}

#[test]
fn config_help_shows_validate_explain_and_migrate() {
    shiplog_cmd()
        .args(["config", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("validate"))
        .stdout(predicate::str::contains("explain"))
        .stdout(predicate::str::contains("migrate"));

    shiplog_cmd()
        .args(["config", "validate", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--config"));

    shiplog_cmd()
        .args(["config", "explain", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--config"));

    shiplog_cmd()
        .args(["config", "migrate", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--config"))
        .stdout(predicate::str::contains("--dry-run"));
}

#[test]
fn cache_help_shows_stats_inspect_and_clean() {
    shiplog_cmd()
        .args(["cache", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("stats"))
        .stdout(predicate::str::contains("inspect"))
        .stdout(predicate::str::contains("clean"));

    shiplog_cmd()
        .args(["cache", "stats", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--out"))
        .stdout(predicate::str::contains("--cache-dir"))
        .stdout(predicate::str::contains("--source"));

    shiplog_cmd()
        .args(["cache", "clean", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--older-than"))
        .stdout(predicate::str::contains("--all"))
        .stdout(predicate::str::contains("--dry-run"))
        .stdout(predicate::str::contains("--yes"));
}

#[test]
fn workstreams_help_shows_list_and_validate() {
    shiplog_cmd()
        .args(["workstreams", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("validate"))
        .stdout(predicate::str::contains("rename"))
        .stdout(predicate::str::contains("move"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("delete"))
        .stdout(predicate::str::contains("receipts"))
        .stdout(predicate::str::contains("receipt"))
        .stdout(predicate::str::contains("split"));
}

#[test]
fn runs_help_shows_list_and_show() {
    shiplog_cmd()
        .args(["runs", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("show"));
}

#[test]
fn open_help_shows_artifact_targets() {
    shiplog_cmd()
        .args(["open", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("packet"))
        .stdout(predicate::str::contains("workstreams"))
        .stdout(predicate::str::contains("out"));
}

#[test]
fn open_packet_help_shows_run_and_print_options() {
    shiplog_cmd()
        .args(["open", "packet", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--latest"))
        .stdout(predicate::str::contains("--run"))
        .stdout(predicate::str::contains("--print-path"));
}

#[test]
fn merge_help_shows_inputs_and_conflict_policy() {
    shiplog_cmd()
        .args(["merge", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--input"))
        .stdout(predicate::str::contains("--conflict"))
        .stdout(predicate::str::contains("prefer-most-recent"));
}

#[test]
fn init_creates_config_and_manual_events() {
    let tmp = TempDir::new().unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized shiplog"))
        .stdout(predicate::str::contains("GITHUB_TOKEN"))
        .stdout(predicate::str::contains("shiplog collect multi"));

    let config = std::fs::read_to_string(tmp.path().join("shiplog.toml")).unwrap();
    assert!(config.contains("[shiplog]\nconfig_version = 1"));
    assert!(config.contains("[sources.github]"));
    assert!(config.contains("enabled = true"));
    assert!(config.contains("me = true"));
    assert!(config.contains("[sources.manual]"));
    assert!(config.contains("events = \"./manual_events.yaml\""));

    let manual = std::fs::read_to_string(tmp.path().join("manual_events.yaml")).unwrap();
    assert!(manual.contains("version: 1"));
    assert!(manual.contains("events: []"));
}

#[test]
fn init_dry_run_does_not_write_files() {
    let tmp = TempDir::new().unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["init", "--dry-run", "--source", "jira"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Would write shiplog.toml"))
        .stdout(predicate::str::contains("[shiplog]"))
        .stdout(predicate::str::contains("config_version = 1"))
        .stdout(predicate::str::contains("JIRA_TOKEN"));

    assert!(!tmp.path().join("shiplog.toml").exists());
    assert!(!tmp.path().join("manual_events.yaml").exists());
}

#[test]
fn init_rejects_existing_files_without_force() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("shiplog.toml"), "existing").unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .arg("init")
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn init_rejects_existing_manual_events_without_partial_write() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("manual_events.yaml"), "existing").unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .arg("init")
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));

    assert!(
        !tmp.path().join("shiplog.toml").exists(),
        "init should not write shiplog.toml after detecting an existing scaffold file"
    );
}

#[test]
fn init_force_overwrites_existing_files() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("shiplog.toml"), "existing").unwrap();
    std::fs::write(tmp.path().join("manual_events.yaml"), "existing").unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["init", "--force", "--source", "jira", "--source", "linear"])
        .assert()
        .success();

    let config = std::fs::read_to_string(tmp.path().join("shiplog.toml")).unwrap();
    assert!(config.contains("[sources.jira]\nenabled = true"));
    assert!(config.contains("[sources.linear]\nenabled = true"));
    assert!(config.contains("[sources.github]\nenabled = false"));
    assert!(config.contains("[sources.manual]\nenabled = false"));
}

#[test]
fn doctor_reports_missing_config_actionably() {
    let tmp = TempDir::new().unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["doctor"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Config: error"))
        .stdout(predicate::str::contains("run `shiplog init` first"));
}

#[test]
fn doctor_checks_init_defaults_without_collecting() {
    let tmp = TempDir::new().unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .assert()
        .success();

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .args(["doctor"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Config: ok"))
        .stdout(predicate::str::contains("Window: ok"))
        .stdout(predicate::str::contains("Redaction: ok"))
        .stdout(predicate::str::contains(
            "GitHub: error, missing GITHUB_TOKEN",
        ))
        .stdout(predicate::str::contains("Manual: ok"));
}

#[test]
fn doctor_accepts_fixture_safe_sources() {
    let tmp = TempDir::new().unwrap();
    let fixtures = fixture_dir();
    std::fs::copy(
        fixtures.join("ledger.events.jsonl"),
        tmp.path().join("ledger.events.jsonl"),
    )
    .unwrap();
    std::fs::copy(
        fixtures.join("coverage.manifest.json"),
        tmp.path().join("coverage.manifest.json"),
    )
    .unwrap();
    write_manual_events(&tmp.path().join("manual_events.yaml"));
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[defaults]
out = "./out"
window = "last-6-months"
profile = "internal"

[sources.json]
enabled = true
events = "./ledger.events.jsonl"
coverage = "./coverage.manifest.json"

[sources.manual]
enabled = true
events = "./manual_events.yaml"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Config: ok"))
        .stdout(predicate::str::contains("JSON: ok"))
        .stdout(predicate::str::contains("Manual: ok"))
        .stdout(predicate::str::contains("Redaction: ok"));
}

#[test]
fn doctor_requires_redaction_key_for_share_profile_defaults() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[defaults]
profile = "manager"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("SHIPLOG_REDACT_KEY")
        .args(["doctor"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Redaction: error"))
        .stdout(predicate::str::contains("SHIPLOG_REDACT_KEY"));
}

#[test]
fn config_validate_accepts_fixture_safe_sources() {
    let tmp = TempDir::new().unwrap();
    let fixtures = fixture_dir();
    std::fs::copy(
        fixtures.join("ledger.events.jsonl"),
        tmp.path().join("ledger.events.jsonl"),
    )
    .unwrap();
    std::fs::copy(
        fixtures.join("coverage.manifest.json"),
        tmp.path().join("coverage.manifest.json"),
    )
    .unwrap();
    write_manual_events(&tmp.path().join("manual_events.yaml"));
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[defaults]
out = "./out"
window = "year:2025"
profile = "internal"

[sources.json]
enabled = true
events = "./ledger.events.jsonl"
coverage = "./coverage.manifest.json"

[sources.manual]
enabled = true
events = "./manual_events.yaml"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["config", "validate"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Config: ok"))
        .stdout(predicate::str::contains("Window: ok, 2025"))
        .stdout(predicate::str::contains("Sources: ok, json, manual"))
        .stdout(predicate::str::contains("Config valid"));
}

#[test]
fn example_configs_validate_without_source_tokens() {
    for name in [
        "github-only.toml",
        "github-gitlab-jira-manual.toml",
        "local-git-json-manual.toml",
        "public-portfolio.toml",
    ] {
        shiplog_cmd()
            .current_dir(repo_root())
            .env_remove("GITHUB_TOKEN")
            .env_remove("GITLAB_TOKEN")
            .env_remove("JIRA_TOKEN")
            .env_remove("LINEAR_API_KEY")
            .env_remove("SHIPLOG_REDACT_KEY")
            .args([
                "config",
                "validate",
                "--config",
                example_config(name).to_str().unwrap(),
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("Version: ok, 1"))
            .stdout(predicate::str::contains("Config valid"));
    }
}

#[test]
fn local_example_config_explains_fixture_sources() {
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
            example_config("local-git-json-manual.toml")
                .to_str()
                .unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Config metadata:"))
        .stdout(predicate::str::contains("- config_version: 1"))
        .stdout(predicate::str::contains("Enabled sources:"))
        .stdout(predicate::str::contains("- git: repo"))
        .stdout(predicate::str::contains("- json: events"))
        .stdout(predicate::str::contains("ledger.events.jsonl"))
        .stdout(predicate::str::contains("- manual: events"));
}

#[test]
fn config_validate_does_not_require_source_tokens() {
    let tmp = TempDir::new().unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .assert()
        .success();

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .args(["config", "validate"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Sources: ok, github, manual"));
}

#[test]
fn legacy_config_without_version_remains_supported() {
    let tmp = TempDir::new().unwrap();
    write_manual_events(&tmp.path().join("manual_events.yaml"));
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[defaults]
window = "year:2025"

[sources.manual]
enabled = true
events = "./manual_events.yaml"
user = "octo"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["config", "validate"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Version: ok, 1 (implicit)"));

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Config version: ok, 1 (implicit)"));

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["collect", "multi"])
        .assert()
        .success()
        .stdout(predicate::str::contains("- manual: success"));

    let run_dir = first_run_dir(&tmp.path().join("out"));
    assert!(run_dir.join("packet.md").exists(), "missing packet");
}

#[test]
fn unsupported_config_version_fails_before_collection_side_effects() {
    let tmp = TempDir::new().unwrap();
    write_manual_events(&tmp.path().join("manual_events.yaml"));
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[shiplog]
config_version = 2

[sources.manual]
enabled = true
events = "./manual_events.yaml"
user = "octo"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["config", "validate"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Version: error"))
        .stdout(predicate::str::contains("unsupported config_version 2"));

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["doctor"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Config version: error"))
        .stdout(predicate::str::contains("unsupported config_version 2"));

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["collect", "multi"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unsupported config_version 2"));

    assert!(
        !tmp.path().join("out").exists(),
        "unsupported config version should fail before writing outputs"
    );
}

#[test]
fn config_validate_reports_missing_config_actionably() {
    let tmp = TempDir::new().unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["config", "validate"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("run `shiplog init` first"));
}

#[test]
fn config_validate_rejects_no_enabled_sources() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[defaults]
window = "last-6-months"
profile = "internal"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["config", "validate"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Sources: error"))
        .stdout(predicate::str::contains("enable at least one"));
}

#[test]
fn config_validate_rejects_missing_file_source_paths() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[sources.manual]
enabled = true
events = "./missing.yaml"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["config", "validate"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Manual: error"))
        .stdout(predicate::str::contains("missing.yaml"));
}

#[test]
fn config_validate_rejects_invalid_default_out_file() {
    let tmp = TempDir::new().unwrap();
    write_manual_events(&tmp.path().join("manual_events.yaml"));
    std::fs::write(tmp.path().join("out-file"), "not a directory").unwrap();
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[defaults]
out = "./out-file"

[sources.manual]
enabled = true
events = "./manual_events.yaml"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["config", "validate"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Output: error"))
        .stdout(predicate::str::contains("exists but is not a directory"));
}

#[test]
fn config_migrate_dry_run_leaves_legacy_config_unchanged() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("shiplog.toml");
    let original = r#"[sources.manual]
enabled = true
events = "./manual_events.yaml"
"#;
    std::fs::write(&config_path, original).unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["config", "migrate", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "would add [shiplog] config_version = 1",
        ));

    let after = std::fs::read_to_string(config_path).unwrap();
    assert_eq!(after, original);
}

#[test]
fn config_migrate_requires_existing_config_even_for_dry_run() {
    let tmp = TempDir::new().unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["config", "migrate", "--dry-run"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("run `shiplog init` first"));

    assert!(!tmp.path().join("shiplog.toml").exists());
}

#[test]
fn config_migrate_adds_explicit_version_to_legacy_config() {
    let tmp = TempDir::new().unwrap();
    write_manual_events(&tmp.path().join("manual_events.yaml"));
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[sources.manual]
enabled = true
events = "./manual_events.yaml"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["config", "migrate"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "added [shiplog] config_version = 1",
        ))
        .stdout(predicate::str::contains("Config migrated"));

    let config = std::fs::read_to_string(tmp.path().join("shiplog.toml")).unwrap();
    assert!(config.starts_with("[shiplog]\nconfig_version = 1\n\n"));

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["config", "validate"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Version: ok, 1"));
}

#[test]
fn config_migrate_inserts_version_into_existing_shiplog_table() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[shiplog]
# local config metadata
schema_note = "keep this"

[sources.manual]
enabled = false
"#,
    )
    .unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["config", "migrate"])
        .assert()
        .success();

    let config = std::fs::read_to_string(tmp.path().join("shiplog.toml")).unwrap();
    assert_eq!(config.matches("[shiplog]").count(), 1);
    assert!(config.contains("[shiplog]\nconfig_version = 1\n# local config metadata"));
    assert!(config.contains("schema_note = \"keep this\""));
}

#[test]
fn config_migrate_current_config_is_noop() {
    let tmp = TempDir::new().unwrap();
    let original = r#"[shiplog]
config_version = 1

[sources.manual]
enabled = false
"#;
    std::fs::write(tmp.path().join("shiplog.toml"), original).unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["config", "migrate"])
        .assert()
        .success()
        .stdout(predicate::str::contains("already current"));

    let after = std::fs::read_to_string(tmp.path().join("shiplog.toml")).unwrap();
    assert_eq!(after, original);
}

#[test]
fn config_migrate_unsupported_version_fails_without_writing() {
    let tmp = TempDir::new().unwrap();
    let original = r#"[shiplog]
config_version = 2

[sources.manual]
enabled = false
"#;
    let config_path = tmp.path().join("shiplog.toml");
    std::fs::write(&config_path, original).unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["config", "migrate"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unsupported config_version 2"));

    let after = std::fs::read_to_string(config_path).unwrap();
    assert_eq!(after, original);
}

#[test]
fn config_explain_prints_effective_defaults_and_sources() {
    let tmp = TempDir::new().unwrap();
    let fixtures = fixture_dir();
    std::fs::copy(
        fixtures.join("ledger.events.jsonl"),
        tmp.path().join("ledger.events.jsonl"),
    )
    .unwrap();
    std::fs::copy(
        fixtures.join("coverage.manifest.json"),
        tmp.path().join("coverage.manifest.json"),
    )
    .unwrap();
    write_manual_events(&tmp.path().join("manual_events.yaml"));
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[defaults]
out = "./review-out"
window = "year:2025"
profile = "internal"
include_reviews = true

[user]
label = "Octo"

[sources.json]
enabled = true
events = "./ledger.events.jsonl"
coverage = "./coverage.manifest.json"

[sources.manual]
enabled = true
events = "./manual_events.yaml"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["config", "explain"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Config metadata:"))
        .stdout(predicate::str::contains("- config_version: 1 (implicit)"))
        .stdout(predicate::str::contains("Resolved defaults:"))
        .stdout(predicate::str::contains(
            "- window: year:2025 -> 2025-01-01..2026-01-01",
        ))
        .stdout(predicate::str::contains("- out: ./review-out ->"))
        .stdout(predicate::str::contains("- user.label: Octo"))
        .stdout(predicate::str::contains("Enabled sources:"))
        .stdout(predicate::str::contains("- json: events"))
        .stdout(predicate::str::contains("- manual: events"))
        .stdout(predicate::str::contains("user Octo"));
}

// ── 3. collect --help shows collect-specific options ───────────────────────

#[test]
fn collect_help_shows_sources_and_options() {
    shiplog_cmd()
        .args(["collect", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("github"))
        .stdout(predicate::str::contains("multi"))
        .stdout(predicate::str::contains("gitlab"))
        .stdout(predicate::str::contains("jira"))
        .stdout(predicate::str::contains("linear"))
        .stdout(predicate::str::contains("json"))
        .stdout(predicate::str::contains("--out"))
        .stdout(predicate::str::contains("--regen"));
}

#[test]
fn collect_multi_help_shows_config_and_conflict_policy() {
    shiplog_cmd()
        .args(["collect", "multi", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--config"))
        .stdout(predicate::str::contains("--last-6-months"))
        .stdout(predicate::str::contains("--conflict"))
        .stdout(predicate::str::contains("prefer-most-recent"));
}

#[test]
fn collect_github_help_shows_github_flags() {
    shiplog_cmd()
        .args(["collect", "github", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--user"))
        .stdout(predicate::str::contains("--me"))
        .stdout(predicate::str::contains("--since"))
        .stdout(predicate::str::contains("--until"))
        .stdout(predicate::str::contains("--last-6-months"))
        .stdout(predicate::str::contains("--last-quarter"))
        .stdout(predicate::str::contains("--year"))
        .stdout(predicate::str::contains("--mode"))
        .stdout(predicate::str::contains("--include-reviews"))
        .stdout(predicate::str::contains("--no-details"));
}

#[test]
fn collect_gitlab_help_shows_gitlab_flags() {
    shiplog_cmd()
        .args(["collect", "gitlab", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--user"))
        .stdout(predicate::str::contains("--me"))
        .stdout(predicate::str::contains("--since"))
        .stdout(predicate::str::contains("--until"))
        .stdout(predicate::str::contains("--state"))
        .stdout(predicate::str::contains("--instance"))
        .stdout(predicate::str::contains("--include-reviews"))
        .stdout(predicate::str::contains("--throttle-ms"))
        .stdout(predicate::str::contains("--token"))
        .stdout(predicate::str::contains("--cache-dir"))
        .stdout(predicate::str::contains("--no-cache"));
}

#[test]
fn collect_jira_help_shows_jira_flags() {
    shiplog_cmd()
        .args(["collect", "jira", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--user"))
        .stdout(predicate::str::contains("--auth-user"))
        .stdout(predicate::str::contains("--since"))
        .stdout(predicate::str::contains("--until"))
        .stdout(predicate::str::contains("--status"))
        .stdout(predicate::str::contains("--instance"))
        .stdout(predicate::str::contains("--throttle-ms"))
        .stdout(predicate::str::contains("--token"))
        .stdout(predicate::str::contains("--cache-dir"))
        .stdout(predicate::str::contains("--no-cache"));
}

#[test]
fn collect_linear_help_shows_linear_flags() {
    shiplog_cmd()
        .args(["collect", "linear", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--user-id"))
        .stdout(predicate::str::contains("--since"))
        .stdout(predicate::str::contains("--until"))
        .stdout(predicate::str::contains("--status"))
        .stdout(predicate::str::contains("--project"))
        .stdout(predicate::str::contains("--throttle-ms"))
        .stdout(predicate::str::contains("--api-key"))
        .stdout(predicate::str::contains("--cache-dir"))
        .stdout(predicate::str::contains("--no-cache"));
}

#[test]
fn collect_json_help_shows_json_flags() {
    shiplog_cmd()
        .args(["collect", "json", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--events"))
        .stdout(predicate::str::contains("--coverage"));
}

// ── 4. render --help shows render-specific options ─────────────────────────

#[test]
fn render_help_shows_render_options() {
    shiplog_cmd()
        .args(["render", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--out"))
        .stdout(predicate::str::contains("--run"))
        .stdout(predicate::str::contains("--latest"))
        .stdout(predicate::str::contains("--user"))
        .stdout(predicate::str::contains("--mode"))
        .stdout(predicate::str::contains("--receipt-limit"))
        .stdout(predicate::str::contains("--appendix"))
        .stdout(predicate::str::contains("--redact-key"));
}

#[test]
fn cache_stats_and_inspect_report_existing_cache_without_tokens() {
    let tmp = TempDir::new().unwrap();
    let cache_dir = tmp.path().join(".cache");
    let cache_path = seed_github_cache(&cache_dir);

    shiplog_cmd()
        .args([
            "cache",
            "stats",
            "--cache-dir",
            cache_dir.to_str().unwrap(),
            "--source",
            "github",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cache root:"))
        .stdout(predicate::str::contains("github:"))
        .stdout(predicate::str::contains(cache_path.display().to_string()))
        .stdout(predicate::str::contains(
            "entries: total 2, valid 1, expired 1",
        ));

    shiplog_cmd()
        .args([
            "cache",
            "inspect",
            "--cache-dir",
            cache_dir.to_str().unwrap(),
            "--source",
            "github",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("oldest:"))
        .stdout(predicate::str::contains("newest:"));
}

#[test]
fn cache_stats_does_not_create_missing_databases() {
    let tmp = TempDir::new().unwrap();
    let cache_dir = tmp.path().join(".cache");
    std::fs::create_dir_all(&cache_dir).unwrap();
    let missing = cache_dir.join("gitlab-api-cache.db");

    shiplog_cmd()
        .args([
            "cache",
            "stats",
            "--cache-dir",
            cache_dir.to_str().unwrap(),
            "--source",
            "gitlab",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("gitlab: missing"))
        .stdout(predicate::str::contains("No cache databases found"));

    assert!(
        !missing.exists(),
        "stats should not create a missing cache database"
    );
}

#[test]
fn cache_stats_and_inspect_do_not_initialize_existing_empty_file() {
    let tmp = TempDir::new().unwrap();
    let cache_dir = tmp.path().join(".cache");
    std::fs::create_dir_all(&cache_dir).unwrap();
    let empty_cache = cache_dir.join("github-api-cache.db");
    std::fs::File::create(&empty_cache).unwrap();
    assert_eq!(std::fs::metadata(&empty_cache).unwrap().len(), 0);

    shiplog_cmd()
        .args([
            "cache",
            "stats",
            "--cache-dir",
            cache_dir.to_str().unwrap(),
            "--source",
            "github",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cache_entries"));
    assert_eq!(
        std::fs::metadata(&empty_cache).unwrap().len(),
        0,
        "stats should not initialize an existing empty cache file"
    );

    shiplog_cmd()
        .args([
            "cache",
            "inspect",
            "--cache-dir",
            cache_dir.to_str().unwrap(),
            "--source",
            "github",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cache_entries"));
    assert_eq!(
        std::fs::metadata(&empty_cache).unwrap().len(),
        0,
        "inspect should not initialize an existing empty cache file"
    );
}

#[test]
fn cache_clean_expired_removes_only_expired_entries() {
    let tmp = TempDir::new().unwrap();
    let cache_dir = tmp.path().join(".cache");
    let cache_path = seed_github_cache(&cache_dir);

    shiplog_cmd()
        .args([
            "cache",
            "clean",
            "--cache-dir",
            cache_dir.to_str().unwrap(),
            "--source",
            "github",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("github: removed 1 entries"));

    let cache = ApiCache::open(cache_path).unwrap();
    let stats = cache.stats().unwrap();
    assert_eq!(stats.total_entries, 1);
    assert_eq!(stats.valid_entries, 1);
    assert_eq!(stats.expired_entries, 0);
}

#[test]
fn cache_clean_older_than_dry_run_reports_without_mutating() {
    let tmp = TempDir::new().unwrap();
    let cache_dir = tmp.path().join(".cache");
    let cache_path = seed_github_cache(&cache_dir);

    shiplog_cmd()
        .args([
            "cache",
            "clean",
            "--cache-dir",
            cache_dir.to_str().unwrap(),
            "--source",
            "github",
            "--older-than",
            "0m",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("github: would remove 2 entries"));

    let cache = ApiCache::open(cache_path).unwrap();
    assert_eq!(cache.stats().unwrap().total_entries, 2);
}

#[test]
fn cache_clean_all_requires_yes_and_preserves_unrelated_files() {
    let tmp = TempDir::new().unwrap();
    let cache_dir = tmp.path().join(".cache");
    let cache_path = seed_github_cache(&cache_dir);
    let unrelated = cache_dir.join("notes.txt");
    std::fs::write(&unrelated, "not a cache database").unwrap();

    shiplog_cmd()
        .args([
            "cache",
            "clean",
            "--cache-dir",
            cache_dir.to_str().unwrap(),
            "--source",
            "github",
            "--all",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cache clean --all requires --yes"));

    assert_eq!(
        ApiCache::open(&cache_path)
            .unwrap()
            .stats()
            .unwrap()
            .total_entries,
        2
    );

    shiplog_cmd()
        .args([
            "cache",
            "clean",
            "--cache-dir",
            cache_dir.to_str().unwrap(),
            "--source",
            "github",
            "--all",
            "--yes",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("github: removed 2 entries"));

    assert!(
        cache_path.exists(),
        "clean should not delete cache database files"
    );
    assert!(
        unrelated.exists(),
        "clean should not delete unrelated files"
    );
    assert_eq!(
        ApiCache::open(cache_path)
            .unwrap()
            .stats()
            .unwrap()
            .total_entries,
        0
    );
}

// ── 5. collect json with sample fixture data ───────────────────────────────

#[test]
fn collect_json_produces_all_outputs() {
    let tmp = TempDir::new().unwrap();
    let fixtures = fixture_dir();

    let mut cmd = shiplog_cmd();
    cmd.env_remove("SHIPLOG_REDACT_KEY")
        .args([
            "collect",
            "--out",
            tmp.path().to_str().unwrap(),
            "json",
            "--events",
            fixtures.join("ledger.events.jsonl").to_str().unwrap(),
            "--coverage",
            fixtures.join("coverage.manifest.json").to_str().unwrap(),
        ])
        .assert()
        .success();

    let run_dir = tmp.path().join("run_fixture");
    assert!(run_dir.join("packet.md").exists(), "missing packet.md");
    assert!(
        run_dir.join("ledger.events.jsonl").exists(),
        "missing ledger.events.jsonl"
    );
    assert!(
        run_dir.join("coverage.manifest.json").exists(),
        "missing coverage.manifest.json"
    );
    assert!(
        run_dir.join("workstreams.suggested.yaml").exists(),
        "missing workstreams.suggested.yaml"
    );
    assert!(
        run_dir.join("bundle.manifest.json").exists(),
        "missing bundle.manifest.json"
    );
    assert!(
        !run_dir.join("profiles/manager/packet.md").exists(),
        "manager profile should require an explicit redaction key"
    );
    assert!(
        !run_dir.join("profiles/public/packet.md").exists(),
        "public profile should require an explicit redaction key"
    );
}

#[test]
fn merge_existing_runs_writes_combined_packet() {
    let json_tmp = TempDir::new().unwrap();
    let manual_tmp = TempDir::new().unwrap();
    let merge_tmp = TempDir::new().unwrap();

    let json_run = collect_json_into(json_tmp.path());
    let manual_run = collect_manual_into(manual_tmp.path());

    shiplog_cmd()
        .args([
            "merge",
            "--out",
            merge_tmp.path().to_str().unwrap(),
            "--input",
            json_run.to_str().unwrap(),
            "--input",
            manual_run.to_str().unwrap(),
            "--conflict",
            "prefer-most-recent",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Merged and wrote:"))
        .stdout(predicate::str::contains("- inputs: 2"));

    let run_dir = first_run_dir(merge_tmp.path());
    assert!(run_dir.join("packet.md").exists(), "missing merged packet");
    assert!(
        run_dir.join("ledger.events.jsonl").exists(),
        "missing merged ledger"
    );
    assert!(
        run_dir.join("coverage.manifest.json").exists(),
        "missing merged coverage"
    );

    let packet = std::fs::read_to_string(run_dir.join("packet.md")).unwrap();
    assert!(
        packet.contains("Payments ledger rewrite"),
        "merged packet should include JSON fixture evidence"
    );
    assert!(
        packet.contains("Manual incident follow-up"),
        "merged packet should include manual evidence"
    );

    let coverage = std::fs::read_to_string(run_dir.join("coverage.manifest.json")).unwrap();
    assert!(
        coverage.contains("\"github\""),
        "merged coverage should include github source"
    );
    assert!(
        coverage.contains("\"manual\""),
        "merged coverage should include manual source"
    );
}

#[test]
fn collect_multi_from_config_merges_json_and_manual_sources() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");
    let fixtures = fixture_dir();
    std::fs::copy(
        fixtures.join("ledger.events.jsonl"),
        tmp.path().join("ledger.events.jsonl"),
    )
    .unwrap();
    std::fs::copy(
        fixtures.join("coverage.manifest.json"),
        tmp.path().join("coverage.manifest.json"),
    )
    .unwrap();
    write_manual_events(&tmp.path().join("manual_events.yaml"));

    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[defaults]
window = "year:2025"
include_reviews = true

[user]
label = "octo"

[sources.json]
enabled = true
events = "./ledger.events.jsonl"
coverage = "./coverage.manifest.json"

[sources.manual]
enabled = true
events = "./manual_events.yaml"
user = "octo"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .args([
            "collect",
            "--out",
            out.to_str().unwrap(),
            "multi",
            "--config",
            tmp.path().join("shiplog.toml").to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Collected configured sources:"))
        .stdout(predicate::str::contains("- json: success"))
        .stdout(predicate::str::contains("- manual: success"))
        .stdout(predicate::str::contains("Merged and wrote:"));

    let run_dir = first_run_dir(&out);
    assert!(run_dir.join("packet.md").exists(), "missing merged packet");
    assert!(
        run_dir.join("ledger.events.jsonl").exists(),
        "missing merged ledger"
    );
    assert!(
        run_dir.join("coverage.manifest.json").exists(),
        "missing merged coverage"
    );

    let packet = std::fs::read_to_string(run_dir.join("packet.md")).unwrap();
    assert!(
        packet.contains("Payments ledger rewrite"),
        "configured multi packet should include JSON fixture evidence"
    );
    assert!(
        packet.contains("Manual incident follow-up"),
        "configured multi packet should include manual evidence"
    );

    let coverage = std::fs::read_to_string(run_dir.join("coverage.manifest.json")).unwrap();
    assert!(
        coverage.contains("\"github\""),
        "configured multi coverage should include JSON fixture source"
    );
    assert!(
        coverage.contains("\"manual\""),
        "configured multi coverage should include manual source"
    );
}

#[test]
fn collect_multi_uses_config_default_out_when_cli_out_is_omitted() {
    let tmp = TempDir::new().unwrap();
    let configured_out = tmp.path().join("configured-out");
    let fixtures = fixture_dir();
    std::fs::copy(
        fixtures.join("ledger.events.jsonl"),
        tmp.path().join("ledger.events.jsonl"),
    )
    .unwrap();
    std::fs::copy(
        fixtures.join("coverage.manifest.json"),
        tmp.path().join("coverage.manifest.json"),
    )
    .unwrap();
    write_manual_events(&tmp.path().join("manual_events.yaml"));

    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[defaults]
out = "./configured-out"
window = "year:2025"

[sources.json]
enabled = true
events = "./ledger.events.jsonl"
coverage = "./coverage.manifest.json"

[sources.manual]
enabled = true
events = "./manual_events.yaml"
user = "octo"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["collect", "multi"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Collected configured sources:"));

    let run_dir = first_run_dir(&configured_out);
    assert!(run_dir.join("packet.md").exists(), "missing merged packet");
    assert!(
        !tmp.path().join("out").exists(),
        "collect multi should not write ./out when defaults.out is set"
    );
}

#[test]
fn collect_multi_uses_config_default_profile_for_redaction_safety() {
    let tmp = TempDir::new().unwrap();
    write_manual_events(&tmp.path().join("manual_events.yaml"));

    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[defaults]
profile = "public"
window = "year:2025"

[redaction]
key_env = "SHIPLOG_TEST_REDACT_KEY_FOR_CONFIG"

[sources.manual]
enabled = true
events = "./manual_events.yaml"
user = "octo"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("SHIPLOG_TEST_REDACT_KEY_FOR_CONFIG")
        .args(["collect", "multi"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "public profile requires --redact-key or SHIPLOG_TEST_REDACT_KEY_FOR_CONFIG",
        ));
}

#[test]
fn collect_multi_records_partial_source_failures() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");
    write_manual_events(&tmp.path().join("manual_events.yaml"));

    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[defaults]
window = "year:2025"

[sources.json]
enabled = true
events = "./missing-ledger.events.jsonl"
coverage = "./missing-coverage.manifest.json"

[sources.manual]
enabled = true
events = "./manual_events.yaml"
user = "octo"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .args([
            "collect",
            "--out",
            out.to_str().unwrap(),
            "multi",
            "--config",
            tmp.path().join("shiplog.toml").to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("- json: skipped"))
        .stdout(predicate::str::contains("- manual: success"))
        .stdout(predicate::str::contains("Merged and wrote:"));

    let run_dir = first_run_dir(&out);
    let packet = std::fs::read_to_string(run_dir.join("packet.md")).unwrap();
    assert!(
        packet.contains("Manual incident follow-up"),
        "configured multi packet should include successful source evidence"
    );
    assert!(
        packet.contains("Skipped:\n- JSON:"),
        "configured multi packet should list skipped sources in the packet coverage summary"
    );
    assert!(
        !packet
            .split("Included:")
            .nth(1)
            .unwrap()
            .split("Skipped:")
            .next()
            .unwrap()
            .contains("JSON"),
        "configured multi packet should not list skipped sources as included"
    );

    let coverage = std::fs::read_to_string(run_dir.join("coverage.manifest.json")).unwrap();
    assert!(
        coverage.contains("\"Partial\""),
        "configured multi coverage should mark skipped sources as partial"
    );
    assert!(
        coverage.contains("Configured source json was skipped"),
        "configured multi coverage should record skipped source warning"
    );
}

#[test]
fn collect_multi_without_enabled_sources_fails_actionably() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[sources.json]
enabled = false
"#,
    )
    .unwrap();

    shiplog_cmd()
        .args([
            "collect",
            "--out",
            tmp.path().join("out").to_str().unwrap(),
            "multi",
            "--config",
            tmp.path().join("shiplog.toml").to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No enabled sources found"));
}

#[test]
fn merge_missing_input_run_fails_actionably() {
    let tmp = TempDir::new().unwrap();
    let missing = tmp.path().join("missing-run");

    shiplog_cmd()
        .args([
            "merge",
            "--out",
            tmp.path().to_str().unwrap(),
            "--input",
            missing.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No ledger.events.jsonl found"));
}

#[test]
fn collect_json_public_profile_without_key_fails_closed() {
    let tmp = TempDir::new().unwrap();
    let fixtures = fixture_dir();

    let mut cmd = shiplog_cmd();
    cmd.env_remove("SHIPLOG_REDACT_KEY")
        .args([
            "collect",
            "--out",
            tmp.path().to_str().unwrap(),
            "--bundle-profile",
            "public",
            "json",
            "--events",
            fixtures.join("ledger.events.jsonl").to_str().unwrap(),
            "--coverage",
            fixtures.join("coverage.manifest.json").to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "public profile requires --redact-key or SHIPLOG_REDACT_KEY",
        ));
}

#[test]
fn collect_json_packet_contains_expected_content() {
    let tmp = TempDir::new().unwrap();
    let fixtures = fixture_dir();

    shiplog_cmd()
        .args([
            "collect",
            "--out",
            tmp.path().to_str().unwrap(),
            "json",
            "--events",
            fixtures.join("ledger.events.jsonl").to_str().unwrap(),
            "--coverage",
            fixtures.join("coverage.manifest.json").to_str().unwrap(),
        ])
        .assert()
        .success();

    let packet = std::fs::read_to_string(tmp.path().join("run_fixture/packet.md")).unwrap();
    // Fixture data contains PRs from acme/payments and acme/platform
    assert!(
        packet.contains("acme/payments") || packet.contains("acme/platform"),
        "packet.md should reference fixture repos"
    );
}

#[test]
fn workstreams_list_shows_latest_run_workstreams() {
    let tmp = TempDir::new().unwrap();
    collect_json_into(tmp.path());

    shiplog_cmd()
        .args(["workstreams", "list", "--out", tmp.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Workstreams:"))
        .stdout(predicate::str::contains("suggested workstreams"))
        .stdout(predicate::str::contains("Count:"))
        .stdout(predicate::str::contains("acme/platform"))
        .stdout(predicate::str::contains("events="));
}

#[test]
fn runs_list_shows_run_summaries() {
    let tmp = TempDir::new().unwrap();
    collect_json_into(tmp.path());

    shiplog_cmd()
        .args(["runs", "list", "--out", tmp.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Runs:"))
        .stdout(predicate::str::contains("run_fixture"))
        .stdout(predicate::str::contains("sources: github"))
        .stdout(predicate::str::contains("events: 3"))
        .stdout(predicate::str::contains("coverage: Complete"))
        .stdout(predicate::str::contains("packet:"));
}

#[test]
fn runs_show_latest_shows_run_details() {
    let tmp = TempDir::new().unwrap();
    collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "runs",
            "show",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "latest",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Run: run_fixture"))
        .stdout(predicate::str::contains("User: octo"))
        .stdout(predicate::str::contains("Window: 2025-01-01..2025-04-01"))
        .stdout(predicate::str::contains("Sources: github"))
        .stdout(predicate::str::contains("Events: 3"))
        .stdout(predicate::str::contains("Gaps: 0"))
        .stdout(predicate::str::contains("Warnings: none"));
}

#[test]
fn open_packet_latest_prints_packet_path_when_forced() {
    let tmp = TempDir::new().unwrap();
    collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "open",
            "packet",
            "--out",
            tmp.path().to_str().unwrap(),
            "--latest",
            "--print-path",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("run_fixture"))
        .stdout(predicate::str::contains("packet.md"));
}

#[test]
fn open_workstreams_latest_prints_effective_workstreams_path_when_forced() {
    let tmp = TempDir::new().unwrap();
    collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "open",
            "workstreams",
            "--out",
            tmp.path().to_str().unwrap(),
            "--latest",
            "--print-path",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("run_fixture"))
        .stdout(predicate::str::contains("workstreams.suggested.yaml"));
}

#[test]
fn open_out_latest_prints_run_directory_when_forced() {
    let tmp = TempDir::new().unwrap();
    collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "open",
            "out",
            "--out",
            tmp.path().to_str().unwrap(),
            "--latest",
            "--print-path",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("run_fixture"));
}

#[test]
fn workstreams_validate_accepts_latest_run_workstreams() {
    let tmp = TempDir::new().unwrap();
    collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "workstreams",
            "validate",
            "--out",
            tmp.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Workstreams valid:"))
        .stdout(predicate::str::contains("assigned events"));
}

#[test]
fn workstreams_validate_rejects_blank_title() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());
    std::fs::write(
        run_dir.join("workstreams.yaml"),
        r#"version: 1
generated_at: "2026-01-01T00:00:00Z"
workstreams:
  - id: "blank-title"
    title: ""
    summary: null
    tags: []
    stats:
      pull_requests: 1
      reviews: 0
      manual_events: 0
    events:
      - "fixture_pr_acme_payments_42"
    receipts:
      - "fixture_pr_acme_payments_42"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .args([
            "workstreams",
            "validate",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("blank title"));
}

#[test]
fn workstreams_rename_promotes_suggested_to_curated() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "workstreams",
            "rename",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--from",
            "acme/platform",
            "--to",
            "Platform Reliability",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Renamed workstream"))
        .stdout(predicate::str::contains("Created curated workstreams.yaml"));

    assert!(run_dir.join("workstreams.suggested.yaml").exists());
    let curated = load_curated_workstreams(&run_dir);
    assert!(
        curated
            .workstreams
            .iter()
            .any(|workstream| workstream.title == "Platform Reliability")
    );
    assert!(
        !curated
            .workstreams
            .iter()
            .any(|workstream| workstream.title == "acme/platform")
    );
}

#[test]
fn workstreams_move_event_reassigns_event_and_validates() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "workstreams",
            "move",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--event",
            "fixture_pr_acme_payments_42",
            "--to",
            "acme/platform",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Moved event fixture_pr_acme_payments_42 to acme/platform",
        ));

    let curated = load_curated_workstreams(&run_dir);
    let platform = curated
        .workstreams
        .iter()
        .find(|workstream| workstream.title == "acme/platform")
        .expect("platform workstream should exist");
    assert!(
        platform
            .events
            .iter()
            .any(|event_id| event_id.to_string() == "fixture_pr_acme_payments_42")
    );

    let payments = curated
        .workstreams
        .iter()
        .find(|workstream| workstream.title == "acme/payments")
        .expect("payments workstream should exist");
    assert!(
        payments
            .events
            .iter()
            .all(|event_id| event_id.to_string() != "fixture_pr_acme_payments_42")
    );

    shiplog_cmd()
        .args([
            "workstreams",
            "validate",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
        ])
        .assert()
        .success();
}

#[test]
fn workstreams_move_unknown_event_fails_without_writing_curated() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "workstreams",
            "move",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--event",
            "missing-event",
            "--to",
            "acme/platform",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "was not found in ledger.events.jsonl",
        ));

    assert!(!run_dir.join("workstreams.yaml").exists());
}

#[test]
fn workstreams_receipts_lists_selected_workstream_receipts() {
    let tmp = TempDir::new().unwrap();
    collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "workstreams",
            "receipts",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--workstream",
            "acme/platform",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Receipts: acme/platform"))
        .stdout(predicate::str::contains("Count: 2"))
        .stdout(predicate::str::contains(
            "Schema hardening for audit exports",
        ))
        .stdout(predicate::str::contains("APPROVED"));
}

#[test]
fn workstreams_receipt_remove_promotes_suggested_to_curated_and_keeps_assignment() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "workstreams",
            "receipt",
            "remove",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--workstream",
            "acme/platform",
            "--event",
            "fixture_pr_acme_platform_13",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Removed receipt anchor fixture_pr_acme_platform_13 from acme/platform",
        ))
        .stdout(predicate::str::contains("Created curated workstreams.yaml"));

    assert!(run_dir.join("workstreams.suggested.yaml").exists());
    let curated = load_curated_workstreams(&run_dir);
    let platform = curated
        .workstreams
        .iter()
        .find(|workstream| workstream.title == "acme/platform")
        .expect("platform workstream should exist");
    assert!(
        platform
            .events
            .iter()
            .any(|event_id| event_id.to_string() == "fixture_pr_acme_platform_13")
    );
    assert!(
        platform
            .receipts
            .iter()
            .all(|event_id| event_id.to_string() != "fixture_pr_acme_platform_13")
    );
}

#[test]
fn workstreams_receipt_add_promotes_suggested_to_curated_and_validates() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());
    std::fs::write(
        run_dir.join("workstreams.suggested.yaml"),
        r#"version: 1
generated_at: "2026-01-01T00:00:00Z"
workstreams:
  - id: "repo-acme-platform"
    title: "acme/platform"
    summary: null
    tags:
      - platform
    stats:
      pull_requests: 1
      reviews: 1
      manual_events: 0
    events:
      - "fixture_pr_acme_platform_13"
      - "fixture_review_acme_platform_77_1"
    receipts:
      - "fixture_review_acme_platform_77_1"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .args([
            "workstreams",
            "receipt",
            "add",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--workstream",
            "acme/platform",
            "--event",
            "fixture_pr_acme_platform_13",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Added receipt anchor fixture_pr_acme_platform_13 to acme/platform",
        ))
        .stdout(predicate::str::contains("Created curated workstreams.yaml"));

    let curated = load_curated_workstreams(&run_dir);
    let platform = curated
        .workstreams
        .iter()
        .find(|workstream| workstream.title == "acme/platform")
        .expect("platform workstream should exist");
    assert!(
        platform
            .receipts
            .iter()
            .any(|event_id| event_id.to_string() == "fixture_pr_acme_platform_13")
    );

    shiplog_cmd()
        .args([
            "workstreams",
            "validate",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
        ])
        .assert()
        .success();
}

#[test]
fn workstreams_receipt_add_rejects_event_not_in_workstream_without_writing() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "workstreams",
            "receipt",
            "add",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--workstream",
            "acme/platform",
            "--event",
            "fixture_pr_acme_payments_42",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("is not assigned to workstream"));

    assert!(!run_dir.join("workstreams.yaml").exists());
}

#[test]
fn workstreams_receipt_add_rejects_duplicate_without_writing() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "workstreams",
            "receipt",
            "add",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--workstream",
            "acme/platform",
            "--event",
            "fixture_pr_acme_platform_13",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already a receipt anchor"));

    assert!(!run_dir.join("workstreams.yaml").exists());
}

#[test]
fn workstreams_receipt_remove_rejects_missing_receipt_without_writing() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "workstreams",
            "receipt",
            "remove",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--workstream",
            "acme/platform",
            "--event",
            "fixture_pr_acme_payments_42",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("is not a receipt anchor"));

    assert!(!run_dir.join("workstreams.yaml").exists());
}

#[test]
fn workstreams_create_promotes_suggested_to_curated() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "workstreams",
            "create",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--title",
            "Platform Reliability",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Created workstream: Platform Reliability",
        ))
        .stdout(predicate::str::contains("Created curated workstreams.yaml"));

    assert!(run_dir.join("workstreams.suggested.yaml").exists());
    let curated = load_curated_workstreams(&run_dir);
    let created = curated
        .workstreams
        .iter()
        .find(|workstream| workstream.title == "Platform Reliability")
        .expect("created workstream should exist");
    assert!(created.events.is_empty());
    assert!(created.receipts.is_empty());
    assert_eq!(created.stats.pull_requests, 0);
    assert_eq!(created.stats.reviews, 0);
}

#[test]
fn workstreams_create_rejects_blank_and_duplicate_without_writing() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "workstreams",
            "create",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--title",
            "   ",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("title cannot be blank"));
    assert!(!run_dir.join("workstreams.yaml").exists());

    shiplog_cmd()
        .args([
            "workstreams",
            "create",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--title",
            "acme/platform",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already matches"));
    assert!(!run_dir.join("workstreams.yaml").exists());
}

#[test]
fn workstreams_delete_empty_workstream_succeeds() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "workstreams",
            "create",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--title",
            "Temporary Bucket",
        ])
        .assert()
        .success();

    shiplog_cmd()
        .args([
            "workstreams",
            "delete",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--workstream",
            "Temporary Bucket",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Deleted workstream: Temporary Bucket",
        ));

    let curated = load_curated_workstreams(&run_dir);
    assert!(
        !curated
            .workstreams
            .iter()
            .any(|workstream| workstream.title == "Temporary Bucket")
    );
}

#[test]
fn workstreams_delete_non_empty_requires_move_or_force_without_writing() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "workstreams",
            "delete",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--workstream",
            "acme/platform",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--move-to"))
        .stderr(predicate::str::contains("--force"));

    assert!(!run_dir.join("workstreams.yaml").exists());
}

#[test]
fn workstreams_delete_non_empty_moves_events_and_recomputes_stats() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "workstreams",
            "create",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--title",
            "Misc",
        ])
        .assert()
        .success();

    shiplog_cmd()
        .args([
            "workstreams",
            "delete",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--workstream",
            "acme/platform",
            "--move-to",
            "Misc",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Deleted workstream: acme/platform",
        ))
        .stdout(predicate::str::contains("Moved 2 event(s)"));

    let curated = load_curated_workstreams(&run_dir);
    assert!(
        !curated
            .workstreams
            .iter()
            .any(|workstream| workstream.title == "acme/platform")
    );
    let misc = curated
        .workstreams
        .iter()
        .find(|workstream| workstream.title == "Misc")
        .expect("move target should exist");
    assert_eq!(misc.stats.pull_requests, 1);
    assert_eq!(misc.stats.reviews, 1);
    assert!(
        misc.events
            .iter()
            .any(|event_id| event_id.to_string() == "fixture_pr_acme_platform_13")
    );
    assert!(
        misc.events
            .iter()
            .any(|event_id| event_id.to_string() == "fixture_review_acme_platform_77_1")
    );

    shiplog_cmd()
        .args([
            "workstreams",
            "validate",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
        ])
        .assert()
        .success();
}

#[test]
fn workstreams_delete_self_move_fails_without_writing() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "workstreams",
            "delete",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--workstream",
            "acme/platform",
            "--move-to",
            "acme/platform",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("must be different"));

    assert!(!run_dir.join("workstreams.yaml").exists());
}

#[test]
fn workstreams_delete_missing_move_target_fails_without_writing() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "workstreams",
            "delete",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--workstream",
            "acme/platform",
            "--move-to",
            "Missing Bucket",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no workstream matched"));

    assert!(!run_dir.join("workstreams.yaml").exists());
}

#[test]
fn workstreams_delete_force_discards_assignments_and_validates() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "workstreams",
            "delete",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--workstream",
            "acme/payments",
            "--force",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Deleted workstream: acme/payments",
        ))
        .stdout(predicate::str::contains("Discarded 1 event assignment"));

    let curated = load_curated_workstreams(&run_dir);
    assert!(
        !curated
            .workstreams
            .iter()
            .any(|workstream| workstream.title == "acme/payments")
    );
    shiplog_cmd()
        .args([
            "workstreams",
            "validate",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
        ])
        .assert()
        .success();
}

#[test]
fn workstreams_split_creates_target_and_recomputes_stats() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "workstreams",
            "split",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--from",
            "acme/platform",
            "--to",
            "Audit Exports",
            "--matching",
            "schema|compliance",
            "--create",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Split 1 event(s) from acme/platform to Audit Exports",
        ))
        .stdout(predicate::str::contains("Created target workstream"))
        .stdout(predicate::str::contains("Created curated workstreams.yaml"));

    assert!(run_dir.join("workstreams.suggested.yaml").exists());
    let curated = load_curated_workstreams(&run_dir);
    let audit = curated
        .workstreams
        .iter()
        .find(|workstream| workstream.title == "Audit Exports")
        .expect("split target should be created");
    assert_eq!(audit.stats.pull_requests, 1);
    assert_eq!(audit.stats.reviews, 0);
    assert!(
        audit
            .events
            .iter()
            .any(|event_id| event_id.to_string() == "fixture_pr_acme_platform_13")
    );

    let platform = curated
        .workstreams
        .iter()
        .find(|workstream| workstream.title == "acme/platform")
        .expect("source workstream should remain");
    assert_eq!(platform.stats.pull_requests, 0);
    assert_eq!(platform.stats.reviews, 1);
    assert!(
        platform
            .events
            .iter()
            .all(|event_id| event_id.to_string() != "fixture_pr_acme_platform_13")
    );

    shiplog_cmd()
        .args([
            "workstreams",
            "validate",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
        ])
        .assert()
        .success();
}

#[test]
fn workstreams_split_matches_source_url() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "workstreams",
            "split",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--from",
            "acme/platform",
            "--to",
            "Review Receipts",
            "--matching",
            "reviews/1",
            "--create",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Split 1 event(s) from acme/platform to Review Receipts",
        ));

    let curated = load_curated_workstreams(&run_dir);
    let review_receipts = curated
        .workstreams
        .iter()
        .find(|workstream| workstream.title == "Review Receipts")
        .expect("source-url split target should be created");
    assert!(
        review_receipts
            .events
            .iter()
            .any(|event_id| event_id.to_string() == "fixture_review_acme_platform_77_1")
    );
}

#[test]
fn workstreams_split_missing_target_requires_create_without_writing_curated() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "workstreams",
            "split",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--from",
            "acme/platform",
            "--to",
            "Audit Exports",
            "--matching",
            "Schema",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("add --create to create it"));

    assert!(!run_dir.join("workstreams.yaml").exists());
}

fn load_curated_workstreams(run_dir: &Path) -> WorkstreamsFile {
    let text = std::fs::read_to_string(run_dir.join("workstreams.yaml")).unwrap();
    serde_yaml::from_str(&text).unwrap()
}

#[test]
fn run_git_produces_outputs() {
    let Some(repo) = create_local_git_repo() else {
        eprintln!("skipping run_git_produces_outputs: git not available");
        return;
    };
    let out = TempDir::new().unwrap();

    shiplog_cmd()
        .args([
            "run",
            "--out",
            out.path().to_str().unwrap(),
            "git",
            "--repo",
            repo.path().to_str().unwrap(),
            "--since",
            "2025-01-01",
            "--until",
            "2025-02-01",
        ])
        .assert()
        .success();

    let run_dir = first_run_dir(out.path());
    assert!(run_dir.join("packet.md").exists(), "missing packet.md");
    assert!(
        run_dir.join("ledger.events.jsonl").exists(),
        "missing ledger.events.jsonl"
    );
    assert!(
        run_dir.join("coverage.manifest.json").exists(),
        "missing coverage.manifest.json"
    );
    assert!(
        run_dir.join("workstreams.suggested.yaml").exists(),
        "missing workstreams.suggested.yaml"
    );
}

#[test]
fn refresh_git_preserves_existing_workstreams() {
    let Some(repo) = create_local_git_repo() else {
        eprintln!("skipping refresh_git_preserves_existing_workstreams: git not available");
        return;
    };
    let out = TempDir::new().unwrap();

    shiplog_cmd()
        .args([
            "collect",
            "--out",
            out.path().to_str().unwrap(),
            "git",
            "--repo",
            repo.path().to_str().unwrap(),
            "--since",
            "2025-01-01",
            "--until",
            "2025-02-01",
        ])
        .assert()
        .success();

    let run_dir = first_run_dir(out.path());
    shiplog_cmd()
        .args([
            "refresh",
            "--out",
            out.path().to_str().unwrap(),
            "--run-dir",
            run_dir.to_str().unwrap(),
            "git",
            "--repo",
            repo.path().to_str().unwrap(),
            "--since",
            "2025-01-01",
            "--until",
            "2025-02-01",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Refreshed while preserving workstream curation",
        ));

    assert!(run_dir.join("packet.md").exists(), "missing packet.md");
    assert!(
        run_dir.join("ledger.events.jsonl").exists(),
        "missing ledger.events.jsonl"
    );
    assert!(
        run_dir.join("workstreams.suggested.yaml").exists(),
        "missing workstreams.suggested.yaml"
    );
}

#[test]
fn refresh_run_dir_latest_alias_on_collected_directory() {
    let tmp = TempDir::new().unwrap();
    let fixtures = fixture_dir();
    let _run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "refresh",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run-dir",
            "latest",
            "json",
            "--events",
            fixtures.join("ledger.events.jsonl").to_str().unwrap(),
            "--coverage",
            fixtures.join("coverage.manifest.json").to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Refreshed while preserving workstream curation",
        ));

    assert!(
        tmp.path().join("run_fixture/packet.md").exists(),
        "packet.md should exist after refresh --run-dir latest"
    );
}

// ── 6. render on a pre-populated output directory ──────────────────────────

#[test]
fn render_on_collected_directory() {
    let tmp = TempDir::new().unwrap();
    let _run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "render",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Rendered"));

    assert!(
        tmp.path().join("run_fixture/packet.md").exists(),
        "packet.md should exist after render"
    );
}

#[test]
fn render_scaffold_mode_writes_prompt_focused_packet() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "render",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--mode",
            "scaffold",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Rendered"));

    let packet = std::fs::read_to_string(run_dir.join("packet.md")).unwrap();
    assert!(packet.contains("## Coverage and Limits"));
    assert!(packet.contains("**Suggested claim prompts**"));
    assert!(
        !packet.contains("\n## Receipts\n"),
        "scaffold mode should omit the full receipts section"
    );
    assert!(
        !packet.contains("\n## Appendix: All Receipts\n"),
        "scaffold mode should omit the full appendix"
    );
}

#[test]
fn render_receipts_mode_writes_audit_focused_packet() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "render",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--mode",
            "receipts",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Rendered"));

    let packet = std::fs::read_to_string(run_dir.join("packet.md")).unwrap();
    assert!(packet.contains("## Coverage and Limits"));
    assert!(packet.contains("\n## Receipts\n"));
    assert!(packet.contains("\n## Appendix: All Receipts\n"));
    assert!(
        !packet.contains("**Suggested claim prompts**"),
        "receipts mode should omit writing prompts"
    );
}

#[test]
fn render_receipt_limit_summary_appendix_preserves_canonical_files() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());
    let ledger_before = std::fs::read_to_string(run_dir.join("ledger.events.jsonl")).unwrap();
    let coverage_before = std::fs::read_to_string(run_dir.join("coverage.manifest.json")).unwrap();

    shiplog_cmd()
        .args([
            "render",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--mode",
            "packet",
            "--receipt-limit",
            "1",
            "--appendix",
            "summary",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Rendered"));

    let packet = std::fs::read_to_string(run_dir.join("packet.md")).unwrap();
    assert!(packet.contains("\n## Receipts\n"));
    assert!(packet.contains("... and"));
    assert!(packet.contains("## Appendix: Receipt Summary"));
    assert!(!packet.contains("## Appendix: All Receipts"));
    assert!(packet.contains("- Assigned events:"));
    assert!(packet.contains("- Curated receipt anchors:"));
    assert_eq!(
        ledger_before,
        std::fs::read_to_string(run_dir.join("ledger.events.jsonl")).unwrap()
    );
    assert_eq!(
        coverage_before,
        std::fs::read_to_string(run_dir.join("coverage.manifest.json")).unwrap()
    );
}

#[test]
fn render_receipt_limit_zero_appendix_none_keeps_omission_notice() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());
    let ledger_before = std::fs::read_to_string(run_dir.join("ledger.events.jsonl")).unwrap();
    let coverage_before = std::fs::read_to_string(run_dir.join("coverage.manifest.json")).unwrap();

    shiplog_cmd()
        .args([
            "render",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--mode",
            "packet",
            "--receipt-limit",
            "0",
            "--appendix",
            "none",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Rendered"));

    let packet = std::fs::read_to_string(run_dir.join("packet.md")).unwrap();
    assert!(packet.contains("\n## Receipts\n"));
    assert!(packet.contains("- (none)"));
    assert!(packet.contains("omitted by appendix settings"));
    assert!(!packet.contains("## Appendix:"));
    assert_eq!(
        ledger_before,
        std::fs::read_to_string(run_dir.join("ledger.events.jsonl")).unwrap()
    );
    assert_eq!(
        coverage_before,
        std::fs::read_to_string(run_dir.join("coverage.manifest.json")).unwrap()
    );
}

#[test]
fn render_receipts_mode_honors_receipt_controls() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "render",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--mode",
            "receipts",
            "--receipt-limit",
            "1",
            "--appendix",
            "full",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Rendered"));

    let packet = std::fs::read_to_string(run_dir.join("packet.md")).unwrap();
    assert!(packet.contains("\n## Receipts\n"));
    assert!(packet.contains("... and"));
    assert!(packet.contains("## Appendix: All Receipts"));
    assert!(!packet.contains("**Suggested claim prompts**"));
}

#[test]
fn render_public_profile_without_key_fails_closed() {
    let tmp = TempDir::new().unwrap();
    collect_json_into(tmp.path());

    let mut cmd = shiplog_cmd();
    cmd.env_remove("SHIPLOG_REDACT_KEY")
        .args([
            "render",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--bundle-profile",
            "public",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "public profile requires --redact-key or SHIPLOG_REDACT_KEY",
        ));
}

#[test]
fn render_public_profile_with_key_writes_public_packet() {
    let tmp = TempDir::new().unwrap();
    collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "render",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--bundle-profile",
            "public",
            "--redact-key",
            "stable-test-key",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Rendered"));

    assert!(
        tmp.path()
            .join("run_fixture/profiles/public/packet.md")
            .exists(),
        "public packet should be written when a redaction key is provided"
    );
}

#[test]
fn render_latest_on_collected_directory() {
    let tmp = TempDir::new().unwrap();
    let _run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args(["render", "--out", tmp.path().to_str().unwrap(), "--latest"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Rendered"));

    assert!(
        tmp.path().join("run_fixture/packet.md").exists(),
        "packet.md should exist after render --latest"
    );
}

#[test]
fn render_run_latest_alias_on_collected_directory() {
    let tmp = TempDir::new().unwrap();
    let _run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "render",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "latest",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Rendered"));
}

#[test]
fn render_nonexistent_run_dir_fails() {
    let tmp = TempDir::new().unwrap();

    shiplog_cmd()
        .args([
            "render",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "nonexistent_run",
        ])
        .assert()
        .failure();
}

// ── 7. invalid subcommand returns error ────────────────────────────────────

#[test]
fn invalid_subcommand_returns_error() {
    shiplog_cmd()
        .arg("nonexistent")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand"));
}

#[test]
fn no_subcommand_returns_error() {
    shiplog_cmd()
        .assert()
        .failure()
        .stderr(predicate::str::is_empty().not());
}

// ── 8. missing required args return helpful error messages ─────────────────

#[test]
fn collect_without_source_fails_with_help() {
    shiplog_cmd().arg("collect").assert().failure().stderr(
        predicate::str::contains("github")
            .or(predicate::str::contains("json"))
            .or(predicate::str::contains("subcommand")),
    );
}

#[test]
fn collect_github_missing_user_fails() {
    shiplog_cmd()
        .args([
            "collect",
            "github",
            "--since",
            "2025-01-01",
            "--until",
            "2025-12-31",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("provide --user").or(predicate::str::contains("--me")));
}

#[test]
fn collect_github_user_and_me_conflict_fails() {
    shiplog_cmd()
        .args(["collect", "github", "--user", "octocat", "--me"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("use either --user or --me"));
}

#[test]
fn collect_github_me_without_token_fails_actionably() {
    shiplog_cmd()
        .env_remove("GITHUB_TOKEN")
        .args(["collect", "github", "--me"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Could not infer GitHub user: --me requires --token or GITHUB_TOKEN",
        ));
}

#[test]
fn collect_gitlab_me_without_token_fails_actionably() {
    shiplog_cmd()
        .env_remove("GITLAB_TOKEN")
        .args(["collect", "gitlab", "--me"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Could not infer GitLab user: --me requires --token or GITLAB_TOKEN",
        ));
}

#[test]
fn collect_github_partial_date_window_fails() {
    shiplog_cmd()
        .args([
            "collect",
            "github",
            "--user",
            "octocat",
            "--until",
            "2025-12-31",
        ])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("provide both --since and --until")
                .or(predicate::str::contains("error")),
        );
}

#[test]
fn collect_github_invalid_date_fails() {
    shiplog_cmd()
        .args([
            "collect",
            "github",
            "--user",
            "octocat",
            "--since",
            "not-a-date",
            "--until",
            "2025-12-31",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn collect_json_missing_coverage_fails() {
    shiplog_cmd()
        .args(["collect", "json", "--events", "some_file.jsonl"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--coverage").or(predicate::str::contains("required")));
}

#[test]
fn collect_json_missing_events_file_fails() {
    let tmp = TempDir::new().unwrap();

    shiplog_cmd()
        .args([
            "collect",
            "--out",
            tmp.path().to_str().unwrap(),
            "json",
            "--events",
            "/nonexistent/events.jsonl",
            "--coverage",
            "/nonexistent/coverage.json",
        ])
        .assert()
        .failure();
}

#[test]
fn render_unknown_flag_fails() {
    shiplog_cmd()
        .args(["render", "--bogus-flag"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("unexpected argument").or(predicate::str::contains("error")),
        );
}

// ── 9. import subcommand exists and shows help ─────────────────────────────

#[test]
fn import_help_shows_options() {
    shiplog_cmd()
        .args(["import", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--dir"))
        .stdout(predicate::str::contains("--out"))
        .stdout(predicate::str::contains("--user"));
}

#[test]
fn import_from_fixture_dir_succeeds() {
    let tmp = TempDir::new().unwrap();
    let fixtures = fixture_dir();

    shiplog_cmd()
        .args([
            "import",
            "--dir",
            fixtures.to_str().unwrap(),
            "--out",
            tmp.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Imported"));

    let run_dir = tmp.path().join("run_fixture");
    assert!(run_dir.join("packet.md").exists());
    assert!(run_dir.join("ledger.events.jsonl").exists());
    assert!(run_dir.join("coverage.manifest.json").exists());
}

#[test]
fn import_missing_dir_fails() {
    let tmp = TempDir::new().unwrap();

    shiplog_cmd()
        .args([
            "import",
            "--dir",
            tmp.path().join("nonexistent").to_str().unwrap(),
            "--out",
            tmp.path().to_str().unwrap(),
        ])
        .assert()
        .failure();
}

// ── additional subcommand help checks ──────────────────────────────────────

#[test]
fn refresh_help_shows_options() {
    shiplog_cmd()
        .args(["refresh", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("github"))
        .stdout(predicate::str::contains("multi").not())
        .stdout(predicate::str::contains("--out"))
        .stdout(predicate::str::contains("--run-dir"));
}

#[test]
fn run_help_shows_options() {
    shiplog_cmd()
        .args(["run", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("github"))
        .stdout(predicate::str::contains("multi").not())
        .stdout(predicate::str::contains("--out"));
}
