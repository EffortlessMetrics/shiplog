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

/// Run `collect json` into `tmp` and return the run directory path.
fn collect_json_into(tmp: &Path) -> PathBuf {
    let fixtures = fixture_dir();
    let out = shiplog_bin()
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
        .output()
        .expect("failed to run shiplog collect json");
    assert!(
        out.status.success(),
        "collect json setup failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    tmp.join("run_fixture")
}

// ── 1. help text for each subcommand ───────────────────────────────────────

#[test]
fn render_help_shows_usage() {
    let out = shiplog_bin()
        .args(["render", "--help"])
        .output()
        .expect("failed to run shiplog render --help");

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("--out"), "render help should mention --out");
    assert!(stdout.contains("--run"), "render help should mention --run");
}

#[test]
fn refresh_help_shows_usage() {
    let out = shiplog_bin()
        .args(["refresh", "--help"])
        .output()
        .expect("failed to run shiplog refresh --help");

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("github"),
        "refresh help should list github source"
    );
    assert!(
        stdout.contains("--out"),
        "refresh help should mention --out"
    );
}

#[test]
fn import_help_shows_usage() {
    let out = shiplog_bin()
        .args(["import", "--help"])
        .output()
        .expect("failed to run shiplog import --help");

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("--dir"), "import help should mention --dir");
    assert!(stdout.contains("--out"), "import help should mention --out");
}

#[test]
fn run_help_shows_usage() {
    let out = shiplog_bin()
        .args(["run", "--help"])
        .output()
        .expect("failed to run shiplog run --help");

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("github"),
        "run help should list github source"
    );
    assert!(stdout.contains("--out"), "run help should mention --out");
}

#[test]
fn collect_json_help_shows_flags() {
    let out = shiplog_bin()
        .args(["collect", "json", "--help"])
        .output()
        .expect("failed to run shiplog collect json --help");

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("--events"),
        "collect json help should mention --events"
    );
    assert!(
        stdout.contains("--coverage"),
        "collect json help should mention --coverage"
    );
}

// ── 2. version flag ────────────────────────────────────────────────────────

#[test]
fn version_flag_prints_version() {
    let out = shiplog_bin()
        .arg("--version")
        .output()
        .expect("failed to run shiplog --version");

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("shiplog"),
        "version output should contain binary name, got: {stdout}"
    );
    // Version string should match semver-like pattern
    assert!(
        stdout.contains('.'),
        "version output should contain a dotted version number, got: {stdout}"
    );
}

// ── 3. error messages for invalid arguments ────────────────────────────────

#[test]
fn collect_github_invalid_since_date_fails() {
    let out = shiplog_bin()
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
        .output()
        .expect("failed to run shiplog");

    assert!(
        !out.status.success(),
        "invalid --since date should cause failure"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("invalid value") || stderr.contains("error"),
        "stderr should report the invalid date, got: {stderr}"
    );
}

#[test]
fn collect_github_invalid_until_date_fails() {
    let out = shiplog_bin()
        .args([
            "collect",
            "github",
            "--user",
            "octocat",
            "--since",
            "2025-01-01",
            "--until",
            "01/31/2025",
        ])
        .output()
        .expect("failed to run shiplog");

    assert!(
        !out.status.success(),
        "invalid --until date format should cause failure"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("invalid value") || stderr.contains("error"),
        "stderr should report the invalid date, got: {stderr}"
    );
}

#[test]
fn collect_github_missing_user_fails() {
    let out = shiplog_bin()
        .args([
            "collect",
            "github",
            "--since",
            "2025-01-01",
            "--until",
            "2025-12-31",
        ])
        .output()
        .expect("failed to run shiplog");

    assert!(!out.status.success(), "missing --user should cause failure");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--user") || stderr.contains("required"),
        "stderr should mention the missing --user arg, got: {stderr}"
    );
}

#[test]
fn collect_json_missing_coverage_arg_fails() {
    let out = shiplog_bin()
        .args(["collect", "json", "--events", "some_file.jsonl"])
        .output()
        .expect("failed to run shiplog");

    assert!(
        !out.status.success(),
        "missing --coverage should cause failure"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--coverage") || stderr.contains("required"),
        "stderr should mention the missing --coverage arg, got: {stderr}"
    );
}

#[test]
fn render_with_unknown_flag_fails() {
    let out = shiplog_bin()
        .args(["render", "--bogus-flag"])
        .output()
        .expect("failed to run shiplog render");

    assert!(!out.status.success(), "unknown flag should cause failure");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unexpected argument") || stderr.contains("error"),
        "stderr should report the unknown flag, got: {stderr}"
    );
}

// ── 4. --no-details flag behaviour ─────────────────────────────────────────

#[test]
fn collect_github_help_mentions_no_details() {
    // Verify the --no-details flag is documented in github subcommand help
    let out = shiplog_bin()
        .args(["collect", "github", "--help"])
        .output()
        .expect("failed to run shiplog collect github --help");

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("--no-details"),
        "collect github help should list --no-details"
    );
}

#[test]
fn run_github_help_mentions_no_details() {
    // --no-details is also available on the `run` subcommand's github source
    let out = shiplog_bin()
        .args(["run", "github", "--help"])
        .output()
        .expect("failed to run shiplog run github --help");

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("--no-details"),
        "run github help should list --no-details"
    );
}

// ── 5. --regen flag with collect json ──────────────────────────────────────

#[test]
fn regen_regenerates_suggested_workstreams() {
    let tmp = TempDir::new().unwrap();
    let fixtures = fixture_dir();

    // First collect: creates workstreams.suggested.yaml
    let run_dir = collect_json_into(tmp.path());
    let suggested = run_dir.join("workstreams.suggested.yaml");
    assert!(
        suggested.exists(),
        "first collect should create suggested yaml"
    );

    // Modify the suggested file so we can detect regeneration
    std::fs::write(&suggested, "# marker: original\n").unwrap();

    // Second collect with --regen: should regenerate workstreams.suggested.yaml
    let out = shiplog_bin()
        .args([
            "collect",
            "--out",
            tmp.path().to_str().unwrap(),
            "--regen",
            "json",
            "--events",
            fixtures.join("ledger.events.jsonl").to_str().unwrap(),
            "--coverage",
            fixtures.join("coverage.manifest.json").to_str().unwrap(),
        ])
        .output()
        .expect("failed to run shiplog collect json --regen");

    assert!(
        out.status.success(),
        "collect json --regen failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // The suggested file should have been regenerated (our marker is gone)
    let content = std::fs::read_to_string(&suggested).unwrap();
    assert!(
        !content.contains("# marker: original"),
        "workstreams.suggested.yaml should have been regenerated by --regen"
    );
}

#[test]
fn regen_does_not_overwrite_curated_workstreams() {
    let tmp = TempDir::new().unwrap();
    let fixtures = fixture_dir();

    // First collect
    let run_dir = collect_json_into(tmp.path());

    // Simulate user curation: copy suggested → curated workstreams.yaml
    let suggested = run_dir.join("workstreams.suggested.yaml");
    let curated = run_dir.join("workstreams.yaml");
    std::fs::copy(&suggested, &curated).unwrap();
    // Add a user marker
    let mut curated_content = std::fs::read_to_string(&curated).unwrap();
    curated_content.push_str("\n# user curation marker\n");
    std::fs::write(&curated, &curated_content).unwrap();

    // Re-collect with --regen
    let out = shiplog_bin()
        .args([
            "collect",
            "--out",
            tmp.path().to_str().unwrap(),
            "--regen",
            "json",
            "--events",
            fixtures.join("ledger.events.jsonl").to_str().unwrap(),
            "--coverage",
            fixtures.join("coverage.manifest.json").to_str().unwrap(),
        ])
        .output()
        .expect("failed to run shiplog collect json --regen");

    assert!(
        out.status.success(),
        "collect json --regen (with curated) failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // workstreams.yaml should be untouched
    let after = std::fs::read_to_string(&curated).unwrap();
    assert!(
        after.contains("# user curation marker"),
        "workstreams.yaml should NOT be overwritten by --regen"
    );
}

// ── 6. import subcommand with a pre-built run directory ────────────────────

#[test]
fn import_from_collected_run_dir() {
    let collect_tmp = TempDir::new().unwrap();
    let import_tmp = TempDir::new().unwrap();

    // Build a run directory via collect json
    let run_dir = collect_json_into(collect_tmp.path());
    assert!(run_dir.join("ledger.events.jsonl").exists());
    assert!(run_dir.join("coverage.manifest.json").exists());

    // Import from that pre-built run directory
    let out = shiplog_bin()
        .args([
            "import",
            "--dir",
            run_dir.to_str().unwrap(),
            "--out",
            import_tmp.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to run shiplog import");

    assert!(
        out.status.success(),
        "import from collected run dir failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Imported"),
        "expected 'Imported' in output, got: {stdout}"
    );

    // Verify outputs exist in the import target
    let import_run_dir = import_tmp.path().join("run_fixture");
    assert!(
        import_run_dir.join("packet.md").exists(),
        "imported run should contain packet.md"
    );
    assert!(
        import_run_dir.join("ledger.events.jsonl").exists(),
        "imported run should contain ledger.events.jsonl"
    );
    assert!(
        import_run_dir.join("coverage.manifest.json").exists(),
        "imported run should contain coverage.manifest.json"
    );
}

#[test]
fn import_with_custom_user_and_window() {
    let collect_tmp = TempDir::new().unwrap();
    let import_tmp = TempDir::new().unwrap();

    let run_dir = collect_json_into(collect_tmp.path());

    let out = shiplog_bin()
        .args([
            "import",
            "--dir",
            run_dir.to_str().unwrap(),
            "--out",
            import_tmp.path().to_str().unwrap(),
            "--user",
            "importbot",
            "--window-label",
            "H1-2025",
        ])
        .output()
        .expect("failed to run shiplog import");

    assert!(
        out.status.success(),
        "import with custom user failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let import_run_dir = import_tmp.path().join("run_fixture");
    let packet = std::fs::read_to_string(import_run_dir.join("packet.md")).unwrap();
    assert!(
        packet.contains("H1-2025"),
        "imported packet should contain custom window label"
    );
}

// ── 7. output directory creation ───────────────────────────────────────────

#[test]
fn collect_creates_nonexistent_out_dir() {
    let tmp = TempDir::new().unwrap();
    let fixtures = fixture_dir();

    // Use a deeply nested --out path that doesn't exist yet
    let nested_out = tmp.path().join("deep").join("nested").join("output");
    assert!(!nested_out.exists());

    let out = shiplog_bin()
        .args([
            "collect",
            "--out",
            nested_out.to_str().unwrap(),
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
        "collect json with nested --out failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // The nested directory should now exist with outputs inside
    let run_dir = nested_out.join("run_fixture");
    assert!(
        run_dir.join("packet.md").exists(),
        "packet.md should exist in newly created nested output dir"
    );
}

#[test]
fn import_creates_nonexistent_out_dir() {
    let collect_tmp = TempDir::new().unwrap();
    let import_tmp = TempDir::new().unwrap();

    let run_dir = collect_json_into(collect_tmp.path());

    let nested_out = import_tmp.path().join("a").join("b").join("c");
    assert!(!nested_out.exists());

    let out = shiplog_bin()
        .args([
            "import",
            "--dir",
            run_dir.to_str().unwrap(),
            "--out",
            nested_out.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run shiplog import");

    assert!(
        out.status.success(),
        "import with nested --out failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let import_run_dir = nested_out.join("run_fixture");
    assert!(
        import_run_dir.join("packet.md").exists(),
        "packet.md should exist in newly created nested output dir"
    );
}
