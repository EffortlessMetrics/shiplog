//! BDD workflow test: cache hit avoids redundant API calls.
//!
//! Exercises the cache's get/set lifecycle to verify that cached data is
//! returned without needing a second "fetch".

use chrono::Duration;
use shiplog_cache_sqlite::ApiCache;
use shiplog_testkit::bdd::Scenario;
use shiplog_testkit::bdd::assertions::*;

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Clone)]
struct PrDetails {
    number: u64,
    title: String,
    additions: u64,
}

#[test]
fn scenario_cache_hit_avoids_api_call() {
    Scenario::new("Cached API response is returned without a second fetch")
        .given("a cached PR details response", |ctx| {
            ctx.strings.insert(
                "cache_key".into(),
                "pr:details:https://api.github.com/repos/acme/app/pulls/42".into(),
            );
            ctx.strings
                .insert("pr_title".into(), "Add auth flow".into());
            ctx.numbers.insert("pr_number".into(), 42);
        })
        .when("the data is stored and then retrieved from cache", |ctx| {
            let key = ctx.string("cache_key").unwrap().to_string();
            let pr = PrDetails {
                number: ctx.number("pr_number").unwrap(),
                title: ctx.string("pr_title").unwrap().to_string(),
                additions: 150,
            };

            let cache = ApiCache::open_in_memory().map_err(|e| e.to_string())?;

            // First access: cache miss
            let miss: Option<PrDetails> = cache.get(&key).map_err(|e| e.to_string())?;
            ctx.flags.insert("first_access_miss".into(), miss.is_none());

            // Simulate: API call made, result stored
            cache
                .set_with_ttl(&key, &pr, Duration::hours(24))
                .map_err(|e| e.to_string())?;
            ctx.flags.insert("api_call_made".into(), true);

            // Second access: cache hit — no API call needed
            let hit: Option<PrDetails> = cache.get(&key).map_err(|e| e.to_string())?;
            ctx.flags.insert("second_access_hit".into(), hit.is_some());

            if let Some(ref cached_pr) = hit {
                ctx.flags.insert("data_matches".into(), cached_pr == &pr);
                ctx.strings
                    .insert("cached_title".into(), cached_pr.title.clone());
                ctx.numbers.insert("cached_number".into(), cached_pr.number);
            }

            // Stats should show one valid entry
            let stats = cache.stats().map_err(|e| e.to_string())?;
            ctx.numbers
                .insert("total_entries".into(), stats.total_entries as u64);
            ctx.numbers
                .insert("valid_entries".into(), stats.valid_entries as u64);

            Ok(())
        })
        .then("the first access is a cache miss", |ctx| {
            assert_true(
                ctx.flag("first_access_miss").unwrap_or(false),
                "first access should miss",
            )
        })
        .then("the second access is a cache hit", |ctx| {
            assert_true(
                ctx.flag("second_access_hit").unwrap_or(false),
                "second access should hit",
            )
        })
        .then("the cached data matches the original", |ctx| {
            assert_true(
                ctx.flag("data_matches").unwrap_or(false),
                "cached data matches",
            )?;
            let title = assert_present(ctx.string("cached_title"), "cached title")?;
            assert_eq(title, "Add auth flow", "cached PR title")?;
            let number = assert_present(ctx.number("cached_number"), "cached number")?;
            assert_eq(number, 42, "cached PR number")
        })
        .then("cache stats reflect one valid entry", |ctx| {
            let total = assert_present(ctx.number("total_entries"), "total entries")?;
            assert_eq(total, 1, "total cache entries")?;
            let valid = assert_present(ctx.number("valid_entries"), "valid entries")?;
            assert_eq(valid, 1, "valid cache entries")
        })
        .run()
        .expect("cache hit workflow should pass");
}
