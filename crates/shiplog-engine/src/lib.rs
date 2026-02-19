//! Orchestration engine for the shiplog pipeline.
//!
//! Wires together ingestors, clusterers, redactors, and renderers to drive the
//! `collect`, `render`, `refresh`, and `run` commands. This is the main
//! coordination layer between the CLI and the microcrate adapters.

use anyhow::{Context, Result};
use shiplog_bundle::{write_bundle_manifest, write_zip};
use shiplog_ports::{IngestOutput, Redactor, Renderer, WorkstreamClusterer};
use shiplog_render_json::{write_coverage_manifest, write_events_jsonl};
use shiplog_schema::bundle::BundleProfile;
use shiplog_schema::coverage::CoverageManifest;
use shiplog_schema::event::EventEnvelope;
use shiplog_schema::workstream::WorkstreamsFile;
use shiplog_workstreams::WorkstreamManager;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Conflict resolution strategy for merging events
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolution {
    /// Prefer the event from the source with higher priority (earlier in list)
    PreferFirst,
    /// Prefer the event with the most recent timestamp
    PreferMostRecent,
    /// Prefer the event with more complete data (more non-null fields)
    PreferMostComplete,
}

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
        bundle_profile: &BundleProfile,
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
        let _bundle = write_bundle_manifest(out_dir, run_id, bundle_profile)?;
        let zip_path = if zip {
            let z = zip_path_for_profile(out_dir, bundle_profile);
            write_zip(out_dir, &z, bundle_profile)?;
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

    /// Import a pre-built ledger and run the full render pipeline.
    ///
    /// When `workstreams` is `Some`, uses them directly (writes as curated).
    /// When `None`, falls through to normal clustering.
    #[allow(clippy::too_many_arguments)]
    pub fn import(
        &self,
        ingest: IngestOutput,
        user: &str,
        window_label: &str,
        out_dir: &Path,
        zip: bool,
        workstreams: Option<WorkstreamsFile>,
        bundle_profile: &BundleProfile,
    ) -> Result<(RunOutputs, WorkstreamSource)> {
        std::fs::create_dir_all(out_dir).with_context(|| format!("create {out_dir:?}"))?;

        let events = ingest.events;
        let coverage = ingest.coverage;

        // Use provided workstreams or generate new ones
        let (ws, ws_source) = if let Some(ws) = workstreams {
            // Write imported workstreams as curated
            let curated_path = WorkstreamManager::curated_path(out_dir);
            shiplog_workstreams::write_workstreams(&curated_path, &ws)?;
            (ws, WorkstreamSource::Curated)
        } else {
            self.load_workstreams(out_dir, &events)?
        };

        // Write canonical outputs
        let ledger_path = out_dir.join("ledger.events.jsonl");
        let coverage_path = out_dir.join("coverage.manifest.json");
        let packet_path = out_dir.join("packet.md");

        write_events_jsonl(&ledger_path, &events)?;
        write_coverage_manifest(&coverage_path, &coverage)?;

        let ws_path = match ws_source {
            WorkstreamSource::Curated => WorkstreamManager::curated_path(out_dir),
            WorkstreamSource::Suggested => WorkstreamManager::suggested_path(out_dir),
            WorkstreamSource::Generated => WorkstreamManager::suggested_path(out_dir),
        };

        let packet =
            self.renderer
                .render_packet_markdown(user, window_label, &events, &ws, &coverage)?;
        std::fs::write(&packet_path, packet)?;

        // Render profiles
        self.render_profile(
            "manager",
            user,
            window_label,
            out_dir,
            &events,
            &ws,
            &coverage,
        )?;
        self.render_profile(
            "public",
            user,
            window_label,
            out_dir,
            &events,
            &ws,
            &coverage,
        )?;

        // Bundle manifest + zip
        let run_id = &coverage.run_id;
        let _bundle = write_bundle_manifest(out_dir, run_id, bundle_profile)?;
        let zip_path = if zip {
            let z = zip_path_for_profile(out_dir, bundle_profile);
            write_zip(out_dir, &z, bundle_profile)?;
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
        bundle_profile: &BundleProfile,
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
        let _bundle = write_bundle_manifest(out_dir, run_id, bundle_profile)?;
        let zip_path = if zip {
            let z = zip_path_for_profile(out_dir, bundle_profile);
            write_zip(out_dir, &z, bundle_profile)?;
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

    /// Merge events from multiple sources with deduplication and conflict resolution.
    ///
    /// This function:
    /// - Deduplicates events by ID
    /// - Resolves conflicts for events that appear in multiple sources
    /// - Merges coverage manifests from all sources
    /// - Sorts events by timestamp
    pub fn merge(
        &self,
        ingest_outputs: Vec<IngestOutput>,
        resolution: ConflictResolution,
    ) -> Result<IngestOutput> {
        if ingest_outputs.is_empty() {
            return Err(anyhow::anyhow!("No ingest outputs to merge"));
        }

        // Collect all events and build a map for deduplication
        let mut event_map: HashMap<String, Vec<EventEnvelope>> = HashMap::new();
        let mut all_sources: Vec<String> = Vec::new();
        let mut all_warnings: Vec<String> = Vec::new();
        let mut all_slices: Vec<shiplog_schema::coverage::CoverageSlice> = Vec::new();

        // Use first output's window and user as base
        let base_output = &ingest_outputs[0];
        let window = base_output.coverage.window.clone();
        let user = base_output.coverage.user.clone();

        for ingest in &ingest_outputs {
            // Collect events by ID
            for event in &ingest.events {
                let id = event.id.to_string();
                event_map.entry(id).or_default().push(event.clone());
            }

            // Collect sources
            all_sources.extend(ingest.coverage.sources.clone());
            all_warnings.extend(ingest.coverage.warnings.clone());
            all_slices.extend(ingest.coverage.slices.clone());
        }

        // Deduplicate and resolve conflicts
        let mut merged_events: Vec<EventEnvelope> = Vec::new();
        let mut conflict_count = 0;

        for (_id, mut events) in event_map {
            if events.len() == 1 {
                merged_events.push(events.remove(0));
            } else {
                // Multiple events with same ID - resolve conflict
                conflict_count += 1;
                let resolved = self.resolve_conflict(&events, resolution);
                merged_events.push(resolved);
            }
        }

        // Sort by timestamp
        merged_events.sort_by_key(|e| e.occurred_at);

        // Deduplicate sources
        all_sources.sort();
        all_sources.dedup();

        // Calculate completeness - if any source is partial, result is partial
        let completeness = if ingest_outputs
            .iter()
            .any(|o| o.coverage.completeness == shiplog_schema::coverage::Completeness::Partial)
        {
            shiplog_schema::coverage::Completeness::Partial
        } else {
            shiplog_schema::coverage::Completeness::Complete
        };

        // Add warning about conflicts if any occurred
        if conflict_count > 0 {
            all_warnings.push(format!(
                "Resolved {} conflict(s) during merge using {:?} strategy",
                conflict_count, resolution
            ));
        }

        // Create merged coverage manifest
        let coverage = shiplog_schema::coverage::CoverageManifest {
            run_id: shiplog_ids::RunId::now("merge"),
            generated_at: chrono::Utc::now(),
            user,
            window,
            mode: "merged".to_string(),
            sources: all_sources,
            slices: all_slices,
            warnings: all_warnings,
            completeness,
        };

        Ok(IngestOutput {
            events: merged_events,
            coverage,
        })
    }

    /// Resolve a conflict between multiple events with same ID
    fn resolve_conflict(
        &self,
        events: &[EventEnvelope],
        resolution: ConflictResolution,
    ) -> EventEnvelope {
        match resolution {
            ConflictResolution::PreferFirst => events[0].clone(),
            ConflictResolution::PreferMostRecent => {
                events.iter().max_by_key(|e| e.occurred_at).unwrap().clone()
            }
            ConflictResolution::PreferMostComplete => events
                .iter()
                .max_by_key(|e| self.completeness_score(e))
                .unwrap()
                .clone(),
        }
    }

    /// Calculate a completeness score for an event (higher = more complete)
    fn completeness_score(&self, event: &EventEnvelope) -> usize {
        let mut score = 0;

        // Check for non-empty fields
        if !event.actor.login.is_empty() {
            score += 1;
        }
        if event.actor.id.is_some() {
            score += 1;
        }
        if !event.repo.full_name.is_empty() {
            score += 1;
        }
        if event.repo.html_url.is_some() {
            score += 1;
        }
        if !event.tags.is_empty() {
            score += 1;
        }
        if !event.links.is_empty() {
            score += 1;
        }
        if event.source.url.is_some() {
            score += 1;
        }
        if event.source.opaque_id.is_some() {
            score += 1;
        }

        // Check payload completeness
        match &event.payload {
            shiplog_schema::event::EventPayload::PullRequest(pr) => {
                if pr.additions.is_some() {
                    score += 1;
                }
                if pr.deletions.is_some() {
                    score += 1;
                }
                if pr.changed_files.is_some() {
                    score += 1;
                }
                if pr.merged_at.is_some() {
                    score += 1;
                }
            }
            shiplog_schema::event::EventPayload::Manual(manual) => {
                if manual.description.is_some() {
                    score += 1;
                }
                if manual.started_at.is_some() {
                    score += 1;
                }
                if manual.ended_at.is_some() {
                    score += 1;
                }
                if manual.impact.is_some() {
                    score += 1;
                }
            }
            _ => {}
        }

        score
    }
}

/// Compute the zip file path based on bundle profile.
/// `Internal` -> `<run_dir>.zip`, others -> `<run_dir>.<profile>.zip`.
fn zip_path_for_profile(out_dir: &Path, profile: &BundleProfile) -> PathBuf {
    match profile {
        BundleProfile::Internal => out_dir.with_extension("zip"),
        _ => {
            let stem = out_dir.file_name().unwrap_or_default().to_string_lossy();
            let name = format!("{}.{}.zip", stem, profile.as_str());
            out_dir.with_file_name(name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, TimeZone, Utc};
    use shiplog_ids::{EventId, RunId};
    use shiplog_ports::IngestOutput;
    use shiplog_schema::coverage::{Completeness, CoverageManifest, TimeWindow};
    use shiplog_schema::event::*;

    fn pr_event(repo: &str, number: u64, title: &str) -> EventEnvelope {
        EventEnvelope {
            id: EventId::from_parts(["github", "pr", repo, &number.to_string()]),
            kind: EventKind::PullRequest,
            occurred_at: Utc.timestamp_opt(0, 0).unwrap(),
            actor: Actor {
                login: "user".into(),
                id: None,
            },
            repo: RepoRef {
                full_name: repo.to_string(),
                html_url: Some(format!("https://github.com/{repo}")),
                visibility: RepoVisibility::Unknown,
            },
            payload: EventPayload::PullRequest(PullRequestEvent {
                number,
                title: title.to_string(),
                state: PullRequestState::Merged,
                created_at: Utc.timestamp_opt(0, 0).unwrap(),
                merged_at: Some(Utc.timestamp_opt(0, 0).unwrap()),
                additions: Some(1),
                deletions: Some(0),
                changed_files: Some(1),
                touched_paths_hint: vec![],
                window: Some(TimeWindow {
                    since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                    until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
                }),
            }),
            tags: vec![],
            links: vec![Link {
                label: "pr".into(),
                url: format!("https://github.com/{repo}/pull/{number}"),
            }],
            source: SourceRef {
                system: SourceSystem::Github,
                url: Some("https://api.github.com/...".into()),
                opaque_id: None,
            },
        }
    }

    fn test_ingest() -> IngestOutput {
        let events = vec![
            pr_event("acme/foo", 1, "Add feature"),
            pr_event("acme/foo", 2, "Fix bug"),
        ];
        let coverage = CoverageManifest {
            run_id: RunId("test_run_1".into()),
            generated_at: Utc.timestamp_opt(0, 0).unwrap(),
            user: "tester".into(),
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
        IngestOutput { events, coverage }
    }

    fn test_engine() -> Engine<'static> {
        let renderer: &'static dyn shiplog_ports::Renderer =
            Box::leak(Box::new(shiplog_render_md::MarkdownRenderer::default()));
        let clusterer: &'static dyn shiplog_ports::WorkstreamClusterer =
            Box::leak(Box::new(shiplog_workstreams::RepoClusterer));
        let redactor: &'static dyn shiplog_ports::Redactor = Box::leak(Box::new(
            shiplog_redact::DeterministicRedactor::new(b"test-key"),
        ));
        Engine::new(renderer, clusterer, redactor)
    }

    #[test]
    fn run_creates_expected_output_files() {
        let dir = tempfile::tempdir().unwrap();
        let out_dir = dir.path().join("test_run_1");

        let engine = test_engine();
        let ingest = test_ingest();

        let (outputs, _) = engine
            .run(
                ingest,
                "tester",
                "2025-01-01..2025-02-01",
                &out_dir,
                false,
                &BundleProfile::Internal,
            )
            .unwrap();

        assert!(outputs.packet_md.exists(), "packet.md missing");
        assert!(
            outputs.ledger_events_jsonl.exists(),
            "ledger.events.jsonl missing"
        );
        assert!(
            outputs.coverage_manifest_json.exists(),
            "coverage.manifest.json missing"
        );
        assert!(
            outputs.bundle_manifest_json.exists(),
            "bundle.manifest.json missing"
        );
        assert!(
            out_dir.join("profiles/manager/packet.md").exists(),
            "manager profile missing"
        );
        assert!(
            out_dir.join("profiles/public/packet.md").exists(),
            "public profile missing"
        );
    }

    #[test]
    fn run_with_zip_creates_archive() {
        let dir = tempfile::tempdir().unwrap();
        let out_dir = dir.path().join("test_run_zip");

        let engine = test_engine();
        let ingest = test_ingest();

        let (outputs, _) = engine
            .run(
                ingest,
                "tester",
                "2025-01-01..2025-02-01",
                &out_dir,
                true,
                &BundleProfile::Internal,
            )
            .unwrap();

        assert!(
            outputs.zip_path.is_some(),
            "zip_path should be Some when zip=true"
        );
        assert!(
            outputs.zip_path.as_ref().unwrap().exists(),
            "zip file missing"
        );
    }

    #[test]
    fn zip_path_internal_uses_plain_extension() {
        let p = zip_path_for_profile(Path::new("/tmp/run_123"), &BundleProfile::Internal);
        assert_eq!(p, Path::new("/tmp/run_123.zip"));
    }

    #[test]
    fn zip_path_manager_includes_profile_name() {
        let p = zip_path_for_profile(Path::new("/tmp/run_123"), &BundleProfile::Manager);
        assert_eq!(p, Path::new("/tmp/run_123.manager.zip"));
    }
}
