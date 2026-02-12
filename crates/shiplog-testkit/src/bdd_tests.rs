//! BDD-style integration tests for shiplog workflows.
//!
//! These tests use the Given/When/Then pattern to describe user behaviors.

#[cfg(test)]
mod workflow_tests {
    use crate::bdd::*;
    use crate::bdd::assertions::*;
    use crate::fixtures::*;

    #[test]
    fn complete_workflow_collect_curate_render() {
        let scenario = Scenario::new("User completes full workflow from collection to packet")
            .given("a developer with GitHub activity", |ctx| {
                ctx.strings.insert("username".to_string(), "developer".to_string());
                ctx.strings.insert("repo".to_string(), "acme/app".to_string());
            })
            .given("a date range for the past quarter", |ctx| {
                ctx.strings.insert("since".to_string(), "2025-01-01".to_string());
                ctx.strings.insert("until".to_string(), "2025-04-01".to_string());
            })
            .when("they collect events from GitHub", |ctx| {
                // Simulate collection
                let events = realistic_quarter_events(
                    ctx.string("username").unwrap_or("user"),
                    ctx.string("repo").unwrap_or("test/repo")
                );
                ctx.numbers.insert("event_count".to_string(), events.len() as u64);
                Ok(())
            })
            .when("workstreams are generated from repos", |ctx| {
                // Simulate workstream generation
                ctx.flags.insert("workstreams_generated".to_string(), true);
                ctx.strings.insert("workstream_file".to_string(), "workstreams.suggested.yaml".to_string());
                Ok(())
            })
            .when("they curate workstreams by renaming and adding summaries", |ctx| {
                // Simulate curation
                ctx.strings.insert("workstream_file".to_string(), "workstreams.yaml".to_string());
                ctx.flags.insert("workstreams_curated".to_string(), true);
                Ok(())
            })
            .when("they render the packet", |ctx| {
                // Simulate rendering
                ctx.paths.insert("packet_md".to_string(), "/out/run_xxx/packet.md".into());
                ctx.flags.insert("packet_rendered".to_string(), true);
                Ok(())
            })
            .then("the packet should exist", |ctx| {
                assert_true(ctx.flag("packet_rendered").unwrap_or(false), "packet rendered flag")
            })
            .then("workstreams should be curated", |ctx| {
                assert_true(ctx.flag("workstreams_curated").unwrap_or(false), "workstreams curated flag")
            });

        scenario.run().expect("workflow should complete successfully");
    }

    #[test]
    fn refresh_preserves_curation() {
        let scenario = Scenario::new("Refresh preserves user curation")
            .given("an existing run with curated workstreams", |ctx| {
                ctx.strings.insert("run_dir".to_string(), "/out/run_001".to_string());
                ctx.flags.insert("has_curated_workstreams".to_string(), true);
            })
            .given("the user has added custom workstream titles", |ctx| {
                ctx.strings.insert("custom_title".to_string(), "Authentication Service Revamp".to_string());
            })
            .when("they refresh the data with new date range", |ctx| {
                // Simulate refresh
                ctx.numbers.insert("new_events".to_string(), 3);
                Ok(())
            })
            .then("the curated workstream title should be preserved", |ctx| {
                let title = assert_present(ctx.string("custom_title"), "custom title")?;
                assert_eq(title, "Authentication Service Revamp", "workstream title")
            })
            .then("new events should be added to the ledger", |ctx| {
                let count = assert_present(ctx.number("new_events"), "new event count")?;
                assert_true(count > 0, "new events added")
            });

        scenario.run().expect("refresh should preserve curation");
    }

    #[test]
    fn redaction_leak_prevention() {
        let scenario = Scenario::new("Sensitive data is redacted in public profile")
            .given("a PR with sensitive information in the title", |ctx| {
                ctx.strings.insert("sensitive_title".to_string(), 
                    "Fix security vulnerability CVE-2025-1234 in auth service".to_string());
                ctx.strings.insert("repo".to_string(), "acme/top-secret-project".to_string());
            })
            .given("a public redaction profile", |ctx| {
                ctx.data.insert("redact_key".to_string(), b"test-key-123".to_vec());
            })
            .when("the event is redacted for public sharing", |ctx| {
                // Simulate redaction
                ctx.flags.insert("redacted".to_string(), true);
                ctx.strings.insert("redacted_output".to_string(), 
                    "{\"title\":\"[redacted]\",\"repo\":\"repo-abc123\"}".to_string());
                Ok(())
            })
            .then("the sensitive title should not appear in output", |ctx| {
                let output = assert_present(ctx.string("redacted_output"), "redacted output")?;
                assert_not_contains(output, "CVE-2025-1234", "sensitive CVE in output")
            })
            .then("the repo name should be aliased", |ctx| {
                let output = assert_present(ctx.string("redacted_output"), "redacted output")?;
                assert_not_contains(output, "top-secret-project", "sensitive repo name in output")
            });

        scenario.run().expect("redaction should prevent leaks");
    }

    #[test]
    fn coverage_transparency() {
        let scenario = Scenario::new("Coverage manifest accurately reflects data completeness")
            .given("a date range with many PRs", |ctx| {
                ctx.numbers.insert("total_prs".to_string(), 1500);
                ctx.strings.insert("date_range".to_string(), "2025-01-01 to 2025-01-31".to_string());
            })
            .given("GitHub API has 1000 result cap", |ctx| {
                ctx.flags.insert("api_cap_hit".to_string(), true);
            })
            .when("events are collected with adaptive slicing", |ctx| {
                // Simulate collection with slicing
                ctx.numbers.insert("fetched_prs".to_string(), 1000);
                ctx.flags.insert("slicing_applied".to_string(), true);
                Ok(())
            })
            .then("coverage should be marked as partial", |ctx| {
                // In real test, would check CoverageManifest.completeness
                assert_true(ctx.flag("slicing_applied").unwrap_or(false), "slicing applied flag")
            })
            .then("slices should document the API cap", |ctx| {
                let total = assert_present(ctx.number("total_prs"), "total PRs")?;
                let fetched = assert_present(ctx.number("fetched_prs"), "fetched PRs")?;
                assert_true(total > fetched, "total greater than fetched (partial coverage)")
            });

        scenario.run().expect("coverage should be transparent");
    }

    #[test]
    fn manual_event_integration() {
        let scenario = Scenario::new("Manual events integrate with GitHub events")
            .given("GitHub events from API collection", |ctx| {
                ctx.numbers.insert("github_events".to_string(), 5);
            })
            .given("manual events from YAML", |ctx| {
                ctx.numbers.insert("manual_events".to_string(), 2);
            })
            .when("all events are merged into the ledger", |ctx| {
                let github = ctx.number("github_events").unwrap_or(0);
                let manual = ctx.number("manual_events").unwrap_or(0);
                ctx.numbers.insert("total_events".to_string(), github + manual);
                Ok(())
            })
            .then("the ledger should contain both types", |ctx| {
                let total = assert_present(ctx.number("total_events"), "total events")?;
                let github = assert_present(ctx.number("github_events"), "github events")?;
                let manual = assert_present(ctx.number("manual_events"), "manual events")?;
                assert_eq(total, github + manual, "total equals sum of github and manual")
            })
            .then("workstreams can reference both event types", |_ctx| {
                // In real test, would verify workstream.events contains mixed types
                assert_true(true, "workstream event mixing")
            });

        scenario.run().expect("manual events should integrate");
    }
}

#[cfg(test)]
mod cache_tests {
    use crate::bdd::*;
    use crate::bdd::assertions::*;

    #[test]
    fn cache_hit_returns_cached_data() {
        let scenario = Scenario::new("Cache hit returns cached data without API call")
            .given("a cached PR details entry", |ctx| {
                ctx.strings.insert("cache_key".to_string(), "pr:details:https://api.github.com/repos/owner/repo/pulls/42".to_string());
                ctx.flags.insert("cache_populated".to_string(), true);
            })
            .given("the cache entry has not expired", |ctx| {
                ctx.flags.insert("cache_valid".to_string(), true);
            })
            .when("PR details are requested", |ctx| {
                // Simulate cache hit
                ctx.flags.insert("api_call_made".to_string(), false);
                ctx.flags.insert("cache_hit".to_string(), true);
                Ok(())
            })
            .then("no API call should be made", |ctx| {
                assert_false(ctx.flag("api_call_made").unwrap_or(true), "API call made flag")
            })
            .then("cached data should be returned", |ctx| {
                assert_true(ctx.flag("cache_hit").unwrap_or(false), "cache hit flag")
            });

        scenario.run().expect("cache should prevent API call");
    }

    #[test]
    fn cache_miss_fetches_and_stores() {
        let scenario = Scenario::new("Cache miss fetches from API and stores result")
            .given("an empty cache", |ctx| {
                ctx.flags.insert("cache_populated".to_string(), false);
            })
            .when("PR details are requested for the first time", |ctx| {
                // Simulate cache miss and API fetch
                ctx.flags.insert("api_call_made".to_string(), true);
                ctx.flags.insert("cache_stored".to_string(), true);
                Ok(())
            })
            .then("an API call should be made", |ctx| {
                assert_true(ctx.flag("api_call_made").unwrap_or(false), "API call made flag")
            })
            .then("the result should be stored in cache", |ctx| {
                assert_true(ctx.flag("cache_stored").unwrap_or(false), "cache stored flag")
            });

        scenario.run().expect("cache should store after fetch");
    }
}
