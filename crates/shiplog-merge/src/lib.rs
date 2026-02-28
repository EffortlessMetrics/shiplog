//! Merging utilities for combining multiple event sources.
//!
//! Provides functions to merge and deduplicate events from multiple sources,
//! handling conflicts and preserving coverage metadata when operating on
//! `IngestOutput` values.

use anyhow::{Result, anyhow};
use chrono::Utc;
use shiplog_ids::EventId;
use shiplog_ids::RunId;
use shiplog_ports::IngestOutput;
use shiplog_schema::coverage::{Completeness, CoverageManifest, CoverageSlice};
use shiplog_schema::event::EventEnvelope;
use std::collections::HashMap;

/// Strategy for handling duplicate events during merge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ConflictResolution {
    /// Keep the first event encountered.
    PreferFirst,
    /// Keep the latest event.
    #[default]
    PreferMostRecent,
    /// Keep the event with more complete data.
    PreferMostComplete,
}

/// Legacy strategy type retained for existing callers.
#[derive(Clone, Debug, Default)]
pub enum MergeStrategy {
    /// Keep the first event encountered.
    KeepFirst,
    /// Keep the last event encountered (by occurred_at).
    #[default]
    KeepLast,
    /// Keep the event with more complete data.
    KeepMostComplete,
}

impl From<ConflictResolution> for MergeStrategy {
    fn from(value: ConflictResolution) -> Self {
        match value {
            ConflictResolution::PreferFirst => Self::KeepFirst,
            ConflictResolution::PreferMostRecent => Self::KeepLast,
            ConflictResolution::PreferMostComplete => Self::KeepMostComplete,
        }
    }
}

/// Metadata from an ingest-output merge.
#[derive(Debug, Clone)]
pub struct MergeReport {
    pub source_count: usize,
    pub input_event_count: usize,
    pub output_event_count: usize,
    pub conflict_count: usize,
    pub skipped_events: usize,
    pub warning_count: usize,
}

/// Merge result that keeps output data and summary metadata together.
#[derive(Debug, Clone)]
pub struct MergeResult {
    pub ingest_output: IngestOutput,
    pub report: MergeReport,
}

/// Merge multiple event lists into one, deduplicating by event ID.
///
/// The strategy determines how to handle conflicts when the same event appears
/// in multiple sources.
pub fn merge_events(
    sources: Vec<Vec<EventEnvelope>>,
    strategy: &MergeStrategy,
) -> Vec<EventEnvelope> {
    let mut events_by_id: HashMap<EventId, EventEnvelope> = HashMap::new();

    for source in sources {
        for event in source {
            match events_by_id.get(&event.id) {
                Some(existing) => {
                    let should_replace = match strategy {
                        MergeStrategy::KeepFirst => false,
                        MergeStrategy::KeepLast => event.occurred_at > existing.occurred_at,
                        MergeStrategy::KeepMostComplete => {
                            completeness_score(&event) > completeness_score(existing)
                        }
                    };
                    if should_replace {
                        events_by_id.insert(event.id.clone(), event);
                    }
                }
                None => {
                    events_by_id.insert(event.id.clone(), event);
                }
            }
        }
    }

    let mut result: Vec<EventEnvelope> = events_by_id.into_values().collect();
    result.sort_by(|a, b| {
        a.occurred_at
            .cmp(&b.occurred_at)
            .then_with(|| a.id.0.cmp(&b.id.0))
    });
    result
}

/// Merge two event lists.
pub fn merge_two(
    left: &[EventEnvelope],
    right: &[EventEnvelope],
    strategy: &MergeStrategy,
) -> Vec<EventEnvelope> {
    merge_events(vec![left.to_vec(), right.to_vec()], strategy)
}

/// Merge complete ingest outputs from multiple sources.
pub fn merge_ingest_outputs(
    ingest_outputs: &[IngestOutput],
    resolution: ConflictResolution,
) -> Result<MergeResult> {
    if ingest_outputs.is_empty() {
        return Err(anyhow!("No ingest outputs to merge"));
    }

    let base_coverage = &ingest_outputs[0].coverage;
    let mut event_groups = Vec::with_capacity(ingest_outputs.len());
    let mut all_sources = Vec::new();
    let mut all_warnings = Vec::new();
    let mut all_slices: Vec<CoverageSlice> = Vec::new();
    let mut input_event_count = 0usize;

    for ingest in ingest_outputs {
        input_event_count += ingest.events.len();
        event_groups.push(ingest.events.clone());
        all_sources.extend(ingest.coverage.sources.clone());
        all_warnings.extend(ingest.coverage.warnings.clone());
        all_slices.extend(ingest.coverage.slices.clone());
    }

    let merged_events = merge_events(event_groups, &resolution.into());
    let mut coverage = CoverageManifest {
        run_id: RunId::now("merge"),
        generated_at: Utc::now(),
        user: base_coverage.user.clone(),
        window: base_coverage.window.clone(),
        mode: "merged".to_string(),
        sources: {
            all_sources.sort();
            all_sources.dedup();
            all_sources
        },
        slices: all_slices,
        warnings: {
            if input_event_count > merged_events.len() {
                let conflicts = input_event_count - merged_events.len();
                all_warnings.push(format!(
                    "Resolved {} conflict(s) during merge using {:?} strategy",
                    conflicts, resolution,
                ));
            }
            all_warnings
        },
        completeness: if ingest_outputs
            .iter()
            .any(|o| o.coverage.completeness == Completeness::Partial)
        {
            Completeness::Partial
        } else {
            Completeness::Complete
        },
    };

    let conflict_count = input_event_count.saturating_sub(merged_events.len());
    coverage.slices.sort_by_key(|slice| slice.window.since);

    let report = MergeReport {
        source_count: coverage.sources.len(),
        input_event_count,
        output_event_count: merged_events.len(),
        conflict_count,
        skipped_events: 0,
        warning_count: coverage.warnings.len(),
    };

    Ok(MergeResult {
        ingest_output: IngestOutput {
            events: merged_events,
            coverage,
        },
        report,
    })
}

/// Merge multiple ingest outputs using the pre-existing engine fallback behavior.
///
/// Kept here to preserve CLI compatibility when the `merge-pipeline` feature
/// is disabled in `shiplog-engine`.
pub fn merge_ingest_outputs_legacy(
    ingest_outputs: &[IngestOutput],
    resolution: ConflictResolution,
) -> Result<IngestOutput> {
    use std::collections::HashMap;

    if ingest_outputs.is_empty() {
        return Err(anyhow!("No ingest outputs to merge"));
    }

    let mut event_map: HashMap<String, Vec<EventEnvelope>> = HashMap::new();
    let mut all_sources: Vec<String> = Vec::new();
    let mut all_warnings: Vec<String> = Vec::new();
    let mut all_slices: Vec<shiplog_schema::coverage::CoverageSlice> = Vec::new();

    let base_output = &ingest_outputs[0];
    let window = base_output.coverage.window.clone();
    let user = base_output.coverage.user.clone();

    for ingest in ingest_outputs {
        for event in &ingest.events {
            let id = event.id.to_string();
            event_map.entry(id).or_default().push(event.clone());
        }

        all_sources.extend(ingest.coverage.sources.clone());
        all_warnings.extend(ingest.coverage.warnings.clone());
        all_slices.extend(ingest.coverage.slices.clone());
    }

    let mut merged_events: Vec<EventEnvelope> = Vec::new();
    let mut conflict_count = 0usize;

    for (_id, events) in event_map {
        if events.len() == 1 {
            merged_events.push(events[0].clone());
        } else {
            conflict_count += 1;
            merged_events.push(resolve_conflict_legacy(&events, resolution));
        }
    }

    merged_events.sort_by(|a, b| {
        a.occurred_at
            .cmp(&b.occurred_at)
            .then_with(|| a.id.0.cmp(&b.id.0))
    });
    all_sources.sort();
    all_sources.dedup();

    let completeness = if ingest_outputs
        .iter()
        .any(|o| o.coverage.completeness == shiplog_schema::coverage::Completeness::Partial)
    {
        shiplog_schema::coverage::Completeness::Partial
    } else {
        shiplog_schema::coverage::Completeness::Complete
    };

    if conflict_count > 0 {
        all_warnings.push(format!(
            "Resolved {} conflict(s) during merge using {:?} strategy",
            conflict_count, resolution,
        ));
    }

    let coverage = shiplog_schema::coverage::CoverageManifest {
        run_id: RunId::now("merge"),
        generated_at: chrono::Utc::now(),
        user,
        window,
        mode: "merged".to_string(),
        sources: all_sources,
        slices: all_slices,
        warnings: all_warnings,
        completeness,
    };

    Ok(IngestOutput {
        events: merged_events,
        coverage,
    })
}

fn resolve_conflict_legacy(
    events: &[EventEnvelope],
    resolution: ConflictResolution,
) -> EventEnvelope {
    match resolution {
        ConflictResolution::PreferFirst => events[0].clone(),
        ConflictResolution::PreferMostRecent => events
            .iter()
            .max_by_key(|e| e.occurred_at)
            .cloned()
            .unwrap(),
        ConflictResolution::PreferMostComplete => events
            .iter()
            .max_by_key(|e| completeness_score_legacy(e))
            .cloned()
            .unwrap(),
    }
}

fn completeness_score_legacy(event: &EventEnvelope) -> usize {
    let mut score = 0;

    // Check for non-empty fields
    if !event.actor.login.is_empty() {
        score += 1;
    }
    if event.actor.id.is_some() {
        score += 1;
    }
    if !event.repo.full_name.is_empty() {
        score += 1;
    }
    if event.repo.html_url.is_some() {
        score += 1;
    }
    if !event.tags.is_empty() {
        score += 1;
    }
    if !event.links.is_empty() {
        score += 1;
    }
    if event.source.url.is_some() {
        score += 1;
    }
    if event.source.opaque_id.is_some() {
        score += 1;
    }

    // Check payload completeness
    match &event.payload {
        shiplog_schema::event::EventPayload::PullRequest(pr) => {
            if pr.additions.is_some() {
                score += 1;
            }
            if pr.deletions.is_some() {
                score += 1;
            }
            if pr.changed_files.is_some() {
                score += 1;
            }
            if pr.merged_at.is_some() {
                score += 1;
            }
        }
        shiplog_schema::event::EventPayload::Manual(manual) => {
            if manual.description.is_some() {
                score += 1;
            }
            if manual.started_at.is_some() {
                score += 1;
            }
            if manual.ended_at.is_some() {
                score += 1;
            }
            if manual.impact.is_some() {
                score += 1;
            }
        }
        _ => {}
    }

    score
}

/// Calculate a completeness score for an event (higher = more complete).
fn completeness_score(event: &EventEnvelope) -> u32 {
    let mut score = 0;

    // Check payload completeness
    match &event.payload {
        shiplog_schema::event::EventPayload::PullRequest(pr) => {
            score += 10;
            if pr.additions.is_some() {
                score += 1;
            }
            if pr.deletions.is_some() {
                score += 1;
            }
            if pr.changed_files.is_some() {
                score += 1;
            }
            if !pr.touched_paths_hint.is_empty() {
                score += 1;
            }
        }
        shiplog_schema::event::EventPayload::Review(r) => {
            score += 8;
            if !r.pull_title.is_empty() {
                score += 1;
            }
        }
        shiplog_schema::event::EventPayload::Manual(m) => {
            score += 5;
            if m.description.is_some() {
                score += 2;
            }
            if m.impact.is_some() {
                score += 2;
            }
        }
    }

    // Check source completeness
    if event.source.url.is_some() {
        score += 1;
    }
    if event.source.opaque_id.is_some() {
        score += 1;
    }

    // Check links
    if !event.links.is_empty() {
        score += 2;
    }

    // Check tags
    if !event.tags.is_empty() {
        score += 1;
    }

    score
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, TimeZone, Utc};
    use shiplog_ids::EventId;
    use shiplog_schema::coverage::{CoverageManifest, CoverageSlice, TimeWindow};
    use shiplog_schema::event::{
        Actor, EventKind, EventPayload, ManualEvent, ManualEventType, RepoRef, RepoVisibility,
        SourceRef, SourceSystem,
    };
    fn make_event(id: &str, occurred_at: chrono::DateTime<chrono::Utc>) -> EventEnvelope {
        EventEnvelope {
            id: EventId::from_parts([id]),
            kind: EventKind::Manual,
            occurred_at,
            actor: Actor {
                login: "testuser".to_string(),
                id: Some(123),
            },
            repo: RepoRef {
                full_name: "owner/test".to_string(),
                html_url: Some("https://github.com/owner/test".to_string()),
                visibility: RepoVisibility::Public,
            },
            payload: EventPayload::Manual(ManualEvent {
                event_type: ManualEventType::Note,
                title: "Test event".to_string(),
                description: None,
                started_at: None,
                ended_at: None,
                impact: None,
            }),
            tags: vec![],
            links: vec![],
            source: SourceRef {
                system: SourceSystem::Manual,
                url: None,
                opaque_id: None,
            },
        }
    }

    fn make_event_with_tags(
        id: &str,
        occurred_at: chrono::DateTime<chrono::Utc>,
        tags: Vec<String>,
    ) -> EventEnvelope {
        let mut e = make_event(id, occurred_at);
        e.tags = tags;
        e
    }

    fn coverage(
        w: usize,
        completeness: Completeness,
        source: &str,
        warning: &str,
    ) -> CoverageManifest {
        CoverageManifest {
            run_id: RunId::now("test"),
            generated_at: Utc.timestamp_nanos(1),
            user: "tester".to_string(),
            window: TimeWindow {
                since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
            },
            mode: "merged".to_string(),
            sources: vec![source.to_string()],
            slices: vec![CoverageSlice {
                window: TimeWindow {
                    since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                    until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
                },
                query: "q".to_string(),
                total_count: w as u64,
                fetched: w as u64,
                incomplete_results: None,
                notes: vec![],
            }],
            warnings: vec![warning.to_string()],
            completeness,
        }
    }

    #[test]
    fn merge_empty_sources() {
        let result = merge_events(vec![], &MergeStrategy::default());
        assert!(result.is_empty());
    }

    #[test]
    fn merge_single_source() {
        let events = vec![
            make_event("1", Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap()),
            make_event("2", Utc.with_ymd_and_hms(2025, 1, 2, 0, 0, 0).unwrap()),
        ];
        let result = merge_events(vec![events], &MergeStrategy::default());
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn merge_deduplicates_by_id() {
        let event1 = make_event("1", Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap());
        let event2 = make_event("1", Utc.with_ymd_and_hms(2025, 1, 2, 0, 0, 0).unwrap());

        let result = merge_events(
            vec![vec![event1.clone()], vec![event2.clone()]],
            &MergeStrategy::KeepLast,
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].occurred_at, event2.occurred_at);
    }

    #[test]
    fn merge_keeps_first_strategy() {
        let event1 = make_event("1", Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap());
        let event2 = make_event("1", Utc.with_ymd_and_hms(2025, 1, 2, 0, 0, 0).unwrap());

        let result = merge_events(
            vec![vec![event1.clone()], vec![event2]],
            &MergeStrategy::KeepFirst,
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].occurred_at, event1.occurred_at);
    }

    #[test]
    fn merge_keeps_last_strategy() {
        let event1 = make_event("1", Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap());
        let event2 = make_event("1", Utc.with_ymd_and_hms(2025, 1, 2, 0, 0, 0).unwrap());

        let result = merge_events(
            vec![vec![event1], vec![event2.clone()]],
            &MergeStrategy::KeepLast,
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].occurred_at, event2.occurred_at);
    }

    #[test]
    fn merge_two_helper() {
        let left = vec![make_event(
            "1",
            Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap(),
        )];
        let right = vec![make_event(
            "2",
            Utc.with_ymd_and_hms(2025, 1, 2, 0, 0, 0).unwrap(),
        )];

        let result = merge_two(&left, &right, &MergeStrategy::default());
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn merge_result_is_sorted() {
        let events = vec![
            make_event("a", Utc.with_ymd_and_hms(2025, 1, 3, 0, 0, 0).unwrap()),
            make_event("b", Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap()),
            make_event("c", Utc.with_ymd_and_hms(2025, 1, 2, 0, 0, 0).unwrap()),
        ];

        let result = merge_events(vec![events], &MergeStrategy::default());

        // Should be sorted by occurred_at (Jan 1, Jan 2, Jan 3)
        assert_eq!(result.len(), 3);
        assert!(result[0].occurred_at <= result[1].occurred_at);
        assert!(result[1].occurred_at <= result[2].occurred_at);
    }

    #[test]
    fn merge_ingest_outputs_unifies_coverage_and_events() {
        let ingest_a = IngestOutput {
            events: vec![
                make_event("a", Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap()),
                make_event("b", Utc.with_ymd_and_hms(2025, 1, 1, 1, 0, 0).unwrap()),
            ],
            coverage: coverage(2, Completeness::Partial, "github", "a.warning"),
        };
        let ingest_b = IngestOutput {
            events: vec![
                make_event("b", Utc.with_ymd_and_hms(2025, 1, 1, 2, 0, 0).unwrap()),
                make_event("c", Utc.with_ymd_and_hms(2025, 1, 1, 3, 0, 0).unwrap()),
            ],
            coverage: coverage(2, Completeness::Complete, "local_git", "b.warning"),
        };

        let merged =
            merge_ingest_outputs(&[ingest_a, ingest_b], ConflictResolution::PreferMostRecent)
                .unwrap();

        assert_eq!(merged.ingest_output.events.len(), 3);
        assert_eq!(merged.report.conflict_count, 1);
        assert_eq!(merged.ingest_output.coverage.sources.len(), 2);
        assert_eq!(merged.ingest_output.coverage.warnings.len(), 3);
        assert_eq!(
            merged.ingest_output.coverage.completeness,
            Completeness::Partial
        );
        assert_eq!(merged.ingest_output.coverage.mode, "merged");
    }

    #[test]
    fn merge_ingest_outputs_rejects_empty_input() {
        let err = merge_ingest_outputs(&[], ConflictResolution::PreferMostRecent)
            .expect_err("expected empty input error");
        assert!(
            err.to_string().contains("No ingest outputs to merge"),
            "{err}"
        );
    }

    // --- Edge-case tests ---

    #[test]
    fn merge_multiple_sources_no_overlap() {
        let t1 = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let t2 = Utc.with_ymd_and_hms(2025, 1, 2, 0, 0, 0).unwrap();
        let t3 = Utc.with_ymd_and_hms(2025, 1, 3, 0, 0, 0).unwrap();
        let result = merge_events(
            vec![
                vec![make_event("a", t1)],
                vec![make_event("b", t2)],
                vec![make_event("c", t3)],
            ],
            &MergeStrategy::default(),
        );
        assert_eq!(result.len(), 3);
        assert!(result[0].occurred_at <= result[1].occurred_at);
        assert!(result[1].occurred_at <= result[2].occurred_at);
    }

    #[test]
    fn merge_all_duplicates() {
        let t = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let e = make_event("same", t);
        let result = merge_events(
            vec![vec![e.clone()], vec![e.clone()], vec![e.clone()]],
            &MergeStrategy::KeepFirst,
        );
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn merge_keeps_most_complete_strategy() {
        let t = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let sparse = make_event("x", t);
        let rich = make_event_with_tags("x", t, vec!["tag1".into(), "tag2".into()]);
        let result = merge_events(
            vec![vec![sparse], vec![rich.clone()]],
            &MergeStrategy::KeepMostComplete,
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].tags, rich.tags);
    }

    #[test]
    fn merge_preserves_order_same_timestamp() {
        let t = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let events = vec![make_event("z", t), make_event("a", t), make_event("m", t)];
        let result = merge_events(vec![events], &MergeStrategy::default());
        assert_eq!(result.len(), 3);
        // With same timestamp, sorted by id hash
        for w in result.windows(2) {
            assert!(w[0].occurred_at <= w[1].occurred_at);
            if w[0].occurred_at == w[1].occurred_at {
                assert!(w[0].id.0 <= w[1].id.0);
            }
        }
    }

    #[test]
    fn merge_single_event_per_source() {
        let t1 = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let t2 = Utc.with_ymd_and_hms(2025, 1, 2, 0, 0, 0).unwrap();
        let result = merge_events(
            vec![vec![make_event("a", t1)], vec![make_event("b", t2)]],
            &MergeStrategy::default(),
        );
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn merge_two_empty_lists() {
        let result = merge_two(&[], &[], &MergeStrategy::default());
        assert!(result.is_empty());
    }

    #[test]
    fn merge_two_one_empty() {
        let t = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let events = vec![make_event("a", t)];
        let result = merge_two(&events, &[], &MergeStrategy::default());
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn merge_ingest_outputs_single_source() {
        let ingest = IngestOutput {
            events: vec![make_event(
                "a",
                Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap(),
            )],
            coverage: coverage(1, Completeness::Complete, "github", ""),
        };
        let merged = merge_ingest_outputs(&[ingest], ConflictResolution::PreferFirst).unwrap();
        assert_eq!(merged.ingest_output.events.len(), 1);
        assert_eq!(merged.report.conflict_count, 0);
    }

    #[test]
    fn merge_ingest_outputs_all_complete() {
        let ingest_a = IngestOutput {
            events: vec![make_event(
                "a",
                Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap(),
            )],
            coverage: coverage(1, Completeness::Complete, "github", ""),
        };
        let ingest_b = IngestOutput {
            events: vec![make_event(
                "b",
                Utc.with_ymd_and_hms(2025, 1, 2, 0, 0, 0).unwrap(),
            )],
            coverage: coverage(1, Completeness::Complete, "local_git", ""),
        };
        let merged =
            merge_ingest_outputs(&[ingest_a, ingest_b], ConflictResolution::PreferFirst).unwrap();
        assert_eq!(
            merged.ingest_output.coverage.completeness,
            Completeness::Complete
        );
    }

    #[test]
    fn merge_legacy_rejects_empty_input() {
        let err = merge_ingest_outputs_legacy(&[], ConflictResolution::PreferFirst)
            .expect_err("expected error");
        assert!(err.to_string().contains("No ingest outputs to merge"));
    }

    #[test]
    fn merge_legacy_deduplicates() {
        let t = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let ingest = IngestOutput {
            events: vec![make_event("a", t)],
            coverage: coverage(1, Completeness::Complete, "github", ""),
        };
        let merged =
            merge_ingest_outputs_legacy(&[ingest.clone(), ingest], ConflictResolution::PreferFirst)
                .unwrap();
        assert_eq!(merged.events.len(), 1);
    }

    #[test]
    fn conflict_resolution_to_merge_strategy_mapping() {
        let s: MergeStrategy = ConflictResolution::PreferFirst.into();
        assert!(matches!(s, MergeStrategy::KeepFirst));
        let s: MergeStrategy = ConflictResolution::PreferMostRecent.into();
        assert!(matches!(s, MergeStrategy::KeepLast));
        let s: MergeStrategy = ConflictResolution::PreferMostComplete.into();
        assert!(matches!(s, MergeStrategy::KeepMostComplete));
    }

    // --- Snapshot tests ---

    #[test]
    fn snapshot_merge_report() {
        let t1 = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let t2 = Utc.with_ymd_and_hms(2025, 1, 2, 0, 0, 0).unwrap();
        let ingest_a = IngestOutput {
            events: vec![make_event("a", t1), make_event("shared", t1)],
            coverage: coverage(2, Completeness::Complete, "github", "warn-a"),
        };
        let ingest_b = IngestOutput {
            events: vec![make_event("shared", t2), make_event("b", t2)],
            coverage: coverage(2, Completeness::Complete, "local_git", "warn-b"),
        };
        let merged =
            merge_ingest_outputs(&[ingest_a, ingest_b], ConflictResolution::PreferMostRecent)
                .unwrap();
        insta::assert_debug_snapshot!(merged.report);
    }

    #[test]
    fn snapshot_merged_event_ids() {
        let t1 = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let t2 = Utc.with_ymd_and_hms(2025, 1, 2, 0, 0, 0).unwrap();
        let t3 = Utc.with_ymd_and_hms(2025, 1, 3, 0, 0, 0).unwrap();
        let result = merge_events(
            vec![
                vec![make_event("alpha", t1), make_event("beta", t2)],
                vec![make_event("beta", t2), make_event("gamma", t3)],
            ],
            &MergeStrategy::KeepFirst,
        );
        let ids: Vec<&str> = result.iter().map(|e| e.id.0.as_str()).collect();
        insta::assert_debug_snapshot!(ids);
    }

    // --- Property tests ---

    mod prop {
        use super::*;
        use proptest::prelude::*;
        use std::collections::HashSet;

        proptest! {
            #[test]
            fn merge_output_length_le_input(n1 in 0usize..5, n2 in 0usize..5) {
                let t = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
                let src1: Vec<_> = (0..n1).map(|i| make_event(&format!("a{i}"), t)).collect();
                let src2: Vec<_> = (0..n2).map(|i| make_event(&format!("b{i}"), t)).collect();
                let result = merge_events(vec![src1, src2], &MergeStrategy::KeepFirst);
                prop_assert!(result.len() <= n1 + n2);
            }

            #[test]
            fn merge_output_always_sorted(n in 1usize..8) {
                let events: Vec<_> = (0..n).map(|i| {
                    let day = (i % 28) as u32 + 1;
                    make_event(&format!("e{i}"), Utc.with_ymd_and_hms(2025, 1, day, 0, 0, 0).unwrap())
                }).collect();
                let result = merge_events(vec![events], &MergeStrategy::default());
                for w in result.windows(2) {
                    prop_assert!(w[0].occurred_at <= w[1].occurred_at);
                }
            }

            #[test]
            fn merge_is_idempotent(n in 1usize..5) {
                let t = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
                let events: Vec<_> = (0..n).map(|i| make_event(&format!("e{i}"), t)).collect();
                let first = merge_events(vec![events.clone()], &MergeStrategy::KeepFirst);
                let second = merge_events(vec![first.clone()], &MergeStrategy::KeepFirst);
                prop_assert_eq!(first.len(), second.len());
                for (a, b) in first.iter().zip(second.iter()) {
                    prop_assert_eq!(&a.id, &b.id);
                }
            }

            #[test]
            fn all_unique_ids_preserved(n1 in 0usize..4, n2 in 0usize..4) {
                let t = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
                let src1: Vec<_> = (0..n1).map(|i| make_event(&format!("s1_{i}"), t)).collect();
                let src2: Vec<_> = (0..n2).map(|i| make_event(&format!("s2_{i}"), t)).collect();
                let mut all_ids: HashSet<_> = src1.iter().map(|e| e.id.clone()).collect();
                all_ids.extend(src2.iter().map(|e| e.id.clone()));
                let result = merge_events(vec![src1, src2], &MergeStrategy::KeepFirst);
                let result_ids: HashSet<_> = result.iter().map(|e| e.id.clone()).collect();
                prop_assert_eq!(all_ids, result_ids);
            }
        }
    }
}
