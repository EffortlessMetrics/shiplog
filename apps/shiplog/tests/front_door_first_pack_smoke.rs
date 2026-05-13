//! Front-door first-pack smoke test (a.k.a. install-to-first-pack).
//!
//! Proves the "front door" works: a user who installed `shiplog` (via
//! `cargo install` or the GitHub release binary) can run one command
//! from an empty directory and get a defensible review pack. The
//! test exercises the same binary `cargo install` would produce — via
//! `CARGO_BIN_EXE_shiplog` — and asserts the four user-promised
//! framing elements appear in the rendered `intake.report.md`.
//!
//! The test file is named `front_door_first_pack_smoke` rather than
//! `install_*` because Windows's UAC heuristic treats executables
//! whose name starts with `install*` as installers and demands
//! elevation, breaking the test binary launch under `cargo test` on
//! Windows runners.
//!
//! This is **not** a contract test on the intake report's full shape
//! — that's [`intake_cold_start.rs`]. This is a single-pass smoke:
//! if any of the four framing elements drift, the front-door promise
//! that
//! [`docs/guides/rapid-first-intake.md`](../../../docs/guides/rapid-first-intake.md)
//! makes to a first-time user is broken, and this test catches it
//! before a release.
//!
//! The four elements pinned here:
//!
//! 1. **Redaction status** — every run states which profile rendered
//!    it, so a reviewer can answer "is this safe to share?" without
//!    reading the share command's output.
//! 2. **Where to Look** — pointers to the durable artifacts
//!    (event ledger, coverage manifest, source freshness, full
//!    artifact list) so the reviewer can find evidence.
//! 3. **Source Freshness** — per-source state (`fresh` / `cached` /
//!    `skipped` / `unavailable`) so the reviewer can answer "is this
//!    pack fresh?" without reading the cache directly. Landed in #218
//!    and pinned at the adapter layer in #219.
//! 4. **Needs-evidence framing** — when the cold-start run produced
//!    zero events, the report leads with the readiness state plus a
//!    concrete `Needs Attention` bullet pointing the user at adding
//!    manual evidence or enabling a source. The honest framing is
//!    what makes the first pack defensible.

use assert_cmd::Command;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Locate the run directory created under `--out`. Cold-start runs
/// produce a sortable-timestamp + short-hash subdirectory; this test
/// only cares about the first (and typically only) one.
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
        .expect("install-to-first-pack smoke: at least one run directory must exist under --out")
        .path()
}

/// The "does the front door work?" receipt. Drive the cargo-built
/// `shiplog` binary (the same one `cargo install shiplog` produces)
/// against an empty directory with every provider token cleared, then
/// assert that `intake.report.md` carries the four framing elements
/// the user-facing guide promises.
#[test]
fn install_to_first_pack_smoke() {
    let tmp = TempDir::new().expect("tempdir for smoke run");
    let out = tmp.path().join("out");

    // Drive the binary at the install path the user would have after
    // `cargo install shiplog` or a release download. `CARGO_BIN_EXE_shiplog`
    // points at the cargo-built artifact for this workspace, which is
    // byte-identical to a `cargo install` output for the same toolchain.
    let assert = Command::from_std(std::process::Command::new(env!("CARGO_BIN_EXE_shiplog")))
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args([
            "intake",
            "--last-6-months",
            "--out",
            out.to_str().expect("--out path must be utf-8"),
            "--no-open",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

    let run = first_run_dir(&out);
    assert!(
        stdout.contains("Review pack written to:"),
        "smoke: intake stdout must include the review-pack footer. stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("shiplog open intake-report") && stdout.contains("--latest"),
        "smoke: intake stdout must point at the latest intake report. stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("shiplog open packet") && stdout.contains("--latest"),
        "smoke: intake stdout must point at the latest packet. stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("Needs evidence:") && stdout.contains("shiplog journal add"),
        "smoke: Needs evidence stdout must include the manual-evidence repair hint. stdout:\n{stdout}"
    );

    // (a) The two artifacts the user opens first must exist. Without
    // them, no framing assertion below is meaningful.
    let packet = run.join("packet.md");
    let report_md = run.join("intake.report.md");
    assert!(
        packet.exists(),
        "smoke: front-door run must produce packet.md (missing at {})",
        packet.display()
    );
    assert!(
        report_md.exists(),
        "smoke: front-door run must produce intake.report.md (missing at {})",
        report_md.display()
    );

    let body = fs::read_to_string(&report_md).expect("read intake.report.md");

    // (b) Redaction status. Internal profile on a first-run; the
    // exact phrasing is part of the contract because reviewers scan
    // for "internal" / "manager" / "public" to decide whether the
    // pack is shareable.
    assert!(
        body.contains("Redaction profile: `internal`"),
        "smoke: intake.report.md must state the redaction profile inline (looked for `Redaction profile: \\`internal\\``). body:\n{body}"
    );

    // (c) Where to Look. The section header anchors the artifact map
    // the user follows to find evidence.
    assert!(
        body.contains("\n## Where to Look\n"),
        "smoke: intake.report.md must include a `## Where to Look` section heading. body:\n{body}"
    );

    // (d) Source Freshness. The section header anchors the per-source
    // freshness rollup that PR #218 introduced and #219 pinned. If
    // this header drifts, the rapid-first-intake guide's "## Source
    // Freshness" reference is broken.
    assert!(
        body.contains("\n## Source Freshness\n"),
        "smoke: intake.report.md must include a `## Source Freshness` section heading. body:\n{body}"
    );

    // (e) Needs-evidence framing. With every provider token cleared
    // and only the auto-scaffolded empty `manual_events.yaml`, the
    // honest readiness state is `Needs evidence`, and the
    // `## Needs Attention` section must call out the missing-evidence
    // gap concretely (so a reviewer sees the gap before forming an
    // opinion of the pack).
    assert!(
        body.contains("Packet readiness: **Needs evidence**"),
        "smoke: intake.report.md must lead with `Packet readiness: **Needs evidence**` on a cold-start with no tokens. body:\n{body}"
    );
    assert!(
        body.contains("No events collected"),
        "smoke: intake.report.md must surface the missing-evidence gap in plain language (looked for `No events collected`). body:\n{body}"
    );
}
