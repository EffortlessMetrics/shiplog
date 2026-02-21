//! Markdown packet renderer for shiplog.
//!
//! Converts canonical events, workstreams, and coverage metadata into an
//! editable self-review packet with receipts and appendix sections.

use anyhow::Result;
use shiplog_ports::Renderer;
use shiplog_schema::coverage::CoverageManifest;
use shiplog_schema::event::{EventEnvelope, EventKind, EventPayload, ManualEventType};
use shiplog_schema::workstream::WorkstreamsFile;
use std::collections::HashMap;

/// Maximum receipts to show per workstream in the main packet
const MAX_RECEIPTS_PER_WORKSTREAM: usize = 5;

/// Minimal renderer that produces a copy-ready Markdown packet.
///
/// The output is intentionally low-magic:
/// - headings
/// - short bullets
/// - receipts with URLs when available
pub struct MarkdownRenderer;

impl Renderer for MarkdownRenderer {
    fn render_packet_markdown(
        &self,
        user: &str,
        window_label: &str,
        events: &[EventEnvelope],
        workstreams: &WorkstreamsFile,
        coverage: &CoverageManifest,
    ) -> Result<String> {
        let mut out = String::new();

        out.push_str(&format!("# Self review packet: {user}\n\n"));
        out.push_str(&format!("**Window:** {window_label}\n\n"));

        // Enhanced Coverage section
        out.push_str("## Coverage\n\n");
        out.push_str(&format!(
            "- **Completeness:** {:?}\n",
            coverage.completeness
        ));
        out.push_str(&format!(
            "- **Date window:** {} to {}\n",
            coverage.window.since, coverage.window.until
        ));
        out.push_str(&format!("- **Mode:** {}\n", coverage.mode));
        out.push_str(&format!("- **Sources:** {}\n", coverage.sources.join(", ")));

        // Event counts by type
        let pr_count = events
            .iter()
            .filter(|e| matches!(e.kind, EventKind::PullRequest))
            .count();
        let review_count = events
            .iter()
            .filter(|e| matches!(e.kind, EventKind::Review))
            .count();
        let manual_count = events
            .iter()
            .filter(|e| matches!(e.kind, EventKind::Manual))
            .count();
        out.push_str(&format!(
            "- **Events ingested:** {pr_count} PRs, {review_count} reviews, {manual_count} manual\n"
        ));

        // Coverage slicing details
        if !coverage.slices.is_empty() {
            out.push_str(&format!("- **Query slices:** {}\n", coverage.slices.len()));

            // Check for partial results or caps
            let partial_count = coverage
                .slices
                .iter()
                .filter(|s| s.incomplete_results.unwrap_or(false))
                .count();
            if partial_count > 0 {
                out.push_str(&format!(
                    "  - ⚠️ {} slices had incomplete results\n",
                    partial_count
                ));
            }

            // Show slices that hit caps
            let capped_slices: Vec<_> = coverage
                .slices
                .iter()
                .filter(|s| s.total_count > s.fetched)
                .collect();
            if !capped_slices.is_empty() {
                out.push_str("  - **Slicing applied (API caps):**\n");
                for slice in capped_slices.iter().take(3) {
                    let pct = if slice.total_count > 0 {
                        (slice.fetched as f64 / slice.total_count as f64 * 100.0) as u64
                    } else {
                        100
                    };
                    out.push_str(&format!(
                        "    - {}: fetched {}/{} ({}%)\n",
                        slice.query, slice.fetched, slice.total_count, pct
                    ));
                }
                if capped_slices.len() > 3 {
                    out.push_str(&format!("    - ... and {} more\n", capped_slices.len() - 3));
                }
            }
        }

        // Warnings
        if !coverage.warnings.is_empty() {
            out.push_str("- **Warnings:**\n");
            for w in &coverage.warnings {
                out.push_str(&format!("  - ⚠️ {w}\n"));
            }
        }
        out.push('\n');

        out.push_str("## Summary\n\n");
        out.push_str(&format!(
            "- Workstreams: {}\n",
            workstreams.workstreams.len()
        ));
        out.push_str(&format!("- Total events: {}\n", events.len()));
        out.push('\n');

        out.push_str("## Workstreams\n\n");

        let by_id: HashMap<String, &EventEnvelope> =
            events.iter().map(|e| (e.id.0.clone(), e)).collect();

        // Track which receipts were shown in main section (for appendix)
        let mut shown_receipts: HashMap<String, Vec<String>> = HashMap::new();

        for ws in &workstreams.workstreams {
            out.push_str(&format!("### {}\n\n", ws.title));
            if let Some(s) = &ws.summary {
                out.push_str(s);
                out.push_str("\n\n");
            }

            out.push_str("**Claim scaffolds**\n\n");
            out.push_str("- Problem: _fill_\n");
            out.push_str("- What I shipped: _fill_\n");
            out.push_str("- Why it mattered: _fill_\n");
            out.push_str("- Result: _fill_\n\n");

            // Split receipts into main (top N) and appendix (remainder)
            let (main_receipts, appendix_receipts): (Vec<_>, Vec<_>) =
                if ws.receipts.len() <= MAX_RECEIPTS_PER_WORKSTREAM {
                    (ws.receipts.clone(), Vec::new())
                } else {
                    let (main, appendix) = ws.receipts.split_at(MAX_RECEIPTS_PER_WORKSTREAM);
                    (main.to_vec(), appendix.to_vec())
                };

            // Track shown receipts for this workstream
            shown_receipts.insert(
                ws.id.0.clone(),
                main_receipts.iter().map(|r| r.0.clone()).collect(),
            );

            out.push_str("**Receipts**\n\n");
            if main_receipts.is_empty() {
                out.push_str("- (none)\n\n");
            } else {
                for id in &main_receipts {
                    if let Some(ev) = by_id.get(&id.0) {
                        out.push_str(&format!("- {}\n", format_receipt(ev)));
                    }
                }
                if !appendix_receipts.is_empty() {
                    out.push_str(&format!(
                        "- *... and {} more in [Appendix](#appendix-receipts)*\n",
                        appendix_receipts.len()
                    ));
                }
                out.push('\n');
            }
            // A small stats line keeps the packet honest without becoming a scoreboard.
            out.push_str(&format!(
                "_PRs: {}, Reviews: {}, Manual: {}_\n\n",
                ws.stats.pull_requests, ws.stats.reviews, ws.stats.manual_events
            ));
        }

        // Appendix with all receipts
        out.push_str("## Appendix: All Receipts\n\n");

        for ws in &workstreams.workstreams {
            if ws.events.is_empty() {
                continue;
            }

            out.push_str(&format!("### {}\n\n", ws.title));

            // Show all events for this workstream, not just receipts
            for event_id in &ws.events {
                if let Some(ev) = by_id.get(&event_id.0) {
                    out.push_str(&format!("- {}\n", format_receipt(ev)));
                }
            }
            out.push('\n');
        }
        out.push_str("---\n\n");
        out.push_str("## File Artifacts\n\n");
        out.push_str("- `ledger.events.jsonl` (canonical events)\n");
        out.push_str("- `coverage.manifest.json` (completeness + slicing)\n");
        out.push_str("- `workstreams.yaml` (editable clustering)\n");
        out.push_str("- `manual_events.yaml` (non-GitHub work)\n");

        Ok(out)
    }
}

fn format_receipt(ev: &EventEnvelope) -> String {
    match (&ev.kind, &ev.payload) {
        (EventKind::PullRequest, EventPayload::PullRequest(pr)) => {
            let title = &pr.title;
            let number = pr.number;
            let repo = &ev.repo.full_name;
            let url = ev
                .links
                .iter()
                .find(|l| l.label == "pr")
                .map(|l| l.url.as_str())
                .unwrap_or("");
            if url.is_empty() {
                format!("{repo}#{number}: {title}")
            } else {
                format!("[{repo}#{number}]({url}) — {title}")
            }
        }
        (EventKind::Review, EventPayload::Review(r)) => {
            let number = r.pull_number;
            let repo = &ev.repo.full_name;
            let url = ev
                .links
                .iter()
                .find(|l| l.label == "pr")
                .map(|l| l.url.as_str())
                .unwrap_or("");
            if url.is_empty() {
                format!("Review on {repo}#{number}: {}", r.state)
            } else {
                format!("Review on [{repo}#{number}]({url}) — {}", r.state)
            }
        }
        (EventKind::Manual, EventPayload::Manual(m)) => {
            let emoji = manual_type_emoji(&m.event_type);
            let title = &m.title;
            let links: Vec<String> = ev
                .links
                .iter()
                .map(|l| format!("[{}]({})", l.label, l.url))
                .collect();
            let links_str = if links.is_empty() {
                String::new()
            } else {
                format!(" ({})", links.join(", "))
            };
            format!("{emoji} {title}{links_str}")
        }
        _ => format!("event {}", ev.id),
    }
}

fn manual_type_emoji(event_type: &ManualEventType) -> &'static str {
    match event_type {
        ManualEventType::Note => "📝",
        ManualEventType::Incident => "🚨",
        ManualEventType::Design => "🏗️",
        ManualEventType::Mentoring => "👨‍🏫",
        ManualEventType::Launch => "🚀",
        ManualEventType::Migration => "🔄",
        ManualEventType::Review => "👀",
        ManualEventType::Other => "📌",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, TimeZone, Utc};
    use shiplog_ids::{EventId, RunId, WorkstreamId};
    use shiplog_schema::coverage::*;
    use shiplog_schema::event::*;
    use shiplog_schema::workstream::*;

    fn create_test_pr(id: &str, number: u64, title: &str) -> EventEnvelope {
        EventEnvelope {
            id: EventId::from_parts(["pr", id]),
            kind: EventKind::PullRequest,
            occurred_at: Utc.timestamp_opt(0, 0).unwrap(),
            actor: Actor {
                login: "octo".into(),
                id: None,
            },
            repo: RepoRef {
                full_name: "o/r".into(),
                html_url: Some("https://github.com/o/r".into()),
                visibility: RepoVisibility::Public,
            },
            payload: EventPayload::PullRequest(PullRequestEvent {
                number,
                title: title.into(),
                state: PullRequestState::Merged,
                created_at: Utc.timestamp_opt(0, 0).unwrap(),
                merged_at: Some(Utc.timestamp_opt(0, 0).unwrap()),
                additions: Some(10),
                deletions: Some(2),
                changed_files: Some(1),
                touched_paths_hint: vec![],
                window: None,
            }),
            tags: vec![],
            links: vec![Link {
                label: "pr".into(),
                url: format!("https://github.com/o/r/pull/{}", number),
            }],
            source: SourceRef {
                system: SourceSystem::Github,
                url: Some("https://api.github.com/...".into()),
                opaque_id: None,
            },
        }
    }

    #[test]
    fn packet_includes_coverage_summary() {
        let ev = create_test_pr("1", 1, "Add thing");

        let ws = WorkstreamsFile {
            version: 1,
            generated_at: Utc.timestamp_opt(0, 0).unwrap(),
            workstreams: vec![Workstream {
                id: WorkstreamId::from_parts(["repo", "o/r"]),
                title: "o/r".into(),
                summary: None,
                tags: vec![],
                stats: WorkstreamStats {
                    pull_requests: 1,
                    reviews: 0,
                    manual_events: 0,
                },
                events: vec![ev.id.clone()],
                receipts: vec![ev.id.clone()],
            }],
        };

        let cov = CoverageManifest {
            run_id: RunId("run_0".into()),
            generated_at: Utc.timestamp_opt(0, 0).unwrap(),
            user: "octo".into(),
            window: TimeWindow {
                since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
            },
            mode: "merged".into(),
            sources: vec!["github".into()],
            slices: vec![CoverageSlice {
                window: TimeWindow {
                    since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                    until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
                },
                query: "is:pr ...".into(),
                total_count: 1,
                fetched: 1,
                incomplete_results: Some(false),
                notes: vec![],
            }],
            warnings: vec![],
            completeness: Completeness::Complete,
        };

        let md = MarkdownRenderer
            .render_packet_markdown("octo", "2025-01-01..2025-02-01", &[ev], &ws, &cov)
            .unwrap();

        // Verify the coverage section contains the new fields
        assert!(md.contains("**Completeness:**"));
        assert!(md.contains("**Date window:**"));
        assert!(md.contains("**Mode:**"));
        assert!(md.contains("**Sources:**"));
        assert!(md.contains("**Events ingested:**"));
        assert!(md.contains("1 PRs, 0 reviews, 0 manual"));
    }

    /// Helper to build a full coverage manifest with fixed timestamps for snapshot tests.
    fn snapshot_coverage(
        completeness: Completeness,
        slices: Vec<CoverageSlice>,
        warnings: Vec<String>,
    ) -> CoverageManifest {
        CoverageManifest {
            run_id: RunId("run_snap".into()),
            generated_at: Utc.timestamp_opt(0, 0).unwrap(),
            user: "octo".into(),
            window: TimeWindow {
                since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
            },
            mode: "merged".into(),
            sources: vec!["github".into()],
            slices,
            warnings,
            completeness,
        }
    }

    #[test]
    fn snapshot_full_packet() {
        let ev = create_test_pr("1", 42, "Add authentication flow");
        let ws = WorkstreamsFile {
            version: 1,
            generated_at: Utc.timestamp_opt(0, 0).unwrap(),
            workstreams: vec![Workstream {
                id: WorkstreamId::from_parts(["repo", "o/r"]),
                title: "o/r".into(),
                summary: Some("Auth improvements".into()),
                tags: vec!["repo".into()],
                stats: WorkstreamStats {
                    pull_requests: 1,
                    reviews: 0,
                    manual_events: 0,
                },
                events: vec![ev.id.clone()],
                receipts: vec![ev.id.clone()],
            }],
        };
        let cov = snapshot_coverage(
            Completeness::Complete,
            vec![CoverageSlice {
                window: TimeWindow {
                    since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                    until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
                },
                query: "is:pr author:octo merged:2025-01-01..2025-01-31".into(),
                total_count: 1,
                fetched: 1,
                incomplete_results: Some(false),
                notes: vec![],
            }],
            vec![],
        );

        let md = MarkdownRenderer
            .render_packet_markdown("octo", "2025-01-01..2025-02-01", &[ev], &ws, &cov)
            .unwrap();
        insta::assert_snapshot!(md);
    }

    #[test]
    fn snapshot_empty_packet() {
        let ws = WorkstreamsFile {
            version: 1,
            generated_at: Utc.timestamp_opt(0, 0).unwrap(),
            workstreams: vec![],
        };
        let cov = snapshot_coverage(Completeness::Complete, vec![], vec![]);

        let md = MarkdownRenderer
            .render_packet_markdown("octo", "2025-01-01..2025-02-01", &[], &ws, &cov)
            .unwrap();
        insta::assert_snapshot!(md);
    }

    #[test]
    fn snapshot_partial_coverage() {
        let ev = create_test_pr("1", 10, "Fix bug");
        let ws = WorkstreamsFile {
            version: 1,
            generated_at: Utc.timestamp_opt(0, 0).unwrap(),
            workstreams: vec![Workstream {
                id: WorkstreamId::from_parts(["repo", "o/r"]),
                title: "o/r".into(),
                summary: None,
                tags: vec![],
                stats: WorkstreamStats {
                    pull_requests: 1,
                    reviews: 0,
                    manual_events: 0,
                },
                events: vec![ev.id.clone()],
                receipts: vec![ev.id.clone()],
            }],
        };
        let cov = snapshot_coverage(
            Completeness::Partial,
            vec![CoverageSlice {
                window: TimeWindow {
                    since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                    until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
                },
                query: "is:pr author:octo merged:2025-01-01..2025-01-31".into(),
                total_count: 1200,
                fetched: 1000,
                incomplete_results: Some(true),
                notes: vec!["partial:unresolvable_at_this_granularity".into()],
            }],
            vec!["Reviews are collected via search + per-PR review fetch; treat as best-effort coverage.".into()],
        );

        let md = MarkdownRenderer
            .render_packet_markdown("octo", "2025-01-01..2025-02-01", &[ev], &ws, &cov)
            .unwrap();
        insta::assert_snapshot!(md);
    }

    #[test]
    fn receipts_truncated_when_exceeds_limit() {
        // Create 7 PRs
        let events: Vec<_> = (1..=7)
            .map(|i| create_test_pr(&i.to_string(), i, &format!("PR {}", i)))
            .collect();

        let event_ids: Vec<_> = events.iter().map(|e| e.id.clone()).collect();
        let receipt_ids: Vec<_> = event_ids.clone();

        let ws = WorkstreamsFile {
            version: 1,
            generated_at: Utc.timestamp_opt(0, 0).unwrap(),
            workstreams: vec![Workstream {
                id: WorkstreamId::from_parts(["repo", "o/r"]),
                title: "o/r".into(),
                summary: None,
                tags: vec![],
                stats: WorkstreamStats {
                    pull_requests: 7,
                    reviews: 0,
                    manual_events: 0,
                },
                events: event_ids,
                receipts: receipt_ids,
            }],
        };

        let cov = CoverageManifest {
            run_id: RunId("run_0".into()),
            generated_at: Utc.timestamp_opt(0, 0).unwrap(),
            user: "octo".into(),
            window: TimeWindow {
                since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
            },
            mode: "merged".into(),
            sources: vec!["github".into()],
            slices: vec![],
            warnings: vec![],
            completeness: Completeness::Complete,
        };

        let md = MarkdownRenderer
            .render_packet_markdown("octo", "2025-01-01..2025-02-01", &events, &ws, &cov)
            .unwrap();

        // Should show truncation notice
        assert!(md.contains("and 2 more in [Appendix]"));
        // Should have appendix section
        assert!(md.contains("## Appendix: All Receipts"));
    }
}
