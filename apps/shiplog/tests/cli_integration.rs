//! Comprehensive CLI integration tests using `assert_cmd` and `predicates`.

use assert_cmd::Command;
use predicates::prelude::*;
use std::path::{Path, PathBuf};
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
        .stdout(predicate::str::contains("collect"))
        .stdout(predicate::str::contains("render"))
        .stdout(predicate::str::contains("refresh"))
        .stdout(predicate::str::contains("import"))
        .stdout(predicate::str::contains("run"));
}

// ── 3. collect --help shows collect-specific options ───────────────────────

#[test]
fn collect_help_shows_sources_and_options() {
    shiplog_cmd()
        .args(["collect", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("github"))
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
        .stdout(predicate::str::contains("--since"))
        .stdout(predicate::str::contains("--until"))
        .stdout(predicate::str::contains("--mode"))
        .stdout(predicate::str::contains("--include-reviews"))
        .stdout(predicate::str::contains("--no-details"));
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
        .stdout(predicate::str::contains("--user"))
        .stdout(predicate::str::contains("--redact-key"));
}

// ── 5. collect json with sample fixture data ───────────────────────────────

#[test]
fn collect_json_produces_all_outputs() {
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
        .stderr(predicate::str::contains("--user").or(predicate::str::contains("required")));
}

#[test]
fn collect_github_missing_since_fails() {
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
        .stderr(predicate::str::contains("--since").or(predicate::str::contains("required")));
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
