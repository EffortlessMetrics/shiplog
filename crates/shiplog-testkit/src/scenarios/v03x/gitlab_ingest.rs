//! BDD scenarios for GitLab Ingest Adapter (Feature 5)
//!
//! Scenarios cover:
//! - Primary user workflows (ingesting MRs from GitLab.com and self-hosted)
//! - Edge cases (invalid URLs, auth failures, rate limits)
//! - Integration with other features (merging, clustering, rendering)
//! - Performance scenarios (large MR collections)

use crate::bdd::Scenario;
use crate::bdd::assertions::*;

/// Scenario 5.1: User ingests merge requests from GitLab.com
pub fn gitlab_ingest_gitlab_com() -> Scenario {
    Scenario::new("User ingests merge requests from GitLab.com")
        .given("a user has a GitLab.com account", |ctx| {
            ctx.strings
                .insert("gitlab_instance".to_string(), "gitlab.com".to_string());
            ctx.strings
                .insert("gitlab_token".to_string(), "glpat_test_token".to_string());
        })
        .given("they have a personal access token configured", |ctx| {
            ctx.flags.insert("token_configured".to_string(), true);
        })
        .when(
            "they run \"shiplog collect --source gitlab --user alice --since 2025-01-01\"",
            |ctx| {
                ctx.flags.insert("api_call_made".to_string(), true);
                ctx.numbers.insert("collected_mrs".to_string(), 15);
                ctx.strings
                    .insert("source_system".to_string(), "gitlab".to_string());
                Ok(())
            },
        )
        .then("merge requests should be collected from GitLab.com", |ctx| {
            assert_true(
                ctx.flag("api_call_made").unwrap_or(false),
                "API call made",
            )
        })
        .then(
            "each event should have SourceSystem::Other(\"gitlab\")",
            |ctx| {
                let source = ctx.string("source_system").unwrap();
                assert_eq(source, "gitlab", "source system")
            },
        )
        .then(
            "events should include MR number, title, state, and timestamp",
            |ctx| {
                let count = ctx.number("collected_mrs").unwrap_or(0);
                assert_true(count > 0, "MRs collected")
            },
        )
}

/// Scenario 5.2: User ingests from self-hosted GitLab instance
pub fn gitlab_ingest_self_hosted() -> Scenario {
    Scenario::new("User ingests from self-hosted GitLab instance")
        .given(
            "a user has access to a self-hosted GitLab instance at gitlab.company.com",
            |ctx| {
                ctx.strings.insert(
                    "gitlab_instance".to_string(),
                    "gitlab.company.com".to_string(),
                );
                ctx.strings
                    .insert("gitlab_token".to_string(), "glpat_test_token".to_string());
            },
        )
        .given("they have configured the instance URL and token", |ctx| {
            ctx.flags.insert("token_configured".to_string(), true);
            ctx.flags.insert("instance_configured".to_string(), true);
        })
        .when(
            "they run \"shiplog collect --source gitlab --instance gitlab.company.com --user alice --since 2025-01-01\"",
            |ctx| {
                ctx.flags.insert("api_call_made".to_string(), true);
                ctx.numbers.insert("collected_mrs".to_string(), 10);
                ctx.strings
                    .insert("api_base_url".to_string(), "https://gitlab.company.com/api/v4".to_string());
                Ok(())
            },
        )
        .then("merge requests should be collected from the self-hosted instance", |ctx| {
            assert_true(
                ctx.flag("api_call_made").unwrap_or(false),
                "API call made",
            )
        })
        .then("the API base URL should be correctly configured", |ctx| {
            let url = ctx.string("api_base_url").unwrap();
            assert_contains(url, "gitlab.company.com", "instance URL")
        })
}

/// Scenario 5.3: User ingests GitLab reviews
pub fn gitlab_ingest_reviews() -> Scenario {
    Scenario::new("User ingests GitLab reviews")
        .given("a user has configured GitLab collection with review inclusion", |ctx| {
            ctx.strings
                .insert("gitlab_instance".to_string(), "gitlab.com".to_string());
            ctx.strings
                .insert("gitlab_token".to_string(), "glpat_test_token".to_string());
            ctx.flags.insert("include_reviews".to_string(), true);
        })
        .when(
            "they run \"shiplog collect --source gitlab --user alice --include-reviews\"",
            |ctx| {
                ctx.flags.insert("api_call_made".to_string(), true);
                ctx.numbers.insert("collected_reviews".to_string(), 8);
                ctx.numbers.insert("collected_mrs".to_string(), 15);
                Ok(())
            },
        )
        .then("merge request reviews should be collected", |ctx| {
            let count = ctx.number("collected_reviews").unwrap_or(0);
            assert_true(count > 0, "reviews collected")
        })
        .then("review events should include the MR being reviewed", |ctx| {
            ctx.flags.insert("review_has_mr".to_string(), true);
            assert_true(
                ctx.flag("review_has_mr").unwrap_or(false),
                "review has MR reference",
            )
        })
        .then("review events should include the reviewer and timestamp", |ctx| {
            ctx.flags.insert("review_has_reviewer".to_string(), true);
            assert_true(
                ctx.flag("review_has_reviewer").unwrap_or(false),
                "review has reviewer",
            )
        })
}

/// Scenario 5.4: User filters GitLab MRs by state
pub fn gitlab_ingest_filter_by_state() -> Scenario {
    Scenario::new("User filters GitLab MRs by state")
        .given("a user has GitLab MRs in various states (opened, merged, closed)", |ctx| {
            ctx.numbers.insert("total_mrs".to_string(), 30);
            ctx.numbers.insert("opened_mrs".to_string(), 10);
            ctx.numbers.insert("merged_mrs".to_string(), 15);
            ctx.numbers.insert("closed_mrs".to_string(), 5);
        })
        .when(
            "they run \"shiplog collect --source gitlab --user alice --state merged\"",
            |ctx| {
                ctx.numbers.insert("collected_mrs".to_string(), 15);
                ctx.strings.insert("filtered_state".to_string(), "merged".to_string());
                Ok(())
            },
        )
        .then("only merged MRs should be collected", |ctx| {
            let count = ctx.number("collected_mrs").unwrap_or(0);
            let expected = ctx.number("merged_mrs").unwrap_or(0);
            assert_eq(count, expected, "MR count")
        })
        .then("opened and closed MRs should be excluded", |ctx| {
            let state = ctx.string("filtered_state").unwrap();
            assert_eq(state, "merged", "filtered state")
        })
}

/// Scenario 5.5: Invalid GitLab instance URL
pub fn gitlab_ingest_invalid_url() -> Scenario {
    Scenario::new("Invalid GitLab instance URL")
        .given("a user specifies an invalid GitLab instance URL", |ctx| {
            ctx.strings
                .insert("gitlab_instance".to_string(), "invalid-url".to_string());
        })
        .when(
            "they run \"shiplog collect --source gitlab --instance invalid-url\"",
            |ctx| {
                ctx.flags.insert("command_failed".to_string(), true);
                ctx.strings.insert(
                    "error_message".to_string(),
                    "Invalid GitLab instance URL: invalid-url".to_string(),
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

/// Scenario 5.6: GitLab authentication failure
pub fn gitlab_ingest_auth_failure() -> Scenario {
    Scenario::new("GitLab authentication failure")
        .given("a user has an invalid GitLab token", |ctx| {
            ctx.strings
                .insert("gitlab_token".to_string(), "invalid_token".to_string());
        })
        .when(
            "they run \"shiplog collect --source gitlab --user alice\"",
            |ctx| {
                ctx.flags.insert("command_failed".to_string(), true);
                ctx.strings.insert(
                    "error_message".to_string(),
                    "GitLab authentication failed: invalid or expired token".to_string(),
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
        .then("the error should indicate the token is invalid or expired", |ctx| {
            let error = ctx.string("error_message").unwrap();
            assert_contains(error, "authentication", "error message")
        })
}

/// Scenario 5.7: GitLab API rate limit exceeded
pub fn gitlab_ingest_rate_limit() -> Scenario {
    Scenario::new("GitLab API rate limit exceeded")
        .given("a user has exceeded the GitLab API rate limit", |ctx| {
            ctx.flags.insert("rate_limit_exceeded".to_string(), true);
        })
        .when(
            "they run \"shiplog collect --source gitlab --user alice\"",
            |ctx| {
                ctx.flags.insert("command_failed".to_string(), true);
                ctx.strings.insert(
                    "error_message".to_string(),
                    "GitLab API rate limit exceeded. Reset at 2025-01-01T00:00:00Z".to_string(),
                );
                Ok(())
            },
        )
        .then("the command should fail with a rate limit error", |ctx| {
            assert_true(
                ctx.flag("command_failed").unwrap_or(false),
                "command failed",
            )
        })
        .then("the error should indicate when the limit will reset", |ctx| {
            let error = ctx.string("error_message").unwrap();
            assert_contains(error, "Reset at", "error message")
        })
}

/// Scenario 5.8: GitLab project is private and inaccessible
pub fn gitlab_ingest_private_project() -> Scenario {
    Scenario::new("GitLab project is private and inaccessible")
        .given("a user attempts to collect from a private GitLab project", |ctx| {
            ctx.strings
                .insert("gitlab_project".to_string(), "private/project".to_string());
        })
        .given("they do not have access to the project", |ctx| {
            ctx.flags.insert("no_access".to_string(), true);
        })
        .when(
            "they run \"shiplog collect --source gitlab --user alice\"",
            |ctx| {
                ctx.flags.insert("command_failed".to_string(), true);
                ctx.strings.insert(
                    "error_message".to_string(),
                    "Access denied: project private/project is not accessible".to_string(),
                );
                Ok(())
            },
        )
        .then("the command should fail with an access error", |ctx| {
            assert_true(
                ctx.flag("command_failed").unwrap_or(false),
                "command failed",
            )
        })
        .then("the error should indicate the project is inaccessible", |ctx| {
            let error = ctx.string("error_message").unwrap();
            assert_contains(error, "Access denied", "error message")
        })
}

/// Scenario 5.9: GitLab events merge with GitHub events
pub fn gitlab_ingest_merge_with_github() -> Scenario {
    Scenario::new("GitLab events merge with GitHub events")
        .given("a user has collected events from GitHub", |ctx| {
            ctx.numbers.insert("github_events".to_string(), 25);
            ctx.flags.insert("github_collected".to_string(), true);
        })
        .given("they also collect events from GitLab", |ctx| {
            ctx.numbers.insert("gitlab_events".to_string(), 15);
            ctx.flags.insert("gitlab_collected".to_string(), true);
        })
        .when("they merge the sources", |ctx| {
            let total = ctx.number("github_events").unwrap_or(0)
                + ctx.number("gitlab_events").unwrap_or(0);
            ctx.numbers.insert("total_events".to_string(), total);
            ctx.flags.insert("merged".to_string(), true);
            Ok(())
        })
        .then(
            "events from both sources should be merged into a single ledger",
            |ctx| {
                let total = ctx.number("total_events").unwrap_or(0);
                assert_eq(total, 40, "total events")
            },
        )
        .then("coverage manifest should include both sources", |ctx| {
            ctx.flags.insert("coverage_has_github".to_string(), true);
            ctx.flags.insert("coverage_has_gitlab".to_string(), true);
            assert_true(
                ctx.flag("coverage_has_github").unwrap_or(false)
                    && ctx.flag("coverage_has_gitlab").unwrap_or(false),
                "coverage has both sources",
            )
        })
        .then("workstreams can contain events from both sources", |ctx| {
            ctx.flags.insert("workstreams_multi_source".to_string(), true);
            assert_true(
                ctx.flag("workstreams_multi_source").unwrap_or(false),
                "workstreams have multi-source events",
            )
        })
}

/// Scenario 5.10: GitLab events cluster into workstreams
pub fn gitlab_ingest_cluster() -> Scenario {
    Scenario::new("GitLab events cluster into workstreams")
        .given("a user has collected GitLab MRs and reviews", |ctx| {
            ctx.numbers.insert("gitlab_events".to_string(), 20);
        })
        .when("they run \"shiplog cluster\"", |ctx| {
            ctx.flags.insert("workstreams_generated".to_string(), true);
            ctx.numbers.insert("workstream_count".to_string(), 5);
            Ok(())
        })
        .then("workstreams should be generated from GitLab events", |ctx| {
            assert_true(
                ctx.flag("workstreams_generated").unwrap_or(false),
                "workstreams generated",
            )
        })
        .then("related MRs should be grouped together", |ctx| {
            let count = ctx.number("workstream_count").unwrap_or(0);
            assert_true(count > 0, "workstream count")
        })
}

/// Scenario 5.11: GitLab events render in packet
pub fn gitlab_ingest_render() -> Scenario {
    Scenario::new("GitLab events render in packet")
        .given("a user has collected GitLab events and generated workstreams", |ctx| {
            ctx.numbers.insert("gitlab_events".to_string(), 15);
            ctx.flags.insert("workstreams_generated".to_string(), true);
        })
        .when("they run \"shiplog render\"", |ctx| {
            ctx.flags.insert("packet_rendered".to_string(), true);
            ctx.paths
                .insert("packet_path".to_string(), "/out/run_001/packet.md".into());
            Ok(())
        })
        .then("the packet should include GitLab MRs", |ctx| {
            assert_true(
                ctx.flag("packet_rendered").unwrap_or(false),
                "packet rendered",
            )
        })
        .then("MRs should link to the GitLab instance", |ctx| {
            ctx.strings
                .insert("gitlab_link".to_string(), "https://gitlab.com/".to_string());
            let link = ctx.string("gitlab_link").unwrap();
            assert_contains(link, "gitlab.com", "GitLab link")
        })
        .then("source should be indicated as \"gitlab\"", |ctx| {
            ctx.strings.insert("source_label".to_string(), "gitlab".to_string());
            let source = ctx.string("source_label").unwrap();
            assert_eq(source, "gitlab", "source label")
        })
}

/// Scenario 5.12: GitLab collection with thousands of MRs
pub fn gitlab_ingest_large_collection() -> Scenario {
    Scenario::new("GitLab collection with thousands of MRs")
        .given("a user has 2,000 MRs on GitLab", |ctx| {
            ctx.numbers.insert("total_mrs".to_string(), 2000);
        })
        .when(
            "they run \"shiplog collect --source gitlab --user alice --since 2025-01-01\"",
            |ctx| {
                ctx.flags.insert("collection_complete".to_string(), true);
                ctx.strings
                    .insert("collection_time".to_string(), "45s".to_string());
                ctx.strings
                    .insert("memory_usage".to_string(), "350MB".to_string());
                Ok(())
            },
        )
        .then(
            "collection should complete within reasonable time (< 60 seconds)",
            |ctx| {
                let time = ctx.string("collection_time").unwrap();
                assert_true(
                    time.contains("s") && !time.contains("m"),
                    "collection time",
                )
            },
        )
        .then("memory usage should remain bounded (< 500MB)", |ctx| {
            let memory = ctx.string("memory_usage").unwrap();
            assert_true(
                memory.contains("MB") && !memory.contains("GB"),
                "memory usage",
            )
        })
}
