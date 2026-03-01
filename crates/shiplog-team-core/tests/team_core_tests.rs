//! Integration tests for shiplog-team-core: config loading, parsing, resolution.

use chrono::NaiveDate;
use shiplog_team_core::{TeamConfig, parse_alias_list, parse_csv_list, resolve_team_config};
use std::collections::HashMap;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// parse_csv_list
// ---------------------------------------------------------------------------

#[test]
fn parse_csv_list_basic() {
    assert_eq!(
        parse_csv_list("alice,bob,carol"),
        vec!["alice", "bob", "carol"]
    );
}

#[test]
fn parse_csv_list_trims_whitespace() {
    assert_eq!(parse_csv_list(" alice , bob "), vec!["alice", "bob"]);
}

#[test]
fn parse_csv_list_deduplicates() {
    assert_eq!(parse_csv_list("alice,bob,alice"), vec!["alice", "bob"]);
}

#[test]
fn parse_csv_list_empty_string() {
    let result = parse_csv_list("");
    assert!(result.is_empty());
}

#[test]
fn parse_csv_list_only_commas() {
    let result = parse_csv_list(",,,");
    assert!(result.is_empty());
}

#[test]
fn parse_csv_list_single_item() {
    assert_eq!(parse_csv_list("alice"), vec!["alice"]);
}

#[test]
fn parse_csv_list_trailing_comma() {
    assert_eq!(parse_csv_list("alice,bob,"), vec!["alice", "bob"]);
}

#[test]
fn parse_csv_list_leading_comma() {
    assert_eq!(parse_csv_list(",alice"), vec!["alice"]);
}

// ---------------------------------------------------------------------------
// parse_alias_list
// ---------------------------------------------------------------------------

#[test]
fn parse_alias_list_basic() {
    let aliases =
        parse_alias_list(&["alice=Alice Smith".to_string(), "bob=Bob Jones".to_string()]).unwrap();
    assert_eq!(aliases.get("alice").unwrap(), "Alice Smith");
    assert_eq!(aliases.get("bob").unwrap(), "Bob Jones");
}

#[test]
fn parse_alias_list_empty_input() {
    let aliases = parse_alias_list(&[]).unwrap();
    assert!(aliases.is_empty());
}

#[test]
fn parse_alias_list_rejects_missing_display_name() {
    let result = parse_alias_list(&["alice=".to_string()]);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("display name cannot be empty"),
        "{err_msg}"
    );
}

#[test]
fn parse_alias_list_rejects_missing_member() {
    let result = parse_alias_list(&["=Display".to_string()]);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("expected member=Display Name"),
        "{err_msg}"
    );
}

#[test]
fn parse_alias_list_handles_equals_in_display_name() {
    let aliases = parse_alias_list(&["alice=Alice = Smith".to_string()]).unwrap();
    assert_eq!(aliases.get("alice").unwrap(), "Alice = Smith");
}

#[test]
fn parse_alias_list_trims_whitespace() {
    let aliases = parse_alias_list(&["  alice  =  Alice Smith  ".to_string()]).unwrap();
    assert_eq!(aliases.get("alice").unwrap(), "Alice Smith");
}

// ---------------------------------------------------------------------------
// TeamConfig::normalized_sections
// ---------------------------------------------------------------------------

#[test]
fn normalized_sections_defaults_when_empty() {
    let cfg = TeamConfig::default();
    let sections = cfg.normalized_sections();
    assert_eq!(
        sections,
        vec!["summary", "workstreams", "coverage", "receipts"]
    );
}

#[test]
fn normalized_sections_custom() {
    let cfg = TeamConfig {
        sections: vec!["coverage".into(), "summary".into()],
        ..TeamConfig::default()
    };
    let sections = cfg.normalized_sections();
    assert_eq!(sections, vec!["coverage", "summary"]);
}

#[test]
fn normalized_sections_deduplicates() {
    let cfg = TeamConfig {
        sections: vec!["summary".into(), "summary".into(), "coverage".into()],
        ..TeamConfig::default()
    };
    let sections = cfg.normalized_sections();
    assert_eq!(sections, vec!["summary", "coverage"]);
}

#[test]
fn normalized_sections_lowercases() {
    let cfg = TeamConfig {
        sections: vec!["Summary".into(), "COVERAGE".into()],
        ..TeamConfig::default()
    };
    let sections = cfg.normalized_sections();
    assert_eq!(sections, vec!["summary", "coverage"]);
}

#[test]
fn normalized_sections_skips_empty_and_whitespace_only() {
    let cfg = TeamConfig {
        sections: vec!["".into(), "  ".into(), "summary".into()],
        ..TeamConfig::default()
    };
    let sections = cfg.normalized_sections();
    assert_eq!(sections, vec!["summary"]);
}

#[test]
fn normalized_sections_all_empty_falls_back_to_default() {
    let cfg = TeamConfig {
        sections: vec!["".into(), "  ".into()],
        ..TeamConfig::default()
    };
    let sections = cfg.normalized_sections();
    assert_eq!(
        sections,
        vec!["summary", "workstreams", "coverage", "receipts"]
    );
}

// ---------------------------------------------------------------------------
// TeamConfig::section_enabled
// ---------------------------------------------------------------------------

#[test]
fn section_enabled_default_includes_all() {
    let cfg = TeamConfig::default();
    assert!(cfg.section_enabled("summary"));
    assert!(cfg.section_enabled("workstreams"));
    assert!(cfg.section_enabled("coverage"));
    assert!(cfg.section_enabled("receipts"));
}

#[test]
fn section_enabled_custom_excludes_missing() {
    let cfg = TeamConfig {
        sections: vec!["summary".into()],
        ..TeamConfig::default()
    };
    assert!(cfg.section_enabled("summary"));
    assert!(!cfg.section_enabled("receipts"));
}

// ---------------------------------------------------------------------------
// TeamConfig::load (YAML)
// ---------------------------------------------------------------------------

#[test]
fn load_valid_yaml() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("team.yaml");
    std::fs::write(
        &path,
        r#"
members:
  - alice
  - bob
aliases:
  alice: "Alice Smith"
sections:
  - summary
  - coverage
"#,
    )
    .unwrap();

    let cfg = TeamConfig::load(&path).unwrap();
    assert_eq!(cfg.members, vec!["alice", "bob"]);
    assert_eq!(cfg.aliases.get("alice").unwrap(), "Alice Smith");
    assert_eq!(cfg.sections, vec!["summary", "coverage"]);
}

#[test]
fn load_minimal_yaml() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("team.yaml");
    std::fs::write(&path, "members: []\n").unwrap();
    let cfg = TeamConfig::load(&path).unwrap();
    assert!(cfg.members.is_empty());
    assert!(cfg.aliases.is_empty());
}

#[test]
fn load_nonexistent_file_returns_error() {
    let result = TeamConfig::load(std::path::Path::new("/does/not/exist.yaml"));
    assert!(result.is_err());
}

#[test]
fn load_invalid_yaml_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.yaml");
    std::fs::write(&path, "members: [[[invalid").unwrap();
    let result = TeamConfig::load(&path);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// TeamConfig serialization round-trip
// ---------------------------------------------------------------------------

#[test]
fn team_config_yaml_round_trip() {
    let cfg = TeamConfig {
        members: vec!["alice".into(), "bob".into()],
        aliases: HashMap::from([("alice".into(), "Alice S.".into())]),
        sections: vec!["summary".into()],
        template: Some(PathBuf::from("tmpl.md")),
        since: Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
        until: Some(NaiveDate::from_ymd_opt(2025, 7, 1).unwrap()),
        required_schema_version: Some("0.2.0".into()),
    };
    let yaml = serde_yaml::to_string(&cfg).unwrap();
    let cfg2: TeamConfig = serde_yaml::from_str(&yaml).unwrap();
    assert_eq!(cfg.members, cfg2.members);
    assert_eq!(cfg.aliases, cfg2.aliases);
    assert_eq!(cfg.sections, cfg2.sections);
    assert_eq!(cfg.since, cfg2.since);
    assert_eq!(cfg.until, cfg2.until);
    assert_eq!(cfg.required_schema_version, cfg2.required_schema_version);
}

#[test]
fn team_config_snapshot_yaml() {
    let cfg = TeamConfig {
        members: vec!["alice".into(), "bob".into()],
        aliases: HashMap::from([("alice".into(), "Alice S.".into())]),
        sections: vec!["summary".into(), "coverage".into()],
        template: None,
        since: Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
        until: Some(NaiveDate::from_ymd_opt(2025, 7, 1).unwrap()),
        required_schema_version: None,
    };
    // Use JSON for deterministic key ordering in snapshots
    let json = serde_json::to_string_pretty(&cfg).unwrap();
    insta::assert_snapshot!("team_config_json", json);
}

// ---------------------------------------------------------------------------
// resolve_team_config
// ---------------------------------------------------------------------------

#[test]
fn resolve_config_from_defaults() {
    let cfg = resolve_team_config(
        None,
        Some("alice,bob".to_string()),
        None,
        None,
        None,
        None,
        None,
        vec![],
    )
    .unwrap();
    assert_eq!(cfg.members, vec!["alice", "bob"]);
}

#[test]
fn resolve_config_cli_overrides_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("team.yaml");
    std::fs::write(&path, "members: [file_member]\nsections: [summary]\n").unwrap();

    let cfg = resolve_team_config(
        Some(path),
        Some("cli_member".to_string()),
        Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
        Some(NaiveDate::from_ymd_opt(2025, 7, 1).unwrap()),
        Some("coverage,receipts".to_string()),
        None,
        None,
        vec!["cli_member=CLI Member".to_string()],
    )
    .unwrap();

    assert_eq!(cfg.members, vec!["cli_member"]);
    assert_eq!(cfg.sections, vec!["coverage", "receipts"]);
    assert_eq!(cfg.aliases.get("cli_member").unwrap(), "CLI Member");
    assert!(cfg.since.is_some());
    assert!(cfg.until.is_some());
}

#[test]
fn resolve_config_rejects_invalid_date_range() {
    let result = resolve_team_config(
        None,
        Some("alice".to_string()),
        Some(NaiveDate::from_ymd_opt(2025, 7, 1).unwrap()),
        Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
        None,
        None,
        None,
        vec![],
    );
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Invalid date range"), "{err_msg}");
}

#[test]
fn resolve_config_rejects_equal_dates() {
    let date = NaiveDate::from_ymd_opt(2025, 3, 1).unwrap();
    let result = resolve_team_config(
        None,
        Some("alice".to_string()),
        Some(date),
        Some(date),
        None,
        None,
        None,
        vec![],
    );
    assert!(result.is_err());
}

#[test]
fn resolve_config_merges_aliases_from_file_and_cli() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("team.yaml");
    std::fs::write(
        &path,
        "members: [alice, bob]\naliases:\n  alice: \"File Alice\"\n",
    )
    .unwrap();

    let cfg = resolve_team_config(
        Some(path),
        None,
        None,
        None,
        None,
        None,
        None,
        vec!["bob=CLI Bob".to_string()],
    )
    .unwrap();

    assert_eq!(cfg.aliases.get("alice").unwrap(), "File Alice");
    assert_eq!(cfg.aliases.get("bob").unwrap(), "CLI Bob");
}

#[test]
fn resolve_config_cli_alias_overrides_file_alias() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("team.yaml");
    std::fs::write(
        &path,
        "members: [alice]\naliases:\n  alice: \"File Alice\"\n",
    )
    .unwrap();

    let cfg = resolve_team_config(
        Some(path),
        None,
        None,
        None,
        None,
        None,
        None,
        vec!["alice=Override Alice".to_string()],
    )
    .unwrap();

    assert_eq!(cfg.aliases.get("alice").unwrap(), "Override Alice");
}

#[test]
fn resolve_config_sets_template() {
    let cfg = resolve_team_config(
        None,
        Some("alice".to_string()),
        None,
        None,
        None,
        Some(PathBuf::from("custom.md")),
        None,
        vec![],
    )
    .unwrap();
    assert_eq!(cfg.template, Some(PathBuf::from("custom.md")));
}

#[test]
fn resolve_config_sets_required_schema_version() {
    let cfg = resolve_team_config(
        None,
        Some("alice".to_string()),
        None,
        None,
        None,
        None,
        Some("1.0.0".to_string()),
        vec![],
    )
    .unwrap();
    assert_eq!(cfg.required_schema_version, Some("1.0.0".to_string()));
}

// ---------------------------------------------------------------------------
// proptest: parse_csv_list properties
// ---------------------------------------------------------------------------

mod property_tests {
    use proptest::prelude::*;
    use shiplog_team_core::parse_csv_list;

    proptest! {
        #[test]
        fn csv_list_never_contains_empty(raw in ".*") {
            let result = parse_csv_list(&raw);
            for item in &result {
                prop_assert!(!item.is_empty(), "found empty item in parse_csv_list output");
            }
        }

        #[test]
        fn csv_list_no_duplicates(raw in ".*") {
            let result = parse_csv_list(&raw);
            let mut seen = std::collections::HashSet::new();
            for item in &result {
                prop_assert!(seen.insert(item.clone()), "duplicate item: {item}");
            }
        }

        #[test]
        fn csv_list_items_are_trimmed(raw in ".*") {
            let result = parse_csv_list(&raw);
            for item in &result {
                prop_assert_eq!(item.as_str(), item.trim());
            }
        }

        #[test]
        fn csv_list_idempotent(raw in "[a-z, ]{0,50}") {
            let first = parse_csv_list(&raw);
            let rejoined = first.join(",");
            let second = parse_csv_list(&rejoined);
            prop_assert_eq!(first, second, "parse_csv_list is not idempotent");
        }
    }
}
