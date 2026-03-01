//! Show how to implement a custom renderer using the `shiplog_ports::Renderer` trait.
//!
//! Shiplog's architecture is ports-and-adapters: you can swap in your own
//! renderer without touching the engine or the schema crates.
//!
//! Run from the workspace root:
//!   cargo run -p shiplog --example custom_renderer

use anyhow::{Context, Result};
use shiplog_ports::{Renderer, WorkstreamClusterer};
use shiplog_schema::coverage::CoverageManifest;
use shiplog_schema::event::{EventEnvelope, EventKind, EventPayload};
use shiplog_schema::workstream::WorkstreamsFile;

// ---------------------------------------------------------------------------
// Custom renderer implementation
// ---------------------------------------------------------------------------

/// A plain-text summary renderer.
///
/// This demonstrates the minimum you need to implement: a single method that
/// receives events, workstreams, and coverage metadata and returns a string.
struct PlainTextRenderer;

impl Renderer for PlainTextRenderer {
    fn render_packet_markdown(
        &self,
        user: &str,
        window_label: &str,
        events: &[EventEnvelope],
        workstreams: &WorkstreamsFile,
        coverage: &CoverageManifest,
    ) -> Result<String> {
        let mut out = String::new();

        // Header
        out.push_str(&format!("=== Shipping Packet for {user} ===\n"));
        out.push_str(&format!("Period: {window_label}\n"));
        out.push_str(&format!("Coverage: {:?}\n\n", coverage.completeness));

        // Workstream summary
        out.push_str(&format!(
            "Workstreams ({}):\n",
            workstreams.workstreams.len()
        ));
        for ws in &workstreams.workstreams {
            out.push_str(&format!(
                "  * {} — {} events, {} PRs, {} reviews\n",
                ws.title,
                ws.events.len(),
                ws.stats.pull_requests,
                ws.stats.reviews,
            ));
        }

        // Event breakdown
        let prs = events
            .iter()
            .filter(|e| matches!(e.kind, EventKind::PullRequest))
            .count();
        let reviews = events
            .iter()
            .filter(|e| matches!(e.kind, EventKind::Review))
            .count();
        out.push_str(&format!("\nEvent Breakdown ({} total):\n", events.len()));
        out.push_str(&format!("  Pull Requests: {prs}\n"));
        out.push_str(&format!("  Reviews:       {reviews}\n"));

        // Detailed listing
        out.push_str("\nDetailed Events:\n");
        for ev in events {
            let title = match &ev.payload {
                EventPayload::PullRequest(pr) => &pr.title,
                EventPayload::Review(r) => &r.pull_title,
                EventPayload::Manual(m) => &m.title,
            };
            out.push_str(&format!(
                "  [{:?}] {} — {}\n",
                ev.kind, ev.repo.full_name, title
            ));
        }

        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() -> Result<()> {
    // Load events and coverage from the fixture files.
    let events = load_fixture_events()?;
    let coverage = load_fixture_coverage()?;

    // Cluster events into workstreams using the built-in repo clusterer.
    let clusterer = shiplog_workstream_cluster::RepoClusterer;
    let workstreams = clusterer.cluster(&events).context("cluster events")?;

    // Render with our custom PlainTextRenderer.
    let custom = PlainTextRenderer;
    let custom_output = custom
        .render_packet_markdown(
            "octo",
            "2025-01-01..2025-04-01",
            &events,
            &workstreams,
            &coverage,
        )
        .context("render with custom renderer")?;

    println!("{custom_output}");

    // For comparison, render the same data with the built-in Markdown renderer.
    let md_renderer = shiplog_render_md::MarkdownRenderer::new();
    let md_output = md_renderer
        .render_packet_markdown(
            "octo",
            "2025-01-01..2025-04-01",
            &events,
            &workstreams,
            &coverage,
        )
        .context("render with built-in markdown renderer")?;

    println!("\n--- Built-in Markdown Renderer (first 30 lines) ---\n");
    for line in md_output.lines().take(30) {
        println!("{line}");
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------------

fn load_fixture_events() -> Result<Vec<EventEnvelope>> {
    let text = std::fs::read_to_string("examples/fixture/ledger.events.jsonl")
        .context("read fixture events")?;
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).context("parse event line"))
        .collect()
}

fn load_fixture_coverage() -> Result<CoverageManifest> {
    let text = std::fs::read_to_string("examples/fixture/coverage.manifest.json")
        .context("read fixture coverage")?;
    serde_json::from_str(&text).context("parse coverage manifest")
}
