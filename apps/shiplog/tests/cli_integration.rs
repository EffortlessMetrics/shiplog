//! Comprehensive CLI integration tests using `assert_cmd` and `predicates`.

use anyhow::Context;
use assert_cmd::Command;
use chrono::{Duration, NaiveDate, TimeZone, Utc};
use predicates::prelude::*;
use shiplog::cache::ApiCache;
use shiplog::ids::{EventId, RunId, WorkstreamId};
use shiplog::schema::coverage::{Completeness, CoverageManifest, CoverageSlice, TimeWindow};
use shiplog::schema::event::{
    Actor, EventEnvelope, EventKind, EventPayload, Link, ManualDate, ManualEvent, ManualEventType,
    ManualEventsFile, PullRequestEvent, PullRequestState, RepoRef, RepoVisibility, ReviewEvent,
    SourceRef, SourceSystem,
};
use shiplog::schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};
use std::collections::BTreeMap;
use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration as StdDuration, Instant};
use tempfile::TempDir;

type CliTestResult = Result<(), Box<dyn std::error::Error>>;

fn shiplog_cmd() -> Command {
    Command::from_std(std::process::Command::new(env!("CARGO_BIN_EXE_shiplog")))
}

fn fixture_dir() -> PathBuf {
    repo_root().join("examples/fixture")
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn redaction_key_env_command(key_env: &str) -> String {
    if cfg!(windows) {
        format!("$env:{key_env}='replace-with-a-stable-secret'")
    } else {
        format!("export {key_env}=replace-with-a-stable-secret")
    }
}

fn example_config(name: &str) -> PathBuf {
    repo_root().join("examples/configs").join(name)
}

fn seed_github_cache(cache_dir: &Path) -> PathBuf {
    std::fs::create_dir_all(cache_dir).unwrap();
    let path = cache_dir.join("github-api-cache.db");
    let cache = ApiCache::open(&path).unwrap();
    cache
        .set("fresh", &serde_json::json!({ "ok": true }))
        .unwrap();
    cache
        .set_with_ttl(
            "expired",
            &serde_json::json!({ "expired": true }),
            Duration::seconds(-1),
        )
        .unwrap();
    path
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

fn assert_packet_opens_with_coverage(packet: &str) {
    let trimmed = packet.trim_start();
    assert!(
        trimmed.starts_with("# Packet Readiness") || trimmed.starts_with("## Coverage and Limits"),
        "packet should put readiness or coverage and gaps first"
    );

    let coverage = packet
        .find("## Coverage and Limits")
        .expect("packet should include coverage section");
    let summary = packet
        .find("# Summary")
        .expect("packet should include summary section");
    let workstreams = packet
        .find("## Workstreams")
        .expect("packet should include workstreams section");
    if let Some(readiness) = packet.find("# Packet Readiness") {
        assert!(
            readiness < coverage,
            "packet readiness should appear before coverage when present"
        );
    }
    assert!(
        coverage < summary && summary < workstreams,
        "packet section order should be coverage, summary, then workstreams"
    );
}

fn assert_packet_uses_summary_appendix(packet: &str) {
    assert!(
        packet.contains("## Appendix: Receipt Summary"),
        "packet mode should default to a summary appendix"
    );
    assert!(
        !packet.contains("## Appendix: All Receipts"),
        "packet mode should avoid replaying full receipt detail by default"
    );
}

fn section_between<'a>(text: &'a str, start: &str, end: &str) -> &'a str {
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
    let readiness = section_between(packet, "# Packet Readiness", "\n# Claim Candidates");
    let claims = section_between(packet, "# Claim Candidates", "\n## Coverage and Limits");
    format!("{}\n\n{}\n", readiness.trim_end(), claims.trim_end())
        .replace(out_arg, "<OUT>")
        .replace(run_id, "<RUN_ID>")
}

fn assert_packet_quality_report_matches_packet(report_json: &serde_json::Value, packet: &str) {
    let packet_quality = report_json
        .get("packet_quality")
        .expect("report should expose packet_quality");
    let readiness = packet_quality
        .get("packet_readiness")
        .and_then(|readiness| readiness.get("summary"))
        .and_then(serde_json::Value::as_str)
        .expect("packet_readiness summary should be a string")
        .trim();
    assert!(
        packet.contains(readiness),
        "packet.md should render report packet_readiness summary {readiness:?}"
    );

    let claim_candidates = packet_quality
        .get("claim_candidates")
        .and_then(serde_json::Value::as_array)
        .expect("claim_candidates should be an array");
    for candidate in claim_candidates {
        let title = candidate
            .get("title")
            .and_then(serde_json::Value::as_str)
            .expect("claim candidate title should be a string");
        let strength = candidate
            .get("evidence_strength")
            .and_then(serde_json::Value::as_str)
            .expect("claim candidate evidence_strength should be a string");
        assert!(
            packet.contains(title),
            "packet.md should render claim candidate title {title:?}"
        );
        assert!(
            packet.contains(&format!("Evidence strength: `{strength}`")),
            "packet.md should render claim candidate evidence_strength {strength:?}"
        );

        for prompt in candidate
            .get("missing_context_prompts")
            .and_then(serde_json::Value::as_array)
            .expect("missing_context_prompts should be an array")
            .iter()
            .filter_map(serde_json::Value::as_str)
        {
            assert!(
                packet.contains(prompt),
                "packet.md should render missing-context prompt {prompt:?}"
            );
        }
    }
}

fn assert_intake_artifacts(run_dir: &Path) {
    for artifact in [
        "packet.md",
        "ledger.events.jsonl",
        "coverage.manifest.json",
        "workstreams.suggested.yaml",
        "bundle.manifest.json",
        "intake.report.md",
        "intake.report.json",
    ] {
        assert!(
            run_dir.join(artifact).exists(),
            "golden intake run should write {artifact}"
        );
    }
}

fn assert_intake_report_schema_contract(report_json: &serde_json::Value) {
    let schema_path = repo_root().join("contracts/schemas/intake-report.v1.schema.json");
    let schema_text = std::fs::read_to_string(&schema_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", schema_path.display()));
    let schema: serde_json::Value = serde_json::from_str(&schema_text)
        .unwrap_or_else(|err| panic!("parse {}: {err}", schema_path.display()));

    assert_eq!(
        schema["properties"]["schema_version"]["const"], 1,
        "intake report schema should lock v1 reports to schema_version 1"
    );
    assert_eq!(
        schema["additionalProperties"], false,
        "intake report schema should reject undeclared top-level fields"
    );

    let required = schema["required"]
        .as_array()
        .expect("schema should list required top-level fields");
    for field in required {
        let field = field.as_str().expect("required field should be a string");
        assert!(
            report_json.get(field).is_some(),
            "generated intake report should contain required schema field {field}"
        );
    }

    let properties = schema["properties"]
        .as_object()
        .expect("schema should declare top-level properties");
    for field in report_json
        .as_object()
        .expect("intake report should be a JSON object")
        .keys()
    {
        assert!(
            properties.contains_key(field),
            "generated intake report field {field} should be declared in the v1 schema"
        );
    }

    assert_schema_field_names_are_not_secret_bearing(&schema);
    let repair_kind_values =
        schema["$defs"]["repair_source"]["allOf"][1]["properties"]["kind"]["enum"]
            .as_array()
            .expect("repair source schema should document repair kind values");
    for kind in ["missing_token", "invalid_filter", "setup_required"] {
        assert!(
            repair_kind_values
                .iter()
                .any(|value| value.as_str() == Some(kind)),
            "repair source schema should allow kind {kind:?}"
        );
    }
    let source_key_values = schema["$defs"]["source_key"]["enum"]
        .as_array()
        .expect("source identity schema should document canonical source keys");
    for key in [
        "github", "gitlab", "jira", "linear", "manual", "json", "git",
    ] {
        assert!(
            source_key_values
                .iter()
                .any(|value| value.as_str() == Some(key)),
            "source identity schema should allow source_key {key:?}"
        );
    }
    for def in [
        "included_source",
        "skipped_source",
        "source_decision",
        "source_freshness",
        "repair_source",
    ] {
        let source_properties = schema["$defs"][def]["allOf"][1]["properties"]
            .as_object()
            .unwrap_or_else(|| panic!("{def} schema should document source identity fields"));
        for field in ["source", "source_key", "source_label"] {
            assert!(
                source_properties.contains_key(field),
                "{def} schema should document {field}"
            );
        }
    }
    let fixup_properties = schema["$defs"]["fixup"]["allOf"][1]["properties"]
        .as_object()
        .expect("fixup schema should document object properties");
    assert!(
        fixup_properties.contains_key("id"),
        "fixup schema should document optional stable id"
    );
    assert!(
        fixup_properties.contains_key("kind"),
        "fixup schema should document optional stable kind"
    );
    let fixup_kind_values = fixup_properties["kind"]["enum"]
        .as_array()
        .expect("fixup schema should document fixup kind values");
    for kind in ["manual_context", "select_receipts", "repair_sources"] {
        assert!(
            fixup_kind_values
                .iter()
                .any(|value| value.as_str() == Some(kind)),
            "fixup schema should allow kind {kind:?}"
        );
    }
    let action_properties = schema["$defs"]["action"]["allOf"][1]["properties"]
        .as_object()
        .expect("action schema should document object properties");
    for field in ["id", "kind", "label", "command", "writes", "risk"] {
        assert!(
            action_properties.contains_key(field),
            "action schema should document {field:?}"
        );
    }
    let action_kind_values = action_properties["kind"]["enum"]
        .as_array()
        .expect("action schema should document action kind values");
    for kind in ["repair_source", "fixup", "share_manager", "share_public"] {
        assert!(
            action_kind_values
                .iter()
                .any(|value| value.as_str() == Some(kind)),
            "action schema should allow kind {kind:?}"
        );
    }
    let repair_item_properties = schema["$defs"]["repair_item"]["allOf"][1]["properties"]
        .as_object()
        .expect("repair item schema should document object properties");
    for field in [
        "repair_id",
        "repair_key",
        "source_key",
        "source_label",
        "kind",
        "reason",
        "action",
        "clears_when",
        "receipt_refs",
    ] {
        assert!(
            repair_item_properties.contains_key(field),
            "repair item schema should document {field:?}"
        );
    }
    let repair_item_kind_values = repair_item_properties["kind"]["enum"]
        .as_array()
        .expect("repair item schema should document repair item kind values");
    for kind in [
        "manual_evidence_missing",
        "source_skipped_configuration",
        "evidence_debt_open",
    ] {
        assert!(
            repair_item_kind_values
                .iter()
                .any(|value| value.as_str() == Some(kind)),
            "repair item schema should allow kind {kind:?}"
        );
    }
    let repair_action_properties = schema["$defs"]["repair_action"]["allOf"][1]["properties"]
        .as_object()
        .expect("repair action schema should document object properties");
    let repair_action_kind_values = repair_action_properties["kind"]["enum"]
        .as_array()
        .expect("repair action schema should document repair action kind values");
    for kind in ["journal_add", "configure_source", "rerun_intake"] {
        assert!(
            repair_action_kind_values
                .iter()
                .any(|value| value.as_str() == Some(kind)),
            "repair action schema should allow kind {kind:?}"
        );
    }
    let receipt_field_values =
        schema["$defs"]["repair_receipt_ref"]["allOf"][1]["properties"]["field"]["enum"]
            .as_array()
            .expect("repair receipt ref schema should document receipt fields");
    for field in [
        "repair_sources",
        "needs_attention",
        "evidence_debt",
        "journal_suggestions",
    ] {
        assert!(
            receipt_field_values
                .iter()
                .any(|value| value.as_str() == Some(field)),
            "repair receipt ref schema should allow field {field:?}"
        );
    }
    let packet_quality_properties = schema["$defs"]["packet_quality"]["allOf"][1]["properties"]
        .as_object()
        .expect("packet_quality schema should document object properties");
    for field in [
        "packet_readiness",
        "evidence_strength",
        "claim_candidates",
        "share_posture",
    ] {
        assert!(
            packet_quality_properties.contains_key(field),
            "packet_quality schema should document {field:?}"
        );
    }
    let evidence_strength_values =
        schema["$defs"]["evidence_strength"]["allOf"][1]["properties"]["status"]["enum"]
            .as_array()
            .expect("evidence_strength schema should document status values");
    for status in [
        "strong",
        "partial",
        "manual_only",
        "source_skipped",
        "needs_context",
    ] {
        assert!(
            evidence_strength_values
                .iter()
                .any(|value| value.as_str() == Some(status)),
            "evidence_strength schema should allow status {status:?}"
        );
    }
    let packet_readiness_values =
        schema["$defs"]["packet_readiness"]["allOf"][1]["properties"]["status"]["enum"]
            .as_array()
            .expect("packet_readiness schema should document status values");
    for status in ["ready", "ready_with_caveats", "needs_evidence", "blocked"] {
        assert!(
            packet_readiness_values
                .iter()
                .any(|value| value.as_str() == Some(status)),
            "packet_readiness schema should allow status {status:?}"
        );
    }
    let quality_receipt_values =
        schema["$defs"]["quality_receipt_ref"]["allOf"][1]["properties"]["field"]["enum"]
            .as_array()
            .expect("quality receipt ref schema should document receipt fields");
    for field in [
        "included_sources",
        "source_freshness",
        "repair_items",
        "needs_attention",
        "artifacts",
    ] {
        assert!(
            quality_receipt_values
                .iter()
                .any(|value| value.as_str() == Some(field)),
            "quality receipt ref schema should allow field {field:?}"
        );
    }
}

fn assert_json_receipt_schema_contract(
    report_json: &serde_json::Value,
    schema_filename: &str,
    schema_version: &str,
) -> serde_json::Value {
    let schema_path = repo_root().join("contracts/schemas").join(schema_filename);
    let schema_text = std::fs::read_to_string(&schema_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", schema_path.display()));
    let schema: serde_json::Value = serde_json::from_str(&schema_text)
        .unwrap_or_else(|err| panic!("parse {}: {err}", schema_path.display()));

    assert_eq!(
        schema["properties"]["schema_version"]["const"], schema_version,
        "{schema_filename} should lock receipt schema_version"
    );
    assert_eq!(
        schema["additionalProperties"], false,
        "{schema_filename} should reject undeclared top-level fields"
    );

    let required = schema["required"]
        .as_array()
        .expect("schema should list required top-level fields");
    for field in required {
        let field = field.as_str().expect("required field should be a string");
        assert!(
            report_json.get(field).is_some(),
            "generated receipt should contain required schema field {field}"
        );
    }

    let properties = schema["properties"]
        .as_object()
        .expect("schema should declare top-level properties");
    for field in report_json
        .as_object()
        .expect("GitHub activity report should be a JSON object")
        .keys()
    {
        assert!(
            properties.contains_key(field),
            "generated receipt field {field} should be declared in {schema_filename}"
        );
    }

    assert_schema_field_names_are_not_secret_bearing(&schema);
    schema
}

fn assert_github_activity_plan_schema_contract(plan_json: &serde_json::Value) {
    let schema = assert_json_receipt_schema_contract(
        plan_json,
        "github-activity-plan.v1.schema.json",
        "github.activity.plan.v1",
    );
    let profiles = schema["$defs"]["profile"]["enum"]
        .as_array()
        .expect("profile schema should document values");
    for profile in ["scout", "authored", "full"] {
        assert!(
            profiles.iter().any(|value| value.as_str() == Some(profile)),
            "GitHub activity plan schema should allow profile {profile:?}"
        );
    }
    for field in [
        "windows",
        "estimated_totals",
        "budget_policy",
        "next_actions",
    ] {
        assert!(
            plan_json.get(field).is_some(),
            "generated GitHub activity plan should contain {field}"
        );
    }
}

fn assert_github_activity_progress_schema_contract(progress_json: &serde_json::Value) {
    let schema = assert_json_receipt_schema_contract(
        progress_json,
        "github-activity-progress.v1.schema.json",
        "github.activity.progress.v1",
    );
    let states = schema["$defs"]["activity_state"]["enum"]
        .as_array()
        .expect("activity state schema should document values");
    for state in [
        "planned",
        "scouting",
        "running",
        "checkpointed",
        "completed",
    ] {
        assert!(
            states.iter().any(|value| value.as_str() == Some(state)),
            "GitHub activity progress schema should allow state {state:?}"
        );
    }
    for field in [
        "completed_windows",
        "pending_windows",
        "active_window",
        "stop_reason",
        "budget_checkpoint",
        "run_ref",
    ] {
        assert!(
            progress_json.get(field).is_some(),
            "generated GitHub activity progress should contain {field}"
        );
    }
}

fn assert_github_activity_api_ledger_schema_contract(api_ledger_json: &serde_json::Value) {
    let _schema = assert_json_receipt_schema_contract(
        api_ledger_json,
        "github-activity-api-ledger.v1.schema.json",
        "github.activity.api-ledger.v1",
    );
    for field in [
        "requests",
        "cache",
        "rate_limit_snapshots",
        "secondary_limit_events",
    ] {
        assert!(
            api_ledger_json["github_api"].get(field).is_some(),
            "generated GitHub activity API ledger github_api should contain {field}"
        );
    }
    for field in ["requested_owners", "query_strategy", "kept", "dropped"] {
        assert!(
            api_ledger_json["owner_filter"].get(field).is_some(),
            "generated GitHub activity API ledger owner_filter should contain {field}"
        );
    }
}

fn assert_github_activity_report_schema_contract(report_json: &serde_json::Value) {
    let schema = assert_json_receipt_schema_contract(
        report_json,
        "github-activity-report.v1.schema.json",
        "github.activity.report.v1",
    );

    let profiles = schema["$defs"]["profile"]["enum"]
        .as_array()
        .expect("profile schema should document values");
    for profile in ["scout", "authored", "full"] {
        assert!(
            profiles.iter().any(|value| value.as_str() == Some(profile)),
            "GitHub activity report schema should allow profile {profile:?}"
        );
    }
    let states = schema["$defs"]["activity_state"]["enum"]
        .as_array()
        .expect("activity state schema should document values");
    for state in [
        "planned",
        "scouting",
        "running",
        "checkpointed",
        "completed",
    ] {
        assert!(
            states.iter().any(|value| value.as_str() == Some(state)),
            "GitHub activity report schema should allow state {state:?}"
        );
    }
    let labels = schema["$defs"]["final_output_label"]["enum"]
        .as_array()
        .expect("final output label schema should document values");
    for label in [
        "packet",
        "intake_report",
        "coverage",
        "ledger",
        "api_ledger",
    ] {
        assert!(
            labels.iter().any(|value| value.as_str() == Some(label)),
            "GitHub activity report schema should allow final output label {label:?}"
        );
    }

    for field in [
        "requests",
        "cache",
        "rate_limit_snapshots",
        "secondary_limit_events",
    ] {
        assert!(
            report_json["github_api"].get(field).is_some(),
            "generated GitHub activity report github_api should contain {field}"
        );
    }
    for field in ["requested_owners", "query_strategy", "kept", "dropped"] {
        assert!(
            report_json["owner_filter"].get(field).is_some(),
            "generated GitHub activity report owner_filter should contain {field}"
        );
    }
}

fn assert_schema_field_names_are_not_secret_bearing(value: &serde_json::Value) {
    if let Some(properties) = value
        .get("properties")
        .and_then(|properties| properties.as_object())
    {
        for field in properties.keys() {
            let lower = field.to_ascii_lowercase();
            assert!(
                ![
                    "token",
                    "secret",
                    "password",
                    "credential",
                    "api_key",
                    "key_value"
                ]
                .iter()
                .any(|needle| lower.contains(needle)),
                "schema field {field:?} should not be secret-bearing"
            );
        }
    }

    match value {
        serde_json::Value::Array(items) => {
            for item in items {
                assert_schema_field_names_are_not_secret_bearing(item);
            }
        }
        serde_json::Value::Object(map) => {
            for item in map.values() {
                assert_schema_field_names_are_not_secret_bearing(item);
            }
        }
        _ => {}
    }
}

fn assert_golden_intake_report(run_dir: &Path, readiness: &str) -> (String, serde_json::Value) {
    let report_md = std::fs::read_to_string(run_dir.join("intake.report.md")).unwrap();
    let report_json_text = std::fs::read_to_string(run_dir.join("intake.report.json")).unwrap();
    let report_json: serde_json::Value = serde_json::from_str(&report_json_text).unwrap();

    assert_eq!(report_json["schema_version"], 1);
    assert_eq!(report_json["readiness"], readiness);
    assert_intake_report_schema_contract(&report_json);
    assert!(
        report_json["run_id"]
            .as_str()
            .is_some_and(|id| !id.is_empty())
    );
    assert!(
        report_json["reports"]["markdown"]
            .as_str()
            .is_some_and(|path| path.ends_with("intake.report.md"))
    );
    assert!(
        report_json["reports"]["json"]
            .as_str()
            .is_some_and(|path| path.ends_with("intake.report.json"))
    );
    assert!(
        report_json["window"]["since"].as_str().is_some(),
        "intake.report.json should expose the resolved window since date"
    );
    assert!(
        report_json["window"]["until"].as_str().is_some(),
        "intake.report.json should expose the resolved window until date"
    );
    let window_since = report_json["window"]["since"]
        .as_str()
        .expect("intake.report.json should expose the resolved window since date");
    let window_until = report_json["window"]["until"]
        .as_str()
        .expect("intake.report.json should expose the resolved window until date");
    let window_label = report_json["window"]["label"]
        .as_str()
        .expect("intake.report.json should expose the resolved window label");
    let expected_window_display =
        if window_label.contains(window_since) && window_label.contains(window_until) {
            window_label.to_string()
        } else {
            format!("{window_label} ({window_since}..{window_until})")
        };
    let window_line = report_md
        .lines()
        .find(|line| line.starts_with("Window: "))
        .expect("intake.report.md should contain a Window line");
    assert_eq!(
        window_line,
        format!("Window: `{expected_window_display}`"),
        "intake.report.md should render the resolved window without duplicating the date range"
    );

    for key in [
        "included_sources",
        "skipped_sources",
        "source_decisions",
        "source_freshness",
        "repair_sources",
        "repair_items",
        "good",
        "needs_attention",
        "next_commands",
        "evidence_debt",
        "top_fixups",
        "journal_suggestions",
        "share_commands",
        "curation_notes",
        "actions",
        "artifacts",
    ] {
        assert!(
            report_json[key].is_array(),
            "intake.report.json should expose array field {key}"
        );
    }
    let packet_quality = report_json["packet_quality"]
        .as_object()
        .expect("intake.report.json should expose packet_quality");
    assert!(
        packet_quality["packet_readiness"].is_object(),
        "packet_quality should expose packet_readiness"
    );
    assert!(
        packet_quality["evidence_strength"]
            .as_array()
            .is_some_and(|items| !items.is_empty()),
        "packet_quality should expose non-empty evidence_strength"
    );
    assert!(
        packet_quality["claim_candidates"].is_array(),
        "packet_quality should expose claim_candidates"
    );
    assert!(
        packet_quality["share_posture"].is_array(),
        "packet_quality should expose share_posture"
    );
    assert!(
        packet_quality["evidence_strength"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| {
                item["scope"].as_str() == Some("packet")
                    && item["status"]
                        .as_str()
                        .is_some_and(|status| !status.is_empty())
                    && item["receipt_refs"]
                        .as_array()
                        .is_some_and(|refs| !refs.is_empty())
            }),
        "packet_quality evidence_strength should include a receipt-backed packet scope"
    );
    for repair in report_json["repair_sources"]
        .as_array()
        .expect("repair_sources should be an array")
    {
        assert!(
            repair["kind"].as_str().is_some_and(|kind| !kind.is_empty()),
            "generated repair source should expose a stable kind"
        );
    }
    for repair_item in report_json["repair_items"]
        .as_array()
        .expect("repair_items should be an array")
    {
        let repair_id = repair_item["repair_id"]
            .as_str()
            .expect("generated repair item should expose a stable id");
        assert!(
            repair_id.starts_with("repair_") && !repair_id.chars().any(char::is_whitespace),
            "generated repair item id should be a stable token"
        );
        assert!(
            repair_item["repair_key"]
                .as_str()
                .is_some_and(|key| !key.is_empty()),
            "generated repair item should expose a stable repair key"
        );
        assert!(
            repair_item["kind"]
                .as_str()
                .is_some_and(|kind| !kind.is_empty()),
            "generated repair item should expose a stable kind"
        );
        assert!(
            repair_item["action"]["kind"]
                .as_str()
                .is_some_and(|kind| !kind.is_empty()),
            "generated repair item should expose an action kind"
        );
        assert!(
            repair_item["clears_when"]
                .as_str()
                .is_some_and(|condition| !condition.is_empty()),
            "generated repair item should explain its clear condition"
        );
        assert!(
            repair_item["receipt_refs"]
                .as_array()
                .is_some_and(|refs| !refs.is_empty()),
            "generated repair item should cite report receipts"
        );
    }
    for fixup in report_json["top_fixups"]
        .as_array()
        .expect("top_fixups should be an array")
    {
        let id = fixup["id"]
            .as_str()
            .expect("generated fixup should expose a stable id");
        assert!(
            id.starts_with("fixup_") && !id.chars().any(char::is_whitespace),
            "generated fixup id should be a stable token"
        );
        assert!(
            fixup["kind"].as_str().is_some_and(|kind| !kind.is_empty()),
            "generated fixup should expose a stable kind"
        );
    }
    let mut seen_fixup_commands = std::collections::BTreeSet::new();
    for command in report_json["top_fixups"]
        .as_array()
        .expect("top_fixups should be an array")
        .iter()
        .filter_map(|fixup| fixup["command"].as_str())
    {
        assert!(
            seen_fixup_commands.insert(command.to_string()),
            "top_fixups should not repeat the same copyable command: {command}"
        );
    }
    for action in report_json["actions"]
        .as_array()
        .expect("actions should be an array")
    {
        let id = action["id"]
            .as_str()
            .expect("generated action should expose a stable id");
        assert!(
            id.starts_with("action_") && !id.chars().any(char::is_whitespace),
            "generated action id should be a stable token"
        );
        assert!(
            action["kind"].as_str().is_some_and(|kind| !kind.is_empty()),
            "generated action should expose a stable kind"
        );
        assert!(
            action["label"]
                .as_str()
                .is_some_and(|label| !label.is_empty()),
            "generated action should expose a label"
        );
        assert!(
            action["command"]
                .as_str()
                .is_some_and(|command| !command.is_empty()),
            "generated action should expose a command"
        );
        assert!(
            action["writes"].is_boolean(),
            "generated action should expose write intent"
        );
        assert!(
            action["risk"].as_str().is_some_and(|risk| !risk.is_empty()),
            "generated action should expose risk"
        );
    }

    for section in [
        "# Review Intake Report",
        "Packet readiness:",
        "Window:",
        "## Included Sources",
        "## Skipped Sources",
        "## Next Commands",
        "## Evidence Debt",
        "## Top Fixups",
        "## Share Explain Commands",
        "## Share Commands",
        "## Artifacts",
    ] {
        assert!(
            report_md.contains(section),
            "intake.report.md should contain {section:?}"
        );
    }
    let share_explain_idx = report_md
        .find("## Share Explain Commands")
        .expect("intake.report.md should include share explain commands");
    let share_render_idx = report_md
        .find("## Share Commands")
        .expect("intake.report.md should include share render commands");
    assert!(
        share_explain_idx < share_render_idx,
        "intake.report.md should show read-only share explanation before render commands"
    );
    assert!(
        report_md.contains("shiplog share explain manager")
            && report_md.contains("shiplog share explain public")
            && report_md.contains("Share explain commands are read-only"),
        "intake.report.md should include read-only share explain commands"
    );

    for forbidden in [
        "stable-env-key",
        "stable-test-key",
        "do-not-leak",
        "super-secret",
    ] {
        assert!(
            !report_md.contains(forbidden) && !report_json_text.contains(forbidden),
            "intake reports should not contain secret sentinel {forbidden:?}"
        );
    }

    (report_md, report_json)
}

fn assert_ledger_event_count(run_dir: &Path, expected: usize) {
    let ledger = std::fs::read_to_string(run_dir.join("ledger.events.jsonl")).unwrap();
    assert_eq!(
        ledger
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count(),
        expected,
        "golden intake ledger should contain {expected} event(s)"
    );
}

/// Run `collect manual` into `tmp` and return the run directory path.
fn collect_manual_into(tmp: &Path) -> PathBuf {
    let manual_events = tmp.join("manual_events.yaml");
    write_manual_events(&manual_events);

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

fn write_manual_events(path: &Path) {
    std::fs::write(
        path,
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
}

fn fixture_time(day: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2025, 2, day, 12, 0, 0).unwrap()
}

fn fixture_window() -> TimeWindow {
    TimeWindow {
        since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        until: NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
    }
}

fn fixture_repo(full_name: &str) -> RepoRef {
    RepoRef {
        full_name: full_name.into(),
        html_url: Some(format!("https://example.invalid/{full_name}")),
        visibility: RepoVisibility::Private,
    }
}

fn fixture_actor() -> Actor {
    Actor {
        login: "octo".into(),
        id: Some(42),
    }
}

fn fixture_link(source: &str, slug: &str) -> Link {
    Link {
        label: "receipt".into(),
        url: format!("https://example.invalid/{source}/{slug}"),
    }
}

fn fixture_source(system: SourceSystem, slug: &str) -> SourceRef {
    let source = system.as_str().to_string();
    SourceRef {
        system,
        url: Some(format!("https://example.invalid/{source}/{slug}")),
        opaque_id: Some(format!("{source}-{slug}")),
    }
}

fn fixture_pr_event(
    system: SourceSystem,
    repo: &str,
    number: u64,
    title: &str,
    day: u32,
) -> EventEnvelope {
    let source = system.as_str().to_string();
    let occurred_at = fixture_time(day);
    let number_string = number.to_string();
    EventEnvelope {
        id: EventId::from_parts([source.as_str(), "pr", repo, number_string.as_str()]),
        kind: EventKind::PullRequest,
        occurred_at,
        actor: fixture_actor(),
        repo: fixture_repo(repo),
        payload: EventPayload::PullRequest(PullRequestEvent {
            number,
            title: title.into(),
            state: PullRequestState::Merged,
            created_at: occurred_at,
            merged_at: Some(occurred_at),
            additions: Some(120),
            deletions: Some(30),
            changed_files: Some(5),
            touched_paths_hint: vec!["src/".into(), "docs/".into()],
            window: Some(fixture_window()),
        }),
        tags: vec!["fixture".into()],
        links: vec![fixture_link(&source, &number_string)],
        source: fixture_source(system, &number_string),
    }
}

fn fixture_review_event(
    system: SourceSystem,
    repo: &str,
    pull_number: u64,
    title: &str,
    day: u32,
) -> EventEnvelope {
    let source = system.as_str().to_string();
    let occurred_at = fixture_time(day);
    let pull_number_string = pull_number.to_string();
    EventEnvelope {
        id: EventId::from_parts([
            source.as_str(),
            "review",
            repo,
            pull_number_string.as_str(),
            "approved",
        ]),
        kind: EventKind::Review,
        occurred_at,
        actor: fixture_actor(),
        repo: fixture_repo(repo),
        payload: EventPayload::Review(ReviewEvent {
            pull_number,
            pull_title: title.into(),
            submitted_at: occurred_at,
            state: "approved".into(),
            window: Some(fixture_window()),
        }),
        tags: vec!["fixture".into(), "review".into()],
        links: vec![fixture_link(&source, &format!("review-{pull_number}"))],
        source: fixture_source(system, &format!("review-{pull_number}")),
    }
}

fn fixture_manual_event(
    system: SourceSystem,
    repo: &str,
    title: &str,
    event_type: ManualEventType,
    day: u32,
) -> EventEnvelope {
    let source = system.as_str().to_string();
    let occurred_at = fixture_time(day);
    let slug = title.to_ascii_lowercase().replace(' ', "-");
    EventEnvelope {
        id: EventId::from_parts([source.as_str(), "manual", repo, slug.as_str()]),
        kind: EventKind::Manual,
        occurred_at,
        actor: fixture_actor(),
        repo: fixture_repo(repo),
        payload: EventPayload::Manual(ManualEvent {
            event_type,
            title: title.into(),
            description: Some(format!(
                "{source} fixture evidence for review-cycle coverage."
            )),
            started_at: Some(occurred_at.date_naive()),
            ended_at: Some(occurred_at.date_naive()),
            impact: Some("Captured evidence that would otherwise be easy to lose.".into()),
        }),
        tags: vec!["fixture".into()],
        links: vec![fixture_link(&source, &slug)],
        source: fixture_source(system, &slug),
    }
}

fn write_events_jsonl(path: &Path, events: &[EventEnvelope]) {
    let mut text = String::new();
    for event in events {
        text.push_str(&serde_json::to_string(event).unwrap());
        text.push('\n');
    }
    std::fs::write(path, text).unwrap();
}

fn write_coverage_manifest(path: &Path, coverage: &CoverageManifest) {
    std::fs::write(path, serde_json::to_string_pretty(coverage).unwrap()).unwrap();
}

fn all_source_fixture_events() -> Vec<EventEnvelope> {
    vec![
        fixture_pr_event(
            SourceSystem::Github,
            "acme/release-tools",
            101,
            "GitHub release automation",
            3,
        ),
        fixture_review_event(
            SourceSystem::Github,
            "acme/release-tools",
            102,
            "Review GitHub release workflow",
            4,
        ),
        fixture_pr_event(
            SourceSystem::Other("gitlab".into()),
            "platform/deploy",
            7,
            "GitLab self-hosted deploy fix",
            8,
        ),
        fixture_manual_event(
            SourceSystem::Other("jira".into()),
            "ops/platform",
            "Jira OPS-42 rollout checklist",
            ManualEventType::Migration,
            10,
        ),
        fixture_manual_event(
            SourceSystem::Other("linear".into()),
            "product/review",
            "Linear issue triage",
            ManualEventType::Note,
            12,
        ),
        fixture_pr_event(
            SourceSystem::LocalGit,
            "local/hotfix",
            1,
            "Local git hotfix commit",
            14,
        ),
        fixture_manual_event(
            SourceSystem::Manual,
            "customer/reliability",
            "Manual customer debugging note",
            ManualEventType::Incident,
            16,
        ),
        fixture_manual_event(
            SourceSystem::JsonImport,
            "imported/artifacts",
            "Imported architecture decision",
            ManualEventType::Design,
            18,
        ),
    ]
}

fn all_source_fixture_coverage() -> CoverageManifest {
    let window = fixture_window();
    let sources = [
        ("github", 2),
        ("gitlab", 1),
        ("jira", 1),
        ("linear", 1),
        ("local_git", 1),
        ("manual", 1),
        ("json_import", 1),
    ];

    CoverageManifest {
        run_id: RunId("run_all_sources".into()),
        generated_at: fixture_time(20),
        user: "octo".into(),
        window: window.clone(),
        mode: "all-source-fixture".into(),
        sources: sources.iter().map(|(source, _)| (*source).into()).collect(),
        slices: sources
            .iter()
            .map(|(source, count)| CoverageSlice {
                window: window.clone(),
                query: format!("{source} fixture"),
                total_count: *count,
                fetched: *count,
                incomplete_results: Some(false),
                notes: vec!["fixture".into()],
            })
            .collect(),
        warnings: vec![],
        completeness: Completeness::Complete,
    }
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

fn run_intake_without_provider_tokens(tmp: &Path, out: &Path) {
    shiplog_cmd()
        .current_dir(tmp)
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["intake", "--out", out.to_str().unwrap(), "--no-open"])
        .assert()
        .success();
}

fn run_guided_setup_intake_2025_with_sources(tmp: &Path, out: &Path, sources: &[&str]) {
    let mut args = vec!["intake"];
    for source in sources {
        args.push("--source");
        args.push(source);
    }
    args.extend([
        "--out",
        out.to_str().unwrap(),
        "--year",
        "2025",
        "--no-open",
        "--explain",
    ]);

    shiplog_cmd()
        .current_dir(tmp)
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(args)
        .assert()
        .success();
}

fn status_latest_json(
    tmp: &Path,
    out: &Path,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let out_arg = out.to_string_lossy().to_string();
    let assert = shiplog_cmd()
        .current_dir(tmp)
        .env_remove("SHIPLOG_REDACT_KEY")
        .args(["status", "--out", out_arg.as_str(), "--latest", "--json"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    Ok(serde_json::from_str(&stdout)?)
}

fn status_latest_json_with_redaction_key(
    tmp: &Path,
    out: &Path,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let out_arg = out.to_string_lossy().to_string();
    let assert = shiplog_cmd()
        .current_dir(tmp)
        .env("SHIPLOG_REDACT_KEY", "stable-redact-key")
        .args(["status", "--out", out_arg.as_str(), "--latest", "--json"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    Ok(serde_json::from_str(&stdout)?)
}

fn status_next_actions(json: &serde_json::Value) -> &[serde_json::Value] {
    json["next_actions"]
        .as_array()
        .expect("next_actions should be an array")
}

fn assert_status_first_action(json: &serde_json::Value, key: &str, writes: bool) {
    let actions = status_next_actions(json);
    let first = actions
        .first()
        .expect("status should include a next action");
    assert_eq!(first["key"], key);
    assert_eq!(first["writes"], writes);
}

fn assert_status_has_action(json: &serde_json::Value, key: &str, writes: bool) {
    assert!(
        status_next_actions(json)
            .iter()
            .any(|action| action["key"] == key && action["writes"] == writes),
        "status should include action {key} with writes={writes}: {json}"
    );
}

fn assert_status_lacks_command(json: &serde_json::Value, needle: &str) {
    assert!(
        status_next_actions(json)
            .iter()
            .all(|action| !action["command"]
                .as_str()
                .unwrap_or_default()
                .contains(needle)),
        "status should not offer command containing {needle:?}: {json}"
    );
}

fn assert_status_lacks_share_render(json: &serde_json::Value) {
    assert_status_lacks_command(json, "shiplog share manager --");
    assert_status_lacks_command(json, "shiplog share public --");
}

fn load_first_intake_report(out: &Path) -> (PathBuf, serde_json::Value) {
    let report_path = first_run_dir(out).join("intake.report.json");
    let report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
    (report_path, report)
}

fn load_latest_intake_report(out: &Path) -> (PathBuf, serde_json::Value) {
    let run_dir = all_run_dirs(out)
        .into_iter()
        .next_back()
        .expect("expected a latest shiplog run directory");
    let report_path = run_dir.join("intake.report.json");
    let report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
    (report_path, report)
}

fn report_source_event_count(report: &serde_json::Value, source_key: &str) -> u64 {
    report["included_sources"]
        .as_array()
        .expect("report should expose included_sources")
        .iter()
        .find(|source| source["source_key"].as_str() == Some(source_key))
        .and_then(|source| source["event_count"].as_u64())
        .unwrap_or(0)
}

fn remove_packet_quality_from_report(report_path: &Path) -> CliTestResult {
    let mut report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(report_path)?)?;
    report
        .as_object_mut()
        .expect("report should be an object")
        .remove("packet_quality");
    std::fs::write(
        report_path,
        format!("{}\n", serde_json::to_string_pretty(&report)?),
    )?;
    Ok(())
}

fn remove_packet_quality_fields_from_report(report_path: &Path, fields: &[&str]) -> CliTestResult {
    let mut report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(report_path)?)?;
    let packet_quality = report
        .get_mut("packet_quality")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "report should expose packet_quality object",
            )
        })?;
    for field in fields {
        packet_quality.remove(*field);
    }
    std::fs::write(
        report_path,
        format!("{}\n", serde_json::to_string_pretty(&report)?),
    )?;
    Ok(())
}

fn first_repair_id_with_action(report: &serde_json::Value, action_kind: &str) -> String {
    report["repair_items"]
        .as_array()
        .expect("repair_items should be an array")
        .iter()
        .find(|item| item["action"]["kind"].as_str() == Some(action_kind))
        .and_then(|item| item["repair_id"].as_str())
        .expect("expected repair item with action kind")
        .to_string()
}

fn first_repair_id_without_action(report: &serde_json::Value, action_kind: &str) -> String {
    report["repair_items"]
        .as_array()
        .expect("repair_items should be an array")
        .iter()
        .find(|item| item["action"]["kind"].as_str() != Some(action_kind))
        .and_then(|item| item["repair_id"].as_str())
        .expect("expected repair item with different action kind")
        .to_string()
}

fn write_repair_diff_report(
    base_report: &serde_json::Value,
    out: &Path,
    run_id: &str,
    repair_items: serde_json::Value,
) -> PathBuf {
    let run_dir = out.join(run_id);
    std::fs::create_dir_all(&run_dir).unwrap();
    std::fs::write(run_dir.join("ledger.events.jsonl"), "").unwrap();
    if let Some(base_run_dir) = base_report["run_dir"].as_str().map(PathBuf::from) {
        let base_coverage = base_run_dir.join("coverage.manifest.json");
        if base_coverage.exists() {
            let mut coverage: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(base_coverage).unwrap()).unwrap();
            coverage["run_id"] = serde_json::json!(run_id);
            std::fs::write(
                run_dir.join("coverage.manifest.json"),
                format!("{}\n", serde_json::to_string_pretty(&coverage).unwrap()),
            )
            .unwrap();
        }
    }
    let report_path = run_dir.join("intake.report.json");
    let mut report = base_report.clone();
    report["run_id"] = serde_json::json!(run_id);
    report["run_dir"] = serde_json::json!(run_dir.display().to_string());
    report["reports"]["json"] = serde_json::json!(report_path.display().to_string());
    report["repair_items"] = repair_items;
    std::fs::write(
        &report_path,
        format!("{}\n", serde_json::to_string_pretty(&report).unwrap()),
    )
    .unwrap();
    report_path
}

fn repair_diff_item(
    repair_id: &str,
    repair_key: &str,
    reason: &str,
    command: &str,
    clears_when: &str,
) -> serde_json::Value {
    serde_json::json!({
        "repair_id": repair_id,
        "repair_key": repair_key,
        "kind": "manual_evidence_missing",
        "reason": reason,
        "action": {
            "kind": "journal_add",
            "command": command
        },
        "clears_when": clears_when,
        "receipt_refs": [
            {
                "field": "evidence_debt"
            }
        ]
    })
}

fn all_run_dirs(out: &Path) -> Vec<PathBuf> {
    let mut runs: Vec<_> = std::fs::read_dir(out)
        .unwrap()
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.join("ledger.events.jsonl").exists())
        .collect();
    runs.sort();
    runs
}

fn file_tree_manifest(root: &Path) -> Vec<(String, u64, Option<std::time::SystemTime>)> {
    fn visit(
        root: &Path,
        dir: &Path,
        entries: &mut Vec<(String, u64, Option<std::time::SystemTime>)>,
    ) {
        let mut children: Vec<_> = std::fs::read_dir(dir)
            .unwrap_or_else(|err| panic!("read {}: {err}", dir.display()))
            .filter_map(Result::ok)
            .collect();
        children.sort_by_key(|entry| entry.path());
        for child in children {
            let path = child.path();
            let metadata = child
                .metadata()
                .unwrap_or_else(|err| panic!("metadata {}: {err}", path.display()));
            if metadata.is_dir() {
                visit(root, &path, entries);
            } else if metadata.is_file() {
                let relative = path
                    .strip_prefix(root)
                    .unwrap_or_else(|err| panic!("strip prefix {}: {err}", path.display()))
                    .to_string_lossy()
                    .replace('\\', "/");
                entries.push((relative, metadata.len(), metadata.modified().ok()));
            }
        }
    }

    let mut entries = Vec::new();
    if root.exists() {
        visit(root, root, &mut entries);
    }
    entries
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
        .stdout(predicate::str::contains("doctor"))
        .stdout(predicate::str::contains("config"))
        .stdout(predicate::str::contains("sources"))
        .stdout(predicate::str::contains("cache"))
        .stdout(predicate::str::contains("journal"))
        .stdout(predicate::str::contains("collect"))
        .stdout(predicate::str::contains("render"))
        .stdout(predicate::str::contains("share"))
        .stdout(predicate::str::contains("refresh"))
        .stdout(predicate::str::contains("workstreams"))
        .stdout(predicate::str::contains("runs"))
        .stdout(predicate::str::contains("review"))
        .stdout(predicate::str::contains("open"))
        .stdout(predicate::str::contains("merge"))
        .stdout(predicate::str::contains("import"))
        .stdout(predicate::str::contains("run"))
        .stdout(predicate::str::contains("Review-ready loop:"))
        .stdout(predicate::str::contains("shiplog init --guided"))
        .stdout(predicate::str::contains("shiplog doctor --setup"))
        .stdout(predicate::str::contains("shiplog status --latest"))
        .stdout(predicate::str::contains(
            "shiplog intake --last-6-months --explain",
        ))
        .stdout(predicate::str::contains("shiplog repair plan --latest"))
        .stdout(predicate::str::contains(
            "shiplog journal add --from-repair <repair_id>",
        ))
        .stdout(predicate::str::contains("shiplog repair diff --latest"))
        .stdout(predicate::str::contains("shiplog runs diff --latest"))
        .stdout(predicate::str::contains(
            "shiplog share explain manager --latest",
        ))
        .stdout(predicate::str::contains("Advanced GitHub activity:"))
        .stdout(predicate::str::contains("shiplog github activity plan"))
        .stdout(predicate::str::contains(
            "shiplog github activity run --profile authored --resume",
        ))
        .stdout(predicate::str::contains(
            "shiplog github activity run --profile full --resume",
        ))
        .stdout(predicate::str::contains("shiplog github activity merge"))
        .stdout(predicate::str::contains("Read-first commands:"));
}

#[test]
fn github_activity_help_teaches_budget_aware_harvest_path() {
    shiplog_cmd()
        .args(["github", "activity", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Recommended harvest path:"))
        .stdout(predicate::str::contains("shiplog github activity plan"))
        .stdout(predicate::str::contains("shiplog github activity scout"))
        .stdout(predicate::str::contains(
            "shiplog github activity run --profile authored --resume",
        ))
        .stdout(predicate::str::contains(
            "shiplog github activity run --profile full --resume",
        ))
        .stdout(predicate::str::contains("shiplog github activity status"))
        .stdout(predicate::str::contains("shiplog github activity report"))
        .stdout(predicate::str::contains("shiplog github activity merge"))
        .stdout(predicate::str::contains("API-budget posture:"))
        .stdout(predicate::str::contains(
            "plan reads config and writes github.activity.plan.json without provider calls",
        ))
        .stdout(predicate::str::contains(
            "scout and run read GitHub, write progress/API-ledger receipts, and honor --resume",
        ))
        .stdout(predicate::str::contains(
            "status reads existing receipts only",
        ))
        .stdout(predicate::str::contains(
            "merge writes final activity outputs from completed receipts",
        ))
        .stdout(predicate::str::contains(
            "it does not render share profiles",
        ));
}

#[test]
fn init_help_shows_options() {
    shiplog_cmd()
        .args(["init", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--source"))
        .stdout(predicate::str::contains("--dry-run"))
        .stdout(predicate::str::contains("--force"))
        .stdout(predicate::str::contains("--guided"));
}

#[test]
fn doctor_help_shows_options() {
    shiplog_cmd()
        .args(["doctor", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--config"))
        .stdout(predicate::str::contains("--source"))
        .stdout(predicate::str::contains("--setup"))
        .stdout(predicate::str::contains("--repair-plan"));
}

#[test]
fn doctor_help_teaches_setup_first_path() {
    shiplog_cmd()
        .args(["doctor", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Setup-first path:"))
        .stdout(predicate::str::contains("shiplog init --guided"))
        .stdout(predicate::str::contains("shiplog doctor --setup"))
        .stdout(predicate::str::contains("shiplog sources status"))
        .stdout(predicate::str::contains("shiplog status --latest"))
        .stdout(predicate::str::contains(
            "shiplog intake --last-6-months --explain",
        ))
        .stdout(predicate::str::contains("Safety posture:"))
        .stdout(predicate::str::contains(
            "doctor --setup reads local setup state without provider network calls or writes",
        ))
        .stdout(predicate::str::contains(
            "doctor --repair-plan prints setup repair guidance, not evidence repair commands",
        ))
        .stdout(predicate::str::contains(
            "sources status shows source readiness without collecting evidence",
        ))
        .stdout(predicate::str::contains(
            "Run doctor --setup before intake when setup or redaction state is uncertain",
        ));
}

#[test]
fn status_help_teaches_review_loop_cockpit() {
    shiplog_cmd()
        .args(["status", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--latest"))
        .stdout(predicate::str::contains("--json"))
        .stdout(predicate::str::contains("Review-loop cockpit:"))
        .stdout(predicate::str::contains("shiplog doctor --setup"))
        .stdout(predicate::str::contains("shiplog status --latest"))
        .stdout(predicate::str::contains(
            "shiplog intake --last-6-months --explain",
        ))
        .stdout(predicate::str::contains("shiplog repair plan --latest"))
        .stdout(predicate::str::contains(
            "shiplog share explain manager --latest",
        ))
        .stdout(predicate::str::contains("Safety posture:"))
        .stdout(predicate::str::contains(
            "status --latest reads setup state and durable receipts without collecting evidence",
        ))
        .stdout(predicate::str::contains(
            "status --latest --json exposes the same review-loop state for agents and scripts",
        ))
        .stdout(predicate::str::contains(
            "status reports the first safe next action; it does not repair, rerun intake, or render share packets",
        ))
        .stdout(predicate::str::contains(
            "Use status before first intake and after reruns to choose the next command",
        ));
}

#[test]
fn sources_help_shows_status_options() {
    shiplog_cmd()
        .args(["sources", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("status"));

    shiplog_cmd()
        .args(["sources", "status", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--config"))
        .stdout(predicate::str::contains("--source"));
}

#[test]
fn config_help_shows_validate_explain_and_migrate() {
    shiplog_cmd()
        .args(["config", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("validate"))
        .stdout(predicate::str::contains("explain"))
        .stdout(predicate::str::contains("migrate"));

    shiplog_cmd()
        .args(["config", "validate", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--config"));

    shiplog_cmd()
        .args(["config", "explain", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--config"));

    shiplog_cmd()
        .args(["config", "migrate", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--config"))
        .stdout(predicate::str::contains("--dry-run"));
}

#[test]
fn cache_help_shows_stats_inspect_and_clean() {
    shiplog_cmd()
        .args(["cache", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("stats"))
        .stdout(predicate::str::contains("inspect"))
        .stdout(predicate::str::contains("clean"));

    shiplog_cmd()
        .args(["cache", "stats", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--out"))
        .stdout(predicate::str::contains("--cache-dir"))
        .stdout(predicate::str::contains("--source"));

    shiplog_cmd()
        .args(["cache", "clean", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--older-than"))
        .stdout(predicate::str::contains("--all"))
        .stdout(predicate::str::contains("--dry-run"))
        .stdout(predicate::str::contains("--yes"));
}

#[test]
fn journal_help_shows_add_options() {
    shiplog_cmd()
        .args(["journal", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("add"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("edit"));

    shiplog_cmd()
        .args(["journal", "add", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--events"))
        .stdout(predicate::str::contains("--date"))
        .stdout(predicate::str::contains("--start"))
        .stdout(predicate::str::contains("--end"))
        .stdout(predicate::str::contains("--title"))
        .stdout(predicate::str::contains("--workstream"))
        .stdout(predicate::str::contains("--receipt"))
        .stdout(predicate::str::contains("--dry-run"));

    shiplog_cmd()
        .args(["journal", "add", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Report-derived repair path:"))
        .stdout(predicate::str::contains("shiplog repair plan --latest"))
        .stdout(predicate::str::contains(
            "shiplog journal add --from-repair <repair_id>",
        ))
        .stdout(predicate::str::contains(
            "shiplog intake --last-6-months --explain",
        ))
        .stdout(predicate::str::contains("shiplog repair diff --latest"))
        .stdout(predicate::str::contains("shiplog runs diff --latest"))
        .stdout(predicate::str::contains("Safety posture:"))
        .stdout(predicate::str::contains(
            "--from-repair resolves one manual-evidence repair item from intake.report.json",
        ))
        .stdout(predicate::str::contains(
            "journal add writes local manual_events.yaml unless --dry-run is used",
        ))
        .stdout(predicate::str::contains(
            "--out, --run, and --latest select the report used for --from-repair lookup",
        ))
        .stdout(predicate::str::contains(
            "Rerun intake after adding evidence so repair diff and runs diff can compare receipts",
        ));

    shiplog_cmd()
        .args(["journal", "list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--events"))
        .stdout(predicate::str::contains("--workstream"))
        .stdout(predicate::str::contains("--tag"));

    shiplog_cmd()
        .args(["journal", "edit", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--events"))
        .stdout(predicate::str::contains("--id"))
        .stdout(predicate::str::contains("--title"))
        .stdout(predicate::str::contains("--workstream"))
        .stdout(predicate::str::contains("--impact"))
        .stdout(predicate::str::contains("--tag"))
        .stdout(predicate::str::contains("--receipt"))
        .stdout(predicate::str::contains("--dry-run"));
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
        .stdout(predicate::str::contains("move"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("delete"))
        .stdout(predicate::str::contains("receipts"))
        .stdout(predicate::str::contains("receipt"))
        .stdout(predicate::str::contains("split"));
}

#[test]
fn runs_help_shows_list_show_and_compare() {
    shiplog_cmd()
        .args(["runs", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("show"))
        .stdout(predicate::str::contains("compare"))
        .stdout(predicate::str::contains("diff"));

    shiplog_cmd()
        .args(["runs", "diff", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--latest"))
        .stdout(predicate::str::contains("--from"))
        .stdout(predicate::str::contains("--to"))
        .stdout(predicate::str::contains("Packet-quality comparison path:"))
        .stdout(predicate::str::contains(
            "shiplog intake --last-6-months --explain",
        ))
        .stdout(predicate::str::contains("shiplog repair diff --latest"))
        .stdout(predicate::str::contains("shiplog runs diff --latest"))
        .stdout(predicate::str::contains("shiplog open packet --latest"))
        .stdout(predicate::str::contains(
            "shiplog share explain manager --latest",
        ))
        .stdout(predicate::str::contains("Safety posture:"))
        .stdout(predicate::str::contains(
            "runs diff reads existing run summaries, intake reports, and repair receipts",
        ))
        .stdout(predicate::str::contains(
            "--latest compares the newest two runs; --from and --to compare explicit run IDs",
        ))
        .stdout(predicate::str::contains(
            "runs diff reports improvement, regressions, and remaining weak signals without writing files",
        ))
        .stdout(predicate::str::contains(
            "Use share explain after runs diff before rendering any share profile",
        ));
}

#[test]
fn open_help_shows_artifact_targets() {
    shiplog_cmd()
        .args(["open", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("packet"))
        .stdout(predicate::str::contains("workstreams"))
        .stdout(predicate::str::contains("intake-report"))
        .stdout(predicate::str::contains("out"));
}

#[test]
fn open_packet_help_shows_run_and_print_options() {
    shiplog_cmd()
        .args(["open", "packet", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--latest"))
        .stdout(predicate::str::contains("--run"))
        .stdout(predicate::str::contains("--print-path"));
}

#[test]
fn open_intake_report_help_shows_run_and_print_options() {
    shiplog_cmd()
        .args(["open", "intake-report", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--latest"))
        .stdout(predicate::str::contains("--run"))
        .stdout(predicate::str::contains("--print-path"));
}

#[test]
fn intake_help_shows_rescue_mode_options() {
    shiplog_cmd()
        .args(["intake", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--last-6-months"))
        .stdout(predicate::str::contains("--period"))
        .stdout(predicate::str::contains("--year"))
        .stdout(predicate::str::contains("--source"))
        .stdout(predicate::str::contains("--profile"))
        .stdout(predicate::str::contains("--no-open"))
        .stdout(predicate::str::contains("--explain"));
}

#[test]
fn intake_help_teaches_evidence_collection_handoff() {
    shiplog_cmd()
        .args(["intake", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Evidence collection path:"))
        .stdout(predicate::str::contains("shiplog doctor --setup"))
        .stdout(predicate::str::contains("shiplog status --latest"))
        .stdout(predicate::str::contains(
            "shiplog intake --last-6-months --explain",
        ))
        .stdout(predicate::str::contains("shiplog repair plan --latest"))
        .stdout(predicate::str::contains(
            "shiplog share explain manager --latest",
        ))
        .stdout(predicate::str::contains("Safety posture:"))
        .stdout(predicate::str::contains(
            "intake writes a new run under the output directory and renders packet/report receipts",
        ))
        .stdout(predicate::str::contains(
            "--explain prints source decisions and repair hints to the terminal",
        ))
        .stdout(predicate::str::contains(
            "--no-open prevents launching artifacts after writes; it does not make intake read-only",
        ))
        .stdout(predicate::str::contains(
            "Run status --latest after intake to choose repair, diff, or share explain next",
        ));
}

#[test]
fn review_help_shows_run_options() {
    shiplog_cmd()
        .args(["review", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("weekly"))
        .stdout(predicate::str::contains("fixups"))
        .stdout(predicate::str::contains("--out"))
        .stdout(predicate::str::contains("--config"))
        .stdout(predicate::str::contains("--latest"))
        .stdout(predicate::str::contains("--period"))
        .stdout(predicate::str::contains("--run"))
        .stdout(predicate::str::contains("--strict"));
}

#[test]
fn review_weekly_help_shows_run_options() {
    shiplog_cmd()
        .args(["review", "weekly", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--out"))
        .stdout(predicate::str::contains("--latest"))
        .stdout(predicate::str::contains("--run"))
        .stdout(predicate::str::contains("--strict"));
}

#[test]
fn review_fixups_help_shows_run_options() {
    shiplog_cmd()
        .args(["review", "fixups", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--out"))
        .stdout(predicate::str::contains("--latest"))
        .stdout(predicate::str::contains("--run"))
        .stdout(predicate::str::contains("--commands-only"))
        .stdout(predicate::str::contains("--journal-template"));
}

#[test]
fn repair_help_teaches_receipt_derived_handoff_loop() {
    shiplog_cmd()
        .args(["repair", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("plan"))
        .stdout(predicate::str::contains("diff"))
        .stdout(predicate::str::contains(
            "Receipt-derived repair loop:",
        ))
        .stdout(predicate::str::contains("shiplog repair plan --latest"))
        .stdout(predicate::str::contains(
            "shiplog journal add --from-repair <repair_id>",
        ))
        .stdout(predicate::str::contains(
            "shiplog intake --last-6-months --explain",
        ))
        .stdout(predicate::str::contains("shiplog repair diff --latest"))
        .stdout(predicate::str::contains("shiplog runs diff --latest"))
        .stdout(predicate::str::contains("Safety posture:"))
        .stdout(predicate::str::contains(
            "repair plan reads the latest intake.report.json and does not rediscover sources",
        ))
        .stdout(predicate::str::contains(
            "journal add --from-repair writes manual evidence only for safe report-derived manual repairs",
        ))
        .stdout(predicate::str::contains(
            "repair diff and runs diff compare receipts across runs without writing repair data",
        ))
        .stdout(predicate::str::contains(
            "Run repair plan before copying individual repair commands",
        ));
}

#[test]
fn share_help_shows_profiles_and_safety_options() {
    shiplog_cmd()
        .args(["share", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("manager"))
        .stdout(predicate::str::contains("public"))
        .stdout(predicate::str::contains("Read-first share path:"))
        .stdout(predicate::str::contains(
            "shiplog share explain manager --latest",
        ))
        .stdout(predicate::str::contains(
            "shiplog share verify manager --latest",
        ))
        .stdout(predicate::str::contains("shiplog share manager --latest"))
        .stdout(predicate::str::contains("Safety posture:"))
        .stdout(predicate::str::contains(
            "share explain reads receipts and reports what a profile would include, remove, or block",
        ))
        .stdout(predicate::str::contains(
            "share verify checks readiness without writing profile packets",
        ))
        .stdout(predicate::str::contains(
            "manager/public render commands write profile artifacts only after redaction setup is available",
        ))
        .stdout(predicate::str::contains(
            "Use share explain before rendering when packet readiness, evidence debt, or redaction setup is uncertain",
        ));

    shiplog_cmd()
        .args(["share", "manager", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--out"))
        .stdout(predicate::str::contains("--latest"))
        .stdout(predicate::str::contains("--run"))
        .stdout(predicate::str::contains("--redact-key"))
        .stdout(predicate::str::contains("--zip"));

    shiplog_cmd()
        .args(["share", "explain", "manager", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--out"))
        .stdout(predicate::str::contains("--latest"))
        .stdout(predicate::str::contains("--run"))
        .stdout(predicate::str::contains("--redact-key"))
        .stdout(predicate::str::contains("Read-first explain path:"))
        .stdout(predicate::str::contains("shiplog status --latest"))
        .stdout(predicate::str::contains("shiplog runs diff --latest"))
        .stdout(predicate::str::contains(
            "shiplog share explain manager --latest",
        ))
        .stdout(predicate::str::contains(
            "shiplog share verify manager --latest",
        ))
        .stdout(predicate::str::contains("shiplog share manager --latest"))
        .stdout(predicate::str::contains("Safety posture:"))
        .stdout(predicate::str::contains(
            "share explain reads run receipts, intake reports, workstreams, and share posture without rendering profile artifacts",
        ))
        .stdout(predicate::str::contains(
            "It reports included, removed, blocked, and needs-review items before any profile write",
        ))
        .stdout(predicate::str::contains(
            "Missing SHIPLOG_REDACT_KEY blocks rendering but does not block explanation",
        ))
        .stdout(predicate::str::contains(
            "Use share verify after explain, then render only when the profile is ready",
        ));

    shiplog_cmd()
        .args(["share", "verify", "manager", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--out"))
        .stdout(predicate::str::contains("--latest"))
        .stdout(predicate::str::contains("--run"))
        .stdout(predicate::str::contains("--redact-key"))
        .stdout(predicate::str::contains("--strict"));

    shiplog_cmd()
        .args(["share", "verify", "manifest", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--out"))
        .stdout(predicate::str::contains("--latest"))
        .stdout(predicate::str::contains("--run"))
        .stdout(predicate::str::contains("--profile"));
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
        .stdout(predicate::str::contains("shiplog collect multi"));

    let config = std::fs::read_to_string(tmp.path().join("shiplog.toml")).unwrap();
    assert!(config.contains("[shiplog]\nconfig_version = 1"));
    assert!(config.contains("[periods.\"review-cycle\"]"));
    assert!(config.contains("preset = \"last-6-months\""));
    assert!(config.contains("[sources.github]"));
    assert!(config.contains("enabled = true"));
    assert!(config.contains("me = true"));
    assert!(config.contains("[sources.manual]"));
    assert!(config.contains("events = \"./manual_events.yaml\""));

    let manual = std::fs::read_to_string(tmp.path().join("manual_events.yaml")).unwrap();
    assert!(manual.contains("version: 1"));
    assert!(manual.contains("events: []"));
}

#[test]
fn init_guided_creates_local_first_setup_without_token_providers() -> CliTestResult {
    let tmp = TempDir::new()?;
    git2::Repository::init(tmp.path())?;

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["init", "--guided"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized guided shiplog setup"))
        .stdout(predicate::str::contains("shiplog doctor --setup"))
        .stdout(predicate::str::contains("shiplog sources status"))
        .stdout(predicate::str::contains("shiplog status --latest"))
        .stdout(predicate::str::contains(
            "shiplog intake --last-6-months --explain",
        ))
        .stdout(predicate::str::contains("shiplog collect multi").not())
        .stdout(predicate::str::contains("export GITHUB_TOKEN").not());

    let config = std::fs::read_to_string(tmp.path().join("shiplog.toml"))?;
    assert!(config.contains("[sources.git]\nenabled = true"));
    assert!(config.contains("[sources.manual]\nenabled = true"));
    assert!(config.contains("[sources.github]\n# Set GITHUB_TOKEN"));
    assert!(config.contains(
        "[sources.github]\n# Set GITHUB_TOKEN. Use either user or me = true.\nenabled = false"
    ));
    assert!(config.contains(
        "[sources.gitlab]\n# Set GITLAB_TOKEN. Use either user or me = true.\nenabled = false"
    ));
    assert!(config.contains("[sources.jira]\n# Set JIRA_TOKEN, JIRA_AUTH_USER, user, and instance before enabling.\nenabled = false"));
    assert!(config.contains(
        "[sources.linear]\n# Set LINEAR_API_KEY and user_id before enabling.\nenabled = false"
    ));
    assert!(config.contains("# Set SHIPLOG_REDACT_KEY before manager or public share rendering."));

    let manual = std::fs::read_to_string(tmp.path().join("manual_events.yaml"))?;
    assert!(manual.contains("version: 1"));
    assert!(manual.contains("events: []"));

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("SHIPLOG_REDACT_KEY")
        .args(["doctor", "--setup"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Setup readiness: Needs setup"))
        .stdout(predicate::str::contains("Local git"))
        .stdout(predicate::str::contains("Manual journal"))
        .stdout(predicate::str::contains("Manager share"))
        .stdout(predicate::str::contains("Public share"))
        .stdout(predicate::str::contains("SHIPLOG_REDACT_KEY not set"));
    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["sources", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("git"))
        .stdout(predicate::str::contains("manual"));

    Ok(())
}

#[test]
fn status_latest_missing_setup_prints_init_guidance_without_writing() -> CliTestResult {
    let tmp = TempDir::new()?;
    let before = file_tree_manifest(tmp.path());

    let assert = shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("SHIPLOG_REDACT_KEY")
        .args(["status", "--latest"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Review loop status: Needs setup"))
        .stdout(predicate::str::contains("Setup:"))
        .stdout(predicate::str::contains(
            "shiplog doctor --setup [read-only]",
        ))
        .stdout(predicate::str::contains("shiplog init --guided [writes]"));
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;

    assert!(
        stdout.contains("Receipts:"),
        "status should name read receipt surfaces"
    );
    assert_eq!(
        before,
        file_tree_manifest(tmp.path()),
        "status should not write setup or run artifacts"
    );
    Ok(())
}

#[test]
fn status_latest_ready_setup_without_run_routes_to_intake() -> CliTestResult {
    let tmp = TempDir::new()?;
    let out = tmp.path().join("custom-out");

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["init", "--guided"])
        .assert()
        .success();

    let before = file_tree_manifest(tmp.path());
    let assert = shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("SHIPLOG_REDACT_KEY")
        .args([
            "status",
            "--out",
            out.to_str().expect("out path should be valid UTF-8"),
            "--latest",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Review loop status: Ready to collect",
        ))
        .stdout(predicate::str::contains("Latest run:"))
        .stdout(predicate::str::contains("none found"))
        .stdout(predicate::str::contains("shiplog intake --out"))
        .stdout(predicate::str::contains("[writes]"));
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;

    assert!(
        !stdout.contains("shiplog share manager"),
        "status should not offer share rendering before a run"
    );
    assert_eq!(
        before,
        file_tree_manifest(tmp.path()),
        "status should be read-only after guided init"
    );
    Ok(())
}

#[test]
fn github_activity_plan_writes_static_receipt_without_provider_calls() -> CliTestResult {
    let tmp = TempDir::new()?;
    let config = tmp.path().join("shiplog-github-full.toml");
    let out = tmp.path().join("out/github-full");
    std::fs::write(
        &config,
        r#"[shiplog]
config_version = 1

[defaults]
out = "./out/github-full"

[github_activity]
actor = "EffortlessSteven"
repo_owners = ["EffortlessMetrics", "EffortlessSteven"]
since = "2026-01-01"
until = "2026-03-01"
include_authored_prs = true
include_reviews = true
profile = "scout"
cache_dir = "./out/github-full/.cache"

[github_activity.budget]
max_search_per_minute = 24
on_exhausted = "checkpoint_and_stop"

[sources.github]
enabled = true
user = "EffortlessSteven"
mode = "created"
"#,
    )?;

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .args([
            "github",
            "activity",
            "plan",
            "--config",
            config.to_str().expect("config path should be valid UTF-8"),
            "--out",
            out.to_str().expect("out path should be valid UTF-8"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("GitHub activity plan written:"))
        .stdout(predicate::str::contains("Actor: EffortlessSteven"))
        .stdout(predicate::str::contains(
            "Repository owners: EffortlessMetrics, EffortlessSteven",
        ))
        .stdout(predicate::str::contains(
            "Provider calls: none (static plan)",
        ))
        .stdout(predicate::str::contains(
            "Next: shiplog github activity scout --resume [writes]",
        ));

    let plan_path = out.join("github.activity.plan.json");
    let plan: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&plan_path)?)?;

    assert_eq!(plan["schema_version"], "github.activity.plan.v1");
    assert_github_activity_plan_schema_contract(&plan);
    assert_eq!(plan["actor"], "EffortlessSteven");
    assert_eq!(plan["repo_owners"][0], "EffortlessMetrics");
    assert_eq!(plan["repo_owners"][1], "EffortlessSteven");
    assert_eq!(plan["owner_filter_requested"], true);
    assert_eq!(plan["query_strategy"], "actor_search_owner_filter");
    assert_eq!(plan["profile"], "scout");
    assert_eq!(plan["planning_mode"], "static");
    assert_eq!(plan["estimated_totals"]["search_requests"], 22);
    assert_eq!(plan["estimated_totals"]["core_requests"], 0);
    assert_eq!(plan["estimated_totals"]["review_requests"], 0);
    assert_eq!(plan["budget_policy"]["max_search_per_minute"], 24);
    assert_eq!(plan["budget_policy"]["on_exhausted"], "checkpoint_and_stop");
    assert_eq!(
        plan["windows"][0]["queries"][0]["search_query"],
        "is:pr author:EffortlessSteven created:2026-01-01..2026-01-31"
    );
    assert_eq!(plan["windows"][0]["queries"][0]["cache_reuse"], "unknown");
    assert_eq!(
        plan["next_actions"][0]["command"],
        "shiplog github activity scout --resume"
    );
    assert_eq!(plan["next_actions"][0]["writes"], true);
    assert!(
        !out.join(".cache").exists(),
        "static planning should not create the GitHub API cache"
    );
    assert!(
        !out.join("packet.md").exists(),
        "static planning should not render a packet"
    );

    Ok(())
}

#[test]
fn github_activity_status_reports_missing_plan_without_writing() -> CliTestResult {
    let tmp = TempDir::new()?;
    let config = tmp.path().join("shiplog-github-full.toml");
    let out = tmp.path().join("out/github-full");
    std::fs::write(
        &config,
        r#"[shiplog]
config_version = 1

[defaults]
out = "./out/github-full"

[github_activity]
actor = "EffortlessSteven"
repo_owners = ["EffortlessMetrics"]
since = "2026-01-01"
until = "2026-02-01"
profile = "scout"

[sources.github]
enabled = true
user = "EffortlessSteven"
mode = "created"
"#,
    )?;
    let before = file_tree_manifest(tmp.path());

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .args([
            "github",
            "activity",
            "status",
            "--config",
            config.to_str().expect("config path should be valid UTF-8"),
            "--out",
            out.to_str().expect("out path should be valid UTF-8"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "GitHub activity status: Not planned",
        ))
        .stdout(predicate::str::contains(
            "Provider calls: none (receipt status)",
        ))
        .stdout(predicate::str::contains("Writes: none"))
        .stdout(predicate::str::contains("Plan:"))
        .stdout(predicate::str::contains("missing"))
        .stdout(predicate::str::contains(
            "shiplog github activity plan --config",
        ))
        .stdout(predicate::str::contains("[writes]"));

    assert_eq!(
        before,
        file_tree_manifest(tmp.path()),
        "github activity status should be read-only when receipts are missing"
    );
    Ok(())
}

#[test]
fn github_activity_status_summarizes_completed_receipts_without_writing() -> CliTestResult {
    let tmp = TempDir::new()?;
    let config = tmp.path().join("shiplog-github-full.toml");
    let out = tmp.path().join("out/github-full");
    std::fs::write(
        &config,
        r#"[shiplog]
config_version = 1

[defaults]
out = "./out/github-full"

[github_activity]
actor = "octocat"
repo_owners = ["acme"]
since = "2025-01-01"
until = "2025-02-01"
include_authored_prs = true
include_reviews = false
profile = "authored"
cache_dir = "./out/github-full/.cache"

[github_activity.budget]
max_search_requests = 10
max_core_requests = 10
max_search_per_minute = 24
on_exhausted = "checkpoint_and_stop"

[sources.github]
enabled = true
user = "octocat"
mode = "created"
repo_owners = ["acme"]
"#,
    )?;

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .args([
            "github",
            "activity",
            "plan",
            "--config",
            config.to_str().expect("config path should be valid UTF-8"),
            "--out",
            out.to_str().expect("out path should be valid UTF-8"),
            "--profile",
            "authored",
        ])
        .assert()
        .success();

    let plan_path = out.join("github.activity.plan.json");
    let plan: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&plan_path)?)?;
    assert_github_activity_plan_schema_contract(&plan);
    let activity_id = plan["activity_id"]
        .as_str()
        .expect("plan should include activity_id");
    std::fs::write(
        out.join("github.activity.progress.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "schema_version": "github.activity.progress.v1",
            "generated_at": "2026-05-19T00:00:00Z",
            "shiplog_version": env!("CARGO_PKG_VERSION"),
            "activity_id": activity_id,
            "plan_ref": "github.activity.plan.json",
            "actor": "octocat",
            "repo_owners": ["acme"],
            "profile": "authored",
            "state": "completed",
            "completed_windows": ["2025-01"],
            "pending_windows": [],
            "active_window": null,
            "stop_reason": null,
            "budget_checkpoint": null,
            "run_ref": "run_fixture",
            "receipt_refs": [
                "github.activity.plan.json",
                "github.activity.api-ledger.json",
                "run_fixture/intake.report.json",
                "run_fixture/coverage.manifest.json"
            ]
        }))?,
    )?;
    let cache_phase = serde_json::json!({
        "fresh_hits": 0,
        "stale_hits": 0,
        "misses": 0
    });
    std::fs::write(
        out.join("github.activity.api-ledger.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "schema_version": "github.activity.api-ledger.v1",
            "generated_at": "2026-05-19T00:00:00Z",
            "shiplog_version": env!("CARGO_PKG_VERSION"),
            "activity_id": activity_id,
            "plan_ref": "github.activity.plan.json",
            "progress_ref": "github.activity.progress.json",
            "actor": "octocat",
            "repo_owners": ["acme"],
            "profile": "authored",
            "stop_reason": null,
            "github_api": {
                "requests": {
                    "search": 2,
                    "core": 1
                },
                "cache": {
                    "search_probe": cache_phase,
                    "search_page": {
                        "fresh_hits": 1,
                        "stale_hits": 0,
                        "misses": 1
                    },
                    "pull_detail": {
                        "fresh_hits": 0,
                        "stale_hits": 0,
                        "misses": 1
                    },
                    "review_page": {
                        "fresh_hits": 0,
                        "stale_hits": 0,
                        "misses": 0
                    }
                },
                "rate_limit_snapshots": [],
                "secondary_limit_events": []
            },
            "owner_filter": {
                "requested_owners": ["acme"],
                "query_strategy": "actor_search_owner_filter",
                "kept": {
                    "acme": 1
                },
                "dropped": []
            },
            "receipt_refs": [
                "github.activity.plan.json",
                "github.activity.progress.json"
            ]
        }))?,
    )?;
    let progress: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        out.join("github.activity.progress.json"),
    )?)?;
    let api_ledger: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        out.join("github.activity.api-ledger.json"),
    )?)?;
    assert_github_activity_progress_schema_contract(&progress);
    assert_github_activity_api_ledger_schema_contract(&api_ledger);
    let before = file_tree_manifest(tmp.path());

    let assert = shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .args([
            "github",
            "activity",
            "status",
            "--config",
            config.to_str().expect("config path should be valid UTF-8"),
            "--out",
            out.to_str().expect("out path should be valid UTF-8"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "GitHub activity status: Completed",
        ))
        .stdout(predicate::str::contains("Provider calls: none"))
        .stdout(predicate::str::contains("Writes: none"))
        .stdout(predicate::str::contains("- actor: octocat"))
        .stdout(predicate::str::contains("- profile: authored"))
        .stdout(predicate::str::contains("- state: completed"))
        .stdout(predicate::str::contains("- run: run_fixture"))
        .stdout(predicate::str::contains(
            "- requests: search 2/10, core 1/10",
        ))
        .stdout(predicate::str::contains(
            "pull_detail fresh 0, stale 0, misses 1",
        ))
        .stdout(predicate::str::contains("- kept owners: acme=1"))
        .stdout(predicate::str::contains(
            "shiplog github activity run --config",
        ))
        .stdout(predicate::str::contains("--profile full --resume [writes]"));
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    assert!(
        !stdout.contains("shiplog share manager"),
        "github activity status must not offer share rendering"
    );
    assert_eq!(
        before,
        file_tree_manifest(tmp.path()),
        "github activity status should only read existing receipts"
    );

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .args([
            "github",
            "activity",
            "report",
            "--config",
            config.to_str().expect("config path should be valid UTF-8"),
            "--out",
            out.to_str().expect("out path should be valid UTF-8"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "GitHub activity report: Available",
        ))
        .stdout(predicate::str::contains(
            "Provider calls: none (receipt report)",
        ))
        .stdout(predicate::str::contains("Writes:"))
        .stdout(predicate::str::contains("github.activity.report.json"))
        .stdout(predicate::str::contains("github.activity.report.md"))
        .stdout(predicate::str::contains("API Budget:"))
        .stdout(predicate::str::contains(
            "- requests: search 2/10, core 1/10",
        ))
        .stdout(predicate::str::contains("- rate-limit snapshots: 0"))
        .stdout(predicate::str::contains("- secondary-limit events: 0"))
        .stdout(predicate::str::contains("Owner Filter:"))
        .stdout(predicate::str::contains("- kept owners: acme=1"))
        .stdout(predicate::str::contains("- dropped owners: none"));

    let report_json_path = out.join("github.activity.report.json");
    let report_md_path = out.join("github.activity.report.md");
    let activity_report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report_json_path)?)?;
    assert_github_activity_report_schema_contract(&activity_report);
    assert_eq!(activity_report["state"], "completed");
    assert_eq!(activity_report["run_ref"], "run_fixture");
    assert_eq!(activity_report["github_api"]["requests"]["search"], 2);
    assert_eq!(activity_report["owner_filter"]["kept"]["acme"], 1);
    let report_md = std::fs::read_to_string(&report_md_path)?;
    assert!(report_md.contains("# GitHub Activity Report"));
    assert!(report_md.contains("- Actor: octocat"));
    assert!(report_md.contains("- State: completed"));
    assert!(report_md.contains("- Requests: search 2, core 1"));
    assert!(report_md.contains("- Kept owners: acme=1"));
    assert!(
        !report_md.contains("dummy-token"),
        "activity report markdown must not print token values"
    );
    Ok(())
}

#[test]
fn github_activity_merge_writes_final_outputs_from_completed_receipts() -> CliTestResult {
    let tmp = TempDir::new()?;
    let config = tmp.path().join("shiplog-github-full.toml");
    let out = tmp.path().join("out/github-full");
    std::fs::write(
        &config,
        r#"[shiplog]
config_version = 1

[defaults]
out = "./out/github-full"

[github_activity]
actor = "octocat"
repo_owners = ["acme"]
since = "2025-01-01"
until = "2025-02-01"
include_authored_prs = true
include_reviews = false
profile = "full"
cache_dir = "./out/github-full/.cache"

[github_activity.budget]
max_search_requests = 10
max_core_requests = 10
max_search_per_minute = 24
on_exhausted = "checkpoint_and_stop"

[sources.github]
enabled = true
user = "octocat"
mode = "created"
repo_owners = ["acme"]
"#,
    )?;

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .args([
            "github",
            "activity",
            "plan",
            "--config",
            config.to_str().expect("config path should be valid UTF-8"),
            "--out",
            out.to_str().expect("out path should be valid UTF-8"),
            "--profile",
            "full",
        ])
        .assert()
        .success();

    let plan_path = out.join("github.activity.plan.json");
    let plan: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&plan_path)?)?;
    assert_github_activity_plan_schema_contract(&plan);
    let activity_id = plan["activity_id"]
        .as_str()
        .expect("plan should include activity_id");
    let run_dir = out.join("run_fixture");
    std::fs::create_dir_all(&run_dir)?;
    std::fs::write(run_dir.join("packet.md"), "# GitHub Activity Packet\n")?;
    std::fs::write(
        run_dir.join("intake.report.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "schema_version": 1,
            "run_id": "run_fixture"
        }))?,
    )?;
    std::fs::write(
        run_dir.join("coverage.manifest.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "run_id": "run_fixture",
            "sources": ["github"]
        }))?,
    )?;
    std::fs::write(run_dir.join("ledger.events.jsonl"), "{}\n")?;
    std::fs::write(
        out.join("github.activity.progress.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "schema_version": "github.activity.progress.v1",
            "generated_at": "2026-05-19T00:00:00Z",
            "shiplog_version": env!("CARGO_PKG_VERSION"),
            "activity_id": activity_id,
            "plan_ref": "github.activity.plan.json",
            "actor": "octocat",
            "repo_owners": ["acme"],
            "profile": "full",
            "state": "completed",
            "completed_windows": ["2025-01"],
            "pending_windows": [],
            "active_window": null,
            "stop_reason": null,
            "budget_checkpoint": null,
            "run_ref": "run_fixture",
            "receipt_refs": [
                "github.activity.plan.json",
                "github.activity.api-ledger.json",
                "run_fixture/intake.report.json",
                "run_fixture/coverage.manifest.json"
            ]
        }))?,
    )?;
    let cache_phase = serde_json::json!({
        "fresh_hits": 0,
        "stale_hits": 0,
        "misses": 0
    });
    std::fs::write(
        out.join("github.activity.api-ledger.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "schema_version": "github.activity.api-ledger.v1",
            "generated_at": "2026-05-19T00:00:00Z",
            "shiplog_version": env!("CARGO_PKG_VERSION"),
            "activity_id": activity_id,
            "plan_ref": "github.activity.plan.json",
            "progress_ref": "github.activity.progress.json",
            "actor": "octocat",
            "repo_owners": ["acme"],
            "profile": "full",
            "stop_reason": null,
            "github_api": {
                "requests": {
                    "search": 2,
                    "core": 1
                },
                "cache": {
                    "search_probe": cache_phase,
                    "search_page": cache_phase,
                    "pull_detail": {
                        "fresh_hits": 0,
                        "stale_hits": 0,
                        "misses": 1
                    },
                    "review_page": cache_phase
                },
                "rate_limit_snapshots": [],
                "secondary_limit_events": []
            },
            "owner_filter": {
                "requested_owners": ["acme"],
                "query_strategy": "actor_search_owner_filter",
                "kept": {
                    "acme": 1
                },
                "dropped": []
            },
            "receipt_refs": [
                "github.activity.plan.json",
                "github.activity.progress.json"
            ]
        }))?,
    )?;
    let progress: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        out.join("github.activity.progress.json"),
    )?)?;
    let api_ledger: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        out.join("github.activity.api-ledger.json"),
    )?)?;
    assert_github_activity_progress_schema_contract(&progress);
    assert_github_activity_api_ledger_schema_contract(&api_ledger);

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .args([
            "github",
            "activity",
            "merge",
            "--config",
            config.to_str().expect("config path should be valid UTF-8"),
            "--out",
            out.to_str().expect("out path should be valid UTF-8"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("GitHub activity merge written:"))
        .stdout(predicate::str::contains(
            "Provider calls: none (receipt merge)",
        ))
        .stdout(predicate::str::contains("Share rendering: none"))
        .stdout(predicate::str::contains("activity_report:"))
        .stdout(predicate::str::contains("shiplog open packet --out"));

    let final_dir = out.join("final");
    assert!(final_dir.join("packet.md").exists(), "missing final packet");
    assert!(
        final_dir.join("intake.report.json").exists(),
        "missing final intake report"
    );
    assert!(
        final_dir.join("github.activity.api-ledger.json").exists(),
        "missing final API ledger"
    );
    let activity_report: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        final_dir.join("github.activity.report.json"),
    )?)?;
    assert_eq!(
        activity_report["schema_version"],
        "github.activity.report.v1"
    );
    assert_eq!(activity_report["state"], "completed");
    assert_eq!(activity_report["run_ref"], "run_fixture");
    assert_eq!(activity_report["owner_filter"]["kept"]["acme"], 1);
    assert_eq!(activity_report["github_api"]["requests"]["search"], 2);
    assert_github_activity_report_schema_contract(&activity_report);
    assert!(
        !final_dir.join("profiles/manager/packet.md").exists(),
        "activity merge should not render manager share artifacts"
    );
    assert!(
        !final_dir.join("profiles/public/packet.md").exists(),
        "activity merge should not render public share artifacts"
    );

    Ok(())
}

struct RecordedGithubCliServer {
    base_url: String,
    requests: Arc<Mutex<Vec<String>>>,
    handle: Option<JoinHandle<anyhow::Result<()>>>,
}

const CLI_GITHUB_FIXTURE_READY_TARGET: &str = "/__shiplog_cli_fixture_ready";

impl RecordedGithubCliServer {
    fn start(expected_requests: usize) -> anyhow::Result<Self> {
        let listener =
            TcpListener::bind("127.0.0.1:0").context("bind CLI GitHub fixture server")?;
        listener
            .set_nonblocking(true)
            .context("set CLI GitHub fixture server nonblocking")?;
        let addr = listener.local_addr()?;
        let base_url = format!("http://{addr}");
        let requests = Arc::new(Mutex::new(Vec::new()));
        let thread_requests = Arc::clone(&requests);
        let thread_base_url = base_url.clone();
        let handle = thread::spawn(move || {
            replay_cli_github_fixtures(
                listener,
                &thread_base_url,
                thread_requests,
                expected_requests,
            )
        });
        wait_for_cli_github_fixture_server(addr)?;

        Ok(Self {
            base_url,
            requests,
            handle: Some(handle),
        })
    }

    fn base_url(&self) -> String {
        self.base_url.clone()
    }

    fn finish(mut self) -> anyhow::Result<Vec<String>> {
        if let Some(handle) = self.handle.take() {
            handle
                .join()
                .map_err(|_| anyhow::anyhow!("CLI GitHub fixture server thread panicked"))??;
        }
        self.requests
            .lock()
            .map_err(|_| anyhow::anyhow!("CLI GitHub fixture request log was poisoned"))
            .map(|requests| requests.clone())
    }
}

fn replay_cli_github_fixtures(
    listener: TcpListener,
    base_url: &str,
    requests: Arc<Mutex<Vec<String>>>,
    expected_requests: usize,
) -> anyhow::Result<()> {
    let deadline = Instant::now() + StdDuration::from_secs(10);

    while cli_fixture_request_count(&requests)? < expected_requests {
        match listener.accept() {
            Ok((mut stream, _peer)) => {
                stream
                    .set_nonblocking(false)
                    .context("set CLI GitHub fixture stream blocking")?;
                stream
                    .set_read_timeout(Some(StdDuration::from_secs(5)))
                    .context("set CLI GitHub fixture read timeout")?;
                stream
                    .set_write_timeout(Some(StdDuration::from_secs(5)))
                    .context("set CLI GitHub fixture write timeout")?;
                if let Some(request_line) =
                    handle_cli_github_fixture_request(&mut stream, base_url)?
                {
                    requests
                        .lock()
                        .map_err(|_| {
                            anyhow::anyhow!("CLI GitHub fixture request log was poisoned")
                        })?
                        .push(request_line);
                }
            }
            Err(err) if err.kind() == ErrorKind::WouldBlock => {
                if Instant::now() > deadline {
                    return Err(anyhow::anyhow!(
                        "CLI GitHub fixture server expected {expected_requests} requests, saw {}",
                        cli_fixture_request_count(&requests)?
                    ));
                }
                thread::sleep(StdDuration::from_millis(10));
            }
            Err(err) => return Err(err).context("accept CLI GitHub fixture request"),
        }
    }

    Ok(())
}

fn wait_for_cli_github_fixture_server(addr: SocketAddr) -> anyhow::Result<()> {
    let deadline = Instant::now() + StdDuration::from_secs(5);
    loop {
        match TcpStream::connect(addr) {
            Ok(mut stream) => {
                let request = format!(
                    "GET {CLI_GITHUB_FIXTURE_READY_TARGET} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n"
                );
                let probe_result = stream
                    .write_all(request.as_bytes())
                    .and_then(|()| stream.flush())
                    .and_then(|()| {
                        let mut response = Vec::new();
                        stream.read_to_end(&mut response).map(|_| ())
                    });
                match probe_result {
                    Ok(()) => return Ok(()),
                    Err(err)
                        if matches!(
                            err.kind(),
                            ErrorKind::BrokenPipe
                                | ErrorKind::ConnectionAborted
                                | ErrorKind::ConnectionReset
                                | ErrorKind::Interrupted
                                | ErrorKind::TimedOut
                                | ErrorKind::WouldBlock
                        ) =>
                    {
                        if Instant::now() > deadline {
                            return Err(err).context("probe CLI GitHub fixture server");
                        }
                        thread::sleep(StdDuration::from_millis(10));
                    }
                    Err(err) => return Err(err).context("probe CLI GitHub fixture server"),
                }
            }
            Err(err)
                if matches!(
                    err.kind(),
                    ErrorKind::ConnectionRefused
                        | ErrorKind::Interrupted
                        | ErrorKind::TimedOut
                        | ErrorKind::WouldBlock
                ) =>
            {
                if Instant::now() > deadline {
                    return Err(err).context("connect CLI GitHub fixture server");
                }
                thread::sleep(StdDuration::from_millis(10));
            }
            Err(err) => return Err(err).context("connect CLI GitHub fixture server"),
        }
    }
}

fn cli_fixture_request_count(requests: &Arc<Mutex<Vec<String>>>) -> anyhow::Result<usize> {
    requests
        .lock()
        .map_err(|_| anyhow::anyhow!("CLI GitHub fixture request log was poisoned"))
        .map(|requests| requests.len())
}

fn handle_cli_github_fixture_request(
    stream: &mut TcpStream,
    base_url: &str,
) -> anyhow::Result<Option<String>> {
    let mut buf = [0_u8; 4096];
    let mut received = Vec::new();
    loop {
        let n = match stream.read(&mut buf) {
            Ok(n) => n,
            Err(err)
                if received.is_empty()
                    && matches!(
                        err.kind(),
                        ErrorKind::ConnectionAborted | ErrorKind::ConnectionReset
                    ) =>
            {
                return Ok(None);
            }
            Err(err) => return Err(err).context("read CLI GitHub fixture request"),
        };
        if n == 0 {
            if received.is_empty() {
                return Ok(None);
            }
            break;
        }
        received.extend_from_slice(&buf[..n]);
        if received.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
        if received.len() > 64 * 1024 {
            return Err(anyhow::anyhow!("CLI GitHub fixture request was too large"));
        }
    }

    let request = String::from_utf8_lossy(&received);
    let request_line = request
        .lines()
        .next()
        .ok_or_else(|| anyhow::anyhow!("CLI GitHub fixture request had no request line"))?
        .to_string();
    let target = request_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("CLI GitHub fixture request had no target"))?;
    if target == CLI_GITHUB_FIXTURE_READY_TARGET {
        let response = "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        stream
            .write_all(response.as_bytes())
            .context("write CLI GitHub fixture readiness response")?;
        stream
            .flush()
            .context("flush CLI GitHub fixture readiness response")?;
        return Ok(None);
    }

    let (status, body) = cli_github_fixture_response(target, base_url);
    let rate_resource = if target.starts_with("/search/issues?") {
        "search"
    } else {
        "core"
    };
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nX-RateLimit-Limit: 30\r\nX-RateLimit-Remaining: 29\r\nX-RateLimit-Used: 1\r\nX-RateLimit-Reset: 1767225600\r\nX-RateLimit-Resource: {rate_resource}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(response.as_bytes())
        .context("write CLI GitHub fixture response")?;
    stream
        .flush()
        .context("flush CLI GitHub fixture response")?;
    Ok(Some(request_line))
}

fn cli_github_fixture_response(target: &str, base_url: &str) -> (&'static str, String) {
    let body = if target.starts_with("/search/issues?")
        && cli_target_has_query_param(target, "per_page", "1")
    {
        include_str!("fixtures/github-warm-rerun/search_meta.json")
    } else if target.starts_with("/search/issues?")
        && cli_target_has_query_param(target, "per_page", "100")
    {
        include_str!("fixtures/github-warm-rerun/search_items.json")
    } else if target == "/repos/acme/widgets/pulls/1" {
        include_str!("fixtures/github-warm-rerun/pr_details.json")
    } else {
        r#"{"message":"unexpected CLI GitHub fixture request"}"#
    };
    let status = if body.contains("unexpected CLI GitHub fixture request") {
        "404 Not Found"
    } else {
        "200 OK"
    };
    (status, body.replace("__API_BASE__", base_url))
}

fn cli_target_has_query_param(target: &str, key: &str, value: &str) -> bool {
    target
        .split_once('?')
        .map(|(_path, query)| {
            query.split('&').any(|pair| {
                pair.split_once('=')
                    .map(|(k, v)| k == key && v == value)
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

#[test]
fn github_activity_scout_writes_checkpoint_progress_on_budget_stop() -> CliTestResult {
    let tmp = TempDir::new()?;
    let config = tmp.path().join("shiplog-github-full.toml");
    let out = tmp.path().join("out/github-full");
    std::fs::write(
        &config,
        r#"[shiplog]
config_version = 1

[defaults]
out = "./out/github-full"

[github_activity]
actor = "EffortlessSteven"
repo_owners = ["EffortlessMetrics"]
since = "2026-01-01"
until = "2026-02-01"
include_authored_prs = true
profile = "scout"
cache_dir = "./out/github-full/.cache"

[github_activity.budget]
max_search_requests = 0
max_core_requests = 0
max_search_per_minute = 24
on_exhausted = "checkpoint_and_stop"

[sources.github]
enabled = true
user = "EffortlessSteven"
mode = "created"
"#,
    )?;

    shiplog_cmd()
        .current_dir(tmp.path())
        .env("GITHUB_TOKEN", "dummy-token-never-used")
        .args([
            "github",
            "activity",
            "scout",
            "--config",
            config.to_str().expect("config path should be valid UTF-8"),
            "--out",
            out.to_str().expect("out path should be valid UTF-8"),
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains(
            "GitHub activity scout checkpointed",
        ))
        .stderr(predicate::str::contains("budget exhausted"));

    let progress_path = out.join("github.activity.progress.json");
    let progress: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&progress_path)?)?;
    assert_eq!(progress["schema_version"], "github.activity.progress.v1");
    assert_github_activity_progress_schema_contract(&progress);
    assert_eq!(progress["profile"], "scout");
    assert_eq!(progress["state"], "checkpointed");
    assert_eq!(progress["stop_reason"], "budget_exhausted");
    assert_eq!(progress["budget_checkpoint"]["search_requests"], 0);
    assert_eq!(progress["budget_checkpoint"]["core_requests"], 0);
    assert_eq!(progress["receipt_refs"][0], "github.activity.plan.json");
    assert_eq!(
        progress["pending_windows"][0],
        serde_json::Value::String("2026-01".to_string())
    );
    assert!(
        out.join("github.activity.plan.json").exists(),
        "scout should keep the plan receipt beside progress"
    );
    let api_ledger_path = out.join("github.activity.api-ledger.json");
    let api_ledger: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&api_ledger_path)?)?;
    assert_eq!(
        api_ledger["schema_version"],
        "github.activity.api-ledger.v1"
    );
    assert_github_activity_api_ledger_schema_contract(&api_ledger);
    assert_eq!(api_ledger["profile"], "scout");
    assert_eq!(api_ledger["stop_reason"], "budget_exhausted");
    assert_eq!(api_ledger["github_api"]["requests"]["search"], 0);
    assert_eq!(api_ledger["github_api"]["requests"]["core"], 0);
    assert_eq!(
        api_ledger["github_api"]["secondary_limit_events"]
            .as_array()
            .map(Vec::len),
        Some(0)
    );
    assert_eq!(
        api_ledger["owner_filter"]["requested_owners"][0],
        "EffortlessMetrics"
    );
    assert_eq!(api_ledger["receipt_refs"][0], "github.activity.plan.json");
    assert!(
        !out.join("packet.md").exists(),
        "checkpointed scout should not render a packet"
    );

    Ok(())
}

#[test]
fn github_activity_run_resume_skips_completed_profile_without_refetching_details() -> CliTestResult
{
    let tmp = TempDir::new()?;
    let server = RecordedGithubCliServer::start(3)?;
    let config = tmp.path().join("shiplog-github-full.toml");
    let out = tmp.path().join("out/github-full");
    std::fs::write(
        &config,
        format!(
            r#"[shiplog]
config_version = 1

[defaults]
out = "./out/github-full"

[github_activity]
actor = "octocat"
repo_owners = ["acme"]
since = "2025-01-01"
until = "2025-02-01"
include_authored_prs = true
include_reviews = false
profile = "authored"
cache_dir = "./out/github-full/.cache"

[github_activity.budget]
max_search_requests = 10
max_core_requests = 10
max_search_per_minute = 24
on_exhausted = "checkpoint_and_stop"

[sources.github]
enabled = true
user = "octocat"
mode = "created"
repo_owners = ["acme"]
include_reviews = false
no_details = false
api_base = "{}"
cache_dir = "./out/github-full/.cache"
"#,
            server.base_url()
        ),
    )?;

    shiplog_cmd()
        .current_dir(tmp.path())
        .env("GITHUB_TOKEN", "dummy-token")
        .args([
            "github",
            "activity",
            "run",
            "--config",
            config.to_str().expect("config path should be valid UTF-8"),
            "--out",
            out.to_str().expect("out path should be valid UTF-8"),
            "--profile",
            "authored",
            "--resume",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "GitHub activity authored completed.",
        ))
        .stdout(predicate::str::contains(
            "Next: shiplog github activity run --out",
        ))
        .stdout(predicate::str::contains("--profile full --resume [writes]"));

    let requests = server.finish()?;
    assert_eq!(
        requests
            .iter()
            .filter(|line| line.contains("/search/issues?"))
            .count(),
        2,
        "first authored run should make search meta and search page requests"
    );
    assert_eq!(
        requests
            .iter()
            .filter(|line| line.contains("/repos/acme/widgets/pulls/1"))
            .count(),
        1,
        "first authored run should fetch PR details once"
    );

    let progress_path = out.join("github.activity.progress.json");
    let api_ledger_path = out.join("github.activity.api-ledger.json");
    let progress_before = std::fs::read_to_string(&progress_path)?;
    let api_ledger_before = std::fs::read_to_string(&api_ledger_path)?;
    let progress: serde_json::Value = serde_json::from_str(&progress_before)?;
    let api_ledger: serde_json::Value = serde_json::from_str(&api_ledger_before)?;
    assert_eq!(progress["schema_version"], "github.activity.progress.v1");
    assert_github_activity_progress_schema_contract(&progress);
    assert_github_activity_api_ledger_schema_contract(&api_ledger);
    assert_eq!(progress["profile"], "authored");
    assert_eq!(progress["state"], "completed");
    assert!(progress["run_ref"].as_str().is_some());
    assert_eq!(
        api_ledger["schema_version"],
        "github.activity.api-ledger.v1"
    );
    assert_eq!(api_ledger["profile"], "authored");
    assert_eq!(api_ledger["github_api"]["requests"]["search"], 2);
    assert_eq!(api_ledger["github_api"]["requests"]["core"], 1);
    assert_eq!(
        api_ledger["github_api"]["cache"]["pull_detail"]["misses"],
        1
    );

    // The fixture server has been shut down. If `--resume` refetches search
    // pages or PR details instead of trusting completed progress plus the API
    // ledger, this command will try the dead API base and fail.
    shiplog_cmd()
        .current_dir(tmp.path())
        .env("GITHUB_TOKEN", "dummy-token")
        .args([
            "github",
            "activity",
            "run",
            "--config",
            config.to_str().expect("config path should be valid UTF-8"),
            "--out",
            out.to_str().expect("out path should be valid UTF-8"),
            "--profile",
            "authored",
            "--resume",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "GitHub activity authored already completed.",
        ))
        .stdout(predicate::str::contains("Provider calls: none (--resume)"));

    assert_eq!(
        std::fs::read_to_string(&progress_path)?,
        progress_before,
        "completed resume should not rewrite progress"
    );
    assert_eq!(
        std::fs::read_to_string(&api_ledger_path)?,
        api_ledger_before,
        "completed resume should not rewrite the API ledger"
    );

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .args([
            "github",
            "activity",
            "report",
            "--config",
            config.to_str().expect("config path should be valid UTF-8"),
            "--out",
            out.to_str().expect("out path should be valid UTF-8"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "GitHub activity report: Available",
        ))
        .stdout(predicate::str::contains(
            "Provider calls: none (receipt report)",
        ))
        .stdout(predicate::str::contains("- kept owners: acme=1"));

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .args([
            "github",
            "activity",
            "merge",
            "--config",
            config.to_str().expect("config path should be valid UTF-8"),
            "--out",
            out.to_str().expect("out path should be valid UTF-8"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("GitHub activity merge written:"))
        .stdout(predicate::str::contains(
            "Provider calls: none (receipt merge)",
        ))
        .stdout(predicate::str::contains("Share rendering: none"));

    let final_dir = out.join("final");
    assert!(final_dir.join("packet.md").exists(), "missing final packet");
    assert!(
        !final_dir.join("intake.report.json").exists(),
        "activity merge should not invent an intake report when the activity run did not produce one"
    );
    assert!(
        final_dir.join("github.activity.api-ledger.json").exists(),
        "missing final API ledger"
    );
    assert!(
        final_dir.join("github.activity.report.json").exists(),
        "missing final activity report"
    );
    let activity_report: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        final_dir.join("github.activity.report.json"),
    )?)?;
    assert_github_activity_report_schema_contract(&activity_report);
    assert!(
        !final_dir.join("profiles/manager/packet.md").exists(),
        "activity merge should not render manager share artifacts"
    );
    Ok(())
}

#[test]
fn github_activity_run_resume_skips_completed_windows_without_refetching() -> CliTestResult {
    let tmp = TempDir::new()?;
    let config = tmp.path().join("shiplog-github-full.toml");
    let out = tmp.path().join("out/github-full");
    let write_config = |api_base: &str| -> CliTestResult {
        std::fs::write(
            &config,
            format!(
                r#"[shiplog]
config_version = 1

[defaults]
out = "./out/github-full"

[github_activity]
actor = "octocat"
repo_owners = ["acme"]
since = "2025-01-01"
until = "2025-03-01"
include_authored_prs = true
include_reviews = false
profile = "authored"
cache_dir = "./out/github-full/.cache"

[github_activity.budget]
max_search_requests = 20
max_core_requests = 20
max_search_per_minute = 24
on_exhausted = "checkpoint_and_stop"

[sources.github]
enabled = true
user = "octocat"
mode = "created"
repo_owners = ["acme"]
include_reviews = false
no_details = false
api_base = "{}"
cache_dir = "./out/github-full/.cache"
"#,
                api_base
            ),
        )?;
        Ok(())
    };
    write_config("http://127.0.0.1:1")?;

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .args([
            "github",
            "activity",
            "plan",
            "--config",
            config.to_str().expect("config path should be valid UTF-8"),
            "--out",
            out.to_str().expect("out path should be valid UTF-8"),
            "--profile",
            "authored",
        ])
        .assert()
        .success();

    let plan_path = out.join("github.activity.plan.json");
    let plan: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&plan_path)?)?;
    assert_github_activity_plan_schema_contract(&plan);
    let activity_id = plan["activity_id"]
        .as_str()
        .expect("plan should include activity_id");

    let jan_window = TimeWindow {
        since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
    };
    let jan_dir = out.join("github.activity.windows/authored/2025-01");
    std::fs::create_dir_all(&jan_dir)?;
    write_events_jsonl(
        &jan_dir.join("ledger.events.jsonl"),
        &[fixture_pr_event(
            SourceSystem::Github,
            "acme/widgets",
            99,
            "Already harvested January PR",
            3,
        )],
    );
    write_coverage_manifest(
        &jan_dir.join("coverage.manifest.json"),
        &CoverageManifest {
            run_id: RunId("github_activity_window_jan".to_string()),
            generated_at: Utc.with_ymd_and_hms(2026, 5, 19, 0, 0, 0).unwrap(),
            user: "octocat".to_string(),
            window: jan_window.clone(),
            mode: "created".to_string(),
            sources: vec!["github".to_string()],
            slices: vec![CoverageSlice {
                window: jan_window,
                query: "is:pr author:octocat created:2025-01-01..2025-01-31".to_string(),
                total_count: 1,
                fetched: 1,
                incomplete_results: Some(false),
                notes: vec![
                    "owner_filter:requested=acme".to_string(),
                    "owner_filter:kept=acme=1".to_string(),
                    "owner_filter:dropped=none".to_string(),
                ],
            }],
            warnings: Vec::new(),
            completeness: Completeness::Complete,
        },
    );
    std::fs::write(jan_dir.join("freshness.json"), "[]\n")?;

    let cache_phase = serde_json::json!({
        "fresh_hits": 0,
        "stale_hits": 0,
        "misses": 0
    });
    std::fs::write(
        out.join("github.activity.progress.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "schema_version": "github.activity.progress.v1",
            "generated_at": "2026-05-19T00:00:00Z",
            "shiplog_version": env!("CARGO_PKG_VERSION"),
            "activity_id": activity_id,
            "plan_ref": "github.activity.plan.json",
            "actor": "octocat",
            "repo_owners": ["acme"],
            "profile": "authored",
            "state": "checkpointed",
            "completed_windows": ["2025-01"],
            "pending_windows": ["2025-02"],
            "active_window": {
                "window_id": "2025-02",
                "query_kind": null
            },
            "stop_reason": "budget_exhausted",
            "budget_checkpoint": {
                "search_requests": 2,
                "core_requests": 1
            },
            "run_ref": null,
            "receipt_refs": [
                "github.activity.plan.json",
                "github.activity.api-ledger.json"
            ]
        }))?,
    )?;
    std::fs::write(
        out.join("github.activity.api-ledger.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "schema_version": "github.activity.api-ledger.v1",
            "generated_at": "2026-05-19T00:00:00Z",
            "shiplog_version": env!("CARGO_PKG_VERSION"),
            "activity_id": activity_id,
            "plan_ref": "github.activity.plan.json",
            "progress_ref": "github.activity.progress.json",
            "actor": "octocat",
            "repo_owners": ["acme"],
            "profile": "authored",
            "stop_reason": "budget_exhausted",
            "github_api": {
                "requests": {
                    "search": 2,
                    "core": 1
                },
                "cache": {
                    "search_probe": {
                        "fresh_hits": 0,
                        "stale_hits": 0,
                        "misses": 1
                    },
                    "search_page": {
                        "fresh_hits": 0,
                        "stale_hits": 0,
                        "misses": 1
                    },
                    "pull_detail": {
                        "fresh_hits": 0,
                        "stale_hits": 0,
                        "misses": 1
                    },
                    "review_page": cache_phase
                },
                "rate_limit_snapshots": [],
                "secondary_limit_events": []
            },
            "owner_filter": {
                "requested_owners": ["acme"],
                "query_strategy": "actor_search_owner_filter",
                "kept": {
                    "acme": 1
                },
                "dropped": []
            },
            "receipt_refs": [
                "github.activity.plan.json",
                "github.activity.progress.json"
            ]
        }))?,
    )?;

    let server = RecordedGithubCliServer::start(3)?;
    write_config(&server.base_url())?;

    shiplog_cmd()
        .current_dir(tmp.path())
        .env("GITHUB_TOKEN", "dummy-token")
        .args([
            "github",
            "activity",
            "run",
            "--config",
            config.to_str().expect("config path should be valid UTF-8"),
            "--out",
            out.to_str().expect("out path should be valid UTF-8"),
            "--profile",
            "authored",
            "--resume",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "GitHub activity authored completed.",
        ));

    let requests = server.finish()?;
    assert_eq!(
        requests
            .iter()
            .filter(|line| line.contains("/search/issues?"))
            .count(),
        2,
        "resume should search only the pending February window"
    );
    assert!(
        requests.iter().all(|line| !line.contains("2025-01-01")),
        "resume should not refetch the completed January window: {requests:?}"
    );
    assert!(
        requests.iter().any(|line| line.contains("2025-02-01")),
        "resume should fetch the pending February window: {requests:?}"
    );

    let progress_path = out.join("github.activity.progress.json");
    let progress: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&progress_path)?)?;
    assert_github_activity_progress_schema_contract(&progress);
    assert_eq!(progress["state"], "completed");
    assert_eq!(
        progress["completed_windows"].as_array().map(Vec::len),
        Some(2)
    );
    assert_eq!(
        progress["pending_windows"].as_array().map(Vec::len),
        Some(0)
    );
    let run_ref = progress["run_ref"]
        .as_str()
        .expect("completed activity should write run_ref");
    assert!(
        out.join(run_ref)
            .join("github.activity.api-ledger.json")
            .exists(),
        "completed activity run should carry the API ledger receipt"
    );

    let api_ledger: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        out.join("github.activity.api-ledger.json"),
    )?)?;
    assert_github_activity_api_ledger_schema_contract(&api_ledger);
    assert_eq!(api_ledger["stop_reason"], serde_json::Value::Null);
    assert_eq!(api_ledger["github_api"]["requests"]["search"], 4);
    assert_eq!(api_ledger["github_api"]["requests"]["core"], 2);

    Ok(())
}

#[test]
fn github_activity_plan_accepts_sources_github_user_alias() -> CliTestResult {
    let tmp = TempDir::new()?;
    let config = tmp.path().join("shiplog.toml");
    std::fs::write(
        &config,
        r#"[shiplog]
config_version = 1

[sources.github]
enabled = true
user = "EffortlessSteven"
mode = "created"

[github_activity]
repo_owners = ["EffortlessMetrics"]
since = "2026-01-01"
until = "2026-02-01"
profile = "authored"
"#,
    )?;

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .args(["github", "activity", "plan"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Actor: EffortlessSteven"))
        .stdout(predicate::str::contains("Profile: authored"));

    let plan_path = tmp.path().join("out/github.activity.plan.json");
    let plan: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&plan_path)?)?;

    assert_github_activity_plan_schema_contract(&plan);
    assert_eq!(plan["actor"], "EffortlessSteven");
    assert_eq!(plan["profile"], "authored");
    assert_eq!(plan["windows"][0]["query_kinds"][0], "authored_prs");
    assert_eq!(plan["estimated_totals"]["search_requests"], 11);
    assert_eq!(plan["estimated_totals"]["core_requests"], 1000);
    assert_eq!(plan["estimated_totals"]["review_requests"], 0);

    Ok(())
}

#[test]
fn status_latest_json_missing_setup_serializes_model_without_writing() -> CliTestResult {
    let tmp = TempDir::new()?;
    let before = file_tree_manifest(tmp.path());

    let assert = shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("SHIPLOG_REDACT_KEY")
        .args(["status", "--latest", "--json"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let json: serde_json::Value = serde_json::from_str(&stdout)?;

    assert_eq!(json["overall_status"], "needs_setup");
    assert_eq!(json["setup_summary"]["status"], "needs_setup");
    assert!(json.get("latest_run").is_some());
    assert_eq!(json["latest_run"], serde_json::Value::Null);
    assert!(
        !stdout.contains("Review loop status:"),
        "json output should not include human status prose"
    );
    let actions = json["next_actions"]
        .as_array()
        .expect("next_actions should be an array");
    assert!(actions.iter().any(|action| {
        action["key"] == "doctor_setup"
            && action["command"] == "shiplog doctor --setup"
            && action["writes"] == false
    }));
    assert!(actions.iter().any(|action| {
        action["key"] == "init_guided"
            && action["command"] == "shiplog init --guided"
            && action["writes"] == true
    }));
    assert!(
        json["receipt_refs"]
            .as_array()
            .is_some_and(|refs| !refs.is_empty()),
        "status json should name receipt refs"
    );
    assert_eq!(
        before,
        file_tree_manifest(tmp.path()),
        "status --json should not write setup or run artifacts"
    );
    Ok(())
}

#[test]
fn status_latest_json_repairable_run_exposes_safe_agent_actions() -> CliTestResult {
    let tmp = TempDir::new()?;
    let out = tmp.path().join("out");
    let out_arg = out.to_str().expect("out path should be valid UTF-8");

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["init", "--guided"])
        .assert()
        .success();
    run_guided_setup_intake_2025_with_sources(tmp.path(), &out, &[]);

    let before = file_tree_manifest(tmp.path());
    let assert = shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("SHIPLOG_REDACT_KEY")
        .args(["status", "--out", out_arg, "--latest", "--json"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let json: serde_json::Value = serde_json::from_str(&stdout)?;

    assert_eq!(json["overall_status"], "needs_repair");
    assert!(
        json["repair_summary"]["open_items"]
            .as_u64()
            .unwrap_or_default()
            > 0
    );
    assert_eq!(json["share_summary"]["profiles"][0]["status"], "blocked");
    assert!(
        !stdout.contains("shiplog share manager --"),
        "json next actions should not include share render while share is blocked"
    );
    let actions = json["next_actions"]
        .as_array()
        .expect("next_actions should be an array");
    let repair_plan_index = actions
        .iter()
        .position(|action| action["key"] == "repair_plan")
        .expect("repair_plan action should exist");
    let journal_index = actions
        .iter()
        .position(|action| action["key"] == "journal_add_from_repair")
        .expect("journal repair action should exist");
    assert!(
        repair_plan_index < journal_index,
        "repair plan should precede write-producing journal repair"
    );
    assert_eq!(actions[repair_plan_index]["writes"], false);
    assert_eq!(actions[journal_index]["writes"], true);
    assert!(
        actions[journal_index]["preconditions"]
            .as_array()
            .is_some_and(|preconditions| !preconditions.is_empty()),
        "write-producing repair should carry preconditions"
    );
    assert!(
        json["receipt_refs"].as_array().is_some_and(|refs| refs
            .iter()
            .any(|receipt| receipt["kind"] == "intake_report")),
        "status json should include intake_report receipt refs"
    );
    assert_eq!(
        before,
        file_tree_manifest(tmp.path()),
        "status --json should be read-only after intake"
    );
    Ok(())
}

#[test]
fn status_latest_repairable_run_is_read_first_and_share_safe() -> CliTestResult {
    let tmp = TempDir::new()?;
    let out = tmp.path().join("out");
    let out_arg = out.to_str().expect("out path should be valid UTF-8");

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["init", "--guided"])
        .assert()
        .success();
    run_guided_setup_intake_2025_with_sources(tmp.path(), &out, &[]);

    let before = file_tree_manifest(tmp.path());
    let assert = shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("SHIPLOG_REDACT_KEY")
        .args(["status", "--out", out_arg, "--latest"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Review loop status: Needs repair"))
        .stdout(predicate::str::contains("Repair:"))
        .stdout(predicate::str::contains("open items:"))
        .stdout(predicate::str::contains("safe writes:"))
        .stdout(predicate::str::contains("shiplog repair plan --out"))
        .stdout(predicate::str::contains("[read-only]"))
        .stdout(predicate::str::contains(
            "shiplog journal add --from-repair <repair_id> --out",
        ))
        .stdout(predicate::str::contains("[writes]"))
        .stdout(predicate::str::contains("Receipts:"))
        .stdout(predicate::str::contains("intake_report"));
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;

    let repair_plan_index = stdout
        .find("shiplog repair plan")
        .expect("status should offer repair plan");
    let journal_index = stdout
        .find("shiplog journal add --from-repair")
        .expect("status should offer journal repair when safe");
    assert!(
        repair_plan_index < journal_index,
        "repair plan should appear before write-producing repair"
    );
    assert!(
        !stdout.contains("shiplog share manager --"),
        "status should not offer share render while share is blocked"
    );
    assert_eq!(
        before,
        file_tree_manifest(tmp.path()),
        "status should not write after intake"
    );
    Ok(())
}

#[test]
fn status_latest_json_safe_next_actions_cover_review_loop_states() -> CliTestResult {
    let missing_setup = TempDir::new()?;
    let missing_out = missing_setup.path().join("out");
    let before_missing = file_tree_manifest(missing_setup.path());
    let missing = status_latest_json(missing_setup.path(), &missing_out)?;
    assert_eq!(missing["overall_status"], "needs_setup");
    assert_status_first_action(&missing, "doctor_setup", false);
    assert_status_has_action(&missing, "init_guided", true);
    assert_status_lacks_command(&missing, "journal add --from-repair");
    assert_status_lacks_share_render(&missing);
    assert_eq!(
        before_missing,
        file_tree_manifest(missing_setup.path()),
        "missing-setup status should stay read-only"
    );

    let setup_blocked = TempDir::new()?;
    let blocked_out = setup_blocked.path().join("out");
    shiplog_cmd()
        .current_dir(setup_blocked.path())
        .args(["init", "--guided"])
        .assert()
        .success();
    std::fs::write(
        setup_blocked.path().join("manual_events.yaml"),
        "events: []\n",
    )?;
    let before_blocked = file_tree_manifest(setup_blocked.path());
    let blocked = status_latest_json(setup_blocked.path(), &blocked_out)?;
    assert_eq!(blocked["overall_status"], "blocked");
    assert_status_first_action(&blocked, "doctor_setup", false);
    assert_status_lacks_command(&blocked, "journal add --from-repair");
    assert_status_lacks_share_render(&blocked);
    assert_eq!(
        before_blocked,
        file_tree_manifest(setup_blocked.path()),
        "setup-blocked status should stay read-only"
    );

    let no_run = TempDir::new()?;
    let no_run_out = no_run.path().join("out");
    shiplog_cmd()
        .current_dir(no_run.path())
        .args(["init", "--guided"])
        .assert()
        .success();
    let before_no_run = file_tree_manifest(no_run.path());
    let ready_to_collect = status_latest_json(no_run.path(), &no_run_out)?;
    assert_eq!(ready_to_collect["overall_status"], "ready_to_collect");
    assert_status_first_action(&ready_to_collect, "intake", true);
    assert_status_lacks_command(&ready_to_collect, "journal add --from-repair");
    assert_status_lacks_share_render(&ready_to_collect);
    assert_eq!(
        before_no_run,
        file_tree_manifest(no_run.path()),
        "ready-to-collect status should stay read-only"
    );

    let repairable = TempDir::new()?;
    let repairable_out = repairable.path().join("out");
    let repairable_out_arg = repairable_out.to_string_lossy().to_string();
    shiplog_cmd()
        .current_dir(repairable.path())
        .args(["init", "--guided"])
        .assert()
        .success();
    run_guided_setup_intake_2025_with_sources(repairable.path(), &repairable_out, &[]);
    let (_, report) = load_latest_intake_report(&repairable_out);
    let repair_id = first_repair_id_with_action(&report, "journal_add");
    let before_repairable = file_tree_manifest(repairable.path());
    let needs_repair = status_latest_json(repairable.path(), &repairable_out)?;
    assert_eq!(needs_repair["overall_status"], "needs_repair");
    assert_status_first_action(&needs_repair, "repair_plan", false);
    assert_status_has_action(&needs_repair, "journal_add_from_repair", true);
    assert_status_lacks_share_render(&needs_repair);
    assert_eq!(
        before_repairable,
        file_tree_manifest(repairable.path()),
        "repairable status should stay read-only"
    );

    std::thread::sleep(std::time::Duration::from_millis(20));
    shiplog_cmd()
        .current_dir(repairable.path())
        .args([
            "journal",
            "add",
            "--from-repair",
            repair_id.as_str(),
            "--out",
            repairable_out_arg.as_str(),
            "--latest",
        ])
        .assert()
        .success();
    let before_rerun = file_tree_manifest(repairable.path());
    let repair_in_progress = status_latest_json(repairable.path(), &repairable_out)?;
    assert_eq!(repair_in_progress["overall_status"], "repair_in_progress");
    assert_status_first_action(&repair_in_progress, "intake", true);
    assert_status_lacks_command(&repair_in_progress, "journal add --from-repair");
    assert_status_lacks_share_render(&repair_in_progress);
    assert_eq!(
        before_rerun,
        file_tree_manifest(repairable.path()),
        "repair-in-progress status should not write while routing to rerun"
    );

    let caveated = TempDir::new()?;
    let caveated_out = caveated.path().join("out");
    let caveated_run = caveated_out.join("run_ready_caveats");
    shiplog_cmd()
        .current_dir(caveated.path())
        .args(["init", "--guided"])
        .assert()
        .success();
    std::fs::create_dir_all(&caveated_run)?;
    std::fs::write(
        caveated_run.join("intake.report.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "run_id": "run_ready_caveats",
            "included_sources": [
                {
                    "source_key": "manual",
                    "source_label": "Manual journal",
                    "event_count": 1
                }
            ],
            "skipped_sources": [],
            "repair_items": [],
            "packet_quality": {
                "packet_readiness": {
                    "status": "ready_with_caveats",
                    "summary": "manual-only packet is ready with caveats"
                }
            }
        }))?,
    )?;
    let before_caveated = file_tree_manifest(caveated.path());
    let ready_with_caveats = status_latest_json_with_redaction_key(caveated.path(), &caveated_out)?;
    assert_eq!(ready_with_caveats["overall_status"], "ready_with_caveats");
    assert_status_first_action(&ready_with_caveats, "share_explain_manager", false);
    assert_status_lacks_command(&ready_with_caveats, "journal add --from-repair");
    assert_status_lacks_share_render(&ready_with_caveats);
    assert_eq!(
        before_caveated,
        file_tree_manifest(caveated.path()),
        "ready-with-caveats status should stay read-only"
    );

    Ok(())
}

#[test]
fn status_latest_json_safe_next_actions_for_old_and_malformed_reports() -> CliTestResult {
    let old = TempDir::new()?;
    let old_out = old.path().join("out");
    let old_run = old_out.join("run_old");
    shiplog_cmd()
        .current_dir(old.path())
        .args(["init", "--guided"])
        .assert()
        .success();
    std::fs::create_dir_all(&old_run)?;
    std::fs::write(
        old_run.join("intake.report.json"),
        serde_json::to_string_pretty(&serde_json::json!({"run_id": "run_old"}))?,
    )?;
    let before_old = file_tree_manifest(old.path());
    let old_status = status_latest_json(old.path(), &old_out)?;
    assert_ne!(old_status["overall_status"], "ready_to_share");
    assert_status_first_action(&old_status, "share_explain_manager", false);
    assert_status_lacks_command(&old_status, "journal add --from-repair");
    assert_status_lacks_share_render(&old_status);
    assert_eq!(
        before_old,
        file_tree_manifest(old.path()),
        "old-report status should stay read-only"
    );

    let malformed = TempDir::new()?;
    let malformed_out = malformed.path().join("out");
    let malformed_run = malformed_out.join("run_bad");
    shiplog_cmd()
        .current_dir(malformed.path())
        .args(["init", "--guided"])
        .assert()
        .success();
    std::fs::create_dir_all(&malformed_run)?;
    std::fs::write(malformed_run.join("intake.report.json"), "not json\n")?;
    let before_malformed = file_tree_manifest(malformed.path());
    let malformed_status = status_latest_json(malformed.path(), &malformed_out)?;
    assert_ne!(malformed_status["overall_status"], "ready_to_share");
    assert_status_first_action(&malformed_status, "share_explain_manager", false);
    assert_status_lacks_command(&malformed_status, "journal add --from-repair");
    assert_status_lacks_share_render(&malformed_status);
    assert!(
        malformed_status["receipt_refs"]
            .as_array()
            .is_some_and(|refs| refs.iter().any(|receipt| {
                receipt["kind"] == "receipt_problem"
                    && receipt["field"]
                        .as_str()
                        .is_some_and(|field| field.contains("intake_report_malformed"))
            })),
        "malformed report status should name the blocked receipt problem: {malformed_status}"
    );
    assert_eq!(
        before_malformed,
        file_tree_manifest(malformed.path()),
        "malformed-report status should stay read-only"
    );

    Ok(())
}

#[test]
fn status_latest_json_agrees_with_doctor_and_sources_for_setup_blockers() -> CliTestResult {
    let tmp = TempDir::new()?;
    let out = tmp.path().join("out");
    let out_arg = out.to_string_lossy().to_string();
    write_manual_events(&tmp.path().join("manual_events.yaml"));
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[shiplog]
config_version = 1

[defaults]
out = "./out"
window = "last-6-months"
profile = "internal"

[sources.manual]
enabled = true
events = "./manual_events.yaml"

[sources.github]
enabled = true
user = "octo"
"#,
    )?;

    let doctor = shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("SHIPLOG_REDACT_KEY")
        .args(["doctor", "--setup", "--json"])
        .assert()
        .failure();
    let doctor_stdout = String::from_utf8(doctor.get_output().stdout.clone())?;
    let doctor_json: serde_json::Value = serde_json::from_str(&doctor_stdout)?;

    let sources = shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .args(["sources", "status"])
        .assert()
        .failure();
    let sources_stdout = String::from_utf8(sources.get_output().stdout.clone())?;
    let source_rows = parse_sources_status_rows(&sources_stdout);
    let github_row = source_rows
        .get("github")
        .ok_or_else(|| std::io::Error::other("sources status should include github"))?;

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args([
            "intake",
            "--out",
            out_arg.as_str(),
            "--no-open",
            "--explain",
        ])
        .assert()
        .success();

    let before = file_tree_manifest(tmp.path());
    let status_json = status_latest_json(tmp.path(), &out)?;

    assert_eq!(
        status_json["setup_summary"]["status"], doctor_json["overall_status"],
        "status setup blocker should match doctor JSON"
    );
    assert!(
        status_json["setup_summary"]["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("GitHub") && reason.contains("GITHUB_TOKEN")),
        "setup blocker should be the missing source credential, not share redaction: {status_json}"
    );
    assert_eq!(github_row.status, "unavailable");
    assert!(github_row.enabled);
    assert!(
        github_row.reason.contains("GITHUB_TOKEN not set"),
        "sources status should name missing GitHub token: {sources_stdout}"
    );
    assert!(
        status_json["source_summary"]["unavailable"]
            .as_array()
            .is_some_and(|sources| sources.iter().any(|source| {
                source["source_key"] == "github"
                    && source["reason"]
                        .as_str()
                        .is_some_and(|reason| reason.contains("GITHUB_TOKEN"))
            })),
        "status source summary should carry the GitHub blocker from receipts"
    );
    assert!(
        status_json["next_actions"]
            .as_array()
            .is_some_and(|actions| actions.iter().all(|action| !action["command"]
                .as_str()
                .unwrap_or_default()
                .contains("journal add --from-repair"))),
        "setup-blocked status should not offer evidence-repair writes"
    );
    assert_eq!(
        before,
        file_tree_manifest(tmp.path()),
        "status consistency read should not write files"
    );
    Ok(())
}

#[test]
fn status_latest_json_agrees_with_report_repair_plan_and_share_explain() -> CliTestResult {
    let tmp = TempDir::new()?;
    let out = tmp.path().join("out");
    let out_arg = out.to_string_lossy().to_string();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["init", "--guided"])
        .assert()
        .success();
    run_guided_setup_intake_2025_with_sources(tmp.path(), &out, &[]);

    let (_, report) = load_latest_intake_report(&out);
    let status_json = status_latest_json(tmp.path(), &out)?;
    let repair_plan = shiplog_cmd()
        .current_dir(tmp.path())
        .args(["repair", "plan", "--out", out_arg.as_str(), "--latest"])
        .assert()
        .success();
    let repair_plan_stdout = String::from_utf8(repair_plan.get_output().stdout.clone())?;
    let share_explain = shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("SHIPLOG_REDACT_KEY")
        .args([
            "share",
            "explain",
            "manager",
            "--out",
            out_arg.as_str(),
            "--latest",
        ])
        .assert()
        .success();
    let share_explain_stdout = String::from_utf8(share_explain.get_output().stdout.clone())?;

    let packet_readiness = &report["packet_quality"]["packet_readiness"];
    assert_eq!(
        status_json["packet_readiness"]["status"], packet_readiness["status"],
        "status packet readiness should match intake.report.json"
    );
    assert_eq!(
        status_json["packet_readiness"]["reason"], packet_readiness["summary"],
        "status packet readiness reason should match intake.report.json summary"
    );

    let repair_items = report["repair_items"]
        .as_array()
        .ok_or_else(|| std::io::Error::other("repair_items should be an array"))?;
    let safe_journal_writes = repair_items
        .iter()
        .filter(|item| item["action"]["kind"] == "journal_add")
        .count() as u64;
    assert_eq!(
        status_json["repair_summary"]["open_items"].as_u64(),
        Some(repair_items.len() as u64),
        "status repair count should match report repair_items"
    );
    assert_eq!(
        status_json["repair_summary"]["safe_write_count"].as_u64(),
        Some(safe_journal_writes),
        "status safe write count should match repair-plan action posture"
    );
    assert!(
        repair_plan_stdout.contains(&format!("Repair queue: {} item(s)", repair_items.len())),
        "repair plan should print the same repair item count. stdout:\n{repair_plan_stdout}"
    );
    if safe_journal_writes > 0 {
        assert!(
            repair_plan_stdout.contains("shiplog journal add --from-repair"),
            "repair plan should expose the same safe journal repair posture. stdout:\n{repair_plan_stdout}"
        );
    }

    let manager = status_json["share_summary"]["profiles"]
        .as_array()
        .and_then(|profiles| {
            profiles
                .iter()
                .find(|profile| profile["profile_key"] == "manager")
        })
        .ok_or_else(|| std::io::Error::other("status should include manager share profile"))?;
    assert_eq!(manager["status"], "blocked");
    assert!(
        manager["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("SHIPLOG_REDACT_KEY")),
        "status manager share blocker should name redaction key"
    );
    assert!(
        share_explain_stdout.contains("SHIPLOG_REDACT_KEY"),
        "share explain should report the same redaction-key blocker. stdout:\n{share_explain_stdout}"
    );
    assert!(
        !status_json.to_string().contains("shiplog share manager --"),
        "status should not offer share rendering while share explain reports a blocker"
    );
    Ok(())
}

#[test]
fn status_latest_json_uses_same_comparable_runs_as_repair_and_runs_diff() -> CliTestResult {
    let tmp = TempDir::new()?;
    let out = tmp.path().join("out");
    let out_arg = out.to_string_lossy().to_string();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["init", "--guided"])
        .assert()
        .success();
    run_guided_setup_intake_2025_with_sources(tmp.path(), &out, &[]);
    let (_, first_report) = load_first_intake_report(&out);
    let repair_id = first_repair_id_with_action(&first_report, "journal_add");
    shiplog_cmd()
        .current_dir(tmp.path())
        .args([
            "journal",
            "add",
            "--from-repair",
            repair_id.as_str(),
            "--out",
            out_arg.as_str(),
            "--latest",
        ])
        .assert()
        .success();
    run_guided_setup_intake_2025_with_sources(tmp.path(), &out, &[]);

    let runs = all_run_dirs(&out);
    let older = runs
        .first()
        .ok_or_else(|| std::io::Error::other("older run should exist"))?;
    let newer = runs
        .last()
        .ok_or_else(|| std::io::Error::other("newer run should exist"))?;
    let older_run_id = older
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| std::io::Error::other("older run id"))?;
    let newer_run_id = newer
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| std::io::Error::other("newer run id"))?;

    let status_json = status_latest_json(tmp.path(), &out)?;
    let repair_diff = shiplog_cmd()
        .current_dir(tmp.path())
        .args(["repair", "diff", "--out", out_arg.as_str(), "--latest"])
        .assert()
        .success();
    let repair_diff_stdout = String::from_utf8(repair_diff.get_output().stdout.clone())?;
    let runs_diff = shiplog_cmd()
        .current_dir(tmp.path())
        .args(["runs", "diff", "--out", out_arg.as_str(), "--latest"])
        .assert()
        .success();
    let runs_diff_stdout = String::from_utf8(runs_diff.get_output().stdout.clone())?;

    assert_eq!(status_json["latest_run"]["run_id"], newer_run_id);
    assert_eq!(status_json["diff_summary"]["status"], "not_generated");
    assert!(
        status_json["diff_summary"]["receipt_refs"]
            .as_array()
            .is_some_and(|refs| {
                refs.iter().any(|receipt| {
                    receipt["kind"] == "intake_report"
                        && receipt["path"]
                            .as_str()
                            .is_some_and(|path| path.contains(older_run_id))
                }) && refs.iter().any(|receipt| {
                    receipt["kind"] == "intake_report"
                        && receipt["path"]
                            .as_str()
                            .is_some_and(|path| path.contains(newer_run_id))
                })
            }),
        "status diff summary should reference the same comparable run reports"
    );
    assert!(
        repair_diff_stdout.contains(&format!("Repair diff: {older_run_id} -> {newer_run_id}")),
        "repair diff should use the same comparable run pair. stdout:\n{repair_diff_stdout}"
    );
    assert!(
        repair_diff_stdout.contains("Cleared:"),
        "repair diff should show repair movement. stdout:\n{repair_diff_stdout}"
    );
    assert!(
        runs_diff_stdout.contains(&format!(
            "Packet quality diff: {older_run_id} -> {newer_run_id}"
        )),
        "runs diff should use the same comparable run pair. stdout:\n{runs_diff_stdout}"
    );
    assert!(
        runs_diff_stdout.contains("manual evidence count 0 -> 1"),
        "runs diff should show the manual repair movement. stdout:\n{runs_diff_stdout}"
    );
    Ok(())
}

#[test]
fn guided_setup_prevents_dead_end_manual_repair_loop() -> CliTestResult {
    let tmp = TempDir::new()?;
    let out = tmp.path().join("out");
    let out_arg = out.to_str().expect("out path should be valid UTF-8");
    let manual_events = tmp.path().join("manual_events.yaml");

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["init", "--guided"])
        .assert()
        .success();

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("SHIPLOG_REDACT_KEY")
        .args(["doctor", "--setup"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Setup readiness: Needs setup"))
        .stdout(predicate::str::contains("Manager share"))
        .stdout(predicate::str::contains("Public share"))
        .stdout(predicate::str::contains("SHIPLOG_REDACT_KEY not set"));

    run_guided_setup_intake_2025_with_sources(tmp.path(), &out, &[]);
    let (_, first_report) = load_latest_intake_report(&out);
    let first_repair_id = first_repair_id_with_action(&first_report, "journal_add");
    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["repair", "plan", "--out", out_arg, "--latest"])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "shiplog journal add --from-repair {first_repair_id}"
        )));

    std::fs::write(&manual_events, "events: []\n")?;
    let before_doctor = file_tree_manifest(tmp.path());
    let doctor = shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("SHIPLOG_REDACT_KEY")
        .args(["doctor", "--setup"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Manual journal"))
        .stdout(predicate::str::contains("manual_events.yaml malformed"))
        .stdout(predicate::str::contains("shiplog doctor --setup"));
    let doctor_stdout = String::from_utf8(doctor.get_output().stdout.clone())?;
    let after_doctor = file_tree_manifest(tmp.path());
    assert!(
        !doctor_stdout.contains("journal add --from-repair"),
        "doctor should not route malformed manual setup to from-repair journal commands. stdout:\n{doctor_stdout}"
    );
    assert_eq!(
        before_doctor, after_doctor,
        "doctor --setup should stay read-only while explaining the setup block"
    );

    std::fs::write(
        &manual_events,
        "version: 1\ngenerated_at: 2026-01-01T00:00:00Z\nevents: []\n",
    )?;
    shiplog_cmd()
        .current_dir(tmp.path())
        .env("SHIPLOG_REDACT_KEY", "stable-redact-key")
        .args(["doctor", "--setup"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Setup readiness: Ready with caveats",
        ))
        .stdout(predicate::str::contains("Manual journal"))
        .stdout(predicate::str::contains("Manager share"))
        .stdout(predicate::str::contains("Public share"));

    shiplog_cmd()
        .current_dir(tmp.path())
        .args([
            "journal",
            "add",
            "--from-repair",
            first_repair_id.as_str(),
            "--out",
            out_arg,
            "--latest",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added manual event:"));

    run_guided_setup_intake_2025_with_sources(tmp.path(), &out, &[]);
    let repair_diff = shiplog_cmd()
        .current_dir(tmp.path())
        .args(["repair", "diff", "--out", out_arg, "--latest"])
        .assert()
        .success();
    let repair_diff_stdout = String::from_utf8(repair_diff.get_output().stdout.clone())?;
    assert!(
        repair_diff_stdout.contains("Cleared:")
            && repair_diff_stdout.contains("manual:manual_evidence_missing:no_events"),
        "repair diff should show the repaired manual evidence item clearing. stdout:\n{repair_diff_stdout}"
    );

    Ok(())
}

#[test]
fn init_guided_dry_run_does_not_write_files() -> CliTestResult {
    let tmp = TempDir::new()?;

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["init", "--guided", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Would write guided shiplog.toml"))
        .stdout(predicate::str::contains("[sources.manual]"))
        .stdout(predicate::str::contains("enabled = true"))
        .stdout(predicate::str::contains("[sources.github]"))
        .stdout(predicate::str::contains("enabled = false"));

    assert!(!tmp.path().join("shiplog.toml").exists());
    assert!(!tmp.path().join("manual_events.yaml").exists());
    Ok(())
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
        .stdout(predicate::str::contains("[shiplog]"))
        .stdout(predicate::str::contains("config_version = 1"))
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
    assert!(config.contains(
        "[sources.jira]\n# Set JIRA_TOKEN, JIRA_AUTH_USER, user, and instance before enabling.\nenabled = true"
    ));
    assert!(config.contains(
        "[sources.linear]\n# Set LINEAR_API_KEY and user_id before enabling.\nenabled = true"
    ));
    assert!(config.contains(
        "[sources.github]\n# Set GITHUB_TOKEN. Use either user or me = true.\nenabled = false"
    ));
    assert!(config.contains("[sources.manual]\nenabled = false"));
}

fn seed_journal_list_events(path: &Path) {
    shiplog_cmd()
        .args([
            "journal",
            "add",
            "--events",
            path.to_str().unwrap(),
            "--id",
            "manual-customer-import",
            "--date",
            "2026-05-08",
            "--title",
            "Debugged customer import incident",
            "--workstream",
            "Customer Reliability",
            "--tag",
            "support",
            "--tag",
            "review-cycle",
            "--receipt",
            "ticket=https://example.invalid/ticket/OPS-123",
        ])
        .assert()
        .success();

    shiplog_cmd()
        .args([
            "journal",
            "add",
            "--events",
            path.to_str().unwrap(),
            "--id",
            "manual-architecture-review",
            "--type",
            "design",
            "--start",
            "2026-05-01",
            "--end",
            "2026-05-03",
            "--title",
            "Reviewed architecture decision",
            "--workstream",
            "Platform Reliability",
            "--tag",
            "design",
        ])
        .assert()
        .success();
}

#[test]
fn journal_add_creates_collectable_manual_event() {
    let tmp = TempDir::new().unwrap();
    let manual_events = tmp.path().join("manual_events.yaml");

    shiplog_cmd()
        .args([
            "journal",
            "add",
            "--events",
            manual_events.to_str().unwrap(),
            "--date",
            "2026-05-08",
            "--title",
            "Debugged customer import incident",
            "--workstream",
            "Customer Reliability",
            "--impact",
            "Prevented repeat failure before the next import window",
            "--tag",
            "support",
            "--tag",
            "review-cycle",
            "--receipt",
            "ticket=https://example.invalid/ticket/OPS-123",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Added manual event: manual-2026-05-08-debugged-customer-import-incident",
        ))
        .stdout(predicate::str::contains("shiplog collect multi"));

    let file: ManualEventsFile =
        serde_yaml::from_str(&std::fs::read_to_string(&manual_events).unwrap()).unwrap();
    assert_eq!(file.version, 1);
    assert_eq!(file.events.len(), 1);
    let entry = &file.events[0];
    assert_eq!(
        entry.id,
        "manual-2026-05-08-debugged-customer-import-incident"
    );
    assert_eq!(entry.event_type, ManualEventType::Note);
    assert_eq!(
        entry.date,
        ManualDate::Single(NaiveDate::from_ymd_opt(2026, 5, 8).unwrap())
    );
    assert_eq!(entry.title, "Debugged customer import incident");
    assert_eq!(entry.workstream.as_deref(), Some("Customer Reliability"));
    assert_eq!(
        entry.impact.as_deref(),
        Some("Prevented repeat failure before the next import window")
    );
    assert_eq!(entry.tags, vec!["support", "review-cycle"]);
    assert_eq!(entry.receipts.len(), 1);
    assert_eq!(entry.receipts[0].label, "ticket");

    shiplog_cmd()
        .args([
            "collect",
            "--out",
            tmp.path().join("out").to_str().unwrap(),
            "manual",
            "--events",
            manual_events.to_str().unwrap(),
            "--user",
            "octo",
            "--since",
            "2026-05-01",
            "--until",
            "2026-06-01",
        ])
        .assert()
        .success();

    let packet =
        std::fs::read_to_string(first_run_dir(&tmp.path().join("out")).join("packet.md")).unwrap();
    assert!(packet.contains("Debugged customer import incident"));
    assert!(packet.contains("Customer Reliability"));
}

#[test]
fn journal_add_dry_run_does_not_write() {
    let tmp = TempDir::new().unwrap();
    let manual_events = tmp.path().join("manual_events.yaml");

    shiplog_cmd()
        .args([
            "journal",
            "add",
            "--events",
            manual_events.to_str().unwrap(),
            "--date",
            "2026-05-08",
            "--title",
            "Captured a dry run note",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Would add manual event"));

    assert!(
        !manual_events.exists(),
        "journal add --dry-run should not create manual_events.yaml"
    );
}

#[test]
fn journal_add_from_repair_appends_report_derived_manual_event() -> CliTestResult {
    let tmp = TempDir::new()?;
    let out = tmp.path().join("out");
    let manual_events = tmp.path().join("manual_events.yaml");
    run_intake_without_provider_tokens(tmp.path(), &out);
    let (report_path, mut report) = load_first_intake_report(&out);
    let repair_id = first_repair_id_with_action(&report, "journal_add");
    let repair_items = report["repair_items"]
        .as_array_mut()
        .expect("repair_items should be an array");
    let repair = repair_items
        .iter_mut()
        .find(|item| item["repair_id"] == repair_id)
        .expect("selected repair item should exist");
    repair["reason"] = serde_json::json!("Add outcome context for \"Customer Reliability\"");
    std::fs::write(
        &report_path,
        format!("{}\n", serde_json::to_string_pretty(&report)?),
    )?;

    shiplog_cmd()
        .args([
            "journal",
            "add",
            "--events",
            manual_events.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
            "--from-repair",
            repair_id.as_str(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added manual event:"))
        .stdout(predicate::str::contains(format!("Repair: {repair_id}")))
        .stdout(predicate::str::contains(format!(
            "Report: {}",
            report_path.display().to_string().replace('\\', "/")
        )))
        .stdout(predicate::str::contains("Workstream: Customer Reliability"))
        .stdout(predicate::str::contains(
            "shiplog intake --last-6-months --explain",
        ))
        .stdout(predicate::str::contains(format!(
            "shiplog repair plan --out \"{}\" --latest",
            out.display()
        )));

    let file: ManualEventsFile = serde_yaml::from_str(&std::fs::read_to_string(&manual_events)?)?;
    assert_eq!(file.events.len(), 1);
    let entry = &file.events[0];
    assert!(entry.id.contains(&repair_id));
    assert_eq!(entry.title, format!("Manual evidence repair ({repair_id})"));
    assert_eq!(entry.workstream.as_deref(), Some("Customer Reliability"));
    assert!(
        entry
            .description
            .as_deref()
            .unwrap_or_default()
            .contains("Report-derived repair")
    );
    let window_until =
        NaiveDate::parse_from_str(report["window"]["until"].as_str().unwrap(), "%Y-%m-%d")?;
    assert_eq!(
        entry.date,
        ManualDate::Single(window_until - Duration::days(1))
    );
    assert!(entry.tags.contains(&"shiplog-repair".to_string()));
    assert!(entry.tags.contains(&repair_id));
    assert!(entry.tags.contains(&"manual_evidence_missing".to_string()));

    Ok(())
}

#[test]
fn malformed_manual_journal_blocks_journal_repair_actions() -> CliTestResult {
    let Some(repo) = create_local_git_repo() else {
        return Ok(());
    };
    let tmp = TempDir::new()?;
    let out = tmp.path().join("out");
    let config_path = tmp.path().join("shiplog.toml");
    let manual_dir = tmp.path().join("manual-dns-path");
    let manual_events = manual_dir.join("manual_events.yaml");
    let repo_path = repo.path().display().to_string().replace('\\', "/");
    std::fs::create_dir_all(&manual_dir)?;

    std::fs::write(
        &config_path,
        format!(
            r#"[user]
label = "shiplog test"

[sources.git]
enabled = true
repo = "{repo_path}"
include_merges = false

[sources.manual]
enabled = true
events = "./manual-dns-path/manual_events.yaml"
"#
        ),
    )?;
    std::fs::write(&manual_events, "events: []\n")?;

    let assert = shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args([
            "intake",
            "--config",
            config_path.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
            "--year",
            "2025",
            "--no-open",
            "--explain",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    assert!(
        stdout.contains("Repair manual_events.yaml so it uses the current manual journal schema."),
        "malformed manual journal should print setup repair guidance. stdout:\n{stdout}"
    );
    assert!(
        !stdout.contains("shiplog journal add"),
        "intake should not advertise journal add while the configured manual journal is malformed. stdout:\n{stdout}"
    );

    let (_, report) = load_first_intake_report(&out);
    for field in ["top_fixups", "next_commands"] {
        assert!(
            report[field]
                .as_array()
                .expect("report field should be an array")
                .iter()
                .filter_map(|item| {
                    if field == "top_fixups" {
                        item["command"].as_str()
                    } else {
                        item.as_str()
                    }
                })
                .all(|command| !command.starts_with("shiplog journal add ")),
            "{field} should not contain journal add commands while manual journal setup is blocked: {:?}",
            report[field]
        );
    }
    assert!(
        report["repair_sources"]
            .as_array()
            .expect("repair_sources should be an array")
            .iter()
            .filter(|repair| repair["source_key"].as_str() == Some("manual"))
            .flat_map(|repair| repair["commands"].as_array().into_iter().flatten())
            .filter_map(|command| command.as_str())
            .all(|command| !command.contains("shiplog journal add")),
        "manual repair source guidance should not suggest journal add while the journal file is malformed: {:?}",
        report["repair_sources"]
    );
    assert!(
        report["repair_sources"]
            .as_array()
            .expect("repair_sources should be an array")
            .iter()
            .filter(|repair| repair["source_key"].as_str() == Some("manual"))
            .flat_map(|repair| repair["commands"].as_array().into_iter().flatten())
            .filter_map(|command| command.as_str())
            .any(|command| command.contains("shiplog doctor --setup")
                && command.contains("--source manual")),
        "manual setup repair should route through read-only doctor setup first: {:?}",
        report["repair_sources"]
    );
    assert!(
        report["repair_items"]
            .as_array()
            .expect("repair_items should be an array")
            .iter()
            .all(|item| item["action"]["kind"].as_str() != Some("journal_add")),
        "repair_items should not expose from-repair journal actions while the manual journal is malformed: {:?}",
        report["repair_items"]
    );
    assert!(
        report["evidence_debt"]
            .as_array()
            .expect("evidence_debt should be an array")
            .iter()
            .filter_map(|item| item["next_step"].as_str())
            .all(|next_step| !next_step.starts_with("shiplog journal add ")),
        "evidence debt should not expose journal add next steps while the manual journal is malformed: {:?}",
        report["evidence_debt"]
    );

    let repair_plan = shiplog_cmd()
        .current_dir(tmp.path())
        .args(["repair", "plan", "--out", out.to_str().unwrap(), "--latest"])
        .assert()
        .success();
    let repair_plan_stdout = String::from_utf8(repair_plan.get_output().stdout.clone())?;
    assert!(
        !repair_plan_stdout.contains("journal add --from-repair"),
        "repair plan should not print from-repair journal commands while manual journal setup is blocked. stdout:\n{repair_plan_stdout}"
    );
    assert!(
        repair_plan_stdout.contains("shiplog doctor --setup")
            && repair_plan_stdout.contains("--source manual"),
        "repair plan should route malformed manual setup through doctor before journal repair. stdout:\n{repair_plan_stdout}"
    );

    Ok(())
}

#[test]
fn journal_add_from_repair_uses_report_config_manual_events_when_events_omitted() -> CliTestResult {
    let tmp = TempDir::new()?;
    let config_dir = tmp.path().join("config");
    std::fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("shiplog.toml");
    let manual_events = config_dir.join("manual_events.yaml");
    let default_manual_events = tmp.path().join("manual_events.yaml");
    let out = tmp.path().join("custom-out");

    std::fs::write(
        &config_path,
        r#"[defaults]
window = "last-6-months"

[sources.manual]
enabled = true
events = "./manual_events.yaml"
user = "octo"
"#,
    )?;
    std::fs::write(
        &manual_events,
        "version: 1\ngenerated_at: 2026-01-01T00:00:00Z\nevents: []\n",
    )?;

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args([
            "intake",
            "--config",
            config_path.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
            "--no-open",
        ])
        .assert()
        .success();
    let (_, report) = load_first_intake_report(&out);
    let repair_id = first_repair_id_with_action(&report, "journal_add");

    shiplog_cmd()
        .current_dir(tmp.path())
        .args([
            "journal",
            "add",
            "--from-repair",
            repair_id.as_str(),
            "--out",
            out.to_str().unwrap(),
            "--latest",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added manual event:"))
        .stdout(predicate::str::contains(
            manual_events.display().to_string(),
        ));

    assert!(
        !default_manual_events.exists(),
        "journal repair should not create the cwd default manual_events.yaml when the report config points elsewhere"
    );
    let file: ManualEventsFile = serde_yaml::from_str(&std::fs::read_to_string(&manual_events)?)?;
    assert_eq!(file.events.len(), 1);
    assert!(
        file.events[0].id.contains(&repair_id),
        "journal repair should append the report-derived repair event to the configured manual events file"
    );

    Ok(())
}

#[test]
fn journal_repair_rerun_includes_manual_when_source_filter_disabled_it() -> CliTestResult {
    let Some(tmp) = create_local_git_repo() else {
        return Ok(());
    };
    run_git(
        tmp.path(),
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/EffortlessMetrics/shiplog.git",
        ],
    );
    let out = tmp.path().join("out");
    let out_arg = out.to_str().unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["intake", "--source", "git", "--out", out_arg, "--no-open"])
        .assert()
        .success();

    let config = std::fs::read_to_string(tmp.path().join("shiplog.toml"))?;
    assert!(
        config.contains("[sources.manual]\nenabled = false"),
        "source-filtered intake should preserve manual as disabled in generated config"
    );
    let (_, first_report) = load_first_intake_report(&out);
    assert_eq!(
        report_source_event_count(&first_report, "manual"),
        0,
        "source-filtered first run should not collect manual evidence"
    );
    let repair_id = first_repair_id_with_action(&first_report, "journal_add");

    let journal_assert = shiplog_cmd()
        .current_dir(tmp.path())
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
        .success()
        .stdout(predicate::str::contains("Added manual event:"))
        .stdout(predicate::str::contains("File: manual_events.yaml"))
        .stdout(predicate::str::contains("--source git --source manual"));
    let journal_stdout = String::from_utf8(journal_assert.get_output().stdout.clone())?;
    assert!(
        journal_stdout.contains("shiplog intake --last-6-months --explain"),
        "journal repair should still print the rerun command. stdout:\n{journal_stdout}"
    );

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args([
            "intake",
            "--source",
            "git",
            "--source",
            "manual",
            "--out",
            out_arg,
            "--no-open",
        ])
        .assert()
        .success();

    let repaired_run = all_run_dirs(&out)
        .into_iter()
        .next_back()
        .expect("rerun should create a latest run");
    let repaired_report: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        repaired_run.join("intake.report.json"),
    )?)?;
    assert_eq!(
        report_source_event_count(&repaired_report, "manual"),
        1,
        "printed source-filtered rerun path should collect the repair journal event"
    );

    Ok(())
}

#[test]
fn journal_add_from_repair_rejects_missing_latest_report() -> CliTestResult {
    let tmp = TempDir::new()?;
    let out = tmp.path().join("missing-out");
    let manual_events = tmp.path().join("manual_events.yaml");

    shiplog_cmd()
        .args([
            "journal",
            "add",
            "--events",
            manual_events.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
            "--from-repair",
            "repair_001_manual_missing",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No latest intake report found"))
        .stderr(predicate::str::contains(
            "shiplog intake --last-6-months --explain",
        ));

    assert!(!manual_events.exists());

    Ok(())
}

#[test]
fn journal_add_from_repair_lists_valid_ids_for_unknown_repair() -> CliTestResult {
    let tmp = TempDir::new()?;
    let out = tmp.path().join("out");
    let manual_events = tmp.path().join("repair_manual_events.yaml");
    run_intake_without_provider_tokens(tmp.path(), &out);
    let (_, report) = load_first_intake_report(&out);
    let valid_repair_id = first_repair_id_with_action(&report, "journal_add");

    shiplog_cmd()
        .args([
            "journal",
            "add",
            "--events",
            manual_events.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
            "--from-repair",
            "repair_999_missing",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown repair id"))
        .stderr(predicate::str::contains("Valid repair IDs:"))
        .stderr(predicate::str::contains(valid_repair_id));

    assert!(!manual_events.exists());

    Ok(())
}

#[test]
fn journal_add_from_repair_rejects_non_journal_action() -> CliTestResult {
    let tmp = TempDir::new()?;
    let out = tmp.path().join("out");
    let manual_events = tmp.path().join("repair_manual_events.yaml");
    run_intake_without_provider_tokens(tmp.path(), &out);
    let (_, report) = load_first_intake_report(&out);
    let repair_id = first_repair_id_without_action(&report, "journal_add");

    shiplog_cmd()
        .args([
            "journal",
            "add",
            "--events",
            manual_events.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
            "--from-repair",
            repair_id.as_str(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("uses action kind"))
        .stderr(predicate::str::contains("not journal_add"))
        .stderr(
            predicate::str::contains("Safe command:").or(predicate::str::contains("Guidance:")),
        );

    assert!(!manual_events.exists());

    Ok(())
}

#[test]
fn journal_add_from_repair_rejects_legacy_report_without_repair_items() -> CliTestResult {
    let tmp = TempDir::new()?;
    let out = tmp.path().join("out");
    let manual_events = tmp.path().join("repair_manual_events.yaml");
    run_intake_without_provider_tokens(tmp.path(), &out);
    let (report_path, mut report) = load_first_intake_report(&out);
    report
        .as_object_mut()
        .expect("report should be an object")
        .remove("repair_items");
    std::fs::write(
        &report_path,
        format!("{}\n", serde_json::to_string_pretty(&report)?),
    )?;

    shiplog_cmd()
        .args([
            "journal",
            "add",
            "--events",
            manual_events.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
            "--from-repair",
            "repair_001_manual_evidence",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not include repair_items"))
        .stderr(predicate::str::contains(
            "shiplog intake --last-6-months --explain",
        ));

    assert!(!manual_events.exists());

    Ok(())
}

#[test]
fn journal_add_rejects_repair_lookup_flags_without_from_repair() -> CliTestResult {
    let tmp = TempDir::new()?;
    let out = tmp.path().join("out");
    let manual_events = tmp.path().join("manual_events.yaml");

    shiplog_cmd()
        .args([
            "journal",
            "add",
            "--events",
            manual_events.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
            "--date",
            "2026-05-08",
            "--title",
            "Normal journal entry",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "journal add --out, --run, and --latest require --from-repair",
        ));

    assert!(!manual_events.exists());

    Ok(())
}

#[test]
fn journal_add_rejects_duplicate_manual_event_id() {
    let tmp = TempDir::new().unwrap();
    let manual_events = tmp.path().join("manual_events.yaml");

    for expected_success in [true, false] {
        let mut assert = shiplog_cmd()
            .args([
                "journal",
                "add",
                "--events",
                manual_events.to_str().unwrap(),
                "--id",
                "manual-duplicate",
                "--date",
                "2026-05-08",
                "--title",
                "Duplicate evidence",
            ])
            .assert();

        if expected_success {
            assert = assert.success();
            assert.stdout(predicate::str::contains("Added manual event"));
        } else {
            assert = assert.failure();
            assert.stderr(predicate::str::contains("already exists"));
        }
    }
}

#[test]
fn journal_list_shows_entries_without_writing() {
    let tmp = TempDir::new().unwrap();
    let manual_events = tmp.path().join("manual_events.yaml");
    seed_journal_list_events(&manual_events);
    let before = std::fs::read_to_string(&manual_events).unwrap();

    let assert = shiplog_cmd()
        .args([
            "journal",
            "list",
            "--events",
            manual_events.to_str().unwrap(),
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert!(stdout.contains("Manual events:"));
    assert!(stdout.contains("Count: 2"));
    assert!(stdout.contains("manual-architecture-review | 2026-05-01..2026-05-03 | Design | Platform Reliability | Reviewed architecture decision"));
    assert!(stdout.contains("manual-customer-import | 2026-05-08 | Note | Customer Reliability | Debugged customer import incident"));
    assert!(stdout.contains("tags: support, review-cycle"));
    assert!(stdout.contains("receipts: 1"));

    let after = std::fs::read_to_string(&manual_events).unwrap();
    assert_eq!(
        before, after,
        "journal list should not write manual_events.yaml"
    );
}

#[test]
fn journal_list_filters_by_workstream_and_tag() {
    let tmp = TempDir::new().unwrap();
    let manual_events = tmp.path().join("manual_events.yaml");
    seed_journal_list_events(&manual_events);
    let before = std::fs::read_to_string(&manual_events).unwrap();

    shiplog_cmd()
        .args([
            "journal",
            "list",
            "--events",
            manual_events.to_str().unwrap(),
            "--workstream",
            "customer reliability",
            "--tag",
            "support",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Count: 1"))
        .stdout(predicate::str::contains("manual-customer-import"))
        .stdout(predicate::str::contains("Customer Reliability"))
        .stdout(predicate::str::contains(
            "Debugged customer import incident",
        ))
        .stdout(predicate::str::contains("manual-architecture-review").not());

    shiplog_cmd()
        .args([
            "journal",
            "list",
            "--events",
            manual_events.to_str().unwrap(),
            "--tag",
            "missing",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Count: 0"))
        .stdout(predicate::str::contains("No manual events matched."));

    let after = std::fs::read_to_string(&manual_events).unwrap();
    assert_eq!(
        before, after,
        "journal list filters should not write manual_events.yaml"
    );
}

#[test]
fn journal_list_missing_file_fails_without_creating_it() {
    let tmp = TempDir::new().unwrap();
    let manual_events = tmp.path().join("manual_events.yaml");

    shiplog_cmd()
        .args([
            "journal",
            "list",
            "--events",
            manual_events.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No manual events file found"));

    assert!(
        !manual_events.exists(),
        "journal list should not create a missing file"
    );
}

#[test]
fn journal_edit_updates_entry_and_preserves_unrelated_entries() {
    let tmp = TempDir::new().unwrap();
    let manual_events = tmp.path().join("manual_events.yaml");
    seed_journal_list_events(&manual_events);
    let before: ManualEventsFile =
        serde_yaml::from_str(&std::fs::read_to_string(&manual_events).unwrap()).unwrap();
    let unrelated_before = before
        .events
        .iter()
        .find(|entry| entry.id == "manual-architecture-review")
        .unwrap()
        .clone();

    shiplog_cmd()
        .args([
            "journal",
            "edit",
            "--events",
            manual_events.to_str().unwrap(),
            "--id",
            "manual-customer-import",
            "--type",
            "incident",
            "--start",
            "2026-05-08",
            "--end",
            "2026-05-09",
            "--title",
            "Debugged customer import and wrote runbook",
            "--workstream",
            "Customer Reliability",
            "--impact",
            "Prevented repeat failure before the next import window",
            "--tag",
            "support",
            "--tag",
            "customer",
            "--receipt",
            "ticket=https://example.invalid/ticket/OPS-123",
            "--receipt",
            "runbook=https://example.invalid/runbooks/import",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Edited manual event: manual-customer-import",
        ))
        .stdout(predicate::str::contains("Date: 2026-05-08..2026-05-09"))
        .stdout(predicate::str::contains(
            "Title: Debugged customer import and wrote runbook",
        ))
        .stdout(predicate::str::contains("shiplog collect multi"));

    let file: ManualEventsFile =
        serde_yaml::from_str(&std::fs::read_to_string(&manual_events).unwrap()).unwrap();
    assert_eq!(file.events.len(), 2);
    let edited = file
        .events
        .iter()
        .find(|entry| entry.id == "manual-customer-import")
        .unwrap();
    assert_eq!(edited.event_type, ManualEventType::Incident);
    assert_eq!(
        edited.date,
        ManualDate::Range {
            start: NaiveDate::from_ymd_opt(2026, 5, 8).unwrap(),
            end: NaiveDate::from_ymd_opt(2026, 5, 9).unwrap()
        }
    );
    assert_eq!(edited.title, "Debugged customer import and wrote runbook");
    assert_eq!(edited.workstream.as_deref(), Some("Customer Reliability"));
    assert_eq!(
        edited.impact.as_deref(),
        Some("Prevented repeat failure before the next import window")
    );
    assert_eq!(edited.tags, vec!["support", "customer"]);
    assert_eq!(edited.receipts.len(), 2);
    assert_eq!(edited.receipts[1].label, "runbook");

    let unrelated_after = file
        .events
        .iter()
        .find(|entry| entry.id == "manual-architecture-review")
        .unwrap();
    assert_eq!(
        unrelated_after, &unrelated_before,
        "journal edit should preserve unrelated entries"
    );
}

#[test]
fn journal_edit_dry_run_does_not_write() {
    let tmp = TempDir::new().unwrap();
    let manual_events = tmp.path().join("manual_events.yaml");
    seed_journal_list_events(&manual_events);
    let before = std::fs::read_to_string(&manual_events).unwrap();

    shiplog_cmd()
        .args([
            "journal",
            "edit",
            "--events",
            manual_events.to_str().unwrap(),
            "--id",
            "manual-customer-import",
            "--title",
            "Dry run title",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Would edit manual event: manual-customer-import",
        ))
        .stdout(predicate::str::contains("Title: Dry run title"));

    let after = std::fs::read_to_string(&manual_events).unwrap();
    assert_eq!(before, after, "journal edit --dry-run should not write");
}

#[test]
fn journal_edit_rejects_invalid_date_without_writing() {
    let tmp = TempDir::new().unwrap();
    let manual_events = tmp.path().join("manual_events.yaml");
    seed_journal_list_events(&manual_events);
    let before = std::fs::read_to_string(&manual_events).unwrap();

    shiplog_cmd()
        .args([
            "journal",
            "edit",
            "--events",
            manual_events.to_str().unwrap(),
            "--id",
            "manual-customer-import",
            "--start",
            "2026-05-10",
            "--end",
            "2026-05-01",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "journal date range must satisfy --start 2026-05-10 <= --end 2026-05-01",
        ));

    let after = std::fs::read_to_string(&manual_events).unwrap();
    assert_eq!(
        before, after,
        "journal edit should not write when date validation fails"
    );
}

#[test]
fn journal_edit_rejects_duplicate_ids_without_writing() {
    let tmp = TempDir::new().unwrap();
    let manual_events = tmp.path().join("manual_events.yaml");
    std::fs::write(
        &manual_events,
        r#"version: 1
generated_at: 2026-01-01T00:00:00Z
events:
  - id: duplicate
    type: Note
    date: 2026-05-08
    title: First duplicate
    tags: []
    receipts: []
  - id: duplicate
    type: Note
    date: 2026-05-09
    title: Second duplicate
    tags: []
    receipts: []
"#,
    )
    .unwrap();
    let before = std::fs::read_to_string(&manual_events).unwrap();

    shiplog_cmd()
        .args([
            "journal",
            "edit",
            "--events",
            manual_events.to_str().unwrap(),
            "--id",
            "duplicate",
            "--title",
            "Should not write",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("appears more than once"));

    let after = std::fs::read_to_string(&manual_events).unwrap();
    assert_eq!(
        before, after,
        "journal edit should not write when an ID is duplicated"
    );
}

#[test]
fn journal_add_writes_date_ranges() {
    let tmp = TempDir::new().unwrap();
    let manual_events = tmp.path().join("manual_events.yaml");

    shiplog_cmd()
        .args([
            "journal",
            "add",
            "--events",
            manual_events.to_str().unwrap(),
            "--type",
            "migration",
            "--start",
            "2026-05-01",
            "--end",
            "2026-05-08",
            "--title",
            "Finished migration rehearsal",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Date: 2026-05-01..2026-05-08"));

    let file: ManualEventsFile =
        serde_yaml::from_str(&std::fs::read_to_string(&manual_events).unwrap()).unwrap();
    assert_eq!(file.events[0].event_type, ManualEventType::Migration);
    assert_eq!(
        file.events[0].date,
        ManualDate::Range {
            start: NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
            end: NaiveDate::from_ymd_opt(2026, 5, 8).unwrap()
        }
    );
}

#[test]
fn doctor_reports_missing_config_actionably() {
    let tmp = TempDir::new().unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["doctor"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Config: error"))
        .stdout(predicate::str::contains("run `shiplog init` first"));
}

#[test]
fn doctor_checks_init_defaults_without_collecting() {
    let tmp = TempDir::new().unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .assert()
        .success();

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .args(["doctor"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Config: ok"))
        .stdout(predicate::str::contains("Window: ok"))
        .stdout(predicate::str::contains("Redaction: ok"))
        .stdout(predicate::str::contains(
            "GitHub: error, missing GITHUB_TOKEN",
        ))
        .stdout(predicate::str::contains("Manual: ok"));
}

#[test]
fn doctor_accepts_fixture_safe_sources() {
    let tmp = TempDir::new().unwrap();
    let fixtures = fixture_dir();
    std::fs::copy(
        fixtures.join("ledger.events.jsonl"),
        tmp.path().join("ledger.events.jsonl"),
    )
    .unwrap();
    std::fs::copy(
        fixtures.join("coverage.manifest.json"),
        tmp.path().join("coverage.manifest.json"),
    )
    .unwrap();
    std::fs::write(
        tmp.path().join("manual_events.yaml"),
        "version: 1\ngenerated_at: 2026-01-01T00:00:00Z\nevents: []\n",
    )
    .unwrap();
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[defaults]
out = "./out"
window = "last-6-months"
profile = "internal"

[sources.json]
enabled = true
events = "./ledger.events.jsonl"
coverage = "./coverage.manifest.json"

[sources.manual]
enabled = true
events = "./manual_events.yaml"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Config: ok"))
        .stdout(predicate::str::contains("JSON: ok"))
        .stdout(predicate::str::contains("Manual: ok"))
        .stdout(predicate::str::contains("Redaction: ok"));
}

#[test]
fn doctor_setup_prints_readiness_without_writing_outputs() -> CliTestResult {
    let tmp = TempDir::new()?;
    git2::Repository::init(tmp.path())?;
    std::fs::write(
        tmp.path().join("manual_events.yaml"),
        "version: 1\ngenerated_at: 2026-01-01T00:00:00Z\nevents: []\n",
    )?;
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[shiplog]
config_version = 1

[defaults]
out = "./out"
window = "last-6-months"
profile = "internal"

[sources.git]
enabled = true
repo = "."

[sources.manual]
enabled = true
events = "./manual_events.yaml"
"#,
    )?;

    shiplog_cmd()
        .current_dir(tmp.path())
        .env("SHIPLOG_REDACT_KEY", "stable-redact-key")
        .args(["doctor", "--setup"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Setup readiness: Ready with caveats",
        ))
        .stdout(predicate::str::contains("Ready:"))
        .stdout(predicate::str::contains("Local git"))
        .stdout(predicate::str::contains("Manual journal"))
        .stdout(predicate::str::contains("Disabled:"))
        .stdout(predicate::str::contains("GitHub"))
        .stdout(predicate::str::contains("Next:"))
        .stdout(predicate::str::contains("[read-only]"));

    assert!(
        !tmp.path().join("out").exists(),
        "doctor --setup should not write run artifacts"
    );
    Ok(())
}

#[test]
fn doctor_setup_reports_share_profile_blocks_without_writing() -> CliTestResult {
    let tmp = TempDir::new()?;
    git2::Repository::init(tmp.path())?;
    std::fs::write(
        tmp.path().join("manual_events.yaml"),
        "version: 1\ngenerated_at: 2026-01-01T00:00:00Z\nevents: []\n",
    )?;
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[shiplog]
config_version = 1

[defaults]
out = "./out"
window = "last-6-months"
profile = "internal"

[sources.git]
enabled = true
repo = "."

[sources.manual]
enabled = true
events = "./manual_events.yaml"
"#,
    )?;

    let before = file_tree_manifest(tmp.path());
    let assert = shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("SHIPLOG_REDACT_KEY")
        .args(["doctor", "--setup"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Setup readiness: Needs setup"))
        .stdout(predicate::str::contains("Blocked:"))
        .stdout(predicate::str::contains("Manager share"))
        .stdout(predicate::str::contains(
            "manager share rendering is blocked",
        ))
        .stdout(predicate::str::contains("Public share"))
        .stdout(predicate::str::contains(
            "strict verification requires a rendered public packet",
        ))
        .stdout(predicate::str::contains(
            "Next (read-only): set SHIPLOG_REDACT_KEY",
        ));
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let after = file_tree_manifest(tmp.path());

    assert!(
        stdout.contains("SHIPLOG_REDACT_KEY not set"),
        "doctor --setup should name the missing redaction key. stdout:\n{stdout}"
    );
    assert_eq!(
        before, after,
        "doctor --setup should not render profile artifacts while reporting share readiness"
    );
    assert!(
        !tmp.path().join("out").exists(),
        "doctor --setup should not write run artifacts"
    );
    assert!(
        !tmp.path().join("profiles").exists(),
        "doctor --setup should not write share profile artifacts"
    );

    Ok(())
}

#[test]
fn doctor_setup_reports_malformed_manual_as_read_only_setup_block() -> CliTestResult {
    let tmp = TempDir::new()?;
    std::fs::write(tmp.path().join("manual_events.yaml"), "version: nope\n")?;
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[shiplog]
config_version = 1

[defaults]
out = "./out"
window = "last-6-months"
profile = "internal"

[sources.manual]
enabled = true
events = "./manual_events.yaml"
"#,
    )?;

    let assert = shiplog_cmd()
        .current_dir(tmp.path())
        .args(["doctor", "--setup"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Setup readiness: Blocked"))
        .stdout(predicate::str::contains("Blocked:"))
        .stdout(predicate::str::contains("Manual journal"))
        .stdout(predicate::str::contains("manual_events.yaml malformed"))
        .stdout(predicate::str::contains("Next:"))
        .stdout(predicate::str::contains("shiplog doctor --setup"))
        .stdout(predicate::str::contains("[read-only]"));
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;

    assert!(
        !stdout.contains("journal add --from-repair"),
        "doctor --setup should not offer repair commands that need a valid journal"
    );
    assert!(
        !tmp.path().join("out").exists(),
        "doctor --setup should not write run artifacts"
    );
    Ok(())
}

#[test]
fn doctor_setup_reports_token_presence_without_exposing_values() -> CliTestResult {
    let tmp = TempDir::new()?;
    std::fs::write(
        tmp.path().join("manual_events.yaml"),
        "version: 1\ngenerated_at: 2026-01-01T00:00:00Z\nevents: []\n",
    )?;
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[shiplog]
config_version = 1

[defaults]
out = "./out"
window = "last-6-months"
profile = "internal"

[sources.github]
enabled = true
user = "octo"

[sources.manual]
enabled = true
events = "./manual_events.yaml"
"#,
    )?;

    let assert = shiplog_cmd()
        .current_dir(tmp.path())
        .env("GITHUB_TOKEN", "shiplog-secret-token")
        .env("SHIPLOG_REDACT_KEY", "stable-redact-key")
        .args(["doctor", "--setup"])
        .assert()
        .success()
        .stdout(predicate::str::contains("GitHub token"))
        .stdout(predicate::str::contains("GITHUB_TOKEN present"));
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;

    assert!(
        !stdout.contains("shiplog-secret-token"),
        "doctor --setup should report credential presence without printing values"
    );
    Ok(())
}

#[test]
fn sources_status_prints_source_only_readiness_without_writing_outputs() -> CliTestResult {
    let tmp = TempDir::new()?;
    git2::Repository::init(tmp.path())?;
    std::fs::write(
        tmp.path().join("manual_events.yaml"),
        "version: 1\ngenerated_at: 2026-01-01T00:00:00Z\nevents: []\n",
    )?;
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[shiplog]
config_version = 1

[defaults]
out = "./out"
window = "last-6-months"
profile = "manager"

[sources.git]
enabled = true
repo = "."

[sources.manual]
enabled = true
events = "./manual_events.yaml"
"#,
    )?;

    let assert = shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("SHIPLOG_REDACT_KEY")
        .args(["sources", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Source setup status:"))
        .stdout(predicate::str::contains("source_key"))
        .stdout(predicate::str::contains("source_label"))
        .stdout(predicate::str::contains("git"))
        .stdout(predicate::str::contains("Local git"))
        .stdout(predicate::str::contains("manual"))
        .stdout(predicate::str::contains("Manual journal"))
        .stdout(predicate::str::contains("Next:"));
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;

    assert!(
        !stdout.contains("Manager share"),
        "sources status should not include share-profile readiness"
    );
    assert!(
        !stdout.contains("Redaction key"),
        "sources status should not include credential/share noise"
    );
    assert!(
        !tmp.path().join("out").exists(),
        "sources status should not write run artifacts"
    );
    Ok(())
}

#[test]
fn sources_status_reports_configured_provider_token_gap() -> CliTestResult {
    let tmp = TempDir::new()?;
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[shiplog]
config_version = 1

[defaults]
out = "./out"
window = "last-6-months"
profile = "internal"

[sources.github]
enabled = true
user = "octo"
"#,
    )?;

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .args(["sources", "status"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("github"))
        .stdout(predicate::str::contains("GitHub"))
        .stdout(predicate::str::contains("unavailable"))
        .stdout(predicate::str::contains("GITHUB_TOKEN not set"))
        .stdout(predicate::str::contains("set GITHUB_TOKEN"))
        .stdout(predicate::str::contains("[read-only]"));

    assert!(
        !tmp.path().join("out").exists(),
        "sources status should not write run artifacts"
    );
    Ok(())
}

#[test]
fn sources_status_reports_token_presence_without_exposing_values() -> CliTestResult {
    let tmp = TempDir::new()?;
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[shiplog]
config_version = 1

[defaults]
out = "./out"
window = "last-6-months"
profile = "internal"

[sources.github]
enabled = true
user = "octo"
"#,
    )?;

    let assert = shiplog_cmd()
        .current_dir(tmp.path())
        .env("GITHUB_TOKEN", "shiplog-source-secret-token")
        .args(["sources", "status", "--source", "github"])
        .assert()
        .success()
        .stdout(predicate::str::contains("github"))
        .stdout(predicate::str::contains("ready"))
        .stdout(predicate::str::contains("token present"));
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;

    assert!(
        !stdout.contains("shiplog-source-secret-token"),
        "sources status should report token presence without printing values"
    );
    Ok(())
}

#[test]
fn doctor_setup_json_outputs_setup_model_without_writes_or_text() -> CliTestResult {
    let tmp = TempDir::new()?;
    git2::Repository::init(tmp.path())?;
    std::fs::write(
        tmp.path().join("manual_events.yaml"),
        "version: 1\ngenerated_at: 2026-01-01T00:00:00Z\nevents: []\n",
    )?;
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[shiplog]
config_version = 1

[defaults]
out = "./out"
window = "last-6-months"
profile = "manager"

[sources.git]
enabled = true
repo = "."

[sources.manual]
enabled = true
events = "./manual_events.yaml"

[sources.github]
enabled = true
user = "octo"
"#,
    )?;

    let before = file_tree_manifest(tmp.path());
    let assert = shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("SHIPLOG_REDACT_KEY")
        .args(["doctor", "--setup", "--json"])
        .assert()
        .failure();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let after = file_tree_manifest(tmp.path());
    assert_eq!(
        before, after,
        "doctor --setup --json should not write setup or run artifacts"
    );
    assert!(
        !stdout.contains("Setup readiness:"),
        "--json should emit machine-readable setup state without human prose"
    );

    let json: serde_json::Value = serde_json::from_str(&stdout)?;
    assert_eq!(json["overall_status"], "needs_setup");

    let git = setup_json_item(&json, "sources", "git");
    assert_eq!(git["label"], "Local git");
    assert_eq!(git["enabled"], true);
    assert_eq!(git["status"], "ready");

    let manual = setup_json_item(&json, "sources", "manual");
    assert_eq!(manual["label"], "Manual journal");
    assert_eq!(manual["status"], "ready");

    let github = setup_json_item(&json, "sources", "github");
    assert_eq!(github["label"], "GitHub");
    assert_eq!(github["enabled"], true);
    assert_eq!(github["status"], "unavailable");
    assert!(
        github["reason"]
            .as_str()
            .unwrap()
            .contains("GITHUB_TOKEN not set")
    );
    assert_eq!(github["next_action"]["command"], "set GITHUB_TOKEN");
    assert_eq!(github["next_action"]["writes"], false);

    let redaction = setup_json_item(&json, "credentials", "redaction_key");
    assert_eq!(redaction["status"], "unavailable");
    assert!(
        redaction["reason"]
            .as_str()
            .unwrap()
            .contains("SHIPLOG_REDACT_KEY not set")
    );

    let manager = setup_json_item(&json, "share_profiles", "manager");
    assert_eq!(manager["status"], "blocked");
    assert!(
        manager["reason"]
            .as_str()
            .unwrap()
            .contains("SHIPLOG_REDACT_KEY not set")
    );

    let next_actions = json["next_actions"]
        .as_array()
        .expect("next_actions should be an array");
    assert!(
        next_actions
            .iter()
            .any(|action| { action["command"] == "set GITHUB_TOKEN" && action["writes"] == false }),
        "agent JSON should expose read-only credential setup next action"
    );
    assert!(
        next_actions.iter().any(|action| {
            action["command"] == "set SHIPLOG_REDACT_KEY" && action["writes"] == false
        }),
        "agent JSON should expose read-only redaction setup next action"
    );

    let assert = shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("SHIPLOG_REDACT_KEY")
        .args(["doctor", "--setup", "--json"])
        .assert()
        .failure();
    let stdout_again = String::from_utf8(assert.get_output().stdout.clone())?;
    assert_eq!(
        stdout, stdout_again,
        "doctor --setup --json should be deterministic across repeated reads"
    );

    Ok(())
}

#[test]
fn doctor_setup_json_reports_presence_without_printing_secret_values() -> CliTestResult {
    let tmp = TempDir::new()?;
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[shiplog]
config_version = 1

[defaults]
out = "./out"
window = "last-6-months"
profile = "internal"

[sources.github]
enabled = true
user = "octo"
"#,
    )?;

    let secret_token = "shiplog-json-source-secret";
    let secret_redaction = "shiplog-json-redaction-secret";
    let assert = shiplog_cmd()
        .current_dir(tmp.path())
        .env("GITHUB_TOKEN", secret_token)
        .env("SHIPLOG_REDACT_KEY", secret_redaction)
        .args(["doctor", "--setup", "--source", "github", "--json"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    assert!(
        !stdout.contains(secret_token) && !stdout.contains(secret_redaction),
        "doctor setup JSON should report env-var presence without printing values"
    );

    let json: serde_json::Value = serde_json::from_str(&stdout)?;
    assert_eq!(json["overall_status"], "ready_with_caveats");
    assert_eq!(
        setup_json_item(&json, "sources", "github")["status"],
        "ready"
    );
    assert_eq!(
        setup_json_item(&json, "credentials", "github_token")["reason"],
        "GITHUB_TOKEN present"
    );
    assert!(
        setup_json_item(&json, "share_profiles", "manager")["reason"]
            .as_str()
            .unwrap()
            .contains("SHIPLOG_REDACT_KEY present")
    );

    Ok(())
}

#[test]
fn doctor_sources_status_agree_on_source_projection_without_share_noise() -> CliTestResult {
    let tmp = TempDir::new()?;
    git2::Repository::init(tmp.path())?;
    std::fs::write(tmp.path().join("manual_events.yaml"), "events: []\n")?;
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[shiplog]
config_version = 1

[defaults]
out = "./out"
window = "last-6-months"
profile = "manager"

[sources.git]
enabled = true
repo = "."

[sources.manual]
enabled = true
events = "./manual_events.yaml"

[sources.github]
enabled = true
user = "octo"
"#,
    )?;

    let before = file_tree_manifest(tmp.path());
    let doctor_assert = shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("SHIPLOG_REDACT_KEY")
        .args(["doctor", "--setup", "--json"])
        .assert()
        .failure();
    let doctor_stdout = String::from_utf8(doctor_assert.get_output().stdout.clone())?;
    let doctor_json: serde_json::Value = serde_json::from_str(&doctor_stdout)?;

    let sources_assert = shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("SHIPLOG_REDACT_KEY")
        .args(["sources", "status"])
        .assert()
        .failure();
    let sources_stdout = String::from_utf8(sources_assert.get_output().stdout.clone())?;
    let after = file_tree_manifest(tmp.path());
    assert_eq!(
        before, after,
        "doctor --setup --json and sources status should be read-only setup projections"
    );

    let sources_rows = parse_sources_status_rows(&sources_stdout);
    let doctor_sources = doctor_json["sources"]
        .as_array()
        .expect("doctor setup JSON should include sources array");
    assert_eq!(
        sources_rows.len(),
        doctor_sources.len(),
        "sources status should print one row for each doctor JSON source"
    );
    for source in doctor_sources {
        let key = source["key"]
            .as_str()
            .expect("source item should include key");
        let row = sources_rows
            .get(key)
            .unwrap_or_else(|| panic!("sources status should include row for {key}"));
        assert_eq!(row.label, source["label"].as_str().unwrap_or_default());
        assert_eq!(row.enabled, source["enabled"].as_bool().unwrap_or_default());
        assert_eq!(row.status, source["status"].as_str().unwrap_or_default());
        assert_eq!(row.reason, source["reason"].as_str().unwrap_or_default());
    }

    assert_eq!(sources_rows["git"].status, "ready");
    assert_eq!(sources_rows["manual"].status, "blocked");
    assert!(
        sources_rows["manual"]
            .reason
            .contains("manual_events.yaml malformed")
    );
    assert_eq!(sources_rows["github"].status, "unavailable");
    assert!(
        sources_rows["github"]
            .reason
            .contains("GITHUB_TOKEN not set")
    );

    assert!(
        !sources_stdout.contains("Manager share") && !sources_stdout.contains("Public share"),
        "sources status should not include share-profile readiness"
    );
    assert!(
        !sources_stdout.contains("Redaction key") && !sources_stdout.contains("SHIPLOG_REDACT_KEY"),
        "sources status should not include redaction/share credential noise"
    );

    Ok(())
}

#[test]
fn doctor_setup_json_exposes_agent_safe_decisions_for_blocked_setup() -> CliTestResult {
    let tmp = TempDir::new()?;
    git2::Repository::init(tmp.path())?;
    std::fs::write(tmp.path().join("manual_events.yaml"), "events: []\n")?;
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[shiplog]
config_version = 1

[defaults]
out = "./out"
window = "last-6-months"
profile = "manager"

[sources.git]
enabled = true
repo = "."

[sources.manual]
enabled = true
events = "./manual_events.yaml"

[sources.github]
enabled = true
user = "octo"
"#,
    )?;

    let before = file_tree_manifest(tmp.path());
    let assert = shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("SHIPLOG_REDACT_KEY")
        .args(["doctor", "--setup", "--json"])
        .assert()
        .failure();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let after = file_tree_manifest(tmp.path());
    assert_eq!(
        before, after,
        "doctor --setup --json should be a read-only agent control-plane read"
    );
    assert!(
        !stdout.contains("GITHUB_TOKEN=") && !stdout.contains("SHIPLOG_REDACT_KEY="),
        "doctor JSON should name credential requirements without printing secret assignments"
    );

    let json: serde_json::Value = serde_json::from_str(&stdout)?;
    assert_eq!(json["overall_status"], "needs_setup");

    let git = setup_json_item(&json, "sources", "git");
    assert_eq!(git["status"], "ready");
    assert_eq!(git["enabled"], true);

    let manual = setup_json_item(&json, "sources", "manual");
    assert_eq!(manual["status"], "blocked");
    assert!(
        manual["reason"]
            .as_str()
            .unwrap()
            .contains("manual_events.yaml malformed")
    );
    assert_eq!(manual["next_action"]["command"], "shiplog doctor --setup");
    assert_eq!(manual["next_action"]["writes"], false);

    let manual_file = setup_json_item(&json, "local_files", "manual_events");
    assert_eq!(manual_file["status"], "malformed");
    assert_eq!(
        manual_file["next_action"]["command"],
        "shiplog doctor --setup"
    );
    assert_eq!(manual_file["next_action"]["writes"], false);

    let github = setup_json_item(&json, "sources", "github");
    assert_eq!(github["status"], "unavailable");
    assert!(
        github["reason"]
            .as_str()
            .unwrap()
            .contains("GITHUB_TOKEN not set")
    );
    assert_eq!(github["next_action"]["command"], "set GITHUB_TOKEN");
    assert_eq!(github["next_action"]["writes"], false);

    let manager = setup_json_item(&json, "share_profiles", "manager");
    let public = setup_json_item(&json, "share_profiles", "public");
    assert_eq!(manager["status"], "blocked");
    assert_eq!(public["status"], "blocked");
    assert!(
        manager["reason"]
            .as_str()
            .unwrap()
            .contains("SHIPLOG_REDACT_KEY not set")
    );
    assert!(
        public["reason"]
            .as_str()
            .unwrap()
            .contains("SHIPLOG_REDACT_KEY not set")
    );

    let next_commands: Vec<&str> = json["next_actions"]
        .as_array()
        .expect("next_actions should be an array")
        .iter()
        .map(|action| {
            action["command"]
                .as_str()
                .expect("next action should include command")
        })
        .collect();
    assert!(
        next_commands.contains(&"shiplog doctor --setup"),
        "agent JSON should route broken setup back to doctor before repair"
    );
    assert!(
        next_commands.contains(&"set GITHUB_TOKEN"),
        "agent JSON should expose missing provider credential setup"
    );
    assert!(
        next_commands.contains(&"set SHIPLOG_REDACT_KEY"),
        "agent JSON should expose missing redaction setup before share rendering"
    );
    assert!(
        next_commands
            .iter()
            .all(|command| !command.contains("journal add --from-repair")),
        "agent JSON should not offer evidence-repair writes while manual setup is malformed"
    );
    assert!(
        next_commands
            .iter()
            .all(|command| !command.starts_with("shiplog share manager")
                && !command.starts_with("shiplog share public")),
        "agent JSON should not offer share rendering while redaction setup is blocked"
    );
    assert!(
        json["next_actions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|action| action["writes"] == false),
        "blocked setup guidance should stay read-first and avoid write-producing actions"
    );

    Ok(())
}

#[test]
fn doctor_setup_json_hardens_older_setup_config_states() -> CliTestResult {
    let tmp = TempDir::new()?;
    git2::Repository::init(tmp.path())?;
    std::fs::write(tmp.path().join("manual_events.yaml"), "version: nope\n")?;
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[shiplog]
config_version = 1

[defaults]
out = "./out"
window = "last-6-months"
profile = "internal"

[sources.git]
enabled = true
repo = "."

[sources.manual]
enabled = false
events = "./manual_events.yaml"

[sources.github]
enabled = true
user = "octo"

[sources.json]
enabled = true
events = "./missing.events.jsonl"
coverage = "./missing.coverage.json"
"#,
    )?;

    let before = file_tree_manifest(tmp.path());
    let assert = shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("SHIPLOG_REDACT_KEY")
        .args(["doctor", "--setup", "--json"])
        .assert()
        .failure();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let after = file_tree_manifest(tmp.path());
    assert_eq!(
        before, after,
        "doctor --setup --json should classify setup compatibility without writes"
    );

    let json: serde_json::Value = serde_json::from_str(&stdout)?;
    assert_eq!(json["overall_status"], "needs_setup");
    assert_eq!(
        setup_json_item(&json, "local_files", "config")["status"],
        "ready"
    );
    assert_eq!(setup_json_item(&json, "sources", "git")["status"], "ready");

    let manual = setup_json_item(&json, "sources", "manual");
    assert_eq!(manual["enabled"], false);
    assert_eq!(manual["status"], "disabled");
    assert!(
        !manual["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("malformed"),
        "disabled manual setup should not validate malformed manual_events.yaml"
    );
    assert_eq!(
        setup_json_item(&json, "local_files", "manual_events")["status"],
        "optional_absent"
    );

    let github = setup_json_item(&json, "sources", "github");
    assert_eq!(github["enabled"], true);
    assert_eq!(github["status"], "unavailable");
    assert!(
        github["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("GITHUB_TOKEN not set")
    );

    let json_source = setup_json_item(&json, "sources", "json");
    assert_eq!(json_source["status"], "unavailable");
    assert!(
        json_source["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("missing.events.jsonl")
    );

    let sources = json["sources"]
        .as_array()
        .context("sources should be an array")?;
    assert!(
        sources
            .iter()
            .all(|source| source["status"] != "stale_config"),
        "missing optional credentials, disabled manual setup, and missing local paths should not be stale_config"
    );

    Ok(())
}

#[test]
fn doctor_setup_json_blocks_unsupported_config_version_as_stale_config() -> CliTestResult {
    let tmp = TempDir::new()?;
    write_manual_events(&tmp.path().join("manual_events.yaml"));
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[shiplog]
config_version = 2

[defaults]
out = "./out"
window = "last-6-months"
profile = "internal"

[sources.manual]
enabled = true
events = "./manual_events.yaml"
"#,
    )?;

    let before = file_tree_manifest(tmp.path());
    let assert = shiplog_cmd()
        .current_dir(tmp.path())
        .env("SHIPLOG_REDACT_KEY", "stable-redact-key")
        .args(["doctor", "--setup", "--json"])
        .assert()
        .failure();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let after = file_tree_manifest(tmp.path());
    assert_eq!(
        before, after,
        "unsupported config version diagnosis should stay read-only"
    );

    let json: serde_json::Value = serde_json::from_str(&stdout)?;
    assert_eq!(json["overall_status"], "blocked");
    let config = setup_json_item(&json, "local_files", "config");
    assert_eq!(config["status"], "stale_config");
    assert!(
        config["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("unsupported config_version 2")
    );
    assert_eq!(
        config["next_action"]["command"],
        "shiplog config migrate --config shiplog.toml"
    );
    assert_eq!(config["next_action"]["writes"], true);
    assert_eq!(
        setup_json_item(&json, "sources", "manual")["status"],
        "ready"
    );

    Ok(())
}

#[test]
fn doctor_setup_ignores_old_reports_and_reads_current_setup_state() -> CliTestResult {
    let tmp = TempDir::new()?;
    let old_run = tmp.path().join("out").join("run_legacy");
    std::fs::create_dir_all(&old_run)?;
    std::fs::write(
        old_run.join("intake.report.json"),
        r#"{
  "schema_version": 1,
  "run_id": "run_legacy",
  "included_sources": [],
  "skipped_sources": [],
  "actions": []
}
"#,
    )?;
    std::fs::write(
        old_run.join("packet.md"),
        "# Setup readiness\n\nReady\n\nThis stale packet text must not drive doctor.\n",
    )?;
    std::fs::write(tmp.path().join("manual_events.yaml"), "events: []\n")?;
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[shiplog]
config_version = 1

[defaults]
out = "./out"
window = "last-6-months"
profile = "internal"

[sources.manual]
enabled = true
events = "./manual_events.yaml"
"#,
    )?;

    let before = file_tree_manifest(tmp.path());
    let assert = shiplog_cmd()
        .current_dir(tmp.path())
        .env("SHIPLOG_REDACT_KEY", "stable-redact-key")
        .args(["doctor", "--setup", "--json"])
        .assert()
        .failure();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let after = file_tree_manifest(tmp.path());
    assert_eq!(
        before, after,
        "doctor should not rewrite or reinterpret old report artifacts"
    );

    let json: serde_json::Value = serde_json::from_str(&stdout)?;
    assert_eq!(json["overall_status"], "blocked");
    let manual = setup_json_item(&json, "sources", "manual");
    assert_eq!(manual["status"], "blocked");
    assert!(
        manual["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("manual_events.yaml malformed")
    );
    assert!(
        !stdout.contains("run_legacy")
            && !stdout.contains("packet_quality")
            && !stdout.contains("This stale packet text must not drive doctor"),
        "doctor setup JSON should come from setup files, not old report or packet artifacts"
    );

    Ok(())
}

fn setup_json_item<'a>(
    json: &'a serde_json::Value,
    group: &str,
    key: &str,
) -> &'a serde_json::Value {
    json[group]
        .as_array()
        .unwrap_or_else(|| panic!("{group} should be an array"))
        .iter()
        .find(|item| item["key"].as_str() == Some(key))
        .unwrap_or_else(|| panic!("{group} should contain setup item {key}"))
}

#[derive(Debug)]
struct SourcesStatusRow {
    enabled: bool,
    status: String,
    label: String,
    reason: String,
}

fn parse_sources_status_rows(stdout: &str) -> BTreeMap<String, SourcesStatusRow> {
    let mut rows = BTreeMap::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed == "Source setup status:"
            || trimmed == "Next:"
            || trimmed.starts_with("source_key")
            || trimmed.starts_with("Reason:")
            || trimmed.chars().next().is_some_and(|ch| ch.is_ascii_digit())
        {
            continue;
        }

        let Some(key) = line.get(0..11).map(str::trim).filter(|key| !key.is_empty()) else {
            continue;
        };
        let Some(enabled) = line.get(12..19).map(str::trim) else {
            continue;
        };
        if enabled != "yes" && enabled != "no" {
            continue;
        }
        let status = line.get(20..38).unwrap_or_default().trim().to_string();
        let label = line.get(39..54).unwrap_or_default().trim().to_string();
        let reason = line.get(55..).unwrap_or_default().trim().to_string();
        rows.insert(
            key.to_string(),
            SourcesStatusRow {
                enabled: enabled == "yes",
                status,
                label,
                reason,
            },
        );
    }
    rows
}

#[test]
fn intake_default_out_paths_do_not_duplicate_current_dir() {
    let tmp = TempDir::new().unwrap();
    let fixtures = fixture_dir();
    let events = fixtures
        .join("ledger.events.jsonl")
        .display()
        .to_string()
        .replace('\\', "/");
    let coverage = fixtures
        .join("coverage.manifest.json")
        .display()
        .to_string()
        .replace('\\', "/");
    write_manual_events(&tmp.path().join("manual_events.yaml"));
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        format!(
            r#"[shiplog]
config_version = 1

[defaults]
out = "./out"
window = "year:2025"
profile = "internal"

[sources.json]
enabled = true
events = "{events}"
coverage = "{coverage}"

[sources.manual]
enabled = true
events = "./manual_events.yaml"
"#
        ),
    )
    .unwrap();

    let assert = shiplog_cmd()
        .current_dir(tmp.path())
        .args(["intake", "--no-open", "--explain"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let normalized_stdout = stdout.replace('\\', "/");

    assert!(
        !normalized_stdout.contains("././out"),
        "default out display should not duplicate the current directory. stdout:\n{stdout}"
    );
    assert!(
        !stdout.contains("./out\\"),
        "default out display should not mix slash styles. stdout:\n{stdout}"
    );
    assert!(
        normalized_stdout.contains("Packet: ./out/"),
        "default out paths should stay copyable and compact. stdout:\n{stdout}"
    );
    assert!(
        normalized_stdout.contains("--out \"./out\""),
        "handoff commands should use the compact default out path. stdout:\n{stdout}"
    );

    let run_dir = first_run_dir(&tmp.path().join("out"));
    let report_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(run_dir.join("intake.report.json")).unwrap())
            .unwrap();
    assert_eq!(
        report_json["out_dir"], "./out",
        "intake.report.json should preserve compact default out display"
    );
    for field in ["run_dir", "packet_path"] {
        let value = report_json[field]
            .as_str()
            .unwrap_or_else(|| panic!("intake report field {field} should be a string"));
        assert!(
            !value.contains('\\'),
            "intake report {field} should use stable slash display: {value:?}"
        );
    }
    for artifact in report_json["artifacts"]
        .as_array()
        .expect("intake report should expose artifacts")
    {
        let path = artifact["path"]
            .as_str()
            .expect("artifact path should be a string");
        assert!(
            !path.contains('\\'),
            "artifact paths should use stable slash display: {path:?}"
        );
    }

    let repair_plan = shiplog_cmd()
        .current_dir(tmp.path())
        .args(["repair", "plan", "--out", "./out", "--latest"])
        .assert()
        .success();
    let repair_plan_stdout = String::from_utf8(repair_plan.get_output().stdout.clone()).unwrap();
    assert!(
        !repair_plan_stdout.contains("./out\\"),
        "repair plan should not mix slash styles. stdout:\n{repair_plan_stdout}"
    );
    assert!(
        repair_plan_stdout.contains("Repair plan: ./out/"),
        "repair plan should use stable slash display. stdout:\n{repair_plan_stdout}"
    );

    let share_explain = shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("SHIPLOG_REDACT_KEY")
        .args(["share", "explain", "manager", "--out", "./out", "--latest"])
        .assert()
        .success();
    let share_explain_stdout =
        String::from_utf8(share_explain.get_output().stdout.clone()).unwrap();
    assert!(
        !share_explain_stdout.contains("./out\\"),
        "share explain should not mix slash styles. stdout:\n{share_explain_stdout}"
    );
    assert!(
        share_explain_stdout.contains("Directory: ./out/"),
        "share explain should use stable slash display. stdout:\n{share_explain_stdout}"
    );
}

#[test]
fn doctor_requires_redaction_key_for_share_profile_defaults() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[defaults]
profile = "manager"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("SHIPLOG_REDACT_KEY")
        .args(["doctor"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Redaction: error"))
        .stdout(predicate::str::contains("SHIPLOG_REDACT_KEY"));
}

#[test]
fn doctor_repair_plan_reports_missing_token_without_writing_outputs() {
    let tmp = TempDir::new().unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .assert()
        .success();

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .args(["doctor", "--repair-plan"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Repair plan:"))
        .stdout(predicate::str::contains("GitHub [missing_token]"))
        .stdout(predicate::str::contains(
            "shiplog sources status --config \"shiplog.toml\" --source github",
        ))
        .stdout(predicate::str::contains(
            "shiplog doctor --config \"shiplog.toml\" --repair-plan",
        ));

    assert!(
        !tmp.path().join("out").exists(),
        "doctor repair plan should not write run artifacts"
    );
}

#[test]
fn doctor_repair_plan_succeeds_for_fixture_safe_sources_without_writing_outputs() {
    let tmp = TempDir::new().unwrap();
    let fixtures = fixture_dir();
    std::fs::copy(
        fixtures.join("ledger.events.jsonl"),
        tmp.path().join("ledger.events.jsonl"),
    )
    .unwrap();
    std::fs::copy(
        fixtures.join("coverage.manifest.json"),
        tmp.path().join("coverage.manifest.json"),
    )
    .unwrap();
    write_manual_events(&tmp.path().join("manual_events.yaml"));
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[defaults]
out = "./out"
window = "last-6-months"
profile = "internal"

[sources.json]
enabled = true
events = "./ledger.events.jsonl"
coverage = "./coverage.manifest.json"

[sources.manual]
enabled = true
events = "./manual_events.yaml"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["doctor", "--repair-plan"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Repair plan:"))
        .stdout(predicate::str::contains("No repair actions found."));

    assert!(
        !tmp.path().join("out").exists(),
        "doctor repair plan should not create defaults.out"
    );
}

#[test]
fn doctor_repair_plan_classifies_source_setup_issues() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[defaults]
profile = "manager"

[sources.gitlab]
enabled = true
user = "steven"
state = "finished"

[sources.jira]
enabled = true
user = "712020:account-id"
instance = "company.atlassian.net"

[sources.linear]
enabled = true
status = "done"

[sources.manual]
enabled = true
events = "./missing_manual_events.yaml"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .env_remove("SHIPLOG_REDACT_KEY")
        .args(["doctor", "--repair-plan"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Redaction [missing_token]"))
        .stdout(predicate::str::contains("GitLab [invalid_filter]"))
        .stdout(predicate::str::contains("Jira [missing_token]"))
        .stdout(predicate::str::contains("Linear [missing_identity]"))
        .stdout(predicate::str::contains("Manual [missing_file]"))
        .stdout(predicate::str::contains(
            "shiplog doctor --config \"shiplog.toml\" --repair-plan",
        ));
}

#[test]
fn config_validate_accepts_fixture_safe_sources() {
    let tmp = TempDir::new().unwrap();
    let fixtures = fixture_dir();
    std::fs::copy(
        fixtures.join("ledger.events.jsonl"),
        tmp.path().join("ledger.events.jsonl"),
    )
    .unwrap();
    std::fs::copy(
        fixtures.join("coverage.manifest.json"),
        tmp.path().join("coverage.manifest.json"),
    )
    .unwrap();
    write_manual_events(&tmp.path().join("manual_events.yaml"));
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[defaults]
out = "./out"
window = "year:2025"
profile = "internal"

[sources.json]
enabled = true
events = "./ledger.events.jsonl"
coverage = "./coverage.manifest.json"

[sources.manual]
enabled = true
events = "./manual_events.yaml"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["config", "validate"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Config: ok"))
        .stdout(predicate::str::contains("Window: ok, 2025"))
        .stdout(predicate::str::contains("Sources: ok, json, manual"))
        .stdout(predicate::str::contains("Config valid"));
}

#[test]
fn config_validate_rejects_invalid_named_periods() {
    let tmp = TempDir::new().unwrap();
    write_manual_events(&tmp.path().join("manual_events.yaml"));
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[periods."broken"]
since = "2026-01-01"

[sources.manual]
enabled = true
events = "./manual_events.yaml"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["config", "validate"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Period: error"))
        .stdout(predicate::str::contains("broken"))
        .stdout(predicate::str::contains("both since and until"));
}

#[test]
fn example_configs_validate_without_source_tokens() {
    for name in [
        "github-only.toml",
        "github-gitlab-jira-manual.toml",
        "local-git-json-manual.toml",
        "public-portfolio.toml",
    ] {
        shiplog_cmd()
            .current_dir(repo_root())
            .env_remove("GITHUB_TOKEN")
            .env_remove("GITLAB_TOKEN")
            .env_remove("JIRA_TOKEN")
            .env_remove("LINEAR_API_KEY")
            .env_remove("SHIPLOG_REDACT_KEY")
            .args([
                "config",
                "validate",
                "--config",
                example_config(name).to_str().unwrap(),
            ])
            .assert()
            .success()
            .stdout(predicate::str::contains("Version: ok, 1"))
            .stdout(predicate::str::contains("Config valid"));
    }
}

#[test]
fn local_example_config_explains_fixture_sources() {
    shiplog_cmd()
        .current_dir(repo_root())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args([
            "config",
            "explain",
            "--config",
            example_config("local-git-json-manual.toml")
                .to_str()
                .unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Config metadata:"))
        .stdout(predicate::str::contains("- config_version: 1"))
        .stdout(predicate::str::contains("Enabled sources:"))
        .stdout(predicate::str::contains("- git: repo"))
        .stdout(predicate::str::contains("- json: events"))
        .stdout(predicate::str::contains("ledger.events.jsonl"))
        .stdout(predicate::str::contains("- manual: events"));
}

#[test]
fn config_validate_does_not_require_source_tokens() {
    let tmp = TempDir::new().unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .assert()
        .success();

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .args(["config", "validate"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Sources: ok, github, manual"));
}

#[test]
fn legacy_config_without_version_remains_supported() {
    let tmp = TempDir::new().unwrap();
    write_manual_events(&tmp.path().join("manual_events.yaml"));
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[defaults]
window = "year:2025"

[sources.manual]
enabled = true
events = "./manual_events.yaml"
user = "octo"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["config", "validate"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Version: ok, 1 (implicit)"));

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Config version: ok, 1 (implicit)"));

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["collect", "multi"])
        .assert()
        .success()
        .stdout(predicate::str::contains("- manual: success"));

    let run_dir = first_run_dir(&tmp.path().join("out"));
    assert!(run_dir.join("packet.md").exists(), "missing packet");
}

#[test]
fn unsupported_config_version_fails_before_collection_side_effects() {
    let tmp = TempDir::new().unwrap();
    write_manual_events(&tmp.path().join("manual_events.yaml"));
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[shiplog]
config_version = 2

[sources.manual]
enabled = true
events = "./manual_events.yaml"
user = "octo"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["config", "validate"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Version: error"))
        .stdout(predicate::str::contains("unsupported config_version 2"));

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["doctor"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Config version: error"))
        .stdout(predicate::str::contains("unsupported config_version 2"));

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["collect", "multi"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unsupported config_version 2"));

    assert!(
        !tmp.path().join("out").exists(),
        "unsupported config version should fail before writing outputs"
    );
}

#[test]
fn config_validate_reports_missing_config_actionably() {
    let tmp = TempDir::new().unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["config", "validate"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("run `shiplog init` first"));
}

#[test]
fn config_validate_rejects_no_enabled_sources() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[defaults]
window = "last-6-months"
profile = "internal"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["config", "validate"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Sources: error"))
        .stdout(predicate::str::contains("enable at least one"));
}

#[test]
fn config_validate_rejects_missing_file_source_paths() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[sources.manual]
enabled = true
events = "./missing.yaml"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["config", "validate"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Manual: error"))
        .stdout(predicate::str::contains("missing.yaml"));
}

#[test]
fn config_validate_rejects_invalid_default_out_file() {
    let tmp = TempDir::new().unwrap();
    write_manual_events(&tmp.path().join("manual_events.yaml"));
    std::fs::write(tmp.path().join("out-file"), "not a directory").unwrap();
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[defaults]
out = "./out-file"

[sources.manual]
enabled = true
events = "./manual_events.yaml"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["config", "validate"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Output: error"))
        .stdout(predicate::str::contains("exists but is not a directory"));
}

#[test]
fn config_migrate_dry_run_leaves_legacy_config_unchanged() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("shiplog.toml");
    let original = r#"[sources.manual]
enabled = true
events = "./manual_events.yaml"
"#;
    std::fs::write(&config_path, original).unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["config", "migrate", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "would add [shiplog] config_version = 1",
        ));

    let after = std::fs::read_to_string(config_path).unwrap();
    assert_eq!(after, original);
}

#[test]
fn config_migrate_requires_existing_config_even_for_dry_run() {
    let tmp = TempDir::new().unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["config", "migrate", "--dry-run"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("run `shiplog init` first"));

    assert!(!tmp.path().join("shiplog.toml").exists());
}

#[test]
fn config_migrate_adds_explicit_version_to_legacy_config() {
    let tmp = TempDir::new().unwrap();
    write_manual_events(&tmp.path().join("manual_events.yaml"));
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[sources.manual]
enabled = true
events = "./manual_events.yaml"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["config", "migrate"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "added [shiplog] config_version = 1",
        ))
        .stdout(predicate::str::contains("Config migrated"));

    let config = std::fs::read_to_string(tmp.path().join("shiplog.toml")).unwrap();
    assert!(config.starts_with("[shiplog]\nconfig_version = 1\n\n"));

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["config", "validate"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Version: ok, 1"));
}

#[test]
fn config_migrate_inserts_version_into_existing_shiplog_table() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[shiplog]
# local config metadata
schema_note = "keep this"

[sources.manual]
enabled = false
"#,
    )
    .unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["config", "migrate"])
        .assert()
        .success();

    let config = std::fs::read_to_string(tmp.path().join("shiplog.toml")).unwrap();
    assert_eq!(config.matches("[shiplog]").count(), 1);
    assert!(config.contains("[shiplog]\nconfig_version = 1\n# local config metadata"));
    assert!(config.contains("schema_note = \"keep this\""));
}

#[test]
fn config_migrate_current_config_is_noop() {
    let tmp = TempDir::new().unwrap();
    let original = r#"[shiplog]
config_version = 1

[sources.manual]
enabled = false
"#;
    std::fs::write(tmp.path().join("shiplog.toml"), original).unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["config", "migrate"])
        .assert()
        .success()
        .stdout(predicate::str::contains("already current"));

    let after = std::fs::read_to_string(tmp.path().join("shiplog.toml")).unwrap();
    assert_eq!(after, original);
}

#[test]
fn config_migrate_unsupported_version_fails_without_writing() {
    let tmp = TempDir::new().unwrap();
    let original = r#"[shiplog]
config_version = 2

[sources.manual]
enabled = false
"#;
    let config_path = tmp.path().join("shiplog.toml");
    std::fs::write(&config_path, original).unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["config", "migrate"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unsupported config_version 2"));

    let after = std::fs::read_to_string(config_path).unwrap();
    assert_eq!(after, original);
}

#[test]
fn config_explain_prints_effective_defaults_and_sources() {
    let tmp = TempDir::new().unwrap();
    let fixtures = fixture_dir();
    std::fs::copy(
        fixtures.join("ledger.events.jsonl"),
        tmp.path().join("ledger.events.jsonl"),
    )
    .unwrap();
    std::fs::copy(
        fixtures.join("coverage.manifest.json"),
        tmp.path().join("coverage.manifest.json"),
    )
    .unwrap();
    write_manual_events(&tmp.path().join("manual_events.yaml"));
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[defaults]
out = "./review-out"
window = "year:2025"
profile = "internal"
include_reviews = true

[user]
label = "Octo"

[sources.json]
enabled = true
events = "./ledger.events.jsonl"
coverage = "./coverage.manifest.json"

[sources.manual]
enabled = true
events = "./manual_events.yaml"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["config", "explain"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Config metadata:"))
        .stdout(predicate::str::contains("- config_version: 1 (implicit)"))
        .stdout(predicate::str::contains("Resolved defaults:"))
        .stdout(predicate::str::contains(
            "- window: year:2025 -> 2025-01-01..2026-01-01",
        ))
        .stdout(predicate::str::contains("- out: ./review-out ->"))
        .stdout(predicate::str::contains("- user.label: Octo"))
        .stdout(predicate::str::contains("Configured periods:"))
        .stdout(predicate::str::contains("- none"))
        .stdout(predicate::str::contains("Enabled sources:"))
        .stdout(predicate::str::contains("- json: events"))
        .stdout(predicate::str::contains("- manual: events"))
        .stdout(predicate::str::contains("user Octo"));
}

#[test]
fn config_explain_prints_named_periods() {
    let tmp = TempDir::new().unwrap();
    write_manual_events(&tmp.path().join("manual_events.yaml"));
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[defaults]
window = "year:2025"

[periods."2026-H1"]
since = "2026-01-01"
until = "2026-07-01"

[periods."review-cycle"]
preset = "year:2025"

[sources.manual]
enabled = true
events = "./manual_events.yaml"
user = "octo"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["config", "explain"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Configured periods:"))
        .stdout(predicate::str::contains(
            "- 2026-H1: 2026-H1 (2026-01-01..2026-07-01)",
        ))
        .stdout(predicate::str::contains(
            "- review-cycle: review-cycle (2025-01-01..2026-01-01)",
        ));
}

#[test]
fn periods_list_prints_named_windows_without_existing_runs() {
    let tmp = TempDir::new().unwrap();
    write_manual_events(&tmp.path().join("manual_events.yaml"));
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[periods."2026-H1"]
since = "2026-01-01"
until = "2026-07-01"

[periods."review-cycle"]
preset = "year:2025"

[sources.manual]
enabled = true
events = "./manual_events.yaml"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["periods", "list", "--out", "out"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Configured periods:"))
        .stdout(predicate::str::contains("- 2026-H1"))
        .stdout(predicate::str::contains(
            "definition: explicit 2026-01-01..2026-07-01",
        ))
        .stdout(predicate::str::contains("window: 2026-01-01..2026-07-01"))
        .stdout(predicate::str::contains("latest run: none"))
        .stdout(predicate::str::contains(
            "shiplog intake --config \"shiplog.toml\" --out \"out\" --period \"2026-H1\"",
        ))
        .stdout(predicate::str::contains("- review-cycle"))
        .stdout(predicate::str::contains("definition: preset year:2025"));
}

#[test]
fn periods_explain_shows_latest_matching_run_and_next_commands() {
    let tmp = TempDir::new().unwrap();
    write_manual_events(&tmp.path().join("manual_events.yaml"));
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[periods."review-cycle"]
preset = "year:2025"

[sources.manual]
enabled = true
events = "./manual_events.yaml"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args([
            "intake",
            "--period",
            "review-cycle",
            "--out",
            "out",
            "--no-open",
        ])
        .assert()
        .success();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["periods", "explain", "review-cycle", "--out", "out"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Period: review-cycle"))
        .stdout(predicate::str::contains("Definition: preset year:2025"))
        .stdout(predicate::str::contains("Window: 2025-01-01..2026-01-01"))
        .stdout(predicate::str::contains("Latest run: "))
        .stdout(predicate::str::contains("Coverage:"))
        .stdout(predicate::str::contains(
            "shiplog review --config \"shiplog.toml\" --out \"out\" --period \"review-cycle\"",
        ));
}

#[test]
fn periods_explain_unknown_period_fails_clearly() {
    let tmp = TempDir::new().unwrap();
    write_manual_events(&tmp.path().join("manual_events.yaml"));
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[periods."review-cycle"]
preset = "year:2025"

[sources.manual]
enabled = true
events = "./manual_events.yaml"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["periods", "explain", "missing"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown period \"missing\""));
}

// ── 3. collect --help shows collect-specific options ───────────────────────

#[test]
fn collect_help_shows_sources_and_options() {
    shiplog_cmd()
        .args(["collect", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("github"))
        .stdout(predicate::str::contains("multi"))
        .stdout(predicate::str::contains("gitlab"))
        .stdout(predicate::str::contains("jira"))
        .stdout(predicate::str::contains("linear"))
        .stdout(predicate::str::contains("json"))
        .stdout(predicate::str::contains("--out"))
        .stdout(predicate::str::contains("--regen"));
}

#[test]
fn collect_multi_help_shows_config_and_conflict_policy() {
    shiplog_cmd()
        .args(["collect", "multi", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--config"))
        .stdout(predicate::str::contains("--period"))
        .stdout(predicate::str::contains("--last-6-months"))
        .stdout(predicate::str::contains("--conflict"))
        .stdout(predicate::str::contains("prefer-most-recent"));
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
        .stdout(predicate::str::contains("--mode"))
        .stdout(predicate::str::contains("--receipt-limit"))
        .stdout(predicate::str::contains("--appendix"))
        .stdout(predicate::str::contains("--redact-key"));
}

#[test]
fn cache_stats_and_inspect_report_existing_cache_without_tokens() {
    let tmp = TempDir::new().unwrap();
    let cache_dir = tmp.path().join(".cache");
    let cache_path = seed_github_cache(&cache_dir);

    shiplog_cmd()
        .args([
            "cache",
            "stats",
            "--cache-dir",
            cache_dir.to_str().unwrap(),
            "--source",
            "github",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cache root:"))
        .stdout(predicate::str::contains("github:"))
        .stdout(predicate::str::contains(cache_path.display().to_string()))
        .stdout(predicate::str::contains(
            "entries: total 2, valid 1, expired 1",
        ));

    shiplog_cmd()
        .args([
            "cache",
            "inspect",
            "--cache-dir",
            cache_dir.to_str().unwrap(),
            "--source",
            "github",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("oldest:"))
        .stdout(predicate::str::contains("newest:"));
}

#[test]
fn cache_stats_does_not_create_missing_databases() {
    let tmp = TempDir::new().unwrap();
    let cache_dir = tmp.path().join(".cache");
    std::fs::create_dir_all(&cache_dir).unwrap();
    let missing = cache_dir.join("gitlab-api-cache.db");

    shiplog_cmd()
        .args([
            "cache",
            "stats",
            "--cache-dir",
            cache_dir.to_str().unwrap(),
            "--source",
            "gitlab",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("gitlab: missing"))
        .stdout(predicate::str::contains("No cache databases found"));

    assert!(
        !missing.exists(),
        "stats should not create a missing cache database"
    );
}

#[test]
fn cache_stats_and_inspect_do_not_initialize_existing_empty_file() {
    let tmp = TempDir::new().unwrap();
    let cache_dir = tmp.path().join(".cache");
    std::fs::create_dir_all(&cache_dir).unwrap();
    let empty_cache = cache_dir.join("github-api-cache.db");
    std::fs::File::create(&empty_cache).unwrap();
    assert_eq!(std::fs::metadata(&empty_cache).unwrap().len(), 0);

    shiplog_cmd()
        .args([
            "cache",
            "stats",
            "--cache-dir",
            cache_dir.to_str().unwrap(),
            "--source",
            "github",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cache_entries"));
    assert_eq!(
        std::fs::metadata(&empty_cache).unwrap().len(),
        0,
        "stats should not initialize an existing empty cache file"
    );

    shiplog_cmd()
        .args([
            "cache",
            "inspect",
            "--cache-dir",
            cache_dir.to_str().unwrap(),
            "--source",
            "github",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cache_entries"));
    assert_eq!(
        std::fs::metadata(&empty_cache).unwrap().len(),
        0,
        "inspect should not initialize an existing empty cache file"
    );
}

#[test]
fn cache_clean_expired_removes_only_expired_entries() {
    let tmp = TempDir::new().unwrap();
    let cache_dir = tmp.path().join(".cache");
    let cache_path = seed_github_cache(&cache_dir);

    shiplog_cmd()
        .args([
            "cache",
            "clean",
            "--cache-dir",
            cache_dir.to_str().unwrap(),
            "--source",
            "github",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("github: removed 1 entries"));

    let cache = ApiCache::open(cache_path).unwrap();
    let stats = cache.stats().unwrap();
    assert_eq!(stats.total_entries, 1);
    assert_eq!(stats.valid_entries, 1);
    assert_eq!(stats.expired_entries, 0);
}

#[test]
fn cache_clean_older_than_dry_run_reports_without_mutating() {
    let tmp = TempDir::new().unwrap();
    let cache_dir = tmp.path().join(".cache");
    let cache_path = seed_github_cache(&cache_dir);

    shiplog_cmd()
        .args([
            "cache",
            "clean",
            "--cache-dir",
            cache_dir.to_str().unwrap(),
            "--source",
            "github",
            "--older-than",
            "0m",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("github: would remove 2 entries"));

    let cache = ApiCache::open(cache_path).unwrap();
    assert_eq!(cache.stats().unwrap().total_entries, 2);
}

#[test]
fn cache_clean_all_requires_yes_and_preserves_unrelated_files() {
    let tmp = TempDir::new().unwrap();
    let cache_dir = tmp.path().join(".cache");
    let cache_path = seed_github_cache(&cache_dir);
    let unrelated = cache_dir.join("notes.txt");
    std::fs::write(&unrelated, "not a cache database").unwrap();

    shiplog_cmd()
        .args([
            "cache",
            "clean",
            "--cache-dir",
            cache_dir.to_str().unwrap(),
            "--source",
            "github",
            "--all",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cache clean --all requires --yes"));

    assert_eq!(
        ApiCache::open(&cache_path)
            .unwrap()
            .stats()
            .unwrap()
            .total_entries,
        2
    );

    shiplog_cmd()
        .args([
            "cache",
            "clean",
            "--cache-dir",
            cache_dir.to_str().unwrap(),
            "--source",
            "github",
            "--all",
            "--yes",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("github: removed 2 entries"));

    assert!(
        cache_path.exists(),
        "clean should not delete cache database files"
    );
    assert!(
        unrelated.exists(),
        "clean should not delete unrelated files"
    );
    assert_eq!(
        ApiCache::open(cache_path)
            .unwrap()
            .stats()
            .unwrap()
            .total_entries,
        0
    );
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
fn collect_multi_from_config_merges_json_and_manual_sources() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");
    let fixtures = fixture_dir();
    std::fs::copy(
        fixtures.join("ledger.events.jsonl"),
        tmp.path().join("ledger.events.jsonl"),
    )
    .unwrap();
    std::fs::copy(
        fixtures.join("coverage.manifest.json"),
        tmp.path().join("coverage.manifest.json"),
    )
    .unwrap();
    write_manual_events(&tmp.path().join("manual_events.yaml"));

    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[defaults]
window = "year:2025"
include_reviews = true

[user]
label = "octo"

[sources.json]
enabled = true
events = "./ledger.events.jsonl"
coverage = "./coverage.manifest.json"

[sources.manual]
enabled = true
events = "./manual_events.yaml"
user = "octo"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .args([
            "collect",
            "--out",
            out.to_str().unwrap(),
            "multi",
            "--config",
            tmp.path().join("shiplog.toml").to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Collected configured sources:"))
        .stdout(predicate::str::contains("- json: success"))
        .stdout(predicate::str::contains("- manual: success, 1 event"))
        .stdout(predicate::str::contains("Merged and wrote:"));

    let run_dir = first_run_dir(&out);
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
    assert_packet_opens_with_coverage(&packet);
    assert_packet_uses_summary_appendix(&packet);
    assert!(
        packet.contains("Included:\n- GitHub: 3 events\n- Manual: 1 event\n"),
        "configured multi packet should summarize successful sources in Coverage and Limits"
    );
    assert!(
        packet.contains("Skipped:\n- None recorded\n"),
        "configured multi packet should make absence of skipped sources explicit"
    );
    assert!(
        packet.contains("Known gaps:\n- Manual events are user-provided\n"),
        "configured multi packet should flag manual evidence as user-provided"
    );
    assert!(
        packet.contains("- **Sources:** GitHub, Manual"),
        "configured multi packet should carry merged source details"
    );
    assert!(
        packet.contains("Payments ledger rewrite"),
        "configured multi packet should include JSON fixture evidence"
    );
    assert!(
        packet.contains("Manual incident follow-up"),
        "configured multi packet should include manual evidence"
    );
    assert!(
        packet.contains("workstreams.suggested.yaml"),
        "configured multi packet should name generated workstream suggestions"
    );
    assert!(
        packet.contains("**Suggested claim prompts**"),
        "configured multi packet should include claim prompts for review-cycle writing"
    );
    assert!(
        packet.contains("bundle.manifest.json"),
        "configured multi packet should name the generated bundle manifest"
    );
    assert!(
        !packet.contains("manual_events.yaml"),
        "configured multi packet should not list manual_events.yaml as a generated run artifact"
    );

    let coverage = std::fs::read_to_string(run_dir.join("coverage.manifest.json")).unwrap();
    assert!(
        coverage.contains("\"github\""),
        "configured multi coverage should include JSON fixture source"
    );
    assert!(
        coverage.contains("\"manual\""),
        "configured multi coverage should include manual source"
    );
}

#[test]
fn collect_multi_uses_named_config_period() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");
    let fixtures = fixture_dir();
    std::fs::copy(
        fixtures.join("ledger.events.jsonl"),
        tmp.path().join("ledger.events.jsonl"),
    )
    .unwrap();
    std::fs::copy(
        fixtures.join("coverage.manifest.json"),
        tmp.path().join("coverage.manifest.json"),
    )
    .unwrap();

    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[defaults]
window = "year:2025"

[periods."review-cycle"]
since = "2026-01-01"
until = "2026-07-01"

[sources.json]
enabled = true
events = "./ledger.events.jsonl"
coverage = "./coverage.manifest.json"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .args([
            "collect",
            "--out",
            out.to_str().unwrap(),
            "multi",
            "--config",
            tmp.path().join("shiplog.toml").to_str().unwrap(),
            "--period",
            "review-cycle",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Collected configured sources:"));

    let run_dir = first_run_dir(&out);
    let coverage: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(run_dir.join("coverage.manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(coverage["window"]["since"], "2026-01-01");
    assert_eq!(coverage["window"]["until"], "2026-07-01");

    shiplog_cmd()
        .args([
            "review",
            "--out",
            out.to_str().unwrap(),
            "--config",
            tmp.path().join("shiplog.toml").to_str().unwrap(),
            "--period",
            "review-cycle",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Window: 2026-01-01..2026-07-01"))
        .stdout(predicate::str::contains("Evidence debt:"));
}

#[test]
fn collect_multi_cli_dates_override_named_period() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");
    let fixtures = fixture_dir();
    std::fs::copy(
        fixtures.join("ledger.events.jsonl"),
        tmp.path().join("ledger.events.jsonl"),
    )
    .unwrap();
    std::fs::copy(
        fixtures.join("coverage.manifest.json"),
        tmp.path().join("coverage.manifest.json"),
    )
    .unwrap();

    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[periods."review-cycle"]
since = "2026-01-01"
until = "2026-07-01"

[sources.json]
enabled = true
events = "./ledger.events.jsonl"
coverage = "./coverage.manifest.json"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .args([
            "collect",
            "--out",
            out.to_str().unwrap(),
            "multi",
            "--config",
            tmp.path().join("shiplog.toml").to_str().unwrap(),
            "--period",
            "review-cycle",
            "--year",
            "2025",
        ])
        .assert()
        .success();

    let run_dir = first_run_dir(&out);
    let coverage: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(run_dir.join("coverage.manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(coverage["window"]["since"], "2025-01-01");
    assert_eq!(coverage["window"]["until"], "2026-01-01");
}

#[test]
fn collect_multi_unknown_period_fails_clearly() {
    let tmp = TempDir::new().unwrap();
    let fixtures = fixture_dir();
    std::fs::copy(
        fixtures.join("ledger.events.jsonl"),
        tmp.path().join("ledger.events.jsonl"),
    )
    .unwrap();
    std::fs::copy(
        fixtures.join("coverage.manifest.json"),
        tmp.path().join("coverage.manifest.json"),
    )
    .unwrap();
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[sources.json]
enabled = true
events = "./ledger.events.jsonl"
coverage = "./coverage.manifest.json"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .args([
            "collect",
            "multi",
            "--config",
            tmp.path().join("shiplog.toml").to_str().unwrap(),
            "--period",
            "missing",
        ])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("unknown period")
                .and(predicate::str::contains("[periods.\"missing\"]")),
        );
}

#[test]
fn intake_uses_named_config_period_and_records_report_window() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");
    let fixtures = fixture_dir();
    std::fs::copy(
        fixtures.join("ledger.events.jsonl"),
        tmp.path().join("ledger.events.jsonl"),
    )
    .unwrap();
    std::fs::copy(
        fixtures.join("coverage.manifest.json"),
        tmp.path().join("coverage.manifest.json"),
    )
    .unwrap();

    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[periods."review-cycle"]
since = "2026-01-01"
until = "2026-07-01"

[sources.json]
enabled = true
events = "./ledger.events.jsonl"
coverage = "./coverage.manifest.json"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .args([
            "intake",
            "--config",
            tmp.path().join("shiplog.toml").to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
            "--period",
            "review-cycle",
            "--no-open",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Review intake complete."));

    let run_dir = first_run_dir(&out);
    let (_, report) = assert_golden_intake_report(&run_dir, "Ready for review");
    assert_eq!(report["period"], "review-cycle");
    assert_eq!(report["window"]["since"], "2026-01-01");
    assert_eq!(report["window"]["until"], "2026-07-01");
    assert_eq!(
        report["window"]["label"],
        "review-cycle (2026-01-01..2026-07-01)"
    );
}

#[test]
fn collect_json_all_source_fixture_packet_is_coherent() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");
    let events_path = tmp.path().join("ledger.events.jsonl");
    let coverage_path = tmp.path().join("coverage.manifest.json");
    let events = all_source_fixture_events();
    let coverage = all_source_fixture_coverage();
    write_events_jsonl(&events_path, &events);
    write_coverage_manifest(&coverage_path, &coverage);

    shiplog_cmd()
        .args([
            "collect",
            "--out",
            out.to_str().unwrap(),
            "json",
            "--events",
            events_path.to_str().unwrap(),
            "--coverage",
            coverage_path.to_str().unwrap(),
            "--user",
            "octo",
            "--window-label",
            "all-source fixture",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Collected and wrote:"));

    let run_dir = out.join("run_all_sources");
    assert!(run_dir.join("packet.md").exists(), "missing packet");
    assert!(
        run_dir.join("ledger.events.jsonl").exists(),
        "missing ledger"
    );
    assert!(
        run_dir.join("coverage.manifest.json").exists(),
        "missing coverage"
    );

    let packet = std::fs::read_to_string(run_dir.join("packet.md")).unwrap();
    assert_packet_opens_with_coverage(&packet);
    assert_packet_uses_summary_appendix(&packet);

    for expected in [
        "- GitHub: 2 events",
        "- GitLab: 1 event",
        "- Jira: 1 event",
        "- Linear: 1 event",
        "- Local git: 1 event",
        "- Manual: 1 event",
        "- JSON import: 1 event",
    ] {
        assert!(
            packet.contains(expected),
            "all-source packet should include source summary line {expected:?}"
        );
    }

    assert!(
        packet.contains("Skipped:\n- None recorded\n"),
        "all-source packet should make absence of skipped sources explicit"
    );
    assert!(
        packet.contains("Known gaps:\n- Manual events are user-provided\n"),
        "all-source packet should keep user-provided manual evidence visible as a gap"
    );
    assert!(
        packet.contains(
            "- **Sources:** GitHub, GitLab, Jira, Linear, Local git, Manual, JSON import"
        ),
        "all-source packet should carry readable source details"
    );

    for title in [
        "GitHub release automation",
        "GitLab self-hosted deploy fix",
        "Jira OPS-42 rollout checklist",
        "Linear issue triage",
        "Local git hotfix commit",
        "Manual customer debugging note",
        "Imported architecture decision",
    ] {
        assert!(
            packet.contains(title),
            "all-source packet should include evidence title {title:?}"
        );
    }
    assert!(
        packet.contains("[Review] approved"),
        "all-source packet should include the review receipt"
    );

    assert!(
        packet.contains("**Suggested claim prompts**"),
        "all-source packet should include claim prompts for review-cycle writing"
    );
    assert!(
        packet.contains("workstreams.suggested.yaml"),
        "all-source packet should name generated workstream suggestions"
    );
    assert!(
        packet.contains("bundle.manifest.json"),
        "all-source packet should name the generated bundle manifest"
    );

    let written_coverage = std::fs::read_to_string(run_dir.join("coverage.manifest.json")).unwrap();
    for source in [
        "\"github\"",
        "\"gitlab\"",
        "\"jira\"",
        "\"linear\"",
        "\"local_git\"",
        "\"manual\"",
        "\"json_import\"",
    ] {
        assert!(
            written_coverage.contains(source),
            "written coverage should preserve source {source}"
        );
    }
}

#[test]
fn collect_multi_uses_config_default_out_when_cli_out_is_omitted() {
    let tmp = TempDir::new().unwrap();
    let configured_out = tmp.path().join("configured-out");
    let fixtures = fixture_dir();
    std::fs::copy(
        fixtures.join("ledger.events.jsonl"),
        tmp.path().join("ledger.events.jsonl"),
    )
    .unwrap();
    std::fs::copy(
        fixtures.join("coverage.manifest.json"),
        tmp.path().join("coverage.manifest.json"),
    )
    .unwrap();
    write_manual_events(&tmp.path().join("manual_events.yaml"));

    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[defaults]
out = "./configured-out"
window = "year:2025"

[sources.json]
enabled = true
events = "./ledger.events.jsonl"
coverage = "./coverage.manifest.json"

[sources.manual]
enabled = true
events = "./manual_events.yaml"
user = "octo"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["collect", "multi"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Collected configured sources:"));

    let run_dir = first_run_dir(&configured_out);
    assert!(run_dir.join("packet.md").exists(), "missing merged packet");
    assert!(
        !tmp.path().join("out").exists(),
        "collect multi should not write ./out when defaults.out is set"
    );
}

#[test]
fn collect_multi_uses_config_default_profile_for_redaction_safety() {
    let tmp = TempDir::new().unwrap();
    write_manual_events(&tmp.path().join("manual_events.yaml"));

    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[defaults]
profile = "public"
window = "year:2025"

[redaction]
key_env = "SHIPLOG_TEST_REDACT_KEY_FOR_CONFIG"

[sources.manual]
enabled = true
events = "./manual_events.yaml"
user = "octo"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("SHIPLOG_TEST_REDACT_KEY_FOR_CONFIG")
        .args(["collect", "multi"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains(
                "public profile requires --redact-key or SHIPLOG_TEST_REDACT_KEY_FOR_CONFIG",
            )
            .and(predicate::str::contains("Try:"))
            .and(predicate::str::contains(redaction_key_env_command(
                "SHIPLOG_TEST_REDACT_KEY_FOR_CONFIG",
            )))
            .and(predicate::str::contains(
                "rerun this command with --bundle-profile public",
            )),
        );
}

#[test]
fn collect_multi_records_partial_source_failures() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");
    write_manual_events(&tmp.path().join("manual_events.yaml"));

    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[defaults]
window = "year:2025"

[sources.json]
enabled = true
events = "./missing-ledger.events.jsonl"
coverage = "./missing-coverage.manifest.json"

[sources.manual]
enabled = true
events = "./manual_events.yaml"
user = "octo"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .args([
            "collect",
            "--out",
            out.to_str().unwrap(),
            "multi",
            "--config",
            tmp.path().join("shiplog.toml").to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("- json: skipped"))
        .stdout(predicate::str::contains("- manual: success"))
        .stdout(predicate::str::contains("Merged and wrote:"));

    let run_dir = first_run_dir(&out);
    let packet = std::fs::read_to_string(run_dir.join("packet.md")).unwrap();
    assert!(
        packet.contains("Manual incident follow-up"),
        "configured multi packet should include successful source evidence"
    );
    assert!(
        packet.contains("Skipped:\n- JSON:"),
        "configured multi packet should list skipped sources in the packet coverage summary"
    );
    assert!(
        !packet
            .split("Included:")
            .nth(1)
            .unwrap()
            .split("Skipped:")
            .next()
            .unwrap()
            .contains("JSON"),
        "configured multi packet should not list skipped sources as included"
    );

    let coverage = std::fs::read_to_string(run_dir.join("coverage.manifest.json")).unwrap();
    assert!(
        coverage.contains("\"Partial\""),
        "configured multi coverage should mark skipped sources as partial"
    );
    assert!(
        coverage.contains("Configured source json was skipped"),
        "configured multi coverage should record skipped source warning"
    );

    let source_failures_path = run_dir.join("source.failures.json");
    assert!(
        source_failures_path.exists(),
        "configured multi should write source failure receipts"
    );
    let source_failures: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(source_failures_path).unwrap()).unwrap();
    assert_eq!(source_failures["schema_version"], 1);
    assert_eq!(source_failures["failures"].as_array().unwrap().len(), 1);
    assert_eq!(source_failures["failures"][0]["source"], "json");
    assert_eq!(source_failures["failures"][0]["kind"], "missing_file");
    assert!(
        source_failures["failures"][0]["rerun_command"]
            .as_str()
            .unwrap()
            .contains("shiplog intake --config"),
        "source failure receipt should include a safe rerun command"
    );
    assert!(
        std::fs::read_to_string(run_dir.join("bundle.manifest.json"))
            .unwrap()
            .contains("source.failures.json"),
        "bundle manifest should receipt the source failure artifact"
    );
}

#[test]
fn intake_from_config_writes_packet_and_review_next_steps() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");
    let fixtures = fixture_dir();
    std::fs::copy(
        fixtures.join("ledger.events.jsonl"),
        tmp.path().join("ledger.events.jsonl"),
    )
    .unwrap();
    std::fs::copy(
        fixtures.join("coverage.manifest.json"),
        tmp.path().join("coverage.manifest.json"),
    )
    .unwrap();
    write_manual_events(&tmp.path().join("manual_events.yaml"));

    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[defaults]
window = "year:2025"
include_reviews = true

[user]
label = "octo"

[sources.json]
enabled = true
events = "./ledger.events.jsonl"
coverage = "./coverage.manifest.json"

[sources.manual]
enabled = true
events = "./manual_events.yaml"
user = "octo"
"#,
    )
    .unwrap();

    let assert = shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args([
            "intake",
            "--out",
            out.to_str().unwrap(),
            "--config",
            tmp.path().join("shiplog.toml").to_str().unwrap(),
            "--last-6-months",
            "--no-open",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert!(stdout.contains("Review intake complete."));
    assert!(stdout.contains("- JSON: success"));
    assert!(stdout.contains("- Manual: success"));
    assert!(stdout.contains("Evidence debt:"));
    assert!(stdout.contains("Intake readiness:"));
    assert!(stdout.contains("Intake status: Needs curation"));
    assert!(stdout.contains("Packet readiness: Ready with caveats"));
    assert!(stdout.contains("- Packet rendered"));
    assert!(stdout.contains("- Review inspection completed"));
    assert!(stdout.contains("shiplog share explain manager --out"));
    assert!(!stdout.contains("--bundle-profile manager"));
    assert!(stdout.contains("Open:"));
    assert!(stdout.contains("Open later:"));
    assert_eq!(
        stdout.lines().filter(|line| *line == "Next:").count(),
        1,
        "intake should print one top-level next-action block. stdout:\n{stdout}"
    );

    let run_dir = first_run_dir(&out);
    assert!(run_dir.join("packet.md").exists(), "missing intake packet");
    assert!(
        run_dir.join("ledger.events.jsonl").exists(),
        "missing intake ledger"
    );
    assert!(
        run_dir.join("coverage.manifest.json").exists(),
        "missing intake coverage"
    );
    assert!(
        run_dir.join("workstreams.suggested.yaml").exists(),
        "missing intake workstream suggestions"
    );
    assert!(
        run_dir.join("bundle.manifest.json").exists(),
        "missing intake bundle manifest"
    );

    let packet = std::fs::read_to_string(run_dir.join("packet.md")).unwrap();
    assert_packet_opens_with_coverage(&packet);
    assert!(
        packet.contains("- GitHub: 3 events"),
        "intake packet should include successful configured sources"
    );
}

#[test]
fn golden_intake_manual_only_success_is_review_ready() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");
    write_manual_events(&tmp.path().join("manual_events.yaml"));

    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[defaults]
window = "year:2025"

[user]
label = "octo"

[sources.manual]
enabled = true
events = "./manual_events.yaml"
user = "octo"
"#,
    )
    .unwrap();

    let assert = shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args([
            "intake",
            "--out",
            out.to_str().unwrap(),
            "--config",
            tmp.path().join("shiplog.toml").to_str().unwrap(),
            "--year",
            "2025",
            "--no-open",
            "--explain",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert!(stdout.contains("Review intake complete."));
    assert!(stdout.contains("- Manual: success, 1 event"));
    assert!(stdout.contains("Skipped:\n- None"));
    assert!(stdout.contains("Source decisions:"));
    assert!(stdout.contains("- Manual: included, manual_events.yaml found"));
    assert!(stdout.contains("Intake readiness:"));
    assert!(stdout.contains("Intake status: Ready for review"));
    assert!(stdout.contains("Packet readiness: Ready with caveats"));
    assert!(stdout.contains("Needs attention:\n- None"));
    assert!(stdout.contains("shiplog share explain manager --out"));
    assert!(!stdout.contains("--bundle-profile manager"));
    assert!(stdout.contains("shiplog open packet --out"));

    let run_dir = first_run_dir(&out);
    assert_intake_artifacts(&run_dir);
    assert_ledger_event_count(&run_dir, 1);

    let (report_md, report_json) = assert_golden_intake_report(&run_dir, "Ready for review");
    assert!(report_md.contains("Intake status: **Ready for review**"));
    assert!(report_md.contains("Packet readiness: **Ready with caveats**"));
    assert!(report_md.contains("- Manual: 1 event"));
    assert!(report_md.contains("## Share Commands"));
    assert!(report_md.contains("shiplog share manager"));
    assert!(report_md.contains("SHIPLOG_REDACT_KEY"));

    assert_eq!(report_json["included_sources"][0]["source"], "manual");
    assert_eq!(report_json["included_sources"][0]["source_key"], "manual");
    assert_eq!(report_json["included_sources"][0]["source_label"], "Manual");
    assert_eq!(report_json["included_sources"][0]["event_count"], 1);
    assert_eq!(report_json["skipped_sources"].as_array().unwrap().len(), 0);
    assert!(
        !report_json["top_fixups"].as_array().unwrap().is_empty(),
        "manual-only intake should keep fixups visible in the report"
    );
    let evidence_debt = report_json["evidence_debt"].as_array().unwrap();
    assert!(
        evidence_debt
            .iter()
            .any(|item| item["kind"] == "manual-only-workstream"),
        "intake report JSON should include packet-quality evidence debt"
    );
    assert!(
        evidence_debt.iter().all(|item| item["next_step"]
            .as_str()
            .is_some_and(|step| !step.is_empty())),
        "intake report evidence debt should include actionable next steps"
    );
    assert!(
        report_json["share_commands"]
            .as_array()
            .unwrap()
            .iter()
            .any(|command| command.as_str().unwrap().contains("shiplog share manager"))
    );
    let claim_candidates = report_json["packet_quality"]["claim_candidates"]
        .as_array()
        .expect("packet_quality should expose claim_candidates");
    assert!(
        claim_candidates.iter().any(|candidate| {
            candidate["evidence_strength"].as_str() == Some("manual_only")
                && candidate["supporting_sources"]
                    .as_array()
                    .is_some_and(|sources| {
                        sources
                            .iter()
                            .any(|source| source.as_str() == Some("manual"))
                    })
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
        "manual-only intake should emit a conservative claim candidate with prompts"
    );

    let packet = std::fs::read_to_string(run_dir.join("packet.md")).unwrap();
    assert_packet_opens_with_coverage(&packet);
    insta::assert_snapshot!(
        "manual_only_review_ready_packet_surface",
        review_ready_packet_surface(
            &packet,
            out.to_str().expect("--out path should be utf-8"),
            run_dir
                .file_name()
                .and_then(|name| name.to_str())
                .expect("run directory should have a utf-8 name")
        )
    );
    assert!(
        packet.contains("Ready with caveats.") && packet.contains("manual_only"),
        "manual-only packet should surface readiness caveats in packet.md"
    );
    assert_packet_quality_report_matches_packet(&report_json, &packet);
    assert!(
        packet.contains("# Claim Candidates")
            && packet.contains("Evidence strength: `manual_only`")
            && packet.contains("Missing context:"),
        "manual-only packet should render claim candidates with missing-context prompts"
    );
    assert!(packet.contains("- Manual: 1 event"));
    assert!(packet.contains("Skipped:\n- None recorded\n"));
    assert!(packet.contains("Known gaps:\n- Manual events are user-provided\n"));
    assert!(packet.contains("Manual incident follow-up"));

    let coverage = std::fs::read_to_string(run_dir.join("coverage.manifest.json")).unwrap();
    assert!(coverage.contains("\"Complete\""));
    assert!(coverage.contains("\"manual\""));
}

#[test]
fn intake_journal_suggestions_use_configured_manual_events_path() -> CliTestResult {
    let tmp = TempDir::new()?;
    let out = tmp.path().join("out");
    let config_dir = tmp.path().join("config");
    let manual_dir = config_dir.join("journal");
    std::fs::create_dir_all(&manual_dir)?;
    let config_path = config_dir.join("shiplog.toml");
    let events_path = config_dir.join("json.events.jsonl");
    let coverage_path = config_dir.join("json.coverage.json");
    let manual_events = manual_dir.join("manual_events.yaml");
    let events: Vec<_> = (0..10)
        .map(|idx| {
            fixture_pr_event(
                SourceSystem::Github,
                "acme/platform",
                700 + idx as u64,
                &format!("Platform reliability fix {}", idx + 1),
                2 + idx,
            )
        })
        .collect();
    let coverage = CoverageManifest {
        run_id: RunId("run_configured_manual_next_steps".into()),
        generated_at: fixture_time(20),
        user: "octo".into(),
        window: fixture_window(),
        mode: "fixture".into(),
        sources: vec!["github".into(), "manual".into()],
        slices: vec![CoverageSlice {
            window: fixture_window(),
            query: "github fixture".into(),
            total_count: events.len() as u64,
            fetched: events.len() as u64,
            incomplete_results: Some(false),
            notes: vec!["fixture".into()],
        }],
        warnings: vec![],
        completeness: Completeness::Complete,
    };
    write_events_jsonl(&events_path, &events);
    write_coverage_manifest(&coverage_path, &coverage);
    std::fs::write(
        &manual_events,
        "version: 1\ngenerated_at: 2026-01-01T00:00:00Z\nevents: []\n",
    )?;
    std::fs::write(
        &config_path,
        r#"[defaults]
window = "year:2025"
include_reviews = true

[user]
label = "octo"

[sources.json]
enabled = true
events = "./json.events.jsonl"
coverage = "./json.coverage.json"

[sources.manual]
enabled = true
events = "./journal/manual_events.yaml"
user = "octo"
"#,
    )?;

    let assert = shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args([
            "intake",
            "--out",
            out.to_str().unwrap(),
            "--config",
            config_path.to_str().unwrap(),
            "--year",
            "2025",
            "--no-open",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let manual_events_arg = manual_events.display().to_string();

    assert!(
        stdout.contains("- JSON collected 10 events"),
        "Good section should keep positive evidence contributions visible. stdout:\n{stdout}"
    );
    assert!(
        !stdout.contains("- Manual collected 0 events"),
        "Good section should not describe a zero-event source as positive evidence. stdout:\n{stdout}"
    );
    assert!(stdout.contains("shiplog journal add --date"));
    assert!(stdout.contains("--events"));
    assert!(stdout.contains(&manual_events_arg));

    let run_dir = first_run_dir(&out);
    let packet_before = std::fs::read_to_string(run_dir.join("packet.md"))?;
    let coverage_before = std::fs::read_to_string(run_dir.join("coverage.manifest.json"))?;
    let report_json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        run_dir.join("intake.report.json"),
    )?)?;
    let good = report_json["good"].as_array().unwrap();
    assert!(
        good.iter()
            .any(|item| item.as_str() == Some("JSON collected 10 events")),
        "intake report should keep positive source evidence in good={good:?}"
    );
    assert!(
        good.iter().all(|item| item
            .as_str()
            .is_none_or(|text| !text.contains("Manual collected 0 events"))),
        "intake report good array should not call zero-event manual evidence good={good:?}"
    );
    let next_commands = report_json["next_commands"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|command| command.as_str())
        .collect::<Vec<_>>();
    assert!(
        next_commands
            .first()
            .is_some_and(|command| command.starts_with("shiplog repair plan ")),
        "repairable intake should route the top-level handoff through repair plan first: {next_commands:?}"
    );
    assert!(
        next_commands
            .iter()
            .all(|command| !command.contains("shiplog journal add --date")),
        "top-level next commands should not bypass repair items with direct journal add commands: {next_commands:?}"
    );
    assert!(
        next_commands
            .iter()
            .all(|command| !command.contains("shiplog workstreams split ")),
        "top-level next commands should not advertise write-producing workstream edits while repair plan is the first handoff: {next_commands:?}"
    );
    for (field, commands) in [
        (
            "top_fixups",
            report_json["top_fixups"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|fixup| fixup["command"].as_str())
                .collect::<Vec<_>>(),
        ),
        (
            "evidence_debt",
            report_json["evidence_debt"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|debt| debt["next_step"].as_str())
                .collect::<Vec<_>>(),
        ),
    ] {
        let journal_commands: Vec<_> = commands
            .into_iter()
            .filter(|command| command.contains("shiplog journal add --date"))
            .collect();
        assert!(
            !journal_commands.is_empty(),
            "{field} should include at least one direct journal add suggestion"
        );
        for command in journal_commands {
            assert!(
                command.contains("--events") && command.contains(&manual_events_arg),
                "{field} journal command should target configured manual events file: {command}"
            );
        }
    }

    let review_assert = shiplog_cmd()
        .args(["review", "--out", out.to_str().unwrap(), "--latest"])
        .assert()
        .success();
    let review_stdout = String::from_utf8(review_assert.get_output().stdout.clone())?;
    assert!(review_stdout.contains("shiplog journal add --date"));
    assert!(review_stdout.contains("--events"));
    assert!(review_stdout.contains(&manual_events_arg));

    let fixups_assert = shiplog_cmd()
        .args([
            "review",
            "fixups",
            "--out",
            out.to_str().unwrap(),
            "--latest",
            "--commands-only",
        ])
        .assert()
        .success();
    let fixups_stdout = String::from_utf8(fixups_assert.get_output().stdout.clone())?;
    assert!(fixups_stdout.contains("shiplog journal add --date"));
    assert!(fixups_stdout.contains("--events"));
    assert!(fixups_stdout.contains(&manual_events_arg));

    assert_eq!(
        packet_before,
        std::fs::read_to_string(run_dir.join("packet.md"))?
    );
    assert_eq!(
        coverage_before,
        std::fs::read_to_string(run_dir.join("coverage.manifest.json"))?
    );

    Ok(())
}

#[test]
fn golden_intake_all_source_fixture_surfaces_every_source_without_network() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");
    let events_path = tmp.path().join("all-source.events.jsonl");
    let coverage_path = tmp.path().join("all-source.coverage.json");
    let events = all_source_fixture_events();
    let coverage = all_source_fixture_coverage();
    write_events_jsonl(&events_path, &events);
    write_coverage_manifest(&coverage_path, &coverage);

    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[defaults]
window = "year:2025"
include_reviews = true

[user]
label = "octo"

[sources.json]
enabled = true
events = "./all-source.events.jsonl"
coverage = "./all-source.coverage.json"
"#,
    )
    .unwrap();

    let assert = shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args([
            "intake",
            "--out",
            out.to_str().unwrap(),
            "--config",
            tmp.path().join("shiplog.toml").to_str().unwrap(),
            "--year",
            "2025",
            "--no-open",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert!(stdout.contains("Review intake complete."));
    assert!(stdout.contains("- JSON: success, 8 events"));
    assert!(stdout.contains("Skipped:\n- None"));
    assert!(stdout.contains("Intake readiness:"));
    assert!(stdout.contains("Good:"));
    assert!(stdout.contains("- Packet rendered"));
    assert!(stdout.contains("- Evidence ledger written"));
    assert!(stdout.contains("- Coverage manifest written"));
    assert!(stdout.contains("shiplog share explain manager --out"));
    assert!(!stdout.contains("--bundle-profile manager"));
    assert!(stdout.contains("shiplog open packet --out"));

    let run_dir = first_run_dir(&out);
    assert_intake_artifacts(&run_dir);
    assert_ledger_event_count(&run_dir, 8);
    let (report_md, report_json) = assert_golden_intake_report(&run_dir, "Ready for review");
    assert!(report_md.contains("- JSON: 8 events"));
    assert!(
        report_json["included_sources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|source| source["source_key"] == "json"
                && source["source"] == "json"
                && source["source_label"] == "JSON"
                && source["event_count"] == 8)
    );
    assert_eq!(report_json["skipped_sources"].as_array().unwrap().len(), 0);
    assert!(
        report_json["share_commands"]
            .as_array()
            .unwrap()
            .iter()
            .any(|command| command.as_str().unwrap().contains("shiplog share public")),
        "all-source intake report should keep share commands visible"
    );

    let packet = std::fs::read_to_string(run_dir.join("packet.md")).unwrap();
    assert_packet_opens_with_coverage(&packet);
    assert_packet_uses_summary_appendix(&packet);

    for expected in [
        "- GitHub: 2 events",
        "- GitLab: 1 event",
        "- Jira: 1 event",
        "- Linear: 1 event",
        "- Local git: 1 event",
        "- Manual: 1 event",
        "- JSON import: 1 event",
        "Skipped:\n- None recorded\n",
        "Known gaps:\n- Manual events are user-provided\n",
        "GitHub release automation",
        "GitLab self-hosted deploy fix",
        "Jira OPS-42 rollout checklist",
        "Linear issue triage",
        "Local git hotfix commit",
        "Manual customer debugging note",
        "Imported architecture decision",
        "**Suggested claim prompts**",
    ] {
        assert!(
            packet.contains(expected),
            "golden all-source intake packet should contain {expected:?}"
        );
    }

    let coverage = std::fs::read_to_string(run_dir.join("coverage.manifest.json")).unwrap();
    for source in [
        "\"github\"",
        "\"gitlab\"",
        "\"jira\"",
        "\"linear\"",
        "\"local_git\"",
        "\"manual\"",
        "\"json_import\"",
    ] {
        assert!(
            coverage.contains(source),
            "golden all-source intake coverage should preserve {source}"
        );
    }
}

#[test]
fn intake_summary_and_report_skipped_sources_mirror_skipped_decisions() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    let assert = shiplog_cmd()
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
            "--explain",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert!(stdout.contains("Skipped:\n- GitHub: GITHUB_TOKEN not found"));
    assert!(
        !stdout.contains("Skipped:\n- None\n\nSource decisions:\n- GitHub: skipped"),
        "summary should not say no sources were skipped while source decisions list skipped sources"
    );

    let run_dir = first_run_dir(&out);
    let report_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(run_dir.join("intake.report.json")).unwrap())
            .unwrap();
    let packet = std::fs::read_to_string(run_dir.join("packet.md")).unwrap();
    let coverage: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(run_dir.join("coverage.manifest.json")).unwrap(),
    )
    .unwrap();
    let decisions = report_json["source_decisions"].as_array().unwrap();
    let skipped_sources = report_json["skipped_sources"].as_array().unwrap();
    let skipped_decisions = decisions
        .iter()
        .filter(|decision| decision["decision"].as_str() == Some("skipped"))
        .collect::<Vec<_>>();

    assert_eq!(
        skipped_sources.len(),
        skipped_decisions.len(),
        "skipped_sources should mirror skipped source decisions"
    );
    for decision in skipped_decisions {
        let source_key = decision["source_key"].as_str().unwrap();
        let reason = decision["reason"].as_str().unwrap();
        assert!(
            skipped_sources.iter().any(|source| {
                source["source_key"].as_str() == Some(source_key)
                    && source["reason"].as_str() == Some(reason)
            }),
            "skipped decision {source_key:?} should be present in skipped_sources"
        );
    }
    assert!(
        packet.contains("Skipped:\n- GitHub: GITHUB_TOKEN not found"),
        "packet coverage summary should mirror skipped source decisions. packet:\n{packet}"
    );
    assert!(
        !packet.contains("Skipped:\n- None recorded\n\nKnown gaps:"),
        "packet coverage summary should not claim no skipped sources when the report records skipped decisions"
    );
    assert_eq!(
        coverage["completeness"], "Partial",
        "coverage manifest should mark autodetected skipped sources as partial"
    );
    let warnings = coverage["warnings"].as_array().unwrap();
    assert!(
        warnings.iter().any(|warning| {
            warning
                .as_str()
                .is_some_and(|warning| warning.contains("Configured source github was skipped"))
        }),
        "coverage warnings should receipt autodetected skipped sources"
    );
}

#[test]
fn intake_created_git_config_outside_repo_points_to_current_repo() {
    let Some(repo) = create_local_git_repo() else {
        eprintln!(
            "skipping intake_created_git_config_outside_repo_points_to_current_repo: git not available"
        );
        return;
    };
    let scratch = repo.path().join("scratch");
    std::fs::create_dir_all(&scratch).unwrap();
    let config = scratch.join("shiplog.toml");
    let out = scratch.join("out");

    let assert = shiplog_cmd()
        .current_dir(repo.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args([
            "intake",
            "--config",
            config.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
            "--source",
            "git",
            "--source",
            "manual",
            "--no-open",
            "--explain",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert!(stdout.contains("Collected:\n- Local git: success"));
    assert!(stdout.contains("- Local git: included, repo"));
    assert!(
        !stdout.contains("- Local git: skipped"),
        "intake-created config should point at the current git repository when --config is outside cwd. stdout:\n{stdout}"
    );

    let normalized_repo_path = repo.path().display().to_string().replace('\\', "/");
    let config_text = std::fs::read_to_string(&config).unwrap();
    assert!(
        config_text.contains(&format!("repo = \"{normalized_repo_path}\"")),
        "intake-created git config should preserve the current repo path. config:\n{config_text}"
    );

    let run_dir = first_run_dir(&out);
    let report_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(run_dir.join("intake.report.json")).unwrap())
            .unwrap();
    assert!(
        report_json["included_sources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|source| source["source_key"] == "git"),
        "intake-created config should collect the current git repository"
    );
}

#[test]
fn guided_git_repo_without_origin_collects_from_dot_repo() {
    let Some(repo) = create_local_git_repo() else {
        eprintln!(
            "skipping guided_git_repo_without_origin_collects_from_dot_repo: git not available"
        );
        return;
    };
    let out = repo.path().join("out");
    std::fs::write(
        repo.path().join("manual_events.yaml"),
        "version: 1\ngenerated_at: 2026-01-01T00:00:00Z\nevents: []\n",
    )
    .unwrap();
    std::fs::write(
        repo.path().join("shiplog.toml"),
        r#"[shiplog]
config_version = 1

[defaults]
window = "last-6-months"
profile = "internal"

[sources.git]
enabled = true
repo = "."
include_merges = false

[sources.manual]
enabled = true
events = "./manual_events.yaml"
"#,
    )
    .unwrap();

    let assert = shiplog_cmd()
        .current_dir(repo.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args([
            "intake",
            "--out",
            out.to_str().unwrap(),
            "--year",
            "2025",
            "--no-open",
            "--explain",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert!(
        stdout.contains("Collected:\n- Local git: success"),
        "repo = \".\" should collect local git evidence even when no origin remote exists. stdout:\n{stdout}"
    );
    assert!(
        !stdout.contains("Could not determine repository name"),
        "local git source should derive repo identity from the workdir before failing setup. stdout:\n{stdout}"
    );

    let run_dir = first_run_dir(&out);
    let report_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(run_dir.join("intake.report.json")).unwrap())
            .unwrap();
    assert!(
        report_json["included_sources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|source| source["source_key"] == "git" && source["event_count"] == 1),
        "report should include the local git event collected from repo = \".\""
    );
}

#[test]
fn intake_source_decisions_skip_configured_git_directory_that_is_not_repo() {
    let tmp = TempDir::new().unwrap();
    let not_repo = tmp.path().join("not-a-repo");
    let config = tmp.path().join("shiplog.toml");
    let out = tmp.path().join("out");
    std::fs::create_dir_all(&not_repo).unwrap();
    std::fs::write(
        &config,
        r#"[shiplog]
config_version = 1

[defaults]
window = "last-6-months"
profile = "internal"

[sources.git]
enabled = true
repo = "./not-a-repo"
include_merges = false

[sources.manual]
enabled = true
events = "./manual_events.yaml"
user = "Tester"
"#,
    )
    .unwrap();

    let assert = shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args([
            "intake",
            "--config",
            config.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
            "--source",
            "git",
            "--source",
            "manual",
            "--no-open",
            "--explain",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert!(stdout.contains("- Local git: skipped, repo"));
    assert!(stdout.contains("is not a git repo"));
    assert!(
        !stdout.contains("collect configured git source"),
        "non-repository git paths should be diagnosed before runtime collection. stdout:\n{stdout}"
    );

    let run_dir = first_run_dir(&out);
    let report_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(run_dir.join("intake.report.json")).unwrap())
            .unwrap();
    assert!(
        report_json["skipped_sources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|source| source["source_key"] == "git"
                && source["reason"]
                    .as_str()
                    .is_some_and(|reason| reason.contains("is not a git repo"))),
        "skipped_sources should include the preflight Local git repair reason"
    );

    let source_failures: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(run_dir.join("source.failures.json")).unwrap(),
    )
    .unwrap();
    assert!(
        source_failures["failures"]
            .as_array()
            .unwrap()
            .iter()
            .any(|failure| failure["source"] == "git"
                && failure["kind"] == "local_source_unavailable"),
        "source failure receipt should classify an existing non-repo directory as local_source_unavailable"
    );
}

#[test]
fn golden_intake_manager_share_missing_key_fails_closed() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["intake", "--out", out.to_str().unwrap(), "--no-open"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Review intake complete."))
        .stdout(predicate::str::contains("Packet readiness: Needs evidence"));

    let run_dir = first_run_dir(&out);
    assert_intake_artifacts(&run_dir);
    let (report_md, report_json) = assert_golden_intake_report(&run_dir, "Needs evidence");
    assert!(report_md.contains("Packet readiness: **Needs evidence**"));
    assert!(
        report_json["share_commands"]
            .as_array()
            .unwrap()
            .iter()
            .any(|command| command.as_str().unwrap().contains("shiplog share manager")),
        "intake report should keep manager share recovery command visible"
    );

    shiplog_cmd()
        .env_remove("SHIPLOG_REDACT_KEY")
        .args([
            "share",
            "manager",
            "--out",
            out.to_str().unwrap(),
            "--latest",
        ])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("manager share requires --redact-key or SHIPLOG_REDACT_KEY")
                .and(predicate::str::contains("shiplog share manager --latest")),
        );

    assert!(
        !run_dir.join("profiles/manager/packet.md").exists(),
        "manager share packet should not be written after intake without a key"
    );

    shiplog_cmd()
        .env_remove("SHIPLOG_REDACT_KEY")
        .args([
            "share",
            "verify",
            "public",
            "--out",
            out.to_str().unwrap(),
            "--latest",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "public share requires --redact-key or SHIPLOG_REDACT_KEY",
        ));
    assert!(
        !run_dir.join("profiles/public/packet.md").exists(),
        "public share verify should not write a packet without a key"
    );
}

#[test]
fn intake_creates_minimal_config_and_manual_rescue_packet() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["intake", "--out", out.to_str().unwrap(), "--no-open"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Review intake complete."))
        .stdout(predicate::str::contains("Config: created"))
        .stdout(predicate::str::contains("- Manual: success, 0 events"))
        .stdout(predicate::str::contains("Packet:"))
        .stdout(predicate::str::contains("Intake readiness:"))
        .stdout(predicate::str::contains("Packet readiness: Needs evidence"))
        .stdout(predicate::str::contains(
            "No events collected; add manual evidence or enable a source.",
        ))
        .stdout(predicate::str::contains("Open later:"));

    assert!(
        tmp.path().join("shiplog.toml").exists(),
        "intake should create shiplog.toml"
    );
    assert!(
        tmp.path().join("manual_events.yaml").exists(),
        "intake should create manual_events.yaml"
    );
    let run_dir = first_run_dir(&out);
    assert!(run_dir.join("packet.md").exists(), "missing rescue packet");
    assert!(
        run_dir.join("coverage.manifest.json").exists(),
        "missing rescue coverage"
    );
    let (report_md, report_json) = assert_golden_intake_report(&run_dir, "Needs evidence");
    assert!(report_md.contains("No events collected"));
    assert!(
        report_json["needs_attention"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str().unwrap().contains("No events collected")),
        "empty rescue report should explain the evidence gap"
    );
    let repair_items = report_json["repair_items"].as_array().unwrap();
    assert!(
        repair_items
            .iter()
            .any(|item| item["kind"] == "manual_evidence_missing"
                && item["source_key"] == "manual"
                && item["action"]["kind"] == "journal_add"
                && item["receipt_refs"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|receipt| receipt["field"] == "needs_attention")),
        "empty rescue report should expose a receipt-derived manual evidence repair item"
    );
}

#[test]
fn intake_explain_reports_source_decisions_for_rescue_config() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    shiplog_cmd()
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
            "--explain",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Source decisions:"))
        .stdout(predicate::str::contains(
            "- GitHub: skipped, GITHUB_TOKEN not found",
        ))
        .stdout(predicate::str::contains("Fix:"))
        .stdout(predicate::str::contains("export GITHUB_TOKEN=..."))
        .stdout(predicate::str::contains(
            "- Local git: skipped, current directory is not a git repo",
        ))
        .stdout(predicate::str::contains(
            "Run intake from a git repository root.",
        ))
        .stdout(predicate::str::contains(
            "- Manual: included, manual_events.yaml found",
        ))
        .stdout(predicate::str::contains(
            "Use `shiplog journal add` to capture missing context without editing YAML.",
        ));
}

#[test]
fn intake_records_configured_missing_tokens_as_skipped_sources() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");
    let fixtures = fixture_dir();
    std::fs::copy(
        fixtures.join("ledger.events.jsonl"),
        tmp.path().join("ledger.events.jsonl"),
    )
    .unwrap();
    std::fs::copy(
        fixtures.join("coverage.manifest.json"),
        tmp.path().join("coverage.manifest.json"),
    )
    .unwrap();

    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[defaults]
window = "year:2025"

[sources.json]
enabled = true
events = "./ledger.events.jsonl"
coverage = "./coverage.manifest.json"

[sources.jira]
enabled = true
user = "712020:account-id"
instance = "example.atlassian.net"
status = "done"

[sources.linear]
enabled = true
user_id = "linear-user-id"
status = "done"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args([
            "intake",
            "--out",
            out.to_str().unwrap(),
            "--config",
            tmp.path().join("shiplog.toml").to_str().unwrap(),
            "--no-open",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("- JSON: success"))
        .stdout(predicate::str::contains("- Jira: missing JIRA_TOKEN"))
        .stdout(predicate::str::contains("- Linear: missing LINEAR_API_KEY"))
        .stdout(predicate::str::contains("Skipped sources:"))
        .stdout(predicate::str::contains("Intake readiness:"))
        .stdout(predicate::str::contains("Intake status: Needs curation"))
        .stdout(predicate::str::contains(
            "Packet readiness: Ready with caveats",
        ))
        .stdout(predicate::str::contains(
            "- Jira skipped: missing JIRA_TOKEN",
        ))
        .stdout(predicate::str::contains(
            "- Linear skipped: missing LINEAR_API_KEY",
        ))
        .stdout(predicate::str::contains("Repair sources:"))
        .stdout(predicate::str::contains("kind: missing_token"))
        .stdout(predicate::str::contains("shiplog sources status"))
        .stdout(predicate::str::contains("--source jira"))
        .stdout(predicate::str::contains("--source linear"))
        .stdout(predicate::str::contains("shiplog doctor --setup --config"));

    let run_dir = first_run_dir(&out);
    let coverage = std::fs::read_to_string(run_dir.join("coverage.manifest.json")).unwrap();
    let (report_md, report_json) = assert_golden_intake_report(&run_dir, "Needs curation");
    let source_failures_path = run_dir.join("source.failures.json");
    assert!(
        source_failures_path.exists(),
        "intake should write source failure receipts for skipped configured sources"
    );
    let source_failures: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&source_failures_path).unwrap()).unwrap();
    assert_eq!(source_failures["schema_version"], 1);
    assert_eq!(
        source_failures["run_id"].as_str().unwrap(),
        run_dir.file_name().unwrap().to_string_lossy().as_ref()
    );
    assert_eq!(source_failures["failures"].as_array().unwrap().len(), 2);
    assert!(
        source_failures["failures"]
            .as_array()
            .unwrap()
            .iter()
            .any(|failure| failure["source"] == "jira"
                && failure["kind"] == "missing_token"
                && failure["rerun_command"]
                    .as_str()
                    .unwrap()
                    .contains("shiplog intake --config"))
    );
    assert!(
        report_json["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|artifact| artifact["path"]
                .as_str()
                .unwrap()
                .ends_with("source.failures.json")),
        "intake report should expose the source failure receipt artifact"
    );
    assert!(report_md.contains("## Skipped Sources"));
    assert!(report_md.contains("## Repair Sources"));
    assert!(report_md.contains("- Jira: missing JIRA_TOKEN"));
    assert!(report_md.contains("- Linear: missing LINEAR_API_KEY"));
    assert!(report_md.contains("kind: `missing_token`"));
    assert!(report_md.contains("export JIRA_TOKEN=..."));
    assert!(report_md.contains("shiplog identify jira --auth-user <email>"));
    assert!(report_md.contains("export LINEAR_API_KEY=..."));
    assert!(report_md.contains("shiplog identify linear"));
    assert!(report_md.contains("shiplog sources status"));
    assert!(report_md.contains("--source jira"));
    assert!(report_md.contains("--source linear"));

    assert_eq!(report_json["skipped_sources"].as_array().unwrap().len(), 2);
    assert_eq!(report_json["repair_sources"].as_array().unwrap().len(), 2);
    assert!(
        report_json["skipped_sources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|source| source["source"] == "jira"
                && source["source_key"] == "jira"
                && source["source_label"] == "Jira"
                && source["reason"] == "missing JIRA_TOKEN")
    );
    assert!(
        report_json["next_commands"]
            .as_array()
            .unwrap()
            .iter()
            .any(|command| command.as_str().unwrap().contains("shiplog doctor")),
        "skipped-source intake report should keep repair/rerun next commands"
    );
    assert!(
        report_json["source_decisions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|decision| decision["source"] == "jira"
                && decision["source_key"] == "jira"
                && decision["source_label"] == "Jira"
                && decision["hint_lines"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|line| line.as_str().unwrap().contains("JIRA_TOKEN")))
    );
    assert!(
        report_json["repair_sources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|repair| repair["source"] == "jira"
                && repair["source_key"] == "jira"
                && repair["source_label"] == "Jira"
                && repair["kind"] == "missing_token"
                && repair["commands"].as_array().unwrap().iter().any(|line| {
                    let Some(line) = line.as_str() else {
                        return false;
                    };
                    line.contains("shiplog sources status") && line.contains("--source jira")
                }))
    );
    assert!(
        report_json["repair_sources"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|repair| repair["source_key"] == "jira" || repair["source_key"] == "linear")
            .flat_map(|repair| repair["commands"].as_array().into_iter().flatten())
            .filter_map(|command| command.as_str())
            .all(|command| !command.contains("export ") && !command.contains("shiplog identify")),
        "setup-blocked repair sources should hand off to read-only setup status instead of token or identity commands: {:?}",
        report_json["repair_sources"]
    );
    let repair_items = report_json["repair_items"].as_array().unwrap();
    assert!(
        repair_items
            .iter()
            .any(|item| item["kind"] == "source_skipped_configuration"
                && item["source_key"] == "jira"
                && item["action"]["kind"] == "configure_source"
                && item["action"]["command"]
                    .as_str()
                    .is_some_and(|command| command.contains("shiplog sources status")
                        && command.contains("--source jira"))
                && item["receipt_refs"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|receipt| receipt["field"] == "repair_sources"
                        && receipt["source_key"] == "jira")),
        "skipped Jira source should produce a receipt-derived repair item"
    );
    assert!(
        repair_items
            .iter()
            .any(|item| item["kind"] == "artifact_missing_or_unopened"
                && item["action"]["kind"] == "open_artifact"
                && item["receipt_refs"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|receipt| receipt["field"] == "artifacts")),
        "source failure artifact should produce an artifact repair item"
    );
    for (source_key, source_label) in [("jira", "Jira"), ("linear", "Linear")] {
        assert!(
            report_json["source_decisions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|decision| decision["source_key"] == source_key
                    && decision["source_label"] == source_label),
            "source_decisions should expose canonical identity for {source_key}"
        );
        assert!(
            report_json["source_freshness"]
                .as_array()
                .unwrap()
                .iter()
                .any(|freshness| freshness["source_key"] == source_key
                    && freshness["source_label"] == source_label
                    && freshness["status"] == "unavailable"),
            "source_freshness should join to source_decisions on source_key for {source_key}"
        );
    }

    let repair_plan = shiplog_cmd()
        .current_dir(tmp.path())
        .args(["repair", "plan", "--out", out.to_str().unwrap(), "--latest"])
        .assert()
        .success();
    let repair_plan_stdout = String::from_utf8(repair_plan.get_output().stdout.clone()).unwrap();
    assert!(
        repair_plan_stdout.contains("shiplog sources status")
            && repair_plan_stdout.contains("--source jira")
            && repair_plan_stdout.contains("--source linear"),
        "repair plan should route provider setup gaps through read-only source status. stdout:\n{repair_plan_stdout}"
    );
    assert!(
        !repair_plan_stdout.contains("shiplog identify")
            && !repair_plan_stdout.contains("export JIRA_TOKEN")
            && !repair_plan_stdout.contains("export LINEAR_API_KEY"),
        "repair plan should not print setup-blocked provider token/identity commands. stdout:\n{repair_plan_stdout}"
    );

    assert!(
        coverage.contains("Configured source jira was skipped: missing JIRA_TOKEN"),
        "intake coverage should record skipped Jira source"
    );
    assert!(
        coverage.contains("Configured source linear was skipped: missing LINEAR_API_KEY"),
        "intake coverage should record skipped Linear source"
    );
    assert!(
        coverage.contains("\"Partial\""),
        "intake coverage should mark partial source collection"
    );
}

#[test]
fn intake_records_remote_filter_failures_without_network_and_keeps_repair_guidance() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");
    let fixtures = fixture_dir();
    let cache_dir = tmp.path().join(".cache");
    let github_cache_path = seed_github_cache(&cache_dir);
    std::fs::copy(
        fixtures.join("ledger.events.jsonl"),
        tmp.path().join("ledger.events.jsonl"),
    )
    .unwrap();
    std::fs::copy(
        fixtures.join("coverage.manifest.json"),
        tmp.path().join("coverage.manifest.json"),
    )
    .unwrap();

    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[defaults]
window = "year:2025"

[sources.json]
enabled = true
events = "./ledger.events.jsonl"
coverage = "./coverage.manifest.json"

[sources.gitlab]
enabled = true
user = "octo"
instance = "https://gitlab.self-hosted.example/platform/reliability"
state = "needs_review"
cache_dir = "./.cache"

[sources.jira]
enabled = true
user = "712020:account-id"
instance = "example.atlassian.net"
status = "blocked"
cache_dir = "./.cache"

[sources.linear]
enabled = true
user_id = "linear-user-id"
status = "waiting"
project = "OPS"
cache_dir = "./.cache"
"#,
    )
    .unwrap();

    let gitlab_token = "edge-gitlab-token-do-not-print";
    let jira_token = "edge-jira-token-do-not-print";
    let linear_key = "edge-linear-key-do-not-print";
    let assert = shiplog_cmd()
        .current_dir(tmp.path())
        .env("GITLAB_TOKEN", gitlab_token)
        .env("JIRA_TOKEN", jira_token)
        .env("LINEAR_API_KEY", linear_key)
        .env_remove("GITHUB_TOKEN")
        .args([
            "intake",
            "--out",
            out.to_str().unwrap(),
            "--config",
            tmp.path().join("shiplog.toml").to_str().unwrap(),
            "--year",
            "2025",
            "--no-open",
            "--explain",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert!(stdout.contains("Review intake complete."));
    assert!(stdout.contains("- JSON: success"));
    assert!(stdout.contains("- GitLab: create configured GitLab ingestor"));
    assert!(stdout.contains("Invalid MR state: needs_review"));
    assert!(stdout.contains("- Jira: create configured Jira ingestor"));
    assert!(stdout.contains("Invalid issue status: blocked"));
    assert!(stdout.contains("- Linear: create configured Linear ingestor"));
    assert!(stdout.contains("Invalid issue status: waiting"));
    assert!(stdout.contains("Source decisions:"));
    assert!(stdout.contains("- GitLab: skipped, create configured GitLab ingestor"));
    assert!(stdout.contains("- Jira: skipped, create configured Jira ingestor"));
    assert!(stdout.contains("- Linear: skipped, create configured Linear ingestor"));
    assert!(
        !stdout.contains("- GitLab: included"),
        "runtime GitLab filter failure should override stale preflight inclusion"
    );
    assert!(
        !stdout.contains("- Jira: included"),
        "runtime Jira filter failure should override stale preflight inclusion"
    );
    assert!(
        !stdout.contains("- Linear: included"),
        "runtime Linear filter failure should override stale preflight inclusion"
    );
    assert!(stdout.contains("Repair sources:"));
    assert!(stdout.contains("kind: invalid_filter"));
    assert!(stdout.contains("Set sources.gitlab.state to opened, merged, closed, or all."));
    assert!(stdout.contains("Set sources.jira.status to open, in_progress, done, closed, or all."));
    assert!(stdout.contains(
        "Set sources.linear.status to backlog, todo, in_progress, done, cancelled, or all."
    ));
    assert!(stdout.contains("Retry with `shiplog intake --last-6-months --explain`."));

    for secret in [gitlab_token, jira_token, linear_key] {
        assert!(
            !stdout.contains(secret),
            "intake stdout should not print secret sentinel {secret:?}"
        );
    }

    let run_dir = first_run_dir(&out);
    assert!(
        github_cache_path.exists(),
        "intake should not delete existing source cache files"
    );
    let coverage = std::fs::read_to_string(run_dir.join("coverage.manifest.json")).unwrap();
    assert!(coverage.contains("\"Partial\""));
    assert!(coverage.contains("Configured source gitlab was skipped"));
    assert!(coverage.contains("Configured source jira was skipped"));
    assert!(coverage.contains("Configured source linear was skipped"));
    assert!(coverage.contains("Invalid MR state: needs_review"));
    assert!(coverage.contains("Invalid issue status: blocked"));
    assert!(coverage.contains("Invalid issue status: waiting"));

    let (report_md, report_json) = assert_golden_intake_report(&run_dir, "Needs curation");
    for secret in [gitlab_token, jira_token, linear_key] {
        assert!(
            !report_md.contains(secret) && !report_json.to_string().contains(secret),
            "intake report should not print secret sentinel {secret:?}"
        );
    }
    assert_eq!(report_json["skipped_sources"].as_array().unwrap().len(), 3);
    let source_decisions = report_json["source_decisions"].as_array().unwrap();
    for source_key in ["gitlab", "jira", "linear"] {
        assert!(
            source_decisions.iter().any(|decision| {
                decision["source_key"] == source_key
                    && decision["decision"] == "skipped"
                    && decision["reason"]
                        .as_str()
                        .is_some_and(|reason| reason.starts_with("create configured"))
            }),
            "source_decisions should report {source_key} runtime filter failure as skipped"
        );
        assert!(
            !source_decisions.iter().any(|decision| {
                decision["source_key"] == source_key && decision["decision"] == "included"
            }),
            "source_decisions should not keep stale {source_key} inclusion after runtime failure"
        );
    }
    let repairs = report_json["repair_sources"].as_array().unwrap();
    assert!(repairs.iter().any(|repair| {
        repair["source"] == "gitlab"
            && repair["source_key"] == "gitlab"
            && repair["source_label"] == "GitLab"
            && repair["kind"] == "invalid_filter"
            && repair["commands"]
                .as_array()
                .unwrap()
                .iter()
                .any(|command| {
                    command.as_str().is_some_and(|command| {
                        command.contains("shiplog sources status")
                            && command.contains("--source gitlab")
                    })
                })
    }));
    assert!(repairs.iter().any(|repair| {
        repair["source"] == "jira"
            && repair["source_key"] == "jira"
            && repair["source_label"] == "Jira"
            && repair["kind"] == "invalid_filter"
            && repair["commands"]
                .as_array()
                .unwrap()
                .iter()
                .any(|command| {
                    command.as_str().is_some_and(|command| {
                        command.contains("shiplog sources status")
                            && command.contains("--source jira")
                    })
                })
    }));
    assert!(repairs.iter().any(|repair| {
        repair["source"] == "linear"
            && repair["source_key"] == "linear"
            && repair["source_label"] == "Linear"
            && repair["kind"] == "invalid_filter"
            && repair["commands"]
                .as_array()
                .unwrap()
                .iter()
                .any(|command| {
                    command.as_str().is_some_and(|command| {
                        command.contains("shiplog sources status")
                            && command.contains("--source linear")
                    })
                })
    }));
    assert!(
        report_json["next_commands"]
            .as_array()
            .unwrap()
            .iter()
            .any(|command| command.as_str().unwrap().contains("shiplog doctor")),
        "intake report should keep rerun/repair guidance visible"
    );

    shiplog_cmd()
        .args([
            "cache",
            "stats",
            "--cache-dir",
            cache_dir.to_str().unwrap(),
            "--source",
            "github",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("github:"))
        .stdout(predicate::str::contains(
            "entries: total 2, valid 1, expired 1",
        ));
}

#[test]
fn intake_rerun_reuses_prior_curation_without_overwriting_manual_events() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");
    let manual_events = tmp.path().join("manual_events.yaml");
    let config_path = tmp.path().join("shiplog.toml");
    write_manual_events(&manual_events);

    std::fs::write(
        &config_path,
        r#"[defaults]
window = "year:2025"

[user]
label = "octo"

[sources.manual]
enabled = true
events = "./manual_events.yaml"
user = "octo"
"#,
    )
    .unwrap();

    let out_arg = out.to_str().unwrap().to_string();
    let config_arg = config_path.to_str().unwrap().to_string();
    let intake_args = [
        "intake",
        "--out",
        out_arg.as_str(),
        "--config",
        config_arg.as_str(),
        "--year",
        "2025",
        "--no-open",
    ];

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(intake_args)
        .assert()
        .success();

    let first_run = first_run_dir(&out);
    let suggested_text =
        std::fs::read_to_string(first_run.join("workstreams.suggested.yaml")).unwrap();
    let mut curated: WorkstreamsFile = serde_yaml::from_str(&suggested_text).unwrap();
    curated.workstreams[0].title = "Curated Reliability".to_string();
    std::fs::write(
        first_run.join("workstreams.yaml"),
        serde_yaml::to_string(&curated).unwrap(),
    )
    .unwrap();
    let first_curated_before = std::fs::read_to_string(first_run.join("workstreams.yaml")).unwrap();
    let manual_before = std::fs::read_to_string(&manual_events).unwrap();

    let assert = shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(intake_args)
        .assert()
        .success()
        .stdout(predicate::str::contains("Curation:"))
        .stdout(predicate::str::contains("Prior workstream curation reused"));
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("workstreams.yaml"));

    let runs = all_run_dirs(&out);
    assert_eq!(runs.len(), 2, "intake rerun should create a new run");
    let second_run = runs
        .into_iter()
        .find(|run| run != &first_run)
        .expect("expected a second intake run");

    assert_eq!(
        std::fs::read_to_string(first_run.join("workstreams.yaml")).unwrap(),
        first_curated_before,
        "intake rerun should not overwrite prior curated workstreams"
    );
    assert_eq!(
        std::fs::read_to_string(second_run.join("workstreams.yaml")).unwrap(),
        first_curated_before,
        "intake rerun should copy prior curated workstreams into the new run"
    );
    assert_eq!(
        std::fs::read_to_string(&manual_events).unwrap(),
        manual_before,
        "intake rerun should not rewrite manual_events.yaml"
    );

    let (report_md, report_json) = assert_golden_intake_report(&second_run, "Ready for review");
    assert!(report_md.contains("## Curation Notes"));
    assert!(report_md.contains("Prior workstream curation reused"));

    assert!(
        report_json["curation_notes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|note| note
                .as_str()
                .unwrap()
                .contains("Prior workstream curation reused"))
    );
}

#[test]
fn intake_explain_reports_configured_source_decisions() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");
    let fixtures = fixture_dir();
    std::fs::copy(
        fixtures.join("ledger.events.jsonl"),
        tmp.path().join("ledger.events.jsonl"),
    )
    .unwrap();
    std::fs::copy(
        fixtures.join("coverage.manifest.json"),
        tmp.path().join("coverage.manifest.json"),
    )
    .unwrap();

    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[defaults]
window = "year:2025"

[sources.json]
enabled = true
events = "./ledger.events.jsonl"
coverage = "./coverage.manifest.json"

[sources.jira]
enabled = true
user = "712020:account-id"
instance = "example.atlassian.net"
status = "done"

[sources.linear]
enabled = true
user_id = "linear-user-id"
status = "done"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args([
            "intake",
            "--out",
            out.to_str().unwrap(),
            "--config",
            tmp.path().join("shiplog.toml").to_str().unwrap(),
            "--no-open",
            "--explain",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Source decisions:"))
        .stdout(predicate::str::contains(
            "- JSON: included, events and coverage files found",
        ))
        .stdout(predicate::str::contains(
            "- Jira: skipped, missing JIRA_TOKEN",
        ))
        .stdout(predicate::str::contains("export JIRA_TOKEN=..."))
        .stdout(predicate::str::contains(
            "shiplog identify jira --auth-user <email>",
        ))
        .stdout(predicate::str::contains(
            "- Linear: skipped, missing LINEAR_API_KEY",
        ))
        .stdout(predicate::str::contains("export LINEAR_API_KEY=..."))
        .stdout(predicate::str::contains("shiplog identify linear"))
        .stdout(predicate::str::contains(
            "Retry with `shiplog intake --last-6-months --explain`.",
        ));
}

#[test]
fn collect_multi_without_enabled_sources_fails_actionably() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[sources.json]
enabled = false
"#,
    )
    .unwrap();

    shiplog_cmd()
        .args([
            "collect",
            "--out",
            tmp.path().join("out").to_str().unwrap(),
            "multi",
            "--config",
            tmp.path().join("shiplog.toml").to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No enabled sources found"));
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
        .stderr(
            predicate::str::contains("public profile requires --redact-key or SHIPLOG_REDACT_KEY")
                .and(predicate::str::contains("Try:"))
                .and(predicate::str::contains(redaction_key_env_command(
                    "SHIPLOG_REDACT_KEY",
                )))
                .and(predicate::str::contains(
                    "rerun this command with --bundle-profile public",
                )),
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
fn runs_list_shows_run_summaries() {
    let tmp = TempDir::new().unwrap();
    collect_json_into(tmp.path());

    shiplog_cmd()
        .args(["runs", "list", "--out", tmp.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Runs:"))
        .stdout(predicate::str::contains("run_fixture"))
        .stdout(predicate::str::contains("sources: github"))
        .stdout(predicate::str::contains("events: 3"))
        .stdout(predicate::str::contains("coverage: Complete"))
        .stdout(predicate::str::contains("packet:"));
}

#[test]
fn runs_show_latest_shows_run_details() {
    let tmp = TempDir::new().unwrap();
    collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "runs",
            "show",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "latest",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Run: run_fixture"))
        .stdout(predicate::str::contains("User: octo"))
        .stdout(predicate::str::contains("Window: 2025-01-01..2025-04-01"))
        .stdout(predicate::str::contains("Sources: github"))
        .stdout(predicate::str::contains("Events: 3"))
        .stdout(predicate::str::contains("Gaps: 0"))
        .stdout(predicate::str::contains("Warnings: none"));
}

#[test]
fn runs_compare_summarizes_cross_run_changes_without_writing_artifacts() {
    let tmp = TempDir::new().unwrap();
    let from_run = collect_json_into(tmp.path());
    let from_packet_before = std::fs::read_to_string(from_run.join("packet.md")).unwrap();
    let from_coverage_before =
        std::fs::read_to_string(from_run.join("coverage.manifest.json")).unwrap();

    let events_path = tmp.path().join("all-source.events.jsonl");
    let coverage_path = tmp.path().join("all-source.coverage.json");
    let events = all_source_fixture_events();
    let coverage = all_source_fixture_coverage();
    write_events_jsonl(&events_path, &events);
    write_coverage_manifest(&coverage_path, &coverage);

    shiplog_cmd()
        .args([
            "collect",
            "--out",
            tmp.path().to_str().unwrap(),
            "json",
            "--events",
            events_path.to_str().unwrap(),
            "--coverage",
            coverage_path.to_str().unwrap(),
            "--user",
            "octo",
            "--window-label",
            "all-source fixture",
        ])
        .assert()
        .success();

    let to_run = tmp.path().join("run_all_sources");
    let to_packet_before = std::fs::read_to_string(to_run.join("packet.md")).unwrap();
    let to_coverage_before =
        std::fs::read_to_string(to_run.join("coverage.manifest.json")).unwrap();

    let assert = shiplog_cmd()
        .args([
            "runs",
            "compare",
            "--out",
            tmp.path().to_str().unwrap(),
            "--from",
            "run_fixture",
            "--to",
            "run_all_sources",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let out_hint = format!("--out \"{}\"", tmp.path().display());

    assert!(stdout.contains("Compare: run_fixture -> run_all_sources"));
    assert!(stdout.contains("Events:"));
    assert!(stdout.contains("- from: 3"));
    assert!(stdout.contains("- to: 8"));
    assert!(stdout.contains("- delta: +5"));
    assert!(stdout.contains("Sources:"));
    assert!(stdout.contains("- Added: Local git, GitLab, Jira, JSON, Linear, Manual"));
    assert!(stdout.contains("- Continued: GitHub"));
    assert!(stdout.contains("Workstreams:"));
    assert!(stdout.contains("- Added:"));
    assert!(stdout.contains("acme/release-tools"));
    assert!(stdout.contains("- Removed:"));
    assert!(stdout.contains("acme/platform"));
    assert!(stdout.contains("Coverage:"));
    assert!(stdout.contains("- from: Complete, gaps: 0"));
    assert!(stdout.contains("- to: Complete, gaps: 0"));
    assert!(stdout.contains(&format!("shiplog review {out_hint}")));
    assert!(stdout.contains("--run run_all_sources"));

    assert_eq!(
        from_packet_before,
        std::fs::read_to_string(from_run.join("packet.md")).unwrap()
    );
    assert_eq!(
        from_coverage_before,
        std::fs::read_to_string(from_run.join("coverage.manifest.json")).unwrap()
    );
    assert_eq!(
        to_packet_before,
        std::fs::read_to_string(to_run.join("packet.md")).unwrap()
    );
    assert_eq!(
        to_coverage_before,
        std::fs::read_to_string(to_run.join("coverage.manifest.json")).unwrap()
    );
}

#[test]
fn runs_compare_resolves_named_period_selectors() {
    let tmp = TempDir::new().unwrap();
    collect_json_into(tmp.path());

    let events_path = tmp.path().join("period-to.events.jsonl");
    let coverage_path = tmp.path().join("period-to.coverage.json");
    let events = all_source_fixture_events();
    let mut coverage = all_source_fixture_coverage();
    coverage.run_id = RunId("run_period_to".into());
    coverage.window = TimeWindow {
        since: NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
        until: NaiveDate::from_ymd_opt(2025, 7, 1).unwrap(),
    };
    for slice in &mut coverage.slices {
        slice.window = coverage.window.clone();
    }
    write_events_jsonl(&events_path, &events);
    write_coverage_manifest(&coverage_path, &coverage);

    shiplog_cmd()
        .args([
            "collect",
            "--out",
            tmp.path().to_str().unwrap(),
            "json",
            "--events",
            events_path.to_str().unwrap(),
            "--coverage",
            coverage_path.to_str().unwrap(),
            "--user",
            "octo",
            "--window-label",
            "period to fixture",
        ])
        .assert()
        .success();

    let config_path = tmp.path().join("shiplog.toml");
    std::fs::write(
        &config_path,
        r#"[periods."2025-Q1"]
since = "2025-01-01"
until = "2025-04-01"

[periods."2025-Q2"]
since = "2025-04-01"
until = "2025-07-01"
"#,
    )
    .unwrap();

    let assert = shiplog_cmd()
        .args([
            "runs",
            "compare",
            "--out",
            tmp.path().to_str().unwrap(),
            "--config",
            config_path.to_str().unwrap(),
            "--from-period",
            "2025-Q1",
            "--to-period",
            "2025-Q2",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let out_hint = format!("--out \"{}\"", tmp.path().display());

    assert!(stdout.contains("Compare: run_fixture -> run_period_to"));
    assert!(stdout.contains("- from: 3"));
    assert!(stdout.contains("- to: 8"));
    assert!(stdout.contains("- Added: Local git, GitLab, Jira, JSON, Linear, Manual"));
    assert!(stdout.contains(&format!("shiplog review {out_hint}")));
    assert!(stdout.contains("--run run_period_to"));
}

#[test]
fn runs_compare_requires_selector_for_each_side() {
    let tmp = TempDir::new().unwrap();

    shiplog_cmd()
        .args([
            "runs",
            "compare",
            "--out",
            tmp.path().to_str().unwrap(),
            "--to",
            "latest",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("missing --from or --from-period"));
}

#[test]
fn runs_compare_rejects_run_and_period_for_same_side() {
    let tmp = TempDir::new().unwrap();

    shiplog_cmd()
        .args([
            "runs",
            "compare",
            "--out",
            tmp.path().to_str().unwrap(),
            "--from",
            "run_fixture",
            "--from-period",
            "2025-Q1",
            "--to",
            "latest",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "use either --from or --from-period, not both",
        ));
}

#[test]
fn review_latest_summarizes_run_attention_items_without_writing_artifacts() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());
    let packet_before = std::fs::read_to_string(run_dir.join("packet.md")).unwrap();
    let coverage_before = std::fs::read_to_string(run_dir.join("coverage.manifest.json")).unwrap();

    let assert = shiplog_cmd()
        .args(["review", "--out", tmp.path().to_str().unwrap(), "--latest"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let out_hint = format!("--out \"{}\"", tmp.path().display());

    assert!(stdout.contains("Run: run_fixture"));
    assert!(stdout.contains("Coverage:"));
    assert!(stdout.contains("- GitHub: 3 event(s)"));
    assert!(stdout.contains("Completeness: Complete"));
    assert!(stdout.contains("Curation:"));
    assert!(stdout.contains("- Validation: ok"));
    assert!(stdout.contains("Evidence debt:"));
    assert!(stdout.contains("- No obvious evidence debt detected."));
    assert!(stdout.contains("Next:"));
    assert!(stdout.contains(&format!("shiplog render {out_hint}")));
    assert!(stdout.contains("--run run_fixture --mode scaffold"));

    assert_eq!(
        packet_before,
        std::fs::read_to_string(run_dir.join("packet.md")).unwrap()
    );
    assert_eq!(
        coverage_before,
        std::fs::read_to_string(run_dir.join("coverage.manifest.json")).unwrap()
    );
}

#[test]
fn review_weekly_summarizes_latest_run_without_writing_artifacts() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());
    let packet_before = std::fs::read_to_string(run_dir.join("packet.md")).unwrap();
    let coverage_before = std::fs::read_to_string(run_dir.join("coverage.manifest.json")).unwrap();

    let assert = shiplog_cmd()
        .args([
            "review",
            "weekly",
            "--out",
            tmp.path().to_str().unwrap(),
            "--latest",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let out_hint = format!("--out \"{}\"", tmp.path().display());

    assert!(stdout.contains("Weekly review: run_fixture"));
    assert!(stdout.contains("New evidence:"));
    assert!(stdout.contains("- total: 3 event(s)"));
    assert!(stdout.contains("- GitHub: 3 event(s)"));
    assert!(stdout.contains("Evidence debt:"));
    assert!(stdout.contains("- No obvious evidence debt detected."));
    assert!(stdout.contains("Next:"));
    assert!(stdout.contains(&format!("shiplog render {out_hint}")));
    assert!(stdout.contains("--run run_fixture --mode scaffold"));

    assert_eq!(
        packet_before,
        std::fs::read_to_string(run_dir.join("packet.md")).unwrap()
    );
    assert_eq!(
        coverage_before,
        std::fs::read_to_string(run_dir.join("coverage.manifest.json")).unwrap()
    );
}

#[test]
fn review_latest_surfaces_skipped_sources_and_manual_context() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");
    write_manual_events(&tmp.path().join("manual_events.yaml"));

    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[defaults]
window = "year:2025"

[sources.json]
enabled = true
events = "./missing-ledger.events.jsonl"
coverage = "./missing-coverage.manifest.json"

[sources.manual]
enabled = true
events = "./manual_events.yaml"
user = "octo"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .args([
            "collect",
            "--out",
            out.to_str().unwrap(),
            "multi",
            "--config",
            tmp.path().join("shiplog.toml").to_str().unwrap(),
        ])
        .assert()
        .success();

    let run_dir = first_run_dir(&out);
    let packet_before = std::fs::read_to_string(run_dir.join("packet.md")).unwrap();
    let coverage_before = std::fs::read_to_string(run_dir.join("coverage.manifest.json")).unwrap();

    let assert = shiplog_cmd()
        .args(["review", "--out", out.to_str().unwrap(), "--latest"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert!(stdout.contains("Coverage:"));
    assert!(stdout.contains("- Manual: 1 event(s)"));
    assert!(stdout.contains("Completeness: Partial"));
    assert!(stdout.contains("Skipped sources:"));
    assert!(stdout.contains("- JSON:"));
    assert!(stdout.contains("Evidence debt:"));
    assert!(stdout.contains("[warning] missing-source"));
    assert!(stdout.contains("[warning] partial-coverage"));
    assert!(stdout.contains("[info] manual-context"));
    assert!(stdout.contains("shiplog doctor"));

    let weekly_assert = shiplog_cmd()
        .args([
            "review",
            "weekly",
            "--out",
            out.to_str().unwrap(),
            "--latest",
        ])
        .assert()
        .success();
    let weekly_stdout = String::from_utf8(weekly_assert.get_output().stdout.clone()).unwrap();

    assert!(weekly_stdout.contains("Weekly review:"));
    assert!(weekly_stdout.contains("New evidence:"));
    assert!(weekly_stdout.contains("- Manual: 1 event(s)"));
    assert!(weekly_stdout.contains("Source gaps:"));
    assert!(weekly_stdout.contains("- JSON:"));
    assert!(weekly_stdout.contains("Evidence debt:"));
    assert!(weekly_stdout.contains("[warning] missing-source"));
    assert!(weekly_stdout.contains("shiplog doctor"));

    let fixups_assert = shiplog_cmd()
        .args([
            "review",
            "fixups",
            "--out",
            out.to_str().unwrap(),
            "--latest",
        ])
        .assert()
        .success();
    let fixups_stdout = String::from_utf8(fixups_assert.get_output().stdout.clone()).unwrap();

    assert!(fixups_stdout.contains("Review fixups:"));
    assert!(fixups_stdout.contains("Repair skipped source setup"));
    assert!(fixups_stdout.contains("Skipped sources: JSON."));
    assert!(fixups_stdout.contains("shiplog doctor"));

    assert_eq!(
        packet_before,
        std::fs::read_to_string(run_dir.join("packet.md")).unwrap()
    );
    assert_eq!(
        coverage_before,
        std::fs::read_to_string(run_dir.join("coverage.manifest.json")).unwrap()
    );
}

#[test]
fn review_suggests_journal_add_for_broad_workstream_without_manual_context() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");
    let events_path = tmp.path().join("ledger.events.jsonl");
    let coverage_path = tmp.path().join("coverage.manifest.json");
    let events: Vec<_> = (0..10)
        .map(|idx| {
            fixture_pr_event(
                SourceSystem::Github,
                "acme/platform",
                200 + idx as u64,
                &format!("Platform reliability fix {}", idx + 1),
                2 + idx,
            )
        })
        .collect();
    let coverage = CoverageManifest {
        run_id: RunId("run_broad_code".into()),
        generated_at: fixture_time(20),
        user: "octo".into(),
        window: fixture_window(),
        mode: "fixture".into(),
        sources: vec!["github".into()],
        slices: vec![CoverageSlice {
            window: fixture_window(),
            query: "github fixture".into(),
            total_count: events.len() as u64,
            fetched: events.len() as u64,
            incomplete_results: Some(false),
            notes: vec!["fixture".into()],
        }],
        warnings: vec![],
        completeness: Completeness::Complete,
    };
    write_events_jsonl(&events_path, &events);
    write_coverage_manifest(&coverage_path, &coverage);

    shiplog_cmd()
        .args([
            "collect",
            "--out",
            out.to_str().unwrap(),
            "json",
            "--events",
            events_path.to_str().unwrap(),
            "--coverage",
            coverage_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    let run_dir = out.join("run_broad_code");
    let packet_before = std::fs::read_to_string(run_dir.join("packet.md")).unwrap();
    let coverage_before = std::fs::read_to_string(run_dir.join("coverage.manifest.json")).unwrap();

    let assert = shiplog_cmd()
        .args(["review", "--out", out.to_str().unwrap(), "--latest"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert!(stdout.contains("[info] manual-context"));
    assert!(stdout.contains("broad workstream(s) have no manual outcome note"));
    assert!(stdout.contains("shiplog journal add --date"));
    assert!(stdout.contains("--title \"Outcome note for acme/platform\""));
    assert!(stdout.contains("--workstream \"acme/platform\""));
    assert!(
        !stdout.contains("[info] code-only-workstream"),
        "manual-context debt should cover broad code-only workstreams without printing a duplicate outcome-context debt item. stdout:\n{stdout}"
    );

    assert_eq!(
        packet_before,
        std::fs::read_to_string(run_dir.join("packet.md")).unwrap()
    );
    assert_eq!(
        coverage_before,
        std::fs::read_to_string(run_dir.join("coverage.manifest.json")).unwrap()
    );
}

#[test]
fn review_fixups_ranks_curation_actions_without_writing_artifacts() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");
    let events_path = tmp.path().join("ledger.events.jsonl");
    let coverage_path = tmp.path().join("coverage.manifest.json");
    let events: Vec<_> = (0..10)
        .map(|idx| {
            fixture_pr_event(
                SourceSystem::Github,
                "acme/platform",
                300 + idx as u64,
                &format!("Platform reliability fix {}", idx + 1),
                2 + idx,
            )
        })
        .collect();
    let coverage = CoverageManifest {
        run_id: RunId("run_fixups".into()),
        generated_at: fixture_time(20),
        user: "octo".into(),
        window: fixture_window(),
        mode: "fixture".into(),
        sources: vec!["github".into()],
        slices: vec![CoverageSlice {
            window: fixture_window(),
            query: "github fixture".into(),
            total_count: events.len() as u64,
            fetched: events.len() as u64,
            incomplete_results: Some(false),
            notes: vec!["fixture".into()],
        }],
        warnings: vec![],
        completeness: Completeness::Complete,
    };
    write_events_jsonl(&events_path, &events);
    write_coverage_manifest(&coverage_path, &coverage);

    shiplog_cmd()
        .args([
            "collect",
            "--out",
            out.to_str().unwrap(),
            "json",
            "--events",
            events_path.to_str().unwrap(),
            "--coverage",
            coverage_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    let run_dir = out.join("run_fixups");
    let curated = WorkstreamsFile {
        version: 1,
        generated_at: fixture_time(21),
        workstreams: vec![Workstream {
            id: WorkstreamId::from_parts(["repo", "acme/platform"]),
            title: "acme/platform".into(),
            summary: None,
            tags: vec!["repo".into()],
            stats: WorkstreamStats {
                pull_requests: events.len(),
                reviews: 0,
                manual_events: 0,
            },
            events: events.iter().map(|event| event.id.clone()).collect(),
            receipts: vec![],
        }],
    };
    std::fs::write(
        run_dir.join("workstreams.yaml"),
        serde_yaml::to_string(&curated).unwrap(),
    )
    .unwrap();
    let packet_before = std::fs::read_to_string(run_dir.join("packet.md")).unwrap();
    let coverage_before = std::fs::read_to_string(run_dir.join("coverage.manifest.json")).unwrap();
    let workstreams_before = std::fs::read_to_string(run_dir.join("workstreams.yaml")).unwrap();

    let assert = shiplog_cmd()
        .args([
            "review",
            "fixups",
            "--out",
            out.to_str().unwrap(),
            "--latest",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert!(stdout.contains("Review fixups: run_fixups"));
    assert!(stdout.contains("Top fixups:"));
    assert!(stdout.contains("1. Add outcome context for \"acme/platform\""));
    assert!(stdout.contains("shiplog journal add --date"));
    assert!(stdout.contains("--title \"Outcome note for acme/platform\""));
    assert!(stdout.contains("2. Select anchor receipts for \"acme/platform\""));
    assert!(stdout.contains("shiplog workstreams receipts --out"));
    assert!(stdout.contains("--run run_fixups --workstream \"acme/platform\""));
    assert!(stdout.contains("3. Split broad workstream \"acme/platform\""));
    assert!(stdout.contains("shiplog workstreams split --out"));
    assert!(stdout.contains("--run run_fixups --from \"acme/platform\""));
    assert!(stdout.contains("shiplog review --out"));
    assert!(stdout.contains("shiplog render --out"));
    assert!(stdout.contains("--mode scaffold"));

    let commands_assert = shiplog_cmd()
        .args([
            "review",
            "fixups",
            "--out",
            out.to_str().unwrap(),
            "--latest",
            "--commands-only",
        ])
        .assert()
        .success();
    let commands_stdout = String::from_utf8(commands_assert.get_output().stdout.clone()).unwrap();
    assert!(!commands_stdout.contains("Review fixups:"));
    assert!(!commands_stdout.contains("Top fixups:"));
    assert!(!commands_stdout.contains("Next:"));
    let commands: Vec<_> = commands_stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    assert_eq!(
        commands.len(),
        4,
        "commands-only should print just the ranked fixup commands"
    );
    assert!(commands[0].starts_with("shiplog journal add --date"));
    assert!(commands[1].starts_with("shiplog workstreams receipts --out"));
    assert!(commands[2].starts_with("shiplog workstreams split --out"));
    assert!(commands[3].starts_with("shiplog journal add --date"));
    assert!(commands.iter().all(|line| line.starts_with("shiplog ")));

    let templates_assert = shiplog_cmd()
        .args([
            "review",
            "fixups",
            "--out",
            out.to_str().unwrap(),
            "--run",
            "run_fixups",
            "--journal-template",
        ])
        .assert()
        .success();
    let templates_stdout = String::from_utf8(templates_assert.get_output().stdout.clone()).unwrap();
    assert!(!templates_stdout.contains("Review fixups:"));
    assert!(!templates_stdout.contains("Top fixups:"));
    assert!(!templates_stdout.contains("shiplog workstreams"));
    let templates: Vec<_> = templates_stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    assert_eq!(
        templates.len(),
        1,
        "journal-template should dedupe repeated context gaps for one workstream"
    );
    assert!(templates[0].starts_with("shiplog journal add --date"));
    assert!(templates[0].contains("--title \"Outcome note for acme/platform\""));
    assert!(templates[0].contains("--workstream \"acme/platform\""));
    assert!(templates[0].contains("--description \"<replace with factual context or outcome>\""));

    shiplog_cmd()
        .args([
            "review",
            "fixups",
            "--out",
            out.to_str().unwrap(),
            "--latest",
            "--commands-only",
            "--journal-template",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "either --commands-only or --journal-template",
        ));

    assert_eq!(
        packet_before,
        std::fs::read_to_string(run_dir.join("packet.md")).unwrap()
    );
    assert_eq!(
        coverage_before,
        std::fs::read_to_string(run_dir.join("coverage.manifest.json")).unwrap()
    );
    assert_eq!(
        workstreams_before,
        std::fs::read_to_string(run_dir.join("workstreams.yaml")).unwrap()
    );
}

#[test]
fn review_surfaces_packet_quality_evidence_debt_categories() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");
    let events_path = tmp.path().join("ledger.events.jsonl");
    let coverage_path = tmp.path().join("coverage.manifest.json");
    let mut events = Vec::new();

    let misc_events: Vec<_> = (0..5)
        .map(|idx| {
            fixture_pr_event(
                SourceSystem::Github,
                "acme/misc",
                400 + idx as u64,
                &format!("Misc cleanup {}", idx + 1),
                2 + idx,
            )
        })
        .collect();
    let ticket_events: Vec<_> = (0..5)
        .map(|idx| {
            fixture_manual_event(
                SourceSystem::Other("jira".into()),
                "acme/support",
                &format!("Support ticket {}", idx + 1),
                ManualEventType::Other,
                8 + idx,
            )
        })
        .collect();
    let manual_events = [fixture_manual_event(
        SourceSystem::Manual,
        "acme/manual",
        "Customer reliability context",
        ManualEventType::Incident,
        14,
    )];
    let release_events: Vec<_> = (0..6)
        .map(|idx| {
            fixture_pr_event(
                SourceSystem::Github,
                "acme/release",
                500 + idx as u64,
                &format!("Release automation {}", idx + 1),
                15 + idx,
            )
        })
        .collect();

    events.extend(misc_events.iter().cloned());
    events.extend(ticket_events.iter().cloned());
    events.extend(manual_events.iter().cloned());
    events.extend(release_events.iter().cloned());

    let coverage = CoverageManifest {
        run_id: RunId("run_quality_debt".into()),
        generated_at: fixture_time(25),
        user: "octo".into(),
        window: fixture_window(),
        mode: "fixture".into(),
        sources: vec!["github".into(), "jira".into(), "manual".into()],
        slices: vec![CoverageSlice {
            window: fixture_window(),
            query: "quality fixture".into(),
            total_count: events.len() as u64,
            fetched: events.len() as u64,
            incomplete_results: Some(false),
            notes: vec!["fixture".into()],
        }],
        warnings: vec![],
        completeness: Completeness::Complete,
    };
    write_events_jsonl(&events_path, &events);
    write_coverage_manifest(&coverage_path, &coverage);

    shiplog_cmd()
        .args([
            "collect",
            "--out",
            out.to_str().unwrap(),
            "json",
            "--events",
            events_path.to_str().unwrap(),
            "--coverage",
            coverage_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    let run_dir = out.join("run_quality_debt");
    let curated = WorkstreamsFile {
        version: 1,
        generated_at: fixture_time(26),
        workstreams: vec![
            Workstream {
                id: WorkstreamId::from_parts(["quality", "misc"]),
                title: "Misc".into(),
                summary: None,
                tags: vec!["misc".into()],
                stats: WorkstreamStats {
                    pull_requests: misc_events.len(),
                    reviews: 0,
                    manual_events: 0,
                },
                events: misc_events.iter().map(|event| event.id.clone()).collect(),
                receipts: vec![],
            },
            Workstream {
                id: WorkstreamId::from_parts(["quality", "tickets"]),
                title: "Customer Tickets".into(),
                summary: None,
                tags: vec!["tickets".into()],
                stats: WorkstreamStats {
                    pull_requests: 0,
                    reviews: 0,
                    manual_events: ticket_events.len(),
                },
                events: ticket_events.iter().map(|event| event.id.clone()).collect(),
                receipts: vec![],
            },
            Workstream {
                id: WorkstreamId::from_parts(["quality", "manual"]),
                title: "Manual Context".into(),
                summary: None,
                tags: vec!["manual".into()],
                stats: WorkstreamStats {
                    pull_requests: 0,
                    reviews: 0,
                    manual_events: manual_events.len(),
                },
                events: manual_events.iter().map(|event| event.id.clone()).collect(),
                receipts: vec![],
            },
            Workstream {
                id: WorkstreamId::from_parts(["quality", "release"]),
                title: "Release Automation".into(),
                summary: None,
                tags: vec!["release".into()],
                stats: WorkstreamStats {
                    pull_requests: release_events.len(),
                    reviews: 0,
                    manual_events: 0,
                },
                events: release_events
                    .iter()
                    .map(|event| event.id.clone())
                    .collect(),
                receipts: release_events
                    .iter()
                    .map(|event| event.id.clone())
                    .collect(),
            },
        ],
    };
    std::fs::write(
        run_dir.join("workstreams.yaml"),
        serde_yaml::to_string(&curated).unwrap(),
    )
    .unwrap();
    let packet_before = std::fs::read_to_string(run_dir.join("packet.md")).unwrap();
    let coverage_before = std::fs::read_to_string(run_dir.join("coverage.manifest.json")).unwrap();
    let workstreams_before = std::fs::read_to_string(run_dir.join("workstreams.yaml")).unwrap();

    let assert = shiplog_cmd()
        .args(["review", "--out", out.to_str().unwrap(), "--latest"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let out_hint = format!("--out \"{}\"", out.display());

    for needle in [
        "[warning] no-selected-receipts".to_string(),
        "[info] thin-workstream".to_string(),
        "[warning] large-misc-workstream".to_string(),
        "[info] code-only-workstream".to_string(),
        "[info] ticket-only-workstream".to_string(),
        "[info] manual-only-workstream".to_string(),
        "[info] too-many-selected-receipts".to_string(),
        format!("shiplog workstreams receipts {out_hint}"),
        "--run run_quality_debt --workstream <title>".to_string(),
        format!("shiplog workstreams split {out_hint}"),
        "--run run_quality_debt".to_string(),
        "shiplog journal add --date".to_string(),
        "shiplog journal list".to_string(),
    ] {
        assert!(
            stdout.contains(&needle),
            "review output should mention {needle:?}"
        );
    }

    let fixups_assert = shiplog_cmd()
        .args([
            "review",
            "fixups",
            "--out",
            out.to_str().unwrap(),
            "--latest",
            "--commands-only",
        ])
        .assert()
        .success();
    let fixups_stdout = String::from_utf8(fixups_assert.get_output().stdout.clone()).unwrap();
    assert!(fixups_stdout.contains("shiplog workstreams split"));
    assert!(fixups_stdout.contains("shiplog workstreams receipts"));
    assert!(fixups_stdout.contains("shiplog journal add"));

    assert_eq!(
        packet_before,
        std::fs::read_to_string(run_dir.join("packet.md")).unwrap()
    );
    assert_eq!(
        coverage_before,
        std::fs::read_to_string(run_dir.join("coverage.manifest.json")).unwrap()
    );
    assert_eq!(
        workstreams_before,
        std::fs::read_to_string(run_dir.join("workstreams.yaml")).unwrap()
    );
}

#[test]
fn review_strict_fails_when_evidence_debt_exists_without_writing_artifacts() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");
    write_manual_events(&tmp.path().join("manual_events.yaml"));

    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[defaults]
window = "year:2025"

[sources.manual]
enabled = true
events = "./manual_events.yaml"
user = "octo"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .args([
            "collect",
            "--out",
            out.to_str().unwrap(),
            "multi",
            "--config",
            tmp.path().join("shiplog.toml").to_str().unwrap(),
        ])
        .assert()
        .success();

    let run_dir = first_run_dir(&out);
    let packet_before = std::fs::read_to_string(run_dir.join("packet.md")).unwrap();
    let coverage_before = std::fs::read_to_string(run_dir.join("coverage.manifest.json")).unwrap();

    shiplog_cmd()
        .args([
            "review",
            "--out",
            out.to_str().unwrap(),
            "--latest",
            "--strict",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Evidence debt:"))
        .stdout(predicate::str::contains("[info] manual-context"))
        .stderr(predicate::str::contains("review found"));

    assert_eq!(
        packet_before,
        std::fs::read_to_string(run_dir.join("packet.md")).unwrap()
    );
    assert_eq!(
        coverage_before,
        std::fs::read_to_string(run_dir.join("coverage.manifest.json")).unwrap()
    );
}

#[test]
fn open_packet_latest_prints_packet_path_when_forced() {
    let tmp = TempDir::new().unwrap();
    collect_json_into(tmp.path());

    let before = file_tree_manifest(tmp.path());
    let assert = shiplog_cmd()
        .args([
            "open",
            "packet",
            "--out",
            tmp.path().to_str().unwrap(),
            "--latest",
            "--print-path",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("run_fixture"))
        .stdout(predicate::str::contains("packet.md"));
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let after = file_tree_manifest(tmp.path());

    assert_eq!(
        before, after,
        "open packet --latest --print-path should not write or rewrite files"
    );
    assert!(
        !stdout.contains(r"\\?\"),
        "open packet --print-path should not expose a Windows verbatim path prefix: {stdout}"
    );
}

#[test]
fn open_packet_latest_selects_lexicographically_newest_run() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path();
    let newest = "2026-05-13T09-00-00Z-newest";
    let older = "2026-05-12T09-00-00Z-older";

    write_minimal_open_run(out, newest, "newest packet");
    std::thread::sleep(std::time::Duration::from_millis(25));
    write_minimal_open_run(out, older, "older packet");

    shiplog_cmd()
        .args([
            "open",
            "packet",
            "--out",
            out.to_str().unwrap(),
            "--latest",
            "--print-path",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(newest))
        .stdout(predicate::str::contains("packet.md"))
        .stdout(predicate::str::contains(older).not());
}

#[test]
fn open_latest_without_runs_prints_intake_command_to_create_one() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("missing runs");
    let out_arg = out.to_str().unwrap();

    shiplog_cmd()
        .args([
            "open",
            "packet",
            "--out",
            out_arg,
            "--latest",
            "--print-path",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No run directories found"))
        .stderr(predicate::str::contains(
            "shiplog intake --last-6-months --explain",
        ))
        .stderr(predicate::str::contains("--out"))
        .stderr(predicate::str::contains(out_arg));
}

fn write_minimal_open_run(out: &Path, run_id: &str, packet_body: &str) {
    let run = out.join(run_id);
    std::fs::create_dir_all(&run).unwrap();
    std::fs::write(run.join("ledger.events.jsonl"), "").unwrap();
    std::fs::write(run.join("packet.md"), packet_body).unwrap();
}

#[test]
fn open_workstreams_latest_prints_effective_workstreams_path_when_forced() {
    let tmp = TempDir::new().unwrap();
    collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "open",
            "workstreams",
            "--out",
            tmp.path().to_str().unwrap(),
            "--latest",
            "--print-path",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("run_fixture"))
        .stdout(predicate::str::contains("workstreams.suggested.yaml"));
}

#[test]
fn open_out_latest_prints_run_directory_when_forced() {
    let tmp = TempDir::new().unwrap();
    collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "open",
            "out",
            "--out",
            tmp.path().to_str().unwrap(),
            "--latest",
            "--print-path",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("run_fixture"));
}

#[test]
fn open_intake_report_latest_prints_report_path_when_forced() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["intake", "--out", out.to_str().unwrap(), "--no-open"])
        .assert()
        .success();

    shiplog_cmd()
        .args([
            "open",
            "intake-report",
            "--out",
            out.to_str().unwrap(),
            "--latest",
            "--print-path",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("intake.report.md"));
}

#[test]
fn open_report_alias_prints_intake_report_path_when_forced() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["intake", "--out", out.to_str().unwrap(), "--no-open"])
        .assert()
        .success();

    shiplog_cmd()
        .args([
            "open",
            "report",
            "--out",
            out.to_str().unwrap(),
            "--latest",
            "--print-path",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("intake.report.md"));
}

#[test]
fn report_validate_accepts_latest_and_direct_path() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["intake", "--out", out.to_str().unwrap(), "--no-open"])
        .assert()
        .success();

    let run_dir = first_run_dir(&out);
    let report_path = run_dir.join("intake.report.json");

    shiplog_cmd()
        .args([
            "report",
            "validate",
            "--out",
            out.to_str().unwrap(),
            "--latest",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Report valid:"))
        .stdout(predicate::str::contains("Schema: v1"))
        .stdout(predicate::str::contains("Intake status:"))
        .stdout(predicate::str::contains("Packet readiness:"))
        .stdout(predicate::str::contains("Artifacts:"));

    shiplog_cmd()
        .args([
            "report",
            "validate",
            "--path",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Report valid:"))
        .stdout(predicate::str::contains("intake.report.json"));
}

#[test]
fn report_validate_rejects_unsupported_schema_version() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["intake", "--out", out.to_str().unwrap(), "--no-open"])
        .assert()
        .success();

    let report_path = first_run_dir(&out).join("intake.report.json");
    let mut report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
    report["schema_version"] = serde_json::json!(2);
    std::fs::write(
        &report_path,
        format!("{}\n", serde_json::to_string_pretty(&report).unwrap()),
    )
    .unwrap();

    shiplog_cmd()
        .args([
            "report",
            "validate",
            "--path",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "unsupported intake report schema_version 2",
        ));
}

#[test]
fn report_validate_rejects_secret_sentinels() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["intake", "--out", out.to_str().unwrap(), "--no-open"])
        .assert()
        .success();

    let report_path = first_run_dir(&out).join("intake.report.json");
    let mut report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
    report["good"] = serde_json::json!(["super-secret"]);
    std::fs::write(
        &report_path,
        format!("{}\n", serde_json::to_string_pretty(&report).unwrap()),
    )
    .unwrap();

    shiplog_cmd()
        .args([
            "report",
            "validate",
            "--path",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("contains secret sentinel"));
}

#[test]
fn report_validate_accepts_legacy_repair_sources_without_kind() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["intake", "--out", out.to_str().unwrap(), "--no-open"])
        .assert()
        .success();

    let report_path = first_run_dir(&out).join("intake.report.json");
    let mut report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
    report["repair_sources"] = serde_json::json!([
        {
            "source": "Jira",
            "reason": "missing JIRA_TOKEN",
            "commands": ["export JIRA_TOKEN=..."]
        }
    ]);
    std::fs::write(
        &report_path,
        format!("{}\n", serde_json::to_string_pretty(&report).unwrap()),
    )
    .unwrap();

    shiplog_cmd()
        .args([
            "report",
            "validate",
            "--path",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Report valid:"));
}

#[test]
fn report_validate_accepts_legacy_fixups_without_id_or_kind() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["intake", "--out", out.to_str().unwrap(), "--no-open"])
        .assert()
        .success();

    let report_path = first_run_dir(&out).join("intake.report.json");
    let mut report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
    report["top_fixups"] = serde_json::json!([
        {
            "title": "Add outcome context",
            "detail": null,
            "command": "shiplog journal add --title \"Outcome note\""
        }
    ]);
    std::fs::write(
        &report_path,
        format!("{}\n", serde_json::to_string_pretty(&report).unwrap()),
    )
    .unwrap();

    shiplog_cmd()
        .args([
            "report",
            "validate",
            "--path",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Report valid:"));
}

#[test]
fn report_validate_accepts_legacy_reports_without_actions() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["intake", "--out", out.to_str().unwrap(), "--no-open"])
        .assert()
        .success();

    let report_path = first_run_dir(&out).join("intake.report.json");
    let mut report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
    report
        .as_object_mut()
        .expect("report should be an object")
        .remove("actions");
    std::fs::write(
        &report_path,
        format!("{}\n", serde_json::to_string_pretty(&report).unwrap()),
    )
    .unwrap();

    shiplog_cmd()
        .args([
            "report",
            "validate",
            "--path",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Report valid:"));
}

#[test]
fn report_validate_accepts_legacy_reports_without_repair_items() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["intake", "--out", out.to_str().unwrap(), "--no-open"])
        .assert()
        .success();

    let report_path = first_run_dir(&out).join("intake.report.json");
    let mut report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
    report
        .as_object_mut()
        .expect("report should be an object")
        .remove("repair_items");
    std::fs::write(
        &report_path,
        format!("{}\n", serde_json::to_string_pretty(&report).unwrap()),
    )
    .unwrap();

    shiplog_cmd()
        .args([
            "report",
            "validate",
            "--path",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Report valid:"));
}

#[test]
fn report_validate_accepts_legacy_reports_without_packet_quality() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["intake", "--out", out.to_str().unwrap(), "--no-open"])
        .assert()
        .success();

    let report_path = first_run_dir(&out).join("intake.report.json");
    let mut report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
    report
        .as_object_mut()
        .expect("report should be an object")
        .remove("packet_quality");
    std::fs::write(
        &report_path,
        format!("{}\n", serde_json::to_string_pretty(&report).unwrap()),
    )
    .unwrap();

    shiplog_cmd()
        .args([
            "report",
            "validate",
            "--path",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Report valid:"));
}

#[test]
fn report_validate_accepts_legacy_packet_quality_without_candidates_or_share_posture() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["intake", "--out", out.to_str().unwrap(), "--no-open"])
        .assert()
        .success();

    let report_path = first_run_dir(&out).join("intake.report.json");
    remove_packet_quality_fields_from_report(&report_path, &["claim_candidates", "share_posture"])
        .unwrap();

    shiplog_cmd()
        .args([
            "report",
            "validate",
            "--path",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Report valid:"));
}

#[test]
fn repair_plan_latest_renders_repair_items_from_latest_report() -> CliTestResult {
    let tmp = TempDir::new()?;
    let out = tmp.path().join("out");
    let out_arg = out.to_string_lossy().to_string();

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["intake", "--out", out_arg.as_str(), "--no-open"])
        .assert()
        .success();

    shiplog_cmd()
        .args(["repair", "plan", "--out", out_arg.as_str(), "--latest"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Repair plan:"))
        .stdout(predicate::str::contains("Intake status:"))
        .stdout(predicate::str::contains("Packet readiness:"))
        .stdout(predicate::str::contains("Repair queue:"))
        .stdout(predicate::str::contains("repair_"))
        .stdout(predicate::str::contains("manual_evidence_missing"))
        .stdout(predicate::str::contains("Action: journal_add"))
        .stdout(predicate::str::contains("Command: shiplog journal add"))
        .stdout(predicate::str::contains("--from-repair"))
        .stdout(predicate::str::contains("Receipts:"));

    Ok(())
}

#[test]
fn repair_plan_latest_accepts_relative_out_report_paths() -> CliTestResult {
    let tmp = TempDir::new()?;
    let out_arg = "relative-out";

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["intake", "--out", out_arg, "--no-open"])
        .assert()
        .success();

    shiplog_cmd()
        .current_dir(tmp.path())
        .args(["repair", "plan", "--out", out_arg, "--latest"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Repair plan:"))
        .stdout(predicate::str::contains("Repair queue:"))
        .stdout(predicate::str::contains(
            "shiplog journal add --from-repair",
        ))
        .stdout(predicate::str::contains("--out \"relative-out\" --latest"));

    Ok(())
}

#[test]
fn repair_plan_latest_without_runs_prints_intake_command() -> CliTestResult {
    let tmp = TempDir::new()?;
    let out = tmp.path().join("missing runs");
    let out_arg = out.to_string_lossy().to_string();

    shiplog_cmd()
        .args(["repair", "plan", "--out", out_arg.as_str(), "--latest"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Repair plan: no latest intake report found",
        ))
        .stdout(predicate::str::contains(
            "shiplog intake --last-6-months --explain",
        ))
        .stdout(predicate::str::contains("--out"))
        .stdout(predicate::str::contains(out_arg));

    Ok(())
}

#[test]
fn repair_plan_latest_handles_legacy_report_without_repair_items() -> CliTestResult {
    let tmp = TempDir::new()?;
    let out = tmp.path().join("out");
    let out_arg = out.to_string_lossy().to_string();

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["intake", "--out", out_arg.as_str(), "--no-open"])
        .assert()
        .success();

    let report_path = first_run_dir(&out).join("intake.report.json");
    let report_text = std::fs::read_to_string(&report_path)?;
    let mut report: serde_json::Value = serde_json::from_str(&report_text)?;
    let Some(report_object) = report.as_object_mut() else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "report should be an object",
        )
        .into());
    };
    report_object.remove("repair_items");
    report_object.remove("packet_quality");
    std::fs::write(
        &report_path,
        format!("{}\n", serde_json::to_string_pretty(&report)?),
    )?;

    shiplog_cmd()
        .args(["repair", "plan", "--out", out_arg.as_str(), "--latest"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Intake status:"))
        .stdout(predicate::str::contains("Packet readiness:").not())
        .stdout(predicate::str::contains(
            "Repair items: unavailable in this compatible v1 report.",
        ))
        .stdout(predicate::str::contains(
            "shiplog intake --last-6-months --explain",
        ))
        .stdout(predicate::str::contains("--out"));

    Ok(())
}

#[test]
fn repair_plan_latest_handles_empty_repair_queue() -> CliTestResult {
    let tmp = TempDir::new()?;
    let out = tmp.path().join("out");
    let out_arg = out.to_string_lossy().to_string();

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["intake", "--out", out_arg.as_str(), "--no-open"])
        .assert()
        .success();

    let report_path = first_run_dir(&out).join("intake.report.json");
    let report_text = std::fs::read_to_string(&report_path)?;
    let mut report: serde_json::Value = serde_json::from_str(&report_text)?;
    report["repair_items"] = serde_json::json!([]);
    std::fs::write(
        &report_path,
        format!("{}\n", serde_json::to_string_pretty(&report)?),
    )?;

    shiplog_cmd()
        .args(["repair", "plan", "--out", out_arg.as_str(), "--latest"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Repair queue: 0 item(s)"))
        .stdout(predicate::str::contains("No repair items found."));

    Ok(())
}

#[test]
fn repair_plan_latest_rejects_invalid_report_json() -> CliTestResult {
    let tmp = TempDir::new()?;
    let out = tmp.path().join("out");
    let run = out.join("run_bad");
    std::fs::create_dir_all(&run)?;
    std::fs::write(run.join("ledger.events.jsonl"), "")?;
    std::fs::write(run.join("intake.report.json"), "not json\n")?;
    let out_arg = out.to_string_lossy().to_string();

    shiplog_cmd()
        .args(["repair", "plan", "--out", out_arg.as_str(), "--latest"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("parse"))
        .stderr(predicate::str::contains("intake.report.json"));

    Ok(())
}

#[test]
fn repair_plan_run_rejects_path_traversal() -> CliTestResult {
    let tmp = TempDir::new()?;
    let out = tmp.path().join("out");
    let out_arg = out.to_string_lossy().to_string();

    shiplog_cmd()
        .args([
            "repair",
            "plan",
            "--out",
            out_arg.as_str(),
            "--run",
            "../outside",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "repair plan --run must be a single run directory name",
        ));

    Ok(())
}

#[test]
fn repair_diff_latest_shows_cleared_new_still_open_and_changed_items() -> CliTestResult {
    let tmp = TempDir::new()?;
    let out = tmp.path().join("out");
    let out_arg = out.to_string_lossy().to_string();
    run_intake_without_provider_tokens(tmp.path(), &out);
    let (_, base_report) = load_first_intake_report(&out);

    write_repair_diff_report(
        &base_report,
        &out,
        "run_900_old",
        serde_json::json!([
            repair_diff_item(
                "repair_001_still_old",
                "manual:still",
                "still open",
                "shiplog journal add --from-repair repair_001_still_old",
                "manual event exists"
            ),
            repair_diff_item(
                "repair_002_changed_old",
                "manual:changed",
                "old reason",
                "shiplog journal add --from-repair repair_002_changed_old",
                "old clear condition"
            ),
            repair_diff_item(
                "repair_003_cleared_old",
                "manual:cleared",
                "cleared reason",
                "shiplog journal add --from-repair repair_003_cleared_old",
                "cleared condition"
            )
        ]),
    );
    write_repair_diff_report(
        &base_report,
        &out,
        "run_901_new",
        serde_json::json!([
            repair_diff_item(
                "repair_001_still_new",
                "manual:still",
                "still open",
                "shiplog journal add --from-repair repair_001_still_old",
                "manual event exists"
            ),
            repair_diff_item(
                "repair_002_changed_new",
                "manual:changed",
                "new reason",
                "shiplog journal add --from-repair repair_002_changed_new",
                "new clear condition"
            ),
            repair_diff_item(
                "repair_004_new",
                "manual:new",
                "new reason",
                "shiplog journal add --from-repair repair_004_new",
                "new condition"
            )
        ]),
    );

    let before = file_tree_manifest(tmp.path());
    shiplog_cmd()
        .args(["repair", "diff", "--out", out_arg.as_str(), "--latest"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Repair diff: run_900_old -> run_901_new",
        ))
        .stdout(predicate::str::contains("Cleared: 1"))
        .stdout(predicate::str::contains("manual:cleared"))
        .stdout(predicate::str::contains("New: 1"))
        .stdout(predicate::str::contains("manual:new"))
        .stdout(predicate::str::contains("Still open: 1"))
        .stdout(predicate::str::contains("manual:still"))
        .stdout(predicate::str::contains("Changed: 1"))
        .stdout(predicate::str::contains("manual:changed"))
        .stdout(predicate::str::contains("Reason: old reason -> new reason"))
        .stdout(predicate::str::contains(
            "Clears when: old clear condition -> new clear condition",
        ))
        .stdout(predicate::str::contains("Next:"))
        .stdout(predicate::str::contains(format!(
            "shiplog runs diff --out \"{}\" --latest",
            out.display()
        )))
        .stdout(predicate::str::contains(format!(
            "shiplog open packet --out \"{}\" --run run_901_new",
            out.display()
        )))
        .stdout(predicate::str::contains(format!(
            "shiplog share explain manager --out \"{}\" --run run_901_new",
            out.display()
        )));
    let after = file_tree_manifest(tmp.path());

    assert_eq!(
        before, after,
        "repair diff should not write or rewrite files"
    );

    Ok(())
}

#[test]
fn repair_diff_clears_source_repairs_only_after_source_contributes() -> CliTestResult {
    let tmp = TempDir::new()?;
    let out = tmp.path().join("out");
    let out_arg = out.to_string_lossy().to_string();
    run_intake_without_provider_tokens(tmp.path(), &out);
    let (_, base_report) = load_first_intake_report(&out);

    write_repair_diff_report(
        &base_report,
        &out,
        "run_900_source_old",
        serde_json::json!([repair_diff_item(
            "repair_001_source_github_old",
            "source:github:source_skipped_configuration",
            "GitHub needs repair: GITHUB_TOKEN not found",
            "export GITHUB_TOKEN=...",
            "GitHub source contributes evidence on a rerun"
        )]),
    );
    let newer_report_path = write_repair_diff_report(
        &base_report,
        &out,
        "run_901_source_new",
        serde_json::json!([]),
    );
    let mut newer_report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&newer_report_path)?)?;
    newer_report["included_sources"] = serde_json::json!([
        {
            "source": "github",
            "source_key": "github",
            "source_label": "GitHub",
            "event_count": 1,
            "summary": "GitHub collected 1 event"
        }
    ]);
    std::fs::write(
        &newer_report_path,
        format!("{}\n", serde_json::to_string_pretty(&newer_report)?),
    )?;

    shiplog_cmd()
        .args(["repair", "diff", "--out", out_arg.as_str(), "--latest"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cleared: 1"))
        .stdout(predicate::str::contains(
            "source:github:source_skipped_configuration",
        ))
        .stdout(predicate::str::contains("Still open: 0"));

    Ok(())
}

#[test]
fn repair_diff_latest_without_two_reports_prints_next_command() -> CliTestResult {
    let tmp = TempDir::new()?;
    let out = tmp.path().join("missing-out");
    let out_arg = out.to_string_lossy().to_string();

    shiplog_cmd()
        .args(["repair", "diff", "--out", out_arg.as_str(), "--latest"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "fewer than two compatible repair reports found",
        ))
        .stdout(predicate::str::contains("Compatible reports: 0"))
        .stdout(predicate::str::contains(
            "shiplog intake --last-6-months --explain",
        ));

    Ok(())
}

#[test]
fn repair_diff_latest_reports_legacy_reports_without_repair_items() -> CliTestResult {
    let tmp = TempDir::new()?;
    let out = tmp.path().join("out");
    let out_arg = out.to_string_lossy().to_string();
    run_intake_without_provider_tokens(tmp.path(), &out);
    let (original_report_path, base_report) = load_first_intake_report(&out);
    let mut original_legacy_report = base_report.clone();
    original_legacy_report
        .as_object_mut()
        .expect("report should be an object")
        .remove("repair_items");
    std::fs::write(
        &original_report_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&original_legacy_report)?
        ),
    )?;

    let mut legacy_report = base_report.clone();
    legacy_report
        .as_object_mut()
        .expect("report should be an object")
        .remove("repair_items");
    write_repair_diff_report(
        &legacy_report,
        &out,
        "run_901_legacy",
        serde_json::json!([]),
    );
    let legacy_path = out.join("run_901_legacy").join("intake.report.json");
    let mut legacy_report_text: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&legacy_path)?)?;
    legacy_report_text
        .as_object_mut()
        .expect("report should be an object")
        .remove("repair_items");
    std::fs::write(
        &legacy_path,
        format!("{}\n", serde_json::to_string_pretty(&legacy_report_text)?),
    )?;

    write_repair_diff_report(
        &base_report,
        &out,
        "run_900_compatible",
        serde_json::json!([]),
    );

    shiplog_cmd()
        .args(["repair", "diff", "--out", out_arg.as_str(), "--latest"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "fewer than two compatible repair reports found",
        ))
        .stdout(predicate::str::contains("Compatible reports: 1"))
        .stdout(predicate::str::contains(
            "Skipped reports without repair_items:",
        ))
        .stdout(predicate::str::contains(
            "shiplog intake --last-6-months --explain",
        ));

    Ok(())
}

#[test]
fn repair_diff_latest_handles_empty_repair_queues() -> CliTestResult {
    let tmp = TempDir::new()?;
    let out = tmp.path().join("out");
    let out_arg = out.to_string_lossy().to_string();
    run_intake_without_provider_tokens(tmp.path(), &out);
    let (_, base_report) = load_first_intake_report(&out);

    write_repair_diff_report(&base_report, &out, "run_900_old", serde_json::json!([]));
    write_repair_diff_report(&base_report, &out, "run_901_new", serde_json::json!([]));

    shiplog_cmd()
        .args(["repair", "diff", "--out", out_arg.as_str(), "--latest"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cleared: 0"))
        .stdout(predicate::str::contains("New: 0"))
        .stdout(predicate::str::contains("Still open: 0"))
        .stdout(predicate::str::contains("Changed: 0"))
        .stdout(predicate::str::contains("No repair state changes."));

    Ok(())
}

#[test]
fn runs_diff_reports_changed_repairs_as_neutral_changes() -> CliTestResult {
    let tmp = TempDir::new()?;
    let out = tmp.path().join("out");
    let out_arg = out.to_string_lossy().to_string();
    run_intake_without_provider_tokens(tmp.path(), &out);
    let (_, base_report) = load_first_intake_report(&out);

    write_repair_diff_report(
        &base_report,
        &out,
        "run_900_old",
        serde_json::json!([repair_diff_item(
            "repair_001_changed_old",
            "manual:changed",
            "old reason",
            "shiplog journal add --from-repair repair_001_changed_old",
            "old clear condition"
        )]),
    );
    write_repair_diff_report(
        &base_report,
        &out,
        "run_901_new",
        serde_json::json!([repair_diff_item(
            "repair_001_changed_new",
            "manual:changed",
            "new reason",
            "shiplog journal add --from-repair repair_001_changed_new",
            "new clear condition"
        )]),
    );

    let assert = shiplog_cmd()
        .args(["runs", "diff", "--out", out_arg.as_str(), "--latest"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;

    assert!(
        stdout.contains("Changed:\n- repair manual:changed changed"),
        "changed repair items should be surfaced as neutral changes. stdout:\n{stdout}"
    );
    assert!(
        !stdout.contains("Regressed:\n- repair manual:changed"),
        "changed repair items should not be labeled as regressions. stdout:\n{stdout}"
    );

    Ok(())
}

#[test]
fn runs_diff_latest_reports_quality_without_writing() -> CliTestResult {
    let tmp = TempDir::new()?;
    let out = tmp.path().join("out");
    let out_arg = out.to_string_lossy().to_string();

    run_intake_without_provider_tokens(tmp.path(), &out);
    let (_, first_report) = load_first_intake_report(&out);
    let repair_id = first_repair_id_with_action(&first_report, "journal_add");
    shiplog_cmd()
        .current_dir(tmp.path())
        .args([
            "journal",
            "add",
            "--from-repair",
            repair_id.as_str(),
            "--out",
            out_arg.as_str(),
            "--latest",
        ])
        .assert()
        .success();
    run_intake_without_provider_tokens(tmp.path(), &out);

    let before = file_tree_manifest(tmp.path());
    let repair_diff = shiplog_cmd()
        .args(["repair", "diff", "--out", out_arg.as_str(), "--latest"])
        .assert()
        .success();
    let repair_diff_stdout = String::from_utf8(repair_diff.get_output().stdout.clone())?;
    assert!(
        repair_diff_stdout.contains("Still open:")
            && repair_diff_stdout.contains("source:github:source_skipped_configuration"),
        "repair diff should keep unproven source repairs open after a journal-only repair. stdout:\n{repair_diff_stdout}"
    );
    let repair_diff_cleared = repair_diff_stdout
        .split("Cleared:")
        .nth(1)
        .and_then(|section| section.split("New:").next())
        .unwrap_or("");
    assert!(
        !repair_diff_cleared.contains("source:github:source_skipped_configuration"),
        "repair diff should not mark an unproven source repair as cleared. stdout:\n{repair_diff_stdout}"
    );

    shiplog_cmd()
        .args(["runs", "diff", "--out", out_arg.as_str(), "--latest"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Packet quality diff:"))
        .stdout(predicate::str::contains("manual evidence count 0 -> 1"))
        .stdout(predicate::str::contains("claim candidates 0 -> 1"))
        .stdout(predicate::str::contains(
            "source:github:source_skipped_configuration still open",
        ))
        .stdout(
            predicate::str::contains("source:github:source_skipped_configuration cleared").not(),
        )
        .stdout(predicate::str::contains(format!(
            "shiplog open packet --out \"{}\"",
            out.display()
        )))
        .stdout(predicate::str::contains(format!(
            "shiplog share explain manager --out \"{}\"",
            out.display()
        )));
    let after = file_tree_manifest(tmp.path());

    assert_eq!(before, after, "runs diff should not write or rewrite files");

    Ok(())
}

#[test]
fn intake_setup_blocked_manual_handoff_starts_with_doctor() -> CliTestResult {
    let tmp = TempDir::new()?;
    let out = tmp.path().join("out");
    let out_arg = out.to_string_lossy().to_string();
    let fixtures = fixture_dir();
    let events = fixtures
        .join("ledger.events.jsonl")
        .display()
        .to_string()
        .replace('\\', "/");
    let coverage = fixtures
        .join("coverage.manifest.json")
        .display()
        .to_string()
        .replace('\\', "/");
    std::fs::write(tmp.path().join("manual_events.yaml"), "events: []\n")?;
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        format!(
            r#"[shiplog]
config_version = 1

[defaults]
out = "./out"
window = "last-6-months"
profile = "internal"

[sources.json]
enabled = true
events = "{events}"
coverage = "{coverage}"

[sources.manual]
enabled = true
events = "./manual_events.yaml"
"#
        ),
    )?;

    let assert = shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args([
            "intake",
            "--out",
            out_arg.as_str(),
            "--no-open",
            "--explain",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    assert!(
        stdout.contains(&format!(
            "Next:\n1. shiplog doctor --setup\n2. shiplog sources status\n3. shiplog repair plan --out \"{}\" --latest",
            out.display()
        )),
        "setup-blocked intake should start with doctor/source status before repair plan. stdout:\n{stdout}"
    );
    assert!(
        !stdout.contains("journal add --from-repair"),
        "malformed manual setup should not offer journal repair writes before setup is fixed. stdout:\n{stdout}"
    );

    let (_, report) = load_first_intake_report(&out);
    let next_commands = report["next_commands"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|command| command.as_str())
        .collect::<Vec<_>>();
    assert_eq!(next_commands.first(), Some(&"shiplog doctor --setup"));
    assert_eq!(next_commands.get(1), Some(&"shiplog sources status"));
    assert!(
        next_commands
            .get(2)
            .is_some_and(|command| command.starts_with("shiplog repair plan ")),
        "repair plan should remain available after setup preflight: {next_commands:?}"
    );
    assert!(
        next_commands
            .iter()
            .all(|command| !command.contains("journal add --from-repair")),
        "persisted next commands should not include dead-end journal repair writes: {next_commands:?}"
    );

    Ok(())
}

#[test]
fn intake_setup_blocked_provider_handoff_starts_with_doctor() -> CliTestResult {
    let tmp = TempDir::new()?;
    let out = tmp.path().join("out");
    let out_arg = out.to_string_lossy().to_string();
    write_manual_events(&tmp.path().join("manual_events.yaml"));
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[shiplog]
config_version = 1

[defaults]
out = "./out"
window = "last-6-months"
profile = "internal"

[sources.manual]
enabled = true
events = "./manual_events.yaml"

[sources.github]
enabled = true
user = "octo"
"#,
    )?;

    let assert = shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args([
            "intake",
            "--out",
            out_arg.as_str(),
            "--no-open",
            "--explain",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    assert!(
        stdout.contains(&format!(
            "Next:\n1. shiplog doctor --setup\n2. shiplog sources status\n3. shiplog repair plan --out \"{}\" --latest",
            out.display()
        )),
        "missing provider token should route through setup status before repair plan. stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("GitHub: missing GITHUB_TOKEN"),
        "intake should still explain the concrete provider setup gap. stdout:\n{stdout}"
    );

    let (_, report) = load_first_intake_report(&out);
    let next_commands = report["next_commands"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|command| command.as_str())
        .collect::<Vec<_>>();
    assert_eq!(next_commands.first(), Some(&"shiplog doctor --setup"));
    assert_eq!(next_commands.get(1), Some(&"shiplog sources status"));
    assert!(
        next_commands
            .get(2)
            .is_some_and(|command| command.starts_with("shiplog repair plan ")),
        "repair plan should remain available after setup preflight: {next_commands:?}"
    );

    Ok(())
}

#[test]
fn repaired_rerun_intake_hands_off_to_repair_diff_first() -> CliTestResult {
    let tmp = TempDir::new()?;
    let out = tmp.path().join("out");
    let out_arg = out.to_string_lossy().to_string();

    let first = shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args([
            "intake",
            "--out",
            out_arg.as_str(),
            "--no-open",
            "--explain",
        ])
        .assert()
        .success();
    let first_stdout = String::from_utf8(first.get_output().stdout.clone())?;
    assert!(
        first_stdout.contains(&format!(
            "Next:\n1. shiplog repair plan --out \"{}\" --latest",
            out.display()
        )),
        "first repairable intake should start at repair plan. stdout:\n{first_stdout}"
    );
    assert!(
        !first_stdout.contains("shiplog repair diff"),
        "first repairable intake should not offer repair diff before a prior report exists. stdout:\n{first_stdout}"
    );

    let (_, first_report) = load_first_intake_report(&out);
    let repair_id = first_repair_id_with_action(&first_report, "journal_add");
    shiplog_cmd()
        .current_dir(tmp.path())
        .args([
            "journal",
            "add",
            "--from-repair",
            repair_id.as_str(),
            "--out",
            out_arg.as_str(),
            "--latest",
        ])
        .assert()
        .success();

    let second = shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args([
            "intake",
            "--out",
            out_arg.as_str(),
            "--no-open",
            "--explain",
        ])
        .assert()
        .success();
    let second_stdout = String::from_utf8(second.get_output().stdout.clone())?;
    assert!(
        second_stdout.contains(&format!(
            "Next:\n1. shiplog repair diff --out \"{}\" --latest\n2. shiplog repair plan --out \"{}\" --latest",
            out.display(),
            out.display()
        )),
        "repaired rerun should hand off to repair diff before planning more work. stdout:\n{second_stdout}"
    );

    let latest_run = all_run_dirs(&out)
        .into_iter()
        .last()
        .expect("rerun should create a latest run");
    let latest_report: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        latest_run.join("intake.report.json"),
    )?)?;
    let next_commands = latest_report["next_commands"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|command| command.as_str())
        .collect::<Vec<_>>();
    assert!(
        next_commands
            .first()
            .is_some_and(|command| command.starts_with("shiplog repair diff ")),
        "latest report should persist repair diff as the first next command: {next_commands:?}"
    );
    assert!(
        next_commands
            .get(1)
            .is_some_and(|command| command.starts_with("shiplog repair plan ")),
        "latest report should keep repair plan as the second next command: {next_commands:?}"
    );

    Ok(())
}

#[test]
fn runs_diff_latest_handles_legacy_reports_without_packet_quality() -> CliTestResult {
    let tmp = TempDir::new()?;
    let out = tmp.path().join("out");
    let out_arg = out.to_string_lossy().to_string();

    run_intake_without_provider_tokens(tmp.path(), &out);
    let (_, first_report) = load_first_intake_report(&out);
    let repair_id = first_repair_id_with_action(&first_report, "journal_add");
    shiplog_cmd()
        .current_dir(tmp.path())
        .args([
            "journal",
            "add",
            "--from-repair",
            repair_id.as_str(),
            "--out",
            out_arg.as_str(),
            "--latest",
        ])
        .assert()
        .success();
    run_intake_without_provider_tokens(tmp.path(), &out);

    let run_dirs = all_run_dirs(&out);
    assert!(
        run_dirs.len() >= 2,
        "legacy packet-quality diff test needs two run directories"
    );
    for run_dir in &run_dirs {
        remove_packet_quality_from_report(&run_dir.join("intake.report.json"))?;
    }

    let before = file_tree_manifest(tmp.path());
    shiplog_cmd()
        .args(["runs", "diff", "--out", out_arg.as_str(), "--latest"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Packet quality diff:"))
        .stdout(predicate::str::contains("Reports:"))
        .stdout(predicate::str::contains("Improved:"))
        .stdout(predicate::str::contains("evidence events 0 -> 1"))
        .stdout(predicate::str::contains("manual evidence count 0 -> 1"))
        .stdout(predicate::str::contains("coverage gaps 6 -> 0"))
        .stdout(predicate::str::contains(
            "source:github:source_skipped_configuration still open",
        ))
        .stdout(
            predicate::str::contains("source:github:source_skipped_configuration cleared").not(),
        )
        .stdout(predicate::str::contains("packet readiness unavailable"))
        .stdout(predicate::str::contains(
            "packet quality unavailable; rerun intake for review-ready signals",
        ))
        .stdout(
            predicate::str::contains("packet readiness Needs evidence -> Ready for review").not(),
        )
        .stdout(predicate::str::contains("packet readiness: Ready for review").not())
        .stdout(predicate::str::contains(format!(
            "shiplog open packet --out \"{}\"",
            out.display()
        )));
    let after = file_tree_manifest(tmp.path());

    assert_eq!(
        before, after,
        "runs diff should not write or rewrite legacy report files"
    );

    Ok(())
}

#[test]
fn runs_diff_latest_handles_legacy_packet_quality_without_candidates_or_share_posture()
-> CliTestResult {
    let tmp = TempDir::new()?;
    let out = tmp.path().join("out");
    let out_arg = out.to_string_lossy().to_string();

    run_intake_without_provider_tokens(tmp.path(), &out);
    let (_, first_report) = load_first_intake_report(&out);
    let repair_id = first_repair_id_with_action(&first_report, "journal_add");
    shiplog_cmd()
        .current_dir(tmp.path())
        .args([
            "journal",
            "add",
            "--from-repair",
            repair_id.as_str(),
            "--out",
            out_arg.as_str(),
            "--latest",
        ])
        .assert()
        .success();
    run_intake_without_provider_tokens(tmp.path(), &out);

    let run_dirs = all_run_dirs(&out);
    assert!(
        run_dirs.len() >= 2,
        "legacy partial packet-quality diff test needs two run directories"
    );
    for run_dir in &run_dirs {
        remove_packet_quality_fields_from_report(
            &run_dir.join("intake.report.json"),
            &["claim_candidates", "share_posture"],
        )?;
    }

    let before = file_tree_manifest(tmp.path());
    shiplog_cmd()
        .args(["runs", "diff", "--out", out_arg.as_str(), "--latest"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Packet quality diff:"))
        .stdout(predicate::str::contains(
            "packet quality incomplete; rerun intake for review-ready signals",
        ))
        .stdout(predicate::str::contains("claim candidates 0 -> 1").not())
        .stdout(predicate::str::contains(format!(
            "shiplog open packet --out \"{}\"",
            out.display()
        )));
    let after = file_tree_manifest(tmp.path());

    assert_eq!(
        before, after,
        "runs diff should not write or rewrite partial legacy packet-quality reports"
    );

    Ok(())
}

#[test]
fn report_validate_rejects_unknown_source_key() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["intake", "--out", out.to_str().unwrap(), "--no-open"])
        .assert()
        .success();

    let report_path = first_run_dir(&out).join("intake.report.json");
    let mut report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
    report["source_decisions"][0]["source_key"] = serde_json::json!("storybook");
    std::fs::write(
        &report_path,
        format!("{}\n", serde_json::to_string_pretty(&report).unwrap()),
    )
    .unwrap();

    shiplog_cmd()
        .args([
            "report",
            "validate",
            "--path",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "source_decisions source_key \"storybook\" is not supported",
        ));
}

#[test]
fn report_validate_rejects_source_identity_mismatch() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["intake", "--out", out.to_str().unwrap(), "--no-open"])
        .assert()
        .success();

    let report_path = first_run_dir(&out).join("intake.report.json");
    let mut report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
    report["source_decisions"][0]["source"] = serde_json::json!("gitlab");
    std::fs::write(
        &report_path,
        format!("{}\n", serde_json::to_string_pretty(&report).unwrap()),
    )
    .unwrap();

    shiplog_cmd()
        .args([
            "report",
            "validate",
            "--path",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "intake report source_decisions source",
        ))
        .stderr(predicate::str::contains("not source_key"));
}

#[test]
fn report_validate_rejects_source_label_mismatch() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["intake", "--out", out.to_str().unwrap(), "--no-open"])
        .assert()
        .success();

    let report_path = first_run_dir(&out).join("intake.report.json");
    let mut report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
    report["source_freshness"][0]["source_label"] = serde_json::json!("GitLab");
    std::fs::write(
        &report_path,
        format!("{}\n", serde_json::to_string_pretty(&report).unwrap()),
    )
    .unwrap();

    shiplog_cmd()
        .args([
            "report",
            "validate",
            "--path",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "intake report source_freshness source_label",
        ))
        .stderr(predicate::str::contains("does not match source_key"));
}

#[test]
fn report_validate_rejects_unknown_fixup_kind() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["intake", "--out", out.to_str().unwrap(), "--no-open"])
        .assert()
        .success();

    let report_path = first_run_dir(&out).join("intake.report.json");
    let mut report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
    report["top_fixups"] = serde_json::json!([
        {
            "id": "fixup_mystery",
            "kind": "mystery",
            "title": "Mystery fixup",
            "detail": null,
            "command": "shiplog review --latest"
        }
    ]);
    std::fs::write(
        &report_path,
        format!("{}\n", serde_json::to_string_pretty(&report).unwrap()),
    )
    .unwrap();

    shiplog_cmd()
        .args([
            "report",
            "validate",
            "--path",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "top_fixups kind \"mystery\" is not supported",
        ));
}

#[test]
fn report_validate_rejects_invalid_fixup_id() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["intake", "--out", out.to_str().unwrap(), "--no-open"])
        .assert()
        .success();

    let report_path = first_run_dir(&out).join("intake.report.json");
    let mut report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
    report["top_fixups"] = serde_json::json!([
        {
            "id": "Fixup Bad Id",
            "kind": "manual_context",
            "title": "Bad fixup id",
            "detail": null,
            "command": "shiplog review --latest"
        }
    ]);
    std::fs::write(
        &report_path,
        format!("{}\n", serde_json::to_string_pretty(&report).unwrap()),
    )
    .unwrap();

    shiplog_cmd()
        .args([
            "report",
            "validate",
            "--path",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "top_fixups id must match fixup_[a-z0-9_]+",
        ));
}

#[test]
fn report_validate_rejects_unknown_action_kind() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["intake", "--out", out.to_str().unwrap(), "--no-open"])
        .assert()
        .success();

    let report_path = first_run_dir(&out).join("intake.report.json");
    let mut report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
    report["actions"] = serde_json::json!([
        {
            "id": "action_mystery",
            "kind": "mystery",
            "label": "Mystery action",
            "command": "shiplog review --latest",
            "writes": false,
            "risk": "low"
        }
    ]);
    std::fs::write(
        &report_path,
        format!("{}\n", serde_json::to_string_pretty(&report).unwrap()),
    )
    .unwrap();

    shiplog_cmd()
        .args([
            "report",
            "validate",
            "--path",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "actions kind \"mystery\" is not supported",
        ));
}

#[test]
fn report_validate_rejects_invalid_action_id() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["intake", "--out", out.to_str().unwrap(), "--no-open"])
        .assert()
        .success();

    let report_path = first_run_dir(&out).join("intake.report.json");
    let mut report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
    report["actions"] = serde_json::json!([
        {
            "id": "Action Bad",
            "kind": "fixup",
            "label": "Bad action",
            "command": "shiplog review --latest",
            "writes": false,
            "risk": "low"
        }
    ]);
    std::fs::write(
        &report_path,
        format!("{}\n", serde_json::to_string_pretty(&report).unwrap()),
    )
    .unwrap();

    shiplog_cmd()
        .args([
            "report",
            "validate",
            "--path",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "actions id must match action_[a-z0-9_]+",
        ));
}

#[test]
fn report_validate_rejects_unknown_repair_item_kind() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["intake", "--out", out.to_str().unwrap(), "--no-open"])
        .assert()
        .success();

    let report_path = first_run_dir(&out).join("intake.report.json");
    let mut report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
    report["repair_items"] = serde_json::json!([
        {
            "repair_id": "repair_001_mystery",
            "repair_key": "mystery",
            "kind": "mystery",
            "reason": "Mystery repair",
            "action": {
                "kind": "no_safe_action"
            },
            "clears_when": "the mystery clears",
            "receipt_refs": [
                {
                    "field": "evidence_debt"
                }
            ]
        }
    ]);
    std::fs::write(
        &report_path,
        format!("{}\n", serde_json::to_string_pretty(&report).unwrap()),
    )
    .unwrap();

    shiplog_cmd()
        .args([
            "report",
            "validate",
            "--path",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "repair_items kind \"mystery\" is not supported",
        ));
}

#[test]
fn report_validate_rejects_invalid_repair_item_id() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["intake", "--out", out.to_str().unwrap(), "--no-open"])
        .assert()
        .success();

    let report_path = first_run_dir(&out).join("intake.report.json");
    let mut report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
    report["repair_items"] = serde_json::json!([
        {
            "repair_id": "Repair Bad",
            "repair_key": "manual:manual_evidence_missing",
            "source_key": "manual",
            "source_label": "Manual",
            "kind": "manual_evidence_missing",
            "reason": "Missing manual evidence",
            "action": {
                "kind": "journal_add",
                "command": "shiplog journal add --title \"Manual evidence\""
            },
            "clears_when": "manual source contributes evidence",
            "receipt_refs": [
                {
                    "field": "journal_suggestions",
                    "source_key": "manual"
                }
            ]
        }
    ]);
    std::fs::write(
        &report_path,
        format!("{}\n", serde_json::to_string_pretty(&report).unwrap()),
    )
    .unwrap();

    shiplog_cmd()
        .args([
            "report",
            "validate",
            "--path",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "repair_items repair_id must match repair_[a-z0-9_]+",
        ));
}

#[test]
fn report_validate_rejects_unknown_repair_action_kind() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["intake", "--out", out.to_str().unwrap(), "--no-open"])
        .assert()
        .success();

    let report_path = first_run_dir(&out).join("intake.report.json");
    let mut report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
    report["repair_items"] = serde_json::json!([
        {
            "repair_id": "repair_001_bad_action",
            "repair_key": "manual:manual_evidence_missing",
            "source_key": "manual",
            "source_label": "Manual",
            "kind": "manual_evidence_missing",
            "reason": "Missing manual evidence",
            "action": {
                "kind": "mutate_provider"
            },
            "clears_when": "manual source contributes evidence",
            "receipt_refs": [
                {
                    "field": "journal_suggestions",
                    "source_key": "manual"
                }
            ]
        }
    ]);
    std::fs::write(
        &report_path,
        format!("{}\n", serde_json::to_string_pretty(&report).unwrap()),
    )
    .unwrap();

    shiplog_cmd()
        .args([
            "report",
            "validate",
            "--path",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "repair_items action.kind \"mutate_provider\" is not supported",
        ));
}

#[test]
fn report_validate_rejects_unknown_repair_kind() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["intake", "--out", out.to_str().unwrap(), "--no-open"])
        .assert()
        .success();

    let report_path = first_run_dir(&out).join("intake.report.json");
    let mut report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
    report["repair_sources"] = serde_json::json!([
        {
            "source": "Jira",
            "kind": "mystery",
            "reason": "missing JIRA_TOKEN",
            "commands": ["export JIRA_TOKEN=..."]
        }
    ]);
    std::fs::write(
        &report_path,
        format!("{}\n", serde_json::to_string_pretty(&report).unwrap()),
    )
    .unwrap();

    shiplog_cmd()
        .args([
            "report",
            "validate",
            "--path",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "repair_sources kind \"mystery\" is not supported",
        ));
}

#[test]
fn report_validate_rejects_missing_artifact() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["intake", "--out", out.to_str().unwrap(), "--no-open"])
        .assert()
        .success();

    let run_dir = first_run_dir(&out);
    std::fs::remove_file(run_dir.join("packet.md")).unwrap();

    shiplog_cmd()
        .args([
            "report",
            "validate",
            "--out",
            out.to_str().unwrap(),
            "--latest",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("artifact missing for packet"));
}

#[test]
fn report_summarize_prints_operator_view_without_writing() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["intake", "--out", out.to_str().unwrap(), "--no-open"])
        .assert()
        .success();

    let run_dir = first_run_dir(&out);
    let report_path = run_dir.join("intake.report.json");
    let mut report_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
    let actions = report_json
        .get_mut("actions")
        .and_then(serde_json::Value::as_array_mut)
        .expect("intake report should include machine actions");
    actions.push(serde_json::json!({
        "id": "action_share_manager_summary_regression",
        "kind": "share_manager",
        "label": "Render manager share",
        "command": format!("shiplog share manager --out \"{}\" --run run_summary", out.display()),
        "writes": true,
        "risk": "medium"
    }));
    actions.push(serde_json::json!({
        "id": "action_share_public_summary_regression",
        "kind": "share_public",
        "label": "Render public share",
        "command": format!("shiplog share public --out \"{}\" --run run_summary", out.display()),
        "writes": true,
        "risk": "high"
    }));
    std::fs::write(
        &report_path,
        serde_json::to_string_pretty(&report_json).unwrap(),
    )
    .unwrap();
    let report_modified = std::fs::metadata(&report_path).unwrap().modified().unwrap();

    let assert = shiplog_cmd()
        .args([
            "report",
            "summarize",
            "--out",
            out.to_str().unwrap(),
            "--latest",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Report summary:"))
        .stdout(predicate::str::contains("Intake status:"))
        .stdout(predicate::str::contains("Packet readiness:"))
        .stdout(predicate::str::contains("Sources:"))
        .stdout(predicate::str::contains("Evidence debt:"))
        .stdout(predicate::str::contains("Top repairs:"))
        .stdout(predicate::str::contains("missing_token"))
        .stdout(predicate::str::contains("Top fixups:"))
        .stdout(predicate::str::contains("Share explain next:"))
        .stdout(predicate::str::contains("shiplog share explain manager"))
        .stdout(predicate::str::contains("shiplog share explain public"))
        .stdout(predicate::str::contains("shiplog share manager").not())
        .stdout(predicate::str::contains("Packet:"))
        .stdout(predicate::str::contains("Intake report:"));
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let window_line = stdout
        .lines()
        .find(|line| line.starts_with("Window: "))
        .expect("report summarize should print a Window line");
    assert!(
        !window_line.contains(") ("),
        "window line should not duplicate the date range: {window_line}"
    );

    assert_eq!(
        report_modified,
        std::fs::metadata(&report_path).unwrap().modified().unwrap(),
        "report summarize should be read-only"
    );
}

#[test]
fn report_summarize_accepts_direct_path_and_rejects_invalid_reports() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["intake", "--out", out.to_str().unwrap(), "--no-open"])
        .assert()
        .success();

    let report_path = first_run_dir(&out).join("intake.report.json");

    shiplog_cmd()
        .args([
            "report",
            "summarize",
            "--path",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Report summary:"))
        .stdout(predicate::str::contains("intake.report.json"));

    let mut report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
    report["schema_version"] = serde_json::json!(2);
    std::fs::write(
        &report_path,
        format!("{}\n", serde_json::to_string_pretty(&report).unwrap()),
    )
    .unwrap();

    shiplog_cmd()
        .args([
            "report",
            "summarize",
            "--path",
            report_path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "unsupported intake report schema_version 2",
        ));
}

#[test]
fn report_export_agent_pack_writes_derived_control_surface_without_mutating_report() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["intake", "--out", out.to_str().unwrap(), "--no-open"])
        .assert()
        .success();

    let run_dir = first_run_dir(&out);
    let report_path = run_dir.join("intake.report.json");
    let report_text = std::fs::read_to_string(&report_path).unwrap();
    let report_json: serde_json::Value = serde_json::from_str(&report_text).unwrap();
    let report_modified = std::fs::metadata(&report_path).unwrap().modified().unwrap();
    let pack_path = tmp.path().join("agent-pack.json");

    shiplog_cmd()
        .args([
            "report",
            "export-agent-pack",
            "--out",
            out.to_str().unwrap(),
            "--latest",
            "--output",
            pack_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Agent pack:"))
        .stdout(predicate::str::contains("agent-pack.json"));

    assert_eq!(
        report_modified,
        std::fs::metadata(&report_path).unwrap().modified().unwrap(),
        "agent pack export should not rewrite intake.report.json"
    );

    let pack_text = std::fs::read_to_string(&pack_path).unwrap();
    let pack_json: serde_json::Value = serde_json::from_str(&pack_text).unwrap();
    assert_eq!(pack_json["schema_version"], 1);
    assert_eq!(pack_json["source_report"]["schema_version"], 1);
    assert!(
        pack_json["source_report"]["path"]
            .as_str()
            .unwrap()
            .contains("intake.report.json")
    );
    assert_eq!(pack_json["run"]["run_id"], report_json["run_id"]);
    assert_eq!(pack_json["run"]["readiness"], report_json["readiness"]);
    assert_eq!(
        pack_json["summary"]["skipped_source_count"].as_u64(),
        Some(report_json["skipped_sources"].as_array().unwrap().len() as u64)
    );
    assert_eq!(
        pack_json["summary"]["repair_count"].as_u64(),
        Some(report_json["repair_items"].as_array().unwrap().len() as u64)
    );
    assert!(pack_json["gaps"]["needs_attention"].is_array());
    assert!(pack_json["gaps"]["skipped_sources"].is_array());
    assert!(pack_json["gaps"]["evidence_debt"].is_array());
    assert!(pack_json["repairs"].is_array());
    assert_eq!(
        pack_json["repairs"], report_json["repair_items"],
        "agent pack repairs should mirror intake report repair_items when present"
    );
    assert!(pack_json["fixups"].is_array());
    assert!(pack_json["actions"].is_array());
    assert!(pack_json["share_status"]["commands"].is_array());
    assert!(pack_json["artifacts"].is_array());
    assert!(
        !pack_text.contains("super-secret"),
        "agent pack should not contain known secret sentinel values"
    );
}

#[test]
fn report_export_agent_pack_accepts_direct_path_and_stdout() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["intake", "--out", out.to_str().unwrap(), "--no-open"])
        .assert()
        .success();

    let report_path = first_run_dir(&out).join("intake.report.json");
    let output = shiplog_cmd()
        .args([
            "report",
            "export-agent-pack",
            "--path",
            report_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "export-agent-pack should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let pack_json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(pack_json["schema_version"], 1);
    assert_eq!(pack_json["source_report"]["schema_version"], 1);
    assert!(pack_json["summary"]["action_count"].is_number());
}

#[test]
fn open_intake_report_fails_clearly_when_report_is_missing() {
    let tmp = TempDir::new().unwrap();
    collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "open",
            "intake-report",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--print-path",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Intake report not found"))
        .stderr(predicate::str::contains("shiplog intake"));
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

#[test]
fn workstreams_receipts_lists_selected_workstream_receipts() {
    let tmp = TempDir::new().unwrap();
    collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "workstreams",
            "receipts",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--workstream",
            "acme/platform",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Receipts: acme/platform"))
        .stdout(predicate::str::contains("Count: 2"))
        .stdout(predicate::str::contains(
            "Schema hardening for audit exports",
        ))
        .stdout(predicate::str::contains("APPROVED"));
}

#[test]
fn workstreams_receipt_remove_promotes_suggested_to_curated_and_keeps_assignment() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "workstreams",
            "receipt",
            "remove",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--workstream",
            "acme/platform",
            "--event",
            "fixture_pr_acme_platform_13",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Removed receipt anchor fixture_pr_acme_platform_13 from acme/platform",
        ))
        .stdout(predicate::str::contains("Created curated workstreams.yaml"));

    assert!(run_dir.join("workstreams.suggested.yaml").exists());
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
            .any(|event_id| event_id.to_string() == "fixture_pr_acme_platform_13")
    );
    assert!(
        platform
            .receipts
            .iter()
            .all(|event_id| event_id.to_string() != "fixture_pr_acme_platform_13")
    );
}

#[test]
fn workstreams_receipt_add_promotes_suggested_to_curated_and_validates() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());
    std::fs::write(
        run_dir.join("workstreams.suggested.yaml"),
        r#"version: 1
generated_at: "2026-01-01T00:00:00Z"
workstreams:
  - id: "repo-acme-platform"
    title: "acme/platform"
    summary: null
    tags:
      - platform
    stats:
      pull_requests: 1
      reviews: 1
      manual_events: 0
    events:
      - "fixture_pr_acme_platform_13"
      - "fixture_review_acme_platform_77_1"
    receipts:
      - "fixture_review_acme_platform_77_1"
"#,
    )
    .unwrap();

    shiplog_cmd()
        .args([
            "workstreams",
            "receipt",
            "add",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--workstream",
            "acme/platform",
            "--event",
            "fixture_pr_acme_platform_13",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Added receipt anchor fixture_pr_acme_platform_13 to acme/platform",
        ))
        .stdout(predicate::str::contains("Created curated workstreams.yaml"));

    let curated = load_curated_workstreams(&run_dir);
    let platform = curated
        .workstreams
        .iter()
        .find(|workstream| workstream.title == "acme/platform")
        .expect("platform workstream should exist");
    assert!(
        platform
            .receipts
            .iter()
            .any(|event_id| event_id.to_string() == "fixture_pr_acme_platform_13")
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
fn workstreams_receipt_add_rejects_event_not_in_workstream_without_writing() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "workstreams",
            "receipt",
            "add",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--workstream",
            "acme/platform",
            "--event",
            "fixture_pr_acme_payments_42",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("is not assigned to workstream"));

    assert!(!run_dir.join("workstreams.yaml").exists());
}

#[test]
fn workstreams_receipt_add_rejects_duplicate_without_writing() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "workstreams",
            "receipt",
            "add",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--workstream",
            "acme/platform",
            "--event",
            "fixture_pr_acme_platform_13",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already a receipt anchor"));

    assert!(!run_dir.join("workstreams.yaml").exists());
}

#[test]
fn workstreams_receipt_remove_rejects_missing_receipt_without_writing() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "workstreams",
            "receipt",
            "remove",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--workstream",
            "acme/platform",
            "--event",
            "fixture_pr_acme_payments_42",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("is not a receipt anchor"));

    assert!(!run_dir.join("workstreams.yaml").exists());
}

#[test]
fn workstreams_create_promotes_suggested_to_curated() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "workstreams",
            "create",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--title",
            "Platform Reliability",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Created workstream: Platform Reliability",
        ))
        .stdout(predicate::str::contains("Created curated workstreams.yaml"));

    assert!(run_dir.join("workstreams.suggested.yaml").exists());
    let curated = load_curated_workstreams(&run_dir);
    let created = curated
        .workstreams
        .iter()
        .find(|workstream| workstream.title == "Platform Reliability")
        .expect("created workstream should exist");
    assert!(created.events.is_empty());
    assert!(created.receipts.is_empty());
    assert_eq!(created.stats.pull_requests, 0);
    assert_eq!(created.stats.reviews, 0);
}

#[test]
fn workstreams_create_rejects_blank_and_duplicate_without_writing() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "workstreams",
            "create",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--title",
            "   ",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("title cannot be blank"));
    assert!(!run_dir.join("workstreams.yaml").exists());

    shiplog_cmd()
        .args([
            "workstreams",
            "create",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--title",
            "acme/platform",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already matches"));
    assert!(!run_dir.join("workstreams.yaml").exists());
}

#[test]
fn workstreams_delete_empty_workstream_succeeds() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "workstreams",
            "create",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--title",
            "Temporary Bucket",
        ])
        .assert()
        .success();

    shiplog_cmd()
        .args([
            "workstreams",
            "delete",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--workstream",
            "Temporary Bucket",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Deleted workstream: Temporary Bucket",
        ));

    let curated = load_curated_workstreams(&run_dir);
    assert!(
        !curated
            .workstreams
            .iter()
            .any(|workstream| workstream.title == "Temporary Bucket")
    );
}

#[test]
fn workstreams_delete_non_empty_requires_move_or_force_without_writing() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "workstreams",
            "delete",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--workstream",
            "acme/platform",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--move-to"))
        .stderr(predicate::str::contains("--force"));

    assert!(!run_dir.join("workstreams.yaml").exists());
}

#[test]
fn workstreams_delete_non_empty_moves_events_and_recomputes_stats() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "workstreams",
            "create",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--title",
            "Misc",
        ])
        .assert()
        .success();

    shiplog_cmd()
        .args([
            "workstreams",
            "delete",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--workstream",
            "acme/platform",
            "--move-to",
            "Misc",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Deleted workstream: acme/platform",
        ))
        .stdout(predicate::str::contains("Moved 2 event(s)"));

    let curated = load_curated_workstreams(&run_dir);
    assert!(
        !curated
            .workstreams
            .iter()
            .any(|workstream| workstream.title == "acme/platform")
    );
    let misc = curated
        .workstreams
        .iter()
        .find(|workstream| workstream.title == "Misc")
        .expect("move target should exist");
    assert_eq!(misc.stats.pull_requests, 1);
    assert_eq!(misc.stats.reviews, 1);
    assert!(
        misc.events
            .iter()
            .any(|event_id| event_id.to_string() == "fixture_pr_acme_platform_13")
    );
    assert!(
        misc.events
            .iter()
            .any(|event_id| event_id.to_string() == "fixture_review_acme_platform_77_1")
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
fn workstreams_delete_self_move_fails_without_writing() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "workstreams",
            "delete",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--workstream",
            "acme/platform",
            "--move-to",
            "acme/platform",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("must be different"));

    assert!(!run_dir.join("workstreams.yaml").exists());
}

#[test]
fn workstreams_delete_missing_move_target_fails_without_writing() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "workstreams",
            "delete",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--workstream",
            "acme/platform",
            "--move-to",
            "Missing Bucket",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no workstream matched"));

    assert!(!run_dir.join("workstreams.yaml").exists());
}

#[test]
fn workstreams_delete_force_discards_assignments_and_validates() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "workstreams",
            "delete",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--workstream",
            "acme/payments",
            "--force",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Deleted workstream: acme/payments",
        ))
        .stdout(predicate::str::contains("Discarded 1 event assignment"));

    let curated = load_curated_workstreams(&run_dir);
    assert!(
        !curated
            .workstreams
            .iter()
            .any(|workstream| workstream.title == "acme/payments")
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
fn workstreams_split_creates_target_and_recomputes_stats() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "workstreams",
            "split",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--from",
            "acme/platform",
            "--to",
            "Audit Exports",
            "--matching",
            "schema|compliance",
            "--create",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Split 1 event(s) from acme/platform to Audit Exports",
        ))
        .stdout(predicate::str::contains("Created target workstream"))
        .stdout(predicate::str::contains("Created curated workstreams.yaml"));

    assert!(run_dir.join("workstreams.suggested.yaml").exists());
    let curated = load_curated_workstreams(&run_dir);
    let audit = curated
        .workstreams
        .iter()
        .find(|workstream| workstream.title == "Audit Exports")
        .expect("split target should be created");
    assert_eq!(audit.stats.pull_requests, 1);
    assert_eq!(audit.stats.reviews, 0);
    assert!(
        audit
            .events
            .iter()
            .any(|event_id| event_id.to_string() == "fixture_pr_acme_platform_13")
    );

    let platform = curated
        .workstreams
        .iter()
        .find(|workstream| workstream.title == "acme/platform")
        .expect("source workstream should remain");
    assert_eq!(platform.stats.pull_requests, 0);
    assert_eq!(platform.stats.reviews, 1);
    assert!(
        platform
            .events
            .iter()
            .all(|event_id| event_id.to_string() != "fixture_pr_acme_platform_13")
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
fn workstreams_split_matches_source_url() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "workstreams",
            "split",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--from",
            "acme/platform",
            "--to",
            "Review Receipts",
            "--matching",
            "reviews/1",
            "--create",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Split 1 event(s) from acme/platform to Review Receipts",
        ));

    let curated = load_curated_workstreams(&run_dir);
    let review_receipts = curated
        .workstreams
        .iter()
        .find(|workstream| workstream.title == "Review Receipts")
        .expect("source-url split target should be created");
    assert!(
        review_receipts
            .events
            .iter()
            .any(|event_id| event_id.to_string() == "fixture_review_acme_platform_77_1")
    );
}

#[test]
fn workstreams_split_missing_target_requires_create_without_writing_curated() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "workstreams",
            "split",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--from",
            "acme/platform",
            "--to",
            "Audit Exports",
            "--matching",
            "Schema",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("add --create to create it"));

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
    let run_dir = collect_json_into(tmp.path());

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
    let packet = std::fs::read_to_string(run_dir.join("packet.md")).unwrap();
    assert_packet_opens_with_coverage(&packet);
    assert_packet_uses_summary_appendix(&packet);
}

#[test]
fn collect_json_writes_coverage_first_packet() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    let packet = std::fs::read_to_string(run_dir.join("packet.md")).unwrap();
    assert_packet_opens_with_coverage(&packet);
    assert_packet_uses_summary_appendix(&packet);
}

#[test]
fn render_scaffold_mode_writes_prompt_focused_packet() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "render",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--mode",
            "scaffold",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Rendered"));

    let packet = std::fs::read_to_string(run_dir.join("packet.md")).unwrap();
    assert!(packet.contains("## Coverage and Limits"));
    assert!(packet.contains("**Suggested claim prompts**"));
    assert!(
        !packet.contains("\n## Receipts\n"),
        "scaffold mode should omit the full receipts section"
    );
    assert!(
        !packet.contains("\n## Appendix: All Receipts\n"),
        "scaffold mode should omit the full appendix"
    );
}

#[test]
fn render_receipts_mode_writes_audit_focused_packet() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "render",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--mode",
            "receipts",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Rendered"));

    let packet = std::fs::read_to_string(run_dir.join("packet.md")).unwrap();
    assert!(packet.contains("## Coverage and Limits"));
    assert!(packet.contains("\n## Receipts\n"));
    assert!(packet.contains("\n## Appendix: All Receipts\n"));
    assert!(
        !packet.contains("**Suggested claim prompts**"),
        "receipts mode should omit writing prompts"
    );
}

#[test]
fn render_receipt_limit_summary_appendix_preserves_canonical_files() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());
    let ledger_before = std::fs::read_to_string(run_dir.join("ledger.events.jsonl")).unwrap();
    let coverage_before = std::fs::read_to_string(run_dir.join("coverage.manifest.json")).unwrap();

    shiplog_cmd()
        .args([
            "render",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--mode",
            "packet",
            "--receipt-limit",
            "1",
            "--appendix",
            "summary",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Rendered"));

    let packet = std::fs::read_to_string(run_dir.join("packet.md")).unwrap();
    assert!(packet.contains("\n## Receipts\n"));
    assert!(packet.contains("... and"));
    assert!(packet.contains("## Appendix: Receipt Summary"));
    assert!(!packet.contains("## Appendix: All Receipts"));
    assert!(packet.contains("- Assigned events:"));
    assert!(packet.contains("- Curated receipt anchors:"));
    assert_eq!(
        ledger_before,
        std::fs::read_to_string(run_dir.join("ledger.events.jsonl")).unwrap()
    );
    assert_eq!(
        coverage_before,
        std::fs::read_to_string(run_dir.join("coverage.manifest.json")).unwrap()
    );
}

#[test]
fn render_receipt_limit_zero_appendix_none_keeps_omission_notice() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());
    let ledger_before = std::fs::read_to_string(run_dir.join("ledger.events.jsonl")).unwrap();
    let coverage_before = std::fs::read_to_string(run_dir.join("coverage.manifest.json")).unwrap();

    shiplog_cmd()
        .args([
            "render",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--mode",
            "packet",
            "--receipt-limit",
            "0",
            "--appendix",
            "none",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Rendered"));

    let packet = std::fs::read_to_string(run_dir.join("packet.md")).unwrap();
    assert!(packet.contains("\n## Receipts\n"));
    assert!(packet.contains("- (none)"));
    assert!(packet.contains("omitted by appendix settings"));
    assert!(!packet.contains("## Appendix:"));
    assert_eq!(
        ledger_before,
        std::fs::read_to_string(run_dir.join("ledger.events.jsonl")).unwrap()
    );
    assert_eq!(
        coverage_before,
        std::fs::read_to_string(run_dir.join("coverage.manifest.json")).unwrap()
    );
}

#[test]
fn render_receipts_mode_honors_receipt_controls() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());

    shiplog_cmd()
        .args([
            "render",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--mode",
            "receipts",
            "--receipt-limit",
            "1",
            "--appendix",
            "full",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Rendered"));

    let packet = std::fs::read_to_string(run_dir.join("packet.md")).unwrap();
    assert!(packet.contains("\n## Receipts\n"));
    assert!(packet.contains("... and"));
    assert!(packet.contains("## Appendix: All Receipts"));
    assert!(!packet.contains("**Suggested claim prompts**"));
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
        .stderr(
            predicate::str::contains("public profile requires --redact-key or SHIPLOG_REDACT_KEY")
                .and(predicate::str::contains("Try:"))
                .and(predicate::str::contains(redaction_key_env_command(
                    "SHIPLOG_REDACT_KEY",
                )))
                .and(predicate::str::contains(
                    "rerun this command with --bundle-profile public",
                )),
        );
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

    let public_packet_path = tmp.path().join("run_fixture/profiles/public/packet.md");
    assert!(
        public_packet_path.exists(),
        "public packet should be written when a redaction key is provided"
    );

    let public_packet = std::fs::read_to_string(public_packet_path).unwrap();
    assert!(
        public_packet.contains("omitted by appendix settings"),
        "public profile should default to a low-density packet with explicit omission notes"
    );
    assert!(
        !public_packet.contains("## Appendix:"),
        "public profile should omit the receipt appendix by default"
    );
}

#[test]
fn share_manager_without_key_fails_closed() {
    let tmp = TempDir::new().unwrap();
    collect_json_into(tmp.path());

    let mut cmd = shiplog_cmd();
    cmd.env_remove("SHIPLOG_REDACT_KEY")
        .args([
            "share",
            "manager",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
        ])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("manager share requires --redact-key or SHIPLOG_REDACT_KEY")
                .and(predicate::str::contains("Try:"))
                .and(predicate::str::contains(redaction_key_env_command(
                    "SHIPLOG_REDACT_KEY",
                )))
                .and(predicate::str::contains("shiplog share manager --latest")),
        );

    assert!(
        !tmp.path()
            .join("run_fixture/profiles/manager/packet.md")
            .exists(),
        "manager share packet should not be written without a redaction key"
    );
    assert!(
        !tmp.path()
            .join("run_fixture/profiles/manager/share.manifest.json")
            .exists(),
        "manager share manifest should not be written without a redaction key"
    );
}

#[test]
fn share_explain_manager_without_key_reports_block_without_writing() {
    let tmp = TempDir::new().unwrap();
    collect_json_into(tmp.path());

    let assert = shiplog_cmd()
        .env_remove("SHIPLOG_REDACT_KEY")
        .args([
            "share",
            "explain",
            "manager",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert!(stdout.contains("Manager profile:"));
    assert!(stdout.contains("Status: blocked"));
    assert!(stdout.contains("Redaction key: missing"));
    assert!(stdout.contains("Included:"));
    assert!(stdout.contains("- packet readiness and claim candidates"));
    assert!(stdout.contains("source-backed evidence counts: GitHub"));
    assert!(stdout.contains("Removed:"));
    assert!(stdout.contains("- opaque provider identifiers"));
    assert!(stdout.contains("Blocked:\n- missing SHIPLOG_REDACT_KEY"));
    assert!(stdout.contains("Needs review:\n- None"));
    assert!(stdout.contains("Profile packet: not written yet"));
    assert!(stdout.contains("Share manifest: not written yet"));
    assert!(stdout.contains("Render when ready:"));
    assert!(stdout.contains("shiplog share manager --out"));
    assert!(!stdout.contains("\nNext:\n"));
    assert!(
        !tmp.path()
            .join("run_fixture/profiles/manager/packet.md")
            .exists(),
        "manager explain should not render the share packet"
    );
    assert!(
        !tmp.path()
            .join("run_fixture/profiles/manager/share.manifest.json")
            .exists(),
        "manager explain should not write the share manifest"
    );
}

#[test]
fn share_explain_manager_surfaces_packet_evidence_debt_without_writing() -> CliTestResult {
    let Some(repo) = create_local_git_repo() else {
        return Ok(());
    };
    let tmp = TempDir::new()?;
    let out = tmp.path().join("out");
    let repo_path = repo.path().display().to_string().replace('\\', "/");

    std::fs::write(
        tmp.path().join("shiplog.toml"),
        format!(
            r#"[defaults]
window = "year:2025"

[user]
label = "shiplog test"

[sources.git]
enabled = true
repo = "{repo_path}"
include_merges = false

[sources.manual]
enabled = true
events = "./manual_events.yaml"
user = "shiplog test"
"#
        ),
    )?;
    std::fs::write(
        tmp.path().join("manual_events.yaml"),
        "version: 1\ngenerated_at: 2026-01-01T00:00:00Z\nevents: []\n",
    )?;

    shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args([
            "intake",
            "--out",
            out.to_str().unwrap(),
            "--year",
            "2025",
            "--no-open",
            "--explain",
        ])
        .assert()
        .success();

    let (report_path, mut report) = load_first_intake_report(&out);
    let evidence_debt_summary = "Manual outcome context is missing.";
    report["evidence_debt"] = serde_json::json!([
        {
            "severity": "info",
            "kind": "manual-context",
            "summary": evidence_debt_summary,
            "detail": "Injected fixture evidence debt",
            "next_step": "shiplog journal add --date 2025-01-15 --title \"Outcome note\""
        }
    ]);
    report["packet_quality"]["packet_readiness"]["summary"] =
        serde_json::json!("Ready with caveats.");
    std::fs::write(
        &report_path,
        format!("{}\n", serde_json::to_string_pretty(&report)?),
    )?;

    let readiness = report["packet_quality"]["packet_readiness"]["summary"]
        .as_str()
        .expect("packet readiness summary should be present")
        .trim_end_matches('.');

    let before = file_tree_manifest(tmp.path());
    let assert = shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("SHIPLOG_REDACT_KEY")
        .args([
            "share",
            "explain",
            "manager",
            "--out",
            out.to_str().unwrap(),
            "--latest",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let after = file_tree_manifest(tmp.path());

    assert!(
        stdout.contains(&format!("Packet readiness: {readiness}")),
        "share explain should surface packet readiness caveats. stdout:\n{stdout}"
    );
    assert!(
        stdout.contains(&format!("Evidence debt: {evidence_debt_summary}")),
        "share explain should surface report evidence debt before sharing. stdout:\n{stdout}"
    );
    assert!(
        !stdout.contains("Needs review:\n- None"),
        "share explain should not claim there is nothing to review when evidence debt exists. stdout:\n{stdout}"
    );
    assert_eq!(
        before, after,
        "share explain should remain read-only while surfacing evidence debt"
    );

    Ok(())
}

#[test]
fn share_explain_manager_surfaces_open_source_repairs_without_writing() -> CliTestResult {
    let tmp = TempDir::new()?;
    let out = tmp.path().join("out");
    let out_arg = out.to_string_lossy().to_string();

    run_intake_without_provider_tokens(tmp.path(), &out);
    let (_, first_report) = load_first_intake_report(&out);
    let repair_id = first_repair_id_with_action(&first_report, "journal_add");
    shiplog_cmd()
        .current_dir(tmp.path())
        .args([
            "journal",
            "add",
            "--from-repair",
            repair_id.as_str(),
            "--out",
            out_arg.as_str(),
            "--latest",
        ])
        .assert()
        .success();
    run_intake_without_provider_tokens(tmp.path(), &out);

    let before = file_tree_manifest(tmp.path());
    let assert = shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("SHIPLOG_REDACT_KEY")
        .args([
            "share",
            "explain",
            "manager",
            "--out",
            out_arg.as_str(),
            "--latest",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let after = file_tree_manifest(tmp.path());

    assert!(
        stdout.contains("Open source repair:")
            && stdout.contains("GitHub needs repair: GITHUB_TOKEN not found"),
        "share explain should name source repairs that remain open after journal-only repair. stdout:\n{stdout}"
    );
    assert!(
        !stdout.contains("Needs review:\n- None"),
        "share explain should not claim nothing needs review while source repairs remain open. stdout:\n{stdout}"
    );
    assert_eq!(
        before, after,
        "share explain should remain read-only while surfacing open source repairs"
    );

    Ok(())
}

#[test]
fn share_explain_uses_report_receipts_not_packet_markdown() -> CliTestResult {
    let tmp = TempDir::new()?;
    let out = tmp.path().join("out");
    run_intake_without_provider_tokens(tmp.path(), &out);
    let run_dir = first_run_dir(&out);
    let (report_path, mut report) = load_first_intake_report(&out);
    let report_only_debt = "Report-only share posture debt.";
    let markdown_only_bait = "Markdown-only share posture bait.";
    report["evidence_debt"] = serde_json::json!([
        {
            "severity": "info",
            "kind": "manual-context",
            "summary": report_only_debt,
            "detail": "Injected report-only receipt",
            "next_step": "shiplog journal add --date 2025-01-15 --title \"Outcome note\""
        }
    ]);
    std::fs::write(
        &report_path,
        format!("{}\n", serde_json::to_string_pretty(&report)?),
    )?;
    std::fs::write(
        run_dir.join("packet.md"),
        format!("# Packet Readiness\n\nReady.\n\n{markdown_only_bait}\n"),
    )?;

    let before = file_tree_manifest(tmp.path());
    let assert = shiplog_cmd()
        .env_remove("SHIPLOG_REDACT_KEY")
        .args([
            "share",
            "explain",
            "manager",
            "--out",
            out.to_str().unwrap(),
            "--latest",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let after = file_tree_manifest(tmp.path());

    assert!(
        stdout.contains(&format!("Evidence debt: {report_only_debt}")),
        "share explain should use intake.report.json evidence debt. stdout:\n{stdout}"
    );
    assert!(
        !stdout.contains(markdown_only_bait),
        "share explain should not scrape packet.md for share posture. stdout:\n{stdout}"
    );
    assert_eq!(
        before, after,
        "share explain should stay read-only while reading report receipts"
    );

    Ok(())
}

#[test]
fn share_explain_public_without_key_reports_public_posture_without_writing() {
    let tmp = TempDir::new().unwrap();
    collect_json_into(tmp.path());

    let assert = shiplog_cmd()
        .env_remove("SHIPLOG_REDACT_KEY")
        .args([
            "share",
            "explain",
            "public",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert!(stdout.contains("Public profile:"));
    assert!(stdout.contains("Status: blocked"));
    assert!(stdout.contains("Included:"));
    assert!(stdout.contains("public-safe summaries with the lowest default receipt density"));
    assert!(stdout.contains("Removed:"));
    assert!(stdout.contains("raw private URLs and original names where strict redaction applies"));
    assert!(stdout.contains("Blocked:\n- missing SHIPLOG_REDACT_KEY"));
    assert!(
        stdout.contains(
            "Public profile should be reviewed after rendering; strict scan is a guardrail"
        )
    );
    assert!(stdout.contains("Public profile uses the strictest redaction profile."));
    assert!(stdout.contains("Render when ready:"));
    assert!(stdout.contains("shiplog share public --out"));
    assert!(!stdout.contains("\nNext:\n"));
    assert!(
        !tmp.path()
            .join("run_fixture/profiles/public/packet.md")
            .exists(),
        "public explain should not render the share packet"
    );
    assert!(
        !tmp.path()
            .join("run_fixture/profiles/public/share.manifest.json")
            .exists(),
        "public explain should not write the share manifest"
    );
}

#[test]
fn share_readiness_missing_key_is_consistent_across_doctor_and_share_commands() -> CliTestResult {
    let tmp = TempDir::new()?;
    collect_json_into(tmp.path());
    std::fs::write(
        tmp.path().join("shiplog.toml"),
        r#"[shiplog]
config_version = 1

[defaults]
out = "./out"
window = "last-6-months"
profile = "manager"

[sources.json]
enabled = true
events = "./run_fixture/ledger.events.jsonl"
coverage = "./run_fixture/coverage.manifest.json"
"#,
    )?;

    let before = file_tree_manifest(tmp.path());

    let doctor = shiplog_cmd()
        .current_dir(tmp.path())
        .env_remove("SHIPLOG_REDACT_KEY")
        .args(["doctor", "--setup"])
        .assert()
        .failure();
    let doctor_stdout = String::from_utf8(doctor.get_output().stdout.clone())?;
    assert!(
        doctor_stdout.contains("Manager share")
            && doctor_stdout.contains("Public share")
            && doctor_stdout.contains("SHIPLOG_REDACT_KEY not set"),
        "doctor --setup should report the shared redaction setup blocker. stdout:\n{doctor_stdout}"
    );

    let manager_explain = shiplog_cmd()
        .env_remove("SHIPLOG_REDACT_KEY")
        .args([
            "share",
            "explain",
            "manager",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
        ])
        .assert()
        .success();
    let manager_explain_stdout = String::from_utf8(manager_explain.get_output().stdout.clone())?;
    assert!(
        manager_explain_stdout.contains("Manager profile:")
            && manager_explain_stdout.contains("Status: blocked")
            && manager_explain_stdout.contains("Redaction key: missing")
            && manager_explain_stdout.contains("Blocked:\n- missing SHIPLOG_REDACT_KEY"),
        "share explain manager should report the same missing-key blocker. stdout:\n{manager_explain_stdout}"
    );

    let public_explain = shiplog_cmd()
        .env_remove("SHIPLOG_REDACT_KEY")
        .args([
            "share",
            "explain",
            "public",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
        ])
        .assert()
        .success();
    let public_explain_stdout = String::from_utf8(public_explain.get_output().stdout.clone())?;
    assert!(
        public_explain_stdout.contains("Public profile:")
            && public_explain_stdout.contains("Status: blocked")
            && public_explain_stdout.contains("Blocked:\n- missing SHIPLOG_REDACT_KEY")
            && public_explain_stdout.contains("Public profile should be reviewed after rendering"),
        "share explain public should report the same missing-key blocker and public review caveat. stdout:\n{public_explain_stdout}"
    );

    for profile in ["manager", "public"] {
        let share_stderr = String::from_utf8(
            shiplog_cmd()
                .env_remove("SHIPLOG_REDACT_KEY")
                .args([
                    "share",
                    profile,
                    "--out",
                    tmp.path().to_str().unwrap(),
                    "--run",
                    "run_fixture",
                ])
                .assert()
                .failure()
                .get_output()
                .stderr
                .clone(),
        )?;
        assert!(
            share_stderr.contains(&format!(
                "{profile} share requires --redact-key or SHIPLOG_REDACT_KEY"
            )),
            "share {profile} should fail closed on the same missing key. stderr:\n{share_stderr}"
        );

        let verify_stderr = String::from_utf8(
            shiplog_cmd()
                .env_remove("SHIPLOG_REDACT_KEY")
                .args([
                    "share",
                    "verify",
                    profile,
                    "--out",
                    tmp.path().to_str().unwrap(),
                    "--run",
                    "run_fixture",
                ])
                .assert()
                .failure()
                .get_output()
                .stderr
                .clone(),
        )?;
        assert!(
            verify_stderr.contains(&format!(
                "{profile} share requires --redact-key or SHIPLOG_REDACT_KEY"
            )),
            "share verify {profile} should fail closed on the same missing key. stderr:\n{verify_stderr}"
        );
    }

    let after = file_tree_manifest(tmp.path());
    assert_eq!(
        before, after,
        "doctor, share explain, share verify, and failed share rendering should not write artifacts while redaction setup is blocked"
    );
    assert!(
        !tmp.path().join("run_fixture/profiles/manager").exists()
            && !tmp.path().join("run_fixture/profiles/public").exists(),
        "blocked share readiness checks should not create profile directories"
    );

    Ok(())
}

#[test]
fn share_explain_public_surfaces_packet_debt_without_writing() -> CliTestResult {
    let tmp = TempDir::new()?;
    let out = tmp.path().join("out");
    run_intake_without_provider_tokens(tmp.path(), &out);
    let (report_path, mut report) = load_first_intake_report(&out);
    let evidence_debt_summary = "Public packet needs human outcome context.";
    report["evidence_debt"] = serde_json::json!([
        {
            "severity": "info",
            "kind": "manual-context",
            "summary": evidence_debt_summary,
            "detail": "Injected fixture public-share debt",
            "next_step": "shiplog journal add --date 2025-01-15 --title \"Outcome note\""
        }
    ]);
    report["packet_quality"]["packet_readiness"]["summary"] =
        serde_json::json!("Ready with caveats.");
    std::fs::write(
        &report_path,
        format!("{}\n", serde_json::to_string_pretty(&report)?),
    )?;

    let before = file_tree_manifest(tmp.path());
    let assert = shiplog_cmd()
        .env_remove("SHIPLOG_REDACT_KEY")
        .args([
            "share",
            "explain",
            "public",
            "--out",
            out.to_str().unwrap(),
            "--latest",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let after = file_tree_manifest(tmp.path());

    assert!(
        stdout.contains("Packet readiness: Ready with caveats"),
        "public share explain should surface packet readiness caveats. stdout:\n{stdout}"
    );
    assert!(
        stdout.contains(&format!("Evidence debt: {evidence_debt_summary}")),
        "public share explain should surface packet evidence debt. stdout:\n{stdout}"
    );
    assert!(
        stdout.contains(
            "Public profile should be reviewed after rendering; strict scan is a guardrail"
        ),
        "public share explain should keep strict-review caveats visible. stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("shiplog share verify public")
            && stdout.contains("--strict")
            && stdout.contains("Render when ready:"),
        "public share explain should route rendering through strict verification. stdout:\n{stdout}"
    );
    assert_eq!(
        before, after,
        "public share explain should remain read-only while surfacing packet debt"
    );

    Ok(())
}

#[test]
fn share_explain_legacy_report_without_packet_quality_prompts_rerun_without_writing()
-> CliTestResult {
    let tmp = TempDir::new()?;
    let out = tmp.path().join("out");
    run_intake_without_provider_tokens(tmp.path(), &out);
    let (report_path, _) = load_first_intake_report(&out);
    remove_packet_quality_from_report(&report_path)?;

    let before = file_tree_manifest(tmp.path());
    for profile in ["manager", "public"] {
        let assert = shiplog_cmd()
            .env_remove("SHIPLOG_REDACT_KEY")
            .args([
                "share",
                "explain",
                profile,
                "--out",
                out.to_str().unwrap(),
                "--latest",
            ])
            .assert()
            .success();
        let stdout = String::from_utf8(assert.get_output().stdout.clone())?;

        assert!(
            stdout.contains("Packet quality unavailable: rerun intake for review-ready signals."),
            "share explain {profile} should prompt rerun for legacy reports. stdout:\n{stdout}"
        );
        assert!(
            !stdout.contains("Packet readiness: Ready")
                && !stdout.contains("Packet readiness: Ready for review"),
            "share explain {profile} should not invent readiness for legacy reports. stdout:\n{stdout}"
        );
    }
    let after = file_tree_manifest(tmp.path());
    assert_eq!(
        before, after,
        "share explain should not write while explaining legacy report quality gaps"
    );

    Ok(())
}

#[test]
fn share_explain_legacy_packet_quality_without_candidates_or_share_posture_prompts_rerun_without_writing()
-> CliTestResult {
    let tmp = TempDir::new()?;
    let out = tmp.path().join("out");
    run_intake_without_provider_tokens(tmp.path(), &out);
    let (report_path, _) = load_first_intake_report(&out);
    remove_packet_quality_fields_from_report(&report_path, &["claim_candidates", "share_posture"])?;

    let before = file_tree_manifest(tmp.path());
    let assert = shiplog_cmd()
        .env_remove("SHIPLOG_REDACT_KEY")
        .args([
            "share",
            "explain",
            "manager",
            "--out",
            out.to_str().unwrap(),
            "--latest",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let after = file_tree_manifest(tmp.path());

    assert!(
        stdout.contains(
            "Packet quality incomplete: rerun intake for claim candidates, share posture signals."
        ),
        "share explain should prompt rerun for partial legacy packet_quality. stdout:\n{stdout}"
    );
    assert_eq!(
        before, after,
        "share explain should not write while explaining partial legacy packet_quality"
    );

    Ok(())
}

#[test]
fn share_explain_latest_profiles_are_read_only() {
    let tmp = TempDir::new().unwrap();
    collect_json_into(tmp.path());
    let before = file_tree_manifest(tmp.path());

    for profile in ["manager", "public"] {
        let assert = shiplog_cmd()
            .env_remove("SHIPLOG_REDACT_KEY")
            .args([
                "share",
                "explain",
                profile,
                "--out",
                tmp.path().to_str().unwrap(),
                "--latest",
            ])
            .assert()
            .success();
        let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
        assert!(
            stdout.contains("Profile packet: not written yet")
                && stdout.contains("Share manifest: not written yet")
                && stdout.contains(&redaction_key_env_command("SHIPLOG_REDACT_KEY")),
            "share explain {profile} should describe existing share artifacts without rendering. stdout:\n{stdout}"
        );
    }

    let after = file_tree_manifest(tmp.path());
    assert_eq!(
        before, after,
        "share explain --latest should not write or rewrite files"
    );
}

#[test]
fn share_verify_manager_without_key_fails_closed_without_writing() {
    let tmp = TempDir::new().unwrap();
    collect_json_into(tmp.path());

    shiplog_cmd()
        .env_remove("SHIPLOG_REDACT_KEY")
        .args([
            "share",
            "verify",
            "manager",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
        ])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("manager share requires --redact-key or SHIPLOG_REDACT_KEY")
                .and(predicate::str::contains("shiplog share manager --latest")),
        );

    assert!(
        !tmp.path()
            .join("run_fixture/profiles/manager/packet.md")
            .exists(),
        "manager verify should not write a share packet without a redaction key"
    );
}

#[test]
fn share_verify_manager_uses_env_key_without_writing_or_printing_secret() {
    let tmp = TempDir::new().unwrap();
    collect_json_into(tmp.path());

    let assert = shiplog_cmd()
        .env("SHIPLOG_REDACT_KEY", "stable-env-key")
        .args([
            "share",
            "verify",
            "manager",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert!(stdout.contains("Share verify: manager"));
    assert!(stdout.contains("Good:"));
    assert!(stdout.contains("- Redaction key found"));
    assert!(stdout.contains("Needs attention:\n- None"));
    assert!(stdout.contains("- Packet: not written yet; share can render it."));
    assert!(stdout.contains("Workstreams:"));
    assert!(stdout.contains("Coverage:"));
    assert!(stdout.contains("- Status: Complete"));
    assert!(stdout.contains("- Gaps: 0"));
    assert!(stdout.contains("Skipped sources:\n- None"));
    assert!(stdout.contains("Result: ready to render manager share output."));
    assert!(stdout.contains("shiplog share manager --out"));
    assert!(!stdout.contains("stable-env-key"));
    assert!(
        !tmp.path()
            .join("run_fixture/profiles/manager/packet.md")
            .exists(),
        "manager verify should not render the share packet"
    );
}

#[test]
fn share_verify_public_accepts_explicit_key_without_writing() {
    let tmp = TempDir::new().unwrap();
    collect_json_into(tmp.path());

    let assert = shiplog_cmd()
        .env_remove("SHIPLOG_REDACT_KEY")
        .args([
            "share",
            "verify",
            "public",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--redact-key",
            "stable-test-key",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert!(stdout.contains("Share verify: public"));
    assert!(stdout.contains("Public profile will use the strictest redaction profile."));
    assert!(stdout.contains("shiplog share public --out"));
    assert!(!stdout.contains("stable-test-key"));
    assert!(
        !tmp.path()
            .join("run_fixture/profiles/public/packet.md")
            .exists(),
        "public verify should not render the share packet"
    );
}

#[test]
fn share_verify_public_strict_scans_in_memory_without_writing() {
    let tmp = TempDir::new().unwrap();
    collect_json_into(tmp.path());

    let assert = shiplog_cmd()
        .env_remove("SHIPLOG_REDACT_KEY")
        .args([
            "share",
            "verify",
            "public",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--redact-key",
            "stable-test-key",
            "--strict",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert!(stdout.contains("Share verify: public"));
    assert!(stdout.contains("Strict public scan checked in-memory public render"));
    assert!(
        stdout
            .contains("Strict public scan inspected in-memory public render for obvious raw URLs")
    );
    assert!(stdout.contains("Strict scan is a guardrail, not a guarantee of perfect privacy."));
    assert!(stdout.contains("Result: ready to render public share output."));
    assert!(!stdout.contains("stable-test-key"));
    assert!(
        !tmp.path()
            .join("run_fixture/profiles/public/packet.md")
            .exists(),
        "strict public verify should not write the share packet"
    );
}

#[test]
fn share_verify_public_strict_fails_on_existing_unredacted_packet() {
    let tmp = TempDir::new().unwrap();
    let run_dir = collect_json_into(tmp.path());
    let public_dir = run_dir.join("profiles/public");
    std::fs::create_dir_all(&public_dir).unwrap();
    std::fs::write(
        public_dir.join("packet.md"),
        "# Public Packet\n\nLeaked https://github.com/acme/platform and acme/platform.\n",
    )
    .unwrap();

    let assert = shiplog_cmd()
        .env_remove("SHIPLOG_REDACT_KEY")
        .args([
            "share",
            "verify",
            "public",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--redact-key",
            "stable-test-key",
            "--strict",
        ])
        .assert()
        .failure();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();

    assert!(stdout.contains("Share verify: public"));
    assert!(stdout.contains("Strict public scan: public packet contains raw URL(s)."));
    assert!(
        stdout.contains("Strict public scan: public packet contains an original repository name.")
    );
    assert!(stdout.contains("Result: review attention items before sharing public output."));
    assert!(stderr.contains("strict public verification found"));
    assert!(!stdout.contains("stable-test-key"));
    assert!(!stderr.contains("stable-test-key"));
}

#[test]
fn share_manager_uses_env_key_without_printing_secret() {
    let tmp = TempDir::new().unwrap();
    collect_json_into(tmp.path());

    let assert = shiplog_cmd()
        .env("SHIPLOG_REDACT_KEY", "stable-env-key")
        .args([
            "share",
            "manager",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert!(stdout.contains("Wrote manager share output:"));
    assert!(stdout.contains("profiles"));
    assert!(stdout.contains("manager"));
    assert!(!stdout.contains("stable-env-key"));
    assert!(
        tmp.path()
            .join("run_fixture/profiles/manager/packet.md")
            .exists(),
        "manager share packet should be written with SHIPLOG_REDACT_KEY"
    );
    let manager_manifest_path = tmp
        .path()
        .join("run_fixture/profiles/manager/share.manifest.json");
    assert!(
        manager_manifest_path.exists(),
        "manager share manifest should be written"
    );
    let manager_manifest_text = std::fs::read_to_string(&manager_manifest_path).unwrap();
    assert!(!manager_manifest_text.contains("stable-env-key"));
    let manager_manifest: serde_json::Value = serde_json::from_str(&manager_manifest_text).unwrap();
    assert_eq!(manager_manifest["schema_version"], 1);
    assert_eq!(manager_manifest["profile"], "manager");
    assert_eq!(manager_manifest["input_run_id"], "run_fixture");
    assert_eq!(manager_manifest["redaction_key_source"], "env");
    assert_eq!(manager_manifest["coverage_completeness"], "Complete");
    assert_eq!(manager_manifest["skipped_source_count"], 0);
    assert_eq!(
        manager_manifest["strict_verify_result"]["status"],
        "not_applicable"
    );
    assert_eq!(
        manager_manifest["packet_path"],
        "profiles/manager/packet.md"
    );
    assert!(manager_manifest["zip_path"].is_null());
    assert_eq!(manager_manifest["checksum"]["algorithm"], "sha256");
    assert!(
        manager_manifest["checksum"]["packet_sha256"]
            .as_str()
            .is_some_and(|value| value.len() == 64)
    );
    assert!(manager_manifest["checksum"]["zip_sha256"].is_null());
    let manager_packet =
        std::fs::read_to_string(tmp.path().join("run_fixture/profiles/manager/packet.md")).unwrap();
    assert!(
        manager_packet.contains("**Window:** 2025-01-01..2025-04-01"),
        "share should derive the render window from coverage"
    );
}

#[test]
fn share_public_with_explicit_key_can_write_zip() {
    let tmp = TempDir::new().unwrap();
    collect_json_into(tmp.path());

    let assert = shiplog_cmd()
        .env_remove("SHIPLOG_REDACT_KEY")
        .args([
            "share",
            "public",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--redact-key",
            "stable-test-key",
            "--zip",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert!(stdout.contains("Wrote public share output:"));
    assert!(!stdout.contains("stable-test-key"));
    assert!(
        tmp.path()
            .join("run_fixture/profiles/public/packet.md")
            .exists(),
        "public share packet should be written with an explicit key"
    );
    assert!(
        tmp.path().join("run_fixture.public.zip").exists(),
        "public share zip should be written when --zip is set"
    );
    let public_manifest_text = std::fs::read_to_string(
        tmp.path()
            .join("run_fixture/profiles/public/share.manifest.json"),
    )
    .unwrap();
    assert!(!public_manifest_text.contains("stable-test-key"));
    let public_manifest: serde_json::Value = serde_json::from_str(&public_manifest_text).unwrap();
    assert_eq!(public_manifest["profile"], "public");
    assert_eq!(public_manifest["redaction_key_source"], "explicit");
    assert_eq!(public_manifest["strict_verify_result"]["status"], "passed");
    assert_eq!(
        public_manifest["strict_verify_result"]["source"],
        "profiles/public/packet.md"
    );
    assert_eq!(public_manifest["packet_path"], "profiles/public/packet.md");
    assert_eq!(public_manifest["zip_path"], "../run_fixture.public.zip");
    assert!(
        public_manifest["checksum"]["zip_sha256"]
            .as_str()
            .is_some_and(|value| value.len() == 64)
    );
}

#[test]
fn share_verify_manifest_public_validates_packet_and_zip_receipts() {
    let tmp = TempDir::new().unwrap();
    collect_json_into(tmp.path());

    shiplog_cmd()
        .env_remove("SHIPLOG_REDACT_KEY")
        .args([
            "share",
            "public",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
            "--redact-key",
            "stable-test-key",
            "--zip",
        ])
        .assert()
        .success();

    let assert = shiplog_cmd()
        .env_remove("SHIPLOG_REDACT_KEY")
        .args([
            "share",
            "verify",
            "manifest",
            "--profile",
            "public",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert!(stdout.contains("Share manifest verify: public"));
    assert!(stdout.contains("Manifest schema v1"));
    assert!(stdout.contains("Profile matches public"));
    assert!(stdout.contains("Packet checksum matches profiles/public/packet.md"));
    assert!(stdout.contains("Zip checksum matches ../run_fixture.public.zip"));
    assert!(stdout.contains("Public strict result recorded as passed"));
    assert!(stdout.contains("Result: share manifest verified."));
    assert!(!stdout.contains("stable-test-key"));
}

#[test]
fn share_verify_manifest_detects_packet_checksum_mismatch() {
    let tmp = TempDir::new().unwrap();
    collect_json_into(tmp.path());

    shiplog_cmd()
        .env("SHIPLOG_REDACT_KEY", "stable-env-key")
        .args([
            "share",
            "manager",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
        ])
        .assert()
        .success();
    std::fs::write(
        tmp.path().join("run_fixture/profiles/manager/packet.md"),
        "# Tampered manager packet\n",
    )
    .unwrap();

    shiplog_cmd()
        .env_remove("SHIPLOG_REDACT_KEY")
        .args([
            "share",
            "verify",
            "manifest",
            "--profile",
            "manager",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("share packet checksum mismatch"));
}

#[test]
fn share_verify_manifest_rejects_profile_mismatch() {
    let tmp = TempDir::new().unwrap();
    collect_json_into(tmp.path());

    shiplog_cmd()
        .env("SHIPLOG_REDACT_KEY", "stable-env-key")
        .args([
            "share",
            "manager",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
        ])
        .assert()
        .success();
    let public_manifest_dir = tmp.path().join("run_fixture/profiles/public");
    std::fs::create_dir_all(&public_manifest_dir).unwrap();
    std::fs::copy(
        tmp.path()
            .join("run_fixture/profiles/manager/share.manifest.json"),
        public_manifest_dir.join("share.manifest.json"),
    )
    .unwrap();

    shiplog_cmd()
        .env_remove("SHIPLOG_REDACT_KEY")
        .args([
            "share",
            "verify",
            "manifest",
            "--profile",
            "public",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "share manifest profile mismatch: expected public, found manager",
        ));
}

#[test]
fn share_verify_manifest_rejects_packet_path_traversal() {
    let tmp = TempDir::new().unwrap();
    collect_json_into(tmp.path());

    shiplog_cmd()
        .env("SHIPLOG_REDACT_KEY", "stable-env-key")
        .args([
            "share",
            "manager",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
        ])
        .assert()
        .success();
    let manifest_path = tmp
        .path()
        .join("run_fixture/profiles/manager/share.manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();
    manifest["packet_path"] = serde_json::json!("../outside.md");
    std::fs::write(
        &manifest_path,
        format!("{}\n", serde_json::to_string_pretty(&manifest).unwrap()),
    )
    .unwrap();

    shiplog_cmd()
        .env_remove("SHIPLOG_REDACT_KEY")
        .args([
            "share",
            "verify",
            "manifest",
            "--profile",
            "manager",
            "--out",
            tmp.path().to_str().unwrap(),
            "--run",
            "run_fixture",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "share manifest packet_path must not traverse outside the run directory",
        ));
}

#[test]
fn render_public_profile_receipt_options_override_profile_defaults() {
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
            "--receipt-limit",
            "3",
            "--appendix",
            "summary",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Rendered"));

    let public_packet =
        std::fs::read_to_string(tmp.path().join("run_fixture/profiles/public/packet.md")).unwrap();
    assert!(
        public_packet.contains("## Appendix: Receipt Summary"),
        "explicit --appendix should override public profile defaults"
    );
    assert!(
        !public_packet.contains("## Appendix: All Receipts"),
        "summary appendix override should not render full receipt detail"
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
        .stdout(predicate::str::contains("multi").not())
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
        .stdout(predicate::str::contains("multi").not())
        .stdout(predicate::str::contains("--out"));
}
