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
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const MANUAL_NO_EVENTS_REPAIR_KEY: &str = "manual:manual_evidence_missing:no_events";

fn shiplog_cmd(tmp: &Path) -> Command {
    let mut cmd = Command::from_std(std::process::Command::new(env!("CARGO_BIN_EXE_shiplog")));
    cmd.current_dir(tmp)
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY");
    cmd
}

/// Locate the run directory created under `--out`. Cold-start runs
/// produce a sortable-timestamp + short-hash subdirectory; this test
/// only cares about the first (and typically only) one.
fn sorted_run_dirs(out_root: &Path) -> Vec<PathBuf> {
    let mut entries: Vec<_> = fs::read_dir(out_root)
        .expect("read out root")
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|entry| entry.path())
        .collect();
    entries.sort();
    entries
}

fn first_run_dir(out_root: &Path) -> PathBuf {
    sorted_run_dirs(out_root)
        .into_iter()
        .next()
        .expect("install-to-first-pack smoke: at least one run directory must exist under --out")
}

fn latest_run_dir(out_root: &Path) -> PathBuf {
    sorted_run_dirs(out_root)
        .into_iter()
        .next_back()
        .expect("install-to-first-pack smoke: at least one run directory must exist under --out")
}

fn read_report_json(run: &Path) -> Value {
    let report_path = run.join("intake.report.json");
    serde_json::from_str(
        &fs::read_to_string(&report_path).expect("read intake.report.json for product proof"),
    )
    .expect("intake.report.json must be valid JSON for product proof")
}

fn repair_item_by_key<'a>(report: &'a Value, repair_key: &str) -> Option<&'a Value> {
    report["repair_items"]
        .as_array()
        .expect("intake.report.json must expose repair_items")
        .iter()
        .find(|item| item["repair_key"].as_str() == Some(repair_key))
}

fn source_event_count(report: &Value, source_key: &str) -> u64 {
    report["included_sources"]
        .as_array()
        .expect("intake.report.json must expose included_sources")
        .iter()
        .find(|source| source["source_key"].as_str() == Some(source_key))
        .and_then(|source| source["event_count"].as_u64())
        .unwrap_or(0)
}

fn packet_quality_status(report: &Value) -> Option<&str> {
    report["packet_quality"]["packet_readiness"]["status"].as_str()
}

fn evidence_strength_status<'a>(report: &'a Value, scope: &str) -> Option<&'a str> {
    report["packet_quality"]["evidence_strength"]
        .as_array()?
        .iter()
        .find(|item| item["scope"].as_str() == Some(scope))
        .and_then(|item| item["status"].as_str())
}

fn slice_between<'a>(text: &'a str, start: &str, end: &str) -> &'a str {
    let start_index = text
        .find(start)
        .unwrap_or_else(|| panic!("expected section start {start:?} in:\n{text}"));
    let rest = &text[start_index..];
    let end_index = rest
        .find(end)
        .unwrap_or_else(|| panic!("expected section end {end:?} after {start:?} in:\n{text}"));
    &rest[..end_index]
}

fn review_ready_packet_surface(packet: &str, out_arg: &str, run_id: &str) -> String {
    let readiness = slice_between(packet, "# Packet Readiness", "\n# Claim Candidates");
    let claims = slice_between(packet, "# Claim Candidates", "\n## Coverage and Limits");
    format!("{}\n\n{}\n", readiness.trim_end(), claims.trim_end())
        .replace(out_arg, "<OUT>")
        .replace(run_id, "<RUN_ID>")
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
    let assert = shiplog_cmd(tmp.path())
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
    assert!(
        stdout.contains("shiplog repair plan --out") && stdout.contains("--latest"),
        "smoke: intake stdout must route repairable packets through repair plan first. stdout:\n{stdout}"
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
    let packet_body = fs::read_to_string(&packet).expect("read packet.md");

    assert!(
        packet_body.starts_with("# Packet Readiness\n\n"),
        "smoke: packet.md must lead with the Packet Readiness section. packet:\n{packet_body}"
    );
    assert!(
        packet_body.contains("Needs evidence."),
        "smoke: Packet Readiness section must summarize cold-start readiness. packet:\n{packet_body}"
    );
    assert!(
        packet_body.contains("Still weak:") && packet_body.contains("needs_context"),
        "smoke: Packet Readiness section must surface weak evidence state. packet:\n{packet_body}"
    );
    assert!(
        !packet_body.contains("# Claim Candidates"),
        "smoke: zero-evidence packet should not render claim candidates. packet:\n{packet_body}"
    );

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
        "smoke: intake.report.md must include `Packet readiness: **Needs evidence**` on a cold-start with no tokens. body:\n{body}"
    );
    assert!(
        body.contains("No events collected"),
        "smoke: intake.report.md must surface the missing-evidence gap in plain language (looked for `No events collected`). body:\n{body}"
    );
    assert!(
        body.contains("shiplog repair plan --out") && body.contains("--latest"),
        "smoke: intake.report.md must route repairable packets through repair plan before direct fixes. body:\n{body}"
    );
}

/// Product proof for the evidence repair loop. A first run with no provider
/// tokens is rough but actionable: the report emits a manual repair item,
/// `journal add --from-repair` records local evidence, rerunning intake clears
/// that repair key, and the next packet contains the new evidence. Every
/// command runs with provider tokens removed; the only write is local journal
/// evidence.
#[test]
fn repair_loop_improves_first_packet_without_provider_mutation() {
    let tmp = TempDir::new().expect("tempdir for repair loop proof");
    let out = tmp.path().join("out");
    let out_arg = out.to_str().expect("--out path must be utf-8");

    shiplog_cmd(tmp.path())
        .args(["intake", "--last-6-months", "--out", out_arg, "--no-open"])
        .assert()
        .success();

    let first_run = first_run_dir(&out);
    let first_report = read_report_json(&first_run);
    assert_eq!(
        first_report["readiness"], "Needs evidence",
        "repair proof: the cold first packet should honestly start as Needs evidence"
    );
    assert_eq!(
        source_event_count(&first_report, "manual"),
        0,
        "repair proof: the cold first packet should have no manual evidence yet"
    );
    let first_next_commands = first_report["next_commands"]
        .as_array()
        .expect("repair proof: report should expose next_commands");
    let first_next = first_next_commands
        .first()
        .and_then(|command| command.as_str())
        .expect("repair proof: report should expose at least one next command");
    assert!(
        first_next.starts_with("shiplog repair plan --out") && first_next.ends_with(" --latest"),
        "repair proof: first next command should send users to repair plan. next_commands={first_next_commands:?}"
    );
    assert_eq!(
        packet_quality_status(&first_report),
        Some("needs_evidence"),
        "repair proof: cold report should expose machine-readable packet readiness"
    );
    assert_eq!(
        evidence_strength_status(&first_report, "packet"),
        Some("needs_context"),
        "repair proof: cold report should classify packet evidence as needing context"
    );
    assert!(
        first_report["packet_quality"]["claim_candidates"]
            .as_array()
            .is_some_and(Vec::is_empty),
        "repair proof: cold zero-evidence report should not emit claim candidates"
    );

    let manual_repair = repair_item_by_key(&first_report, MANUAL_NO_EVENTS_REPAIR_KEY)
        .expect("repair proof: cold report should expose the no-events manual repair item");
    assert_eq!(
        manual_repair["action"]["kind"], "journal_add",
        "repair proof: manual no-events repair must be a local journal action"
    );
    let repair_id = manual_repair["repair_id"]
        .as_str()
        .expect("repair proof: manual repair should have a repair_id")
        .to_string();
    let repair_command = manual_repair["action"]["command"]
        .as_str()
        .expect("repair proof: manual repair should have a command");
    assert!(
        repair_command.contains("--from-repair") && !repair_command.contains("export "),
        "repair proof: manual repair command should be local and report-derived. command={repair_command:?}"
    );

    let plan_assert = shiplog_cmd(tmp.path())
        .args(["repair", "plan", "--out", out_arg, "--latest"])
        .assert()
        .success();
    let plan_stdout = String::from_utf8_lossy(&plan_assert.get_output().stdout);
    assert!(
        plan_stdout.contains(&repair_id)
            && plan_stdout.contains("Command: shiplog journal add --from-repair"),
        "repair proof: repair plan should print the copyable journal repair command. stdout:\n{plan_stdout}"
    );

    let journal_assert = shiplog_cmd(tmp.path())
        .args([
            "journal",
            "add",
            "--from-repair",
            repair_id.as_str(),
            "--out",
            out_arg,
            "--latest",
        ])
        .assert()
        .success();
    let journal_stdout = String::from_utf8_lossy(&journal_assert.get_output().stdout);
    assert!(
        journal_stdout.contains(&format!("Repair: {repair_id}"))
            && journal_stdout.contains("Clears when: manual source contributes"),
        "repair proof: journal repair should echo the report-derived repair context. stdout:\n{journal_stdout}"
    );
    assert!(
        tmp.path().join("manual_events.yaml").exists(),
        "repair proof: journal repair should write local manual evidence only"
    );

    shiplog_cmd(tmp.path())
        .args(["intake", "--last-6-months", "--out", out_arg, "--no-open"])
        .assert()
        .success();

    let repaired_run = latest_run_dir(&out);
    assert_ne!(
        first_run, repaired_run,
        "repair proof: rerun should write a fresh run directory"
    );
    let repaired_report = read_report_json(&repaired_run);
    assert_ne!(
        repaired_report["readiness"], "Needs evidence",
        "repair proof: repaired run should no longer be zero-evidence"
    );
    assert_eq!(
        source_event_count(&repaired_report, "manual"),
        1,
        "repair proof: repaired run should include the journal event as manual evidence"
    );
    assert_eq!(
        packet_quality_status(&repaired_report),
        Some("ready_with_caveats"),
        "repair proof: repaired report should improve packet readiness while manual-only caveats remain"
    );
    assert_eq!(
        evidence_strength_status(&repaired_report, "packet"),
        Some("manual_only"),
        "repair proof: repaired report should classify the packet as manual-only until provider receipts are available"
    );
    let claim_candidates = repaired_report["packet_quality"]["claim_candidates"]
        .as_array()
        .expect("repair proof: repaired report should expose claim_candidates");
    assert!(
        claim_candidates.iter().any(|candidate| {
            candidate["title"]
                .as_str()
                .is_some_and(|title| title.contains("Manual evidence repair"))
                && candidate["evidence_strength"].as_str() == Some("manual_only")
                && candidate["supporting_receipt_refs"]
                    .as_array()
                    .is_some_and(|refs| !refs.is_empty())
                && candidate["missing_context_prompts"]
                    .as_array()
                    .is_some_and(|prompts| !prompts.is_empty())
                && candidate["safe_profiles"]
                    .as_array()
                    .is_some_and(|profiles| {
                        profiles
                            .iter()
                            .any(|profile| profile.as_str() == Some("manager"))
                    })
        }),
        "repair proof: repaired report should emit a manual-only claim candidate with receipts and prompts (claim_candidates={claim_candidates:?})"
    );
    assert!(
        repair_item_by_key(&repaired_report, MANUAL_NO_EVENTS_REPAIR_KEY).is_none(),
        "repair proof: the no-events manual repair key should clear after journal evidence"
    );

    let repaired_packet =
        fs::read_to_string(repaired_run.join("packet.md")).expect("read repaired packet.md");
    assert!(
        repaired_packet.starts_with("# Packet Readiness\n\n")
            && repaired_packet.contains("Ready with caveats.")
            && repaired_packet.contains("manual_only")
            && !repaired_packet.contains("- None yet."),
        "repair proof: repaired packet should render packet readiness and manual-only evidence strength. packet:\n{repaired_packet}"
    );
    assert!(
        repaired_packet.contains("# Claim Candidates")
            && repaired_packet.contains("## Manual evidence repair")
            && repaired_packet.contains("Evidence strength: `manual_only`")
            && repaired_packet.contains("Missing context:")
            && repaired_packet.contains("Which source-backed receipt could confirm this?"),
        "repair proof: repaired packet should render claim candidates and missing-context prompts. packet:\n{repaired_packet}"
    );
    assert!(
        repaired_packet.contains("Manual evidence repair") && repaired_packet.contains(&repair_id),
        "repair proof: repaired packet should contain the journal repair evidence. packet:\n{repaired_packet}"
    );
    insta::assert_snapshot!(
        "review_ready_repaired_packet_surface",
        review_ready_packet_surface(
            &repaired_packet,
            out_arg,
            repaired_run
                .file_name()
                .and_then(|name| name.to_str())
                .expect("repaired run directory should have a utf-8 name")
        )
    );

    let diff_assert = shiplog_cmd(tmp.path())
        .args(["repair", "diff", "--out", out_arg, "--latest"])
        .assert()
        .success();
    let diff_stdout = String::from_utf8_lossy(&diff_assert.get_output().stdout);
    assert!(
        diff_stdout.contains("Cleared:")
            && diff_stdout.contains(MANUAL_NO_EVENTS_REPAIR_KEY)
            && diff_stdout.contains(&repair_id),
        "repair proof: repair diff should show the manual no-events repair clearing. stdout:\n{diff_stdout}"
    );

    let quality_diff_assert = shiplog_cmd(tmp.path())
        .args(["runs", "diff", "--out", out_arg, "--latest"])
        .assert()
        .success();
    let quality_diff_stdout = String::from_utf8_lossy(&quality_diff_assert.get_output().stdout);
    assert!(
        quality_diff_stdout.contains("Packet quality diff:")
            && quality_diff_stdout.contains("manual evidence count 0 -> 1")
            && quality_diff_stdout
                .contains("packet readiness Needs evidence -> Ready with caveats")
            && quality_diff_stdout.contains("claim candidates 0 -> 1")
            && quality_diff_stdout.contains(MANUAL_NO_EVENTS_REPAIR_KEY)
            && quality_diff_stdout.contains("Still weak:")
            && quality_diff_stdout.contains("packet evidence: manual_only")
            && quality_diff_stdout.contains("shiplog share explain manager"),
        "repair proof: runs diff should show packet quality movement after repair. stdout:\n{quality_diff_stdout}"
    );

    let manager_share_assert = shiplog_cmd(tmp.path())
        .env_remove("SHIPLOG_REDACT_KEY")
        .args(["share", "explain", "manager", "--out", out_arg, "--latest"])
        .assert()
        .success();
    let manager_share_stdout = String::from_utf8_lossy(&manager_share_assert.get_output().stdout);
    assert!(
        manager_share_stdout.contains("Manager profile:")
            && manager_share_stdout.contains("Included:")
            && manager_share_stdout.contains("Removed:")
            && manager_share_stdout.contains("Blocked:\n- missing SHIPLOG_REDACT_KEY")
            && manager_share_stdout.contains("shiplog share verify manager"),
        "repair proof: manager share explain should describe blocked share posture without rendering. stdout:\n{manager_share_stdout}"
    );

    let public_share_assert = shiplog_cmd(tmp.path())
        .env_remove("SHIPLOG_REDACT_KEY")
        .args(["share", "explain", "public", "--out", out_arg, "--latest"])
        .assert()
        .success();
    let public_share_stdout = String::from_utf8_lossy(&public_share_assert.get_output().stdout);
    assert!(
        public_share_stdout.contains("Public profile:")
            && public_share_stdout.contains("Included:")
            && public_share_stdout.contains("Removed:")
            && public_share_stdout.contains("Blocked:\n- missing SHIPLOG_REDACT_KEY")
            && public_share_stdout.contains("Public profile uses the strictest redaction profile.")
            && public_share_stdout.contains("shiplog share verify public")
            && public_share_stdout.contains("--strict"),
        "repair proof: public share explain should describe blocked public posture without rendering. stdout:\n{public_share_stdout}"
    );

    assert!(
        !repaired_run.join("profiles/manager/packet.md").exists()
            && !repaired_run.join("profiles/public/packet.md").exists()
            && !repaired_run
                .join("profiles/manager/share.manifest.json")
                .exists()
            && !repaired_run
                .join("profiles/public/share.manifest.json")
                .exists(),
        "repair proof: share explain must not write profile packets or manifests"
    );
}
