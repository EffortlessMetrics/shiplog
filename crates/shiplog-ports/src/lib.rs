use anyhow::Result;
use shiplog_schema::coverage::CoverageManifest;
use shiplog_schema::event::EventEnvelope;
use shiplog_schema::workstream::{Workstream, WorkstreamsFile};

/// Output of an ingestion run.
///
/// The tool treats these as immutable receipts.
#[derive(Clone, Debug)]
pub struct IngestOutput {
    pub events: Vec<EventEnvelope>,
    pub coverage: CoverageManifest,
}

/// Basic ingestion trait.
///
/// Adapters live in `shiplog-ingest-*` crates.
pub trait Ingestor {
    fn ingest(&self) -> Result<IngestOutput>;
}

/// Workstream clustering.
///
/// This is intentionally a port so the default clustering can be swapped without rewriting the app.
pub trait WorkstreamClusterer {
    fn cluster(&self, events: &[EventEnvelope]) -> Result<WorkstreamsFile>;
}

/// Rendering.
///
/// Renderers should be pure: input in, bytes out.
pub trait Renderer {
    fn render_packet_markdown(
        &self,
        user: &str,
        window_label: &str,
        events: &[EventEnvelope],
        workstreams: &WorkstreamsFile,
        coverage: &CoverageManifest,
    ) -> Result<String>;
}

/// Redaction.
///
/// Redaction is a rendering mode. Same underlying ledger, different projections.
pub trait Redactor {
    fn redact_events(&self, events: &[EventEnvelope], profile: &str) -> Result<Vec<EventEnvelope>>;
    fn redact_workstreams(&self, workstreams: &WorkstreamsFile, profile: &str)
        -> Result<WorkstreamsFile>;
}
