//! Structural receipt validation for release first-use acceptance (issue #245).
//!
//! `shiplog report validate --receipts` must structurally validate a run's
//! durable receipts — `packet.md`, `ledger.events.jsonl`,
//! `coverage.manifest.json`, `bundle.manifest.json` — not merely check that
//! they exist. Release first-use acceptance drives the published binary against
//! an empty directory and then runs this command, so a broken receipt fails
//! acceptance instead of shipping.
//!
//! These tests drive the same cargo-built binary a release download would
//! provide (via `CARGO_BIN_EXE_shiplog`) and express each user-facing behavior
//! through the repository's Given/When/Then testkit.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use shiplog_testkit::bdd::assertions::{assert_contains, assert_false, assert_true};
use shiplog_testkit::bdd::{Scenario, ScenarioContext};

fn shiplog_cmd(cwd: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_shiplog"));
    cmd.current_dir(cwd)
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .env_remove("SHIPLOG_REDACT_KEY");
    cmd
}

fn given_a_clean_workspace(ctx: &mut ScenarioContext) {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let workspace =
        std::env::temp_dir().join(format!("shiplog-receipts-{}-{nonce}", std::process::id()));
    ctx.paths.insert("workspace".into(), workspace);
}

fn workspace(ctx: &ScenarioContext) -> Result<&Path, String> {
    ctx.path("workspace")
        .ok_or_else(|| "scenario workspace was not configured".to_string())
}

fn run_intake(ctx: &ScenarioContext) -> Result<PathBuf, String> {
    let workspace = workspace(ctx)?;
    let output_dir = workspace.join("out");
    fs::create_dir_all(workspace).map_err(|error| format!("create workspace: {error}"))?;
    let output = shiplog_cmd(workspace)
        .args([
            "intake",
            "--last-6-months",
            "--out",
            output_dir
                .to_str()
                .ok_or_else(|| "workspace path is not valid UTF-8".to_string())?,
            "--no-open",
        ])
        .output()
        .map_err(|error| format!("run intake: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "intake failed with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    fs::read_dir(&output_dir)
        .map_err(|error| format!("read intake output: {error}"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.is_dir() && path.join("intake.report.json").exists())
        .ok_or_else(|| "intake did not produce a run directory".to_string())
}

fn validate(run: &Path, receipts: bool) -> Result<Output, String> {
    let report = run.join("intake.report.json");
    let mut command = shiplog_cmd(run);
    command.args(["report", "validate", "--path"]).arg(
        report
            .to_str()
            .ok_or_else(|| "report path is not valid UTF-8".to_string())?,
    );
    if receipts {
        command.arg("--receipts");
    }
    command
        .output()
        .map_err(|error| format!("run receipt validation: {error}"))
}

fn save_output(ctx: &mut ScenarioContext, key: &str, output: Output) {
    ctx.flags
        .insert(format!("{key}.success"), output.status.success());
    ctx.data.insert(format!("{key}.stdout"), output.stdout);
    ctx.data.insert(format!("{key}.stderr"), output.stderr);
}

fn output_text(ctx: &ScenarioContext, key: &str, stream: &str) -> Result<String, String> {
    let bytes = ctx
        .data
        .get(&format!("{key}.{stream}"))
        .ok_or_else(|| format!("scenario output {key}.{stream} was not recorded"))?;
    Ok(String::from_utf8_lossy(bytes).into_owned())
}

fn run_receipt_validation(
    ctx: &mut ScenarioContext,
    corrupt: fn(&Path) -> Result<(), String>,
) -> Result<(), String> {
    let run = run_intake(ctx)?;
    corrupt(&run)?;
    save_output(ctx, "receipts", validate(&run, true)?);
    Ok(())
}

fn corrupt_coverage(run: &Path) -> Result<(), String> {
    fs::write(run.join("coverage.manifest.json"), "not valid json{")
        .map_err(|error| format!("corrupt coverage manifest: {error}"))
}

fn corrupt_bundle(run: &Path) -> Result<(), String> {
    fs::write(run.join("bundle.manifest.json"), "{}")
        .map_err(|error| format!("corrupt bundle manifest: {error}"))
}

fn corrupt_ledger(run: &Path) -> Result<(), String> {
    let ledger = run.join("ledger.events.jsonl");
    let pristine = fs::read_to_string(&ledger)
        .map_err(|error| format!("read ledger before corruption: {error}"))?;
    fs::write(
        ledger,
        format!("{pristine}\nthis line is not a json event\n"),
    )
    .map_err(|error| format!("corrupt ledger: {error}"))
}

fn corrupt_packet(run: &Path) -> Result<(), String> {
    fs::write(
        run.join("packet.md"),
        "# Something Else\n\nno required sections here\n",
    )
    .map_err(|error| format!("corrupt packet: {error}"))
}

fn then_receipts_succeed(ctx: &ScenarioContext) -> Result<(), String> {
    assert_true(
        ctx.flag("receipts.success").unwrap_or(false),
        "receipt validation success",
    )?;
    let stdout = output_text(ctx, "receipts", "stdout")?;
    assert_contains(
        &stdout,
        "Receipts: 4 structurally validated",
        "receipt validation summary",
    )
}

fn then_receipts_reject_coverage(ctx: &ScenarioContext) -> Result<(), String> {
    then_receipts_reject(ctx, "coverage.manifest.json", "malformed")
}

fn then_receipts_reject_bundle(ctx: &ScenarioContext) -> Result<(), String> {
    then_receipts_reject(ctx, "bundle.manifest.json", "malformed")
}

fn then_receipts_reject_ledger(ctx: &ScenarioContext) -> Result<(), String> {
    then_receipts_reject(
        ctx,
        "ledger.events.jsonl",
        "is not a well-formed event record",
    )
}

fn then_receipts_reject_packet(ctx: &ScenarioContext) -> Result<(), String> {
    then_receipts_reject(ctx, "packet.md", "missing required section")
}

fn then_receipts_reject(ctx: &ScenarioContext, artifact: &str, reason: &str) -> Result<(), String> {
    assert_false(
        ctx.flag("receipts.success").unwrap_or(true),
        "receipt validation success",
    )?;
    let stderr = output_text(ctx, "receipts", "stderr")?;
    assert_contains(&stderr, artifact, "offending receipt artifact")?;
    assert_contains(&stderr, reason, "receipt validation reason")
}

fn when_receipts_accept_a_healthy_run(ctx: &mut ScenarioContext) -> Result<(), String> {
    let run = run_intake(ctx)?;
    save_output(ctx, "receipts", validate(&run, true)?);
    Ok(())
}

fn when_receipts_reject_malformed_coverage(ctx: &mut ScenarioContext) -> Result<(), String> {
    run_receipt_validation(ctx, corrupt_coverage)
}

fn when_receipts_reject_malformed_bundle(ctx: &mut ScenarioContext) -> Result<(), String> {
    run_receipt_validation(ctx, corrupt_bundle)
}

fn when_receipts_reject_malformed_ledger(ctx: &mut ScenarioContext) -> Result<(), String> {
    run_receipt_validation(ctx, corrupt_ledger)
}

fn when_receipts_reject_malformed_packet(ctx: &mut ScenarioContext) -> Result<(), String> {
    run_receipt_validation(ctx, corrupt_packet)
}

fn when_receipts_flag_is_opt_in(ctx: &mut ScenarioContext) -> Result<(), String> {
    let run = run_intake(ctx)?;
    corrupt_coverage(&run)?;
    save_output(ctx, "default", validate(&run, false)?);
    save_output(ctx, "receipts", validate(&run, true)?);
    Ok(())
}

fn then_receipts_flag_is_opt_in(ctx: &ScenarioContext) -> Result<(), String> {
    assert_true(
        ctx.flag("default.success").unwrap_or(false),
        "default validation success",
    )?;
    assert_false(
        ctx.flag("receipts.success").unwrap_or(true),
        "receipt validation success",
    )
}

#[test]
fn receipts_validation_accepts_a_healthy_cold_start_run() -> Result<(), String> {
    Scenario::new("A healthy cold-start run has structurally valid receipts")
        .given("a clean workspace", given_a_clean_workspace)
        .when(
            "the published binary validates all receipts",
            when_receipts_accept_a_healthy_run,
        )
        .then(
            "receipt validation succeeds with a summary",
            then_receipts_succeed,
        )
        .run()
}

#[test]
fn receipts_validation_rejects_malformed_coverage_manifest() -> Result<(), String> {
    Scenario::new("Malformed coverage manifests fail receipt validation")
        .given("a clean workspace", given_a_clean_workspace)
        .when(
            "the coverage manifest is corrupted before validation",
            when_receipts_reject_malformed_coverage,
        )
        .then(
            "validation names the coverage receipt and reason",
            then_receipts_reject_coverage,
        )
        .run()
}

#[test]
fn receipts_validation_rejects_malformed_bundle_manifest() -> Result<(), String> {
    Scenario::new("Malformed bundle manifests fail receipt validation")
        .given("a clean workspace", given_a_clean_workspace)
        .when(
            "the bundle manifest is corrupted before validation",
            when_receipts_reject_malformed_bundle,
        )
        .then(
            "validation names the bundle receipt and reason",
            then_receipts_reject_bundle,
        )
        .run()
}

#[test]
fn receipts_validation_rejects_malformed_ledger_line() -> Result<(), String> {
    Scenario::new("Malformed ledger lines fail receipt validation")
        .given("a clean workspace", given_a_clean_workspace)
        .when(
            "the ledger is corrupted before validation",
            when_receipts_reject_malformed_ledger,
        )
        .then(
            "validation names the ledger receipt and reason",
            then_receipts_reject_ledger,
        )
        .run()
}

#[test]
fn receipts_validation_rejects_packet_missing_required_sections() -> Result<(), String> {
    Scenario::new("Packets missing required sections fail receipt validation")
        .given("a clean workspace", given_a_clean_workspace)
        .when(
            "the packet is corrupted before validation",
            when_receipts_reject_malformed_packet,
        )
        .then(
            "validation names the packet receipt and reason",
            then_receipts_reject_packet,
        )
        .run()
}

#[test]
fn receipts_flag_is_opt_in_and_does_not_change_default_validate() -> Result<(), String> {
    Scenario::new("Receipt validation remains opt-in")
        .given("a clean workspace", given_a_clean_workspace)
        .when(
            "the same corrupt run is validated with and without the flag",
            when_receipts_flag_is_opt_in,
        )
        .then(
            "default validation passes while receipt validation fails",
            then_receipts_flag_is_opt_in,
        )
        .run()
}
