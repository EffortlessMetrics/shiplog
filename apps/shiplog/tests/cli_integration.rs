//! Comprehensive CLI integration tests using `assert_cmd` and `predicates`.

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
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
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
    assert!(
        report_json["window"]["label"].as_str().is_some(),
        "intake.report.json should expose the resolved window label"
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
        "## Share Commands",
        "## Artifacts",
    ] {
        assert!(
            report_md.contains(section),
            "intake.report.md should contain {section:?}"
        );
    }

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

fn load_first_intake_report(out: &Path) -> (PathBuf, serde_json::Value) {
    let report_path = first_run_dir(out).join("intake.report.json");
    let report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report_path).unwrap()).unwrap();
    (report_path, report)
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
        .stdout(predicate::str::contains("run"));
}

#[test]
fn init_help_shows_options() {
    shiplog_cmd()
        .args(["init", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--source"))
        .stdout(predicate::str::contains("--dry-run"))
        .stdout(predicate::str::contains("--force"));
}

#[test]
fn doctor_help_shows_options() {
    shiplog_cmd()
        .args(["doctor", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--config"))
        .stdout(predicate::str::contains("--source"))
        .stdout(predicate::str::contains("--repair-plan"));
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
        .stdout(predicate::str::contains("compare"));
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
fn share_help_shows_profiles_and_safety_options() {
    shiplog_cmd()
        .args(["share", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("manager"))
        .stdout(predicate::str::contains("public"));

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
    assert!(config.contains("[sources.jira]\nenabled = true"));
    assert!(config.contains("[sources.linear]\nenabled = true"));
    assert!(config.contains("[sources.github]\nenabled = false"));
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
    let (_, report) = load_first_intake_report(&out);
    let repair_id = first_repair_id_with_action(&report, "journal_add");

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
        .stdout(predicate::str::contains(
            "shiplog intake --last-6-months --explain",
        ));

    let file: ManualEventsFile = serde_yaml::from_str(&std::fs::read_to_string(&manual_events)?)?;
    assert_eq!(file.events.len(), 1);
    let entry = &file.events[0];
    assert!(entry.id.contains(&repair_id));
    assert_eq!(entry.title, format!("Manual evidence repair ({repair_id})"));
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
        .args(["doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Config: ok"))
        .stdout(predicate::str::contains("JSON: ok"))
        .stdout(predicate::str::contains("Manual: ok"))
        .stdout(predicate::str::contains("Redaction: ok"));
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
        .stdout(predicate::str::contains("export GITHUB_TOKEN=..."))
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
            .and(predicate::str::contains(
                "export SHIPLOG_TEST_REDACT_KEY_FOR_CONFIG=replace-with-a-stable-secret",
            ))
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
            "--config",
            tmp.path().join("shiplog.toml").to_str().unwrap(),
            "--last-6-months",
            "--no-open",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Review intake complete."))
        .stdout(predicate::str::contains("- JSON: success"))
        .stdout(predicate::str::contains("- Manual: success"))
        .stdout(predicate::str::contains("Evidence debt:"))
        .stdout(predicate::str::contains("Next:"))
        .stdout(predicate::str::contains("Intake readiness:"))
        .stdout(predicate::str::contains("Packet readiness: Needs curation"))
        .stdout(predicate::str::contains("- Packet rendered"))
        .stdout(predicate::str::contains("- Review inspection completed"))
        .stdout(predicate::str::contains("shiplog render --out"))
        .stdout(predicate::str::contains("--bundle-profile manager"))
        .stdout(predicate::str::contains("Open later:"));

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
    assert!(stdout.contains("Packet readiness: Ready for review"));
    assert!(stdout.contains("Needs attention:\n- None"));
    assert!(stdout.contains("shiplog render --out"));
    assert!(stdout.contains("--bundle-profile manager"));
    assert!(stdout.contains("shiplog open packet --out"));

    let run_dir = first_run_dir(&out);
    assert_intake_artifacts(&run_dir);
    assert_ledger_event_count(&run_dir, 1);

    let (report_md, report_json) = assert_golden_intake_report(&run_dir, "Ready for review");
    assert!(report_md.contains("Packet readiness: **Ready for review**"));
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
    assert!(
        packet.contains("Ready with caveats.") && packet.contains("manual_only"),
        "manual-only packet should surface readiness caveats in packet.md"
    );
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
    assert!(stdout.contains("shiplog render --out"));
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
        .stdout(predicate::str::contains("Packet readiness: Needs curation"))
        .stdout(predicate::str::contains(
            "- Jira skipped: missing JIRA_TOKEN",
        ))
        .stdout(predicate::str::contains(
            "- Linear skipped: missing LINEAR_API_KEY",
        ))
        .stdout(predicate::str::contains("Repair sources:"))
        .stdout(predicate::str::contains("kind: missing_token"))
        .stdout(predicate::str::contains("export JIRA_TOKEN=..."))
        .stdout(predicate::str::contains(
            "shiplog identify jira --auth-user <email>",
        ))
        .stdout(predicate::str::contains("export LINEAR_API_KEY=..."))
        .stdout(predicate::str::contains("shiplog identify linear"))
        .stdout(predicate::str::contains("shiplog doctor --config"));

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
                && repair["commands"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|line| line.as_str().unwrap().contains("shiplog identify jira")))
    );
    let repair_items = report_json["repair_items"].as_array().unwrap();
    assert!(
        repair_items
            .iter()
            .any(|item| item["kind"] == "source_skipped_configuration"
                && item["source_key"] == "jira"
                && item["action"]["kind"] == "configure_source"
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
    assert!(stdout.contains("- GitLab: included"));
    assert!(stdout.contains("- Jira: included"));
    assert!(stdout.contains("- Linear: included"));
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
                .any(|command| command.as_str().unwrap().contains("sources.gitlab.state"))
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
                .any(|command| command.as_str().unwrap().contains("sources.jira.status"))
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
                .any(|command| command.as_str().unwrap().contains("sources.linear.status"))
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
                .and(predicate::str::contains(
                    "export SHIPLOG_REDACT_KEY=replace-with-a-stable-secret",
                ))
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
    assert!(stdout.contains("shiplog review --run run_all_sources"));

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

    assert!(stdout.contains("Compare: run_fixture -> run_period_to"));
    assert!(stdout.contains("- from: 3"));
    assert!(stdout.contains("- to: 8"));
    assert!(stdout.contains("- Added: Local git, GitLab, Jira, JSON, Linear, Manual"));
    assert!(stdout.contains("shiplog review --run run_period_to"));
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

    assert!(stdout.contains("Run: run_fixture"));
    assert!(stdout.contains("Coverage:"));
    assert!(stdout.contains("- GitHub: 3 event(s)"));
    assert!(stdout.contains("Completeness: Complete"));
    assert!(stdout.contains("Curation:"));
    assert!(stdout.contains("- Validation: ok"));
    assert!(stdout.contains("Evidence debt:"));
    assert!(stdout.contains("- No obvious evidence debt detected."));
    assert!(stdout.contains("Next:"));
    assert!(stdout.contains("shiplog render --run run_fixture --mode scaffold"));

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

    assert!(stdout.contains("Weekly review: run_fixture"));
    assert!(stdout.contains("New evidence:"));
    assert!(stdout.contains("- total: 3 event(s)"));
    assert!(stdout.contains("- GitHub: 3 event(s)"));
    assert!(stdout.contains("Evidence debt:"));
    assert!(stdout.contains("- No obvious evidence debt detected."));
    assert!(stdout.contains("Next:"));
    assert!(stdout.contains("shiplog render --run run_fixture --mode scaffold"));

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

    for needle in [
        "[warning] no-selected-receipts",
        "[info] thin-workstream",
        "[warning] large-misc-workstream",
        "[info] code-only-workstream",
        "[info] ticket-only-workstream",
        "[info] manual-only-workstream",
        "[info] too-many-selected-receipts",
        "shiplog workstreams receipts --run run_quality_debt --workstream <title>",
        "shiplog workstreams split --run run_quality_debt",
        "shiplog journal add --date",
        "shiplog journal list",
    ] {
        assert!(
            stdout.contains(needle),
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

    shiplog_cmd()
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
        .stdout(predicate::str::contains("Readiness:"))
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
    std::fs::write(
        &report_path,
        format!("{}\n", serde_json::to_string_pretty(&report)?),
    )?;

    shiplog_cmd()
        .args(["repair", "plan", "--out", out_arg.as_str(), "--latest"])
        .assert()
        .success()
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
        ));

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
    let report_modified = std::fs::metadata(&report_path).unwrap().modified().unwrap();

    shiplog_cmd()
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
        .stdout(predicate::str::contains("Readiness:"))
        .stdout(predicate::str::contains("Sources:"))
        .stdout(predicate::str::contains("Evidence debt:"))
        .stdout(predicate::str::contains("Top repairs:"))
        .stdout(predicate::str::contains("missing_token"))
        .stdout(predicate::str::contains("Top fixups:"))
        .stdout(predicate::str::contains("Share next:"))
        .stdout(predicate::str::contains("Packet:"))
        .stdout(predicate::str::contains("Intake report:"));

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
                .and(predicate::str::contains(
                    "export SHIPLOG_REDACT_KEY=replace-with-a-stable-secret",
                ))
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
                .and(predicate::str::contains(
                    "export SHIPLOG_REDACT_KEY=replace-with-a-stable-secret",
                ))
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
