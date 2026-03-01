//! Cross-platform path handling tests for the shiplog CLI.
//!
//! Ensures the CLI correctly handles path arguments with spaces, unicode,
//! and special characters on all platforms.

use assert_cmd::Command;
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

// ---------------------------------------------------------------------------
// collect json with spaces in --out path
// ---------------------------------------------------------------------------

#[test]
fn collect_json_with_spaces_in_out_path() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("output with spaces");
    let fixtures = fixture_dir();

    shiplog_cmd()
        .args([
            "collect",
            "--out",
            out.to_str().unwrap(),
            "json",
            "--events",
            fixtures.join("ledger.events.jsonl").to_str().unwrap(),
            "--coverage",
            fixtures.join("coverage.manifest.json").to_str().unwrap(),
        ])
        .assert()
        .success();

    let run_dir = out.join("run_fixture");
    assert!(
        run_dir.join("packet.md").exists(),
        "packet.md missing with spaces in --out path"
    );
    assert!(run_dir.join("ledger.events.jsonl").exists());
    assert!(run_dir.join("coverage.manifest.json").exists());
    assert!(run_dir.join("bundle.manifest.json").exists());
}

// ---------------------------------------------------------------------------
// collect json with unicode in --out path
// ---------------------------------------------------------------------------

#[test]
fn collect_json_with_unicode_in_out_path() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("données_出力");
    let fixtures = fixture_dir();

    shiplog_cmd()
        .args([
            "collect",
            "--out",
            out.to_str().unwrap(),
            "json",
            "--events",
            fixtures.join("ledger.events.jsonl").to_str().unwrap(),
            "--coverage",
            fixtures.join("coverage.manifest.json").to_str().unwrap(),
        ])
        .assert()
        .success();

    let run_dir = out.join("run_fixture");
    assert!(
        run_dir.join("packet.md").exists(),
        "packet.md missing with unicode in --out path"
    );
}

// ---------------------------------------------------------------------------
// collect json with special characters in --out path
// ---------------------------------------------------------------------------

#[test]
fn collect_json_with_special_chars_in_out_path() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("project (v2.0-beta)");
    let fixtures = fixture_dir();

    shiplog_cmd()
        .args([
            "collect",
            "--out",
            out.to_str().unwrap(),
            "json",
            "--events",
            fixtures.join("ledger.events.jsonl").to_str().unwrap(),
            "--coverage",
            fixtures.join("coverage.manifest.json").to_str().unwrap(),
        ])
        .assert()
        .success();

    let run_dir = out.join("run_fixture");
    assert!(run_dir.join("packet.md").exists());
}

// ---------------------------------------------------------------------------
// render with spaces in --out path (after collect)
// ---------------------------------------------------------------------------

#[test]
fn render_with_spaces_in_out_path() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("render output dir");
    let fixtures = fixture_dir();

    // First, collect
    shiplog_cmd()
        .args([
            "collect",
            "--out",
            out.to_str().unwrap(),
            "json",
            "--events",
            fixtures.join("ledger.events.jsonl").to_str().unwrap(),
            "--coverage",
            fixtures.join("coverage.manifest.json").to_str().unwrap(),
        ])
        .assert()
        .success();

    let run_dir = out.join("run_fixture");
    assert!(run_dir.join("packet.md").exists());

    // Then, render with the same spaced path
    shiplog_cmd()
        .args([
            "render",
            "--out",
            out.to_str().unwrap(),
            "--run",
            "run_fixture",
            "--user",
            "tester",
        ])
        .assert()
        .success();

    assert!(
        run_dir.join("packet.md").exists(),
        "packet.md should exist after re-render with spaces in path"
    );
}

// ---------------------------------------------------------------------------
// import with spaces in path
// ---------------------------------------------------------------------------

#[test]
fn import_with_spaces_in_paths() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("import output dir");
    let fixtures = fixture_dir();

    // First collect to create a run directory with ledger artifacts
    shiplog_cmd()
        .args([
            "collect",
            "--out",
            out.to_str().unwrap(),
            "json",
            "--events",
            fixtures.join("ledger.events.jsonl").to_str().unwrap(),
            "--coverage",
            fixtures.join("coverage.manifest.json").to_str().unwrap(),
        ])
        .assert()
        .success();

    let run_dir = out.join("run_fixture");

    // Import from that run directory using --dir (the correct flag)
    let import_out = tmp.path().join("import dest dir");
    shiplog_cmd()
        .args([
            "import",
            "--dir",
            run_dir.to_str().unwrap(),
            "--out",
            import_out.to_str().unwrap(),
        ])
        .assert()
        .success();
}

// ---------------------------------------------------------------------------
// Deeply nested --out path
// ---------------------------------------------------------------------------

#[test]
fn collect_json_with_deeply_nested_out_path() {
    let tmp = TempDir::new().unwrap();
    let out = tmp
        .path()
        .join("level1")
        .join("level2")
        .join("level3")
        .join("output");
    let fixtures = fixture_dir();

    shiplog_cmd()
        .args([
            "collect",
            "--out",
            out.to_str().unwrap(),
            "json",
            "--events",
            fixtures.join("ledger.events.jsonl").to_str().unwrap(),
            "--coverage",
            fixtures.join("coverage.manifest.json").to_str().unwrap(),
        ])
        .assert()
        .success();

    let run_dir = out.join("run_fixture");
    assert!(run_dir.join("packet.md").exists());
}
