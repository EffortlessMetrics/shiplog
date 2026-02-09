use anyhow::{Context, Result};
use shiplog_bundle::{write_bundle_manifest, write_zip};
use shiplog_ports::{IngestOutput, Redactor, Renderer, WorkstreamClusterer};
use shiplog_render_json::{write_coverage_manifest, write_events_jsonl};
use shiplog_schema::coverage::CoverageManifest;
use shiplog_schema::event::EventEnvelope;
use shiplog_schema::workstream::{WorkstreamsFile};
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

    pub fn run(
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

        let workstreams = self.clusterer.cluster(&events)?;

        // Write canonical outputs
        let ledger_path = out_dir.join("ledger.events.jsonl");
        let coverage_path = out_dir.join("coverage.manifest.json");
        let ws_path = out_dir.join("workstreams.yaml");
        let packet_path = out_dir.join("packet.md");

        write_events_jsonl(&ledger_path, &events)?;
        write_coverage_manifest(&coverage_path, &coverage)?;
        std::fs::write(&ws_path, serde_yaml::to_string(&workstreams)?)?;

        let packet = self
            .renderer
            .render_packet_markdown(user, window_label, &events, &workstreams, &coverage)?;
        std::fs::write(&packet_path, packet)?;

        // Render profiles
        self.render_profile("manager", user, window_label, out_dir, &events, &workstreams, &coverage)?;
        self.render_profile("public", user, window_label, out_dir, &events, &workstreams, &coverage)?;

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
