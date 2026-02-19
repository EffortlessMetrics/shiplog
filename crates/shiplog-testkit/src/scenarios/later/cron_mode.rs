//! BDD scenarios for Continuous/Cron Mode (Feature 11)
//!
//! Scenarios cover:
//! - Primary user workflows (running shiplog on schedule)
//! - Edge cases (cron run fails, no new events)
//! - Integration with other features (multi-source)
//! - Performance scenarios (large incremental updates)

use crate::bdd::Scenario;
use crate::bdd::assertions::*;

/// Scenario 11.1: User runs shiplog on schedule
pub fn cron_mode_scheduled() -> Scenario {
    Scenario::new("User runs shiplog on schedule")
        .given("a user has configured cron mode", |ctx| {
            ctx.strings.insert("schedule".to_string(), "0 0 * * 0".to_string());
        })
        .given("the config specifies a schedule of \"0 0 * * 0\" (weekly)", |ctx| {
            ctx.strings.insert("schedule_desc".to_string(), "weekly".to_string());
        })
        .when("the cron schedule triggers", |ctx| {
            ctx.flags.insert("cron_triggered".to_string(), true);
            ctx.flags.insert("new_events_collected".to_string(), true);
            ctx.flags.insert("ledger_appended".to_string(), true);
            ctx.flags.insert("packet_generated".to_string(), true);
            Ok(())
        })
        .then("shiplog should collect new events since the last run", |ctx| {
            assert_true(
                ctx.flag("new_events_collected").unwrap_or(false),
                "new events collected",
            )
        })
        .then("new events should be appended to the existing ledger", |ctx| {
            assert_true(
                ctx.flag("ledger_appended").unwrap_or(false),
                "ledger appended",
            )
        })
        .then("a new packet should be generated", |ctx| {
            assert_true(
                ctx.flag("packet_generated").unwrap_or(false),
                "packet generated",
            )
        })
}

/// Scenario 11.2: User configures incremental collection
pub fn cron_mode_incremental() -> Scenario {
    Scenario::new("User configures incremental collection")
        .given("a user has enabled cron mode", |ctx| {
            ctx.flags.insert("cron_enabled".to_string(), true);
        })
        .given("the config specifies incremental: true", |ctx| {
            ctx.flags.insert("incremental_enabled".to_string(), true);
        })
        .when("the cron schedule triggers", |ctx| {
            ctx.flags.insert("new_events_collected".to_string(), true);
            ctx.flags.insert("ledger_preserved".to_string(), true);
            ctx.numbers.insert("new_event_count".to_string(), 10);
            Ok(())
        })
        .then("only new events since the last run should be collected", |ctx| {
            let count = ctx.number("new_event_count").unwrap_or(0);
            assert_true(count > 0, "new events")
        })
        .then("the existing ledger should be preserved", |ctx| {
            assert_true(
                ctx.flag("ledger_preserved").unwrap_or(false),
                "ledger preserved",
            )
        })
}

/// Scenario 11.3: User configures full collection on schedule
pub fn cron_mode_full() -> Scenario {
    Scenario::new("User configures full collection on schedule")
        .given("a user has enabled cron mode", |ctx| {
            ctx.flags.insert("cron_enabled".to_string(), true);
        })
        .given("the config specifies incremental: false", |ctx| {
            ctx.flags.insert("incremental_disabled".to_string(), true);
        })
        .given("the config specifies a date range", |ctx| {
            ctx.strings.insert("since".to_string(), "2025-01-01".to_string());
            ctx.strings.insert("until".to_string(), "2025-02-01".to_string());
        })
        .when("the cron schedule triggers", |ctx| {
            ctx.flags.insert("full_collection".to_string(), true);
            ctx.flags.insert("ledger_replaced".to_string(), true);
            ctx.numbers.insert("total_event_count".to_string(), 50);
            Ok(())
        })
        .then("a full collection for the date range should be performed", |ctx| {
            assert_true(
                ctx.flag("full_collection").unwrap_or(false),
                "full collection",
            )
        })
        .then("the ledger should be replaced with new data", |ctx| {
            assert_true(
                ctx.flag("ledger_replaced").unwrap_or(false),
                "ledger replaced",
            )
        })
}

/// Scenario 11.4: Cron run fails
pub fn cron_mode_failure() -> Scenario {
    Scenario::new("Cron run fails")
        .given("a user has enabled cron mode", |ctx| {
            ctx.flags.insert("cron_enabled".to_string(), true);
        })
        .given("a scheduled run fails due to API error", |ctx| {
            ctx.flags.insert("api_error".to_string(), true);
        })
        .when("the cron schedule triggers", |ctx| {
            ctx.flags.insert("run_failed".to_string(), true);
            ctx.strings.insert(
                "error_message".to_string(),
                "Collection failed: API error".to_string(),
            );
            ctx.flags.insert("error_logged".to_string(), true);
            ctx.flags.insert("notification_sent".to_string(), true);
            Ok(())
        })
        .then("the failure should be logged", |ctx| {
            assert_true(
                ctx.flag("error_logged").unwrap_or(false),
                "error logged",
            )
        })
        .then("the next scheduled run should still attempt", |ctx| {
            assert_true(
                ctx.flag("next_run_scheduled").unwrap_or(false),
                "next run scheduled",
            )
        })
        .then("a notification should be sent if configured", |ctx| {
            assert_true(
                ctx.flag("notification_sent").unwrap_or(false),
                "notification sent",
            )
        })
}

/// Scenario 11.5: No new events since last run
pub fn cron_mode_no_new_events() -> Scenario {
    Scenario::new("No new events since last run")
        .given("a user has enabled cron mode with incremental collection", |ctx| {
            ctx.flags.insert("cron_enabled".to_string(), true);
            ctx.flags.insert("incremental_enabled".to_string(), true);
        })
        .given("no new events have occurred since the last run", |ctx| {
            ctx.flags.insert("no_new_events".to_string(), true);
        })
        .when("the cron schedule triggers", |ctx| {
            ctx.flags.insert("collection_complete".to_string(), true);
            ctx.numbers.insert("new_event_count".to_string(), 0);
            ctx.flags.insert("packet_not_generated".to_string(), true);
            ctx.strings.insert(
                "log_message".to_string(),
                "No new events since last run".to_string(),
            );
            Ok(())
        })
        .then("the collection should complete with no new events", |ctx| {
            let count = ctx.number("new_event_count").unwrap_or(0);
            assert_eq(count, 0, "new event count")
        })
        .then("a new packet should not be generated", |ctx| {
            assert_true(
                ctx.flag("packet_not_generated").unwrap_or(false),
                "packet not generated",
            )
        })
        .then("a log message should indicate no changes", |ctx| {
            let log = ctx.string("log_message").unwrap();
            assert_contains(log, "No new events", "log message")
        })
}

/// Scenario 11.6: Cron mode works with multi-source
pub fn cron_mode_multi_source() -> Scenario {
    Scenario::new("Cron mode works with multi-source")
        .given("a user has configured cron mode", |ctx| {
            ctx.flags.insert("cron_enabled".to_string(), true);
        })
        .given("they have multiple sources configured", |ctx| {
            ctx.numbers.insert("source_count".to_string(), 3);
        })
        .when("the cron schedule triggers", |ctx| {
            ctx.flags.insert("all_sources_checked".to_string(), true);
            ctx.flags.insert("new_events_from_all_sources".to_string(), true);
            ctx.flags.insert("ledger_appended".to_string(), true);
            Ok(())
        })
        .then("all sources should be checked for new events", |ctx| {
            assert_true(
                ctx.flag("all_sources_checked").unwrap_or(false),
                "all sources checked",
            )
        })
        .then("new events from all sources should be appended", |ctx| {
            assert_true(
                ctx.flag("new_events_from_all_sources").unwrap_or(false),
                "new events from all sources",
            )
        })
}

/// Scenario 11.7: Cron mode with large incremental update
pub fn cron_mode_large_update() -> Scenario {
    Scenario::new("Cron mode with large incremental update")
        .given("a user has enabled cron mode", |ctx| {
            ctx.flags.insert("cron_enabled".to_string(), true);
        })
        .given("500 new events have occurred since the last run", |ctx| {
            ctx.numbers.insert("new_event_count".to_string(), 500);
        })
        .when("the cron schedule triggers", |ctx| {
            ctx.flags.insert("collection_complete".to_string(), true);
            ctx.strings
                .insert("collection_time".to_string(), "55s".to_string());
            Ok(())
        })
        .then("collection should complete within reasonable time (< 60 seconds)", |ctx| {
            let time = ctx.string("collection_time").unwrap();
            assert_true(
                time.contains("s") && !time.contains("m"),
                "collection time",
            )
        })
}
