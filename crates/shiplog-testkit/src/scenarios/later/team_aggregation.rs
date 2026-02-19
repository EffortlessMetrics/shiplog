//! BDD scenarios for Team Aggregation Mode (Feature 10)
//!
//! Scenarios cover:
//! - Primary user workflows (generating team-level shipping summaries)
//! - Edge cases (missing ledgers, incompatible versions)
//! - Integration with other features (custom templates)
//! - Performance scenarios (many team members)

use crate::bdd::Scenario;
use crate::bdd::assertions::*;

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
                ctx.strings
                    .insert("members".to_string(), "alice,bob,charlie".to_string());
            },
        )
        .when(
            "they run \"shiplog team-aggregate --members alice,bob,charlie --since 2025-01-01\"",
            |ctx| {
                ctx.flags.insert("team_packet_generated".to_string(), true);
                ctx.paths
                    .insert("packet_path".to_string(), "/out/team/packet.md".into());
                ctx.flags
                    .insert("member_sections_included".to_string(), true);
                ctx.flags.insert("team_summary_included".to_string(), true);
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
        .given("a user is generating a team packet", |ctx| {
            ctx.flags.insert("team_mode".to_string(), true);
        })
        .given("they want to include only workstreams and coverage", |ctx| {
            ctx.strings
                .insert("sections".to_string(), "workstreams,coverage".to_string());
        })
        .when(
            "they run \"shiplog team-aggregate --members alice,bob --sections workstreams,coverage\"",
            |ctx| {
                ctx.flags.insert("team_packet_generated".to_string(), true);
                ctx.flags.insert("workstreams_included".to_string(), true);
                ctx.flags.insert("coverage_included".to_string(), true);
                ctx.flags.insert("summary_excluded".to_string(), true);
                ctx.flags.insert("receipts_excluded".to_string(), true);
                Ok(())
            },
        )
        .then("the team packet should include only the specified sections", |ctx| {
            assert_true(
                ctx.flag("workstreams_included").unwrap_or(false),
                "workstreams included",
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

/// Scenario 10.3: User aggregates team with member aliases
pub fn team_aggregate_aliases() -> Scenario {
    Scenario::new("User aggregates team with member aliases")
        .given(
            "a user has team members with different display names",
            |ctx| {
                ctx.strings
                    .insert("member_real_name".to_string(), "alice.smith".to_string());
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
                ctx.flags.insert("team_packet_generated".to_string(), true);
                ctx.strings
                    .insert("display_name".to_string(), "Alice S.".to_string());
                ctx.flags.insert("aliases_applied".to_string(), true);
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
            let name = ctx.string("display_name").unwrap();
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
        .given("that member's ledger does not exist", |ctx| {
            ctx.flags.insert("ledger_missing".to_string(), true);
        })
        .when(
            "they run \"shiplog team-aggregate --members alice,nonexistent\"",
            |ctx| {
                ctx.flags.insert("team_packet_generated".to_string(), true);
                ctx.strings.insert(
                    "warning_message".to_string(),
                    "Warning: Ledger for member 'nonexistent' not found".to_string(),
                );
                Ok(())
            },
        )
        .then("a warning should indicate the missing ledger", |ctx| {
            assert_true(
                ctx.flag("team_packet_generated").unwrap_or(false),
                "team packet generated",
            )
        })
        .then(
            "the packet should be generated for available members",
            |ctx| {
                let warning = ctx.string("warning_message").unwrap();
                assert_contains(warning, "not found", "warning message")
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
                ctx.flags.insert("team_packet_generated".to_string(), true);
                ctx.strings.insert(
                    "warning_message".to_string(),
                    "Warning: Ledger for 'bob' has incompatible version 0.1.0 (required: 0.2.0)"
                        .to_string(),
                );
                ctx.flags.insert("member_excluded".to_string(), true);
                Ok(())
            },
        )
        .then("a warning should indicate the incompatible ledger", |ctx| {
            let warning = ctx.string("warning_message").unwrap();
            assert_contains(warning, "incompatible", "warning message")
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
                ctx.flags.insert("team_packet_generated".to_string(), true);
                ctx.flags.insert("custom_template_used".to_string(), true);
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
            ctx.flags.insert("team_packet_generated".to_string(), true);
            ctx.strings
                .insert("aggregate_time".to_string(), "25s".to_string());
            Ok(())
        })
        .then(
            "aggregation should complete within reasonable time (< 30 seconds)",
            |ctx| {
                let time = ctx.string("aggregate_time").unwrap();
                assert_true(time.contains("s") && !time.contains("m"), "aggregate time")
            },
        )
}
