//! Proptest strategies for shiplog property-based testing
//!
//! This module provides reusable proptest strategies for generating valid test data
//! across all shiplog crates.

use chrono::{NaiveDate, TimeZone, Utc};
use proptest::prelude::*;
use shiplog_ids::{EventId, WorkstreamId};
use shiplog_schema::coverage::{Completeness, CoverageManifest, TimeWindow};
use shiplog_schema::event::*;
use shiplog_schema::workstream::{Workstream, WorkstreamFile, WorkstreamStats};

// ============================================================================
// Base Strategies
// ============================================================================

/// Strategy for generating valid NaiveDate values
pub fn strategy_naive_date() -> impl Strategy<Value = NaiveDate> {
    // Generate dates from 2020-01-01 to 2030-12-31
    prop::num::i32::ANY
        .prop_map(|days| {
            NaiveDate::from_ymd_opt(2020, 1, 1)
                .unwrap()
                .checked_add_days(chrono::Days::new((days.abs() % 4000) as u64))
                .unwrap()
        })
}

/// Strategy for generating valid DateTime<Utc> values
pub fn strategy_datetime_utc() -> impl Strategy<Value = chrono::DateTime<Utc>> {
    strategy_naive_date().prop_map(|date| {
        Utc.with_ymd_and_hms(
            date.year(),
            date.month(),
            date.day(),
            0,
            0,
            0,
        )
        .unwrap()
    })
}

/// Strategy for generating valid date ranges (since < until)
pub fn strategy_date_range() -> impl Strategy<Value = (NaiveDate, NaiveDate)> {
    (strategy_naive_date(), strategy_naive_date()).prop_map(|(d1, d2)| {
        if d1 < d2 {
            (d1, d2)
        } else {
            (d2, d1)
        }
    })
}

/// Strategy for generating non-empty strings
pub fn strategy_non_empty_string() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_-]{1,100}"
}

/// Strategy for generating repository names in "owner/repo" format
pub fn strategy_repo_name() -> impl Strategy<Value = String> {
    "[a-z0-9_-]{3,20}/[a-z0-9_-]{3,50}"
}

/// Strategy for generating URLs
pub fn strategy_url() -> impl Strategy<Value = String> {
    "https://[a-z0-9.-]{5,50}/[a-z0-9._/-]{5,100}"
}

/// Strategy for generating PR numbers
pub fn strategy_pr_number() -> impl Strategy<Value = u64> {
    1u64..10000
}

/// Strategy for generating positive counts
pub fn strategy_positive_count() -> impl Strategy<Value = usize> {
    0usize..1000
}

/// Strategy for generating SourceSystem enum values
pub fn strategy_source_system() -> impl Strategy<Value = SourceSystem> {
    prop_oneof![
        Just(SourceSystem::Github),
        Just(SourceSystem::LocalGit),
        Just(SourceSystem::Manual),
    ]
}

/// Strategy for generating RepoVisibility enum values
pub fn strategy_repo_visibility() -> impl Strategy<Value = RepoVisibility> {
    prop_oneof![
        Just(RepoVisibility::Public),
        Just(RepoVisibility::Private),
        Just(RepoVisibility::Unknown),
    ]
}

/// Strategy for generating PullRequestState enum values
pub fn strategy_pr_state() -> impl Strategy<Value = PullRequestState> {
    prop_oneof![
        Just(PullRequestState::Open),
        Just(PullRequestState::Closed),
        Just(PullRequestState::Merged),
    ]
}

/// Strategy for generating EventKind enum values
pub fn strategy_event_kind() -> impl Strategy<Value = EventKind> {
    prop_oneof![
        Just(EventKind::PullRequest),
        Just(EventKind::Review),
        Just(EventKind::Manual),
    ]
}

/// Strategy for generating Completeness enum values
pub fn strategy_completeness() -> impl Strategy<Value = Completeness> {
    prop_oneof![
        Just(Completeness::Complete),
        Just(Completeness::Partial),
    ]
}

// ============================================================================
// Event Strategies
// ============================================================================

/// Strategy for generating Actor values
pub fn strategy_actor() -> impl Strategy<Value = Actor> {
    "[a-zA-Z0-9_-]{1,50}".prop_map(|login| Actor {
        login,
        id: None,
    })
}

/// Strategy for generating RepoRef values
pub fn strategy_repo_ref() -> impl Strategy<Value = RepoRef> {
    (
        strategy_repo_name(),
        strategy_url(),
        strategy_repo_visibility(),
    )
        .prop_map(|(full_name, html_url, visibility)| RepoRef {
            full_name,
            html_url: Some(html_url),
            visibility,
        })
}

/// Strategy for generating Link values
pub fn strategy_link() -> impl Strategy<Value = Link> {
    (
        "[a-z]{1,20}",
        strategy_url(),
    )
        .prop_map(|(label, url)| Link { label, url })
}

/// Strategy for generating SourceRef values
pub fn strategy_source_ref() -> impl Strategy<Value = SourceRef> {
    (
        strategy_source_system(),
        proptest::option::of(strategy_url()),
    )
        .prop_map(|(system, url)| SourceRef {
            system,
            url,
            opaque_id: None,
        })
}

/// Strategy for generating TimeWindow values
pub fn strategy_time_window() -> impl Strategy<Value = TimeWindow> {
    strategy_date_range().prop_map(|(since, until)| TimeWindow { since, until })
}

/// Strategy for generating PullRequestEvent values
pub fn strategy_pr_payload() -> impl Strategy<Value = PullRequestEvent> {
    (
        strategy_pr_number(),
        strategy_non_empty_string(),
        strategy_pr_state(),
        strategy_datetime_utc(),
        proptest::option::of(strategy_datetime_utc()),
        proptest::option::of(strategy_positive_count()),
        proptest::option::of(strategy_positive_count()),
        proptest::option::of(strategy_positive_count()),
        proptest::option::of(proptest::collection::vec("[a-zA-Z0-9_/-]{1,100}", 0..10)),
        proptest::option::of(strategy_time_window()),
    )
        .prop_map(
            |(
                number,
                title,
                state,
                created_at,
                merged_at,
                additions,
                deletions,
                changed_files,
                touched_paths_hint,
                window,
            )| PullRequestEvent {
                number,
                title,
                state,
                created_at,
                merged_at,
                additions,
                deletions,
                changed_files,
                touched_paths_hint,
                window,
            },
        )
}

/// Strategy for generating ReviewEvent values
pub fn strategy_review_payload() -> impl Strategy<Value = ReviewEvent> {
    (
        strategy_pr_number(),
        strategy_non_empty_string(),
        strategy_pr_state(),
        strategy_datetime_utc(),
        strategy_non_empty_string(),
    )
        .prop_map(
            |(pull_number, pull_title, state, submitted_at, author)| ReviewEvent {
                pull_number,
                pull_title,
                state,
                submitted_at,
                author,
            },
        )
}

/// Strategy for generating ManualEvent values
pub fn strategy_manual_payload() -> impl Strategy<Value = ManualEvent> {
    (
        strategy_non_empty_string(),
        proptest::option::of("[a-zA-Z0-9_ ]{10,500}"),
        proptest::option::of("[a-zA-Z0-9_ ]{10,200}"),
        proptest::option::of(strategy_url()),
        proptest::option::of(strategy_time_window()),
    )
        .prop_map(
            |(title, description, impact, receipt_url, window)| ManualEvent {
                title,
                description,
                impact,
                receipt_url,
                window,
            },
        )
}

/// Strategy for generating EventPayload values
pub fn strategy_event_payload() -> impl Strategy<Value = EventPayload> {
    prop_oneof![
        strategy_pr_payload().prop_map(EventPayload::PullRequest),
        strategy_review_payload().prop_map(EventPayload::Review),
        strategy_manual_payload().prop_map(EventPayload::Manual),
    ]
}

/// Strategy for generating EventEnvelope values
pub fn strategy_event_envelope() -> impl Strategy<Value = EventEnvelope> {
    (
        strategy_event_kind(),
        strategy_event_payload(),
        strategy_actor(),
        strategy_repo_ref(),
        strategy_source_ref(),
        proptest::collection::vec(strategy_link(), 0..5),
        proptest::collection::vec("[a-z]{1,20}", 0..5),
    )
        .prop_map(
            |(kind, payload, actor, repo, source, links, tags)| {
                // Generate ID based on kind and repo
                let id = match &payload {
                    EventPayload::PullRequest(pr) => EventId::from_parts([
                        "github",
                        "pr",
                        &repo.full_name,
                        &pr.number.to_string(),
                    ]),
                    EventPayload::Review(r) => EventId::from_parts([
                        "github",
                        "review",
                        &repo.full_name,
                        &r.pull_number.to_string(),
                    ]),
                    EventPayload::Manual(_) => EventId::from_parts(["manual", &repo.full_name]),
                };

                EventEnvelope {
                    id,
                    kind,
                    occurred_at: Utc::now(),
                    actor,
                    repo,
                    payload,
                    tags,
                    links,
                    source,
                }
            },
        )
}

/// Strategy for generating a vector of EventEnvelope values
pub fn strategy_event_vec(max_size: usize) -> impl Strategy<Value = Vec<EventEnvelope>> {
    proptest::collection::vec(strategy_event_envelope(), 0..max_size)
}

// ============================================================================
// Coverage Strategies
// ============================================================================

/// Strategy for generating CoverageSlice values
pub fn strategy_coverage_slice() -> impl Strategy<Value = shiplog_schema::coverage::CoverageSlice> {
    (
        strategy_time_window(),
        strategy_positive_count(),
        strategy_positive_count(),
        strategy_completeness(),
        proptest::collection::vec("[a-zA-Z0-9_ ]{10,100}", 0..5),
    )
        .prop_map(
            |(window, fetched, total_count, completeness, warnings)| {
                shiplog_schema::coverage::CoverageSlice {
                    window,
                    fetched: fetched as u64,
                    total_count: total_count as u64,
                    completeness,
                    warnings,
                }
            },
        )
}

/// Strategy for generating CoverageManifest values
pub fn strategy_coverage_manifest() -> impl Strategy<Value = CoverageManifest> {
    (
        "[a-zA-Z0-9_-]{1,50}",
        strategy_date_range(),
        proptest::collection::vec(strategy_coverage_slice(), 0..10),
        proptest::collection::vec("[a-zA-Z0-9_ ]{10,100}", 0..5),
        strategy_completeness(),
    )
        .prop_map(
            |(user, (since, until), slices, warnings, completeness)| CoverageManifest {
                run_id: shiplog_ids::RunId::now("test"),
                generated_at: Utc::now(),
                user,
                window: TimeWindow { since, until },
                mode: "merged".to_string(),
                sources: vec!["github".to_string()],
                slices,
                warnings,
                completeness,
            },
        )
}

// ============================================================================
// Workstream Strategies
// ============================================================================

/// Strategy for generating WorkstreamStats values
pub fn strategy_workstream_stats() -> impl Strategy<Value = WorkstreamStats> {
    (
        strategy_positive_count(),
        strategy_positive_count(),
        strategy_positive_count(),
    )
        .prop_map(|(pull_requests, reviews, manual_events)| WorkstreamStats {
            pull_requests,
            reviews,
            manual_events,
        })
}

/// Strategy for generating Workstream values
pub fn strategy_workstream() -> impl Strategy<Value = Workstream> {
    (
        "[a-zA-Z0-9_ ]{5,100}",
        proptest::option::of("[a-zA-Z0-9_ ]{10,500}"),
        proptest::collection::vec("[a-z]{1,20}", 0..5),
        strategy_workstream_stats(),
        proptest::collection::vec(strategy_event_envelope(), 0..20),
        proptest::collection::vec(strategy_event_envelope(), 0..10),
    )
        .prop_map(
            |(title, summary, tags, stats, mut events, mut receipts)| {
                // Ensure all receipt IDs are in events list
                let receipt_ids: Vec<_> = receipts.iter().map(|e| e.id.clone()).collect();
                for receipt in &receipts {
                    if !events.iter().any(|e| e.id == receipt.id) {
                        events.push(receipt.clone());
                    }
                }

                Workstream {
                    id: WorkstreamId::from_parts(["ws", &title.to_lowercase().replace(" ", "-")]),
                    title,
                    summary,
                    tags,
                    stats,
                    events: events.iter().map(|e| e.id.clone()).collect(),
                    receipts: receipt_ids,
                }
            },
        )
}

/// Strategy for generating WorkstreamFile values
pub fn strategy_workstreams_file() -> impl Strategy<Value = WorkstreamFile> {
    (
        proptest::collection::vec(strategy_workstream(), 0..10),
        1u32..10u32,
    )
        .prop_map(|(workstreams, version)| WorkstreamFile {
            workstreams,
            version,
            generated_at: Utc::now(),
        })
}

// ============================================================================
// ID Strategies
// ============================================================================

/// Strategy for generating EventId parts
pub fn strategy_event_id_parts() -> impl Strategy<Value = Vec<String>> {
    proptest::collection::vec("[a-zA-Z0-9_-]{1,50}", 1..5)
}

/// Strategy for generating WorkstreamId parts
pub fn strategy_workstream_id_parts() -> impl Strategy<Value = Vec<String>> {
    proptest::collection::vec("[a-zA-Z0-9_-]{1,50}", 1..3)
}

// ============================================================================
// Cache Strategies
// ============================================================================

/// Strategy for generating cache keys
pub fn strategy_cache_key() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_/-]{5,200}"
}

/// Strategy for generating API URLs
pub fn strategy_api_url() -> impl Strategy<Value = String> {
    "https://api.[a-z0-9.-]{5,20}.com/[a-z0-9_/-]{5,100}"
}

/// Strategy for generating TTL durations (in seconds)
pub fn strategy_ttl_duration() -> impl Strategy<Value = std::time::Duration> {
    0u64..86400u64.prop_map(|secs| std::time::Duration::from_secs(secs))
}

/// Strategy for generating cache entries
pub fn strategy_cache_entry() -> impl Strategy<Value = (String, serde_json::Value, std::time::Duration)> {
    (
        strategy_cache_key(),
        "[a-zA-Z0-9_ ]{1,100}".prop_map(|s| serde_json::Value::String(s)),
        strategy_ttl_duration(),
    )
}
