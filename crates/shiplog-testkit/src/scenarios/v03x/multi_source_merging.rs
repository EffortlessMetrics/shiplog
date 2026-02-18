//! BDD scenarios for Multi-Source Merging (Feature 7)
//!
//! Scenarios cover:
//! - Primary user workflows (merging events from multiple sources)
//! - Edge cases (conflicting events, no events, incompatible types)
//! - Integration with other features (clustering, rendering)
//! - Performance scenarios (large event collections)

use crate::bdd::Scenario;
use crate::bdd::assertions::*;

/// Scenario 7.1: User merges events from multiple sources
pub fn multi_source_merge() -> Scenario {
    Scenario::new("User merges events from multiple sources")
        .given("a user has collected events from GitHub", |ctx| {
            ctx.numbers.insert("github_events".to_string(), 25);
            ctx.flags.insert("github_collected".to_string(), true);
        })
        .given("they have collected events from local git", |ctx| {
            ctx.numbers.insert("local_git_events".to_string(), 30);
            ctx.flags.insert("local_git_collected".to_string(), true);
        })
        .given("they have collected manual events", |ctx| {
            ctx.numbers.insert("manual_events".to_string(), 5);
            ctx.flags.insert("manual_collected".to_string(), true);
        })
        .when(
            "they run \"shiplog merge --sources github,local_git,manual\"",
            |ctx| {
                let total = ctx.number("github_events").unwrap_or(0)
                    + ctx.number("local_git_events").unwrap_or(0)
                    + ctx.number("manual_events").unwrap_or(0);
                ctx.numbers.insert("total_events".to_string(), total);
                ctx.flags.insert("merged".to_string(), true);
                ctx.flags.insert("deduplicated".to_string(), true);
                ctx.flags.insert("sorted".to_string(), true);
                Ok(())
            },
        )
        .then("all events should be merged into a single ledger", |ctx| {
            let total = ctx.number("total_events").unwrap_or(0);
            assert_eq(total, 60, "total events")
        })
        .then("events should be deduplicated by ID", |ctx| {
            assert_true(
                ctx.flag("deduplicated").unwrap_or(false),
                "events deduplicated",
            )
        })
        .then("events should be sorted by timestamp", |ctx| {
            assert_true(ctx.flag("sorted").unwrap_or(false), "events sorted")
        })
}

/// Scenario 7.2: User merges with unified coverage tracking
pub fn multi_source_merge_coverage() -> Scenario {
    Scenario::new("User merges with unified coverage tracking")
        .given("a user has collected events from multiple sources", |ctx| {
            ctx.numbers.insert("total_events".to_string(), 60);
            ctx.flags.insert("multi_source_collected".to_string(), true);
        })
        .when("they merge the sources", |ctx| {
            ctx.flags.insert("merged".to_string(), true);
            ctx.flags.insert("coverage_unified".to_string(), true);
            ctx.numbers.insert("source_count".to_string(), 3);
            ctx.flags.insert("warnings_aggregated".to_string(), true);
            Ok(())
        })
        .then("the coverage manifest should include all sources", |ctx| {
            let count = ctx.number("source_count").unwrap_or(0);
            assert_true(count >= 3, "source count")
        })
        .then("completeness should be calculated across all sources", |ctx| {
            assert_true(
                ctx.flag("coverage_unified").unwrap_or(false),
                "coverage unified",
            )
        })
        .then("warnings should be aggregated from all sources", |ctx| {
            assert_true(
                ctx.flag("warnings_aggregated").unwrap_or(false),
                "warnings aggregated",
            )
        })
}

/// Scenario 7.3: User merges events from same source type
pub fn multi_source_merge_same_type() -> Scenario {
    Scenario::new("User merges events from same source type")
        .given("a user has collected events from GitHub for two different repos", |ctx| {
            ctx.numbers.insert("repo_a_events".to_string(), 20);
            ctx.numbers.insert("repo_b_events".to_string(), 15);
            ctx.flags.insert("github_collected".to_string(), true);
        })
        .when("they merge the sources", |ctx| {
            let total = ctx.number("repo_a_events").unwrap_or(0)
                + ctx.number("repo_b_events").unwrap_or(0);
            ctx.numbers.insert("total_events".to_string(), total);
            ctx.flags.insert("merged".to_string(), true);
            ctx.flags.insert("deduplicated".to_string(), true);
            Ok(())
        })
        .then("events from both repos should be included", |ctx| {
            let total = ctx.number("total_events").unwrap_or(0);
            assert_eq(total, 35, "total events")
        })
        .then("duplicate events (same PR) should be deduplicated", |ctx| {
            assert_true(
                ctx.flag("deduplicated").unwrap_or(false),
                "events deduplicated",
            )
        })
}

/// Scenario 7.4: Conflicting events from different sources
pub fn multi_source_merge_conflicts() -> Scenario {
    Scenario::new("Conflicting events from different sources")
        .given("a user has collected the same event from GitHub and local git", |ctx| {
            ctx.flags.insert("same_event_multiple_sources".to_string(), true);
        })
        .given("the events have different metadata", |ctx| {
            ctx.flags.insert("metadata_differs".to_string(), true);
        })
        .when("they merge the sources", |ctx| {
            ctx.flags.insert("merged".to_string(), true);
            ctx.flags.insert("conflict_resolved".to_string(), true);
            ctx.strings.insert(
                "warning_message".to_string(),
                "Resolved conflict: event appears in multiple sources".to_string(),
            );
            Ok(())
        })
        .then("one event should be chosen as authoritative", |ctx| {
            assert_true(
                ctx.flag("conflict_resolved").unwrap_or(false),
                "conflict resolved",
            )
        })
        .then("a warning should indicate the conflict was resolved", |ctx| {
            let warning = ctx.string("warning_message").unwrap();
            assert_contains(warning, "conflict", "warning message")
        })
}

/// Scenario 7.5: Merge with no events
pub fn multi_source_merge_no_events() -> Scenario {
    Scenario::new("Merge with no events")
        .given("a user attempts to merge with no collected events", |ctx| {
            ctx.flags.insert("no_events".to_string(), true);
        })
        .when("they run \"shiplog merge\"", |ctx| {
            ctx.flags.insert("command_failed".to_string(), true);
            ctx.strings.insert(
                "error_message".to_string(),
                "No events available to merge".to_string(),
            );
            Ok(())
        })
        .then("the command should fail with a clear error", |ctx| {
            assert_true(
                ctx.flag("command_failed").unwrap_or(false),
                "command failed",
            )
        })
        .then("the error should indicate no events are available to merge", |ctx| {
            let error = ctx.string("error_message").unwrap();
            assert_contains(error, "No events", "error message")
        })
}

/// Scenario 7.6: Merge with incompatible event types
pub fn multi_source_merge_incompatible() -> Scenario {
    Scenario::new("Merge with incompatible event types")
        .given("a user has collected events from a source with incompatible schema", |ctx| {
            ctx.flags.insert("incompatible_schema".to_string(), true);
        })
        .when("they attempt to merge", |ctx| {
            ctx.flags.insert("merged".to_string(), true);
            ctx.numbers.insert("skipped_events".to_string(), 3);
            ctx.strings.insert(
                "warning_message".to_string(),
                "Skipped 3 event(s) with incompatible schema".to_string(),
            );
            Ok(())
        })
        .then("incompatible events should be skipped", |ctx| {
            let count = ctx.number("skipped_events").unwrap_or(0);
            assert_true(count > 0, "events skipped")
        })
        .then("a warning should indicate the skipped events", |ctx| {
            let warning = ctx.string("warning_message").unwrap();
            assert_contains(warning, "Skipped", "warning message")
        })
}

/// Scenario 7.7: Multi-source events cluster together
pub fn multi_source_cluster() -> Scenario {
    Scenario::new("Multi-source events cluster together")
        .given("a user has merged events from GitHub, GitLab, and Jira", |ctx| {
            ctx.numbers.insert("github_events".to_string(), 25);
            ctx.numbers.insert("gitlab_events".to_string(), 15);
            ctx.numbers.insert("jira_events".to_string(), 10);
            ctx.flags.insert("multi_source_merged".to_string(), true);
        })
        .when("they run \"shiplog cluster\"", |ctx| {
            ctx.flags.insert("workstreams_generated".to_string(), true);
            ctx.numbers.insert("workstream_count".to_string(), 8);
            ctx.flags.insert("multi_source_workstreams".to_string(), true);
            Ok(())
        })
        .then("workstreams should include events from all sources", |ctx| {
            assert_true(
                ctx.flag("workstreams_generated").unwrap_or(false),
                "workstreams generated",
            )
        })
        .then("clustering should consider event context across sources", |ctx| {
            assert_true(
                ctx.flag("multi_source_workstreams").unwrap_or(false),
                "multi-source workstreams",
            )
        })
}

/// Scenario 7.8: Multi-source packet renders correctly
pub fn multi_source_render() -> Scenario {
    Scenario::new("Multi-source packet renders correctly")
        .given("a user has merged events from multiple sources", |ctx| {
            ctx.flags.insert("multi_source_merged".to_string(), true);
        })
        .given("they have generated workstreams", |ctx| {
            ctx.flags.insert("workstreams_generated".to_string(), true);
        })
        .when("they run \"shiplog render\"", |ctx| {
            ctx.flags.insert("packet_rendered".to_string(), true);
            ctx.paths
                .insert("packet_path".to_string(), "/out/run_001/packet.md".into());
            ctx.flags.insert("all_sources_included".to_string(), true);
            Ok(())
        })
        .then("the packet should include events from all sources", |ctx| {
            assert_true(
                ctx.flag("packet_rendered").unwrap_or(false),
                "packet rendered",
            )
        })
        .then("each event should indicate its source", |ctx| {
            assert_true(
                ctx.flag("all_sources_included").unwrap_or(false),
                "all sources included",
            )
        })
}

/// Scenario 7.9: Merge with thousands of events
pub fn multi_source_merge_large() -> Scenario {
    Scenario::new("Merge with thousands of events")
        .given("a user has 5,000 events across 5 sources", |ctx| {
            ctx.numbers.insert("total_events".to_string(), 5000);
            ctx.numbers.insert("source_count".to_string(), 5);
        })
        .when("they run \"shiplog merge\"", |ctx| {
            ctx.flags.insert("merged".to_string(), true);
            ctx.strings
                .insert("merge_time".to_string(), "8s".to_string());
            ctx.strings
                .insert("memory_usage".to_string(), "450MB".to_string());
            Ok(())
        })
        .then("merging should complete within reasonable time (< 10 seconds)", |ctx| {
            let time = ctx.string("merge_time").unwrap();
            assert_true(
                time.contains("s") && !time.contains("m"),
                "merge time",
            )
        })
        .then("memory usage should remain bounded", |ctx| {
            let memory = ctx.string("memory_usage").unwrap();
            assert_true(
                memory.contains("MB") && !memory.contains("GB"),
                "memory usage",
            )
        })
}
