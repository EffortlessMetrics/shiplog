//! Fixture-safe command tests for documented review-cycle workflows.

use assert_cmd::Command;
use predicates::prelude::*;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use tempfile::TempDir;

fn shiplog_cmd() -> Command {
    Command::from_std(std::process::Command::new(env!("CARGO_BIN_EXE_shiplog")))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn example_config(name: &str) -> PathBuf {
    repo_root().join("examples/configs").join(name)
}

fn assert_contains_in_order(doc: &str, doc_label: &str, needles: &[&str]) {
    let mut cursor = 0;
    for needle in needles {
        let Some(offset) = doc[cursor..].find(needle) else {
            panic!("{doc_label} should mention {needle:?} after the prior ordered command");
        };
        cursor += offset + needle.len();
    }
}

fn section_between<'a>(doc: &'a str, start: &str, end: &str) -> &'a str {
    let start_index = doc
        .find(start)
        .unwrap_or_else(|| panic!("document should contain section start {start:?}"));
    let section_start = start_index + start.len();
    let end_index = doc[section_start..]
        .find(end)
        .map(|offset| section_start + offset)
        .unwrap_or_else(|| panic!("document should contain section end {end:?} after {start:?}"));
    &doc[section_start..end_index]
}

#[test]
fn config_reference_documents_current_surface() {
    let doc_path = repo_root().join("docs/config-reference.md");
    let doc = std::fs::read_to_string(&doc_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", doc_path.display()));

    for needle in [
        "shiplog init --guided",
        "shiplog config validate --config shiplog.toml",
        "shiplog config explain --config shiplog.toml",
        "shiplog doctor --config shiplog.toml",
        "shiplog doctor --config shiplog.toml --setup",
        "shiplog doctor --config shiplog.toml --setup --json",
        "shiplog sources status --config shiplog.toml",
        "shiplog status --latest",
        "shiplog status --latest --json",
        "shiplog doctor --config shiplog.toml --repair-plan",
        "[shiplog]",
        "config_version = 1",
        "implicit v1",
        "[defaults]",
        "defaults.out",
        "defaults.window",
        "[periods.\"review-cycle\"]",
        "shiplog periods list",
        "shiplog periods explain review-cycle",
        "shiplog intake --period review-cycle",
        "shiplog collect multi --period 2026-H1",
        "shiplog review --period 2026-H1",
        "shiplog runs compare --from-period 2025-H2 --to-period 2026-H1",
        "last-6-months",
        "last-quarter",
        "year:2025",
        "internal",
        "manager",
        "public",
        "[sources.github]",
        "me = false",
        "GITHUB_TOKEN",
        "api_base",
        "no_details",
        "[sources.gitlab]",
        "GITLAB_TOKEN",
        "opened",
        "merged",
        "closed",
        "[sources.jira]",
        "auth_user_env",
        "JIRA_TOKEN",
        "assignee JQL value",
        "[sources.linear]",
        "user_id",
        "LINEAR_API_KEY",
        "backlog",
        "cancelled",
        "[sources.git]",
        "include_merges",
        "[sources.json]",
        "coverage",
        "[sources.manual]",
        "manual_events.yaml",
        "[redaction]",
        "SHIPLOG_REDACT_KEY",
        "config validate",
        "doctor",
        "doctor --setup",
        "doctor --setup --json",
        "sources status",
        "review-loop preflight",
        "not `packet.md`",
        "Setup Readiness v1",
        "Review Loop Status v1",
        "schemas/setup-readiness-v1.md",
        "schemas/review-loop-status-v1.md",
        "doctor --repair-plan",
    ] {
        assert!(
            doc.contains(needle),
            "config reference should mention {needle:?}"
        );
    }
}

#[test]
fn changelog_curates_0_9_as_review_loop_cockpit_release_notes() {
    let doc_path = repo_root().join("CHANGELOG.md");
    let doc = std::fs::read_to_string(&doc_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", doc_path.display()));

    let unreleased = section_between(&doc, "## [Unreleased]", "## [0.9.0]");
    assert!(
        unreleased.contains("GitHub activity harvest profiles"),
        "Unreleased should describe post-0.9-candidate GitHub activity harvest work"
    );
    assert!(
        unreleased.contains("shiplog github activity scout")
            && unreleased.contains("shiplog github activity run --profile authored|full --resume"),
        "Unreleased should name the executable GitHub activity harvest commands"
    );
    assert!(
        unreleased.contains("without release execution"),
        "Unreleased should preserve release-hold boundaries for post-candidate work"
    );
    assert!(
        !unreleased.contains("#424") && !unreleased.contains("#436"),
        "review-loop status receipts belong in the 0.9 candidate section, not Unreleased"
    );

    let candidate = section_between(&doc, "## [0.9.0]", "## [0.8.0]");
    for needle in [
        "review-loop cockpit release",
        "diagnose setup, collect evidence, inspect status, repair gaps,",
        "shiplog status --latest",
        "shiplog status --latest --json",
        "shiplog init --guided",
        "doctor --setup",
        "sources status",
        "doctor --setup --json",
        "Packet Readiness",
        "evidence strength",
        "receipt-backed claim candidates",
        "missing-context prompts",
        "share explain manager|public",
        "runs diff --latest",
        "read-first",
        "setup-blocked repairs route through doctor/source status",
        "fail closed",
        "repo = \".\"",
        "zero-event source \"Good\"",
        "old/partial report and setup compatibility",
        "Windows path and environment-variable display",
        "setup-readiness, review-ready, and review-loop status",
        "review-loop status transcript (#434)",
        "recurring",
        "review-loop guide",
        "Key receipts: #307-#319, #337-#398, #399-#422, #424-#436.",
        "Release execution is still paused",
    ] {
        assert!(
            candidate.contains(needle),
            "0.9 changelog candidate should mention {needle:?}"
        );
    }
    assert!(
        !candidate.contains("### Post-0.8 soak"),
        "0.9 changelog should be release-note shaped, not a PR-by-PR soak ledger"
    );
}

#[test]
fn docs_teach_status_as_review_loop_cockpit_after_setup() {
    let root = repo_root();

    for (relative_path, ordered_commands) in [
        (
            "README.md",
            vec![
                "shiplog init --guided",
                "shiplog doctor --setup",
                "shiplog sources status",
                "shiplog doctor --setup --json",
                "shiplog status --latest",
                "shiplog intake --last-6-months --explain",
                "shiplog status --latest",
                "shiplog repair plan --latest",
            ],
        ),
        (
            "apps/shiplog/README.md",
            vec![
                "shiplog init --guided",
                "shiplog doctor --setup",
                "shiplog sources status",
                "shiplog doctor --setup --json",
                "shiplog status --latest",
                "shiplog intake --last-6-months --explain",
                "shiplog status --latest",
                "shiplog repair plan --latest",
            ],
        ),
        (
            "docs/guides/rapid-first-intake.md",
            vec![
                "shiplog init --guided",
                "shiplog doctor --setup",
                "shiplog sources status",
                "shiplog doctor --setup --json",
                "shiplog status --latest",
                "shiplog intake --last-6-months --explain",
                "shiplog status --latest",
            ],
        ),
        (
            "docs/guides/guided-setup-doctor.md",
            vec![
                "shiplog init --guided",
                "shiplog doctor --setup",
                "shiplog sources status",
                "shiplog doctor --setup --json",
                "shiplog status --latest",
                "shiplog intake --last-6-months --explain",
                "shiplog status --latest",
                "shiplog repair plan --latest",
            ],
        ),
        (
            "docs/guides/review-ready-packet.md",
            vec![
                "shiplog init --guided",
                "shiplog doctor --setup",
                "shiplog sources status",
                "shiplog doctor --setup --json",
                "shiplog status --latest",
                "shiplog intake --last-6-months --explain",
                "shiplog status --latest",
            ],
        ),
    ] {
        let path = root.join(relative_path);
        let doc = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
        assert_contains_in_order(&doc, relative_path, &ordered_commands);
        assert!(
            doc.contains("status --latest"),
            "{relative_path} should teach status as the review-loop cockpit"
        );
    }
}

#[test]
fn intake_report_schema_docs_describe_v1_contract() {
    let doc_path = repo_root().join("docs/schemas/intake-report-v1.md");
    let doc = std::fs::read_to_string(&doc_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", doc_path.display()));
    let schema_path = repo_root().join("contracts/schemas/intake-report.v1.schema.json");
    let schema = std::fs::read_to_string(&schema_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", schema_path.display()));
    let schema_json: serde_json::Value = serde_json::from_str(&schema)
        .unwrap_or_else(|err| panic!("parse {}: {err}", schema_path.display()));

    assert_eq!(schema_json["properties"]["schema_version"]["const"], 1);
    assert_eq!(schema_json["additionalProperties"], false);

    for needle in [
        "contracts/schemas/intake-report.v1.schema.json",
        "shiplog report validate --latest",
        "shiplog report validate --path out/<run>/intake.report.json",
        "shiplog report summarize --latest",
        "shiplog report export-agent-pack --latest --output agent-pack.json",
        "shiplog repair plan --latest",
        "schema_version",
        "Ready for review",
        "Needs curation",
        "Needs evidence",
        "Needs repair",
        "included_sources",
        "skipped_sources",
        "source_freshness",
        "repair_sources",
        "repair_sources[].kind",
        "repair_items",
        "repair_id",
        "repair_key",
        "manual_evidence_missing",
        "source_skipped_configuration",
        "receipt_refs[].field",
        "packet_quality",
        "packet_readiness",
        "evidence_strength",
        "ready_with_caveats",
        "manual_only",
        "source_skipped",
        "claim_candidates",
        "supporting_receipt_refs",
        "missing_context_prompts",
        "safe_profiles",
        "needs_attention",
        "journal_add",
        "no_safe_action",
        "missing_token",
        "invalid_filter",
        "evidence_debt",
        "top_fixups",
        "top_fixups[].id",
        "top_fixups[].kind",
        "manual_context",
        "select_receipts",
        "actions",
        "repair_source",
        "share_manager",
        "writes",
        "risk",
        "share_commands",
        "Share Explain Commands",
        "read-only commands",
        "before `Share Commands`",
        "must not include token values",
        "not be used for productivity scoring",
        "agent-pack-v1.md",
    ] {
        assert!(
            doc.contains(needle),
            "intake report schema docs should mention {needle:?}"
        );
    }
}

#[test]
fn agent_pack_schema_docs_describe_v1_contract() {
    let doc_path = repo_root().join("docs/schemas/agent-pack-v1.md");
    let doc = std::fs::read_to_string(&doc_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", doc_path.display()));
    let schema_path = repo_root().join("contracts/schemas/agent-pack.v1.schema.json");
    let schema = std::fs::read_to_string(&schema_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", schema_path.display()));
    let schema_json: serde_json::Value = serde_json::from_str(&schema)
        .unwrap_or_else(|err| panic!("parse {}: {err}", schema_path.display()));

    assert_eq!(schema_json["properties"]["schema_version"]["const"], 1);
    assert_eq!(schema_json["additionalProperties"], false);

    for needle in [
        "contracts/schemas/agent-pack.v1.schema.json",
        "shiplog report export-agent-pack --latest --output agent-pack.json",
        "shiplog report export-agent-pack --path out/<run>/intake.report.json",
        "schema_version",
        "source_report",
        "summary",
        "gaps",
        "repairs",
        "fixups",
        "share_status",
        "actions",
        "artifacts",
        "productivity metrics",
        "must not include token values",
        "shiplog share verify public --latest --strict",
    ] {
        assert!(
            doc.contains(needle),
            "agent pack schema docs should mention {needle:?}"
        );
    }
}

#[test]
fn setup_readiness_schema_docs_and_examples_describe_v1_contract() {
    let root = repo_root();
    let doc_path = root.join("docs/schemas/setup-readiness-v1.md");
    let schema_path = root.join("contracts/schemas/setup-readiness.v1.schema.json");
    let guide_path = root.join("docs/guides/guided-setup-doctor.md");
    let matrix_path = root.join("docs/product/setup-readiness-dogfood-matrix.md");

    let doc = std::fs::read_to_string(&doc_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", doc_path.display()));
    let schema_text = std::fs::read_to_string(&schema_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", schema_path.display()));
    let guide = std::fs::read_to_string(&guide_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", guide_path.display()));
    let matrix = std::fs::read_to_string(&matrix_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", matrix_path.display()));
    let schema: serde_json::Value = serde_json::from_str(&schema_text)
        .unwrap_or_else(|err| panic!("parse {}: {err}", schema_path.display()));

    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(
        schema["properties"]["overall_status"]["$ref"],
        "#/$defs/overall_status"
    );
    assert!(
        schema["propertyNames"].is_object(),
        "setup readiness schema should include property-name hygiene"
    );

    for field in [
        "overall_status",
        "sources",
        "local_files",
        "credentials",
        "share_profiles",
        "next_actions",
    ] {
        assert!(
            schema["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value == field),
            "schema should require {field}"
        );
        assert!(
            doc.contains(field),
            "setup readiness schema docs should mention {field}"
        );
    }

    for needle in [
        "contracts/schemas/setup-readiness.v1.schema.json",
        "shiplog doctor --setup --json",
        "examples/setup-readiness/blocked.json",
        "examples/setup-readiness/needs-setup.json",
        "examples/setup-readiness/ready-with-caveats.json",
        "setup readiness, not evidence freshness",
        "ready_with_caveats",
        "needs_setup",
        "blocked",
        "next_action.writes",
        "must not include token values",
        "does not query providers",
        "does not mutate config",
        "does not render share artifacts",
        "does not scrape `packet.md`",
    ] {
        assert!(
            doc.contains(needle),
            "setup readiness schema docs should mention {needle:?}"
        );
    }
    assert!(
        guide.contains("docs/schemas/setup-readiness-v1.md")
            || guide.contains("../schemas/setup-readiness-v1.md"),
        "guided setup guide should link setup readiness schema docs"
    );
    assert!(
        !matrix.contains("schema/example coverage for setup readiness JSON"),
        "dogfood matrix should no longer list schema/example coverage as missing proof"
    );

    let overall_statuses = schema_string_set(&schema, "/$defs/overall_status/enum");
    for status in ["ready", "ready_with_caveats", "needs_setup", "blocked"] {
        assert!(
            overall_statuses.contains(status),
            "schema should allow overall status {status}"
        );
    }

    let item_statuses = schema_string_set(&schema, "/$defs/item_status/enum");
    for status in [
        "ready",
        "ready_with_caveats",
        "disabled",
        "unavailable",
        "blocked",
        "stale_config",
        "unknown",
        "missing",
        "malformed",
        "optional_absent",
        "not_generated",
    ] {
        assert!(
            item_statuses.contains(status),
            "schema should allow setup item status {status}"
        );
    }

    for example in [
        "blocked.json",
        "needs-setup.json",
        "ready-with-caveats.json",
    ] {
        let example_path = root.join("examples/setup-readiness").join(example);
        let text = std::fs::read_to_string(&example_path)
            .unwrap_or_else(|err| panic!("read {}: {err}", example_path.display()));
        let json: serde_json::Value = serde_json::from_str(&text)
            .unwrap_or_else(|err| panic!("parse {}: {err}", example_path.display()));
        assert_setup_readiness_example_matches_schema_shape(
            &json,
            &overall_statuses,
            &item_statuses,
            &example_path,
        );
    }
}

#[test]
fn review_loop_status_schema_docs_and_examples_describe_v1_contract()
-> Result<(), Box<dyn std::error::Error>> {
    let root = repo_root();
    let doc_path = root.join("docs/schemas/review-loop-status-v1.md");
    let schema_path = root.join("contracts/schemas/review-loop-status.v1.schema.json");
    let spec_path = root.join("docs/specs/SHIPLOG-SPEC-0008-review-loop-status.md");

    let doc = std::fs::read_to_string(&doc_path)?;
    let schema_text = std::fs::read_to_string(&schema_path)?;
    let spec = std::fs::read_to_string(&spec_path)?;
    let schema: serde_json::Value = serde_json::from_str(&schema_text)?;

    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(
        schema["properties"]["overall_status"]["$ref"],
        "#/$defs/overall_status"
    );
    assert!(
        schema["propertyNames"].is_object(),
        "review-loop status schema should include property-name hygiene"
    );

    for field in [
        "overall_status",
        "setup_summary",
        "latest_run",
        "packet_readiness",
        "source_summary",
        "repair_summary",
        "diff_summary",
        "share_summary",
        "blocking_reasons",
        "next_actions",
        "receipt_refs",
    ] {
        let required = schema["required"].as_array();
        assert!(
            required.is_some_and(|values| values.iter().any(|value| value == field)),
            "review-loop status schema should require {field}"
        );
        assert!(
            doc.contains(field),
            "review-loop status schema docs should mention {field}"
        );
    }

    for needle in [
        "contracts/schemas/review-loop-status.v1.schema.json",
        "shiplog status --latest --json",
        "examples/review-loop-status/missing-setup.json",
        "examples/review-loop-status/ready-to-collect.json",
        "examples/review-loop-status/needs-evidence.json",
        "examples/review-loop-status/needs-repair.json",
        "examples/review-loop-status/repair-in-progress.json",
        "examples/review-loop-status/ready-with-caveats.json",
        "examples/review-loop-status/share-blocked.json",
        "examples/review-loop-status/ready-to-share.json",
        "examples/review-loop-status/unknown-old-report.json",
        "examples/review-loop-status/malformed-report.json",
        "setup/evidence/repair/diff",
        "not packet prose",
        "ready_to_collect",
        "needs_evidence",
        "needs_repair",
        "repair_in_progress",
        "ready_with_caveats",
        "share_blocked",
        "ready_to_share",
        "next_actions",
        "preconditions",
        "receipt_refs",
        "must not include token values",
        "does not query providers",
        "does not mutate config",
        "does not run intake implicitly",
        "does not run repair commands",
        "does not render share artifacts",
        "does not scrape `packet.md`",
        "does not call an LLM",
        "does not execute release work",
        "Future compatible changes should be additive",
        "README",
        "crates.io README",
        "recurring",
        "review-loop guide",
        "config reference",
        "agent control-plane contract",
    ] {
        assert!(
            doc.contains(needle),
            "review-loop status schema docs should mention {needle:?}"
        );
    }
    assert!(
        spec.contains("review-loop-status.v1"),
        "review-loop status spec should link or name the v1 schema contract"
    );

    let overall_statuses = schema_string_set(&schema, "/$defs/overall_status/enum");
    for status in [
        "unknown",
        "needs_setup",
        "ready_to_collect",
        "needs_evidence",
        "needs_repair",
        "repair_in_progress",
        "ready_with_caveats",
        "ready_to_explain_share",
        "share_blocked",
        "ready_to_share",
        "blocked",
    ] {
        assert!(
            overall_statuses.contains(status),
            "schema should allow review-loop status {status}"
        );
    }

    let setup_statuses = schema_string_set(&schema, "/$defs/setup_status/enum");
    for status in [
        "ready",
        "ready_with_caveats",
        "needs_setup",
        "blocked",
        "unknown",
    ] {
        assert!(
            setup_statuses.contains(status),
            "schema should allow setup summary status {status}"
        );
    }

    let packet_statuses = schema_string_set(&schema, "/$defs/packet_readiness_status/enum");
    for status in [
        "ready",
        "ready_with_caveats",
        "needs_evidence",
        "needs_repair",
        "unknown",
    ] {
        assert!(
            packet_statuses.contains(status),
            "schema should allow packet readiness status {status}"
        );
    }

    let diff_statuses = schema_string_set(&schema, "/$defs/diff_status/enum");
    for status in [
        "available",
        "no_prior_comparable_run",
        "not_generated",
        "unknown",
    ] {
        assert!(
            diff_statuses.contains(status),
            "schema should allow diff status {status}"
        );
    }

    let share_statuses = schema_string_set(&schema, "/$defs/share_profile_status/enum");
    for status in [
        "ready",
        "ready_with_caveats",
        "blocked",
        "not_generated",
        "unknown",
    ] {
        assert!(
            share_statuses.contains(status),
            "schema should allow share profile status {status}"
        );
    }

    for example in [
        "missing-setup.json",
        "ready-to-collect.json",
        "needs-evidence.json",
        "needs-repair.json",
        "repair-in-progress.json",
        "ready-with-caveats.json",
        "share-blocked.json",
        "ready-to-share.json",
        "unknown-old-report.json",
        "malformed-report.json",
    ] {
        let example_path = root.join("examples/review-loop-status").join(example);
        let text = std::fs::read_to_string(&example_path)?;
        let json: serde_json::Value = serde_json::from_str(&text)?;
        assert_review_loop_status_example_matches_schema_shape(
            &json,
            &overall_statuses,
            &setup_statuses,
            &packet_statuses,
            &diff_statuses,
            &share_statuses,
            &example_path,
        );
    }

    Ok(())
}

fn schema_string_set(
    schema: &serde_json::Value,
    pointer: &str,
) -> std::collections::BTreeSet<String> {
    schema
        .pointer(pointer)
        .unwrap_or_else(|| panic!("schema pointer {pointer} should exist"))
        .as_array()
        .unwrap_or_else(|| panic!("schema pointer {pointer} should be an array"))
        .iter()
        .map(|value| {
            value
                .as_str()
                .unwrap_or_else(|| panic!("schema pointer {pointer} should contain strings"))
                .to_string()
        })
        .collect()
}

fn assert_setup_readiness_example_matches_schema_shape(
    json: &serde_json::Value,
    overall_statuses: &std::collections::BTreeSet<String>,
    item_statuses: &std::collections::BTreeSet<String>,
    path: &Path,
) {
    assert_allowed_object_keys(
        json,
        &[
            "overall_status",
            "sources",
            "local_files",
            "credentials",
            "share_profiles",
            "next_actions",
        ],
        path,
    );
    let overall = json["overall_status"]
        .as_str()
        .unwrap_or_else(|| panic!("{} overall_status should be a string", path.display()));
    assert!(
        overall_statuses.contains(overall),
        "{} overall_status {overall:?} should be allowed by schema",
        path.display()
    );

    for group in ["sources", "local_files", "credentials", "share_profiles"] {
        let items = json[group]
            .as_array()
            .unwrap_or_else(|| panic!("{} {group} should be an array", path.display()));
        for item in items {
            assert_setup_item_matches_schema_shape(item, item_statuses, path);
        }
    }
    let actions = json["next_actions"]
        .as_array()
        .unwrap_or_else(|| panic!("{} next_actions should be an array", path.display()));
    for action in actions {
        assert_next_action_matches_schema_shape(action, path);
    }
    assert_no_secret_sentinel_values(json, path);
}

fn assert_setup_item_matches_schema_shape(
    item: &serde_json::Value,
    item_statuses: &std::collections::BTreeSet<String>,
    path: &Path,
) {
    assert_allowed_object_keys(
        item,
        &[
            "key",
            "label",
            "enabled",
            "status",
            "reason",
            "next_action",
            "writes",
            "receipt_refs",
        ],
        path,
    );
    for field in ["key", "label", "reason"] {
        assert_non_empty_string(item, field, path);
    }
    assert!(
        item["enabled"].is_boolean(),
        "{} setup item enabled should be boolean",
        path.display()
    );
    assert!(
        item["writes"].is_boolean(),
        "{} setup item writes should be boolean",
        path.display()
    );
    let status = item["status"]
        .as_str()
        .unwrap_or_else(|| panic!("{} setup item status should be a string", path.display()));
    assert!(
        item_statuses.contains(status),
        "{} setup item status {status:?} should be allowed by schema",
        path.display()
    );
    if !item["next_action"].is_null() {
        assert_next_action_matches_schema_shape(&item["next_action"], path);
    }
    assert_receipt_refs_match_schema_shape(&item["receipt_refs"], path);
}

fn assert_next_action_matches_schema_shape(action: &serde_json::Value, path: &Path) {
    assert_allowed_object_keys(
        action,
        &[
            "key",
            "label",
            "command",
            "writes",
            "reason",
            "priority",
            "receipt_refs",
        ],
        path,
    );
    for field in ["key", "label", "command", "reason"] {
        assert_non_empty_string(action, field, path);
    }
    assert!(
        action["writes"].is_boolean(),
        "{} next action writes should be boolean",
        path.display()
    );
    assert!(
        action["priority"].as_u64().is_some(),
        "{} next action priority should be a non-negative integer",
        path.display()
    );
    assert_receipt_refs_match_schema_shape(&action["receipt_refs"], path);
}

fn assert_receipt_refs_match_schema_shape(receipt_refs: &serde_json::Value, path: &Path) {
    let refs = receipt_refs
        .as_array()
        .unwrap_or_else(|| panic!("{} receipt_refs should be an array", path.display()));
    for receipt in refs {
        assert_allowed_object_keys(receipt, &["field", "key", "path"], path);
        assert_non_empty_string(receipt, "field", path);
        assert!(
            receipt["key"].is_null() || receipt["key"].as_str().is_some(),
            "{} receipt key should be string or null",
            path.display()
        );
        assert!(
            receipt["path"].is_null() || receipt["path"].as_str().is_some(),
            "{} receipt path should be string or null",
            path.display()
        );
    }
}

fn assert_review_loop_status_example_matches_schema_shape(
    json: &serde_json::Value,
    overall_statuses: &std::collections::BTreeSet<String>,
    setup_statuses: &std::collections::BTreeSet<String>,
    packet_statuses: &std::collections::BTreeSet<String>,
    diff_statuses: &std::collections::BTreeSet<String>,
    share_statuses: &std::collections::BTreeSet<String>,
    path: &Path,
) {
    assert_allowed_object_keys(
        json,
        &[
            "overall_status",
            "setup_summary",
            "latest_run",
            "packet_readiness",
            "source_summary",
            "repair_summary",
            "diff_summary",
            "share_summary",
            "blocking_reasons",
            "next_actions",
            "receipt_refs",
        ],
        path,
    );
    assert_status_field_allowed(json, "overall_status", overall_statuses, path);
    assert_setup_status_summary_matches_schema_shape(&json["setup_summary"], setup_statuses, path);
    assert_latest_run_matches_status_schema_shape(&json["latest_run"], path);
    assert_packet_readiness_matches_status_schema_shape(
        &json["packet_readiness"],
        packet_statuses,
        path,
    );
    assert_source_summary_matches_status_schema_shape(&json["source_summary"], path);
    assert_repair_summary_matches_status_schema_shape(&json["repair_summary"], path);
    assert_diff_summary_matches_status_schema_shape(&json["diff_summary"], diff_statuses, path);
    assert_share_summary_matches_status_schema_shape(&json["share_summary"], share_statuses, path);
    assert_blocking_reasons_match_status_schema_shape(&json["blocking_reasons"], path);

    let actions = json["next_actions"].as_array();
    assert!(
        actions.is_some(),
        "{} next_actions should be an array",
        path.display()
    );
    if let Some(actions) = actions {
        for action in actions {
            assert_status_next_action_matches_schema_shape(action, path);
        }
    }
    assert_status_receipt_refs_match_schema_shape(&json["receipt_refs"], path);
    assert_no_secret_sentinel_values(json, path);
}

fn assert_setup_status_summary_matches_schema_shape(
    summary: &serde_json::Value,
    setup_statuses: &std::collections::BTreeSet<String>,
    path: &Path,
) {
    assert_allowed_object_keys(
        summary,
        &["status", "reason", "next_actions", "receipt_refs"],
        path,
    );
    assert_status_field_allowed(summary, "status", setup_statuses, path);
    assert_non_empty_string(summary, "reason", path);
    let actions = summary["next_actions"].as_array();
    assert!(
        actions.is_some(),
        "{} setup_summary.next_actions should be an array",
        path.display()
    );
    if let Some(actions) = actions {
        for action in actions {
            assert_status_next_action_matches_schema_shape(action, path);
        }
    }
    assert_status_receipt_refs_match_schema_shape(&summary["receipt_refs"], path);
}

fn assert_latest_run_matches_status_schema_shape(latest_run: &serde_json::Value, path: &Path) {
    if latest_run.is_null() {
        return;
    }
    assert_allowed_object_keys(latest_run, &["run_id", "report_path", "receipt_refs"], path);
    assert_non_empty_string(latest_run, "run_id", path);
    assert_non_empty_string(latest_run, "report_path", path);
    assert_status_receipt_refs_match_schema_shape(&latest_run["receipt_refs"], path);
}

fn assert_packet_readiness_matches_status_schema_shape(
    readiness: &serde_json::Value,
    packet_statuses: &std::collections::BTreeSet<String>,
    path: &Path,
) {
    assert_allowed_object_keys(readiness, &["status", "reason", "receipt_refs"], path);
    assert_status_field_allowed(readiness, "status", packet_statuses, path);
    assert_non_empty_string(readiness, "reason", path);
    assert_status_receipt_refs_match_schema_shape(&readiness["receipt_refs"], path);
}

fn assert_source_summary_matches_status_schema_shape(summary: &serde_json::Value, path: &Path) {
    assert_allowed_object_keys(
        summary,
        &["included", "unavailable", "disabled", "receipt_refs"],
        path,
    );
    let included = summary["included"].as_array();
    assert!(
        included.is_some(),
        "{} source_summary.included should be an array",
        path.display()
    );
    if let Some(included) = included {
        for source in included {
            assert_allowed_object_keys(
                source,
                &["source_key", "source_label", "event_count"],
                path,
            );
            assert_stable_key_string(source, "source_key", path);
            assert_non_empty_string(source, "source_label", path);
            assert!(
                source["event_count"].as_u64().is_some(),
                "{} included source event_count should be non-negative integer",
                path.display()
            );
        }
    }
    for group in ["unavailable", "disabled"] {
        let sources = summary[group].as_array();
        assert!(
            sources.is_some(),
            "{} source_summary.{group} should be an array",
            path.display()
        );
        if let Some(sources) = sources {
            for source in sources {
                assert_allowed_object_keys(source, &["source_key", "source_label", "reason"], path);
                assert_stable_key_string(source, "source_key", path);
                assert_non_empty_string(source, "source_label", path);
                assert_non_empty_string(source, "reason", path);
            }
        }
    }
    assert_status_receipt_refs_match_schema_shape(&summary["receipt_refs"], path);
}

fn assert_repair_summary_matches_status_schema_shape(summary: &serde_json::Value, path: &Path) {
    assert_allowed_object_keys(
        summary,
        &[
            "open_items",
            "safe_write_count",
            "setup_blocked_write_count",
            "applied_not_rerun",
            "receipt_refs",
        ],
        path,
    );
    for field in [
        "open_items",
        "safe_write_count",
        "setup_blocked_write_count",
    ] {
        assert!(
            summary[field].as_u64().is_some(),
            "{} repair_summary.{field} should be non-negative integer",
            path.display()
        );
    }
    assert!(
        summary["applied_not_rerun"].is_boolean(),
        "{} repair_summary.applied_not_rerun should be boolean",
        path.display()
    );
    assert_status_receipt_refs_match_schema_shape(&summary["receipt_refs"], path);
}

fn assert_diff_summary_matches_status_schema_shape(
    summary: &serde_json::Value,
    diff_statuses: &std::collections::BTreeSet<String>,
    path: &Path,
) {
    assert_allowed_object_keys(summary, &["status", "reason", "receipt_refs"], path);
    assert_status_field_allowed(summary, "status", diff_statuses, path);
    assert_non_empty_string(summary, "reason", path);
    assert_status_receipt_refs_match_schema_shape(&summary["receipt_refs"], path);
}

fn assert_share_summary_matches_status_schema_shape(
    summary: &serde_json::Value,
    share_statuses: &std::collections::BTreeSet<String>,
    path: &Path,
) {
    assert_allowed_object_keys(summary, &["profiles", "receipt_refs"], path);
    let profiles = summary["profiles"].as_array();
    assert!(
        profiles.is_some(),
        "{} share_summary.profiles should be an array",
        path.display()
    );
    if let Some(profiles) = profiles {
        for profile in profiles {
            assert_allowed_object_keys(
                profile,
                &[
                    "profile_key",
                    "profile_label",
                    "status",
                    "reason",
                    "receipt_refs",
                ],
                path,
            );
            assert_stable_key_string(profile, "profile_key", path);
            assert_non_empty_string(profile, "profile_label", path);
            assert_status_field_allowed(profile, "status", share_statuses, path);
            assert_non_empty_string(profile, "reason", path);
            assert_status_receipt_refs_match_schema_shape(&profile["receipt_refs"], path);
        }
    }
    assert_status_receipt_refs_match_schema_shape(&summary["receipt_refs"], path);
}

fn assert_blocking_reasons_match_status_schema_shape(reasons: &serde_json::Value, path: &Path) {
    let reasons = reasons.as_array();
    assert!(
        reasons.is_some(),
        "{} blocking_reasons should be an array",
        path.display()
    );
    if let Some(reasons) = reasons {
        for reason in reasons {
            assert_allowed_object_keys(
                reason,
                &["key", "label", "status", "reason", "scope", "receipt_refs"],
                path,
            );
            assert_stable_key_string(reason, "key", path);
            assert_non_empty_string(reason, "label", path);
            assert_stable_key_string(reason, "status", path);
            assert_non_empty_string(reason, "reason", path);
            assert_stable_key_string(reason, "scope", path);
            assert_status_receipt_refs_match_schema_shape(&reason["receipt_refs"], path);
        }
    }
}

fn assert_status_next_action_matches_schema_shape(action: &serde_json::Value, path: &Path) {
    assert_allowed_object_keys(
        action,
        &[
            "key",
            "label",
            "command",
            "writes",
            "reason",
            "preconditions",
            "priority",
            "receipt_refs",
        ],
        path,
    );
    assert_stable_key_string(action, "key", path);
    for field in ["label", "command", "reason"] {
        assert_non_empty_string(action, field, path);
    }
    assert!(
        action["writes"].is_boolean(),
        "{} status next action writes should be boolean",
        path.display()
    );
    let preconditions = action["preconditions"].as_array();
    assert!(
        preconditions.is_some(),
        "{} status next action preconditions should be an array",
        path.display()
    );
    if let Some(preconditions) = preconditions {
        for precondition in preconditions {
            assert!(
                precondition.as_str().is_some_and(|value| !value.is_empty()),
                "{} status next action precondition should be non-empty string",
                path.display()
            );
        }
    }
    assert!(
        action["priority"].as_u64().is_some(),
        "{} status next action priority should be a non-negative integer",
        path.display()
    );
    assert_status_receipt_refs_match_schema_shape(&action["receipt_refs"], path);
}

fn assert_status_receipt_refs_match_schema_shape(receipt_refs: &serde_json::Value, path: &Path) {
    let refs = receipt_refs.as_array();
    assert!(
        refs.is_some(),
        "{} status receipt_refs should be an array",
        path.display()
    );
    if let Some(refs) = refs {
        for receipt in refs {
            assert_allowed_object_keys(receipt, &["field", "kind", "path", "key"], path);
            assert_stable_key_string(receipt, "field", path);
            assert_stable_key_string(receipt, "kind", path);
            assert!(
                receipt["key"].is_null() || receipt["key"].as_str().is_some(),
                "{} status receipt key should be string or null",
                path.display()
            );
            assert!(
                receipt["path"].is_null() || receipt["path"].as_str().is_some(),
                "{} status receipt path should be string or null",
                path.display()
            );
        }
    }
}

fn assert_stable_key_string(json: &serde_json::Value, field: &str, path: &Path) {
    let value = json[field].as_str();
    assert!(
        value.is_some(),
        "{} field {field} should be a string",
        path.display()
    );
    if let Some(value) = value {
        assert!(
            is_schema_stable_key(value),
            "{} field {field} value {value:?} should match stable_key",
            path.display()
        );
    }
}

fn is_schema_stable_key(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_alphabetic()
        && chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | ':' | '-'))
}

fn assert_status_field_allowed(
    json: &serde_json::Value,
    field: &str,
    allowed: &std::collections::BTreeSet<String>,
    path: &Path,
) {
    let value = json[field].as_str();
    assert!(
        value.is_some(),
        "{} field {field} should be a string",
        path.display()
    );
    if let Some(value) = value {
        assert!(
            allowed.contains(value),
            "{} field {field} value {value:?} should be allowed by schema",
            path.display()
        );
    }
}

fn assert_allowed_object_keys(json: &serde_json::Value, allowed: &[&str], path: &Path) {
    let object = json
        .as_object()
        .unwrap_or_else(|| panic!("{} should contain objects at this level", path.display()));
    for required in allowed {
        assert!(
            object.contains_key(*required),
            "{} object should contain required key {required}",
            path.display()
        );
    }
    for key in object.keys() {
        assert!(
            allowed.contains(&key.as_str()),
            "{} object contains unexpected key {key:?}",
            path.display()
        );
    }
}

fn assert_non_empty_string(json: &serde_json::Value, field: &str, path: &Path) {
    assert!(
        json[field].as_str().is_some_and(|value| !value.is_empty()),
        "{} field {field} should be a non-empty string",
        path.display()
    );
}

fn assert_no_secret_sentinel_values(json: &serde_json::Value, path: &Path) {
    let text = serde_json::to_string(json)
        .unwrap_or_else(|err| panic!("serialize {} for secret scan: {err}", path.display()));
    for sentinel in [
        "shiplog-json-source-secret",
        "shiplog-json-redaction-secret",
        "replace-with-a-stable-secret",
        "token_value",
        "secret_value",
        "password",
    ] {
        assert!(
            !text.contains(sentinel),
            "{} should not contain secret sentinel {sentinel:?}",
            path.display()
        );
    }
}

#[test]
fn crate_readme_documents_first_loop_for_crates_io() {
    let doc_path = repo_root().join("apps/shiplog/README.md");
    let doc = std::fs::read_to_string(&doc_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", doc_path.display()));

    for needle in [
        "Review evidence loop for people who need receipts, not review prose.",
        "cargo install shiplog --locked",
        "What you get",
        "First useful loop",
        "Read vs write",
        "Agent-readable surfaces",
        "Read next",
        "What shiplog does not do",
        "shiplog init --guided",
        "shiplog doctor --setup",
        "shiplog sources status",
        "shiplog doctor --setup --json",
        "shiplog status --latest",
        "shiplog status --latest --json",
        "shiplog intake --last-6-months --explain",
        "shiplog repair plan --latest",
        "shiplog journal add --from-repair <repair_id>",
        "shiplog repair diff --latest",
        "shiplog runs diff --latest",
        "shiplog share explain manager --latest",
        "doctor`, `status`, and `share explain` do not render profile artifacts",
        "report export-agent-pack",
        "rapid-first-intake.md",
        "guided-setup-doctor.md",
        "recurring-review-loop.md",
        "github-activity-harvest.md",
        "evidence-repair-loop.md",
        "review-ready-packet.md",
        "config-reference.md",
        "review-loop-status-v1.md",
        "Evidence repair loop guide",
        "Review-ready packet guide",
        "Does not write performance-review prose",
        "Does not score employees",
        "Does not mutate provider records",
        "Does not query providers from `doctor` or `status`",
        "Does not render manager/public packets from `status` or `share explain`",
    ] {
        assert!(
            doc.contains(needle),
            "crate README should mention {needle:?}"
        );
    }

    for stale_detail in [
        "Curate workstreams without hand-editing YAML",
        "| Command | Description |",
        "Deterministic redaction. Three profiles",
        "keyed SHA-256 aliasing",
    ] {
        assert!(
            !doc.contains(stale_detail),
            "crate README should stay concise and omit {stale_detail:?}"
        );
    }
}

#[test]
fn root_readme_documents_0_9_review_loop_front_door() {
    let doc_path = repo_root().join("README.md");
    let doc = std::fs::read_to_string(&doc_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", doc_path.display()));
    let ripr_badge_path = repo_root().join("badges/ripr-plus.json");
    let ripr_badge = std::fs::read_to_string(&ripr_badge_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", ripr_badge_path.display()));

    for needle in [
        "<h1 align=\"center\">shiplog</h1>",
        "badges/ripr-plus.json",
        "GitHub release",
        "crates.io downloads",
        "MSRV 1.95",
        "Review readiness with receipts: setup, status, intake, repair, rerun, diff, and share safely.",
        "What works in the 0.9 candidate",
        "shiplog turns work evidence into a review-readiness loop",
        "shiplog init --guided",
        "shiplog doctor --setup",
        "shiplog sources status",
        "shiplog doctor --setup --json",
        "shiplog status --latest",
        "shiplog status --latest --json",
        "shiplog intake --last-6-months --explain",
        "shiplog repair plan --latest",
        "shiplog journal add --from-repair <repair_id>",
        "shiplog repair diff --latest",
        "shiplog runs diff --latest",
        "shiplog share explain manager --latest",
        "Status at a glance",
        "Docs map",
        "Machine-readable contracts",
        "Proof receipts",
        "What shiplog does not do",
        "Does not write performance-review prose",
        "Does not score employees",
        "Does not mutate provider records",
        "Does not query providers from `doctor` or `status`",
        "Does not render manager/public packets from `status` or `share explain`",
        "Single supported public crate: `shiplog`",
        "contracts/schemas/",
        "review-loop-status-v1.md",
        "github-activity-harvest.md",
        "review-loop-status-transcript.md",
    ] {
        assert!(
            doc.contains(needle),
            "root README should mention {needle:?}"
        );
    }
    assert!(
        !doc.contains("contracted for 0.7"),
        "root README should not carry stale 0.7 crate-surface wording"
    );
    for needle in [
        "\"schemaVersion\": 1",
        "\"label\": \"ripr+\"",
        "\"message\": \"unavailable\"",
        "\"color\": \"lightgrey\"",
    ] {
        assert!(
            ripr_badge.contains(needle),
            "ripr+ badge endpoint should mention {needle:?}"
        );
    }
}

#[test]
fn install_guide_documents_current_install_paths() {
    let doc_path = repo_root().join("docs/install.md");
    let doc = std::fs::read_to_string(&doc_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", doc_path.display()));

    for needle in [
        "shiplog-x86_64-unknown-linux-gnu",
        "shiplog-x86_64-apple-darwin",
        "shiplog-aarch64-apple-darwin",
        "shiplog-x86_64-pc-windows-msvc.exe",
        "SHA256SUMS.txt",
        "Get-FileHash",
        "sha256sum shiplog",
        "shasum -a 256 shiplog",
        "cargo install shiplog --locked",
        "cargo-binstall",
        "Homebrew",
        "Scoop and winget",
        "shiplog intake --last-6-months --explain",
        "shiplog doctor --setup --help",
        "shiplog status --help",
        "shiplog init --guided",
        "shiplog status --latest",
        "scripts/release-install-smoke.sh v0.8.0",
        "scripts\\release-install-smoke.ps1 v0.8.0",
        "scripts/demo-review-rescue.sh --out ./out/demo-review-rescue",
        "scripts\\demo-review-rescue.ps1",
        "shiplog share verify public --help",
        "Do not run a 0.9 release-install",
        "examples/configs/local-git-json-manual.toml",
    ] {
        assert!(
            doc.contains(needle),
            "install guide should mention {needle:?}"
        );
    }
}

#[test]
fn rapid_first_intake_guide_routes_manual_evidence_through_repair_plan() {
    let doc_path = repo_root().join("docs/guides/rapid-first-intake.md");
    let doc = std::fs::read_to_string(&doc_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", doc_path.display()));

    for needle in [
        "Repair Items",
        "read-first handoff",
        "shiplog repair plan --latest",
        "shiplog init --guided",
        "shiplog doctor --setup",
        "shiplog sources status",
        "shiplog doctor --setup --json",
        "shiplog status --latest",
        "review-loop cockpit",
        "recurring-review-loop.md",
        "shiplog journal add --from-repair <repair_id>",
        "write-producing curation command",
        "not tied to a repair item",
    ] {
        assert!(
            doc.contains(needle),
            "rapid first-intake guide should mention {needle:?}"
        );
    }
}

#[test]
fn evidence_repair_loop_guide_documents_report_derived_flow() {
    let doc_path = repo_root().join("docs/guides/evidence-repair-loop.md");
    let doc = std::fs::read_to_string(&doc_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", doc_path.display()));

    for needle in [
        "shiplog intake --last-6-months --explain",
        "shiplog init --guided",
        "shiplog doctor --setup",
        "shiplog sources status",
        "shiplog doctor --setup --json",
        "shiplog status --latest",
        "review-loop cockpit",
        "shiplog repair plan --latest",
        "shiplog journal add --from-repair <repair_id>",
        "shiplog repair diff --latest",
        "shiplog runs diff --latest",
        "shiplog open packet --latest",
        "Next:",
        "share explain manager",
        "Cleared",
        "New",
        "Still open",
        "Changed",
        "intake.report.json",
        "provider tickets or source records are edited",
        "read-first handoff",
        "receipt trimming",
        "workstream splitting",
        "recurring-review-loop.md",
    ] {
        assert!(
            doc.contains(needle),
            "evidence repair loop guide should mention {needle:?}"
        );
    }
}

#[test]
fn review_ready_packet_guide_documents_quality_flow() {
    let doc_path = repo_root().join("docs/guides/review-ready-packet.md");
    let doc = std::fs::read_to_string(&doc_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", doc_path.display()));

    for needle in [
        "shiplog intake --last-6-months --explain",
        "shiplog init --guided",
        "shiplog doctor --setup",
        "shiplog sources status",
        "shiplog doctor --setup --json",
        "shiplog repair plan --latest",
        "shiplog journal add --from-repair <repair_id>",
        "shiplog repair diff --latest",
        "shiplog runs diff --latest",
        "shiplog open packet --latest",
        "front panel for the loop",
        "as the detailed artifact",
        "OUT=./out/review-ready-soak",
        "shiplog repair plan --out \"$OUT\" --latest",
        "shiplog runs diff --out \"$OUT\" --latest",
        "shiplog share explain manager --out \"$OUT\" --latest",
        "Packet Readiness",
        "Claim Candidates",
        "Needs curation",
        "source-backed evidence",
        "manual_only",
        "missing-context prompts",
        "Run the repair plan before copying individual fix commands",
        "top-level `Next` handoff is intentionally",
        "write-producing commands",
        "no safe copyable command",
        "shiplog share explain manager --latest",
        "shiplog share explain public --latest",
        "shiplog share verify public --latest --strict",
        "$env:SHIPLOG_REDACT_KEY='replace-with-a-stable-secret'",
        "After a successful comparison",
        "not write `profiles/<profile>/packet.md`",
        "copy-ready commands printed by",
        "post-0.8 soak",
        "Shiplog should not invent",
    ] {
        assert!(
            doc.contains(needle),
            "review-ready packet guide should mention {needle:?}"
        );
    }
}

#[test]
fn recurring_review_loop_guide_documents_status_first_practice() {
    let doc_path = repo_root().join("docs/guides/recurring-review-loop.md");
    let doc = std::fs::read_to_string(&doc_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", doc_path.display()));

    for needle in [
        "shiplog status --latest",
        "shiplog status --latest --json",
        "shiplog intake --last-6-months --explain",
        "shiplog repair plan --latest",
        "shiplog journal add --from-repair <repair_id>",
        "shiplog repair diff --latest",
        "shiplog runs diff --latest",
        "shiplog share explain manager --latest",
        "status first, then the next receipt-producing command",
        "flagship 0.9 guide",
        "Weekly self-review",
        "Monthly manager prep",
        "Promotion packet prep",
        "Local-only mode",
        "Token-backed GitHub mode",
        "Public-share cautious mode",
        "Agent-assisted mode",
        "ready_to_collect",
        "needs_repair",
        "repair_in_progress",
        "share_blocked",
        "`writes = true`",
        "Status should not:",
        "scrape `packet.md`",
        "performance-review prose",
    ] {
        assert!(
            doc.contains(needle),
            "recurring review-loop guide should mention {needle:?}"
        );
    }
}

#[test]
fn guided_setup_doctor_guide_documents_setup_flow() {
    let doc_path = repo_root().join("docs/guides/guided-setup-doctor.md");
    let doc = std::fs::read_to_string(&doc_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", doc_path.display()));

    for needle in [
        "shiplog init --guided",
        "shiplog doctor --setup",
        "shiplog doctor --setup --json",
        "shiplog sources status",
        "shiplog status --latest",
        "review-loop preflight",
        "shiplog intake --last-6-months --explain",
        "shiplog repair plan --latest",
        "shiplog share explain manager --latest",
        "Local-only mode",
        "Manual-only mode",
        "Token-backed GitHub mode",
        "Manager-share-ready mode",
        "Public-share-cautious mode",
        "SHIPLOG_REDACT_KEY",
        "journal add --from-repair",
        "does not call the GitHub API",
        "Doctor is not a dry-run intake",
        "read-only command first",
        "Setup-readiness dogfood matrix",
    ] {
        assert!(
            doc.contains(needle),
            "guided setup doctor guide should mention {needle:?}"
        );
    }
}

#[test]
fn guided_setup_dogfood_matrix_documents_setup_control_plane() {
    let root = repo_root();
    let matrix_path = root.join("docs/product/setup-readiness-dogfood-matrix.md");
    let hold_path = root.join("docs/release/0.9.0-release-hold.md");
    let readiness_path = root.join("docs/release/0.9.0-readiness.md");

    let matrix = std::fs::read_to_string(&matrix_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", matrix_path.display()));
    let hold = std::fs::read_to_string(&hold_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", hold_path.display()));
    let readiness = std::fs::read_to_string(&readiness_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", readiness_path.display()));

    assert!(
        hold.contains("setup-readiness dogfood matrix") && readiness.contains("#411"),
        "release hold/readiness docs should link the setup-readiness dogfood matrix without lifting the hold"
    );

    for needle in [
        "0.9 remains paused",
        "does not approve tag, publish",
        "manual `release.yml` dispatch",
        "Setup readiness is a prerequisite signal",
        "source freshness",
        "packet readiness",
        "repair clearance",
        "share posture",
        "shiplog init --guided",
        "shiplog doctor --setup",
        "shiplog sources status",
        "shiplog intake --last-6-months --explain",
        "shiplog repair plan --latest",
        "shiplog journal add --from-repair <repair_id>",
        "shiplog repair diff --latest",
        "shiplog runs diff --latest",
        "shiplog share explain manager --latest",
        "Empty directory",
        "Git repo with no config",
        "Git repo with guided config",
        "Malformed manual journal",
        "Disabled manual source",
        "Enabled GitHub without token",
        "Manager share without redaction key",
        "Public share cautious path",
        "Old config / old report compatibility",
        "missing optional provider tokens",
        "unsupported config versions",
        "old report/packet artifacts do not drive doctor setup state",
        "Windows path/env-var display",
        "Trusted proof",
        "Missing proof",
        "Accepted caveat",
        "Release Proof Map",
        "malformed manual journal routing to doctor before repair writes",
        "manager/public share blocked consistently",
        "doctor --setup --json",
        "without scraping text",
        "not itself the release decision",
        "Current decision: keep the `v0.9.0` hold active (#398, #410, #422, and #440).",
        "owner explicitly approves any release execution",
    ] {
        assert!(
            matrix.contains(needle),
            "guided setup dogfood matrix should mention {needle:?}"
        );
    }
}

#[test]
fn review_loop_status_proposal_defines_receipt_front_panel() {
    let doc_path = repo_root().join("docs/proposals/SHIPLOG-PROP-0006-review-loop-status.md");
    let doc = std::fs::read_to_string(&doc_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", doc_path.display()));

    for needle in [
        "Review Loop Status",
        "Status: implemented in the held 0.9 candidate",
        "Implementation Receipts",
        "human `shiplog status --latest`",
        "shiplog status --latest",
        "shiplog status --latest --json",
        "setup, evidence, repair",
        "diff, and share receipts",
        "What state is my review loop in, and what is the next safe action?",
        "read-only control-plane surface",
        "overall_status",
        "setup_summary",
        "latest_run",
        "packet_readiness",
        "source_summary",
        "repair_summary",
        "diff_summary",
        "share_summary",
        "next_actions[]",
        "blocking_reasons[]",
        "receipt_refs[]",
        "setup blocked -> do not run evidence repair",
        "share blocked -> do not render manager or public packets",
        "setup readiness != evidence quality",
        "repair readiness != share readiness",
        "share explain != share render",
        "status != packet prose",
        "setup readiness model used by `doctor --setup`",
        "`intake.report.json`",
        "repair-plan data derived from `repair_items`",
        "repair-diff report data when available",
        "runs-diff report data when available",
        "share explain / share readiness data",
        "no provider network calls",
        "no writes",
        "no Markdown scraping",
        "no generated performance-review prose",
        "Status must not:",
        "query providers",
        "mutate config",
        "render share artifacts",
        "scrape `packet.md`",
        "call an LLM",
        "dashboard, TUI, GUI",
        "release execution",
        "status reads receipts, not packet prose",
        "0.9 hold active",
    ] {
        assert!(
            doc.contains(needle),
            "review-loop status proposal should mention {needle:?}"
        );
    }
}

#[test]
fn github_activity_harvest_proposal_defines_actor_first_budgeted_workflow() {
    let doc_path = repo_root().join("docs/proposals/SHIPLOG-PROP-0007-github-activity-harvest.md");
    let doc = std::fs::read_to_string(&doc_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", doc_path.display()));

    for needle in [
        "GitHub Activity Harvest",
        "Status: proposed",
        "plan -> scout -> run -> resume -> merge -> report API cost",
        "actor: EffortlessSteven",
        "repo owners: EffortlessMetrics, EffortlessSteven",
        "actor-first search with optional owner filtering and receipts",
        "author:<user>",
        "reviewed-by:<user>",
        "actor = \"EffortlessSteven\"",
        "repo_owners = [\"EffortlessMetrics\", \"EffortlessSteven\"]",
        "user = \"EffortlessSteven\"",
        "compatibility alias",
        "actor search with owner filtering",
        "plan before spending budget",
        "scout before full fidelity",
        "checkpoint before stopping",
        "resume without refetching completed work",
        "report API cost and cache reuse",
        "scout:",
        "authored:",
        "full:",
        "search metadata probes",
        "search result pages",
        "github.activity.plan.json",
        "github.activity.progress.json",
        "github.activity.api-ledger.json",
        "API Budget",
        "search_requests",
        "core_requests",
        "cache_hits",
        "secondary_limit_events",
        "Authenticated REST API requests generally count against a 5,000 requests/hour",
        "authenticated search",
        "30 requests/minute",
        "up to 1,000 results",
        "incomplete_results",
        "x-ratelimit-remaining",
        "x-ratelimit-reset",
        "retry-after",
        "repository crawling",
        "No token values should ever appear",
        "SHIPLOG-ADR-0010-github-harvest-is-actor-first-owner-filtered",
        "does not approve",
        "`v0.9.0` release",
    ] {
        assert!(
            doc.contains(needle),
            "GitHub activity harvest proposal should mention {needle:?}"
        );
    }
}

#[test]
fn github_activity_harvest_spec_defines_plan_progress_and_api_ledger_contracts() {
    let root = repo_root();
    let proposal_path = root.join("docs/proposals/SHIPLOG-PROP-0007-github-activity-harvest.md");
    let spec_path = root.join("docs/specs/SHIPLOG-SPEC-0009-github-activity-harvest.md");
    let adr_path =
        root.join("docs/adr/SHIPLOG-ADR-0010-github-harvest-is-actor-first-owner-filtered.md");

    let proposal = std::fs::read_to_string(&proposal_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", proposal_path.display()));
    let spec = std::fs::read_to_string(&spec_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", spec_path.display()));
    let adr = std::fs::read_to_string(&adr_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", adr_path.display()));

    for needle in [
        "SHIPLOG-SPEC-0009: GitHub Activity Harvest",
        "Status: proposed",
        "SHIPLOG-PROP-0007-github-activity-harvest",
        "github.activity.plan.json",
        "github.activity.progress.json",
        "github.activity.api-ledger.json",
        "plan -> scout -> run -> resume -> merge -> report API cost",
        "actor-first and owner-filtered",
        "author:<actor>",
        "reviewed-by:<actor>",
        "Activity plan",
        "Activity progress",
        "API cost",
        "shiplog github activity plan",
        "shiplog github activity scout",
        "shiplog github activity run",
        "shiplog github activity status",
        "shiplog github activity report",
        "shiplog github activity merge",
        "sources.github.user",
        "compatibility alias",
        "Profile | Authored PR search | PR details | Review search | Review pages",
        "`scout`",
        "`authored`",
        "`full`",
        "github.activity.plan.v1",
        "github.activity.progress.v1",
        "github.activity.api-ledger.v1",
        "planning_mode = \"static\"",
        "planning_mode = \"probe\"",
        "search_probe",
        "search_page",
        "pull_detail",
        "review_page",
        "checkpoint_and_stop",
        "owner_not_requested",
        "x-ratelimit-remaining",
        "retry-after",
        "secondary_limit_events",
        "token values",
        "does not authorize tag",
    ] {
        assert!(
            spec.contains(needle),
            "GitHub activity harvest spec should mention {needle:?}"
        );
    }

    for boundary in [
        "No provider mutation",
        "No packet Markdown scraping",
        "No release execution",
        "does not add behavior by itself",
        "must not crawl every repository",
        "must not scrape `packet.md`",
        "must not fetch PR details",
        "must not fetch review pages",
        "must not render packets",
        "must not retry forever",
    ] {
        assert!(
            spec.contains(boundary),
            "GitHub activity harvest spec should preserve boundary {boundary:?}"
        );
    }

    assert!(
        proposal.contains("SHIPLOG-SPEC-0009-github-activity-harvest.md"),
        "proposal should link the follow-up GitHub activity harvest spec"
    );
    assert!(
        spec.contains("SHIPLOG-PROP-0007-github-activity-harvest"),
        "spec should link the GitHub activity harvest proposal"
    );
    assert!(
        proposal.contains("SHIPLOG-ADR-0010-github-harvest-is-actor-first-owner-filtered.md"),
        "proposal should link the GitHub activity harvest ADR"
    );
    assert!(
        spec.contains("SHIPLOG-ADR-0010-github-harvest-is-actor-first-owner-filtered.md"),
        "spec should link the GitHub activity harvest ADR"
    );
    assert!(
        adr.contains("SHIPLOG-SPEC-0009-github-activity-harvest"),
        "ADR should link the GitHub activity harvest spec"
    );
}

#[test]
fn github_activity_harvest_guide_documents_current_receipt_workflow() {
    let doc_path = repo_root().join("docs/guides/github-activity-harvest.md");
    let doc = std::fs::read_to_string(&doc_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", doc_path.display()));

    for needle in [
        "GitHub activity harvest guide",
        "plan -> scout -> authored -> full -> inspect receipts",
        "actor-first and owner-filtered",
        "shiplog config validate --config shiplog-github-full.toml",
        "shiplog doctor --config shiplog-github-full.toml --setup",
        "shiplog sources status --config shiplog-github-full.toml",
        "shiplog github activity plan --config shiplog-github-full.toml",
        "shiplog github activity scout --config shiplog-github-full.toml --resume",
        "shiplog github activity run --config shiplog-github-full.toml --profile authored --resume",
        "shiplog github activity run --config shiplog-github-full.toml --profile full --resume",
        "github.activity.plan.json",
        "github.activity.progress.json",
        "github.activity.api-ledger.json",
        "shiplog status --out ./out/github-full --latest",
        "shiplog runs list --out ./out/github-full",
        "shiplog open intake-report --out ./out/github-full --latest",
        "shiplog open packet --out ./out/github-full --latest",
        "What is not landed yet",
        "shiplog github activity status",
        "shiplog github activity report",
        "shiplog github activity merge",
        "store token values",
        "execute release work",
    ] {
        assert!(
            doc.contains(needle),
            "GitHub activity harvest guide should mention {needle:?}"
        );
    }
}

#[test]
fn github_activity_harvest_adr_records_actor_first_owner_filtered_decision() {
    let adr_path = repo_root()
        .join("docs/adr/SHIPLOG-ADR-0010-github-harvest-is-actor-first-owner-filtered.md");
    let adr = std::fs::read_to_string(&adr_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", adr_path.display()));

    for needle in [
        "SHIPLOG-ADR-0010: GitHub Harvest Is Actor-First, Owner-Filtered",
        "Status: accepted",
        "SHIPLOG-PROP-0007-github-activity-harvest",
        "SHIPLOG-SPEC-0009-github-activity-harvest",
        "GitHub activity harvest is actor-first and owner-filtered",
        "author:<actor>",
        "reviewed-by:<actor>",
        "sources.github.user",
        "compatibility alias",
        "`repo_owners` is an optional inclusion scope",
        "owner filtering is a receipt-backed inclusion",
        "actor_search_owner_filter",
        "which repository owners were kept",
        "which repository owners were dropped",
        "owner_not_requested",
        "must not crawl every repository",
        "plan -> scout -> run -> resume -> merge -> report API cost",
        "github.activity.plan.json",
        "github.activity.progress.json",
        "github.activity.api-ledger.json",
        "One GitHub Source Per Repository Owner",
        "Crawl Every Repository Under Each Owner",
        "Encode Owner Scope Only In GitHub Search Syntax",
        "Infer Owner Coverage From Token Permissions",
        "github.activity.plan.v1",
        "github.activity.progress.v1",
        "github.activity.api-ledger.v1",
        "without token values",
        "does not add runtime behavior by itself",
        "does not lift the `v0.9.0` release hold",
    ] {
        assert!(
            adr.contains(needle),
            "GitHub activity harvest ADR should mention {needle:?}"
        );
    }
}

#[test]
fn review_loop_status_spec_defines_receipt_contract() {
    let root = repo_root();
    let spec_path = root.join("docs/specs/SHIPLOG-SPEC-0008-review-loop-status.md");
    let proposal_path = root.join("docs/proposals/SHIPLOG-PROP-0006-review-loop-status.md");
    let adr_path = root.join("docs/adr/SHIPLOG-ADR-0009-status-reads-receipts-not-packet-prose.md");

    let spec = std::fs::read_to_string(&spec_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", spec_path.display()));
    let proposal = std::fs::read_to_string(&proposal_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", proposal_path.display()));
    let adr = std::fs::read_to_string(&adr_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", adr_path.display()));

    for needle in [
        "SHIPLOG-SPEC-0008: Review Loop Status",
        "Status: implemented in the held 0.9 candidate",
        "Implementation note",
        "landed through #429-#436",
        "SHIPLOG-PROP-0006-review-loop-status",
        "shiplog status --latest",
        "shiplog status --latest --json",
        "read-only projection",
        "setup_summary",
        "latest_run",
        "packet_readiness",
        "source_summary",
        "repair_summary",
        "diff_summary",
        "share_summary",
        "next_actions[]",
        "blocking_reasons[]",
        "receipt_refs[]",
        "Required fields for `next_actions[]`:",
        "preconditions[]",
        "priority",
        "unknown",
        "needs_setup",
        "ready_to_collect",
        "needs_evidence",
        "needs_repair",
        "repair_in_progress",
        "ready_with_caveats",
        "ready_to_explain_share",
        "share_blocked",
        "ready_to_share",
        "blocked",
        "Triggering receipt condition",
        "setup readiness != evidence quality",
        "repair readiness != share readiness",
        "share explain != share render",
        "status != packet prose",
        "Status must not scrape `packet.md`",
        "no status path reads `packet.md` as machine truth",
        "No provider probing",
        "No config mutation",
        "No implicit intake rerun",
        "No automatic repair",
        "No share rendering",
        "No packet Markdown scraping",
        "No LLM calls",
        "No generated performance-review prose",
        "No release execution",
        "does not approve a release",
    ] {
        assert!(
            spec.contains(needle),
            "review-loop status spec should mention {needle:?}"
        );
    }

    for receipt in [
        "setup model",
        "intake.report.json",
        "repair diff receipts",
        "runs diff receipts",
        "share manifests",
        "share readiness receipts",
    ] {
        assert!(
            spec.contains(receipt),
            "review-loop status spec should name receipt source {receipt:?}"
        );
    }

    assert!(
        proposal.contains("SHIPLOG-SPEC-0008-review-loop-status.md"),
        "proposal should link the follow-up status spec"
    );
    assert!(
        spec.contains("SHIPLOG-ADR-0009-status-reads-receipts-not-packet-prose.md"),
        "spec should link the follow-up status ADR"
    );
    assert!(
        proposal.contains("SHIPLOG-ADR-0009-status-reads-receipts-not-packet-prose.md"),
        "proposal should link the follow-up status ADR"
    );
    assert!(
        adr.contains("SHIPLOG-SPEC-0008-review-loop-status.md"),
        "ADR should link the review-loop status spec"
    );
}

#[test]
fn review_loop_status_adr_keeps_status_receipt_derived() {
    let adr_path =
        repo_root().join("docs/adr/SHIPLOG-ADR-0009-status-reads-receipts-not-packet-prose.md");
    let adr = std::fs::read_to_string(&adr_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", adr_path.display()));

    for needle in [
        "Status Reads Receipts, Not Packet Prose",
        "Implementation Note",
        "#429-#436",
        "shiplog status",
        "shiplog status --latest",
        "shiplog status --latest --json",
        "typed models and durable machine receipts only",
        "setup readiness model",
        "intake.report.json",
        "repair diff receipts",
        "runs diff receipts",
        "share explain, share verify, and share readiness receipts",
        "bundle and share manifests",
        "setup readiness != evidence quality",
        "evidence quality != repair readiness",
        "repair readiness != share readiness",
        "share explain != share render",
        "status != packet prose",
        "must not:",
        "scrape `packet.md`",
        "query GitHub, GitLab, Jira, Linear",
        "rerun intake implicitly",
        "mutate config",
        "run repair commands automatically",
        "render manager or public share artifacts",
        "call an LLM",
        "generate performance-review prose",
        "disclose secret values",
        "does not authorize tagging",
        "release creation, workflow dispatch",
    ] {
        assert!(
            adr.contains(needle),
            "review-loop status ADR should mention {needle:?}"
        );
    }
}

#[test]
fn release_hold_guard_blocks_held_0_9_tag() {
    let root = repo_root();
    let workflow_path = root.join(".github/workflows/release.yml");
    let guard_path = root.join("scripts/check-release-hold.sh");
    let hold_path = root.join("docs/release/0.9.0-release-hold.md");
    let readiness_path = root.join("docs/release/0.9.0-readiness.md");
    let process_allowlist_path = root.join("policy/process-allowlist.toml");

    let workflow = std::fs::read_to_string(&workflow_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", workflow_path.display()));
    let guard = std::fs::read_to_string(&guard_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", guard_path.display()));
    let hold = std::fs::read_to_string(&hold_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", hold_path.display()));
    let readiness = std::fs::read_to_string(&readiness_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", readiness_path.display()));
    let process_allowlist = std::fs::read_to_string(&process_allowlist_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", process_allowlist_path.display()));

    assert!(
        workflow.contains("bash scripts/check-release-hold.sh"),
        "release workflow should run the release-hold guard before release proof"
    );
    assert!(
        workflow.contains("release_tag:")
            && workflow.contains("owner_approved_release_execution")
            && workflow.contains("steps.release_tag.outputs.release_tag"),
        "manual release workflow dispatch should require an explicit tag and owner approval"
    );
    let resolve_tag_index = workflow
        .find("Resolve release tag")
        .expect("release workflow should resolve the release tag");
    let hold_guard_index = workflow
        .find("Release hold guard")
        .expect("release workflow should run the release-hold guard");
    let publish_dry_run_index = workflow
        .find("scripts/publish-dry-run.sh")
        .expect("release workflow should keep publish dry-run proof explicit");
    assert!(
        resolve_tag_index < hold_guard_index && hold_guard_index < publish_dry_run_index,
        "release workflow should resolve/approve the tag and run the hold guard before publish dry-run proof"
    );
    assert!(
        !workflow.contains("needs: [build-binary, create-release]"),
        "release workflow jobs after artifact build should depend on release-preflight so they cannot bypass the hold guard"
    );
    assert!(
        workflow.matches("needs: [release-preflight").count() >= 4,
        "release workflow should route all release-producing and release-test jobs through release-preflight"
    );
    assert!(
        guard.contains("requires an explicit release tag")
            && guard.contains("requires a semver release tag"),
        "release-hold guard should reject missing or branch-like release refs"
    );
    assert!(
        hold.contains("scripts/check-release-hold.sh")
            && readiness.contains("scripts/check-release-hold.sh"),
        "release hold/readiness docs should name the workflow guard"
    );
    assert!(
        process_allowlist.contains("proc-bash-check-release-hold"),
        "release-hold workflow process should be receipted in the process allowlist"
    );

    if cfg!(windows) {
        eprintln!(
            "skipping release hold guard execution on Windows; workflow runs this guard on Ubuntu"
        );
        return;
    }

    if StdCommand::new("bash").arg("--version").output().is_err() {
        eprintln!("skipping release hold guard execution: bash not available");
        return;
    }

    let blocked = StdCommand::new("bash")
        .current_dir(&root)
        .arg("scripts/check-release-hold.sh")
        .arg("v0.9.0")
        .output()
        .expect("run release hold guard for v0.9.0");
    assert!(
        !blocked.status.success(),
        "release hold guard should block v0.9.0 while hold receipt exists"
    );
    let blocked_stderr = String::from_utf8_lossy(&blocked.stderr);
    assert!(
        blocked_stderr.contains("release hold blocks v0.9.0"),
        "blocked stderr should explain the held release tag. stderr:\n{blocked_stderr}"
    );

    let missing_tag = StdCommand::new("bash")
        .current_dir(&root)
        .env_remove("GITHUB_REF")
        .env_remove("GITHUB_REF_NAME")
        .arg("scripts/check-release-hold.sh")
        .output()
        .expect("run release hold guard without a tag");
    assert!(
        !missing_tag.status.success(),
        "release hold guard should reject missing release tag input"
    );
    let missing_tag_stderr = String::from_utf8_lossy(&missing_tag.stderr);
    assert!(
        missing_tag_stderr.contains("requires an explicit release tag"),
        "missing-tag stderr should explain explicit release tag requirement. stderr:\n{missing_tag_stderr}"
    );

    let branch_ref = StdCommand::new("bash")
        .current_dir(&root)
        .env_remove("GITHUB_REF")
        .env_remove("GITHUB_REF_NAME")
        .arg("scripts/check-release-hold.sh")
        .arg("main")
        .output()
        .expect("run release hold guard for a branch ref");
    assert!(
        !branch_ref.status.success(),
        "release hold guard should reject branch refs before release proof"
    );
    let branch_ref_stderr = String::from_utf8_lossy(&branch_ref.stderr);
    assert!(
        branch_ref_stderr.contains("requires a semver release tag"),
        "branch-ref stderr should explain semver release tag requirement. stderr:\n{branch_ref_stderr}"
    );

    let allowed = StdCommand::new("bash")
        .current_dir(&root)
        .arg("scripts/check-release-hold.sh")
        .arg("v0.8.0")
        .output()
        .expect("run release hold guard for v0.8.0");
    assert!(
        allowed.status.success(),
        "release hold guard should allow tags outside the active 0.9 hold"
    );
}

#[test]
fn release_hold_docs_record_post_0_8_soak_receipts() {
    let root = repo_root();
    let hold_path = root.join("docs/release/0.9.0-release-hold.md");
    let readiness_path = root.join("docs/release/0.9.0-readiness.md");

    let hold = std::fs::read_to_string(&hold_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", hold_path.display()));
    let readiness = std::fs::read_to_string(&readiness_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", readiness_path.display()));

    assert!(
        hold.contains("## Soak Receipts"),
        "release hold receipt should summarize post-0.8 soak hardening"
    );
    assert!(
        readiness.contains("## Post-0.8 Soak Evidence"),
        "paused readiness ledger should record post-0.8 soak evidence"
    );

    for needle in [
        "#337", "#338", "#339", "#340", "#341", "#342", "#343", "#344", "#345", "#346", "#347",
        "#348", "#349", "#350", "#351", "#352", "#357", "#364", "#365", "#367", "#369", "#370",
        "#371", "#372", "#373", "#374", "#375", "#376", "#377", "#378", "#379", "#380", "#381",
        "#382", "#383", "#384", "#385", "#386", "#387", "#388", "#389", "#390", "#391", "#392",
        "#393", "#394", "#395", "#396", "#397", "#398", "#399", "#400", "#401", "#402", "#403",
        "#404", "#405", "#406", "#407", "#408", "#409", "#410", "#411", "#412", "#413", "#414",
        "#415", "#416", "#417", "#418", "#419", "#420", "#421", "#422", "#424", "#425", "#426",
        "#427", "#428", "#429", "#430", "#431", "#432", "#433", "#434", "#435", "#436", "#437",
        "#438", "#439", "#440",
    ] {
        assert!(
            hold.contains(needle) && readiness.contains(needle),
            "hold and readiness docs should both record {needle}"
        );
    }

    for needle in [
        "do not lift the release hold",
        "not release approval",
        "shiplog open packet --latest --print-path",
        "shiplog repair diff --latest",
        "shiplog share explain manager",
        "journal suggestions",
        "journal repair items",
        "Windows-native redaction-key setup",
        "repair diff",
        "Consolidated Late-Soak Ledger",
        "Path display",
        "Unsafe manual repair handoffs",
        "Share posture caveats",
        "Evidence-debt noise",
        "Zero-event evidence claims",
        "Read-first handoffs",
        "Skipped-source visibility",
        "Repair-diff truthfulness",
        "Repaired rerun handoff",
        "RIPR",
        "non-release-blocking",
        "Intake status",
        "Packet readiness",
        "packet_quality",
        "single window date range",
        "direct manager/public share render commands",
        "intake.report.md",
        "explain posture before rendering",
        "Top Fixups",
        "JSON contract",
        "crates.io README",
        "quality-diff",
        "current-directory duplication",
        "slash-separated artifact paths",
        "configured manual journal",
        "Needs review",
        "outcome-context evidence debt",
        "zero-event sources",
        "journal add --from-repair",
        "write-producing commands",
        "rapid first-intake guide",
        "top-level README",
        "review-ready dogfood matrix",
        "trusted-vs-needs-exercise ledger",
        "legacy",
        "report quality gaps",
        "packet-readiness improvement",
        "still-open source repairs",
        "journal-only repair rerun",
        "report/packet consistency",
        "rendered Markdown",
        "partial `packet_quality`",
        "richer review-ready signals",
        "manual release workflow dispatch",
        "owner approval",
        "semver tag",
        "before publish dry-run proof can run",
        "review-ready loop transcript",
        "fail-closed manager verification",
        "release decision",
        "keep the 0.9 hold active",
        "owner approval and final release preflight are not present",
        "Guided Setup / Doctor",
        "setup readiness",
        "shiplog doctor --setup",
        "shiplog sources status",
        "shiplog init --guided",
        "unreleased candidate scope",
        "Review-loop Status",
        "shiplog status --latest",
        "shiplog status --latest --json",
        "review-loop-status.v1",
        "safe next actions",
        "review-loop status dogfood transcript",
        "recurring",
        "review-loop guide",
        "status as the review-loop",
        "crates.io README into an install-and-first-loop page",
        "Review-loop cockpit",
        "Release-facing communication",
        "Candidate value; not release approval",
        "Review Loop Cockpit Readiness",
    ] {
        assert!(
            hold.contains(needle) || readiness.contains(needle),
            "soak evidence docs should mention {needle:?}"
        );
    }
}

#[test]
fn release_decision_keeps_0_9_hold_without_execution() {
    let root = repo_root();
    let decision_path = root.join("docs/release/0.9.0-release-decision.md");
    let hold_path = root.join("docs/release/0.9.0-release-hold.md");
    let readiness_path = root.join("docs/release/0.9.0-readiness.md");
    let matrix_path = root.join("docs/product/review-ready-dogfood-matrix.md");

    let decision = std::fs::read_to_string(&decision_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", decision_path.display()));
    let hold = std::fs::read_to_string(&hold_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", hold_path.display()));
    let readiness = std::fs::read_to_string(&readiness_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", readiness_path.display()));
    let matrix = std::fs::read_to_string(&matrix_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", matrix_path.display()));

    for needle in [
        "**Decision:** keep hold",
        "Guided Setup / Doctor",
        "Review-loop Status",
        "Do not tag, publish to crates.io, create a GitHub release",
        "dispatch the release",
        "A. Keep hold after review-loop status",
        "B. Resume 0.9 release after final preflight",
        "C. Split Guided Setup / Doctor or Review-loop Status to 0.10",
        "#390",
        "#397",
        "#409",
        "#421",
        "#422",
        "#424",
        "#425",
        "#426",
        "#427",
        "#428",
        "#429",
        "#430",
        "#431",
        "#432",
        "#433",
        "#434",
        "#436",
        "#437",
        "#438",
        "#439",
        "#440",
        "owner approval and final release preflight are still missing",
        "post-status release decision",
        "feature-complete enough to consider a release-resume PR",
        "Owner explicitly approves release execution",
        "not met",
        "crates.io latest remains `shiplog = \"0.8.0\"`",
        "scripts/check-release-hold.sh",
        "owner_approved_release_execution",
        "final release preflight has not been rerun",
        "candidate is represented by review-ready packet quality,",
        "Review-loop Status, and",
        "release-facing docs",
        "Do not extend Guided Setup / Doctor or Review-loop Status just to make 0.9",
    ] {
        assert!(
            decision.contains(needle),
            "release decision should mention {needle:?}"
        );
    }

    assert!(
        hold.contains("0.9.0-release-decision.md")
            && readiness.contains("0.9.0-release-decision.md")
            && matrix.contains("0.9.0-release-decision.md")
            && matrix.contains(
                "Current decision: keep the `v0.9.0` hold active (#398, #410, #422, and #440)."
            ),
        "hold, readiness, and matrix docs should link the release decision without lifting the hold"
    );
}

#[test]
fn review_ready_loop_transcript_records_final_dogfood() {
    let root = repo_root();
    let transcript_path = root.join("docs/product/review-ready-loop-transcript.md");
    let matrix_path = root.join("docs/product/review-ready-dogfood-matrix.md");
    let hold_path = root.join("docs/release/0.9.0-release-hold.md");
    let readiness_path = root.join("docs/release/0.9.0-readiness.md");

    let transcript = std::fs::read_to_string(&transcript_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", transcript_path.display()));
    let matrix = std::fs::read_to_string(&matrix_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", matrix_path.display()));
    let hold = std::fs::read_to_string(&hold_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", hold_path.display()));
    let readiness = std::fs::read_to_string(&readiness_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", readiness_path.display()));

    for needle in [
        "post-0.8 soak receipt",
        "does not approve tagging, publishing",
        "sources.git.enabled = true",
        "shiplog intake --config",
        "shiplog repair plan --out",
        "shiplog journal add --from-repair",
        "shiplog repair diff --out",
        "shiplog runs diff --out",
        "shiplog open packet --out",
        "shiplog share explain manager --out",
        "shiplog share verify manager --out",
        "manual:manual_evidence_missing:fixup_manual_context_shiplog",
        "manual evidence count: 0 -> 1",
        "packet_readiness = ready_with_caveats",
        "evidence_strength = partial",
        "one claim candidate",
        "Profile packet and share manifest were not written",
        "manager share requires --redact-key or SHIPLOG_REDACT_KEY",
        "Did not generate performance-review prose",
        "explicit owner approval",
        "temporary dogfood workspace",
    ] {
        assert!(
            transcript.contains(needle),
            "review-ready loop transcript should mention {needle:?}"
        );
    }

    assert!(
        matrix.contains("review-ready loop transcript")
            && hold.contains("review-ready loop transcript")
            && readiness.contains("#397"),
        "matrix and release docs should link the final dogfood transcript without lifting the hold"
    );
}

#[test]
fn review_loop_status_transcript_records_status_cockpit_dogfood() {
    let root = repo_root();
    let transcript_path = root.join("docs/product/review-loop-status-transcript.md");
    let spec_path = root.join("docs/specs/SHIPLOG-SPEC-0008-review-loop-status.md");
    let changelog_path = root.join("CHANGELOG.md");

    let transcript = std::fs::read_to_string(&transcript_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", transcript_path.display()));
    let spec = std::fs::read_to_string(&spec_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", spec_path.display()));
    let changelog = std::fs::read_to_string(&changelog_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", changelog_path.display()));

    for needle in [
        "Review-loop status dogfood transcript",
        "Release Proof Map",
        "shiplog status --out",
        "ready_to_collect",
        "needs_repair",
        "repair_in_progress",
        "manual evidence count 0 -> 1",
        "File manifest was unchanged by `status`",
        "No manager profile packet was written",
        "does not approve",
        "`v0.9.0` release execution",
        "temporary dogfood workspace",
    ] {
        assert!(
            transcript.contains(needle),
            "review-loop status transcript should mention {needle:?}"
        );
    }

    assert!(
        spec.contains("docs/product/review-loop-status-transcript.md")
            && spec.contains("after rerun/diff")
            && changelog.contains("#434"),
        "spec and changelog should link the status transcript without lifting the hold"
    );
}

#[test]
fn review_ready_dogfood_matrix_documents_soak_flows() {
    let root = repo_root();
    let matrix_path = root.join("docs/product/review-ready-dogfood-matrix.md");
    let hold_path = root.join("docs/release/0.9.0-release-hold.md");
    let readiness_path = root.join("docs/release/0.9.0-readiness.md");

    let matrix = std::fs::read_to_string(&matrix_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", matrix_path.display()));
    let hold = std::fs::read_to_string(&hold_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", hold_path.display()));
    let readiness = std::fs::read_to_string(&readiness_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", readiness_path.display()));

    assert!(
        hold.contains("review-ready dogfood matrix") && readiness.contains("#391"),
        "release hold/readiness docs should link the dogfood matrix without lifting the hold"
    );

    for needle in [
        "0.9 remains paused",
        "does not approve tag, publish",
        "GitHub release execution",
        "git tag v0.9.0",
        "cargo publish -p shiplog --locked",
        "shiplog intake --last-6-months --explain",
        "shiplog repair plan --latest",
        "shiplog journal add --from-repair <repair_id>",
        "shiplog repair diff --latest",
        "shiplog runs diff --latest",
        "shiplog share explain manager --latest",
        "shiplog share verify manager --latest",
        "Empty directory, no tokens",
        "Local git plus empty valid manual journal",
        "Local git plus malformed manual journal",
        "Repaired manual-only packet",
        "Skipped provider sources",
        "Old report without `packet_quality`",
        "Manager `share explain` without key",
        "Public share explain/verify path",
        "Public `share explain` now has targeted proof",
        "Release Proof Map",
        "repaired manual-only packet",
        "skipped provider sources staying visible",
        "old reports degrading without invented readiness",
        "packet debt, strict-review caveats",
        "`share explain` and `runs diff` now avoid inventing",
        "reports without `packet_quality`",
        "hand off to `repair diff` before planning again",
        "no-write surfaces remain no-write",
        "repair diff cannot clear provider repair items without provider evidence",
        "not itself the release decision",
        "owner explicitly approves release execution",
    ] {
        assert!(
            matrix.contains(needle),
            "review-ready dogfood matrix should mention {needle:?}"
        );
    }
}

#[test]
fn ci_docs_keep_ripr_advisory_and_non_release_blocking() {
    let root = repo_root();
    let lanes_path = root.join("docs/ci/test-evidence-lanes.md");
    let ripr_path = root.join("docs/ci/ripr.md");
    let workflow_path = root.join(".github/workflows/ripr.yml");

    let lanes = std::fs::read_to_string(&lanes_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", lanes_path.display()));
    let ripr = std::fs::read_to_string(&ripr_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", ripr_path.display()));
    let workflow = std::fs::read_to_string(&workflow_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", workflow_path.display()));

    for needle in [
        "advisory stub on Rust diffs; never release-blocking",
        "real analysis remains a follow-up release",
        "does not block",
        "merges or releases",
        "Hard enforcement of lane budgets remains",
        "release decision",
        "## How `ripr` fits today",
    ] {
        assert!(
            lanes.contains(needle),
            "test evidence lanes doc should keep ripr advisory posture current: {needle:?}"
        );
    }

    assert!(
        !lanes.contains("**(planned, PR #153)** `ripr.yml`"),
        "test evidence lanes doc should not describe the landed ripr workflow as planned"
    );
    assert!(
        ripr.contains("Blocking | no (advisory)")
            && workflow.contains("Always advisory: never blocks merge"),
        "ripr operating docs and workflow should keep the lane advisory"
    );
}

#[test]
fn documented_help_commands_stay_available() {
    shiplog_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("intake"))
        .stdout(predicate::str::contains("collect"))
        .stdout(predicate::str::contains("render"))
        .stdout(predicate::str::contains("share"))
        .stdout(predicate::str::contains("workstreams"))
        .stdout(predicate::str::contains("journal"))
        .stdout(predicate::str::contains("cache"))
        .stdout(predicate::str::contains("identify"))
        .stdout(predicate::str::contains("report"))
        .stdout(predicate::str::contains("sources"))
        .stdout(predicate::str::contains("periods"));

    shiplog_cmd()
        .args(["collect", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("multi"))
        .stdout(predicate::str::contains("github"))
        .stdout(predicate::str::contains("gitlab"))
        .stdout(predicate::str::contains("jira"))
        .stdout(predicate::str::contains("linear"))
        .stdout(predicate::str::contains("git"))
        .stdout(predicate::str::contains("json"))
        .stdout(predicate::str::contains("manual"));

    shiplog_cmd()
        .args(["intake", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--last-6-months"))
        .stdout(predicate::str::contains("--source"))
        .stdout(predicate::str::contains("--profile"))
        .stdout(predicate::str::contains("--no-open"))
        .stdout(predicate::str::contains("--explain"));

    shiplog_cmd()
        .args(["init", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--guided"));

    shiplog_cmd()
        .args(["doctor", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--setup"))
        .stdout(predicate::str::contains("--repair-plan"));

    shiplog_cmd()
        .args(["sources", "status", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--config"))
        .stdout(predicate::str::contains("--source"));

    shiplog_cmd()
        .args(["periods", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("explain"));

    shiplog_cmd()
        .args(["runs", "compare", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--from-period"))
        .stdout(predicate::str::contains("--to-period"));

    shiplog_cmd()
        .args(["runs", "diff", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--latest"))
        .stdout(predicate::str::contains("--from"))
        .stdout(predicate::str::contains("--to"));

    shiplog_cmd()
        .args(["render", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--latest"))
        .stdout(predicate::str::contains("--mode"))
        .stdout(predicate::str::contains("--receipt-limit"))
        .stdout(predicate::str::contains("--appendix"))
        .stdout(predicate::str::contains("--redact-key"));

    shiplog_cmd()
        .args(["share", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("manager"))
        .stdout(predicate::str::contains("public"))
        .stdout(predicate::str::contains("explain"));

    shiplog_cmd()
        .args(["share", "manager", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--latest"))
        .stdout(predicate::str::contains("--redact-key"))
        .stdout(predicate::str::contains("--zip"));

    shiplog_cmd()
        .args(["share", "explain", "manager", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--latest"))
        .stdout(predicate::str::contains("--redact-key"));

    shiplog_cmd()
        .args(["share", "verify", "manager", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--latest"))
        .stdout(predicate::str::contains("--redact-key"))
        .stdout(predicate::str::contains("--strict"));

    shiplog_cmd()
        .args(["share", "verify", "manifest", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--latest"))
        .stdout(predicate::str::contains("--profile"));

    shiplog_cmd()
        .args(["review", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("weekly"))
        .stdout(predicate::str::contains("--latest"))
        .stdout(predicate::str::contains("--run"))
        .stdout(predicate::str::contains("--strict"));

    shiplog_cmd()
        .args(["review", "weekly", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--latest"))
        .stdout(predicate::str::contains("--run"))
        .stdout(predicate::str::contains("--strict"));

    shiplog_cmd()
        .args(["review", "fixups", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--latest"))
        .stdout(predicate::str::contains("--run"))
        .stdout(predicate::str::contains("--commands-only"))
        .stdout(predicate::str::contains("--journal-template"));

    shiplog_cmd()
        .args(["open", "intake-report", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--latest"))
        .stdout(predicate::str::contains("--run"))
        .stdout(predicate::str::contains("--print-path"));

    shiplog_cmd()
        .args(["report", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("validate"))
        .stdout(predicate::str::contains("summarize"))
        .stdout(predicate::str::contains("export-agent-pack"));

    shiplog_cmd()
        .args(["report", "validate", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--latest"))
        .stdout(predicate::str::contains("--run"))
        .stdout(predicate::str::contains("--path"));

    shiplog_cmd()
        .args(["report", "summarize", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--latest"))
        .stdout(predicate::str::contains("--run"))
        .stdout(predicate::str::contains("--path"));

    shiplog_cmd()
        .args(["report", "export-agent-pack", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--latest"))
        .stdout(predicate::str::contains("--run"))
        .stdout(predicate::str::contains("--path"))
        .stdout(predicate::str::contains("--output"));

    shiplog_cmd()
        .args(["workstreams", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("rename"))
        .stdout(predicate::str::contains("move"))
        .stdout(predicate::str::contains("split"))
        .stdout(predicate::str::contains("receipts"));

    shiplog_cmd()
        .args(["journal", "add", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--date"))
        .stdout(predicate::str::contains("--title"))
        .stdout(predicate::str::contains("--workstream"))
        .stdout(predicate::str::contains("--receipt"));

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
        .stdout(predicate::str::contains("--impact"));

    shiplog_cmd()
        .args(["cache", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("stats"))
        .stdout(predicate::str::contains("inspect"))
        .stdout(predicate::str::contains("clean"));

    shiplog_cmd()
        .args(["identify", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("jira"))
        .stdout(predicate::str::contains("linear"));

    shiplog_cmd()
        .args(["identify", "jira", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--instance"))
        .stdout(predicate::str::contains("--auth-user"))
        .stdout(predicate::str::contains("JIRA_TOKEN"));

    shiplog_cmd()
        .args(["identify", "linear", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--api-key"))
        .stdout(predicate::str::contains("LINEAR_API_KEY"));
}

#[test]
fn review_cycle_fixture_commands_execute_without_network() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");
    let local_config = example_config("local-git-json-manual.toml");

    shiplog_cmd()
        .current_dir(repo_root())
        .args([
            "init",
            "--source",
            "json",
            "--source",
            "manual",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Would write shiplog.toml"));

    let journal_path = tmp.path().join("manual_events.yaml");
    shiplog_cmd()
        .args([
            "journal",
            "add",
            "--events",
            journal_path.to_str().unwrap(),
            "--date",
            "2026-05-08",
            "--title",
            "Documented fixture rehearsal",
            "--workstream",
            "Docs",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Would add manual event"));

    shiplog_cmd()
        .current_dir(repo_root())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args([
            "config",
            "validate",
            "--config",
            local_config.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Config valid"));

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
            local_config.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Enabled sources:"));

    shiplog_cmd()
        .current_dir(repo_root())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args(["doctor", "--config", local_config.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Config: ok"))
        .stdout(predicate::str::contains("Manual: ok"));

    shiplog_cmd()
        .current_dir(repo_root())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args([
            "collect",
            "--out",
            out.to_str().unwrap(),
            "multi",
            "--config",
            local_config.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Collected configured sources:"))
        .stdout(predicate::str::contains("- json: success"))
        .stdout(predicate::str::contains("- manual: success"))
        .stdout(predicate::str::contains("Merged and wrote:"));

    let intake_out = tmp.path().join("intake-out");
    shiplog_cmd()
        .current_dir(repo_root())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args([
            "intake",
            "--out",
            intake_out.to_str().unwrap(),
            "--config",
            local_config.to_str().unwrap(),
            "--no-open",
            "--explain",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Review intake complete."))
        .stdout(predicate::str::contains("Source decisions:"))
        .stdout(predicate::str::contains("intake.report.md"))
        .stdout(predicate::str::contains("Evidence debt:"))
        .stdout(predicate::str::contains("Open later:"));

    shiplog_cmd()
        .args([
            "open",
            "intake-report",
            "--out",
            intake_out.to_str().unwrap(),
            "--latest",
            "--print-path",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("intake.report.md"));

    shiplog_cmd()
        .args([
            "review",
            "fixups",
            "--out",
            intake_out.to_str().unwrap(),
            "--latest",
            "--commands-only",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("shiplog"));

    shiplog_cmd()
        .args([
            "share",
            "verify",
            "manager",
            "--out",
            intake_out.to_str().unwrap(),
            "--latest",
            "--redact-key",
            "fixture-key",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Share verify: manager"))
        .stdout(predicate::str::contains(
            "Result: ready to render manager share output",
        ));

    shiplog_cmd()
        .args(["runs", "list", "--out", out.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Count: 1"))
        .stdout(predicate::str::contains("events:"));

    shiplog_cmd()
        .args([
            "runs",
            "show",
            "--out",
            out.to_str().unwrap(),
            "--run",
            "latest",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Run:"))
        .stdout(predicate::str::contains("Sources:"));

    shiplog_cmd()
        .args([
            "runs",
            "compare",
            "--out",
            out.to_str().unwrap(),
            "--from",
            "latest",
            "--to",
            "latest",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Compare:"))
        .stdout(predicate::str::contains("Events:"));

    shiplog_cmd()
        .args([
            "review",
            "weekly",
            "--out",
            out.to_str().unwrap(),
            "--latest",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Weekly review:"))
        .stdout(predicate::str::contains("New evidence:"))
        .stdout(predicate::str::contains("Evidence debt:"));

    shiplog_cmd()
        .args([
            "workstreams",
            "list",
            "--out",
            out.to_str().unwrap(),
            "--run",
            "latest",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Count:"));

    shiplog_cmd()
        .args([
            "workstreams",
            "validate",
            "--out",
            out.to_str().unwrap(),
            "--run",
            "latest",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Workstreams valid"));

    shiplog_cmd()
        .args([
            "render",
            "--out",
            out.to_str().unwrap(),
            "--latest",
            "--mode",
            "scaffold",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Rendered from existing events"));

    shiplog_cmd()
        .args([
            "render",
            "--out",
            out.to_str().unwrap(),
            "--latest",
            "--receipt-limit",
            "3",
            "--appendix",
            "summary",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Rendered from existing events"));

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
        .stdout(predicate::str::contains("packet.md"));

    shiplog_cmd()
        .args(["cache", "stats", "--out", out.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cache root:"));
}

#[test]
fn share_profile_docs_fail_closed_without_key() {
    let tmp = TempDir::new().unwrap();
    let out = tmp.path().join("out");
    let local_config = example_config("local-git-json-manual.toml");

    shiplog_cmd()
        .current_dir(repo_root())
        .env_remove("GITHUB_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .args([
            "collect",
            "--out",
            out.to_str().unwrap(),
            "multi",
            "--config",
            local_config.to_str().unwrap(),
        ])
        .assert()
        .success();

    shiplog_cmd()
        .env_remove("SHIPLOG_REDACT_KEY")
        .args([
            "render",
            "--out",
            out.to_str().unwrap(),
            "--latest",
            "--bundle-profile",
            "manager",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "manager profile requires --redact-key or SHIPLOG_REDACT_KEY",
        ));
}

#[test]
fn review_deadline_guide_documents_rescue_path() {
    let doc_path = repo_root().join("docs/guides/review-deadline.md");
    let doc = std::fs::read_to_string(&doc_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", doc_path.display()));

    for needle in [
        "shiplog intake --last-6-months --explain",
        "shiplog open intake-report --latest",
        "shiplog review fixups --latest --commands-only",
        "shiplog review fixups --latest --journal-template",
        "shiplog share explain manager --latest",
        "shiplog share verify manager --latest",
        "shiplog share verify public --latest --strict",
        "shiplog share manager --latest --zip",
        "Repair sources",
        "shiplog journal add",
        "shiplog journal list",
        "shiplog journal edit",
        "No-network rehearsal",
        "scripts/demo-review-rescue.sh --out ./out/deadline-fixture",
        "scripts\\demo-review-rescue.ps1",
        "It should not invent impact claims",
    ] {
        assert!(
            doc.contains(needle),
            "review deadline guide should mention {needle:?}"
        );
    }
}
