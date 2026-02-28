use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

fn shiplog_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_shiplog"))
}

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("examples/fixture")
}

// ── help / usage ────────────────────────────────────────────────────────────

#[test]
fn help_flag_shows_usage() {
    let out = shiplog_bin()
        .arg("--help")
        .output()
        .expect("failed to run shiplog --help");

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("shiplog"), "expected binary name in help");
    assert!(stdout.contains("collect"), "expected collect subcommand");
    assert!(stdout.contains("render"), "expected render subcommand");
    assert!(stdout.contains("import"), "expected import subcommand");
    assert!(stdout.contains("run"), "expected run subcommand");
}

#[test]
fn collect_help_lists_sources() {
    let out = shiplog_bin()
        .args(["collect", "--help"])
        .output()
        .expect("failed to run shiplog collect --help");

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("github"), "expected github source");
    assert!(stdout.contains("json"), "expected json source");
}

#[test]
fn github_help_mentions_no_details() {
    let out = shiplog_bin()
        .args(["collect", "github", "--help"])
        .output()
        .expect("failed to run shiplog collect github --help");

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("--no-details"),
        "expected --no-details flag in github help"
    );
}

// ── collect json ────────────────────────────────────────────────────────────

#[test]
fn collect_json_produces_outputs() {
    let tmp = TempDir::new().unwrap();
    let fixtures = fixture_dir();

    let out = shiplog_bin()
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
        .output()
        .expect("failed to run shiplog collect json");

    assert!(
        out.status.success(),
        "collect json failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let run_dir = tmp.path().join("run_fixture");
    assert!(run_dir.join("ledger.events.jsonl").exists());
    assert!(run_dir.join("coverage.manifest.json").exists());
    assert!(run_dir.join("packet.md").exists());
    assert!(run_dir.join("workstreams.suggested.yaml").exists());
    assert!(run_dir.join("bundle.manifest.json").exists());
}

#[test]
fn collect_json_with_custom_user_and_window() {
    let tmp = TempDir::new().unwrap();
    let fixtures = fixture_dir();

    let out = shiplog_bin()
        .args([
            "collect",
            "--out",
            tmp.path().to_str().unwrap(),
            "json",
            "--events",
            fixtures.join("ledger.events.jsonl").to_str().unwrap(),
            "--coverage",
            fixtures.join("coverage.manifest.json").to_str().unwrap(),
            "--user",
            "testbot",
            "--window-label",
            "Q1-2025",
        ])
        .output()
        .expect("failed to run shiplog collect json");

    assert!(
        out.status.success(),
        "collect json with custom user failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let run_dir = tmp.path().join("run_fixture");
    let packet = std::fs::read_to_string(run_dir.join("packet.md")).unwrap();
    assert!(
        packet.contains("Q1-2025"),
        "packet should contain custom window label"
    );
}

// ── render ──────────────────────────────────────────────────────────────────

#[test]
fn render_after_collect_json() {
    let tmp = TempDir::new().unwrap();
    let fixtures = fixture_dir();

    // Step 1: collect
    let out = shiplog_bin()
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
        .output()
        .expect("collect failed");
    assert!(
        out.status.success(),
        "collect failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Step 2: render
    let out = shiplog_bin()
        .args([
            "render",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
        ])
        .output()
        .expect("render failed");

    assert!(
        out.status.success(),
        "render failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Rendered"),
        "expected 'Rendered' in output, got: {stdout}"
    );
}

#[test]
fn render_missing_run_dir_fails() {
    let tmp = TempDir::new().unwrap();

    let out = shiplog_bin()
        .args([
            "render",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "nonexistent_run",
        ])
        .output()
        .expect("failed to run shiplog render");

    assert!(!out.status.success(), "render on missing dir should fail");
}

// ── import ──────────────────────────────────────────────────────────────────

#[test]
fn import_from_fixture_dir() {
    let tmp = TempDir::new().unwrap();
    let fixtures = fixture_dir();

    let out = shiplog_bin()
        .args([
            "import",
            "--dir",
            fixtures.to_str().unwrap(),
            "--out",
            tmp.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to run shiplog import");

    assert!(
        out.status.success(),
        "import failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Imported"),
        "expected 'Imported' in output, got: {stdout}"
    );

    let run_dir = tmp.path().join("run_fixture");
    assert!(run_dir.join("packet.md").exists());
    assert!(run_dir.join("ledger.events.jsonl").exists());
    assert!(run_dir.join("coverage.manifest.json").exists());
}

#[test]
fn import_missing_dir_fails() {
    let tmp = TempDir::new().unwrap();

    let out = shiplog_bin()
        .args([
            "import",
            "--dir",
            tmp.path().join("nonexistent").to_str().unwrap(),
            "--out",
            tmp.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to run shiplog import");

    assert!(
        !out.status.success(),
        "import from missing dir should fail"
    );
}

// ── error cases ─────────────────────────────────────────────────────────────

#[test]
fn invalid_subcommand_fails() {
    let out = shiplog_bin()
        .arg("nonexistent")
        .output()
        .expect("failed to run shiplog");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unrecognized subcommand"),
        "expected error about unrecognized subcommand, got: {stderr}"
    );
}

#[test]
fn collect_json_missing_events_file_fails() {
    let tmp = TempDir::new().unwrap();

    let out = shiplog_bin()
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
        .output()
        .expect("failed to run shiplog");

    assert!(
        !out.status.success(),
        "collect json with missing files should fail"
    );
}

#[test]
fn collect_without_source_fails() {
    let out = shiplog_bin()
        .arg("collect")
        .output()
        .expect("failed to run shiplog collect");

    assert!(
        !out.status.success(),
        "collect without source subcommand should fail"
    );
}
