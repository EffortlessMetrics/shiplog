//! Proptest strategies for shiplog property-based testing
//!
//! This module provides reusable proptest strategies for generating valid test data
//! across all shiplog crates.

use chrono::{Datelike, NaiveDate, TimeZone, Utc};
use proptest::prelude::*;
use shiplog_ids::{EventId, RunId, WorkstreamId};
use shiplog_schema::coverage::{Completeness, CoverageManifest, CoverageSlice, TimeWindow};
use shiplog_schema::event::*;
use shiplog_schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};

// ============================================================================
// Base Strategies
// ============================================================================

/// Strategy for generating valid NaiveDate values
pub fn strategy_naive_date() -> impl Strategy<Value = NaiveDate> {
    // Generate dates from 2020-01-01 to 2030-12-31
    prop::num::i32::ANY.prop_map(|days| {
        NaiveDate::from_ymd_opt(2020, 1, 1)
            .unwrap()
            .checked_add_days(chrono::Days::new((days.abs() % 4000) as u64))
            .unwrap()
    })
}

/// Strategy for generating valid DateTime<Utc> values
pub fn strategy_datetime_utc() -> impl Strategy<Value = chrono::DateTime<Utc>> {
    strategy_naive_date().prop_map(|date| {
        Utc.with_ymd_and_hms(date.year(), date.month(), date.day(), 0, 0, 0)
            .unwrap()
    })
}

/// Strategy for generating valid date ranges (since < until)
pub fn strategy_date_range() -> impl Strategy<Value = (NaiveDate, NaiveDate)> {
    (strategy_naive_date(), strategy_naive_date())
        .prop_map(|(d1, d2)| if d1 < d2 { (d1, d2) } else { (d2, d1) })
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
        Just(SourceSystem::JsonImport),
        Just(SourceSystem::LocalGit),
        Just(SourceSystem::Manual),
        Just(SourceSystem::Unknown),
        "[a-z]{3,12}".prop_map(SourceSystem::Other),
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
        Just(PullRequestState::Unknown),
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
        Just(Completeness::Unknown),
    ]
}

fn strategy_manual_event_type() -> impl Strategy<Value = ManualEventType> {
    prop_oneof![
        Just(ManualEventType::Note),
        Just(ManualEventType::Incident),
        Just(ManualEventType::Design),
        Just(ManualEventType::Mentoring),
        Just(ManualEventType::Launch),
        Just(ManualEventType::Migration),
        Just(ManualEventType::Review),
        Just(ManualEventType::Other),
    ]
}

// ============================================================================
// Event Strategies
// ============================================================================

/// Strategy for generating Actor values
pub fn strategy_actor() -> impl Strategy<Value = Actor> {
    (
        "[a-zA-Z0-9_-]{1,50}",
        proptest::option::of(1u64..1_000_000u64),
    )
        .prop_map(|(login, id)| Actor { login, id })
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
    ("[a-z]{1,20}", strategy_url()).prop_map(|(label, url)| Link { label, url })
}

/// Strategy for generating SourceRef values
pub fn strategy_source_ref() -> impl Strategy<Value = SourceRef> {
    (
        strategy_source_system(),
        proptest::option::of(strategy_url()),
        proptest::option::of("[a-zA-Z0-9_-]{1,64}"),
    )
        .prop_map(|(system, url, opaque_id)| SourceRef {
            system,
            url,
            opaque_id,
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
        proptest::option::of(0u64..10000u64),
        proptest::option::of(0u64..10000u64),
        proptest::option::of(0u64..1000u64),
        proptest::collection::vec("[a-zA-Z0-9_./-]{1,100}", 0..10),
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
        prop_oneof![
            Just("approved".to_string()),
            Just("changes_requested".to_string()),
            Just("commented".to_string()),
            "[a-z_]{3,20}".prop_map(|s| s.to_lowercase()),
        ],
        strategy_datetime_utc(),
        proptest::option::of(strategy_time_window()),
    )
        .prop_map(
            |(pull_number, pull_title, state, submitted_at, window)| ReviewEvent {
                pull_number,
                pull_title,
                submitted_at,
                state,
                window,
            },
        )
}

/// Strategy for generating ManualEvent values
pub fn strategy_manual_payload() -> impl Strategy<Value = ManualEvent> {
    (
        strategy_manual_event_type(),
        strategy_non_empty_string(),
        proptest::option::of("[a-zA-Z0-9_ ,.:-]{10,500}"),
        proptest::option::of(strategy_date_range()),
        proptest::option::of("[a-zA-Z0-9_ ,.:-]{10,200}"),
    )
        .prop_map(
            |(event_type, title, description, date_range, impact)| ManualEvent {
                event_type,
                title,
                description,
                started_at: date_range.map(|(since, _)| since),
                ended_at: date_range.map(|(_, until)| until),
                impact,
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
        strategy_event_payload(),
        strategy_actor(),
        strategy_repo_ref(),
        strategy_source_ref(),
        proptest::collection::vec(strategy_link(), 0..5),
        proptest::collection::vec("[a-z]{1,20}", 0..5),
        any::<u64>(),
    )
        .prop_map(|(payload, actor, repo, source, links, tags, nonce)| {
            let (kind, id) = match &payload {
                EventPayload::PullRequest(pr) => (
                    EventKind::PullRequest,
                    EventId::from_parts([
                        "github",
                        "pr",
                        &repo.full_name,
                        &pr.number.to_string(),
                        &nonce.to_string(),
                    ]),
                ),
                EventPayload::Review(r) => (
                    EventKind::Review,
                    EventId::from_parts([
                        "github",
                        "review",
                        &repo.full_name,
                        &r.pull_number.to_string(),
                        &nonce.to_string(),
                    ]),
                ),
                EventPayload::Manual(manual) => (
                    EventKind::Manual,
                    EventId::from_parts([
                        "manual",
                        &repo.full_name,
                        &manual.title,
                        &nonce.to_string(),
                    ]),
                ),
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
        })
}

/// Strategy for generating a vector of EventEnvelope values
pub fn strategy_event_vec(max_size: usize) -> impl Strategy<Value = Vec<EventEnvelope>> {
    proptest::collection::vec(strategy_event_envelope(), 0..max_size)
}

// ============================================================================
// Coverage Strategies
// ============================================================================

/// Strategy for generating CoverageSlice values
pub fn strategy_coverage_slice() -> impl Strategy<Value = CoverageSlice> {
    (
        strategy_time_window(),
        "[a-zA-Z0-9_:=, /.-]{5,120}",
        0u64..5000u64,
        0u64..5000u64,
        proptest::collection::vec("[a-zA-Z0-9_ ,.:-]{5,120}", 0..5),
    )
        .prop_map(|(window, query, a, b, notes)| {
            let total_count = a.max(b);
            let fetched = a.min(b);
            let incomplete_results = Some(fetched < total_count);

            CoverageSlice {
                window,
                query,
                total_count,
                fetched,
                incomplete_results,
                notes,
            }
        })
}

/// Strategy for generating CoverageManifest values
pub fn strategy_coverage_manifest() -> impl Strategy<Value = CoverageManifest> {
    (
        "[a-zA-Z0-9_-]{1,50}",
        strategy_date_range(),
        proptest::collection::vec(strategy_coverage_slice(), 0..10),
        proptest::collection::vec("[a-zA-Z0-9_ ,.:-]{5,100}", 0..5),
        strategy_completeness(),
    )
        .prop_map(
            |(user, (since, until), slices, warnings, completeness)| CoverageManifest {
                run_id: RunId::now("test"),
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
        proptest::option::of("[a-zA-Z0-9_ ,.:-]{10,500}"),
        proptest::collection::vec("[a-z]{1,20}", 0..5),
        proptest::collection::vec(strategy_event_envelope(), 0..20),
    )
        .prop_map(|(title, summary, tags, events)| {
            let mut stats = WorkstreamStats::zero();
            for event in &events {
                match event.kind {
                    EventKind::PullRequest => stats.pull_requests += 1,
                    EventKind::Review => stats.reviews += 1,
                    EventKind::Manual => stats.manual_events += 1,
                }
            }

            let event_ids: Vec<_> = events.iter().map(|e| e.id.clone()).collect();
            let receipts = event_ids.iter().take(10).cloned().collect();

            Workstream {
                id: WorkstreamId::from_parts(["ws", &title.to_lowercase().replace(' ', "-")]),
                title,
                summary,
                tags,
                stats,
                events: event_ids,
                receipts,
            }
        })
}

/// Strategy for generating WorkstreamsFile values
pub fn strategy_workstreams_file() -> impl Strategy<Value = WorkstreamsFile> {
    (
        proptest::collection::vec(strategy_workstream(), 0..10),
        1u32..10u32,
    )
        .prop_map(|(workstreams, version)| WorkstreamsFile {
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
    (0u64..86400u64).prop_map(std::time::Duration::from_secs)
}

/// Strategy for generating cache entries
pub fn strategy_cache_entry()
-> impl Strategy<Value = (String, serde_json::Value, std::time::Duration)> {
    (
        strategy_cache_key(),
        "[a-zA-Z0-9_ ]{1,100}".prop_map(serde_json::Value::String),
        strategy_ttl_duration(),
    )
}
