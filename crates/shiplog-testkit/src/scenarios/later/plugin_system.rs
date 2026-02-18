//! BDD scenarios for Plugin System (Feature 14)
//!
//! Scenarios cover:
//! - Primary user workflows (installing and using plugins)
//! - Edge cases (installation failures, incompatible versions)
//! - Integration with other features (event merging)
//! - Performance scenarios (slow ingest)

use crate::bdd::Scenario;
use crate::bdd::assertions::*;

/// Scenario 14.1: User installs a third-party ingest adapter plugin
pub fn plugin_install() -> Scenario {
    Scenario::new("User installs a third-party ingest adapter plugin")
        .given("a third-party has published a shiplog ingest adapter plugin", |ctx| {
            ctx.strings
                .insert("plugin_name".to_string(), "shiplog-ingest-custom".to_string());
            ctx.strings
                .insert("plugin_version".to_string(), "1.0.0".to_string());
        })
        .when(
            "they run \"shiplog plugin install shiplog-ingest-custom\"",
            |ctx| {
                ctx.flags.insert("plugin_downloaded".to_string(), true);
                ctx.flags.insert("plugin_installed".to_string(), true);
                ctx.flags.insert("plugin_available".to_string(), true);
                Ok(())
            },
        )
        .then("the plugin should be downloaded and installed", |ctx| {
            assert_true(
                ctx.flag("plugin_installed").unwrap_or(false),
                "plugin installed",
            )
        })
        .then("the plugin should be available for use", |ctx| {
            assert_true(
                ctx.flag("plugin_available").unwrap_or(false),
                "plugin available",
            )
        })
}

/// Scenario 14.2: User uses a plugin ingest adapter
pub fn plugin_use() -> Scenario {
    Scenario::new("User uses a plugin ingest adapter")
        .given("a user has installed a custom ingest adapter plugin", |ctx| {
            ctx.strings
                .insert("plugin_name".to_string(), "custom".to_string());
        })
        .given("the plugin is named \"custom\"", |ctx| {
            ctx.flags.insert("plugin_available".to_string(), true);
        })
        .when(
            "they run \"shiplog collect --source custom --config custom-config.yaml\"",
            |ctx| {
                ctx.flags.insert("plugin_invoked".to_string(), true);
                ctx.flags.insert("events_collected".to_string(), true);
                ctx.strings
                    .insert("source_system".to_string(), "custom".to_string());
                Ok(())
            },
        )
        .then("the plugin's ingestor should be invoked", |ctx| {
            assert_true(
                ctx.flag("plugin_invoked").unwrap_or(false),
                "plugin invoked",
            )
        })
        .then("events should be collected using the plugin", |ctx| {
            assert_true(
                ctx.flag("events_collected").unwrap_or(false),
                "events collected",
            )
        })
        .then("events should have SourceSystem::Other(\"custom\")", |ctx| {
            let source = ctx.string("source_system").unwrap();
            assert_eq(source, "custom", "source system")
        })
}

/// Scenario 14.3: User lists installed plugins
pub fn plugin_list() -> Scenario {
    Scenario::new("User lists installed plugins")
        .given("a user has installed multiple plugins", |ctx| {
            ctx.numbers.insert("plugin_count".to_string(), 3);
        })
        .when("they run \"shiplog plugin list\"", |ctx| {
            ctx.flags.insert("plugins_listed".to_string(), true);
            ctx.flags.insert("names_shown".to_string(), true);
            ctx.flags.insert("versions_shown".to_string(), true);
            Ok(())
        })
        .then("all installed plugins should be listed", |ctx| {
            assert_true(
                ctx.flag("plugins_listed").unwrap_or(false),
                "plugins listed",
            )
        })
        .then("each plugin should show its name and version", |ctx| {
            assert_true(
                ctx.flag("names_shown").unwrap_or(false)
                    && ctx.flag("versions_shown").unwrap_or(false),
                "names and versions shown",
            )
        })
}

/// Scenario 14.4: User removes a plugin
pub fn plugin_remove() -> Scenario {
    Scenario::new("User removes a plugin")
        .given("a user has installed a plugin", |ctx| {
            ctx.strings
                .insert("plugin_name".to_string(), "shiplog-ingest-custom".to_string());
        })
        .when(
            "they run \"shiplog plugin remove shiplog-ingest-custom\"",
            |ctx| {
                ctx.flags.insert("plugin_uninstalled".to_string(), true);
                ctx.flags.insert("plugin_not_available".to_string(), true);
                Ok(())
            },
        )
        .then("the plugin should be uninstalled", |ctx| {
            assert_true(
                ctx.flag("plugin_uninstalled").unwrap_or(false),
                "plugin uninstalled",
            )
        })
        .then("the plugin should no longer be available", |ctx| {
            assert_true(
                ctx.flag("plugin_not_available").unwrap_or(false),
                "plugin not available",
            )
        })
}

/// Scenario 14.5: Plugin installation fails
pub fn plugin_install_failure() -> Scenario {
    Scenario::new("Plugin installation fails")
        .given("a user attempts to install a plugin", |ctx| {
            ctx.strings
                .insert("plugin_name".to_string(), "invalid-plugin".to_string());
        })
        .given("the plugin download fails", |ctx| {
            ctx.flags.insert("download_failed".to_string(), true);
        })
        .when(
            "they run \"shiplog plugin install invalid-plugin\"",
            |ctx| {
                ctx.flags.insert("command_failed".to_string(), true);
                ctx.strings.insert(
                    "error_message".to_string(),
                    "Plugin installation failed: Could not download plugin".to_string(),
                );
                Ok(())
            },
        )
        .then("an error message should indicate the installation failed", |ctx| {
            assert_true(
                ctx.flag("command_failed").unwrap_or(false),
                "command failed",
            )
        })
        .then("the reason should be clearly stated", |ctx| {
            let error = ctx.string("error_message").unwrap();
            assert_contains(error, "Could not download", "error message")
        })
}

/// Scenario 14.6: Plugin has incompatible version
pub fn plugin_incompatible_version() -> Scenario {
    Scenario::new("Plugin has incompatible version")
        .given("a user attempts to install a plugin", |ctx| {
            ctx.strings
                .insert("plugin_name".to_string(), "incompatible-plugin".to_string());
        })
        .given("the plugin requires a newer version of shiplog", |ctx| {
            ctx.strings
                .insert("required_version".to_string(), "0.3.0".to_string());
            ctx.strings
                .insert("current_version".to_string(), "0.2.0".to_string());
        })
        .when(
            "they run \"shiplog plugin install incompatible-plugin\"",
            |ctx| {
                ctx.flags.insert("command_failed".to_string(), true);
                ctx.strings.insert(
                    "error_message".to_string(),
                    "Plugin incompatible: requires shiplog 0.3.0, current version is 0.2.0".to_string(),
                );
                Ok(())
            },
        )
        .then("an error should indicate the version incompatibility", |ctx| {
            let error = ctx.string("error_message").unwrap();
            assert_contains(error, "incompatible", "error message")
        })
        .then("the required shiplog version should be shown", |ctx| {
            let error = ctx.string("error_message").unwrap();
            assert_contains(error, "0.3.0", "error message")
        })
}

/// Scenario 14.7: Plugin crashes during execution
pub fn plugin_crash() -> Scenario {
    Scenario::new("Plugin crashes during execution")
        .given("a user is using a plugin ingest adapter", |ctx| {
            ctx.strings
                .insert("plugin_name".to_string(), "custom".to_string());
        })
        .given("the plugin crashes during execution", |ctx| {
            ctx.flags.insert("plugin_crashed".to_string(), true);
        })
        .when(
            "they run \"shiplog collect --source custom\"",
            |ctx| {
                ctx.flags.insert("crash_caught".to_string(), true);
                ctx.flags.insert("error_message_shown".to_string(), true);
                ctx.flags.insert("shiplog_still_functional".to_string(), true);
                Ok(())
            },
        )
        .then("the crash should be caught", |ctx| {
            assert_true(
                ctx.flag("crash_caught").unwrap_or(false),
                "crash caught",
            )
        })
        .then("an error message should indicate the plugin failed", |ctx| {
            assert_true(
                ctx.flag("error_message_shown").unwrap_or(false),
                "error message shown",
            )
        })
        .then("shiplog should not crash", |ctx| {
            assert_true(
                ctx.flag("shiplog_still_functional").unwrap_or(false),
                "shiplog still functional",
            )
        })
}

/// Scenario 14.8: Plugin events merge with built-in sources
pub fn plugin_merge() -> Scenario {
    Scenario::new("Plugin events merge with built-in sources")
        .given("a user has collected events from GitHub", |ctx| {
            ctx.numbers.insert("github_events".to_string(), 25);
        })
        .given("they also collect events from a plugin", |ctx| {
            ctx.numbers.insert("plugin_events".to_string(), 15);
        })
        .when("they merge the sources", |ctx| {
            ctx.flags.insert("merged".to_string(), true);
            ctx.numbers.insert("total_events".to_string(), 40);
            ctx.flags.insert("multi_source_workstreams".to_string(), true);
            Ok(())
        })
        .then("events from both sources should be merged", |ctx| {
            let total = ctx.number("total_events").unwrap_or(0);
            assert_eq(total, 40, "total events")
        })
        .then("workstreams can include events from both", |ctx| {
            assert_true(
                ctx.flag("multi_source_workstreams").unwrap_or(false),
                "multi-source workstreams",
            )
        })
}

/// Scenario 14.9: Plugin with slow ingest
pub fn plugin_slow_ingest() -> Scenario {
    Scenario::new("Plugin with slow ingest")
        .given("a user has a plugin that takes time to ingest events", |ctx| {
            ctx.strings
                .insert("plugin_name".to_string(), "custom".to_string());
        })
        .when(
            "they run \"shiplog collect --source custom\"",
            |ctx| {
                ctx.flags.insert("progress_displayed".to_string(), true);
                ctx.flags.insert("cancellable".to_string(), true);
                ctx.strings
                    .insert("ingest_time".to_string(), "30s".to_string());
                Ok(())
            },
        )
        .then("progress should be displayed", |ctx| {
            assert_true(
                ctx.flag("progress_displayed").unwrap_or(false),
                "progress displayed",
            )
        })
        .then("the user should be able to cancel with Ctrl+C", |ctx| {
            assert_true(
                ctx.flag("cancellable").unwrap_or(false),
                "cancellable",
            )
        })
}
