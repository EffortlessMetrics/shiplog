//! BDD scenario tests for shiplog-coverage
//!
//! Exercises coverage domain logic through Given/When/Then scenarios.

use chrono::NaiveDate;
use shiplog_coverage::{day_windows, month_windows, window_len_days};
use shiplog_schema::coverage::{Completeness, CoverageSlice, TimeWindow};
use shiplog_testkit::bdd::Scenario;
use shiplog_testkit::bdd::assertions::*;
use shiplog_testkit::bdd::builders::CoverageBuilder;

// ============================================================================
// Scenario: Complete coverage shows 100% completeness
// ============================================================================

#[test]
fn complete_coverage_with_all_receipts() {
    Scenario::new("Complete coverage when all events are fetched")
        .given("a date range covering Q1 2025", |ctx| {
            ctx.strings.insert("since".into(), "2025-01-01".into());
            ctx.strings.insert("until".into(), "2025-04-01".into());
        })
        .given("all slices successfully fetched all results", |ctx| {
            ctx.numbers.insert("total_slices".into(), 3);
            ctx.numbers.insert("complete_slices".into(), 3);
            ctx.flags.insert("all_complete".into(), true);
        })
        .when("the coverage manifest is generated", |ctx| {
            let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
            let until = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
            let windows = month_windows(since, until);

            let slices: Vec<CoverageSlice> = windows
                .iter()
                .map(|w| CoverageSlice {
                    window: w.clone(),
                    query: "github prs".to_string(),
                    total_count: 10,
                    fetched: 10,
                    incomplete_results: Some(false),
                    notes: vec![],
                })
                .collect();

            let all_complete = slices.iter().all(|s| s.fetched == s.total_count);
            ctx.flags.insert("all_complete".into(), all_complete);
            ctx.numbers
                .insert("slice_count".into(), slices.len() as u64);
            Ok(())
        })
        .then("completeness should be Complete", |ctx| {
            assert_true(
                ctx.flag("all_complete").unwrap_or(false),
                "all slices complete",
            )
        })
        .then("all three monthly slices should be present", |ctx| {
            let count = assert_present(ctx.number("slice_count"), "slice count")?;
            assert_eq(count, 3, "monthly slice count")
        })
        .run()
        .expect("complete coverage scenario should pass");
}

#[test]
fn complete_manifest_has_no_warnings() {
    Scenario::new("Complete manifest has zero warnings")
        .given("a fully fetched coverage manifest", |ctx| {
            ctx.flags.insert("has_warnings".into(), false);
        })
        .when("the manifest is built with Complete status", |ctx| {
            let manifest = CoverageBuilder::new("alice")
                .completeness(Completeness::Complete)
                .build();

            ctx.flags
                .insert("has_warnings".into(), !manifest.warnings.is_empty());
            ctx.numbers
                .insert("warning_count".into(), manifest.warnings.len() as u64);
            Ok(())
        })
        .then("there should be zero warnings", |ctx| {
            let count = assert_present(ctx.number("warning_count"), "warning count")?;
            assert_eq(count, 0, "warning count for complete manifest")
        })
        .run()
        .expect("complete manifest warnings scenario should pass");
}

// ============================================================================
// Scenario: Missing receipts are explicitly reported
// ============================================================================

#[test]
fn missing_receipts_flagged_as_partial() {
    Scenario::new("Missing receipts result in Partial completeness")
        .given("a coverage window of January 2025", |ctx| {
            ctx.strings.insert("since".into(), "2025-01-01".into());
            ctx.strings.insert("until".into(), "2025-02-01".into());
        })
        .given(
            "the query returned 50 total but only 30 were fetched",
            |ctx| {
                ctx.numbers.insert("total_count".into(), 50);
                ctx.numbers.insert("fetched".into(), 30);
            },
        )
        .when("the coverage slice is evaluated", |ctx| {
            let total = ctx.number("total_count").unwrap_or(0);
            let fetched = ctx.number("fetched").unwrap_or(0);
            let is_incomplete = fetched < total;
            ctx.flags.insert("incomplete".into(), is_incomplete);
            ctx.numbers.insert("missing".into(), total - fetched);
            Ok(())
        })
        .then("the slice should be marked incomplete", |ctx| {
            assert_true(
                ctx.flag("incomplete").unwrap_or(false),
                "slice marked incomplete",
            )
        })
        .then("20 receipts should be missing", |ctx| {
            let missing = assert_present(ctx.number("missing"), "missing count")?;
            assert_eq(missing, 20, "missing receipt count")
        })
        .run()
        .expect("missing receipts scenario should pass");
}

#[test]
fn warnings_capture_specific_missing_items() {
    Scenario::new("Warnings list specific missing receipts")
        .given("a manifest with partial coverage", |ctx| {
            ctx.strings.insert("completeness".into(), "Partial".into());
        })
        .given("PR #42 and PR #99 were not fetched", |ctx| {
            ctx.strings
                .insert("missing_1".into(), "Missing PR #42 from acme/app".into());
            ctx.strings
                .insert("missing_2".into(), "Missing PR #99 from acme/lib".into());
        })
        .when("the manifest is finalized with warnings", |ctx| {
            let mut manifest = CoverageBuilder::new("bob")
                .completeness(Completeness::Partial)
                .build();

            let w1 = ctx.string("missing_1").unwrap_or("").to_string();
            let w2 = ctx.string("missing_2").unwrap_or("").to_string();
            manifest.warnings.push(w1);
            manifest.warnings.push(w2);

            ctx.numbers
                .insert("warning_count".into(), manifest.warnings.len() as u64);
            ctx.flags.insert(
                "has_pr_42".into(),
                manifest.warnings.iter().any(|w| w.contains("PR #42")),
            );
            ctx.flags.insert(
                "has_pr_99".into(),
                manifest.warnings.iter().any(|w| w.contains("PR #99")),
            );
            Ok(())
        })
        .then("two warnings should be present", |ctx| {
            let count = assert_present(ctx.number("warning_count"), "warning count")?;
            assert_eq(count, 2, "warning count")
        })
        .then("PR #42 warning should be included", |ctx| {
            assert_true(ctx.flag("has_pr_42").unwrap_or(false), "PR #42 in warnings")
        })
        .then("PR #99 warning should be included", |ctx| {
            assert_true(ctx.flag("has_pr_99").unwrap_or(false), "PR #99 in warnings")
        })
        .run()
        .expect("warnings scenario should pass");
}

#[test]
fn slice_notes_document_why_incomplete() {
    Scenario::new("Coverage slice notes explain why results are incomplete")
        .given("a query that hit the GitHub search limit", |ctx| {
            ctx.strings
                .insert("note".into(), "GitHub search limit reached".into());
        })
        .when("the slice is recorded with a note", |ctx| {
            let note = ctx.string("note").unwrap_or("").to_string();
            let slice = CoverageSlice {
                window: TimeWindow {
                    since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                    until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
                },
                query: "github prs".to_string(),
                total_count: 100,
                fetched: 30,
                incomplete_results: Some(true),
                notes: vec![note],
            };

            ctx.numbers
                .insert("note_count".into(), slice.notes.len() as u64);
            ctx.flags.insert(
                "note_explains_limit".into(),
                slice.notes.iter().any(|n| n.contains("limit")),
            );
            Ok(())
        })
        .then("the slice should have at least one note", |ctx| {
            let count = assert_present(ctx.number("note_count"), "note count")?;
            assert_true(count >= 1, "at least one note present")
        })
        .then("the note should explain the limit", |ctx| {
            assert_true(
                ctx.flag("note_explains_limit").unwrap_or(false),
                "note mentions limit",
            )
        })
        .run()
        .expect("slice notes scenario should pass");
}

// ============================================================================
// Scenario: Date range filtering works correctly
// ============================================================================

#[test]
fn date_filtering_with_day_windows() {
    Scenario::new("Day windows correctly partition a week")
        .given("a one-week date range", |ctx| {
            ctx.strings.insert("since".into(), "2025-03-10".into());
            ctx.strings.insert("until".into(), "2025-03-17".into());
        })
        .when("day windows are generated", |ctx| {
            let since = NaiveDate::from_ymd_opt(2025, 3, 10).unwrap();
            let until = NaiveDate::from_ymd_opt(2025, 3, 17).unwrap();
            let windows = day_windows(since, until);

            ctx.numbers
                .insert("window_count".into(), windows.len() as u64);
            ctx.flags.insert(
                "all_one_day".into(),
                windows.iter().all(|w| window_len_days(w) == 1),
            );
            ctx.flags.insert(
                "contiguous".into(),
                windows.windows(2).all(|p| p[0].until == p[1].since),
            );
            Ok(())
        })
        .then("there should be 7 day windows", |ctx| {
            let count = assert_present(ctx.number("window_count"), "window count")?;
            assert_eq(count, 7, "day window count for one week")
        })
        .then("each window should be exactly one day", |ctx| {
            assert_true(
                ctx.flag("all_one_day").unwrap_or(false),
                "all windows are one day",
            )
        })
        .then("windows should be contiguous", |ctx| {
            assert_true(
                ctx.flag("contiguous").unwrap_or(false),
                "day windows are contiguous",
            )
        })
        .run()
        .expect("day windows scenario should pass");
}

#[test]
fn date_filtering_with_month_windows_mid_month() {
    Scenario::new("Month windows handle mid-month start correctly")
        .given("a date range starting mid-January", |ctx| {
            ctx.strings.insert("since".into(), "2025-01-15".into());
            ctx.strings.insert("until".into(), "2025-03-15".into());
        })
        .when("month windows are generated", |ctx| {
            let since = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
            let until = NaiveDate::from_ymd_opt(2025, 3, 15).unwrap();
            let windows = month_windows(since, until);

            ctx.numbers
                .insert("window_count".into(), windows.len() as u64);
            ctx.flags.insert(
                "starts_at_since".into(),
                windows.first().is_some_and(|w| w.since == since),
            );
            ctx.flags.insert(
                "ends_at_until".into(),
                windows.last().is_some_and(|w| w.until == until),
            );
            Ok(())
        })
        .then("there should be 3 month windows", |ctx| {
            let count = assert_present(ctx.number("window_count"), "window count")?;
            assert_eq(count, 3, "month window count for mid-month range")
        })
        .then("first window should start at since date", |ctx| {
            assert_true(
                ctx.flag("starts_at_since").unwrap_or(false),
                "starts at since",
            )
        })
        .then("last window should end at until date", |ctx| {
            assert_true(ctx.flag("ends_at_until").unwrap_or(false), "ends at until")
        })
        .run()
        .expect("month windows mid-month scenario should pass");
}

#[test]
fn time_window_contains_filters_dates_correctly() {
    Scenario::new("TimeWindow::contains filters dates with half-open semantics")
        .given("a time window for March 2025", |ctx| {
            ctx.strings.insert("since".into(), "2025-03-01".into());
            ctx.strings.insert("until".into(), "2025-04-01".into());
        })
        .when("various dates are tested for containment", |ctx| {
            let window = TimeWindow {
                since: NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
                until: NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
            };

            ctx.flags.insert(
                "since_included".into(),
                window.contains(NaiveDate::from_ymd_opt(2025, 3, 1).unwrap()),
            );
            ctx.flags.insert(
                "until_excluded".into(),
                !window.contains(NaiveDate::from_ymd_opt(2025, 4, 1).unwrap()),
            );
            ctx.flags.insert(
                "mid_included".into(),
                window.contains(NaiveDate::from_ymd_opt(2025, 3, 15).unwrap()),
            );
            ctx.flags.insert(
                "before_excluded".into(),
                !window.contains(NaiveDate::from_ymd_opt(2025, 2, 28).unwrap()),
            );
            Ok(())
        })
        .then("since date should be included", |ctx| {
            assert_true(
                ctx.flag("since_included").unwrap_or(false),
                "since is inclusive",
            )
        })
        .then("until date should be excluded", |ctx| {
            assert_true(
                ctx.flag("until_excluded").unwrap_or(false),
                "until is exclusive",
            )
        })
        .then("mid-range date should be included", |ctx| {
            assert_true(
                ctx.flag("mid_included").unwrap_or(false),
                "mid-range date included",
            )
        })
        .then("before-range date should be excluded", |ctx| {
            assert_true(
                ctx.flag("before_excluded").unwrap_or(false),
                "before-range date excluded",
            )
        })
        .run()
        .expect("time window contains scenario should pass");
}

#[test]
fn coverage_manifest_roundtrip_preserves_sources() {
    Scenario::new("Coverage manifest preserves source metadata")
        .given("a manifest built with github source", |ctx| {
            ctx.strings.insert("source".into(), "github".into());
            ctx.strings.insert("mode".into(), "merged".into());
        })
        .when("the manifest is constructed via builder", |ctx| {
            let manifest = CoverageBuilder::new("test-user")
                .completeness(Completeness::Complete)
                .build();

            ctx.flags.insert(
                "has_github_source".into(),
                manifest.sources.contains(&"github".to_string()),
            );
            ctx.strings
                .insert("mode_actual".into(), manifest.mode.clone());
            Ok(())
        })
        .then("github should be listed as a source", |ctx| {
            assert_true(
                ctx.flag("has_github_source").unwrap_or(false),
                "github in sources",
            )
        })
        .then("mode should be merged", |ctx| {
            let mode = assert_present(ctx.string("mode_actual"), "mode")?;
            assert_eq(mode, "merged", "manifest mode")
        })
        .run()
        .expect("manifest sources scenario should pass");
}

// ============================================================================
// Scenario: Multi-source coverage tracking
// ============================================================================

#[test]
fn multi_source_manifest_lists_all_sources() {
    Scenario::new("Multi-source coverage manifest tracks github and manual sources")
        .given("events from both github and manual sources", |ctx| {
            ctx.strings.insert("source_1".into(), "github".into());
            ctx.strings.insert("source_2".into(), "manual".into());
        })
        .when("the manifest is constructed with multiple sources", |ctx| {
            let mut manifest = CoverageBuilder::new("multi-user")
                .completeness(Completeness::Complete)
                .build();
            manifest.sources.push("manual".to_string());

            ctx.numbers
                .insert("source_count".into(), manifest.sources.len() as u64);
            ctx.flags.insert(
                "has_github".into(),
                manifest.sources.contains(&"github".to_string()),
            );
            ctx.flags.insert(
                "has_manual".into(),
                manifest.sources.contains(&"manual".to_string()),
            );
            Ok(())
        })
        .then("two sources should be listed", |ctx| {
            let count = assert_present(ctx.number("source_count"), "source count")?;
            assert_eq(count, 2, "source count")
        })
        .then("github should be a source", |ctx| {
            assert_true(ctx.flag("has_github").unwrap_or(false), "github in sources")
        })
        .then("manual should be a source", |ctx| {
            assert_true(ctx.flag("has_manual").unwrap_or(false), "manual in sources")
        })
        .run()
        .expect("multi-source scenario should pass");
}

// ============================================================================
// Scenario: Partial coverage with multiple slices tracks per-slice status
// ============================================================================

#[test]
fn partial_coverage_per_slice_status() {
    Scenario::new("Partial coverage tracks incomplete slices alongside complete ones")
        .given("a Q1 2025 range with three monthly slices", |ctx| {
            ctx.strings.insert("since".into(), "2025-01-01".into());
            ctx.strings.insert("until".into(), "2025-04-01".into());
        })
        .when("two slices are complete and one is partial", |ctx| {
            let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
            let until = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
            let windows = month_windows(since, until);

            let slices: Vec<CoverageSlice> = windows
                .iter()
                .enumerate()
                .map(|(i, w)| CoverageSlice {
                    window: w.clone(),
                    query: "github prs".to_string(),
                    total_count: 10,
                    fetched: if i == 1 { 7 } else { 10 },
                    incomplete_results: Some(i == 1),
                    notes: if i == 1 {
                        vec!["Rate limit hit".to_string()]
                    } else {
                        vec![]
                    },
                })
                .collect();

            let complete_count = slices.iter().filter(|s| s.fetched == s.total_count).count();
            let incomplete_count = slices.iter().filter(|s| s.fetched < s.total_count).count();
            let total_missing: u64 = slices.iter().map(|s| s.total_count - s.fetched).sum();

            ctx.numbers
                .insert("complete_slices".into(), complete_count as u64);
            ctx.numbers
                .insert("incomplete_slices".into(), incomplete_count as u64);
            ctx.numbers.insert("total_missing".into(), total_missing);
            Ok(())
        })
        .then("two slices should be complete", |ctx| {
            let count = assert_present(ctx.number("complete_slices"), "complete slices")?;
            assert_eq(count, 2, "complete slice count")
        })
        .then("one slice should be incomplete", |ctx| {
            let count = assert_present(ctx.number("incomplete_slices"), "incomplete slices")?;
            assert_eq(count, 1, "incomplete slice count")
        })
        .then("3 total receipts should be missing", |ctx| {
            let missing = assert_present(ctx.number("total_missing"), "total missing")?;
            assert_eq(missing, 3, "total missing receipts")
        })
        .run()
        .expect("partial coverage per-slice scenario should pass");
}

// ============================================================================
// Scenario: Week windows produce correct partitions
// ============================================================================

#[test]
fn week_windows_partition_correctly() {
    Scenario::new("Week windows partition a 2-week range into 2 windows")
        .given("a two-week date range", |ctx| {
            ctx.strings.insert("since".into(), "2025-03-03".into());
            ctx.strings.insert("until".into(), "2025-03-17".into());
        })
        .when("week windows are generated", |ctx| {
            let since = NaiveDate::from_ymd_opt(2025, 3, 3).unwrap();
            let until = NaiveDate::from_ymd_opt(2025, 3, 17).unwrap();
            let windows = shiplog_coverage::week_windows(since, until);

            ctx.numbers
                .insert("window_count".into(), windows.len() as u64);
            ctx.flags.insert(
                "all_seven_days".into(),
                windows.iter().all(|w| window_len_days(w) == 7),
            );
            ctx.flags.insert(
                "contiguous".into(),
                windows.windows(2).all(|p| p[0].until == p[1].since),
            );
            Ok(())
        })
        .then("there should be 2 week windows", |ctx| {
            let count = assert_present(ctx.number("window_count"), "window count")?;
            assert_eq(count, 2, "week window count")
        })
        .then("each window should be exactly 7 days", |ctx| {
            assert_true(
                ctx.flag("all_seven_days").unwrap_or(false),
                "all windows are 7 days",
            )
        })
        .then("windows should be contiguous", |ctx| {
            assert_true(
                ctx.flag("contiguous").unwrap_or(false),
                "week windows are contiguous",
            )
        })
        .run()
        .expect("week windows scenario should pass");
}
