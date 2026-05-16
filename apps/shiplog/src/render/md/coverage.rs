//! Coverage section rendering for Markdown packets.
//!
//! The coverage section has several independent responsibilities (included
//! sources, skipped sources, known gaps, and detailed query metadata). Keeping
//! each responsibility in a focused helper makes the packet contract easier to
//! review without changing the emitted Markdown shape.

use shiplog::schema::coverage::{Completeness, CoverageManifest, CoverageSlice};
use shiplog::schema::event::EventEnvelope;

use super::source::{
    SkippedSource, display_source_label, display_source_list, event_source_present,
    included_source_summary, skipped_source_warning, skipped_source_warnings, source_event_count,
    source_present,
};

pub(crate) fn render_coverage(
    out: &mut String,
    coverage: &CoverageManifest,
    events: &[EventEnvelope],
) {
    out.push_str("## Coverage and Limits\n\n");

    let skipped_sources = skipped_source_warnings(&coverage.warnings);
    render_included_sources(out, coverage, events, &skipped_sources);
    render_skipped_sources(out, &skipped_sources);
    render_known_gaps(out, coverage, events);
    render_coverage_details(out, coverage);
}

fn render_included_sources(
    out: &mut String,
    coverage: &CoverageManifest,
    events: &[EventEnvelope],
    skipped_sources: &[SkippedSource<'_>],
) {
    out.push_str("Included:\n");
    let included_sources = included_source_summary(&coverage.sources, events, skipped_sources);
    if included_sources.is_empty() {
        out.push_str("- No completed sources recorded\n");
    } else {
        for source in &included_sources {
            let count = source_event_count(events, source);
            let noun = if count == 1 { "event" } else { "events" };
            out.push_str(&format!(
                "- {}: {} {}\n",
                display_source_label(source),
                count,
                noun
            ));
        }
    }

    render_query_slice_summary(out, &coverage.slices);
    out.push('\n');
}

fn render_query_slice_summary(out: &mut String, slices: &[CoverageSlice]) {
    if slices.is_empty() {
        out.push_str("- Fetched events: not reported by query slices\n");
        return;
    }

    let fetched: u64 = slices.iter().map(|slice| slice.fetched).sum();
    let total: u64 = slices.iter().map(|slice| slice.total_count).sum();
    let slice_label = if slices.len() == 1 { "slice" } else { "slices" };
    out.push_str(&format!(
        "- Query slices: {} {}, fetched {} of {} reported results\n",
        slices.len(),
        slice_label,
        fetched,
        total
    ));
}

fn render_skipped_sources(out: &mut String, skipped_sources: &[SkippedSource<'_>]) {
    out.push_str("Skipped:\n");
    if skipped_sources.is_empty() {
        out.push_str("- None recorded\n");
    } else {
        for skipped in skipped_sources {
            out.push_str(&format!(
                "- {}: {}\n",
                display_source_label(skipped.source),
                skipped.reason
            ));
        }
    }
    out.push('\n');
}

fn render_known_gaps(out: &mut String, coverage: &CoverageManifest, events: &[EventEnvelope]) {
    out.push_str("Known gaps:\n");
    let mut has_gap = render_completeness_gap(out, &coverage.completeness);
    has_gap |= render_warning_gaps(out, &coverage.warnings);
    has_gap |= render_manual_source_gap(out, coverage, events);
    has_gap |= render_slice_quality_gaps(out, &coverage.slices);

    if !has_gap {
        out.push_str("- None recorded\n");
    }
    out.push('\n');
}

fn render_completeness_gap(out: &mut String, completeness: &Completeness) -> bool {
    if matches!(completeness, Completeness::Complete) {
        return false;
    }

    out.push_str(&format!("- Overall completeness is {}\n", completeness));
    true
}

fn render_warning_gaps(out: &mut String, warnings: &[String]) -> bool {
    let mut has_gap = false;
    for warning in warnings {
        if skipped_source_warning(warning).is_none() {
            has_gap = true;
            out.push_str(&format!("- {}\n", warning));
        }
    }
    has_gap
}

fn render_manual_source_gap(
    out: &mut String,
    coverage: &CoverageManifest,
    events: &[EventEnvelope],
) -> bool {
    if !source_present(&coverage.sources, "manual") && !event_source_present(events, "manual") {
        return false;
    }

    out.push_str("- Manual events are user-provided\n");
    true
}

fn render_slice_quality_gaps(out: &mut String, slices: &[CoverageSlice]) -> bool {
    let mut has_gap = false;

    let incomplete_count = slices
        .iter()
        .filter(|slice| slice.incomplete_results.unwrap_or(false))
        .count();
    if incomplete_count > 0 {
        has_gap = true;
        let slice_label = if incomplete_count == 1 {
            "slice"
        } else {
            "slices"
        };
        out.push_str(&format!(
            "- {} query {} reported incomplete results\n",
            incomplete_count, slice_label
        ));
    }

    let capped_count = slices
        .iter()
        .filter(|slice| slice.total_count > slice.fetched)
        .count();
    if capped_count > 0 {
        has_gap = true;
        let slice_label = if capped_count == 1 { "slice" } else { "slices" };
        out.push_str(&format!(
            "- {} query {} fetched fewer results than reported\n",
            capped_count, slice_label
        ));
    }

    has_gap
}

fn render_coverage_details(out: &mut String, coverage: &CoverageManifest) {
    out.push_str("Details:\n");
    out.push_str(&format!(
        "- **Date window:** {} to {}\n",
        coverage.window.since, coverage.window.until
    ));
    out.push_str(&format!("- **Mode:** {}\n", coverage.mode));
    out.push_str(&format!(
        "- **Sources:** {}\n",
        display_source_list(&coverage.sources)
    ));
    out.push_str(&format!(
        "- **Completeness:** {:?}\n",
        coverage.completeness
    ));
    render_query_slice_details(out, &coverage.slices);
    out.push('\n');
}

fn render_query_slice_details(out: &mut String, slices: &[CoverageSlice]) {
    if slices.is_empty() {
        return;
    }

    out.push_str(&format!("- **Query slices:** {}\n", slices.len()));
    render_incomplete_slice_detail(out, slices);
    render_capped_slice_details(out, slices);
}

fn render_incomplete_slice_detail(out: &mut String, slices: &[CoverageSlice]) {
    let partial_count = slices
        .iter()
        .filter(|s| s.incomplete_results.unwrap_or(false))
        .count();
    if partial_count > 0 {
        out.push_str(&format!(
            "  - ⚠️ {} slices had incomplete results\n",
            partial_count
        ));
    }
}

fn render_capped_slice_details(out: &mut String, slices: &[CoverageSlice]) {
    let capped_slices: Vec<_> = slices
        .iter()
        .filter(|s| s.total_count > s.fetched)
        .collect();
    if capped_slices.is_empty() {
        return;
    }

    out.push_str("  - **Slicing applied (API caps):**\n");
    for slice in capped_slices.iter().take(3) {
        out.push_str(&format!(
            "    - {}: fetched {}/{} ({}%)\n",
            slice.query,
            slice.fetched,
            slice.total_count,
            fetched_percent(slice)
        ));
    }
    if capped_slices.len() > 3 {
        out.push_str(&format!("    - ... and {} more\n", capped_slices.len() - 3));
    }
}

fn fetched_percent(slice: &CoverageSlice) -> u64 {
    if slice.total_count > 0 {
        (slice.fetched as f64 / slice.total_count as f64 * 100.0) as u64
    } else {
        100
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, TimeZone, Utc};
    use shiplog::ids::{EventId, RunId};
    use shiplog::schema::coverage::TimeWindow;
    use shiplog::schema::event::{
        Actor, EventKind, EventPayload, Link, PullRequestEvent, PullRequestState, RepoRef,
        RepoVisibility, SourceRef, SourceSystem,
    };

    fn coverage_manifest() -> CoverageManifest {
        CoverageManifest {
            run_id: RunId("test_run".into()),
            generated_at: Utc.timestamp_opt(0, 0).single().unwrap_or_default(),
            user: "tester".into(),
            window: TimeWindow {
                since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap_or_default(),
                until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap_or_default(),
            },
            mode: "merged".into(),
            sources: vec!["github".into()],
            slices: vec![],
            warnings: vec![],
            completeness: Completeness::Complete,
        }
    }

    fn slice(query: &str, fetched: u64, total: u64, incomplete: Option<bool>) -> CoverageSlice {
        CoverageSlice {
            window: TimeWindow {
                since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap_or_default(),
                until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap_or_default(),
            },
            query: query.to_string(),
            total_count: total,
            fetched,
            incomplete_results: incomplete,
            notes: vec![],
        }
    }

    fn pr_event(system: SourceSystem) -> EventEnvelope {
        EventEnvelope {
            id: EventId::from_parts(["github", "pr", "acme/foo", "1"]),
            kind: EventKind::PullRequest,
            occurred_at: Utc.timestamp_opt(0, 0).single().unwrap_or_default(),
            actor: Actor {
                login: "user".into(),
                id: None,
            },
            repo: RepoRef {
                full_name: "acme/foo".into(),
                html_url: Some("https://github.com/acme/foo".into()),
                visibility: RepoVisibility::Unknown,
            },
            payload: EventPayload::PullRequest(PullRequestEvent {
                number: 1,
                title: "PR 1".into(),
                state: PullRequestState::Merged,
                created_at: Utc.timestamp_opt(0, 0).single().unwrap_or_default(),
                merged_at: Some(Utc.timestamp_opt(0, 0).single().unwrap_or_default()),
                additions: Some(1),
                deletions: Some(0),
                changed_files: Some(1),
                touched_paths_hint: vec![],
                window: Some(TimeWindow {
                    since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap_or_default(),
                    until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap_or_default(),
                }),
            }),
            tags: vec![],
            links: vec![Link {
                label: "pr".into(),
                url: "https://github.com/acme/foo/pull/1".into(),
            }],
            source: SourceRef {
                system,
                url: None,
                opaque_id: None,
            },
        }
    }

    #[test]
    fn render_coverage_happy_path_emits_expected_sections() {
        let mut out = String::new();
        let coverage = coverage_manifest();
        let events: Vec<EventEnvelope> = vec![];

        render_coverage(&mut out, &coverage, &events);

        assert!(out.starts_with("## Coverage and Limits\n\n"));
        assert!(out.contains("Included:\n- GitHub: 0 events\n"));
        assert!(out.contains("- Fetched events: not reported by query slices\n"));
        assert!(out.contains("Skipped:\n- None recorded\n"));
        assert!(out.contains("Known gaps:\n- None recorded\n"));
        assert!(out.contains("Details:\n"));
        assert!(out.contains("- **Date window:** 2025-01-01 to 2025-02-01\n"));
        assert!(out.contains("- **Mode:** merged\n"));
        assert!(out.contains("- **Sources:** GitHub\n"));
        assert!(out.contains("- **Completeness:** Complete\n"));
    }

    #[test]
    fn render_coverage_with_events_counts_per_source_and_flags_manual_gap() {
        let mut out = String::new();
        let mut coverage = coverage_manifest();
        coverage.sources = vec!["github".into(), "manual".into()];
        coverage.slices = vec![slice("author:me", 5, 5, Some(false))];
        let events = vec![
            pr_event(SourceSystem::Github),
            pr_event(SourceSystem::Github),
            pr_event(SourceSystem::Manual),
        ];

        render_coverage(&mut out, &coverage, &events);

        assert!(out.contains("- GitHub: 2 events\n"));
        assert!(out.contains("- Manual: 1 event\n"));
        assert!(out.contains("- Manual events are user-provided\n"));
        assert!(out.contains("- Query slices: 1 slice, fetched 5 of 5 reported results\n"));
    }

    #[test]
    fn render_query_slice_summary_empty_reports_not_reported() {
        let mut out = String::new();
        render_query_slice_summary(&mut out, &[]);
        assert_eq!(out, "- Fetched events: not reported by query slices\n");
    }

    #[test]
    fn render_query_slice_summary_single_uses_singular_slice() {
        let mut out = String::new();
        render_query_slice_summary(&mut out, &[slice("q", 3, 4, None)]);
        assert_eq!(
            out,
            "- Query slices: 1 slice, fetched 3 of 4 reported results\n"
        );
    }

    #[test]
    fn render_query_slice_summary_multi_uses_plural_slices_and_sums() {
        let mut out = String::new();
        let slices = vec![slice("q1", 3, 4, None), slice("q2", 7, 10, None)];
        render_query_slice_summary(&mut out, &slices);
        assert_eq!(
            out,
            "- Query slices: 2 slices, fetched 10 of 14 reported results\n"
        );
    }

    #[test]
    fn render_skipped_sources_empty_emits_none_recorded() {
        let mut out = String::new();
        render_skipped_sources(&mut out, &[]);
        assert_eq!(out, "Skipped:\n- None recorded\n\n");
    }

    #[test]
    fn render_skipped_sources_non_empty_lists_each() {
        let mut out = String::new();
        let skipped = vec![
            SkippedSource {
                source: "github",
                reason: "rate limit",
            },
            SkippedSource {
                source: "gitlab",
                reason: "auth failure",
            },
        ];
        render_skipped_sources(&mut out, &skipped);
        assert_eq!(
            out,
            "Skipped:\n- GitHub: rate limit\n- GitLab: auth failure\n\n"
        );
    }

    #[test]
    fn render_known_gaps_clean_emits_none_recorded() {
        let mut out = String::new();
        let coverage = coverage_manifest();
        render_known_gaps(&mut out, &coverage, &[]);
        assert_eq!(out, "Known gaps:\n- None recorded\n\n");
    }

    #[test]
    fn render_completeness_gap_complete_returns_false_no_emit() {
        let mut out = String::new();
        let emitted = render_completeness_gap(&mut out, &Completeness::Complete);
        assert!(!emitted);
        assert!(out.is_empty());
    }

    #[test]
    fn render_completeness_gap_partial_emits_line_and_returns_true() {
        let mut out = String::new();
        let emitted = render_completeness_gap(&mut out, &Completeness::Partial);
        assert!(emitted);
        assert_eq!(out, "- Overall completeness is Partial\n");
    }

    #[test]
    fn render_completeness_gap_unknown_emits_line() {
        let mut out = String::new();
        let emitted = render_completeness_gap(&mut out, &Completeness::Unknown);
        assert!(emitted);
        assert_eq!(out, "- Overall completeness is Unknown\n");
    }

    #[test]
    fn render_warning_gaps_filters_skipped_source_format() {
        let mut out = String::new();
        let warnings = vec![
            "Configured source github was skipped: rate limit".to_string(),
            "Pagination capped at 1000 results".to_string(),
            "Configured source gitlab was skipped: auth".to_string(),
            "Another general warning".to_string(),
        ];
        let emitted = render_warning_gaps(&mut out, &warnings);
        assert!(emitted);
        assert_eq!(
            out,
            "- Pagination capped at 1000 results\n- Another general warning\n"
        );
    }

    #[test]
    fn render_warning_gaps_returns_false_when_all_filtered() {
        let mut out = String::new();
        let warnings = vec!["Configured source github was skipped: rate limit".to_string()];
        let emitted = render_warning_gaps(&mut out, &warnings);
        assert!(!emitted);
        assert!(out.is_empty());
    }

    #[test]
    fn render_manual_source_gap_source_only_emits() {
        let mut out = String::new();
        let mut coverage = coverage_manifest();
        coverage.sources = vec!["manual".into()];
        let emitted = render_manual_source_gap(&mut out, &coverage, &[]);
        assert!(emitted);
        assert_eq!(out, "- Manual events are user-provided\n");
    }

    #[test]
    fn render_manual_source_gap_event_only_emits() {
        let mut out = String::new();
        let coverage = coverage_manifest(); // sources = ["github"]
        let events = vec![pr_event(SourceSystem::Manual)];
        let emitted = render_manual_source_gap(&mut out, &coverage, &events);
        assert!(emitted);
        assert_eq!(out, "- Manual events are user-provided\n");
    }

    #[test]
    fn render_manual_source_gap_neither_returns_false() {
        let mut out = String::new();
        let coverage = coverage_manifest();
        let events = vec![pr_event(SourceSystem::Github)];
        let emitted = render_manual_source_gap(&mut out, &coverage, &events);
        assert!(!emitted);
        assert!(out.is_empty());
    }

    #[test]
    fn render_manual_source_gap_both_emits_single_line() {
        let mut out = String::new();
        let mut coverage = coverage_manifest();
        coverage.sources = vec!["github".into(), "manual".into()];
        let events = vec![pr_event(SourceSystem::Manual)];
        let emitted = render_manual_source_gap(&mut out, &coverage, &events);
        assert!(emitted);
        assert_eq!(out, "- Manual events are user-provided\n");
    }

    #[test]
    fn render_slice_quality_gaps_incomplete_singular() {
        let mut out = String::new();
        let slices = vec![slice("q", 5, 5, Some(true))];
        let emitted = render_slice_quality_gaps(&mut out, &slices);
        assert!(emitted);
        assert_eq!(out, "- 1 query slice reported incomplete results\n");
    }

    #[test]
    fn render_slice_quality_gaps_incomplete_plural() {
        let mut out = String::new();
        let slices = vec![slice("q1", 5, 5, Some(true)), slice("q2", 3, 3, Some(true))];
        let emitted = render_slice_quality_gaps(&mut out, &slices);
        assert!(emitted);
        assert_eq!(out, "- 2 query slices reported incomplete results\n");
    }

    #[test]
    fn render_slice_quality_gaps_capped_singular() {
        let mut out = String::new();
        let slices = vec![slice("q", 5, 10, Some(false))];
        let emitted = render_slice_quality_gaps(&mut out, &slices);
        assert!(emitted);
        assert_eq!(out, "- 1 query slice fetched fewer results than reported\n");
    }

    #[test]
    fn render_slice_quality_gaps_capped_plural() {
        let mut out = String::new();
        let slices = vec![
            slice("q1", 5, 10, Some(false)),
            slice("q2", 7, 20, Some(false)),
        ];
        let emitted = render_slice_quality_gaps(&mut out, &slices);
        assert!(emitted);
        assert_eq!(
            out,
            "- 2 query slices fetched fewer results than reported\n"
        );
    }

    #[test]
    fn render_slice_quality_gaps_incomplete_and_capped_together() {
        let mut out = String::new();
        let slices = vec![
            slice("q1", 5, 10, Some(true)),
            slice("q2", 3, 3, Some(true)),
            slice("q3", 7, 20, Some(false)),
        ];
        let emitted = render_slice_quality_gaps(&mut out, &slices);
        assert!(emitted);
        assert_eq!(
            out,
            "- 2 query slices reported incomplete results\n\
             - 2 query slices fetched fewer results than reported\n"
        );
    }

    #[test]
    fn render_slice_quality_gaps_clean_returns_false() {
        let mut out = String::new();
        let slices = vec![slice("q", 10, 10, Some(false))];
        let emitted = render_slice_quality_gaps(&mut out, &slices);
        assert!(!emitted);
        assert!(out.is_empty());
    }

    #[test]
    fn fetched_percent_zero_total_returns_one_hundred() {
        let s = slice("q", 0, 0, None);
        assert_eq!(fetched_percent(&s), 100);
    }

    #[test]
    fn fetched_percent_truncates_toward_zero() {
        let s = slice("q", 1, 3, None);
        assert_eq!(fetched_percent(&s), 33);
    }

    #[test]
    fn fetched_percent_full_fetch_is_one_hundred() {
        let s = slice("q", 7, 7, None);
        assert_eq!(fetched_percent(&s), 100);
    }

    #[test]
    fn render_capped_slice_details_truncates_past_three() {
        let mut out = String::new();
        let slices = vec![
            slice("q1", 1, 10, None),
            slice("q2", 2, 10, None),
            slice("q3", 3, 10, None),
            slice("q4", 4, 10, None),
            slice("q5", 5, 10, None),
        ];
        render_capped_slice_details(&mut out, &slices);
        assert!(out.contains("- **Slicing applied (API caps):**\n"));
        assert!(out.contains("    - q1: fetched 1/10 (10%)\n"));
        assert!(out.contains("    - q2: fetched 2/10 (20%)\n"));
        assert!(out.contains("    - q3: fetched 3/10 (30%)\n"));
        assert!(!out.contains("q4:"));
        assert!(!out.contains("q5:"));
        assert!(out.contains("    - ... and 2 more\n"));
    }

    #[test]
    fn render_capped_slice_details_empty_when_no_capped() {
        let mut out = String::new();
        let slices = vec![slice("q", 10, 10, None)];
        render_capped_slice_details(&mut out, &slices);
        assert!(out.is_empty());
    }
}
