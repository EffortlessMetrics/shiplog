//! Cold-start contract tests for `shiplog intake`.
//!
//! These tests pin the user-facing contract documented in
//! [`docs/product/rapid-first-intake.md`](../../../docs/product/rapid-first-intake.md).
//! Each `#[test]` names the section anchor it pins, so a regression on the
//! product contract is visible in the test report without cross-referencing
//! commits.
//!
//! These tests are intentionally narrow:
//!
//! - They assert the cold-start *contract* — what the first-time user is
//!   promised — not the full intake behavior. Broader coverage of intake
//!   behavior (config-driven runs, multi-source merging, share commands,
//!   rerun semantics, schema-contract verification) already lives in
//!   `cli_integration.rs`; this file does not duplicate it.
//! - Tests in the "happy-path scaffold" group drive `shiplog intake
//!   --last-6-months --no-open` from an empty temp directory with every
//!   provider token cleared. Tests in the "edge defaults" group probe the
//!   remaining contract clauses (default time window without the explicit
//!   flag, source-selection visibility, the truly-zero-sources-succeeded
//!   exit case).
//!
//! See the ladder in `docs/product/rapid-first-intake.md` § 5 for the
//! full PR sequence.

use assert_cmd::Command;
use chrono::{Duration, NaiveDate, Utc};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Standard cold-start invocation. Matches the documented happy path in
/// `docs/product/rapid-first-intake.md` § 2.
fn cold_start_cmd(tmp: &Path, out: &Path) -> Command {
    let mut cmd = Command::from_std(std::process::Command::new(env!("CARGO_BIN_EXE_shiplog")));
    cmd.current_dir(tmp)
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args([
            "intake",
            "--last-6-months",
            "--out",
            out.to_str().unwrap(),
            "--no-open",
        ]);
    cmd
}

fn first_run_dir(out_root: &Path) -> PathBuf {
    let mut entries: Vec<_> = fs::read_dir(out_root)
        .expect("read out root")
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .collect();
    entries.sort_by_key(|entry| entry.file_name());
    entries
        .into_iter()
        .next()
        .expect("cold-start intake should leave at least one run directory under --out")
        .path()
}

/// § 2 — one-command happy path. From an empty directory with no provider
/// tokens, `shiplog intake --last-6-months` must produce the full set of
/// review-pack artifacts the doc promises a first-time user: `packet.md`,
/// `intake.report.md`, `intake.report.json`, `coverage.manifest.json`.
#[test]
fn cold_start_one_command_emits_required_artifacts() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    cold_start_cmd(tmp.path(), &out).assert().success();

    let run = first_run_dir(&out);
    for artifact in [
        "packet.md",
        "intake.report.md",
        "intake.report.json",
        "coverage.manifest.json",
    ] {
        assert!(
            run.join(artifact).exists(),
            "rapid-first-intake.md § 2: cold-start must emit {artifact} (missing from {})",
            run.display()
        );
    }
}

/// § 2 / § 3 — terminal next-step footer. A first-time user should not
/// have to inspect the filesystem to know where the review pack went or
/// what command to run next. The stdout footer must point at the run
/// directory, latest-open commands, and the missing-evidence repair loop.
#[test]
fn cold_start_stdout_prints_review_pack_next_steps() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    let assert = cold_start_cmd(tmp.path(), &out).assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

    let run = first_run_dir(&out);
    assert!(
        stdout.contains("Review pack written to:"),
        "rapid-first-intake.md § 2: intake stdout must name where the review pack was written. stdout:\n{stdout}"
    );
    assert!(
        stdout.contains(&run.display().to_string()),
        "rapid-first-intake.md § 2: intake stdout must include the concrete run directory {}. stdout:\n{stdout}",
        run.display()
    );
    assert!(
        stdout.contains("shiplog open intake-report") && stdout.contains("--latest"),
        "rapid-first-intake.md § 2: intake stdout must include the latest intake-report open command. stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("shiplog open packet") && stdout.contains("--latest"),
        "rapid-first-intake.md § 2: intake stdout must include the latest packet open command. stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("Needs evidence:")
            && stdout.contains("shiplog journal add")
            && stdout.contains("shiplog intake --last-6-months --explain"),
        "rapid-first-intake.md § 3: Needs evidence stdout must include the repair loop. stdout:\n{stdout}"
    );
}

/// § 3 — config scaffolding default. From a literal empty directory, the
/// cold-start command must create the starter `shiplog.toml` and
/// `manual_events.yaml` so the user does not need to run `shiplog init`
/// first. The doc promises "Creates a starter `shiplog.toml` and
/// `manual_events.yaml` if missing — Lets `intake` complete from a
/// literal empty directory."
#[test]
fn cold_start_scaffolds_starter_config_files() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    cold_start_cmd(tmp.path(), &out).assert().success();

    assert!(
        tmp.path().join("shiplog.toml").exists(),
        "rapid-first-intake.md § 3 config scaffolding default: shiplog.toml must be created from an empty directory"
    );
    assert!(
        tmp.path().join("manual_events.yaml").exists(),
        "rapid-first-intake.md § 3 config scaffolding default: manual_events.yaml must be created from an empty directory"
    );
}

/// § 3 — exit-status semantics + readiness framing. With every provider
/// token cleared and no prior `manual_events.yaml`, the manual source is
/// the only source the cold-start can succeed on, and it succeeds with
/// zero events (because the scaffolded `manual_events.yaml` is empty).
/// The doc's exit-status contract says: "Non-zero only when zero sources
/// succeeded." So one-source-succeeded-with-zero-events is a success
/// exit — and the readiness summary in `intake.report.json` must frame
/// the run honestly as needing evidence rather than ready for review.
#[test]
fn cold_start_succeeds_with_needs_evidence_readiness_when_no_events() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    cold_start_cmd(tmp.path(), &out).assert().success();

    let run = first_run_dir(&out);
    let report_json_text = fs::read_to_string(run.join("intake.report.json"))
        .expect("intake.report.json must exist after a successful cold-start run");
    let report: serde_json::Value = serde_json::from_str(&report_json_text)
        .expect("intake.report.json must be well-formed JSON");

    assert_eq!(
        report["readiness"], "Needs evidence",
        "rapid-first-intake.md § 3 exit-status + readiness contract: a cold-start with zero collected events must report readiness=Needs evidence in intake.report.json (got {})",
        report["readiness"]
    );

    let needs_attention = report["needs_attention"]
        .as_array()
        .expect("intake.report.json must expose a needs_attention array");
    assert!(
        needs_attention.iter().any(|item| item
            .as_str()
            .is_some_and(|text| text.contains("No events collected"))),
        "rapid-first-intake.md § 3 readiness contract: the readiness summary must surface the missing-evidence gap so the reviewer sees it before forming an opinion (needs_attention={needs_attention:?})"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Edge defaults
// ─────────────────────────────────────────────────────────────────────────
//
// PR 2 of the rapid-first-intake ladder probes the contract clauses the
// PR 1 tests above did not cover: the time-window default when the user
// does not pass `--last-6-months`, the source-decision visibility in the
// intake report without `--explain`, and the exit-status branch where
// literally zero sources succeed.

/// § 3 — time-window default. The doc promises a six-month window ending
/// today when the user does not pass an explicit window flag. The window
/// recorded in `intake.report.json` after `shiplog intake --no-open` from
/// an empty directory must therefore land within a small tolerance of
/// "today minus six months" through "today".
#[test]
fn cold_start_default_window_resolves_to_six_months_when_flag_omitted() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    Command::from_std(std::process::Command::new(env!("CARGO_BIN_EXE_shiplog")))
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["intake", "--out", out.to_str().unwrap(), "--no-open"])
        .assert()
        .success();

    let run = first_run_dir(&out);
    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(run.join("intake.report.json")).unwrap()).unwrap();

    let since = NaiveDate::parse_from_str(report["window"]["since"].as_str().unwrap(), "%Y-%m-%d")
        .expect("window.since must be ISO YYYY-MM-DD");
    let until = NaiveDate::parse_from_str(report["window"]["until"].as_str().unwrap(), "%Y-%m-%d")
        .expect("window.until must be ISO YYYY-MM-DD");

    let today = Utc::now().date_naive();
    let span_days = (until - since).num_days();

    // Six months is 180 ± 4 calendar days depending on which months the
    // window crosses; the doc's contract is "six months", not "182 days
    // exactly", so allow a small calendar-bound tolerance.
    assert!(
        (176..=186).contains(&span_days),
        "rapid-first-intake.md § 3 time-window default: cold-start window without --last-6-months should span ~6 months, got {span_days} days (since={since}, until={until})"
    );

    // `until` should land at or very near today; allow ±2 days for
    // timezone / midnight rollover.
    let until_offset = (today - until).num_days().abs();
    assert!(
        until_offset <= 2,
        "rapid-first-intake.md § 3 time-window default: window.until should be today (±2 days), got {until} vs today {today} (offset={until_offset})"
    );

    // `since` should be roughly today minus six months. Use 180 ± 4 days.
    let expected_since = today - Duration::days(180);
    let since_offset = (expected_since - since).num_days().abs();
    assert!(
        since_offset <= 6,
        "rapid-first-intake.md § 3 time-window default: window.since should be ~6 months before today, got {since} vs expected ~{expected_since} (offset={since_offset})"
    );
}

/// § 3 — source-selection visibility. The intake report must record a
/// `source_decisions` entry per source even when `--explain` is not
/// passed: a reviewer reading `intake.report.json` should see which
/// sources were considered, which were skipped, and the reason for each
/// skip, without having to rerun with a different flag.
#[test]
fn cold_start_intake_report_records_source_decisions_without_explain() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    cold_start_cmd(tmp.path(), &out).assert().success();

    let run = first_run_dir(&out);
    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(run.join("intake.report.json")).unwrap()).unwrap();

    let decisions = report["source_decisions"]
        .as_array()
        .expect("rapid-first-intake.md § 3: intake.report.json must expose source_decisions");

    assert!(
        !decisions.is_empty(),
        "rapid-first-intake.md § 3 source-selection visibility: source_decisions must be populated even without --explain (got empty array)"
    );

    // The auto-scaffolded `manual_events.yaml` must show up as an
    // `included` decision; the absent provider tokens must show up as
    // `skipped` decisions with a reason. The reviewer reading the report
    // alone must be able to answer "what did shiplog do, and why?"
    let manual_included = decisions.iter().any(|entry| {
        entry["source"]
            .as_str()
            .is_some_and(|s| s.eq_ignore_ascii_case("manual"))
            && entry["decision"].as_str() == Some("included")
    });
    assert!(
        manual_included,
        "rapid-first-intake.md § 3 source-selection visibility: the scaffolded manual source must appear as decision=\"included\" in source_decisions (decisions={decisions:?})"
    );

    let any_skipped_with_reason = decisions.iter().any(|entry| {
        entry["decision"].as_str() == Some("skipped")
            && entry["reason"]
                .as_str()
                .is_some_and(|reason| !reason.is_empty())
    });
    assert!(
        any_skipped_with_reason,
        "rapid-first-intake.md § 3 source-selection visibility: with all provider tokens cleared, at least one source must be reported as decision=\"skipped\" with a non-empty reason — not silently ignored (decisions={decisions:?})"
    );
}

/// § Cache freshness receipt. The intake report must record per-source
/// freshness so a first-time reviewer can answer "Is this fresh?" without
/// reading the cache directly. On a cold-start run with an empty cache,
/// the scaffolded manual source must report status="fresh" — there is
/// nothing in the cache to mark cached, and the manual ingest path reads
/// the YAML file from disk every run.
#[test]
fn cold_start_intake_report_records_source_freshness_per_source() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    cold_start_cmd(tmp.path(), &out).assert().success();

    let run = first_run_dir(&out);
    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(run.join("intake.report.json")).unwrap()).unwrap();

    let freshness = report["source_freshness"]
        .as_array()
        .expect("intake.report.json must expose a source_freshness array");
    assert!(
        !freshness.is_empty(),
        "freshness receipt: source_freshness must be populated on cold-start runs (got empty array)"
    );

    let manual_fresh = freshness.iter().any(|entry| {
        entry["source"].as_str() == Some("manual") && entry["status"].as_str() == Some("fresh")
    });
    assert!(
        manual_fresh,
        "freshness receipt: cold-start manual source must report status=\"fresh\" (freshness={freshness:?})"
    );

    // No source should be `cached` on cold-start: the cache is empty so
    // every adapter that uses it either misses (=> Fresh) or doesn't use
    // it at all. A `cached` status here would mean we lied about the
    // first-run state.
    let cached_on_cold_start = freshness
        .iter()
        .any(|entry| entry["status"].as_str() == Some("cached"));
    assert!(
        !cached_on_cold_start,
        "freshness receipt: no source should report status=\"cached\" on a cold-start run (freshness={freshness:?})"
    );

    // The rendered markdown must include the freshness block as well.
    let report_md = fs::read_to_string(run.join("intake.report.md"))
        .expect("intake.report.md must exist after a successful cold-start run");
    assert!(
        report_md.contains("## Source Freshness"),
        "freshness receipt: intake.report.md must include a `## Source Freshness` section (missing)"
    );
}

/// § 3 — exit-status: the truly-zero-sources branch. The PR 1 test
/// `cold_start_succeeds_with_needs_evidence_readiness_when_no_events`
/// pins the success-exit branch (manual source succeeded with zero
/// events). This test pins the complementary clause from the doc's
/// contract — "Non-zero only when zero sources succeeded" — by writing
/// a `shiplog.toml` that enables a single remote source (GitHub) with no
/// token available and disables manual, so every source either skips or
/// fails. The intake command must exit non-zero in that case.
///
/// This is not strictly a "cold-start" run (the user has a pre-existing
/// `shiplog.toml`), but it pins the exit-status half of the cold-start
/// contract that PR 1 could not engineer from a literal empty directory.
#[test]
fn intake_exits_non_zero_when_zero_sources_succeeded() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[defaults]
window = "year:2025"

[sources.github]
enabled = true
user = "octocat"

[sources.manual]
enabled = false
"#,
    )
    .unwrap();

    Command::from_std(std::process::Command::new(env!("CARGO_BIN_EXE_shiplog")))
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args([
            "intake",
            "--out",
            out.to_str().unwrap(),
            "--no-open",
            "--year",
            "2025",
        ])
        .assert()
        .failure();
}
