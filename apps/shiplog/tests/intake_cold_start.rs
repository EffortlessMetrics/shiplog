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
//! - They drive `shiplog intake --last-6-months --no-open` from an empty
//!   temp directory with every provider token cleared. PR 2 of the ladder
//!   (`feat(intake)`) will add coverage for the default-window case
//!   without the explicit `--last-6-months` flag.
//!
//! See the ladder in `docs/product/rapid-first-intake.md` § 5 for the
//! full PR sequence.

use assert_cmd::Command;
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
