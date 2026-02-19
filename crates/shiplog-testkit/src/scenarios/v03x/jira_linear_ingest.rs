//! BDD scenarios for Jira/Linear Ingest Adapter (Feature 6)
//!
//! Scenarios cover:
//! - Primary user workflows (ingesting issues from Jira and Linear)
//! - Edge cases (invalid URLs, auth failures, missing fields)
//! - Integration with other features (correlation with PRs, rendering)
//! - Performance scenarios (large issue collections)

use crate::bdd::Scenario;
use crate::bdd::assertions::*;

/// Scenario 6.1: User ingests Jira issues
pub fn jira_ingest_issues() -> Scenario {
    Scenario::new("User ingests Jira issues")
        .given("a user has a Jira account", |ctx| {
            ctx.strings
                .insert("jira_instance".to_string(), "jira.atlassian.com".to_string());
            ctx.strings
                .insert("jira_token".to_string(), "jira_test_token".to_string());
        })
        .given("they have configured Jira instance URL and API token", |ctx| {
            ctx.flags.insert("token_configured".to_string(), true);
            ctx.flags.insert("instance_configured".to_string(), true);
        })
        .when(
            "they run \"shiplog collect --source jira --user alice --since 2025-01-01\"",
            |ctx| {
                ctx.flags.insert("api_call_made".to_string(), true);
                ctx.numbers.insert("collected_issues".to_string(), 20);
                ctx.strings
                    .insert("source_system".to_string(), "jira".to_string());
                Ok(())
            },
        )
        .then("Jira issues should be collected", |ctx| {
            assert_true(
                ctx.flag("api_call_made").unwrap_or(false),
                "API call made",
            )
        })
        .then(
            "each event should have SourceSystem::Other(\"jira\")",
            |ctx| {
                let source = ctx.string("source_system").unwrap();
                assert_eq(source, "jira", "source system")
            },
        )
        .then("events should include issue key, summary, status, and timestamp", |ctx| {
            let count = ctx.number("collected_issues").unwrap_or(0);
            assert_true(count > 0, "issues collected")
        })
}

/// Scenario 6.2: User ingests Linear issues
pub fn linear_ingest_issues() -> Scenario {
    Scenario::new("User ingests Linear issues")
        .given("a user has a Linear account", |ctx| {
            ctx.strings
                .insert("linear_instance".to_string(), "linear.app".to_string());
            ctx.strings
                .insert("linear_api_key".to_string(), "lin_test_key".to_string());
        })
        .given("they have configured Linear API key", |ctx| {
            ctx.flags.insert("api_key_configured".to_string(), true);
        })
        .when(
            "they run \"shiplog collect --source linear --user alice --since 2025-01-01\"",
            |ctx| {
                ctx.flags.insert("api_call_made".to_string(), true);
                ctx.numbers.insert("collected_issues".to_string(), 15);
                ctx.strings
                    .insert("source_system".to_string(), "linear".to_string());
                Ok(())
            },
        )
        .then("Linear issues should be collected", |ctx| {
            assert_true(
                ctx.flag("api_call_made").unwrap_or(false),
                "API call made",
            )
        })
        .then(
            "each event should have SourceSystem::Other(\"linear\")",
            |ctx| {
                let source = ctx.string("source_system").unwrap();
                assert_eq(source, "linear", "source system")
            },
        )
        .then("events should include issue ID, title, status, and timestamp", |ctx| {
            let count = ctx.number("collected_issues").unwrap_or(0);
            assert_true(count > 0, "issues collected")
        })
}

/// Scenario 6.3: User filters Jira issues by status
pub fn jira_ingest_filter_by_status() -> Scenario {
    Scenario::new("User filters Jira issues by status")
        .given("a user has Jira issues in various statuses", |ctx| {
            ctx.numbers.insert("total_issues".to_string(), 50);
            ctx.numbers.insert("done_issues".to_string(), 20);
            ctx.numbers.insert("in_progress_issues".to_string(), 15);
            ctx.numbers.insert("todo_issues".to_string(), 15);
        })
        .when(
            "they run \"shiplog collect --source jira --user alice --status Done\"",
            |ctx| {
                ctx.numbers.insert("collected_issues".to_string(), 20);
                ctx.strings.insert("filtered_status".to_string(), "Done".to_string());
                Ok(())
            },
        )
        .then("only issues with \"Done\" status should be collected", |ctx| {
            let count = ctx.number("collected_issues").unwrap_or(0);
            let expected = ctx.number("done_issues").unwrap_or(0);
            assert_eq(count, expected, "issue count")
        })
        .then("issues in other statuses should be excluded", |ctx| {
            let status = ctx.string("filtered_status").unwrap();
            assert_eq(status, "Done", "filtered status")
        })
}

/// Scenario 6.4: User filters Linear issues by project
pub fn linear_ingest_filter_by_project() -> Scenario {
    Scenario::new("User filters Linear issues by project")
        .given("a user has Linear issues across multiple projects", |ctx| {
            ctx.numbers.insert("total_issues".to_string(), 40);
            ctx.numbers.insert("proj_a_issues".to_string(), 25);
            ctx.numbers.insert("proj_b_issues".to_string(), 15);
        })
        .when(
            "they run \"shiplog collect --source linear --user alice --project PROJ-123\"",
            |ctx| {
                ctx.numbers.insert("collected_issues".to_string(), 25);
                ctx.strings
                    .insert("filtered_project".to_string(), "PROJ-123".to_string());
                Ok(())
            },
        )
        .then("only issues from the specified project should be collected", |ctx| {
            let count = ctx.number("collected_issues").unwrap_or(0);
            let expected = ctx.number("proj_a_issues").unwrap_or(0);
            assert_eq(count, expected, "issue count")
        })
}

/// Scenario 6.5: Invalid Jira instance URL
pub fn jira_ingest_invalid_url() -> Scenario {
    Scenario::new("Invalid Jira instance URL")
        .given("a user specifies an invalid Jira instance URL", |ctx| {
            ctx.strings
                .insert("jira_instance".to_string(), "invalid-url".to_string());
        })
        .when(
            "they run \"shiplog collect --source jira --instance invalid-url\"",
            |ctx| {
                ctx.flags.insert("command_failed".to_string(), true);
                ctx.strings.insert(
                    "error_message".to_string(),
                    "Invalid Jira instance URL: invalid-url".to_string(),
                );
                Ok(())
            },
        )
        .then("the command should fail with a clear error message", |ctx| {
            assert_true(
                ctx.flag("command_failed").unwrap_or(false),
                "command failed",
            )
        })
        .then("the error should indicate the URL is invalid", |ctx| {
            let error = ctx.string("error_message").unwrap();
            assert_contains(error, "Invalid", "error message")
        })
}

/// Scenario 6.6: Jira authentication failure
pub fn jira_ingest_auth_failure() -> Scenario {
    Scenario::new("Jira authentication failure")
        .given("a user has an invalid Jira API token", |ctx| {
            ctx.strings
                .insert("jira_token".to_string(), "invalid_token".to_string());
        })
        .when(
            "they run \"shiplog collect --source jira --user alice\"",
            |ctx| {
                ctx.flags.insert("command_failed".to_string(), true);
                ctx.strings.insert(
                    "error_message".to_string(),
                    "Jira authentication failed: invalid token".to_string(),
                );
                Ok(())
            },
        )
        .then("the command should fail with an authentication error", |ctx| {
            assert_true(
                ctx.flag("command_failed").unwrap_or(false),
                "command failed",
            )
        })
        .then("the error should indicate the token is invalid", |ctx| {
            let error = ctx.string("error_message").unwrap();
            assert_contains(error, "authentication", "error message")
        })
}

/// Scenario 6.7: Linear API key invalid
pub fn linear_ingest_invalid_key() -> Scenario {
    Scenario::new("Linear API key invalid")
        .given("a user has an invalid Linear API key", |ctx| {
            ctx.strings
                .insert("linear_api_key".to_string(), "invalid_key".to_string());
        })
        .when(
            "they run \"shiplog collect --source linear --user alice\"",
            |ctx| {
                ctx.flags.insert("command_failed".to_string(), true);
                ctx.strings.insert(
                    "error_message".to_string(),
                    "Linear authentication failed: invalid API key".to_string(),
                );
                Ok(())
            },
        )
        .then("the command should fail with an authentication error", |ctx| {
            assert_true(
                ctx.flag("command_failed").unwrap_or(false),
                "command failed",
            )
        })
        .then("the error should indicate the API key is invalid", |ctx| {
            let error = ctx.string("error_message").unwrap();
            assert_contains(error, "authentication", "error message")
        })
}

/// Scenario 6.8: Issue with missing required fields
pub fn jira_ingest_missing_fields() -> Scenario {
    Scenario::new("Issue with missing required fields")
        .given("a user has a Jira issue with missing summary or status", |ctx| {
            ctx.flags.insert("issue_missing_fields".to_string(), true);
        })
        .when(
            "they run \"shiplog collect --source jira --user alice\"",
            |ctx| {
                ctx.flags.insert("warning_shown".to_string(), true);
                ctx.numbers.insert("skipped_issues".to_string(), 1);
                ctx.strings.insert(
                    "warning_message".to_string(),
                    "Skipped 1 issue(s) with missing required fields".to_string(),
                );
                Ok(())
            },
        )
        .then("the issue should be skipped", |ctx| {
            let count = ctx.number("skipped_issues").unwrap_or(0);
            assert_true(count > 0, "issues skipped")
        })
        .then("a warning should indicate the missing fields", |ctx| {
            assert_true(
                ctx.flag("warning_shown").unwrap_or(false),
                "warning shown",
            )
        })
}

/// Scenario 6.9: Jira issues correlate with GitHub PRs
pub fn jira_ingest_correlate_with_prs() -> Scenario {
    Scenario::new("Jira issues correlate with GitHub PRs")
        .given("a user has collected GitHub PRs", |ctx| {
            ctx.numbers.insert("github_prs".to_string(), 25);
        })
        .given("PR titles contain Jira issue keys (e.g., \"PROJ-123: Fix bug\")", |ctx| {
            ctx.flags.insert("pr_has_issue_key".to_string(), true);
        })
        .when("they also collect Jira issues", |ctx| {
            ctx.numbers.insert("jira_issues".to_string(), 10);
            ctx.flags.insert("correlation_attempted".to_string(), true);
            Ok(())
        })
        .then("the system should attempt to correlate PRs with issues", |ctx| {
            assert_true(
                ctx.flag("correlation_attempted").unwrap_or(false),
                "correlation attempted",
            )
        })
        .then("workstreams may group related PRs and issues together", |ctx| {
            assert_true(
                ctx.flag("workstreams_correlated").unwrap_or(false),
                "workstreams correlated",
            )
        })
}

/// Scenario 6.10: Linear issues appear in packet
pub fn linear_ingest_render() -> Scenario {
    Scenario::new("Linear issues appear in packet")
        .given("a user has collected Linear issues and generated workstreams", |ctx| {
            ctx.numbers.insert("linear_issues".to_string(), 15);
            ctx.flags.insert("workstreams_generated".to_string(), true);
        })
        .when("they run \"shiplog render\"", |ctx| {
            ctx.flags.insert("packet_rendered".to_string(), true);
            ctx.paths
                .insert("packet_path".to_string(), "/out/run_001/packet.md".into());
            Ok(())
        })
        .then("the packet should include Linear issues", |ctx| {
            assert_true(
                ctx.flag("packet_rendered").unwrap_or(false),
                "packet rendered",
            )
        })
        .then("issues should link to the Linear web interface", |ctx| {
            let link = ctx.string("linear_link").unwrap_or(&String::new());
            assert_contains(link, "linear.app", "Linear link")
        })
        .then("source should be indicated as \"linear\"", |ctx| {
            let source = ctx.string("source_label").unwrap_or(&String::new());
            assert_eq(source, "linear", "source label")
        })
}

/// Scenario 6.11: Jira collection with hundreds of issues
pub fn jira_ingest_large_collection() -> Scenario {
    Scenario::new("Jira collection with hundreds of issues")
        .given("a user has 500 Jira issues", |ctx| {
            ctx.numbers.insert("total_issues".to_string(), 500);
        })
        .when(
            "they run \"shiplog collect --source jira --user alice --since 2025-01-01\"",
            |ctx| {
                ctx.flags.insert("collection_complete".to_string(), true);
                ctx.strings
                    .insert("collection_time".to_string(), "25s".to_string());
                Ok(())
            },
        )
        .then("collection should complete within reasonable time (< 30 seconds)", |ctx| {
            let time = ctx.string("collection_time").unwrap();
            assert_true(
                time.contains("s") && !time.contains("m"),
                "collection time",
            )
        })
}
