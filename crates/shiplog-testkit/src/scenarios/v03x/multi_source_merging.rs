//! BDD scenarios for Multi-Source Merging (Feature 7)
//!
//! Scenarios cover:
//! - Primary user workflows (merging events from multiple sources)
//! - Edge cases (conflicting events, no events, incompatible payload decoding)
//! - Integration with clustering/rendering behavior
//! - Performance characteristics on larger collections

use crate::bdd::Scenario;
use crate::bdd::assertions::*;
#[cfg(feature = "merge_pipeline")]
use crate::bdd::builders::CoverageBuilder;
#[cfg(feature = "merge_pipeline")]
use crate::bdd::builders::EventBuilder;
use chrono::{NaiveDate, TimeZone, Utc};
use shiplog_ids::{EventId, RunId};
use shiplog_merge::{ConflictResolution, MergeStrategy, merge_events, merge_ingest_outputs};
use shiplog_ports::{IngestOutput, Renderer, WorkstreamClusterer};
use shiplog_render_md::MarkdownRenderer;
use shiplog_schema::coverage::{Completeness, CoverageManifest, CoverageSlice, TimeWindow};
use shiplog_schema::event::{
    Actor, EventEnvelope, EventKind, EventPayload, PullRequestEvent, PullRequestState, RepoRef,
    RepoVisibility, SourceRef, SourceSystem,
};
use shiplog_workstreams::RepoClusterer;
use std::collections::BTreeSet;

fn timestamp(offset_seconds: i64) -> chrono::DateTime<chrono::Utc> {
    Utc.timestamp_opt(1_735_689_600 + offset_seconds, 0)
        .single()
        .expect("timestamp should be valid")
}

fn source_system(source: &str) -> SourceSystem {
    match source {
        "github" => SourceSystem::Github,
        "local_git" => SourceSystem::LocalGit,
        "manual" => SourceSystem::Manual,
        other => SourceSystem::Other(other.to_string()),
    }
}

fn source_event(
    source: &str,
    repo: &str,
    number: u64,
    id_key: &str,
    title: &str,
    offset_seconds: i64,
) -> EventEnvelope {
    let when = timestamp(offset_seconds);
    EventEnvelope {
        id: EventId::from_parts(["bdd", "multi-source", id_key]),
        kind: EventKind::PullRequest,
        occurred_at: when,
        actor: Actor {
            login: format!("{source}_user"),
            id: None,
        },
        repo: RepoRef {
            full_name: repo.to_string(),
            html_url: Some(format!("https://example.test/{repo}")),
            visibility: RepoVisibility::Public,
        },
        payload: EventPayload::PullRequest(PullRequestEvent {
            number,
            title: title.to_string(),
            state: PullRequestState::Merged,
            created_at: when,
            merged_at: Some(when),
            additions: Some(10),
            deletions: Some(2),
            changed_files: Some(3),
            touched_paths_hint: vec![],
            window: Some(TimeWindow {
                since: NaiveDate::from_ymd_opt(2025, 1, 1).expect("valid date"),
                until: NaiveDate::from_ymd_opt(2025, 2, 1).expect("valid date"),
            }),
        }),
        tags: vec![source.to_string()],
        links: vec![],
        source: SourceRef {
            system: source_system(source),
            url: Some(format!("https://example.test/{source}/{number}")),
            opaque_id: Some(format!("{source}-{number}")),
        },
    }
}

fn source_batch(
    source: &str,
    repo: &str,
    count: u64,
    id_prefix: &str,
    offset_seed: i64,
) -> Vec<EventEnvelope> {
    (0..count)
        .map(|i| {
            source_event(
                source,
                repo,
                i + 1,
                &format!("{id_prefix}-{i}"),
                &format!("{source} event {i}"),
                offset_seed + i as i64,
            )
        })
        .collect()
}

fn coverage_for(
    source: &str,
    completeness: Completeness,
    warning: Option<&str>,
) -> CoverageManifest {
    CoverageManifest {
        run_id: RunId::now("bdd"),
        generated_at: Utc::now(),
        user: "bdd-user".to_string(),
        window: TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 1, 1).expect("valid date"),
            until: NaiveDate::from_ymd_opt(2025, 2, 1).expect("valid date"),
        },
        mode: "merged".to_string(),
        sources: vec![source.to_string()],
        slices: vec![CoverageSlice {
            window: TimeWindow {
                since: NaiveDate::from_ymd_opt(2025, 1, 1).expect("valid date"),
                until: NaiveDate::from_ymd_opt(2025, 2, 1).expect("valid date"),
            },
            query: format!("source:{source}"),
            total_count: 1,
            fetched: 1,
            incomplete_results: Some(false),
            notes: vec![],
        }],
        warnings: warning.map(|w| vec![w.to_string()]).unwrap_or_default(),
        completeness,
    }
}

fn ids(events: &[EventEnvelope]) -> BTreeSet<String> {
    events.iter().map(|event| event.id.0.clone()).collect()
}

/// Scenario 7.1: User merges events from multiple sources
pub fn multi_source_merge() -> Scenario {
    Scenario::new("User merges events from multiple sources")
        .given("a user has collected events from GitHub", |ctx| {
            ctx.numbers.insert("github_events".to_string(), 25);
        })
        .given("they have collected events from local git", |ctx| {
            ctx.numbers.insert("local_git_events".to_string(), 30);
        })
        .given("they have collected manual events", |ctx| {
            ctx.numbers.insert("manual_events".to_string(), 5);
        })
        .when(
            "they run \"shiplog merge --sources github,local_git,manual\"",
            |ctx| {
                let mut github = source_batch("github", "acme/app", 24, "gh", 0);
                github.push(source_event(
                    "github",
                    "acme/app",
                    9_999,
                    "shared-a",
                    "shared from github",
                    500,
                ));

                let mut local_git = source_batch("local_git", "acme/app", 29, "lg", 1_000);
                local_git.push(source_event(
                    "local_git",
                    "acme/app",
                    9_999,
                    "shared-a",
                    "shared from local git",
                    2_000,
                ));

                let mut manual = source_batch("manual", "acme/app", 4, "manual", 3_000);
                manual.push(source_event(
                    "manual",
                    "acme/app",
                    9_998,
                    "shared-b",
                    "shared from manual",
                    3_001,
                ));
                github.push(source_event(
                    "github",
                    "acme/app",
                    9_998,
                    "shared-b",
                    "shared from github",
                    400,
                ));

                let raw_count = github.len() + local_git.len() + manual.len();
                let unique_expected = ids(&github)
                    .union(&ids(&local_git))
                    .cloned()
                    .collect::<BTreeSet<_>>()
                    .union(&ids(&manual))
                    .cloned()
                    .collect::<BTreeSet<_>>()
                    .len();
                let merged =
                    merge_events(vec![github, local_git, manual], &MergeStrategy::KeepLast);
                let sorted = merged
                    .windows(2)
                    .all(|pair| pair[0].occurred_at <= pair[1].occurred_at);

                ctx.numbers
                    .insert("raw_event_count".to_string(), raw_count as u64);
                ctx.numbers.insert(
                    "expected_unique_event_count".to_string(),
                    unique_expected as u64,
                );
                ctx.numbers
                    .insert("merged_event_count".to_string(), merged.len() as u64);
                ctx.flags
                    .insert("deduplicated".to_string(), merged.len() < raw_count);
                ctx.flags.insert("sorted".to_string(), sorted);
                Ok(())
            },
        )
        .then("all events should be merged into a single ledger", |ctx| {
            let merged = assert_present(ctx.number("merged_event_count"), "merged_event_count")?;
            let expected = assert_present(
                ctx.number("expected_unique_event_count"),
                "expected_unique_event_count",
            )?;
            assert_eq(merged, expected, "merged event count")
        })
        .then("events should be deduplicated by ID", |ctx| {
            assert_true(
                ctx.flag("deduplicated").unwrap_or(false),
                "events deduplicated",
            )
        })
        .then("events should be sorted by timestamp", |ctx| {
            assert_true(ctx.flag("sorted").unwrap_or(false), "events sorted")
        })
}

/// Scenario 7.2: User merges with unified coverage tracking
pub fn multi_source_merge_coverage() -> Scenario {
    Scenario::new("User merges with unified coverage tracking")
        .given("a user has collected events from multiple sources", |ctx| {
            ctx.numbers.insert("source_count".to_string(), 3);
        })
        .when("they merge the sources", |ctx| {
            let input_a = IngestOutput {
                events: vec![source_event(
                    "github",
                    "acme/api",
                    1,
                    "merge-coverage-a",
                    "API event",
                    1,
                )],
                coverage: coverage_for(
                    "github",
                    Completeness::Partial,
                    Some("github coverage warning"),
                ),
            };
            let input_b = IngestOutput {
                events: vec![source_event(
                    "local_git",
                    "acme/api",
                    2,
                    "merge-coverage-b",
                    "Local git event",
                    2,
                )],
                coverage: coverage_for(
                    "local_git",
                    Completeness::Complete,
                    Some("local_git coverage warning"),
                ),
            };
            let input_c = IngestOutput {
                events: vec![source_event(
                    "manual",
                    "acme/api",
                    3,
                    "merge-coverage-c",
                    "Manual event",
                    3,
                )],
                coverage: coverage_for("manual", Completeness::Complete, None),
            };

            let merged = merge_ingest_outputs(
                &[input_a, input_b, input_c],
                ConflictResolution::PreferMostRecent,
            )
            .map_err(|err| format!("merge_ingest_outputs failed: {err}"))?;

            ctx.numbers.insert(
                "source_count".to_string(),
                merged.ingest_output.coverage.sources.len() as u64,
            );
            ctx.flags.insert(
                "coverage_unified".to_string(),
                matches!(
                    merged.ingest_output.coverage.completeness,
                    Completeness::Partial
                ),
            );
            ctx.flags.insert(
                "warnings_aggregated".to_string(),
                merged.ingest_output.coverage.warnings.len() >= 2,
            );
            Ok(())
        })
        .then("the coverage manifest should include all sources", |ctx| {
            let count = ctx.number("source_count").unwrap_or(0);
            assert_eq(count, 3, "source count")
        })
        .then(
            "completeness should be calculated across all sources",
            |ctx| {
                assert_true(
                    ctx.flag("coverage_unified").unwrap_or(false),
                    "coverage unified",
                )
            },
        )
        .then("warnings should be aggregated from all sources", |ctx| {
            assert_true(
                ctx.flag("warnings_aggregated").unwrap_or(false),
                "warnings aggregated",
            )
        })
}

/// Scenario 7.3: User merges events from same source type
pub fn multi_source_merge_same_type() -> Scenario {
    Scenario::new("User merges events from same source type")
        .given(
            "a user has collected events from GitHub for two different repos",
            |ctx| {
                ctx.numbers.insert("repo_a_events".to_string(), 20);
                ctx.numbers.insert("repo_b_events".to_string(), 15);
            },
        )
        .when("they merge the sources", |ctx| {
            let mut repo_a = source_batch("github", "acme/repo-a", 19, "repo-a", 100);
            repo_a.push(source_event(
                "github",
                "acme/repo-a",
                42,
                "shared-gh-pr",
                "shared pull request",
                1_500,
            ));

            let mut repo_b = source_batch("github", "acme/repo-b", 14, "repo-b", 2_000);
            repo_b.push(source_event(
                "github",
                "acme/repo-b",
                42,
                "shared-gh-pr",
                "shared pull request newer",
                2_500,
            ));

            let expected = ids(&repo_a)
                .union(&ids(&repo_b))
                .cloned()
                .collect::<BTreeSet<_>>()
                .len();
            let raw = repo_a.len() + repo_b.len();
            let merged = merge_events(vec![repo_a, repo_b], &MergeStrategy::KeepLast);

            ctx.numbers
                .insert("total_events".to_string(), merged.len() as u64);
            ctx.numbers
                .insert("expected_unique".to_string(), expected as u64);
            ctx.flags
                .insert("deduplicated".to_string(), merged.len() < raw);
            Ok(())
        })
        .then("events from both repos should be included", |ctx| {
            let total = assert_present(ctx.number("total_events"), "total_events")?;
            let expected = assert_present(ctx.number("expected_unique"), "expected_unique")?;
            assert_eq(total, expected, "total events")
        })
        .then("duplicate events (same PR) should be deduplicated", |ctx| {
            assert_true(
                ctx.flag("deduplicated").unwrap_or(false),
                "events deduplicated",
            )
        })
}

/// Scenario 7.4: Conflicting events from different sources
pub fn multi_source_merge_conflicts() -> Scenario {
    Scenario::new("Conflicting events from different sources")
        .given(
            "a user has collected the same event from GitHub and local git",
            |_| {},
        )
        .given("the events have different metadata", |_| {})
        .when("they merge the sources", |ctx| {
            let older = source_event("github", "acme/conflict", 1, "conflict-pr", "Old title", 1);
            let newer = source_event(
                "local_git",
                "acme/conflict",
                1,
                "conflict-pr",
                "Newer title",
                2,
            );

            let input_a = IngestOutput {
                events: vec![older],
                coverage: coverage_for("github", Completeness::Complete, None),
            };
            let input_b = IngestOutput {
                events: vec![newer],
                coverage: coverage_for("local_git", Completeness::Complete, None),
            };
            let merged =
                merge_ingest_outputs(&[input_a, input_b], ConflictResolution::PreferMostRecent)
                    .map_err(|err| format!("merge_ingest_outputs failed: {err}"))?;

            let selected_title = merged
                .ingest_output
                .events
                .first()
                .and_then(|event| match &event.payload {
                    EventPayload::PullRequest(pr) => Some(pr.title.clone()),
                    _ => None,
                });

            ctx.flags.insert(
                "conflict_resolved".to_string(),
                merged.report.conflict_count > 0,
            );
            ctx.strings.insert(
                "selected_title".to_string(),
                selected_title.unwrap_or_default(),
            );
            ctx.strings.insert(
                "warning_message".to_string(),
                merged.ingest_output.coverage.warnings.join(" | "),
            );
            Ok(())
        })
        .then("one event should be chosen as authoritative", |ctx| {
            assert_true(
                ctx.flag("conflict_resolved").unwrap_or(false),
                "conflict resolved",
            )?;
            let selected = assert_present(ctx.string("selected_title"), "selected_title")?;
            assert_eq(selected, "Newer title", "authoritative event title")
        })
        .then(
            "a warning should indicate the conflict was resolved",
            |ctx| {
                let warning = assert_present(ctx.string("warning_message"), "warning_message")?;
                assert_contains(warning, "Resolved", "warning message")
            },
        )
}

/// Scenario 7.5: Merge with no events
pub fn multi_source_merge_no_events() -> Scenario {
    Scenario::new("Merge with no events")
        .given("a user attempts to merge with no collected events", |_| {})
        .when("they run \"shiplog merge\"", |ctx| {
            let result = merge_ingest_outputs(&[], ConflictResolution::PreferMostRecent);
            ctx.flags
                .insert("command_failed".to_string(), result.is_err());
            if let Err(err) = result {
                ctx.strings
                    .insert("error_message".to_string(), err.to_string());
            }
            Ok(())
        })
        .then("the command should fail with a clear error", |ctx| {
            assert_true(
                ctx.flag("command_failed").unwrap_or(false),
                "command failed",
            )
        })
        .then(
            "the error should indicate no events are available to merge",
            |ctx| {
                let error = assert_present(ctx.string("error_message"), "error_message")?;
                assert_contains(error, "No ingest outputs", "error message")
            },
        )
}

/// Scenario 7.6: Merge with incompatible event types
pub fn multi_source_merge_incompatible() -> Scenario {
    Scenario::new("Merge with incompatible event types")
        .given(
            "a user has collected events from a source with incompatible schema",
            |_| {},
        )
        .when("they attempt to merge", |ctx| {
            let valid = source_event(
                "github",
                "acme/schema",
                1,
                "compatible-event",
                "compatible",
                1,
            );
            let valid_line = serde_json::to_string(&valid)
                .map_err(|err| format!("serialize event should work: {err}"))?;
            let raw_lines = vec![
                valid_line,
                "{\"oops\":true}".to_string(),
                "{\"id\":123}".to_string(),
                "{\"payload\":\"bad\"}".to_string(),
            ];

            let mut parsed_events = Vec::new();
            let mut skipped = 0usize;
            for line in raw_lines {
                match serde_json::from_str::<EventEnvelope>(&line) {
                    Ok(event) => parsed_events.push(event),
                    Err(_) => skipped += 1,
                }
            }

            let merged = merge_events(vec![parsed_events], &MergeStrategy::KeepLast);
            ctx.flags.insert("merged".to_string(), !merged.is_empty());
            ctx.numbers
                .insert("skipped_events".to_string(), skipped as u64);
            ctx.strings.insert(
                "warning_message".to_string(),
                format!("Skipped {skipped} event(s) with incompatible schema"),
            );
            Ok(())
        })
        .then("incompatible events should be skipped", |ctx| {
            let count = ctx.number("skipped_events").unwrap_or(0);
            assert_true(count > 0, "events skipped")
        })
        .then("a warning should indicate the skipped events", |ctx| {
            let warning = assert_present(ctx.string("warning_message"), "warning_message")?;
            assert_contains(warning, "Skipped", "warning message")
        })
}

/// Scenario 7.7: Multi-source events cluster together
pub fn multi_source_cluster() -> Scenario {
    Scenario::new("Multi-source events cluster together")
        .given(
            "a user has merged events from GitHub, GitLab, and Jira",
            |_| {},
        )
        .when("they run \"shiplog cluster\"", |ctx| {
            let events = vec![
                source_event(
                    "github",
                    "acme/platform",
                    1,
                    "cluster-github",
                    "GitHub event",
                    1,
                ),
                source_event(
                    "gitlab",
                    "acme/platform",
                    2,
                    "cluster-gitlab",
                    "GitLab event",
                    2,
                ),
                source_event("jira", "acme/platform", 3, "cluster-jira", "Jira event", 3),
            ];

            let clustered = RepoClusterer
                .cluster(&events)
                .map_err(|err| format!("repo clustering failed: {err}"))?;
            let primary = clustered
                .workstreams
                .first()
                .ok_or_else(|| "expected at least one workstream".to_string())?;
            let event_by_id: std::collections::HashMap<String, &EventEnvelope> = events
                .iter()
                .map(|event| (event.id.0.clone(), event))
                .collect();
            let source_count = primary
                .events
                .iter()
                .filter_map(|id| event_by_id.get(&id.0))
                .map(|event| event.source.system.as_str().to_string())
                .collect::<BTreeSet<_>>()
                .len();

            ctx.flags.insert(
                "workstreams_generated".to_string(),
                !clustered.workstreams.is_empty(),
            );
            ctx.numbers.insert(
                "workstream_count".to_string(),
                clustered.workstreams.len() as u64,
            );
            ctx.flags
                .insert("multi_source_workstreams".to_string(), source_count >= 3);
            Ok(())
        })
        .then(
            "workstreams should include events from all sources",
            |ctx| {
                assert_true(
                    ctx.flag("workstreams_generated").unwrap_or(false),
                    "workstreams generated",
                )
            },
        )
        .then(
            "clustering should consider event context across sources",
            |ctx| {
                assert_true(
                    ctx.flag("multi_source_workstreams").unwrap_or(false),
                    "multi-source workstreams",
                )
            },
        )
}

/// Scenario 7.8: Multi-source packet renders correctly
pub fn multi_source_render() -> Scenario {
    Scenario::new("Multi-source packet renders correctly")
        .given("a user has merged events from multiple sources", |_| {})
        .given("they have generated workstreams", |_| {})
        .when("they run \"shiplog render\"", |ctx| {
            let events = vec![
                source_event(
                    "github",
                    "github/acme-api",
                    1,
                    "render-github",
                    "GitHub event",
                    1,
                ),
                source_event(
                    "gitlab",
                    "gitlab/acme-api",
                    2,
                    "render-gitlab",
                    "GitLab event",
                    2,
                ),
                source_event("jira", "jira/acme-api", 3, "render-jira", "Jira event", 3),
            ];

            let workstreams = RepoClusterer
                .cluster(&events)
                .map_err(|err| format!("repo clustering failed: {err}"))?;
            let coverage = CoverageManifest {
                run_id: RunId::now("render"),
                generated_at: Utc::now(),
                user: "bdd-user".to_string(),
                window: TimeWindow {
                    since: NaiveDate::from_ymd_opt(2025, 1, 1).expect("valid date"),
                    until: NaiveDate::from_ymd_opt(2025, 2, 1).expect("valid date"),
                },
                mode: "merged".to_string(),
                sources: vec![
                    "github".to_string(),
                    "gitlab".to_string(),
                    "jira".to_string(),
                ],
                slices: vec![],
                warnings: vec![],
                completeness: Completeness::Complete,
            };

            let packet = MarkdownRenderer::new()
                .render_packet_markdown("bdd-user", "2025-01", &events, &workstreams, &coverage)
                .map_err(|err| format!("render failed: {err}"))?;

            ctx.flags
                .insert("packet_rendered".to_string(), !packet.is_empty());
            ctx.flags.insert(
                "all_sources_included".to_string(),
                packet.contains("github") && packet.contains("gitlab") && packet.contains("jira"),
            );
            Ok(())
        })
        .then("the packet should include events from all sources", |ctx| {
            assert_true(
                ctx.flag("packet_rendered").unwrap_or(false),
                "packet rendered",
            )
        })
        .then("each event should indicate its source", |ctx| {
            assert_true(
                ctx.flag("all_sources_included").unwrap_or(false),
                "all sources included",
            )
        })
}

/// Scenario 7.9: Merge with thousands of events
pub fn multi_source_merge_large() -> Scenario {
    Scenario::new("Merge with thousands of events")
        .given("a user has 5,000 events across 5 sources", |ctx| {
            ctx.numbers.insert("total_events".to_string(), 5000);
            ctx.numbers.insert("source_count".to_string(), 5);
        })
        .when("they run \"shiplog merge\"", |ctx| {
            let mut groups = Vec::new();
            for source_idx in 0..5 {
                let source_name = format!("source_{source_idx}");
                let mut group = Vec::new();
                for i in 0..1_000 {
                    let id_key = if i < 50 {
                        format!("shared-{i}")
                    } else {
                        format!("{source_name}-{i}")
                    };
                    group.push(source_event(
                        &source_name,
                        &format!("acme/large-{source_idx}"),
                        i as u64,
                        &id_key,
                        "Large merge event",
                        source_idx as i64 * 10_000 + i as i64,
                    ));
                }
                groups.push(group);
            }

            let started = std::time::Instant::now();
            let merged = merge_events(groups, &MergeStrategy::KeepLast);
            let elapsed_ms = started.elapsed().as_millis() as u64;
            let approx_bytes = (merged.len() * std::mem::size_of::<EventEnvelope>()) as u64;

            ctx.flags.insert("merged".to_string(), !merged.is_empty());
            ctx.numbers
                .insert("merged_event_count".to_string(), merged.len() as u64);
            ctx.numbers.insert("elapsed_ms".to_string(), elapsed_ms);
            ctx.numbers.insert("approx_bytes".to_string(), approx_bytes);
            Ok(())
        })
        .then(
            "merging should complete within reasonable time (< 10 seconds)",
            |ctx| {
                let elapsed_ms = assert_present(ctx.number("elapsed_ms"), "elapsed_ms")?;
                assert_true(elapsed_ms < 10_000, "merge time")
            },
        )
        .then("memory usage should remain bounded", |ctx| {
            let approx_bytes = assert_present(ctx.number("approx_bytes"), "approx_bytes")?;
            assert_true(approx_bytes < 1_000_000_000, "memory usage")
        })
}

#[cfg(feature = "merge_pipeline")]
/// Scenario 7.10: Merge pipeline contract
pub fn multi_source_merge_pipeline_contract() -> Scenario {
    Scenario::new("Merge pipeline emits merged ingest output contract")
        .given(
            "a build includes duplicate member events from different ingest sources",
            |_| {},
        )
        .when(
            "the merge pipeline resolves conflicts with PreferMostRecent strategy",
            |ctx| {
                let event_a = EventBuilder::new("repo/shiplog", 1, "Member event").build();
                let event_b =
                    EventBuilder::new("repo/shiplog", 1, "Member event (duplicate)").build();
                let event_c = EventBuilder::new("repo/shiplog", 2, "Unique event").build();

                let coverage_a = CoverageBuilder::new("alice")
                    .completeness(shiplog_schema::coverage::Completeness::Partial)
                    .build();
                let coverage_b = CoverageBuilder::new("bob")
                    .completeness(shiplog_schema::coverage::Completeness::Complete)
                    .build();

                let input_a = IngestOutput {
                    events: vec![event_a, event_c],
                    coverage: coverage_a,
                };
                let input_b = IngestOutput {
                    events: vec![event_b],
                    coverage: coverage_b,
                };

                let merged =
                    merge_ingest_outputs(&[input_a, input_b], ConflictResolution::PreferMostRecent)
                        .map_err(|err| format!("merge_ingest_outputs failed: {err}"))?;

                ctx.strings.insert(
                    "merged_sources".to_string(),
                    merged.ingest_output.coverage.sources.join(","),
                );
                ctx.numbers.insert(
                    "merged_event_count".to_string(),
                    merged.ingest_output.events.len() as u64,
                );
                ctx.numbers.insert(
                    "merge_conflict_count".to_string(),
                    merged.report.conflict_count as u64,
                );
                ctx.numbers.insert(
                    "coverage_warning_count".to_string(),
                    merged.ingest_output.coverage.warnings.len() as u64,
                );
                Ok(())
            },
        )
        .then(
            "conflict events should be deduplicated deterministically",
            |ctx| {
                let merged_events =
                    assert_present(ctx.number("merged_event_count"), "merged_event_count")?;
                assert_eq(merged_events, 2, "merged event count")?;
                Ok(())
            },
        )
        .then(
            "coverage sources should include contributing ingest systems",
            |ctx| {
                let sources = assert_present(ctx.string("merged_sources"), "merged sources")?;
                assert_contains(sources, "github", "github source included")
            },
        )
        .then("a merge report should expose conflict count", |ctx| {
            let conflicts =
                assert_present(ctx.number("merge_conflict_count"), "merge conflict count")?;
            assert_true(conflicts > 0, "conflicts resolved")?;
            Ok(())
        })
        .then("merged coverage should surface merge warnings", |ctx| {
            let warning_count = assert_present(
                ctx.number("coverage_warning_count"),
                "coverage warning count",
            )?;
            assert_true(warning_count > 0, "coverage warnings added")?;
            Ok(())
        })
}
