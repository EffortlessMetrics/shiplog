//! BDD scenarios that lock in microcrate public contracts for external reuse.

#[cfg(feature = "microcrate_date_windows")]
use crate::bdd::assertions::assert_present;
#[cfg(any(
    feature = "microcrate_export",
    feature = "microcrate_output_layout",
    feature = "microcrate_cache_key",
    feature = "microcrate_validate",
    feature = "microcrate_storage",
    feature = "microcrate_notify",
    feature = "microcrate_cache_stats",
    feature = "microcrate_cache_expiry",
    feature = "microcrate_redaction_repo",
    feature = "microcrate_date_windows"
))]
use crate::bdd::assertions::assert_true;
#[cfg(feature = "microcrate_export")]
use shiplog_export::{
    ExportData, ExportEvent, ExportFormat, ExportOptions,
    FILE_BUNDLE_MANIFEST_JSON as EXPORT_FILE_BUNDLE_MANIFEST_JSON,
    FILE_COVERAGE_MANIFEST_JSON as EXPORT_FILE_COVERAGE_MANIFEST_JSON,
    FILE_LEDGER_EVENTS_JSONL as EXPORT_FILE_LEDGER_EVENTS_JSONL,
    FILE_PACKET_MD as EXPORT_FILE_PACKET_MD, RunArtifactPaths as ExportRunArtifactPaths,
    export_data, zip_path_for_profile as export_zip_path_for_profile,
};
#[cfg(feature = "microcrate_output_layout")]
use shiplog_output_layout::{
    DIR_PROFILES, FILE_BUNDLE_MANIFEST_JSON, FILE_COVERAGE_MANIFEST_JSON, FILE_LEDGER_EVENTS_JSONL,
    FILE_PACKET_MD, FILE_REDACTION_ALIASES_JSON, PROFILE_INTERNAL, PROFILE_MANAGER, PROFILE_PUBLIC,
    RunArtifactPaths, zip_path_for_profile,
};
#[cfg(feature = "microcrate_export")]
use std::path::Path;

#[cfg(feature = "microcrate_notify")]
use shiplog_notify::{Notification, NotificationService};

#[cfg(feature = "microcrate_cache_key")]
use shiplog_cache_key::CacheKey;

#[cfg(feature = "microcrate_cache_stats")]
use shiplog_cache_stats::CacheStats;

#[cfg(feature = "microcrate_cache_expiry")]
use chrono::{DateTime, Duration, Utc};

#[cfg(feature = "microcrate_cache_expiry")]
use shiplog_cache_expiry::{CacheExpiryWindow, is_expired, is_valid, parse_rfc3339_utc};

#[cfg(feature = "microcrate_redaction_repo")]
use shiplog_redaction_repo::redact_repo_public;

#[cfg(feature = "microcrate_date_windows")]
use chrono::{Datelike, NaiveDate};

#[cfg(feature = "microcrate_date_windows")]
use shiplog_date_windows::{day_windows, month_windows, week_windows, window_len_days};

#[cfg(feature = "microcrate_storage")]
use shiplog_storage::{InMemoryStorage, Storage, StorageKey};

#[cfg(feature = "microcrate_validate")]
use shiplog_validate::{EventValidator, Packet, PacketValidator};

#[cfg(any(
    feature = "microcrate_export",
    feature = "microcrate_output_layout",
    feature = "microcrate_cache_key",
    feature = "microcrate_validate",
    feature = "microcrate_storage",
    feature = "microcrate_notify",
    feature = "microcrate_cache_stats",
    feature = "microcrate_cache_expiry",
    feature = "microcrate_redaction_repo",
    feature = "microcrate_date_windows"
))]
use crate::bdd::Scenario;

#[cfg(feature = "microcrate_export")]
/// Scenario: canonical filenames and zip naming are stable.
pub fn microcrate_export_contract() -> Scenario {
    Scenario::new("Export crate keeps canonical artifact contract stable")
        .when(
            "artifact paths and contract exports are resolved from the microcrate",
            |ctx| {
                let run_dir = Path::new("/tmp/shiplog-contracts");
                let paths = ExportRunArtifactPaths::new(run_dir);
                ctx.paths.insert("packet_md".to_string(), paths.packet_md());
                ctx.paths
                    .insert("ledger_events".to_string(), paths.ledger_events());
                ctx.paths
                    .insert("coverage_manifest".to_string(), paths.coverage_manifest());
                ctx.paths
                    .insert("bundle_manifest".to_string(), paths.bundle_manifest());
                ctx.paths.insert(
                    "manager_profile_packet".to_string(),
                    paths.profile_packet("manager"),
                );

                ctx.strings.insert(
                    "internal_zip".to_string(),
                    export_zip_path_for_profile(run_dir, "internal")
                        .to_string_lossy()
                        .to_string(),
                );
                ctx.strings.insert(
                    "manager_zip".to_string(),
                    export_zip_path_for_profile(run_dir, "manager")
                        .to_string_lossy()
                        .to_string(),
                );

                let data = {
                    let mut data = ExportData::new("contract-run".to_string());
                    data.add_event(ExportEvent {
                        id: "abc".to_string(),
                        source: "github".to_string(),
                        title: "Contract event".to_string(),
                        timestamp: "2025-01-01T00:00:00Z".to_string(),
                    });
                    data
                };
                let payload = export_data(
                    &data,
                    &ExportOptions {
                        format: ExportFormat::Json,
                        pretty: true,
                        include_metadata: true,
                    },
                )
                .map_err(|err| format!("export_data should succeed: {err}"))?;
                ctx.strings.insert("json_output".to_string(), payload);
                Ok(())
            },
        )
        .then("all canonical filenames should be stable", |ctx| {
            let packet_md = crate::bdd::assertions::assert_present(
                ctx.path("packet_md").map(|p| p.to_path_buf()),
                "packet_md",
            )?;
            let ledger = crate::bdd::assertions::assert_present(
                ctx.path("ledger_events").map(|p| p.to_path_buf()),
                "ledger_events",
            )?;
            let coverage = crate::bdd::assertions::assert_present(
                ctx.path("coverage_manifest").map(|p| p.to_path_buf()),
                "coverage_manifest",
            )?;
            let bundle = crate::bdd::assertions::assert_present(
                ctx.path("bundle_manifest").map(|p| p.to_path_buf()),
                "bundle_manifest",
            )?;

            if packet_md.file_name().and_then(|v| v.to_str()) != Some(EXPORT_FILE_PACKET_MD) {
                return Err("packet filename must stay packet.md".to_string());
            }
            if ledger.file_name().and_then(|v| v.to_str()) != Some(EXPORT_FILE_LEDGER_EVENTS_JSONL)
            {
                return Err("ledger filename must stay ledger.events.jsonl".to_string());
            }
            if coverage.file_name().and_then(|v| v.to_str())
                != Some(EXPORT_FILE_COVERAGE_MANIFEST_JSON)
            {
                return Err("coverage filename must stay coverage.manifest.json".to_string());
            }
            if bundle.file_name().and_then(|v| v.to_str()) != Some(EXPORT_FILE_BUNDLE_MANIFEST_JSON)
            {
                return Err("bundle filename must stay bundle.manifest.json".to_string());
            }
            Ok(())
        })
        .then(
            "the profile packet path should be under profiles/manager",
            |ctx| {
                let manager_profile_packet = crate::bdd::assertions::assert_present(
                    ctx.path("manager_profile_packet").map(|p| p.to_path_buf()),
                    "manager_profile_packet",
                )?;
                if manager_profile_packet.file_name().and_then(|v| v.to_str())
                    != Some(EXPORT_FILE_PACKET_MD)
                {
                    return Err("manager profile packet filename must be packet.md".to_string());
                }
                let manager_dir = manager_profile_packet.parent().and_then(|p| p.file_name());
                if manager_dir != Some(std::ffi::OsStr::new("manager")) {
                    return Err(
                        "manager profile packet path should be profiles/manager/packet.md"
                            .to_string(),
                    );
                }
                Ok(())
            },
        )
        .then("zip naming should include profile suffixes", |ctx| {
            let internal =
                crate::bdd::assertions::assert_present(ctx.string("internal_zip"), "internal_zip")?;
            let manager =
                crate::bdd::assertions::assert_present(ctx.string("manager_zip"), "manager_zip")?;
            assert_true(
                internal.ends_with(".zip") && !internal.contains("manager"),
                "internal zip naming",
            )?;
            assert_true(manager.ends_with(".zip"), "manager zip naming")?;
            Ok(())
        })
        .then("JSON export should contain canonical event id", |ctx| {
            let output =
                crate::bdd::assertions::assert_present(ctx.string("json_output"), "json_output")?;
            crate::bdd::assertions::assert_contains(output, "abc", "json output includes event id")
        })
}

#[cfg(feature = "microcrate_output_layout")]
/// Scenario: artifact layout paths are canonical.
pub fn microcrate_output_layout_contract() -> Scenario {
    Scenario::new("Output layout contract keeps canonical paths and zip naming")
        .when("layout paths are derived from a stable API", |ctx| {
            let run_dir = std::path::Path::new("/tmp/shiplog-layout-contract");
            let paths = RunArtifactPaths::new(run_dir);

            ctx.paths.insert("packet_md".to_string(), paths.packet_md());
            ctx.paths
                .insert("ledger_events".to_string(), paths.ledger_events());
            ctx.paths
                .insert("coverage_manifest".to_string(), paths.coverage_manifest());
            ctx.paths
                .insert("bundle_manifest".to_string(), paths.bundle_manifest());
            ctx.paths.insert(
                "manager_packet".to_string(),
                paths.profile_packet(PROFILE_MANAGER),
            );
            ctx.paths.insert(
                "public_packet".to_string(),
                paths.profile_packet(PROFILE_PUBLIC),
            );
            ctx.strings.insert(
                "internal_zip".to_string(),
                zip_path_for_profile(run_dir, PROFILE_INTERNAL)
                    .to_string_lossy()
                    .to_string(),
            );
            ctx.strings.insert(
                "public_zip".to_string(),
                zip_path_for_profile(run_dir, PROFILE_PUBLIC)
                    .to_string_lossy()
                    .to_string(),
            );
            ctx.strings.insert(
                "redaction_aliases_filename".to_string(),
                FILE_REDACTION_ALIASES_JSON.to_string(),
            );
            ctx.strings
                .insert("profiles_dir".to_string(), DIR_PROFILES.to_string());
            ctx.flags.insert(
                "has_redaction_aliases_filename".to_string(),
                !FILE_REDACTION_ALIASES_JSON.is_empty(),
            );
            Ok(())
        })
        .then("all canonical filenames should be stable", |ctx| {
            let packet_md = crate::bdd::assertions::assert_present(
                ctx.path("packet_md").map(|p| p.to_path_buf()),
                "packet_md",
            )?;
            let ledger = crate::bdd::assertions::assert_present(
                ctx.path("ledger_events").map(|p| p.to_path_buf()),
                "ledger_events",
            )?;
            let coverage = crate::bdd::assertions::assert_present(
                ctx.path("coverage_manifest").map(|p| p.to_path_buf()),
                "coverage_manifest",
            )?;
            let bundle = crate::bdd::assertions::assert_present(
                ctx.path("bundle_manifest").map(|p| p.to_path_buf()),
                "bundle_manifest",
            )?;

            if packet_md.file_name().and_then(|v| v.to_str()) != Some(FILE_PACKET_MD) {
                return Err("packet filename must stay packet.md".to_string());
            }
            if ledger.file_name().and_then(|v| v.to_str()) != Some(FILE_LEDGER_EVENTS_JSONL) {
                return Err("ledger filename must stay ledger.events.jsonl".to_string());
            }
            if coverage.file_name().and_then(|v| v.to_str()) != Some(FILE_COVERAGE_MANIFEST_JSON) {
                return Err("coverage filename must stay coverage.manifest.json".to_string());
            }
            if bundle.file_name().and_then(|v| v.to_str()) != Some(FILE_BUNDLE_MANIFEST_JSON) {
                return Err("bundle filename must stay bundle.manifest.json".to_string());
            }
            let expected_redaction_aliases = crate::bdd::assertions::assert_present(
                ctx.string("redaction_aliases_filename").map(String::from),
                "redaction_aliases_filename",
            )?;
            if expected_redaction_aliases != FILE_REDACTION_ALIASES_JSON {
                return Err("redaction alias filename must stay redaction.aliases.json".to_string());
            }

            let manager_packet = crate::bdd::assertions::assert_present(
                ctx.path("manager_packet").map(|p| p.to_path_buf()),
                "manager_packet",
            )?;
            let public_packet = crate::bdd::assertions::assert_present(
                ctx.path("public_packet").map(|p| p.to_path_buf()),
                "public_packet",
            )?;
            if manager_packet.parent().and_then(|p| p.file_name())
                != Some(std::ffi::OsStr::new(PROFILE_MANAGER))
            {
                return Err("manager profile should be under profiles/manager".to_string());
            }
            if public_packet.parent().and_then(|p| p.file_name())
                != Some(std::ffi::OsStr::new(PROFILE_PUBLIC))
            {
                return Err("public profile should be under profiles/public".to_string());
            }
            let has_profiles_root = ctx.string("profiles_dir").unwrap_or_default();
            if has_profiles_root != DIR_PROFILES {
                return Err("profiles dir constant should remain profiles".to_string());
            }
            assert_true(
                ctx.flag("has_redaction_aliases_filename").unwrap_or(false),
                "has redaction alias constant",
            )?;
            Ok(())
        })
        .then("zip naming should include profile suffixes", |ctx| {
            let internal =
                crate::bdd::assertions::assert_present(ctx.string("internal_zip"), "internal_zip")?;
            let public =
                crate::bdd::assertions::assert_present(ctx.string("public_zip"), "public_zip")?;
            assert_true(internal.ends_with(".zip"), "internal zip naming")?;
            assert_true(
                !internal.contains(PROFILE_PUBLIC),
                "internal zip no profile suffix",
            )?;
            assert_true(public.ends_with(".zip"), "public zip naming")?;
            assert_true(
                public.contains(PROFILE_PUBLIC),
                "public zip includes profile suffix",
            )?;
            Ok(())
        })
}

#[cfg(feature = "microcrate_cache_key")]
/// Scenario: cache-key crate keeps canonical key contracts stable.
pub fn microcrate_cache_key_contract() -> Scenario {
    Scenario::new("Cache-key crate keeps canonical key formats stable")
        .given("canonical GitHub and GitLab request identifiers", |ctx| {
            ctx.strings
                .insert("query".to_string(), "is:pr author:octocat".to_string());
            ctx.strings.insert(
                "pr_url".to_string(),
                "https://api.github.com/repos/octo/repo/pulls/42".to_string(),
            );
            ctx.numbers.insert("project_id".to_string(), 123);
            ctx.numbers.insert("mr_iid".to_string(), 77);
        })
        .when(
            "cache keys are generated for all supported request types",
            |ctx| {
                let query = ctx.string("query").unwrap_or("");
                let pr_url = ctx.string("pr_url").unwrap_or("");
                let project_id = ctx.number("project_id").unwrap_or(0);
                let mr_iid = ctx.number("mr_iid").unwrap_or(0);

                let search = CacheKey::search(query, 2, 100);
                let details = CacheKey::pr_details(pr_url);
                let reviews = CacheKey::pr_reviews(pr_url, 2);
                let notes = CacheKey::mr_notes(project_id, mr_iid, 2);

                ctx.strings.insert("search".to_string(), search);
                ctx.strings.insert("details".to_string(), details);
                ctx.strings.insert("reviews".to_string(), reviews);
                ctx.strings.insert("notes".to_string(), notes);
                Ok(())
            },
        )
        .then(
            "every key should keep its canonical namespace and paging suffix",
            |ctx| {
                let search =
                    crate::bdd::assertions::assert_present(ctx.string("search"), "search")?;
                let details =
                    crate::bdd::assertions::assert_present(ctx.string("details"), "details")?;
                let reviews =
                    crate::bdd::assertions::assert_present(ctx.string("reviews"), "reviews")?;
                let notes = crate::bdd::assertions::assert_present(ctx.string("notes"), "notes")?;

                assert_true(search.starts_with("search:"), "search namespace")?;
                assert_true(search.ends_with(":page2:per100"), "search paging suffix")?;
                assert_true(details.starts_with("pr:details:"), "details namespace")?;
                assert_true(reviews.starts_with("pr:reviews:"), "reviews namespace")?;
                assert_true(notes.starts_with("gitlab:mr:notes:"), "notes namespace")?;
                assert_true(details != reviews, "details and reviews are distinct")?;
                assert_true(search != notes, "search and notes are distinct")
            },
        )
}

#[cfg(feature = "microcrate_cache_stats")]
/// Scenario: cache-stats crate keeps normalization contracts stable.
pub fn microcrate_cache_stats_contract() -> Scenario {
    Scenario::new("Cache-stats crate keeps normalization contracts stable")
        .when(
            "raw cache row counts are normalized through the microcrate API",
            |ctx| {
                let normal = CacheStats::from_raw_counts(12, 3, 4 * 1024 * 1024 + 88);
                let clamped = CacheStats::from_raw_counts(-4, 99, -10);

                ctx.numbers
                    .insert("normal_total".to_string(), normal.total_entries as u64);
                ctx.numbers
                    .insert("normal_expired".to_string(), normal.expired_entries as u64);
                ctx.numbers
                    .insert("normal_valid".to_string(), normal.valid_entries as u64);
                ctx.numbers
                    .insert("normal_mb".to_string(), normal.cache_size_mb);

                ctx.numbers
                    .insert("clamped_total".to_string(), clamped.total_entries as u64);
                ctx.numbers.insert(
                    "clamped_expired".to_string(),
                    clamped.expired_entries as u64,
                );
                ctx.numbers
                    .insert("clamped_valid".to_string(), clamped.valid_entries as u64);
                ctx.numbers
                    .insert("clamped_mb".to_string(), clamped.cache_size_mb);
                Ok(())
            },
        )
        .then(
            "normalized values should satisfy the canonical contract",
            |ctx| {
                assert_true(
                    ctx.number("normal_total").unwrap_or(0) == 12,
                    "normal total",
                )?;
                assert_true(
                    ctx.number("normal_expired").unwrap_or(0) == 3,
                    "normal expired",
                )?;
                assert_true(ctx.number("normal_valid").unwrap_or(0) == 9, "normal valid")?;
                assert_true(ctx.number("normal_mb").unwrap_or(0) == 4, "normal size mb")?;

                assert_true(
                    ctx.number("clamped_total").unwrap_or(1) == 0,
                    "clamped total",
                )?;
                assert_true(
                    ctx.number("clamped_expired").unwrap_or(1) == 0,
                    "clamped expired",
                )?;
                assert_true(
                    ctx.number("clamped_valid").unwrap_or(1) == 0,
                    "clamped valid",
                )?;
                assert_true(
                    ctx.number("clamped_mb").unwrap_or(1) == 0,
                    "clamped size mb",
                )?;

                let total = ctx.number("normal_total").unwrap_or(0);
                let expired = ctx.number("normal_expired").unwrap_or(0);
                let valid = ctx.number("normal_valid").unwrap_or(0);
                assert_true(valid + expired == total, "valid + expired == total")
            },
        )
}

#[cfg(feature = "microcrate_cache_expiry")]
/// Scenario: cache-expiry crate keeps timestamp-window contracts stable.
pub fn microcrate_cache_expiry_contract() -> Scenario {
    Scenario::new("Cache-expiry crate keeps canonical timestamp window contracts stable")
        .when(
            "a cache window is derived from a fixed base timestamp and ttl",
            |ctx| {
                let base =
                    DateTime::<Utc>::from_timestamp(1_700_000_000, 0).expect("valid timestamp");
                let window = CacheExpiryWindow::from_base(base, Duration::seconds(90));
                let at_expiry = base + Duration::seconds(90);
                let before_expiry = base + Duration::seconds(89);

                ctx.strings
                    .insert("cached_at".to_string(), window.cached_at_rfc3339());
                ctx.strings
                    .insert("expires_at".to_string(), window.expires_at_rfc3339());
                ctx.flags.insert(
                    "valid_before_expiry".to_string(),
                    is_valid(window.expires_at, before_expiry),
                );
                ctx.flags.insert(
                    "expired_at_boundary".to_string(),
                    is_expired(window.expires_at, at_expiry),
                );
                Ok(())
            },
        )
        .then(
            "ttl delta and boundary predicates should stay canonical",
            |ctx| {
                let cached_at_raw =
                    crate::bdd::assertions::assert_present(ctx.string("cached_at"), "cached_at")?;
                let expires_at_raw =
                    crate::bdd::assertions::assert_present(ctx.string("expires_at"), "expires_at")?;

                let cached_at = parse_rfc3339_utc(cached_at_raw)
                    .map_err(|err| format!("cached_at should parse as RFC3339: {err}"))?;
                let expires_at = parse_rfc3339_utc(expires_at_raw)
                    .map_err(|err| format!("expires_at should parse as RFC3339: {err}"))?;

                assert_true(
                    expires_at - cached_at == Duration::seconds(90),
                    "ttl delta remains exact",
                )?;
                assert_true(
                    ctx.flag("valid_before_expiry").unwrap_or(false),
                    "entry valid before expiry",
                )?;
                assert_true(
                    ctx.flag("expired_at_boundary").unwrap_or(false),
                    "entry expired at boundary",
                )
            },
        )
}

#[cfg(feature = "microcrate_date_windows")]
/// Scenario: date-windows crate keeps partitioning contracts stable.
pub fn microcrate_date_windows_contract() -> Scenario {
    Scenario::new("Date-windows crate keeps partitioning contracts stable")
        .given("a canonical date range", |ctx| {
            ctx.strings
                .insert("since".to_string(), "2025-01-15".to_string());
            ctx.strings
                .insert("until".to_string(), "2025-04-02".to_string());
        })
        .when("day, week, and month windows are derived", |ctx| {
            let since_raw = assert_present(ctx.string("since"), "since")?;
            let until_raw = assert_present(ctx.string("until"), "until")?;

            let since = NaiveDate::parse_from_str(since_raw, "%Y-%m-%d")
                .map_err(|err| format!("since should parse as date: {err}"))?;
            let until = NaiveDate::parse_from_str(until_raw, "%Y-%m-%d")
                .map_err(|err| format!("until should parse as date: {err}"))?;

            let month = month_windows(since, until);
            let week = week_windows(since, until);
            let day = day_windows(since, until);

            let requested_days = if until > since {
                (until - since).num_days()
            } else {
                0
            };
            let month_days = month.iter().map(window_len_days).sum::<i64>();
            let week_days = week.iter().map(window_len_days).sum::<i64>();
            let day_days = day.iter().map(window_len_days).sum::<i64>();

            ctx.numbers
                .insert("requested_days".to_string(), requested_days as u64);
            ctx.numbers
                .insert("month_window_count".to_string(), month.len() as u64);
            ctx.numbers
                .insert("week_window_count".to_string(), week.len() as u64);
            ctx.numbers
                .insert("day_window_count".to_string(), day.len() as u64);
            ctx.numbers
                .insert("month_window_days".to_string(), month_days as u64);
            ctx.numbers
                .insert("week_window_days".to_string(), week_days as u64);
            ctx.numbers
                .insert("day_window_days".to_string(), day_days as u64);

            ctx.flags.insert(
                "month_partition".to_string(),
                check_partition_contract(&month, since, until),
            );
            ctx.flags.insert(
                "week_partition".to_string(),
                check_partition_contract(&week, since, until),
            );
            ctx.flags.insert(
                "day_partition".to_string(),
                check_partition_contract(&day, since, until),
            );
            ctx.flags.insert(
                "week_internal_monday".to_string(),
                week_internal_boundaries_are_monday(&week),
            );
            ctx.flags.insert(
                "day_units".to_string(),
                day.iter().all(|window| window_len_days(window) == 1),
            );
            Ok(())
        })
        .then(
            "window partitioning should remain contiguous, complete, and unit-accurate",
            |ctx| {
                let requested_days = ctx.number("requested_days").unwrap_or(0);
                assert_true(
                    ctx.flag("month_partition").unwrap_or(false),
                    "month windows partition",
                )?;
                assert_true(
                    ctx.flag("week_partition").unwrap_or(false),
                    "week windows partition",
                )?;
                assert_true(
                    ctx.flag("day_partition").unwrap_or(false),
                    "day windows partition",
                )?;
                assert_true(
                    ctx.flag("week_internal_monday").unwrap_or(false),
                    "week internal boundaries align to monday",
                )?;
                assert_true(
                    ctx.flag("day_units").unwrap_or(false),
                    "day windows are unit days",
                )?;
                assert_true(
                    ctx.number("month_window_days").unwrap_or(0) == requested_days,
                    "month windows cover requested range",
                )?;
                assert_true(
                    ctx.number("week_window_days").unwrap_or(0) == requested_days,
                    "week windows cover requested range",
                )?;
                assert_true(
                    ctx.number("day_window_days").unwrap_or(0) == requested_days,
                    "day windows cover requested range",
                )?;
                assert_true(
                    ctx.number("month_window_count").unwrap_or(0)
                        + ctx.number("week_window_count").unwrap_or(0)
                        + ctx.number("day_window_count").unwrap_or(0)
                        > 0,
                    "all granularities produce windows when range is non-empty",
                )?;
                Ok(())
            },
        )
}

#[cfg(feature = "microcrate_date_windows")]
fn check_partition_contract(
    windows: &[shiplog_schema::coverage::TimeWindow],
    since: NaiveDate,
    until: NaiveDate,
) -> bool {
    if windows.is_empty() {
        return until <= since;
    }

    if windows.first().unwrap().since != since {
        return false;
    }
    if windows.last().unwrap().until != until {
        return false;
    }

    for pair in windows.windows(2) {
        if pair[0].until != pair[1].since || pair[0].since >= pair[0].until {
            return false;
        }
    }

    windows.last().unwrap().since < windows.last().unwrap().until
}

#[cfg(feature = "microcrate_date_windows")]
fn week_internal_boundaries_are_monday(windows: &[shiplog_schema::coverage::TimeWindow]) -> bool {
    if windows.is_empty() || windows.len() == 1 {
        return true;
    }

    windows
        .iter()
        .take(windows.len() - 1)
        .all(|window| window.until.weekday() == chrono::Weekday::Mon)
}

#[cfg(feature = "microcrate_redaction_repo")]
/// Scenario: redaction-repo crate keeps public repo redaction contract stable.
pub fn microcrate_redaction_repo_contract() -> Scenario {
    Scenario::new("Redaction-repo crate keeps canonical public repo redaction contract stable")
        .given(
            "a private repository reference and canonical alias inputs",
            |ctx| {
                ctx.strings
                    .insert("repo_name".to_string(), "acme/top-secret".to_string());
                ctx.strings.insert(
                    "repo_url".to_string(),
                    "https://github.com/acme/top-secret".to_string(),
                );
            },
        )
        .when(
            "public repo redaction is applied through the microcrate API",
            |ctx| {
                let repo_name = ctx.string("repo_name").unwrap_or("");
                let repo_url = ctx.string("repo_url").unwrap_or("");
                let alias = |kind: &str, value: &str| format!("{kind}:{}", value.replace('/', "_"));
                let repo = shiplog_schema::event::RepoRef {
                    full_name: repo_name.to_string(),
                    html_url: Some(repo_url.to_string()),
                    visibility: shiplog_schema::event::RepoVisibility::Private,
                };
                let redacted = redact_repo_public(&repo, &alias);

                ctx.strings
                    .insert("aliased_name".to_string(), redacted.full_name);
                ctx.flags
                    .insert("url_removed".to_string(), redacted.html_url.is_none());
                ctx.flags.insert(
                    "visibility_unknown".to_string(),
                    matches!(
                        redacted.visibility,
                        shiplog_schema::event::RepoVisibility::Unknown
                    ),
                );
                Ok(())
            },
        )
        .then(
            "repo name should be aliased and url/visibility should be sanitized",
            |ctx| {
                let aliased_name = crate::bdd::assertions::assert_present(
                    ctx.string("aliased_name").map(String::from),
                    "aliased_name",
                )?;
                assert_true(
                    aliased_name == "repo:acme_top-secret",
                    "canonical repo alias",
                )?;
                assert_true(ctx.flag("url_removed").unwrap_or(false), "url removed")?;
                assert_true(
                    ctx.flag("visibility_unknown").unwrap_or(false),
                    "visibility unknown",
                )
            },
        )
}

#[cfg(feature = "microcrate_validate")]
/// Scenario: validation contract for ids, sources, and packet shells.
pub fn microcrate_validate_contract() -> Scenario {
    Scenario::new("Validate crate preserves event and packet validation contracts")
        .given("an event id and team source payload", |ctx| {
            ctx.strings
                .insert("good_event_id".to_string(), "a".repeat(64));
            ctx.strings
                .insert("bad_event_id".to_string(), "bad".to_string());
            ctx.strings
                .insert("good_source".to_string(), "github".to_string());
        })
        .when(
            "validation routines run against known-good and known-bad values",
            |ctx| {
                let good_id = ctx.string("good_event_id").unwrap_or("");
                let bad_id = ctx.string("bad_event_id").unwrap_or("");
                let good_source = ctx.string("good_source").unwrap_or("");

                let valid_id = EventValidator::validate_event_id(good_id);
                let invalid_id = EventValidator::validate_event_id(bad_id);
                let valid_source = EventValidator::validate_source(good_source);
                let invalid_source = EventValidator::validate_source("bad-source");

                ctx.flags
                    .insert("valid_event_id".to_string(), valid_id.is_ok());
                ctx.flags
                    .insert("invalid_event_id".to_string(), invalid_id.is_err());
                ctx.flags
                    .insert("valid_source".to_string(), valid_source.is_ok());
                ctx.flags
                    .insert("invalid_source".to_string(), invalid_source.is_err());

                let packet = Packet {
                    id: "contract-packet".to_string(),
                    events: vec!["evt".to_string()],
                };
                let packet_empty = Packet {
                    id: "empty".to_string(),
                    events: vec![],
                };
                ctx.flags.insert(
                    "valid_packet".to_string(),
                    PacketValidator::validate_packet(&packet).is_ok(),
                );
                ctx.flags.insert(
                    "invalid_packet".to_string(),
                    PacketValidator::validate_packet(&packet_empty).is_err(),
                );
                Ok(())
            },
        )
        .then(
            "good values should validate and bad values should fail",
            |ctx| {
                assert_true(
                    ctx.flag("valid_event_id").unwrap_or(false),
                    "good event id validated",
                )?;
                assert_true(
                    ctx.flag("invalid_event_id").unwrap_or(false),
                    "bad event id rejected",
                )?;
                assert_true(
                    ctx.flag("valid_source").unwrap_or(false),
                    "good source validated",
                )?;
                assert_true(
                    ctx.flag("invalid_source").unwrap_or(false),
                    "bad source rejected",
                )?;
                assert_true(
                    ctx.flag("valid_packet").unwrap_or(false),
                    "packet with events valid",
                )?;
                assert_true(
                    ctx.flag("invalid_packet").unwrap_or(false),
                    "packet without events rejected",
                )
            },
        )
}

#[cfg(feature = "microcrate_storage")]
/// Scenario: storage crate keeps a deterministic in-memory contract.
pub fn microcrate_storage_contract() -> Scenario {
    Scenario::new("Storage crate supports deterministic in-memory operations")
        .given("an empty memory-backed store", |ctx| {
            ctx.flags.insert("empty".to_string(), true);
        })
        .when("values are written, read, listed, then deleted", |ctx| {
            let mut store = InMemoryStorage::new();
            let key = StorageKey::from_path("contract/run_001");
            store
                .set(&key, b"payload".to_vec())
                .map_err(|err| format!("set should succeed: {err}"))?;
            let exists = store
                .exists(&key)
                .map_err(|err| format!("exists should succeed: {err}"))?;
            let value = store
                .get(&key)
                .map_err(|err| format!("get should succeed: {err}"))?;
            let listed = store
                .list(&StorageKey::from_path("contract"))
                .map_err(|err| format!("list should succeed: {err}"))?;
            let has_prefix = listed.iter().any(|k| k.0 == key.0);
            store
                .delete(&key)
                .map_err(|err| format!("delete should succeed: {err}"))?;
            let exists_after = store
                .exists(&key)
                .map_err(|err| format!("exists should succeed after delete: {err}"))?;

            ctx.flags
                .insert("write_read_roundtrip".to_string(), value.is_some());
            ctx.flags.insert("exists_before_delete".to_string(), exists);
            ctx.flags.insert("has_prefix_key".to_string(), has_prefix);
            ctx.flags
                .insert("missing_after_delete".to_string(), !exists_after);
            Ok(())
        })
        .then(
            "write/read/list/delete should behave deterministically",
            |ctx| {
                assert_true(
                    ctx.flag("write_read_roundtrip").unwrap_or(false),
                    "write/read roundtrip",
                )?;
                assert_true(
                    ctx.flag("exists_before_delete").unwrap_or(false),
                    "exists before delete",
                )?;
                assert_true(ctx.flag("has_prefix_key").unwrap_or(false), "key listed")?;
                assert_true(
                    ctx.flag("missing_after_delete").unwrap_or(false),
                    "key removed",
                )
            },
        )
}

#[cfg(feature = "microcrate_notify")]
/// Scenario: notification crate exposes constructor and send contracts.
pub fn microcrate_notify_contract() -> Scenario {
    Scenario::new("Notify crate provides a stable notification contract")
        .given("a default notification service", |ctx| {
            ctx.flags.insert("service_ready".to_string(), true);
        })
        .when("a high-priority packet notification is sent", |ctx| {
            let service = NotificationService::default();
            let note = Notification::alert("Contract Packet", "Stable contract validation");
            service
                .send(&note)
                .map_err(|err| format!("send should succeed: {err}"))?;
            ctx.strings
                .insert("priority".to_string(), format!("{:?}", note.priority));
            ctx.flags.insert("notification_sent".to_string(), true);
            Ok(())
        })
        .then(
            "the notification should be produced with a stable alert priority",
            |ctx| {
                assert_true(
                    ctx.flag("notification_sent").unwrap_or(false),
                    "notification sent",
                )?;
                assert_true(
                    ctx.string("priority").unwrap_or("").contains("High"),
                    "notification priority",
                )
            },
        )
}
