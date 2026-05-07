//! Comprehensive CLI integration tests using `assert_cmd` and `predicates`.

use assert_cmd::Command;
use predicates::prelude::*;
use shiplog_schema::workstream::WorkstreamsFile;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use tempfile::TempDir;

fn shiplog_cmd() -> Command {
    Command::from_std(std::process::Command::new(env!("CARGO_BIN_EXE_shiplog")))
}

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("examples/fixture")
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
    std::fs::write(
        &manual_events,
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
        .stdout(predicate::str::contains("collect"))
        .stdout(predicate::str::contains("render"))
        .stdout(predicate::str::contains("refresh"))
        .stdout(predicate::str::contains("workstreams"))
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
fn workstreams_help_shows_list_and_validate() {
    shiplog_cmd()
        .args(["workstreams", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("validate"))
        .stdout(predicate::str::contains("rename"))
        .stdout(predicate::str::contains("move"));
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
        .stdout(predicate::str::contains("shiplog collect github"));

    let config = std::fs::read_to_string(tmp.path().join("shiplog.toml")).unwrap();
    assert!(config.contains("[sources.github]"));
    assert!(config.contains("enabled = true"));
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

// ── 3. collect --help shows collect-specific options ───────────────────────

#[test]
fn collect_help_shows_sources_and_options() {
    shiplog_cmd()
        .args(["collect", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("github"))
        .stdout(predicate::str::contains("gitlab"))
        .stdout(predicate::str::contains("jira"))
        .stdout(predicate::str::contains("linear"))
        .stdout(predicate::str::contains("json"))
        .stdout(predicate::str::contains("--out"))
        .stdout(predicate::str::contains("--regen"));
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
        .stdout(predicate::str::contains("--redact-key"));
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
        .stdout(predicate::str::contains("--out"));
}
