//! BDD scenarios for shiplog-team-core: config loading, CSV parsing, defaults.

use shiplog_team_core::{TeamConfig, parse_alias_list, parse_csv_list};
use shiplog_testkit::bdd::{Scenario, assertions};

// ---------------------------------------------------------------------------
// Scenario: Given a team config YAML, when loading config, then all members resolved
// ---------------------------------------------------------------------------

#[test]
fn bdd_load_team_config_resolves_all_members() {
    Scenario::new("Load team config YAML resolves all members")
        .given("a team config YAML with members and aliases", |ctx| {
            let dir = tempfile::tempdir().expect("tempdir").keep();
            let path = dir.join("team.yaml");
            std::fs::write(
                &path,
                r#"
members:
  - alice
  - bob
  - carol
aliases:
  alice: "Alice Smith"
  bob: "Bob Jones"
sections:
  - summary
  - coverage
"#,
            )
            .expect("write yaml");
            ctx.strings
                .insert("config_path".into(), path.to_string_lossy().into_owned());
        })
        .when("loading config from that file", |ctx| {
            let path_str = ctx.string("config_path").unwrap().to_string();
            let cfg = TeamConfig::load(std::path::Path::new(&path_str))
                .map_err(|e| format!("load failed: {e}"))?;
            ctx.numbers
                .insert("member_count".into(), cfg.members.len() as u64);
            ctx.numbers
                .insert("alias_count".into(), cfg.aliases.len() as u64);
            ctx.strings
                .insert("members_csv".into(), cfg.members.join(","));
            ctx.strings
                .insert("sections_csv".into(), cfg.normalized_sections().join(","));
            // Store alias resolution check
            let alice_alias = cfg.aliases.get("alice").cloned().unwrap_or_default();
            ctx.strings.insert("alice_alias".into(), alice_alias);
            Ok(())
        })
        .then("all three members are resolved", |ctx| {
            let count = assertions::assert_present(ctx.number("member_count"), "member_count")?;
            assertions::assert_eq(count, 3, "member_count")
        })
        .then("aliases map to display names", |ctx| {
            let alias_count = assertions::assert_present(ctx.number("alias_count"), "alias_count")?;
            assertions::assert_eq(alias_count, 2, "alias_count")?;
            let alice = assertions::assert_present(ctx.string("alice_alias"), "alice_alias")?;
            assertions::assert_eq(alice, "Alice Smith", "alice alias value")
        })
        .then("sections are preserved from YAML", |ctx| {
            let sections = assertions::assert_present(ctx.string("sections_csv"), "sections_csv")?;
            assertions::assert_eq(sections, "summary,coverage", "sections")
        })
        .run()
        .expect("scenario should pass");
}

// ---------------------------------------------------------------------------
// Scenario: Given CSV aliases, when parsing, then aliases map correctly
// ---------------------------------------------------------------------------

#[test]
fn bdd_csv_aliases_parse_correctly() {
    Scenario::new("CSV aliases parse into correct mapping")
        .given(
            "a set of alias entries in member=Display Name format",
            |ctx| {
                ctx.strings.insert(
                    "alias_input".into(),
                    "alice=Alice Smith,bob=Bob Jones,carol=Carol W.".into(),
                );
            },
        )
        .when("parsing the alias list", |ctx| {
            let raw = ctx.string("alias_input").unwrap().to_string();
            let entries: Vec<String> = raw.split(',').map(|s| s.to_string()).collect();
            let aliases = parse_alias_list(&entries).map_err(|e| format!("parse failed: {e}"))?;
            ctx.numbers
                .insert("alias_count".into(), aliases.len() as u64);

            for (k, v) in &aliases {
                ctx.strings.insert(format!("alias_{k}"), v.clone());
            }
            Ok(())
        })
        .then("all aliases are present", |ctx| {
            let count = assertions::assert_present(ctx.number("alias_count"), "alias_count")?;
            assertions::assert_eq(count, 3, "alias_count")
        })
        .then("alice maps to Alice Smith", |ctx| {
            let val = assertions::assert_present(ctx.string("alias_alice"), "alias_alice")?;
            assertions::assert_eq(val, "Alice Smith", "alice alias")
        })
        .then("bob maps to Bob Jones", |ctx| {
            let val = assertions::assert_present(ctx.string("alias_bob"), "alias_bob")?;
            assertions::assert_eq(val, "Bob Jones", "bob alias")
        })
        .then("carol maps to Carol W.", |ctx| {
            let val = assertions::assert_present(ctx.string("alias_carol"), "alias_carol")?;
            assertions::assert_eq(val, "Carol W.", "carol alias")
        })
        .run()
        .expect("scenario should pass");
}

#[test]
fn bdd_csv_list_parsing_deduplicates_and_trims() {
    Scenario::new("CSV list parsing deduplicates and trims entries")
        .given("a raw CSV string with duplicates and whitespace", |ctx| {
            ctx.strings
                .insert("raw".into(), " alice , bob , alice , carol ".into());
        })
        .when("parsing the CSV list", |ctx| {
            let raw = ctx.string("raw").unwrap().to_string();
            let result = parse_csv_list(&raw);
            ctx.numbers.insert("count".into(), result.len() as u64);
            ctx.strings.insert("parsed".into(), result.join(","));
            Ok(())
        })
        .then("duplicates are removed", |ctx| {
            let count = assertions::assert_present(ctx.number("count"), "count")?;
            assertions::assert_eq(count, 3, "unique count")
        })
        .then(
            "entries are trimmed and ordered by first occurrence",
            |ctx| {
                let parsed = assertions::assert_present(ctx.string("parsed"), "parsed")?;
                assertions::assert_eq(parsed, "alice,bob,carol", "parsed order")
            },
        )
        .run()
        .expect("scenario should pass");
}

// ---------------------------------------------------------------------------
// Scenario: Given empty config, when loading, then defaults applied
// ---------------------------------------------------------------------------

#[test]
fn bdd_empty_config_applies_defaults() {
    Scenario::new("Empty config applies default values")
        .given("an empty team config YAML", |ctx| {
            let dir = tempfile::tempdir().expect("tempdir").keep();
            let path = dir.join("empty.yaml");
            std::fs::write(&path, "members: []\n").expect("write yaml");
            ctx.strings
                .insert("config_path".into(), path.to_string_lossy().into_owned());
        })
        .when("loading the config", |ctx| {
            let path_str = ctx.string("config_path").unwrap().to_string();
            let cfg = TeamConfig::load(std::path::Path::new(&path_str))
                .map_err(|e| format!("load failed: {e}"))?;
            ctx.numbers
                .insert("member_count".into(), cfg.members.len() as u64);
            ctx.numbers
                .insert("alias_count".into(), cfg.aliases.len() as u64);
            ctx.strings
                .insert("sections".into(), cfg.normalized_sections().join(","));
            ctx.flags
                .insert("has_template".into(), cfg.template.is_some());
            ctx.flags.insert("has_since".into(), cfg.since.is_some());
            ctx.flags.insert("has_until".into(), cfg.until.is_some());
            ctx.flags.insert(
                "has_schema_version".into(),
                cfg.required_schema_version.is_some(),
            );
            Ok(())
        })
        .then("members list is empty", |ctx| {
            let count = assertions::assert_present(ctx.number("member_count"), "member_count")?;
            assertions::assert_eq(count, 0, "member_count")
        })
        .then("aliases are empty", |ctx| {
            let count = assertions::assert_present(ctx.number("alias_count"), "alias_count")?;
            assertions::assert_eq(count, 0, "alias_count")
        })
        .then("default sections are applied", |ctx| {
            let sections = assertions::assert_present(ctx.string("sections"), "sections")?;
            assertions::assert_eq(
                sections,
                "summary,workstreams,coverage,receipts",
                "default sections",
            )
        })
        .then("optional fields are None", |ctx| {
            let has_template =
                assertions::assert_present(ctx.flag("has_template"), "has_template")?;
            assertions::assert_false(has_template, "template should be None")?;
            let has_since = assertions::assert_present(ctx.flag("has_since"), "has_since")?;
            assertions::assert_false(has_since, "since should be None")?;
            let has_until = assertions::assert_present(ctx.flag("has_until"), "has_until")?;
            assertions::assert_false(has_until, "until should be None")?;
            let has_sv =
                assertions::assert_present(ctx.flag("has_schema_version"), "has_schema_version")?;
            assertions::assert_false(has_sv, "required_schema_version should be None")
        })
        .run()
        .expect("scenario should pass");
}
