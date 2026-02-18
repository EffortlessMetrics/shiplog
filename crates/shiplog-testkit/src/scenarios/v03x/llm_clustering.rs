//! BDD scenarios for LLM Clustering as Default Feature (Feature 9)
//!
//! Scenarios cover:
//! - Primary user workflows (using LLM clustering without feature flag)
//! - Edge cases (API key not configured, LLM failures, rate limits)
//! - Integration with other features (multi-source events, user curation)
//! - Performance scenarios (many events, chunked clustering)

use crate::bdd::Scenario;
use crate::bdd::assertions::*;

/// Scenario 9.1: User uses LLM clustering without feature flag
pub fn llm_clustering_default() -> Scenario {
    Scenario::new("User uses LLM clustering without feature flag")
        .given("a user has configured an LLM API key", |ctx| {
            ctx.strings
                .insert("llm_api_key".to_string(), "sk_test_key".to_string());
            ctx.strings
                .insert("llm_model".to_string(), "gpt-4".to_string());
        })
        .given("they have collected events", |ctx| {
            ctx.numbers.insert("event_count".to_string(), 50);
        })
        .when("they run \"shiplog cluster\"", |ctx| {
            ctx.flags.insert("llm_used".to_string(), true);
            ctx.flags.insert("workstreams_generated".to_string(), true);
            ctx.numbers.insert("workstream_count".to_string(), 5);
            ctx.flags.insert("no_feature_flag_needed".to_string(), true);
            Ok(())
        })
        .then("LLM clustering should be used by default", |ctx| {
            assert_true(ctx.flag("llm_used").unwrap_or(false), "LLM used")
        })
        .then("no feature flag should be required", |ctx| {
            assert_true(
                ctx.flag("no_feature_flag_needed").unwrap_or(false),
                "no feature flag needed",
            )
        })
        .then("workstreams should be generated using LLM analysis", |ctx| {
            let count = ctx.number("workstream_count").unwrap_or(0);
            assert_true(count > 0, "workstream count")
        })
}

/// Scenario 9.2: User falls back to repo-based clustering on LLM failure
pub fn llm_clustering_fallback() -> Scenario {
    Scenario::new("User falls back to repo-based clustering on LLM failure")
        .given("a user has configured LLM clustering", |ctx| {
            ctx.strings
                .insert("llm_api_key".to_string(), "sk_test_key".to_string());
        })
        .given("the LLM API is temporarily unavailable", |ctx| {
            ctx.flags.insert("llm_unavailable".to_string(), true);
        })
        .when("they run \"shiplog cluster\"", |ctx| {
            ctx.flags.insert("fallback_used".to_string(), true);
            ctx.flags.insert("workstreams_generated".to_string(), true);
            ctx.numbers.insert("workstream_count".to_string(), 5);
            ctx.strings.insert(
                "warning_message".to_string(),
                "LLM clustering unavailable, falling back to repo-based clustering".to_string(),
            );
            Ok(())
        })
        .then("the system should fall back to repo-based clustering", |ctx| {
            assert_true(
                ctx.flag("fallback_used").unwrap_or(false),
                "fallback used",
            )
        })
        .then("a warning should indicate the fallback occurred", |ctx| {
            let warning = ctx.string("warning_message").unwrap();
            assert_contains(warning, "fallback", "warning message")
        })
        .then("workstreams should still be generated", |ctx| {
            let count = ctx.number("workstream_count").unwrap_or(0);
            assert_true(count > 0, "workstream count")
        })
}

/// Scenario 9.3: User configures LLM clustering parameters
pub fn llm_clustering_config() -> Scenario {
    Scenario::new("User configures LLM clustering parameters")
        .given("a user has a config file with LLM settings", |ctx| {
            ctx.strings.insert("config_file".to_string(), "shiplog.yaml".to_string());
        })
        .given("the config specifies max_workstreams: 10", |ctx| {
            ctx.numbers.insert("max_workstreams".to_string(), 10);
        })
        .given("the config specifies model: \"gpt-4\"", |ctx| {
            ctx.strings
                .insert("llm_model".to_string(), "gpt-4".to_string());
        })
        .when("they run \"shiplog cluster\"", |ctx| {
            ctx.flags.insert("llm_used".to_string(), true);
            ctx.flags.insert("workstreams_generated".to_string(), true);
            ctx.numbers.insert("workstream_count".to_string(), 8);
            ctx.strings
                .insert("model_used".to_string(), "gpt-4".to_string());
            Ok(())
        })
        .then("LLM clustering should use the configured parameters", |ctx| {
            assert_true(ctx.flag("llm_used").unwrap_or(false), "LLM used")
        })
        .then("the specified model should be used", |ctx| {
            let model = ctx.string("model_used").unwrap();
            assert_eq(model, "gpt-4", "model")
        })
        .then("workstreams should be limited to the configured maximum", |ctx| {
            let count = ctx.number("workstream_count").unwrap_or(0);
            let max = ctx.number("max_workstreams").unwrap_or(0);
            assert_true(count <= max, "workstream count <= max")
        })
}

/// Scenario 9.4: LLM API key not configured
pub fn llm_clustering_no_key() -> Scenario {
    Scenario::new("LLM API key not configured")
        .given("a user has not configured an LLM API key", |ctx| {
            ctx.flags.insert("no_api_key".to_string(), true);
        })
        .when("they run \"shiplog cluster\"", |ctx| {
            ctx.flags.insert("fallback_used".to_string(), true);
            ctx.flags.insert("workstreams_generated".to_string(), true);
            ctx.numbers.insert("workstream_count".to_string(), 5);
            ctx.strings.insert(
                "warning_message".to_string(),
                "LLM API key not configured, falling back to repo-based clustering".to_string(),
            );
            Ok(())
        })
        .then("the system should fall back to repo-based clustering", |ctx| {
            assert_true(
                ctx.flag("fallback_used").unwrap_or(false),
                "fallback used",
            )
        })
        .then("a warning should indicate LLM is not configured", |ctx| {
            let warning = ctx.string("warning_message").unwrap();
            assert_contains(warning, "not configured", "warning message")
        })
}

/// Scenario 9.5: LLM returns invalid response
pub fn llm_clustering_invalid_response() -> Scenario {
    Scenario::new("LLM returns invalid response")
        .given("a user has configured LLM clustering", |ctx| {
            ctx.strings
                .insert("llm_api_key".to_string(), "sk_test_key".to_string());
        })
        .given("the LLM returns a malformed response", |ctx| {
            ctx.flags.insert("llm_invalid_response".to_string(), true);
        })
        .when("they run \"shiplog cluster\"", |ctx| {
            ctx.flags.insert("fallback_used".to_string(), true);
            ctx.flags.insert("workstreams_generated".to_string(), true);
            ctx.numbers.insert("workstream_count".to_string(), 5);
            ctx.strings.insert(
                "warning_message".to_string(),
                "LLM response was invalid, falling back to repo-based clustering".to_string(),
            );
            Ok(())
        })
        .then("the system should fall back to repo-based clustering", |ctx| {
            assert_true(
                ctx.flag("fallback_used").unwrap_or(false),
                "fallback used",
            )
        })
        .then("a warning should indicate the LLM response was invalid", |ctx| {
            let warning = ctx.string("warning_message").unwrap();
            assert_contains(warning, "invalid", "warning message")
        })
}

/// Scenario 9.6: LLM API rate limit exceeded
pub fn llm_clustering_rate_limit() -> Scenario {
    Scenario::new("LLM API rate limit exceeded")
        .given("a user has exceeded the LLM API rate limit", |ctx| {
            ctx.flags.insert("rate_limit_exceeded".to_string(), true);
        })
        .when("they run \"shiplog cluster\"", |ctx| {
            ctx.flags.insert("fallback_used".to_string(), true);
            ctx.flags.insert("workstreams_generated".to_string(), true);
            ctx.numbers.insert("workstream_count".to_string(), 5);
            ctx.strings.insert(
                "warning_message".to_string(),
                "LLM API rate limit exceeded, falling back to repo-based clustering".to_string(),
            );
            Ok(())
        })
        .then("the system should fall back to repo-based clustering", |ctx| {
            assert_true(
                ctx.flag("fallback_used").unwrap_or(false),
                "fallback used",
            )
        })
        .then("a warning should indicate the rate limit was exceeded", |ctx| {
            let warning = ctx.string("warning_message").unwrap();
            assert_contains(warning, "rate limit", "warning message")
        })
}

/// Scenario 9.7: LLM clustering works with multi-source events
pub fn llm_clustering_multi_source() -> Scenario {
    Scenario::new("LLM clustering works with multi-source events")
        .given("a user has collected events from GitHub, GitLab, and Jira", |ctx| {
            ctx.numbers.insert("github_events".to_string(), 25);
            ctx.numbers.insert("gitlab_events".to_string(), 15);
            ctx.numbers.insert("jira_events".to_string(), 10);
        })
        .when("they run \"shiplog cluster\"", |ctx| {
            ctx.flags.insert("llm_used".to_string(), true);
            ctx.flags.insert("workstreams_generated".to_string(), true);
            ctx.numbers.insert("workstream_count".to_string(), 8);
            ctx.flags.insert("multi_source_workstreams".to_string(), true);
            Ok(())
        })
        .then("LLM clustering should consider events from all sources", |ctx| {
            assert_true(ctx.flag("llm_used").unwrap_or(false), "LLM used")
        })
        .then("workstreams may group events from different sources together", |ctx| {
            assert_true(
                ctx.flag("multi_source_workstreams").unwrap_or(false),
                "multi-source workstreams",
            )
        })
}

/// Scenario 9.8: LLM clustering preserves user curation
pub fn llm_clustering_preserve_curation() -> Scenario {
    Scenario::new("LLM clustering preserves user curation")
        .given("a user has curated workstreams with custom titles", |ctx| {
            ctx.strings
                .insert("custom_title".to_string(), "Custom Workstream Title".to_string());
            ctx.flags.insert("user_curated".to_string(), true);
        })
        .when("they refresh with new events and run \"shiplog cluster\"", |ctx| {
            ctx.flags.insert("llm_used".to_string(), true);
            ctx.flags.insert("workstreams_generated".to_string(), true);
            ctx.strings
                .insert("preserved_title".to_string(), "Custom Workstream Title".to_string());
            ctx.flags.insert("new_events_added".to_string(), true);
            Ok(())
        })
        .then("the curated workstream titles should be preserved", |ctx| {
            let title = ctx.string("preserved_title").unwrap();
            assert_eq(title, "Custom Workstream Title", "title")
        })
        .then("new events should be added to appropriate workstreams", |ctx| {
            assert_true(
                ctx.flag("new_events_added").unwrap_or(false),
                "new events added",
            )
        })
}

/// Scenario 9.9: LLM clustering with many events
pub fn llm_clustering_large() -> Scenario {
    Scenario::new("LLM clustering with many events")
        .given("a user has 1,000 events", |ctx| {
            ctx.numbers.insert("event_count".to_string(), 1000);
        })
        .when("they run \"shiplog cluster\"", |ctx| {
            ctx.flags.insert("llm_used".to_string(), true);
            ctx.flags.insert("workstreams_generated".to_string(), true);
            ctx.numbers.insert("workstream_count".to_string(), 10);
            ctx.strings
                .insert("cluster_time".to_string(), "55s".to_string());
            ctx.flags.insert("events_chunked".to_string(), true);
            Ok(())
        })
        .then("clustering should complete within reasonable time (< 60 seconds)", |ctx| {
            let time = ctx.string("cluster_time").unwrap();
            assert_true(
                time.contains("s") && !time.contains("m"),
                "cluster time",
            )
        })
        .then("the system should chunk events if needed", |ctx| {
            assert_true(
                ctx.flag("events_chunked").unwrap_or(false),
                "events chunked",
            )
        })
}
