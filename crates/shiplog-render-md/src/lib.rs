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

/// Maximum receipts to show per workstream in main packet
const MAX_RECEIPTS_PER_WORKSTREAM: usize = 5;

/// Section ordering configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SectionOrder {
    /// Default order: Summary, Workstreams, Receipts, Coverage
    #[default]
    Default,
    /// Alternative order: Coverage, Summary, Workstreams, Receipts
    CoverageFirst,
}

/// Minimal renderer that produces a copy-ready Markdown packet.
///
/// The output is intentionally low-magic:
/// - headings
/// - short bullets
/// - receipts with URLs when available
pub struct MarkdownRenderer {
    /// Section ordering configuration
    pub section_order: SectionOrder,
}

impl Default for MarkdownRenderer {
    fn default() -> Self {
        Self {
            section_order: SectionOrder::Default,
        }
    }
}

impl MarkdownRenderer {
    /// Create a new renderer with default section ordering.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new renderer with custom section ordering.
    pub fn with_section_order(mut self, order: SectionOrder) -> Self {
        self.section_order = order;
        self
    }
}

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

        // Render sections based on configured order
        match self.section_order {
            SectionOrder::Default => {
                render_summary(&mut out, user, window_label, events, workstreams, coverage);
                render_workstreams(&mut out, events, workstreams);
                render_receipts(&mut out, events, workstreams);
                render_coverage(&mut out, coverage);
            }
            SectionOrder::CoverageFirst => {
                render_coverage(&mut out, coverage);
                render_summary(&mut out, user, window_label, events, workstreams, coverage);
                render_workstreams(&mut out, events, workstreams);
                render_receipts(&mut out, events, workstreams);
            }
        }

        // Appendix with all receipts
        render_appendix(&mut out, events, workstreams);

        // File artifacts
        render_file_artifacts(&mut out);

        Ok(out)
    }
}

fn render_summary(
    out: &mut String,
    _user: &str,
    window_label: &str,
    events: &[EventEnvelope],
    workstreams: &WorkstreamsFile,
    coverage: &CoverageManifest,
) {
    out.push_str("# Summary\n\n");

    // Window
    out.push_str(&format!("**Window:** {}\n\n", window_label));

    // Workstream count
    out.push_str(&format!(
        "**Workstreams:** {}\n\n",
        workstreams.workstreams.len()
    ));

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
        "**Events:** {} PRs, {} reviews, {} manual\n\n",
        pr_count, review_count, manual_count
    ));

    // Completeness
    out.push_str(&format!("**Coverage:** {:?}\n\n", coverage.completeness));

    // Sources
    out.push_str(&format!("**Sources:** {}\n\n", coverage.sources.join(", ")));

    // Warnings
    if !coverage.warnings.is_empty() {
        out.push_str("**Warnings:**\n");
        for w in &coverage.warnings {
            out.push_str(&format!("  - ⚠️ {}\n", w));
        }
        out.push('\n');
    }
}

fn render_workstreams(out: &mut String, events: &[EventEnvelope], workstreams: &WorkstreamsFile) {
    out.push_str("## Workstreams\n\n");

    if workstreams.workstreams.is_empty() {
        out.push_str("_No workstreams found_\n\n");
        return;
    }

    let _by_id: HashMap<String, &EventEnvelope> =
        events.iter().map(|e| (e.id.0.clone(), e)).collect();

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

        // Stats
        out.push_str(&format!(
            "_PRs: {}, Reviews: {}, Manual: {}_\n\n",
            ws.stats.pull_requests, ws.stats.reviews, ws.stats.manual_events
        ));
    }
}

fn render_receipts(out: &mut String, events: &[EventEnvelope], workstreams: &WorkstreamsFile) {
    out.push_str("## Receipts\n\n");

    if workstreams.workstreams.is_empty() {
        out.push_str("_No workstreams, no receipts_\n\n");
        return;
    }

    let by_id: HashMap<String, &EventEnvelope> =
        events.iter().map(|e| (e.id.0.clone(), e)).collect();

    // Track which receipts were shown in main section (for appendix)
    let mut shown_receipts: HashMap<String, Vec<String>> = HashMap::new();

    for ws in &workstreams.workstreams {
        out.push_str(&format!("### Workstream: {}\n\n", ws.title));

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

        if main_receipts.is_empty() {
            out.push_str("- (none)\n\n");
        } else {
            for id in &main_receipts {
                if let Some(ev) = by_id.get(&id.0) {
                    out.push_str(&format!("- {}\n", format_receipt_clean(ev)));
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
    }
}

fn render_coverage(out: &mut String, coverage: &CoverageManifest) {
    out.push_str("## Coverage\n\n");

    // Date window
    out.push_str(&format!(
        "- **Date window:** {} to {}\n",
        coverage.window.since, coverage.window.until
    ));

    // Mode
    out.push_str(&format!("- **Mode:** {}\n", coverage.mode));

    // Sources
    out.push_str(&format!("- **Sources:** {}\n", coverage.sources.join(", ")));

    // Completeness
    out.push_str(&format!(
        "- **Completeness:** {:?}\n",
        coverage.completeness
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
    out.push('\n');
}

fn render_appendix(out: &mut String, events: &[EventEnvelope], workstreams: &WorkstreamsFile) {
    out.push_str("## Appendix: All Receipts\n\n");

    if workstreams.workstreams.is_empty() {
        return;
    }

    let by_id: HashMap<String, &EventEnvelope> =
        events.iter().map(|e| (e.id.0.clone(), e)).collect();

    for ws in &workstreams.workstreams {
        if ws.events.is_empty() {
            continue;
        }

        out.push_str(&format!("### {}\n\n", ws.title));

        // Show all events for this workstream, not just receipts
        for event_id in &ws.events {
            if let Some(ev) = by_id.get(&event_id.0) {
                out.push_str(&format!("- {}\n", format_receipt_clean(ev)));
            }
        }
        out.push('\n');
    }
    out.push_str("---\n\n");
}

fn render_file_artifacts(out: &mut String) {
    out.push_str("## File Artifacts\n\n");
    out.push_str("- `ledger.events.jsonl` (canonical events)\n");
    out.push_str("- `coverage.manifest.json` (completeness + slicing)\n");
    out.push_str("- `workstreams.yaml` (editable clustering)\n");
    out.push_str("- `manual_events.yaml` (non-GitHub work)\n");
}

/// Format a receipt with cleaner presentation including event type, title, date, and link.
fn format_receipt_clean(ev: &EventEnvelope) -> String {
    match (&ev.kind, &ev.payload) {
        (EventKind::PullRequest, EventPayload::PullRequest(pr)) => {
            let title = &pr.title;
            let _number = pr.number;
            let repo = &ev.repo.full_name;
            let url = ev
                .links
                .iter()
                .find(|l| l.label == "pr")
                .map(|l| l.url.as_str())
                .unwrap_or("");
            let date_str = ev.occurred_at.format("%Y-%m-%d").to_string();

            if url.is_empty() {
                format!("- [PR] {} ({}) — {}", title, date_str, repo)
            } else {
                format!("- [PR] {} ({}) — [{}]({})", title, date_str, repo, url)
            }
        }
        (EventKind::Review, EventPayload::Review(r)) => {
            let _number = r.pull_number;
            let repo = &ev.repo.full_name;
            let url = ev
                .links
                .iter()
                .find(|l| l.label == "pr")
                .map(|l| l.url.as_str())
                .unwrap_or("");
            let date_str = ev.occurred_at.format("%Y-%m-%d").to_string();

            if url.is_empty() {
                format!("- [Review] {} ({}) — {}", r.state, date_str, repo)
            } else {
                format!(
                    "- [Review] {} ({}) — [{}]({})",
                    r.state, date_str, repo, url
                )
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
                format!(" — {}", links.join(", "))
            };
            let date_str = ev.occurred_at.format("%Y-%m-%d").to_string();

            format!("- [{}] {} ({}){}", emoji, title, date_str, links_str)
        }
        _ => format!("- event {}", ev.id),
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
                full_name: "owner/repo".into(),
                html_url: None,
                visibility: RepoVisibility::Public,
            },
            payload: EventPayload::PullRequest(PullRequestEvent {
                number,
                title: title.into(),
                state: PullRequestState::Merged,
                created_at: Utc.timestamp_opt(0, 0).unwrap(),
                merged_at: Some(Utc.timestamp_opt(0, 0).unwrap()),
                additions: Some(10),
                deletions: Some(5),
                changed_files: Some(2),
                touched_paths_hint: vec![],
                window: None,
            }),
            tags: vec![],
            links: vec![Link {
                label: "pr".into(),
                url: format!("https://github.com/owner/repo/pull/{}", number),
            }],
            source: SourceRef {
                system: SourceSystem::Github,
                url: None,
                opaque_id: Some(id.into()),
            },
        }
    }

    fn create_test_manual(id: &str, event_type: ManualEventType, title: &str) -> EventEnvelope {
        EventEnvelope {
            id: EventId::from_parts(["manual", id]),
            kind: EventKind::Manual,
            occurred_at: Utc.timestamp_opt(0, 0).unwrap(),
            actor: Actor {
                login: "user".into(),
                id: None,
            },
            repo: RepoRef {
                full_name: "owner/repo".into(),
                html_url: None,
                visibility: RepoVisibility::Public,
            },
            payload: EventPayload::Manual(ManualEvent {
                event_type: event_type.clone(),
                title: title.into(),
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
                opaque_id: Some(id.into()),
            },
        }
    }

    #[test]
    fn test_snapshot_empty_packet() {
        let renderer = MarkdownRenderer::new();
        let events: Vec<EventEnvelope> = vec![];
        let workstreams = WorkstreamsFile {
            version: 1,
            generated_at: Utc::now(),
            workstreams: vec![],
        };
        let coverage = CoverageManifest {
            run_id: RunId::now("test"),
            generated_at: Utc::now(),
            user: "test".into(),
            window: TimeWindow {
                since: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                until: NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
            },
            mode: "test".into(),
            sources: vec![],
            slices: vec![],
            warnings: vec![],
            completeness: Completeness::Complete,
        };

        let result = renderer
            .render_packet_markdown("test", "2024", &events, &workstreams, &coverage)
            .unwrap();

        insta::assert_snapshot!(result);
    }

    #[test]
    fn test_snapshot_full_packet() {
        let renderer = MarkdownRenderer::new();
        let events = vec![
            create_test_pr("1", 1, "Fix authentication bug"),
            create_test_manual("2", ManualEventType::Incident, "Handle production incident"),
        ];
        let workstreams = WorkstreamsFile {
            version: 1,
            generated_at: Utc::now(),
            workstreams: vec![Workstream {
                id: WorkstreamId::from_parts(["ws", "1"]),
                title: "Authentication".into(),
                summary: Some("Fixed auth bugs and improved security".into()),
                tags: vec![],
                receipts: vec![EventId::from_parts(["pr", "1"])],
                events: vec![EventId::from_parts(["pr", "1"])],
                stats: WorkstreamStats {
                    pull_requests: 1,
                    reviews: 0,
                    manual_events: 0,
                },
            }],
        };
        let coverage = CoverageManifest {
            run_id: RunId::now("test"),
            generated_at: Utc::now(),
            user: "test".into(),
            window: TimeWindow {
                since: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                until: NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
            },
            mode: "test".into(),
            sources: vec!["github".into(), "manual".into()],
            slices: vec![],
            warnings: vec![],
            completeness: Completeness::Complete,
        };

        let result = renderer
            .render_packet_markdown("test", "2024", &events, &workstreams, &coverage)
            .unwrap();

        insta::assert_snapshot!(result);
    }

    #[test]
    fn test_snapshot_partial_coverage() {
        let renderer = MarkdownRenderer::new();
        let events = vec![create_test_pr("1", 1, "Add feature")];
        let workstreams = WorkstreamsFile {
            version: 1,
            generated_at: Utc::now(),
            workstreams: vec![Workstream {
                id: WorkstreamId::from_parts(["ws", "1"]),
                title: "Feature".into(),
                summary: None,
                tags: vec![],
                receipts: vec![EventId::from_parts(["pr", "1"])],
                events: vec![EventId::from_parts(["pr", "1"])],
                stats: WorkstreamStats {
                    pull_requests: 1,
                    reviews: 0,
                    manual_events: 0,
                },
            }],
        };
        let coverage = CoverageManifest {
            run_id: RunId::now("test"),
            generated_at: Utc::now(),
            user: "test".into(),
            window: TimeWindow {
                since: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                until: NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
            },
            mode: "test".into(),
            sources: vec!["github".into()],
            slices: vec![CoverageSlice {
                window: TimeWindow {
                    since: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                    until: NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
                },
                query: "test".into(),
                total_count: 100,
                fetched: 50,
                incomplete_results: Some(true),
                notes: vec![],
            }],
            warnings: vec!["API rate limit hit".into()],
            completeness: Completeness::Partial,
        };

        let result = renderer
            .render_packet_markdown("test", "2024", &events, &workstreams, &coverage)
            .unwrap();

        insta::assert_snapshot!(result);
    }
}
