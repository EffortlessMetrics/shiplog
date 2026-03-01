#![warn(missing_docs)]
//! Port trait definitions for the shiplog pipeline.
//!
//! Defines the four core abstractions: [`Ingestor`] (data collection),
//! [`WorkstreamClusterer`] (event grouping), [`Renderer`] (output generation),
//! and [`Redactor`] (privacy-aware projection). Adapters depend on ports;
//! ports never depend on adapters.

use anyhow::Result;
use shiplog_schema::coverage::CoverageManifest;
use shiplog_schema::event::EventEnvelope;
use shiplog_schema::workstream::WorkstreamsFile;

/// Output of an ingestion run.
///
/// The tool treats these as immutable receipts.
///
/// # Examples
///
/// ```
/// use shiplog_ports::IngestOutput;
/// use shiplog_schema::coverage::{CoverageManifest, Completeness, TimeWindow};
/// use chrono::{NaiveDate, Utc};
/// use shiplog_ids::RunId;
///
/// let output = IngestOutput {
///     events: vec![],
///     coverage: CoverageManifest {
///         run_id: RunId::now("test"),
///         generated_at: Utc::now(),
///         user: "octocat".into(),
///         window: TimeWindow {
///             since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
///             until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
///         },
///         mode: "merged".into(),
///         sources: vec!["github".into()],
///         slices: vec![],
///         warnings: vec![],
///         completeness: Completeness::Complete,
///     },
/// };
/// assert!(output.events.is_empty());
/// ```
#[derive(Clone, Debug)]
pub struct IngestOutput {
    /// The collected event envelopes.
    pub events: Vec<EventEnvelope>,
    /// Coverage manifest describing what was queried and fetched.
    pub coverage: CoverageManifest,
}

/// Basic ingestion trait.
///
/// Adapters live in `shiplog-ingest-*` crates.
///
/// # Examples
///
/// ```rust,no_run
/// use shiplog_ports::{Ingestor, IngestOutput};
/// use anyhow::Result;
///
/// struct MyIngestor;
///
/// impl Ingestor for MyIngestor {
///     fn ingest(&self) -> Result<IngestOutput> {
///         todo!("fetch events from your source")
///     }
/// }
/// ```
pub trait Ingestor {
    /// Fetch events from the data source and return them with coverage metadata.
    fn ingest(&self) -> Result<IngestOutput>;
}

/// Workstream clustering.
///
/// This is intentionally a port so the default clustering can be swapped without rewriting the app.
///
/// # Examples
///
/// ```rust,no_run
/// use shiplog_ports::WorkstreamClusterer;
/// use shiplog_schema::event::EventEnvelope;
/// use shiplog_schema::workstream::WorkstreamsFile;
/// use anyhow::Result;
///
/// struct RepoClusterer;
///
/// impl WorkstreamClusterer for RepoClusterer {
///     fn cluster(&self, events: &[EventEnvelope]) -> Result<WorkstreamsFile> {
///         todo!("group events by repository")
///     }
/// }
/// ```
pub trait WorkstreamClusterer {
    /// Group events into workstreams and return the resulting file.
    fn cluster(&self, events: &[EventEnvelope]) -> Result<WorkstreamsFile>;
}

/// Rendering.
///
/// Renderers should be pure: input in, bytes out.
///
/// # Examples
///
/// ```rust,no_run
/// use shiplog_ports::Renderer;
/// use shiplog_schema::event::EventEnvelope;
/// use shiplog_schema::workstream::WorkstreamsFile;
/// use shiplog_schema::coverage::CoverageManifest;
/// use anyhow::Result;
///
/// struct MarkdownRenderer;
///
/// impl Renderer for MarkdownRenderer {
///     fn render_packet_markdown(
///         &self,
///         user: &str,
///         window_label: &str,
///         events: &[EventEnvelope],
///         workstreams: &WorkstreamsFile,
///         coverage: &CoverageManifest,
///     ) -> Result<String> {
///         Ok(format!("# Packet for {user}\n"))
///     }
/// }
/// ```
pub trait Renderer {
    /// Render a Markdown shipping packet from the given events and metadata.
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
///
/// # Examples
///
/// ```rust,no_run
/// use shiplog_ports::Redactor;
/// use shiplog_schema::event::EventEnvelope;
/// use shiplog_schema::workstream::WorkstreamsFile;
/// use anyhow::Result;
///
/// struct NoOpRedactor;
///
/// impl Redactor for NoOpRedactor {
///     fn redact_events(&self, events: &[EventEnvelope], _profile: &str) -> Result<Vec<EventEnvelope>> {
///         Ok(events.to_vec())
///     }
///     fn redact_workstreams(&self, ws: &WorkstreamsFile, _profile: &str) -> Result<WorkstreamsFile> {
///         Ok(ws.clone())
///     }
/// }
/// ```
pub trait Redactor {
    /// Apply a redaction profile to events, returning redacted copies.
    fn redact_events(&self, events: &[EventEnvelope], profile: &str) -> Result<Vec<EventEnvelope>>;
    /// Apply a redaction profile to workstreams, returning redacted copies.
    fn redact_workstreams(
        &self,
        workstreams: &WorkstreamsFile,
        profile: &str,
    ) -> Result<WorkstreamsFile>;
}
