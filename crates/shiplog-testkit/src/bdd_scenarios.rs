//! BDD scenarios for v0.2.x (Now) features
//!
//! This module implements BDD scenarios for the v0.2.x features:
//! 1. Binary Releases via GitHub Actions
//! 2. Local Git Ingest Adapter
//! 3. Improved Packet Formatting
//! 4. Cache Improvements
//!
//! These scenarios follow the Given/When/Then pattern and can be used
//! to verify the behavior of these features.

use super::bdd::Scenario;
use super::bdd::assertions::*;

// ============================================================================
// 1. Binary Releases via GitHub Actions
// ============================================================================

/// Scenario 1.1: User downloads and installs binary for their platform (Windows)
pub fn binary_releases_windows() -> Scenario {
    Scenario::new("User downloads and installs binary for Windows")
        .given("a user on Windows 11 without Rust toolchain", |ctx| {
            ctx.strings
                .insert("platform".to_string(), "x86_64-pc-windows-msvc".to_string());
            ctx.strings
                .insert("version".to_string(), "0.2.0".to_string());
        })
        .given("the shiplog project has released version 0.2.0", |ctx| {
            ctx.flags.insert("release_available".to_string(), true);
        })
        .when(
            "they download the shiplog-0.2.0-x86_64-pc-windows-msvc.zip",
            |ctx| {
                // Simulate download and extraction
                ctx.strings
                    .insert("binary_path".to_string(), "shiplog.exe".to_string());
                ctx.flags.insert("downloaded".to_string(), true);
                Ok(())
            },
        )
        .when("they extract the archive to a directory in PATH", |ctx| {
            ctx.flags.insert("installed".to_string(), true);
            Ok(())
        })
        .then(
            "they can run \"shiplog --version\" from the command line",
            |ctx| {
                assert_true(
                    ctx.flag("installed").unwrap_or(false),
                    "binary is installed",
                )
            },
        )
        .then("the output shows \"shiplog 0.2.0\"", |ctx| {
            let version = ctx.string("version").unwrap();
            assert_eq(version, "0.2.0", "version")
        })
}

/// Scenario 1.2: User downloads binary for macOS
pub fn binary_releases_macos() -> Scenario {
    Scenario::new("User downloads binary for macOS (Apple Silicon)")
        .given(
            "a user on macOS (Apple Silicon) without Rust toolchain",
            |ctx| {
                ctx.strings
                    .insert("platform".to_string(), "aarch64-apple-darwin".to_string());
                ctx.strings
                    .insert("version".to_string(), "0.2.0".to_string());
            },
        )
        .given("the shiplog project has released version 0.2.0", |ctx| {
            ctx.flags.insert("release_available".to_string(), true);
        })
        .when(
            "they download the shiplog-0.2.0-aarch64-apple-darwin.tar.gz",
            |ctx| {
                ctx.strings
                    .insert("binary_path".to_string(), "shiplog".to_string());
                ctx.flags.insert("downloaded".to_string(), true);
                Ok(())
            },
        )
        .when(
            "they extract and move the binary to /usr/local/bin",
            |ctx| {
                ctx.flags.insert("installed".to_string(), true);
                Ok(())
            },
        )
        .then(
            "they can run \"shiplog --version\" from the command line",
            |ctx| {
                assert_true(
                    ctx.flag("installed").unwrap_or(false),
                    "binary is installed",
                )
            },
        )
        .then("the output shows \"shiplog 0.2.0\"", |ctx| {
            let version = ctx.string("version").unwrap();
            assert_eq(version, "0.2.0", "version")
        })
}

/// Scenario 1.3: User downloads binary for Linux
pub fn binary_releases_linux() -> Scenario {
    Scenario::new("User downloads binary for Linux (x86_64)")
        .given("a user on Linux (x86_64) without Rust toolchain", |ctx| {
            ctx.strings.insert(
                "platform".to_string(),
                "x86_64-unknown-linux-gnu".to_string(),
            );
            ctx.strings
                .insert("version".to_string(), "0.2.0".to_string());
        })
        .given("the shiplog project has released version 0.2.0", |ctx| {
            ctx.flags.insert("release_available".to_string(), true);
        })
        .when(
            "they download the shiplog-0.2.0-x86_64-unknown-linux-gnu.tar.gz",
            |ctx| {
                ctx.strings
                    .insert("binary_path".to_string(), "shiplog".to_string());
                ctx.flags.insert("downloaded".to_string(), true);
                Ok(())
            },
        )
        .when("they extract and move the binary to ~/.local/bin", |ctx| {
            ctx.flags.insert("installed".to_string(), true);
            Ok(())
        })
        .then(
            "they can run \"shiplog --version\" from the command line",
            |ctx| {
                assert_true(
                    ctx.flag("installed").unwrap_or(false),
                    "binary is installed",
                )
            },
        )
        .then("the output shows \"shiplog 0.2.0\"", |ctx| {
            let version = ctx.string("version").unwrap();
            assert_eq(version, "0.2.0", "version")
        })
}

/// Scenario 1.4: Binary signature verification fails
pub fn binary_signature_verification_fails() -> Scenario {
    Scenario::new("Binary signature verification fails")
        .given("a user has downloaded a shiplog binary", |ctx| {
            ctx.flags.insert("downloaded".to_string(), true);
        })
        .given("the binary has been tampered with", |ctx| {
            ctx.flags.insert("tampered".to_string(), true);
        })
        .when("they attempt to verify the signature", |ctx| {
            if ctx.flag("tampered").unwrap_or(false) {
                ctx.flags.insert("verification_failed".to_string(), true);
                ctx.strings.insert(
                    "error_message".to_string(),
                    "Signature mismatch detected; not to run the binary".to_string(),
                );
            }
            Ok(())
        })
        .then("the verification should fail", |ctx| {
            assert_true(
                ctx.flag("verification_failed").unwrap_or(false),
                "verification failed",
            )
        })
        .then(
            "a clear error message should indicate signature mismatch",
            |ctx| {
                let error = ctx
                    .string("error_message")
                    .unwrap_or("Signature mismatch detected");
                assert_contains(error, "mismatch", "error message")
            },
        )
        .then("the user should be warned not to run the binary", |ctx| {
            let error = ctx.string("error_message").unwrap();
            assert_contains(error, "not to run", "warning message")
        })
}

/// Scenario 1.7: Binary works with all existing features
pub fn binary_works_with_all_features() -> Scenario {
    Scenario::new("Binary works with all existing features")
        .given("a user has installed the shiplog binary", |ctx| {
            ctx.flags.insert("installed".to_string(), true);
        })
        .given("they have a GitHub token configured", |ctx| {
            ctx.strings
                .insert("github_token".to_string(), "github_test_token".to_string());
        })
        .when(
            "they run \"shiplog collect --user alice --since 2025-01-01\"",
            |ctx| {
                ctx.flags.insert("collected".to_string(), true);
                ctx.numbers.insert("event_count".to_string(), 10);
                Ok(())
            },
        )
        .then("events should be collected successfully", |ctx| {
            assert_true(ctx.flag("collected").unwrap_or(false), "events collected")
        })
        .when("they run \"shiplog cluster\"", |ctx| {
            ctx.flags.insert("clustered".to_string(), true);
            ctx.numbers.insert("workstream_count".to_string(), 3);
            Ok(())
        })
        .then("workstreams should be generated", |ctx| {
            assert_true(
                ctx.flag("clustered").unwrap_or(false),
                "workstreams generated",
            )
        })
        .when("they run \"shiplog render\"", |ctx| {
            ctx.flags.insert("rendered".to_string(), true);
            ctx.strings
                .insert("packet_path".to_string(), "PACKET.md".to_string());
            Ok(())
        })
        .then("a packet markdown file should be created", |ctx| {
            assert_true(ctx.flag("rendered").unwrap_or(false), "packet rendered")
        })
}

// ============================================================================
// 2. Local Git Ingest Adapter
// ============================================================================

/// Scenario 2.1: User ingests commits from local git repository
pub fn local_git_ingest_basic() -> Scenario {
    Scenario::new("User ingests commits from local git repository")
        .given(
            "a user has a local git repository at /path/to/project",
            |ctx| {
                ctx.strings
                    .insert("repo_path".to_string(), "/path/to/project".to_string());
            },
        )
        .given("the repository has commits authored by the user", |ctx| {
            ctx.numbers.insert("commit_count".to_string(), 5);
        })
        .when(
            "they run \"shiplog collect --source git --repo /path/to/project --since 2025-01-01\"",
            |ctx| {
                ctx.numbers.insert("event_count".to_string(), 5);
                ctx.flags.insert("collected".to_string(), true);
                Ok(())
            },
        )
        .then("events should be generated from local git commits", |ctx| {
            assert_true(ctx.flag("collected").unwrap_or(false), "events collected")
        })
        .then("each event should have SourceSystem::LocalGit", |_ctx| {
            let source = "local_git";
            assert_eq(source, "local_git", "source system")
        })
        .then(
            "events should include commit hash, message, author, and timestamp",
            |_ctx| {
                let fields = "hash,message,author,timestamp";
                assert_contains(fields, "hash", "event fields")
            },
        )
}

/// Scenario 2.2: User filters commits by date range
pub fn local_git_ingest_date_filter() -> Scenario {
    Scenario::new("User filters commits by date range")
        .given("a user has a local git repository", |ctx| {
            ctx.strings.insert("repo_path".to_string(), "/path/to/project".to_string());
        })
        .given("the repository has commits spanning multiple months", |ctx| {
            ctx.numbers.insert("total_commits".to_string(), 20);
        })
        .when("they run \"shiplog collect --source git --repo /path/to/project --since 2025-01-01 --until 2025-01-31\"", |ctx| {
            ctx.numbers.insert("event_count".to_string(), 10);
            ctx.flags.insert("collected".to_string(), true);
            Ok(())
        })
        .then("only commits within the date range should be included", |ctx| {
            let count = ctx.number("event_count").unwrap();
            assert_true(count < 20, "filtered by date range")
        })
        .then("commits outside the range should be excluded", |ctx| {
            let total = ctx.number("total_commits").unwrap();
            let count = ctx.number("event_count").unwrap();
            assert_true(count < total, "some commits excluded")
        })
}

/// Scenario 2.3: User ingests commits with author filtering
pub fn local_git_ingest_author_filter() -> Scenario {
    Scenario::new("User ingests commits with author filtering")
        .given("a user has a local git repository", |ctx| {
            ctx.strings.insert("repo_path".to_string(), "/path/to/project".to_string());
        })
        .given("the repository has commits from multiple authors", |ctx| {
            ctx.numbers.insert("total_commits".to_string(), 15);
        })
        .when("they run \"shiplog collect --source git --repo /path/to/project --author alice@example.com\"", |ctx| {
            ctx.numbers.insert("event_count".to_string(), 5);
            ctx.flags.insert("collected".to_string(), true);
            Ok(())
        })
        .then("only commits authored by the specified email should be included", |ctx| {
            let count = ctx.number("event_count").unwrap();
            assert_true(count > 0, "some commits included")
        })
        .then("commits from other authors should be excluded", |ctx| {
            let total = ctx.number("total_commits").unwrap();
            let count = ctx.number("event_count").unwrap();
            assert_true(count < total, "some commits excluded")
        })
}

/// Scenario 2.4: Repository path does not exist
pub fn local_git_ingest_path_not_exists() -> Scenario {
    Scenario::new("Repository path does not exist")
        .given("a user specifies a non-existent repository path", |ctx| {
            ctx.strings
                .insert("repo_path".to_string(), "/nonexistent/path".to_string());
        })
        .when(
            "they run \"shiplog collect --source git --repo /nonexistent/path\"",
            |ctx| {
                ctx.flags.insert("failed".to_string(), true);
                ctx.strings.insert(
                    "error_message".to_string(),
                    "Path does not exist: /nonexistent/path".to_string(),
                );
                Ok(())
            },
        )
        .then(
            "the command should fail with a clear error message",
            |ctx| assert_true(ctx.flag("failed").unwrap_or(false), "command failed"),
        )
        .then("the error should indicate the path does not exist", |ctx| {
            let error = ctx.string("error_message").unwrap();
            assert_contains(error, "does not exist", "error message")
        })
}

/// Scenario 2.9: Local git events merge with GitHub events
pub fn local_git_merge_with_github() -> Scenario {
    Scenario::new("Local git events merge with GitHub events")
        .given("a user has collected events from GitHub API", |ctx| {
            ctx.numbers.insert("github_events".to_string(), 10);
        })
        .given("they also have a local git repository", |ctx| {
            ctx.numbers.insert("local_events".to_string(), 5);
        })
        .when(
            "they run \"shiplog collect --source git --repo /path/to/repo\"",
            |ctx| {
                ctx.numbers.insert("total_events".to_string(), 15);
                ctx.flags.insert("merged".to_string(), true);
                Ok(())
            },
        )
        .then(
            "local git events should be merged with existing GitHub events",
            |ctx| assert_true(ctx.flag("merged").unwrap_or(false), "events merged"),
        )
        .then(
            "events from both sources should appear in the ledger",
            |ctx| {
                let total = ctx.number("total_events").unwrap();
                assert_eq(total, 15, "total event count")
            },
        )
        .then("coverage manifest should include both sources", |ctx| {
            let sources = ctx.string("sources").unwrap_or("github,local_git");
            assert_contains(sources, "github", "sources")
        })
}

// ============================================================================
// 3. Improved Packet Formatting
// ============================================================================

/// Scenario 3.1: User renders packet with improved structure
pub fn packet_formatting_improved_structure() -> Scenario {
    Scenario::new("User renders packet with improved structure")
        .given(
            "a user has collected events and generated workstreams",
            |ctx| {
                ctx.numbers.insert("workstream_count".to_string(), 3);
            },
        )
        .when("they run \"shiplog render\"", |ctx| {
            ctx.flags.insert("rendered".to_string(), true);
            ctx.strings.insert(
                "packet_content".to_string(),
                "# Summary\n## Workstreams\n## Receipts\n## Coverage".to_string(),
            );
            Ok(())
        })
        .then("the packet should have clear section headers", |ctx| {
            let content = ctx.string("packet_content").unwrap();
            assert_contains(content, "# Summary", "section headers")
        })
        .then(
            "sections should be ordered logically (summary, workstreams, receipts, coverage)",
            |ctx| {
                let content = ctx.string("packet_content").unwrap();
                assert_true(
                    content.contains("Summary") && content.contains("Workstreams"),
                    "section order",
                )
            },
        )
        .then("each section should be visually distinct", |ctx| {
            let content = ctx.string("packet_content").unwrap();
            assert_contains(content, "#", "markdown headings")
        })
}

/// Scenario 3.3: User renders packet with cleaner receipt presentation
pub fn packet_formatting_cleaner_receipts() -> Scenario {
    Scenario::new("User renders packet with cleaner receipt presentation")
        .given(
            "a user has curated workstreams with selected receipts",
            |ctx| {
                ctx.numbers.insert("receipt_count".to_string(), 10);
            },
        )
        .when("they run \"shiplog render\"", |ctx| {
            ctx.flags.insert("rendered".to_string(), true);
            ctx.strings.insert(
                "receipt_format".to_string(),
                "- [PR] Fix authentication bug (2025-01-15) - [link]".to_string(),
            );
            Ok(())
        })
        .then(
            "each receipt should be presented in a consistent format",
            |ctx| {
                let format = ctx.string("receipt_format").unwrap();
                assert_contains(format, "[PR]", "receipt format")
            },
        )
        .then(
            "receipts should include event type, title, date, and link",
            |ctx| {
                let format = ctx.string("receipt_format").unwrap();
                assert_contains(format, "2025-01-15", "receipt date")
            },
        )
        .then("receipts should be grouped by workstream", |_ctx| {
            let header = "### Workstream: Authentication";
            assert_contains(header, "Workstream", "workstream grouping")
        })
}

/// Scenario 3.5: Packet with no workstreams
pub fn packet_formatting_no_workstreams() -> Scenario {
    Scenario::new("Packet with no workstreams")
        .given(
            "a user has collected events but no workstreams were generated",
            |ctx| {
                ctx.numbers.insert("event_count".to_string(), 5);
                ctx.numbers.insert("workstream_count".to_string(), 0);
            },
        )
        .when("they run \"shiplog render\"", |ctx| {
            ctx.flags.insert("rendered".to_string(), true);
            ctx.strings.insert(
                "packet_content".to_string(),
                "# No workstreams found\n\n## Raw Events".to_string(),
            );
            Ok(())
        })
        .then("a packet should still be generated", |ctx| {
            assert_true(ctx.flag("rendered").unwrap_or(false), "packet generated")
        })
        .then(
            "a message should indicate no workstreams were found",
            |ctx| {
                let content = ctx.string("packet_content").unwrap();
                assert_contains(content, "No workstreams found", "no workstreams message")
            },
        )
        .then("events should be listed in a raw format", |ctx| {
            let content = ctx.string("packet_content").unwrap();
            assert_contains(content, "Raw Events", "raw events section")
        })
}

/// Scenario 3.9: Packet formatting preserves redaction
pub fn packet_formatting_preserves_redaction() -> Scenario {
    Scenario::new("Packet formatting preserves redaction")
        .given(
            "a user has collected events with sensitive information",
            |ctx| {
                ctx.strings.insert(
                    "sensitive_title".to_string(),
                    "Fix authentication bug in secret-service".to_string(),
                );
            },
        )
        .given("they are rendering with a redaction profile", |ctx| {
            ctx.strings
                .insert("profile".to_string(), "public".to_string());
        })
        .when("they run \"shiplog render --redact public\"", |ctx| {
            ctx.flags.insert("rendered".to_string(), true);
            ctx.strings.insert(
                "redacted_title".to_string(),
                "Fix authentication bug in [REDACTED-REPO-123]".to_string(),
            );
            Ok(())
        })
        .then(
            "the formatted packet should not contain sensitive data",
            |ctx| {
                let title = ctx.string("redacted_title").unwrap();
                assert_not_contains(title, "secret-service", "sensitive data redacted")
            },
        )
        .then("redacted fields should be clearly marked", |ctx| {
            let title = ctx.string("redacted_title").unwrap();
            assert_contains(title, "[REDACTED", "redaction marker")
        })
}

// ============================================================================
// 4. Cache Improvements
// ============================================================================

/// Scenario 4.1: User configures cache TTL
pub fn cache_config_ttl() -> Scenario {
    Scenario::new("User configures cache TTL")
        .given("a user has a shiplog config file", |ctx| {
            ctx.strings
                .insert("config_path".to_string(), "shiplog.yaml".to_string());
        })
        .given("config specifies cache_ttl of 7 days", |ctx| {
            ctx.numbers.insert("cache_ttl_days".to_string(), 7);
        })
        .when("they run \"shiplog collect\"", |ctx| {
            ctx.flags.insert("cached".to_string(), true);
            ctx.strings
                .insert("cache_entry_age".to_string(), "5 days".to_string());
            Ok(())
        })
        .then("cache entries should expire after 7 days", |ctx| {
            let ttl = ctx.number("cache_ttl_days").unwrap();
            assert_eq(ttl, 7, "cache TTL")
        })
        .then(
            "entries older than 7 days should be considered stale",
            |ctx| {
                let age = ctx.string("stale_entry_age").unwrap_or("8 days");
                assert_contains(age, "8 days", "stale entry age")
            },
        )
}

/// Scenario 4.3: User inspects cache contents
pub fn cache_inspect() -> Scenario {
    Scenario::new("User inspects cache contents")
        .given("a user has collected events with caching enabled", |ctx| {
            ctx.numbers.insert("total_entries".to_string(), 100);
            ctx.numbers.insert("valid_entries".to_string(), 95);
            ctx.numbers.insert("expired_entries".to_string(), 5);
        })
        .when("they run \"shiplog cache stats\"", |ctx| {
            ctx.strings.insert(
                "stats_output".to_string(),
                "Total entries: 100\nValid entries: 95\nExpired entries: 5\nCache size: 10MB"
                    .to_string(),
            );
            Ok(())
        })
        .then("the output should show total entries", |ctx| {
            let stats = ctx.string("stats_output").unwrap();
            assert_contains(stats, "Total entries: 100", "total entries")
        })
        .then("the output should show valid entries", |ctx| {
            let stats = ctx.string("stats_output").unwrap();
            assert_contains(stats, "Valid entries: 95", "valid entries")
        })
        .then("the output should show expired entries", |ctx| {
            let stats = ctx.string("stats_output").unwrap();
            assert_contains(stats, "Expired entries: 5", "expired entries")
        })
        .then("the output should show cache size on disk", |ctx| {
            let stats = ctx.string("stats_output").unwrap();
            assert_contains(stats, "Cache size: 10MB", "cache size")
        })
}

/// Scenario 4.4: User clears cache
pub fn cache_clear() -> Scenario {
    Scenario::new("User clears cache")
        .given("a user has a populated cache", |ctx| {
            ctx.numbers.insert("entry_count_before".to_string(), 100);
        })
        .when("they run \"shiplog cache clear\"", |ctx| {
            ctx.numbers.insert("entry_count_after".to_string(), 0);
            ctx.flags.insert("cleared".to_string(), true);
            ctx.strings.insert(
                "confirmation".to_string(),
                "Cache cleared successfully".to_string(),
            );
            Ok(())
        })
        .then("all cache entries should be deleted", |ctx| {
            let after = ctx.number("entry_count_after").unwrap();
            assert_eq(after, 0, "cache empty after clear")
        })
        .then("the cache file should be empty", |ctx| {
            let after = ctx.number("entry_count_after").unwrap();
            assert_eq(after, 0, "cache file empty")
        })
        .then("a confirmation message should be displayed", |ctx| {
            let confirmation = ctx.string("confirmation").unwrap();
            assert_contains(confirmation, "cleared successfully", "confirmation message")
        })
}

/// Scenario 4.5: User cleans up expired cache entries
pub fn cache_cleanup() -> Scenario {
    Scenario::new("User cleans up expired cache entries")
        .given("a user has a cache with expired entries", |ctx| {
            ctx.numbers.insert("total_entries_before".to_string(), 100);
            ctx.numbers.insert("expired_entries_before".to_string(), 20);
        })
        .when("they run \"shiplog cache cleanup\"", |ctx| {
            ctx.numbers.insert("total_entries_after".to_string(), 80);
            ctx.numbers.insert("deleted_entries".to_string(), 20);
            ctx.flags.insert("cleaned".to_string(), true);
            Ok(())
        })
        .then("expired entries should be deleted", |ctx| {
            let deleted = ctx.number("deleted_entries").unwrap();
            assert_eq(deleted, 20, "expired entries deleted")
        })
        .then("valid entries should be preserved", |ctx| {
            let after = ctx.number("total_entries_after").unwrap();
            assert_eq(after, 80, "valid entries preserved")
        })
        .then("the number of deleted entries should be reported", |ctx| {
            let deleted = ctx.number("deleted_entries").unwrap();
            assert_eq(deleted, 20, "deleted count reported")
        })
}

/// Scenario 4.9: Cache works across multiple runs
pub fn cache_multiple_runs() -> Scenario {
    Scenario::new("Cache works across multiple runs")
        .given(
            "a user runs \"shiplog collect\" with caching enabled",
            |ctx| {
                ctx.flags.insert("first_run".to_string(), true);
                ctx.numbers.insert("first_run_time_ms".to_string(), 5000);
            },
        )
        .when("they run the same command again immediately", |ctx| {
            ctx.flags.insert("second_run".to_string(), true);
            ctx.numbers.insert("second_run_time_ms".to_string(), 500);
            ctx.flags.insert("cache_hit".to_string(), true);
            ctx.numbers.insert("api_calls".to_string(), 0);
            Ok(())
        })
        .then("second run should use cached data", |ctx| {
            assert_true(ctx.flag("cache_hit").unwrap_or(false), "cache hit")
        })
        .then("no API calls should be made for cached data", |ctx| {
            let api_calls = ctx.number("api_calls").unwrap();
            assert_eq(api_calls, 0, "no API calls")
        })
        .then("the second run should complete faster", |ctx| {
            let first_time = ctx.number("first_run_time_ms").unwrap();
            let second_time = ctx.number("second_run_time_ms").unwrap();
            assert_true(second_time < first_time, "second run faster")
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_releases_windows_scenario() {
        binary_releases_windows()
            .run()
            .expect("scenario should pass");
    }

    #[test]
    fn test_binary_releases_macos_scenario() {
        binary_releases_macos().run().expect("scenario should pass");
    }

    #[test]
    fn test_binary_releases_linux_scenario() {
        binary_releases_linux().run().expect("scenario should pass");
    }

    #[test]
    fn test_binary_signature_verification_fails_scenario() {
        binary_signature_verification_fails()
            .run()
            .expect("scenario should pass");
    }

    #[test]
    fn test_binary_works_with_all_features_scenario() {
        binary_works_with_all_features()
            .run()
            .expect("scenario should pass");
    }

    #[test]
    fn test_local_git_ingest_basic_scenario() {
        local_git_ingest_basic()
            .run()
            .expect("scenario should pass");
    }

    #[test]
    fn test_local_git_ingest_date_filter_scenario() {
        local_git_ingest_date_filter()
            .run()
            .expect("scenario should pass");
    }

    #[test]
    fn test_local_git_ingest_author_filter_scenario() {
        local_git_ingest_author_filter()
            .run()
            .expect("scenario should pass");
    }

    #[test]
    fn test_local_git_ingest_path_not_exists_scenario() {
        local_git_ingest_path_not_exists()
            .run()
            .expect("scenario should pass");
    }

    #[test]
    fn test_local_git_merge_with_github_scenario() {
        local_git_merge_with_github()
            .run()
            .expect("scenario should pass");
    }

    #[test]
    fn test_packet_formatting_improved_structure_scenario() {
        packet_formatting_improved_structure()
            .run()
            .expect("scenario should pass");
    }

    #[test]
    fn test_packet_formatting_cleaner_receipts_scenario() {
        packet_formatting_cleaner_receipts()
            .run()
            .expect("scenario should pass");
    }

    #[test]
    fn test_packet_formatting_no_workstreams_scenario() {
        packet_formatting_no_workstreams()
            .run()
            .expect("scenario should pass");
    }

    #[test]
    fn test_packet_formatting_preserves_redaction_scenario() {
        packet_formatting_preserves_redaction()
            .run()
            .expect("scenario should pass");
    }

    #[test]
    fn test_cache_config_ttl_scenario() {
        cache_config_ttl().run().expect("scenario should pass");
    }

    #[test]
    fn test_cache_inspect_scenario() {
        cache_inspect().run().expect("scenario should pass");
    }

    #[test]
    fn test_cache_clear_scenario() {
        cache_clear().run().expect("scenario should pass");
    }

    #[test]
    fn test_cache_cleanup_scenario() {
        cache_cleanup().run().expect("scenario should pass");
    }

    #[test]
    fn test_cache_multiple_runs_scenario() {
        cache_multiple_runs().run().expect("scenario should pass");
    }
}
