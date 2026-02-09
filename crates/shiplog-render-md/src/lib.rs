use anyhow::Result;
use itertools::Itertools;
use shiplog_ports::Renderer;
use shiplog_schema::coverage::CoverageManifest;
use shiplog_schema::event::{EventEnvelope, EventKind, EventPayload};
use shiplog_schema::workstream::WorkstreamsFile;
use std::collections::HashMap;

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

        out.push_str("## Coverage\n\n");
        out.push_str(&format!("- Completeness: **{:?}**\n", coverage.completeness));
        if !coverage.warnings.is_empty() {
            out.push_str("- Warnings:\n");
            for w in &coverage.warnings {
                out.push_str(&format!("  - {w}\n"));
            }
        }
        out.push_str("\n");
        out.push_str(&format!(
            "- Slices executed: {}\n",
            coverage.slices.len()
        ));
        out.push_str("\n");

        out.push_str("## Summary\n\n");
        out.push_str(&format!(
            "- Workstreams: {}\n",
            workstreams.workstreams.len()
        ));
        out.push_str(&format!("- Events: {}\n", events.len()));
        out.push_str("\n");

        out.push_str("## Workstreams\n\n");

        let by_id: HashMap<String, &EventEnvelope> =
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

            out.push_str("**Receipts**\n\n");
            if ws.receipts.is_empty() {
                out.push_str("- (none)\n\n");
            } else {
                for id in &ws.receipts {
                    if let Some(ev) = by_id.get(&id.0) {
                        out.push_str(&format!("- {}\n", format_receipt(ev)));
                    }
                }
                out.push_str("\n");
            }

            // A small stats line keeps the packet honest without becoming a scoreboard.
            out.push_str(&format!(
                "_PRs: {}, Reviews: {}_\n\n",
                ws.stats.pull_requests, ws.stats.reviews
            ));
        }

        out.push_str("## Appendix\n\n");
        out.push_str("- `ledger.events.jsonl` (canonical events)\n");
        out.push_str("- `coverage.manifest.json` (completeness + slicing)\n");
        out.push_str("- `workstreams.yaml` (editable clustering)\n");

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
        _ => format!("event {}", ev.id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shiplog_schema::coverage::*;
    use shiplog_schema::event::*;
    use shiplog_schema::workstream::*;
    use shiplog_ids::{EventId, RunId, WorkstreamId};
    use chrono::{NaiveDate, TimeZone, Utc};

    #[test]
    fn packet_snapshot_is_stable() {
        let ev = EventEnvelope {
            id: EventId::from_parts(["pr", "1"]),
            kind: EventKind::PullRequest,
            occurred_at: Utc.timestamp_opt(0, 0).unwrap(),
            actor: Actor { login: "octo".into(), id: None },
            repo: RepoRef { full_name: "o/r".into(), html_url: Some("https://github.com/o/r".into()), visibility: RepoVisibility::Public },
            payload: EventPayload::PullRequest(PullRequestEvent {
                number: 1,
                title: "Add thing".into(),
                state: PullRequestState::Merged,
                created_at: Utc.timestamp_opt(0,0).unwrap(),
                merged_at: Some(Utc.timestamp_opt(0,0).unwrap()),
                additions: Some(10),
                deletions: Some(2),
                changed_files: Some(1),
                touched_paths_hint: vec![],
                window: None,
            }),
            tags: vec![],
            links: vec![Link { label: "pr".into(), url: "https://github.com/o/r/pull/1".into() }],
            source: SourceRef { system: SourceSystem::Github, url: Some("https://api.github.com/...".into()), opaque_id: None },
        };

        let ws = WorkstreamsFile {
            version: 1,
            generated_at: Utc.timestamp_opt(0,0).unwrap(),
            workstreams: vec![Workstream {
                id: WorkstreamId::from_parts(["repo","o/r"]),
                title: "o/r".into(),
                summary: None,
                tags: vec![],
                stats: WorkstreamStats { pull_requests: 1, reviews: 0 },
                events: vec![ev.id.clone()],
                receipts: vec![ev.id.clone()],
            }],
        };

        let cov = CoverageManifest {
            run_id: RunId("run_0".into()),
            generated_at: Utc.timestamp_opt(0,0).unwrap(),
            user: "octo".into(),
            window: TimeWindow {
                since: NaiveDate::from_ymd_opt(2025,1,1).unwrap(),
                until: NaiveDate::from_ymd_opt(2025,2,1).unwrap(),
            },
            mode: "merged".into(),
            sources: vec!["github".into()],
            slices: vec![CoverageSlice {
                window: TimeWindow {
                    since: NaiveDate::from_ymd_opt(2025,1,1).unwrap(),
                    until: NaiveDate::from_ymd_opt(2025,2,1).unwrap(),
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

        insta::assert_snapshot!(md, @r###"# Self review packet: octo

**Window:** 2025-01-01..2025-02-01

## Coverage

- Completeness: **Complete**

- Slices executed: 1

## Summary

- Workstreams: 1
- Events: 1

## Workstreams

### o/r

**Claim scaffolds**

- Problem: _fill_
- What I shipped: _fill_
- Why it mattered: _fill_
- Result: _fill_

**Receipts**

- [o/r#1](https://github.com/o/r/pull/1) — Add thing

_PRs: 1, Reviews: 0_

## Appendix

- `ledger.events.jsonl` (canonical events)
- `coverage.manifest.json` (completeness + slicing)
- `workstreams.yaml` (editable clustering)
"###);
    }
}
