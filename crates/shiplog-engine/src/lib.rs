//! Orchestration engine for the shiplog pipeline.
//!
//! Wires together ingestors, clusterers, redactors, and renderers to drive the
//! `collect`, `render`, `refresh`, and `run` commands. This is the main
//! coordination layer between the CLI and the microcrate adapters.

use anyhow::{Context, Result};
use shiplog_bundle::{write_bundle_manifest, write_zip};
use shiplog_ports::{IngestOutput, Redactor, Renderer, WorkstreamClusterer};
use shiplog_render_json::{write_coverage_manifest, write_events_jsonl};
use shiplog_schema::coverage::CoverageManifest;
use shiplog_schema::event::EventEnvelope;
use shiplog_schema::workstream::WorkstreamsFile;
use shiplog_workstreams::WorkstreamManager;
use std::path::{Path, PathBuf};

pub struct Engine<'a> {
    pub renderer: &'a dyn Renderer,
    pub clusterer: &'a dyn WorkstreamClusterer,
    pub redactor: &'a dyn Redactor,
}

pub struct RunOutputs {
    pub out_dir: PathBuf,
    pub packet_md: PathBuf,
    pub workstreams_yaml: PathBuf,
    pub ledger_events_jsonl: PathBuf,
    pub coverage_manifest_json: PathBuf,
    pub bundle_manifest_json: PathBuf,
    pub zip_path: Option<PathBuf>,
}

/// What type of workstream file was used/created
pub enum WorkstreamSource {
    /// User-curated workstreams.yaml
    Curated,
    /// Machine-generated workstreams.suggested.yaml
    Suggested,
    /// Newly generated from events
    Generated,
}

impl<'a> Engine<'a> {
    pub fn new(
        renderer: &'a dyn Renderer,
        clusterer: &'a dyn WorkstreamClusterer,
        redactor: &'a dyn Redactor,
    ) -> Self {
        Self {
            renderer,
            clusterer,
            redactor,
        }
    }

    /// Run the full pipeline: ingest → cluster → render
    ///
    /// Uses WorkstreamManager to respect user-curated workstreams.
    pub fn run(
        &self,
        ingest: IngestOutput,
        user: &str,
        window_label: &str,
        out_dir: &Path,
        zip: bool,
    ) -> Result<(RunOutputs, WorkstreamSource)> {
        std::fs::create_dir_all(out_dir).with_context(|| format!("create {out_dir:?}"))?;

        let events = ingest.events;
        let coverage = ingest.coverage;

        // Use WorkstreamManager to load or generate workstreams
        let (workstreams, ws_source) = self.load_workstreams(out_dir, &events)?;

        // Write canonical outputs
        let ledger_path = out_dir.join("ledger.events.jsonl");
        let coverage_path = out_dir.join("coverage.manifest.json");
        let packet_path = out_dir.join("packet.md");

        write_events_jsonl(&ledger_path, &events)?;
        write_coverage_manifest(&coverage_path, &coverage)?;

        // Note: workstreams.yaml is user-owned; we don't overwrite it
        // workstreams.suggested.yaml is already written by WorkstreamManager if needed
        let ws_path = match ws_source {
            WorkstreamSource::Curated => WorkstreamManager::curated_path(out_dir),
            WorkstreamSource::Suggested => WorkstreamManager::suggested_path(out_dir),
            WorkstreamSource::Generated => WorkstreamManager::suggested_path(out_dir),
        };

        let packet = self.renderer.render_packet_markdown(
            user,
            window_label,
            &events,
            &workstreams,
            &coverage,
        )?;
        std::fs::write(&packet_path, packet)?;

        // Render profiles
        self.render_profile(
            "manager",
            user,
            window_label,
            out_dir,
            &events,
            &workstreams,
            &coverage,
        )?;
        self.render_profile(
            "public",
            user,
            window_label,
            out_dir,
            &events,
            &workstreams,
            &coverage,
        )?;

        // Bundle manifest + zip
        let run_id = &coverage.run_id;
        let _bundle = write_bundle_manifest(out_dir, run_id)?;
        let zip_path = if zip {
            let z = out_dir.with_extension("zip");
            write_zip(out_dir, &z)?;
            Some(z)
        } else {
            None
        };

        Ok((
            RunOutputs {
                out_dir: out_dir.to_path_buf(),
                packet_md: packet_path,
                workstreams_yaml: ws_path,
                ledger_events_jsonl: ledger_path,
                coverage_manifest_json: coverage_path,
                bundle_manifest_json: out_dir.join("bundle.manifest.json"),
                zip_path,
            },
            ws_source,
        ))
    }

    /// Load workstreams using WorkstreamManager
    fn load_workstreams(
        &self,
        out_dir: &Path,
        events: &[EventEnvelope],
    ) -> Result<(WorkstreamsFile, WorkstreamSource)> {
        let curated_exists = WorkstreamManager::has_curated(out_dir);
        let suggested_exists = WorkstreamManager::suggested_path(out_dir).exists();

        let ws = WorkstreamManager::load_effective(out_dir, self.clusterer, events)?;

        let source = if curated_exists {
            WorkstreamSource::Curated
        } else if suggested_exists {
            WorkstreamSource::Suggested
        } else {
            WorkstreamSource::Generated
        };

        Ok((ws, source))
    }

    /// Refresh receipts and stats without regenerating workstreams
    ///
    /// This preserves user curation while updating event data.
    pub fn refresh(
        &self,
        ingest: IngestOutput,
        user: &str,
        window_label: &str,
        out_dir: &Path,
        zip: bool,
    ) -> Result<RunOutputs> {
        std::fs::create_dir_all(out_dir).with_context(|| format!("create {out_dir:?}"))?;

        let events = ingest.events;
        let coverage = ingest.coverage;

        // Load existing workstreams — error if none exist
        let workstreams = if WorkstreamManager::has_curated(out_dir) {
            let path = WorkstreamManager::curated_path(out_dir);
            let text = std::fs::read_to_string(&path)
                .with_context(|| format!("read curated workstreams from {path:?}"))?;
            serde_yaml::from_str(&text)
                .with_context(|| format!("parse curated workstreams yaml {path:?}"))?
        } else {
            let suggested_path = WorkstreamManager::suggested_path(out_dir);
            if suggested_path.exists() {
                let text = std::fs::read_to_string(&suggested_path).with_context(|| {
                    format!("read suggested workstreams from {suggested_path:?}")
                })?;
                serde_yaml::from_str(&text).with_context(|| {
                    format!("parse suggested workstreams yaml {suggested_path:?}")
                })?
            } else {
                anyhow::bail!(
                    "No workstreams found. Run `shiplog collect` first to generate workstreams."
                );
            }
        };

        // Write canonical outputs
        let ledger_path = out_dir.join("ledger.events.jsonl");
        let coverage_path = out_dir.join("coverage.manifest.json");
        let packet_path = out_dir.join("packet.md");

        write_events_jsonl(&ledger_path, &events)?;
        write_coverage_manifest(&coverage_path, &coverage)?;

        let ws_path = if WorkstreamManager::has_curated(out_dir) {
            WorkstreamManager::curated_path(out_dir)
        } else {
            WorkstreamManager::suggested_path(out_dir)
        };

        let packet = self.renderer.render_packet_markdown(
            user,
            window_label,
            &events,
            &workstreams,
            &coverage,
        )?;
        std::fs::write(&packet_path, packet)?;

        // Render profiles
        self.render_profile(
            "manager",
            user,
            window_label,
            out_dir,
            &events,
            &workstreams,
            &coverage,
        )?;
        self.render_profile(
            "public",
            user,
            window_label,
            out_dir,
            &events,
            &workstreams,
            &coverage,
        )?;

        // Bundle manifest + zip
        let run_id = &coverage.run_id;
        let _bundle = write_bundle_manifest(out_dir, run_id)?;
        let zip_path = if zip {
            let z = out_dir.with_extension("zip");
            write_zip(out_dir, &z)?;
            Some(z)
        } else {
            None
        };

        Ok(RunOutputs {
            out_dir: out_dir.to_path_buf(),
            packet_md: packet_path,
            workstreams_yaml: ws_path,
            ledger_events_jsonl: ledger_path,
            coverage_manifest_json: coverage_path,
            bundle_manifest_json: out_dir.join("bundle.manifest.json"),
            zip_path,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn render_profile(
        &self,
        profile: &str,
        user: &str,
        window_label: &str,
        out_dir: &Path,
        events: &[EventEnvelope],
        workstreams: &WorkstreamsFile,
        coverage: &CoverageManifest,
    ) -> Result<()> {
        let prof_dir = out_dir.join("profiles").join(profile);
        std::fs::create_dir_all(&prof_dir)?;

        let red_events = self.redactor.redact_events(events, profile)?;
        let red_ws = self.redactor.redact_workstreams(workstreams, profile)?;

        let md = self.renderer.render_packet_markdown(
            user,
            window_label,
            &red_events,
            &red_ws,
            coverage,
        )?;
        std::fs::write(prof_dir.join("packet.md"), md)?;
        Ok(())
    }
}
