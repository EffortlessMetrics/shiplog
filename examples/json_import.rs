//! Demonstrate importing events from JSONL fixtures, clustering into
//! workstreams, and rendering to Markdown using the shiplog engine.
//!
//! Run from the workspace root:
//!   cargo run -p shiplog --example json_import

use anyhow::{Context, Result};
use shiplog_engine::Engine;
use shiplog_ingest_json::JsonIngestor;
use shiplog_ports::Ingestor;
use shiplog_redact::DeterministicRedactor;
use shiplog_render_md::MarkdownRenderer;
use shiplog_schema::bundle::BundleProfile;
use shiplog_workstream_cluster::RepoClusterer;
use std::path::PathBuf;

fn main() -> Result<()> {
    // Step 1: Point the JSON ingestor at the fixture files.
    // In a real workflow `collect` writes these; here we use pre-built fixtures.
    let ingestor = JsonIngestor {
        events_path: PathBuf::from("examples/fixture/ledger.events.jsonl"),
        coverage_path: PathBuf::from("examples/fixture/coverage.manifest.json"),
    };

    // Step 2: Ingest — reads JSONL events + JSON coverage manifest.
    let ingest_output = ingestor.ingest().context("ingest fixture data")?;
    println!(
        "Ingested {} events for user '{}'",
        ingest_output.events.len(),
        ingest_output.coverage.user
    );

    // Step 3: Wire up the engine components.
    //  - MarkdownRenderer: built-in packet renderer
    //  - RepoClusterer:    groups events by repository
    //  - DeterministicRedactor: HMAC-based redaction for manager/public profiles
    let renderer = MarkdownRenderer::new();
    let clusterer = RepoClusterer;
    let redactor = DeterministicRedactor::new(b"example-key");
    let engine = Engine::new(&renderer, &clusterer, &redactor);

    // Step 4: Run the full pipeline (ingest → cluster → render → bundle).
    let out_dir = tempfile::tempdir().context("create temp dir")?;
    let (outputs, _ws_source) = engine
        .run(
            ingest_output,
            "octo",
            "2025-01-01..2025-04-01",
            out_dir.path(),
            false, // no zip archive
            &BundleProfile::Internal,
        )
        .context("run engine pipeline")?;

    // Step 5: Read and display the rendered packet.
    let packet = std::fs::read_to_string(&outputs.packet_md).context("read rendered packet")?;
    println!("\n--- Rendered Packet ---\n");
    println!("{packet}");

    println!("--- Output artifacts ---");
    println!("  packet.md:              {:?}", outputs.packet_md);
    println!(
        "  ledger.events.jsonl:    {:?}",
        outputs.ledger_events_jsonl
    );
    println!(
        "  coverage.manifest.json: {:?}",
        outputs.coverage_manifest_json
    );
    println!(
        "  bundle.manifest.json:   {:?}",
        outputs.bundle_manifest_json
    );
    println!("  workstreams yaml:       {:?}", outputs.workstreams_yaml);

    Ok(())
}
