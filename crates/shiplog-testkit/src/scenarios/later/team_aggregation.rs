//! BDD scenarios for Team Aggregation Mode (Feature 10)
//!
//! These scenarios execute the real team configuration and aggregation crates,
//! instead of synthetic flag mutations, to lock behavior to actual runtime code.

use crate::bdd::Scenario;
use crate::bdd::assertions::*;
use chrono::{NaiveDate, TimeZone, Utc};
use serde_json::Value;
use shiplog_ids::{EventId, RunId};
use shiplog_schema::coverage::{Completeness, CoverageManifest, TimeWindow};
use shiplog_schema::event::{
    Actor, EventEnvelope, EventKind, EventPayload, PullRequestEvent, PullRequestState, RepoRef,
    RepoVisibility, SourceRef, SourceSystem,
};
use shiplog_team_aggregate::TeamAggregator;
use shiplog_team_core::{TeamConfig, parse_alias_list, parse_csv_list, resolve_team_config};
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

fn member_event(member: &str, repo: &str, idx: u64) -> EventEnvelope {
    let when = Utc
        .timestamp_opt(1_735_689_600 + idx as i64, 0)
        .single()
        .expect("valid timestamp");
    EventEnvelope {
        id: EventId::from_parts(["team", member, &idx.to_string()]),
        kind: EventKind::PullRequest,
        occurred_at: when,
        actor: Actor {
            login: member.to_string(),
            id: None,
        },
        repo: RepoRef {
            full_name: repo.to_string(),
            html_url: Some(format!("https://github.com/{repo}")),
            visibility: RepoVisibility::Public,
        },
        payload: EventPayload::PullRequest(PullRequestEvent {
            number: idx,
            title: format!("{member} event {idx}"),
            state: PullRequestState::Merged,
            created_at: when,
            merged_at: Some(when),
            additions: Some(5),
            deletions: Some(1),
            changed_files: Some(2),
            touched_paths_hint: vec![],
            window: Some(TimeWindow {
                since: NaiveDate::from_ymd_opt(2025, 1, 1).expect("valid date"),
                until: NaiveDate::from_ymd_opt(2025, 2, 1).expect("valid date"),
            }),
        }),
        tags: vec![],
        links: vec![],
        source: SourceRef {
            system: SourceSystem::Github,
            url: Some(format!("https://github.com/{repo}/pull/{idx}")),
            opaque_id: None,
        },
    }
}

fn member_coverage(member: &str) -> CoverageManifest {
    CoverageManifest {
        run_id: RunId::now("team"),
        generated_at: Utc::now(),
        user: member.to_string(),
        window: TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 1, 1).expect("valid date"),
            until: NaiveDate::from_ymd_opt(2025, 2, 1).expect("valid date"),
        },
        mode: "merged".to_string(),
        sources: vec!["github".to_string()],
        slices: vec![],
        warnings: vec![],
        completeness: Completeness::Complete,
    }
}

fn write_member_ledger(
    root: &Path,
    member: &str,
    event_count: usize,
    schema_version: Option<&str>,
) -> Result<(), String> {
    let member_dir = root.join(member);
    std::fs::create_dir_all(&member_dir)
        .map_err(|err| format!("create member dir for {member}: {err}"))?;

    let mut events_file = std::fs::File::create(member_dir.join("ledger.events.jsonl"))
        .map_err(|err| format!("create events file for {member}: {err}"))?;
    for idx in 0..event_count {
        let event = member_event(member, "acme/team-repo", idx as u64 + 1);
        let line = serde_json::to_string(&event)
            .map_err(|err| format!("serialize event for {member}: {err}"))?;
        writeln!(events_file, "{line}")
            .map_err(|err| format!("write events line for {member}: {err}"))?;
    }

    let mut coverage_value = serde_json::to_value(member_coverage(member))
        .map_err(|err| format!("serialize coverage for {member}: {err}"))?;
    if let Some(version) = schema_version {
        coverage_value["schema_version"] = Value::String(version.to_string());
    }
    let coverage_json = serde_json::to_string(&coverage_value)
        .map_err(|err| format!("encode coverage json for {member}: {err}"))?;
    std::fs::write(member_dir.join("coverage.manifest.json"), coverage_json)
        .map_err(|err| format!("write coverage for {member}: {err}"))?;
    Ok(())
}

fn aggregate_and_render(
    cfg: TeamConfig,
    root: &Path,
) -> Result<(shiplog_team_aggregate::TeamAggregateResult, String), String> {
    let aggregator = TeamAggregator::new(cfg);
    let result = aggregator
        .aggregate(root)
        .map_err(|err| format!("team aggregation failed: {err}"))?;
    let packet = aggregator
        .render_packet_markdown(&result)
        .map_err(|err| format!("team render failed: {err}"))?;
    Ok((result, packet))
}

/// Scenario 10.1: User generates team-level shipping summary
pub fn team_aggregate_summary() -> Scenario {
    Scenario::new("User generates team-level shipping summary")
        .given("a user is a team lead", |ctx| {
            ctx.strings
                .insert("user_role".to_string(), "team_lead".to_string());
        })
        .given(
            "they have access to multiple team members' shiplog ledgers",
            |ctx| {
                ctx.numbers.insert("member_count".to_string(), 3);
            },
        )
        .when(
            "they run \"shiplog team-aggregate --members alice,bob,charlie --since 2025-01-01\"",
            |ctx| {
                let temp = tempfile::tempdir().map_err(|err| format!("create tempdir: {err}"))?;
                write_member_ledger(temp.path(), "alice", 2, Some("0.2.0"))?;
                write_member_ledger(temp.path(), "bob", 3, Some("0.2.0"))?;
                write_member_ledger(temp.path(), "charlie", 1, Some("0.2.0"))?;

                let cfg = TeamConfig {
                    members: vec![
                        "alice".to_string(),
                        "bob".to_string(),
                        "charlie".to_string(),
                    ],
                    since: Some(NaiveDate::from_ymd_opt(2025, 1, 1).expect("valid date")),
                    until: Some(NaiveDate::from_ymd_opt(2025, 2, 1).expect("valid date")),
                    ..TeamConfig::default()
                };

                let (result, packet) = aggregate_and_render(cfg, temp.path())?;
                ctx.flags
                    .insert("team_packet_generated".to_string(), !packet.is_empty());
                ctx.flags.insert(
                    "member_sections_included".to_string(),
                    packet.contains("### alice")
                        && packet.contains("### bob")
                        && packet.contains("### charlie"),
                );
                ctx.flags.insert(
                    "team_summary_included".to_string(),
                    packet.contains("## Summary"),
                );
                ctx.numbers.insert(
                    "included_member_count".to_string(),
                    result.included_members.len() as u64,
                );
                Ok(())
            },
        )
        .then("a team-level packet should be generated", |ctx| {
            assert_true(
                ctx.flag("team_packet_generated").unwrap_or(false),
                "team packet generated",
            )
        })
        .then(
            "the packet should include sections for each team member",
            |ctx| {
                assert_true(
                    ctx.flag("member_sections_included").unwrap_or(false),
                    "member sections included",
                )
            },
        )
        .then("the packet should include a team summary section", |ctx| {
            assert_true(
                ctx.flag("team_summary_included").unwrap_or(false),
                "team summary included",
            )
        })
}

/// Scenario 10.2: User aggregates team with configurable sections
pub fn team_aggregate_sections() -> Scenario {
    Scenario::new("User aggregates team with configurable sections")
        .given("a user is generating a team packet", |_| {})
        .given("they want to include only workstreams and coverage", |ctx| {
            ctx.strings
                .insert("sections".to_string(), "workstreams,coverage".to_string());
        })
        .when(
            "they run \"shiplog team-aggregate --members alice,bob --sections workstreams,coverage\"",
            |ctx| {
                let temp = tempfile::tempdir().map_err(|err| format!("create tempdir: {err}"))?;
                write_member_ledger(temp.path(), "alice", 2, Some("0.2.0"))?;
                write_member_ledger(temp.path(), "bob", 2, Some("0.2.0"))?;

                let cfg = TeamConfig {
                    members: vec!["alice".to_string(), "bob".to_string()],
                    sections: vec!["workstreams".to_string(), "coverage".to_string()],
                    ..TeamConfig::default()
                };
                let (_result, packet) = aggregate_and_render(cfg, temp.path())?;

                ctx.flags
                    .insert("team_packet_generated".to_string(), !packet.is_empty());
                ctx.flags.insert(
                    "workstreams_included".to_string(),
                    packet.contains("## Workstreams"),
                );
                ctx.flags
                    .insert("coverage_included".to_string(), packet.contains("## Coverage"));
                ctx.flags
                    .insert("summary_excluded".to_string(), !packet.contains("## Summary"));
                ctx.flags
                    .insert("receipts_excluded".to_string(), !packet.contains("## Receipts"));
                Ok(())
            },
        )
        .then("the team packet should include only the specified sections", |ctx| {
            assert_true(
                ctx.flag("workstreams_included").unwrap_or(false),
                "workstreams included",
            )?;
            assert_true(
                ctx.flag("coverage_included").unwrap_or(false),
                "coverage included",
            )
        })
        .then("other sections should be excluded", |ctx| {
            assert_true(
                ctx.flag("summary_excluded").unwrap_or(false)
                    && ctx.flag("receipts_excluded").unwrap_or(false),
                "other sections excluded",
            )
        })
}

pub fn team_aggregate_runtime_constructor_contract() -> Scenario {
    Scenario::new("Team aggregator runtime crate exposes stable constructor contract")
        .given("a normalized team config", |ctx| {
            ctx.strings
                .insert("member".to_string(), "alice".to_string());
        })
        .when("the runtime aggregator is constructed", |ctx| {
            let cfg = TeamConfig {
                members: vec!["alice".to_string()],
                aliases: HashMap::new(),
                sections: vec!["summary".to_string()],
                template: None,
                since: None,
                until: None,
                required_schema_version: None,
            };

            let _aggregator = TeamAggregator::new(cfg);
            ctx.flags.insert("runtime_constructed".to_string(), true);
            Ok(())
        })
        .then(
            "constructor should be available from the microcrate",
            |ctx| {
                assert_true(
                    ctx.flag("runtime_constructed").unwrap_or(false),
                    "runtime constructor",
                )
            },
        )
}

pub fn team_aggregate_resolver_contract() -> Scenario {
    Scenario::new("Team CLI config resolver enforces normalized deterministic config")
        .given(
            "a team run is configured with duplicate members, section list, aliases, and date range",
            |ctx| {
                ctx.strings.insert(
                    "raw_members".to_string(),
                    "alice,bob,alice".to_string(),
                );
                ctx.strings.insert(
                    "raw_sections".to_string(),
                    "summary,workstreams,summary".to_string(),
                );
                ctx.strings
                    .insert("required_version".to_string(), "0.2.1".to_string());
            },
        )
        .when(
            "the resolver parses CLI list fields and validates the date window",
            |ctx| {
                let raw_members = ctx.string("raw_members").unwrap_or("");
                let parsed_members = parse_csv_list(raw_members);
                let alias_pairs = vec!["alice=Alice S.".to_string()];
                let aliases =
                    parse_alias_list(&alias_pairs).map_err(|err| format!("parse aliases: {err}"))?;

                let cfg = resolve_team_config(
                    None,
                    Some(raw_members.to_string()),
                    Some(NaiveDate::from_ymd_opt(2025, 1, 1).expect("valid date")),
                    Some(NaiveDate::from_ymd_opt(2025, 2, 1).expect("valid date")),
                    Some("summary,workstreams,summary".to_string()),
                    None,
                    Some("0.2.1".to_string()),
                    alias_pairs,
                )
                .map_err(|err| format!("resolve team config: {err}"))?;

                ctx.numbers
                    .insert("parsed_member_count".to_string(), parsed_members.len() as u64);
                ctx.numbers
                    .insert("resolved_member_count".to_string(), cfg.members.len() as u64);
                ctx.numbers
                    .insert("resolved_section_count".to_string(), cfg.sections.len() as u64);
                ctx.strings
                    .insert("alice_alias".to_string(), aliases["alice"].clone());
                ctx.strings
                    .insert("resolved_since".to_string(), cfg.since.unwrap().to_string());
                Ok(())
            },
        )
        .then("duplicate members should be collapsed", |ctx| {
            let parsed = assert_present(ctx.number("parsed_member_count"), "parsed member count")?;
            let resolved =
                assert_present(ctx.number("resolved_member_count"), "resolved member count")?;
            assert_eq(parsed, resolved, "deduped members are preserved")
        })
        .then("section list should be deduplicated and normalized", |ctx| {
            let sections =
                assert_present(ctx.number("resolved_section_count"), "resolved section count")?;
            assert_eq(sections, 2, "section count")
        })
        .then("aliases should be resolved and retained", |ctx| {
            let alias = assert_present(ctx.string("alice_alias"), "alice alias")?;
            assert_eq(alias, "Alice S.", "alias value")
        })
        .then("resolved date range should be retained", |ctx| {
            let since = assert_present(ctx.string("resolved_since"), "resolved since")?;
            assert_eq(since, "2025-01-01", "resolved since date")
        })
}

/// Scenario 10.3: User aggregates team with member aliases
pub fn team_aggregate_aliases() -> Scenario {
    Scenario::new("User aggregates team with member aliases")
        .given(
            "a user has team members with different display names",
            |ctx| {
                ctx.strings
                    .insert("member_real_name".to_string(), "alice".to_string());
                ctx.strings
                    .insert("member_alias".to_string(), "Alice S.".to_string());
            },
        )
        .given("they configure member aliases in a config file", |ctx| {
            ctx.strings
                .insert("config_file".to_string(), "team.yaml".to_string());
            ctx.flags.insert("aliases_configured".to_string(), true);
        })
        .when(
            "they run \"shiplog team-aggregate --config team.yaml\"",
            |ctx| {
                let temp = tempfile::tempdir().map_err(|err| format!("create tempdir: {err}"))?;
                write_member_ledger(temp.path(), "alice", 2, Some("0.2.0"))?;

                let mut aliases = HashMap::new();
                aliases.insert("alice".to_string(), "Alice S.".to_string());
                let cfg = TeamConfig {
                    members: vec!["alice".to_string()],
                    aliases,
                    ..TeamConfig::default()
                };
                let (result, packet) = aggregate_and_render(cfg, temp.path())?;
                let display_name = result
                    .members
                    .iter()
                    .find(|member| member.member == "alice")
                    .map(|member| member.display_name.clone())
                    .unwrap_or_default();

                ctx.flags
                    .insert("team_packet_generated".to_string(), !packet.is_empty());
                ctx.strings.insert("display_name".to_string(), display_name);
                ctx.flags.insert(
                    "aliases_applied".to_string(),
                    packet.contains("Alice S.") || ctx.string("display_name") == Some("Alice S."),
                );
                Ok(())
            },
        )
        .then("the team packet should use the configured aliases", |ctx| {
            assert_true(
                ctx.flag("aliases_applied").unwrap_or(false),
                "aliases applied",
            )
        })
        .then("member identities should be consistent", |ctx| {
            let name = assert_present(ctx.string("display_name"), "display_name")?;
            assert_eq(name, "Alice S.", "display name")
        })
}

/// Scenario 10.4: Member ledger not found
pub fn team_aggregate_missing_ledger() -> Scenario {
    Scenario::new("Member ledger not found")
        .given("a user specifies a team member", |ctx| {
            ctx.strings
                .insert("member_name".to_string(), "alice".to_string());
        })
        .given("that member's ledger does not exist", |_| {})
        .when(
            "they run \"shiplog team-aggregate --members alice,nonexistent\"",
            |ctx| {
                let temp = tempfile::tempdir().map_err(|err| format!("create tempdir: {err}"))?;
                write_member_ledger(temp.path(), "alice", 2, Some("0.2.0"))?;

                let cfg = TeamConfig {
                    members: vec!["alice".to_string(), "nonexistent".to_string()],
                    ..TeamConfig::default()
                };
                let (result, packet) = aggregate_and_render(cfg, temp.path())?;
                let warning = result
                    .warnings
                    .iter()
                    .find(|message| message.contains("Missing ledger"))
                    .cloned()
                    .unwrap_or_default();

                ctx.flags
                    .insert("team_packet_generated".to_string(), !packet.is_empty());
                ctx.flags.insert(
                    "has_available_members".to_string(),
                    result.included_members.contains(&"alice".to_string()),
                );
                ctx.strings.insert("warning_message".to_string(), warning);
                Ok(())
            },
        )
        .then("a warning should indicate the missing ledger", |ctx| {
            let warning = assert_present(ctx.string("warning_message"), "warning_message")?;
            assert_contains(warning, "Missing ledger", "warning message")
        })
        .then(
            "the packet should be generated for available members",
            |ctx| {
                assert_true(
                    ctx.flag("team_packet_generated").unwrap_or(false)
                        && ctx.flag("has_available_members").unwrap_or(false),
                    "packet generated for available members",
                )
            },
        )
}

/// Scenario 10.5: Member ledger has incompatible version
pub fn team_aggregate_incompatible_version() -> Scenario {
    Scenario::new("Member ledger has incompatible version")
        .given("a user specifies a team member", |ctx| {
            ctx.strings
                .insert("member_name".to_string(), "bob".to_string());
        })
        .given(
            "that member's ledger uses an incompatible schema version",
            |ctx| {
                ctx.strings
                    .insert("ledger_version".to_string(), "0.1.0".to_string());
                ctx.strings
                    .insert("required_version".to_string(), "0.2.0".to_string());
            },
        )
        .when(
            "they run \"shiplog team-aggregate --members alice,bob\"",
            |ctx| {
                let temp = tempfile::tempdir().map_err(|err| format!("create tempdir: {err}"))?;
                write_member_ledger(temp.path(), "alice", 2, Some("0.2.0"))?;
                write_member_ledger(temp.path(), "bob", 2, Some("0.1.0"))?;

                let cfg = TeamConfig {
                    members: vec!["alice".to_string(), "bob".to_string()],
                    required_schema_version: Some("0.2.0".to_string()),
                    ..TeamConfig::default()
                };
                let (result, packet) = aggregate_and_render(cfg, temp.path())?;
                let warning = result
                    .warnings
                    .iter()
                    .find(|message| message.contains("incompatib"))
                    .cloned()
                    .unwrap_or_default();

                ctx.flags
                    .insert("team_packet_generated".to_string(), !packet.is_empty());
                ctx.flags.insert(
                    "member_excluded".to_string(),
                    result.incompatible_members.contains(&"bob".to_string()),
                );
                ctx.strings.insert("warning_message".to_string(), warning);
                Ok(())
            },
        )
        .then("a warning should indicate the incompatible ledger", |ctx| {
            let warning = assert_present(ctx.string("warning_message"), "warning_message")?;
            assert_contains(warning, "incompatib", "warning message")
        })
        .then("that member's data should be excluded", |ctx| {
            assert_true(
                ctx.flag("member_excluded").unwrap_or(false),
                "member excluded",
            )
        })
}

/// Scenario 10.6: Team aggregation uses custom template
pub fn team_aggregate_custom_template() -> Scenario {
    Scenario::new("Team aggregation uses custom template")
        .given("a user has a custom team template", |ctx| {
            ctx.paths
                .insert("template_path".to_string(), "templates/team.md".into());
        })
        .when(
            "they run \"shiplog team-aggregate --template team.md\"",
            |ctx| {
                let temp = tempfile::tempdir().map_err(|err| format!("create tempdir: {err}"))?;
                write_member_ledger(temp.path(), "alice", 2, Some("0.2.0"))?;
                write_member_ledger(temp.path(), "bob", 1, Some("0.2.0"))?;

                let template_path = temp.path().join("team-template.md");
                std::fs::write(
                    &template_path,
                    "# Custom Team Packet\nMembers: {{ member_count }}\nIncluded: {{ included_member_count }}\nSources: {{ coverage_sources }}\n",
                )
                .map_err(|err| format!("write custom template: {err}"))?;

                let cfg = TeamConfig {
                    members: vec!["alice".to_string(), "bob".to_string()],
                    template: Some(template_path),
                    ..TeamConfig::default()
                };
                let (_result, packet) = aggregate_and_render(cfg, temp.path())?;

                ctx.flags
                    .insert("team_packet_generated".to_string(), !packet.is_empty());
                ctx.flags.insert(
                    "custom_template_used".to_string(),
                    packet.contains("# Custom Team Packet"),
                );
                Ok(())
            },
        )
        .then("the team packet should use the custom template", |ctx| {
            assert_true(
                ctx.flag("team_packet_generated").unwrap_or(false),
                "team packet generated",
            )
        })
        .then("the template should render all team members", |ctx| {
            assert_true(
                ctx.flag("custom_template_used").unwrap_or(false),
                "custom template used",
            )
        })
}

/// Scenario 10.7: Team aggregation with many members
pub fn team_aggregate_large() -> Scenario {
    Scenario::new("Team aggregation with many members")
        .given("a user has a team of 20 members", |ctx| {
            ctx.numbers.insert("member_count".to_string(), 20);
        })
        .when("they run \"shiplog team-aggregate --members all\"", |ctx| {
            let temp = tempfile::tempdir().map_err(|err| format!("create tempdir: {err}"))?;
            let members: Vec<String> = (1..=20).map(|idx| format!("member-{idx}")).collect();
            for member in &members {
                write_member_ledger(temp.path(), member, 2, Some("0.2.0"))?;
            }

            let cfg = TeamConfig {
                members,
                ..TeamConfig::default()
            };
            let started = std::time::Instant::now();
            let (_result, packet) = aggregate_and_render(cfg, temp.path())?;
            let elapsed_ms = started.elapsed().as_millis() as u64;

            ctx.flags
                .insert("team_packet_generated".to_string(), !packet.is_empty());
            ctx.numbers
                .insert("aggregate_time_ms".to_string(), elapsed_ms);
            Ok(())
        })
        .then(
            "aggregation should complete within reasonable time (< 30 seconds)",
            |ctx| {
                let elapsed_ms =
                    assert_present(ctx.number("aggregate_time_ms"), "aggregate_time_ms")?;
                assert_true(elapsed_ms < 30_000, "aggregate time")
            },
        )
}
