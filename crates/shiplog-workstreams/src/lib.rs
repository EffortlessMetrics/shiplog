//! Repository-based event clustering and user-curated workstream management.
//!
//! Groups ingested events into workstreams (default: by repository) and manages
//! the two-file workflow: auto-generated `workstreams.suggested.yaml` and
//! user-curated `workstreams.yaml` that is never overwritten.

use anyhow::{Context, Result};
use chrono::Utc;
use shiplog_ids::WorkstreamId;
use shiplog_ports::WorkstreamClusterer;
use shiplog_schema::event::{EventEnvelope, EventKind};
use shiplog_schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};
use std::collections::BTreeMap;
use std::path::Path;

/// Default clustering strategy:
/// - group by repo
///
/// It's intentionally boring. It gives the user a stable starting point.
/// A later LLM-assisted clusterer can sit behind the same port.
pub struct RepoClusterer;

impl WorkstreamClusterer for RepoClusterer {
    fn cluster(&self, events: &[EventEnvelope]) -> Result<WorkstreamsFile> {
        let mut by_repo: BTreeMap<String, Vec<&EventEnvelope>> = BTreeMap::new();
        for ev in events {
            by_repo
                .entry(ev.repo.full_name.clone())
                .or_default()
                .push(ev);
        }

        let mut workstreams = Vec::new();
        for (repo, evs) in by_repo {
            let id = WorkstreamId::from_parts(["repo", &repo]);
            let mut ws = Workstream {
                id,
                title: repo.clone(),
                summary: None,
                tags: vec!["repo".to_string()],
                stats: WorkstreamStats::zero(),
                events: vec![],
                receipts: vec![],
            };

            for ev in evs {
                ws.events.push(ev.id.clone());
                ws.bump_stats(&ev.kind);

                // simple receipt heuristic: keep PRs, manual events, plus reviews on top repos.
                match ev.kind {
                    EventKind::PullRequest => ws.receipts.push(ev.id.clone()),
                    EventKind::Review => {
                        if ws.receipts.len() < 5 {
                            ws.receipts.push(ev.id.clone())
                        }
                    }
                    EventKind::Manual => {
                        // Manual events are high-value receipts
                        if ws.receipts.len() < 7 {
                            ws.receipts.push(ev.id.clone())
                        }
                    }
                }
            }

            // avoid enormous receipt lists in the packet.
            ws.receipts.truncate(10);

            workstreams.push(ws);
        }

        Ok(WorkstreamsFile {
            version: 1,
            generated_at: Utc::now(),
            workstreams,
        })
    }
}

/// Load an existing workstreams.yaml if present, otherwise generate.
///
/// This is a critical design point: workstreams are user-owned.
/// The tool can suggest structure, but shouldn't trap the user.
pub fn load_or_cluster(
    maybe_yaml: Option<&Path>,
    clusterer: &dyn WorkstreamClusterer,
    events: &[EventEnvelope],
) -> Result<WorkstreamsFile> {
    if let Some(path) = maybe_yaml
        && path.exists()
    {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("read workstreams from {path:?}"))?;
        let ws: WorkstreamsFile = serde_yaml::from_str(&text)
            .with_context(|| format!("parse workstreams yaml {path:?}"))?;
        return Ok(ws);
    }

    clusterer.cluster(events)
}

/// Write workstreams to a YAML file.
pub fn write_workstreams(path: &Path, workstreams: &WorkstreamsFile) -> Result<()> {
    let yaml = serde_yaml::to_string(workstreams)?;
    std::fs::write(path, yaml).with_context(|| format!("write workstreams to {path:?}"))?;
    Ok(())
}

/// Two-layer workstream management:
/// - `workstreams.suggested.yaml` is machine-generated (can be overwritten)
/// - `workstreams.yaml` is user-curated (never overwritten without --regen)
///
/// This follows the principle: treat curation as state.
pub struct WorkstreamManager;

impl WorkstreamManager {
    /// Suggested file name (machine-generated proposals)
    pub const SUGGESTED_FILENAME: &'static str = "workstreams.suggested.yaml";
    /// Curated file name (user-owned source of truth)
    pub const CURATED_FILENAME: &'static str = "workstreams.yaml";

    /// Load the effective workstreams for rendering.
    ///
    /// Priority:
    /// 1. If `workstreams.yaml` exists, use it (user-curated)
    /// 2. If `workstreams.suggested.yaml` exists, use it (machine-generated)
    /// 3. Otherwise, generate from events and write to suggested file
    pub fn load_effective(
        out_dir: &Path,
        clusterer: &dyn WorkstreamClusterer,
        events: &[EventEnvelope],
    ) -> Result<WorkstreamsFile> {
        let curated_path = out_dir.join(Self::CURATED_FILENAME);
        let suggested_path = out_dir.join(Self::SUGGESTED_FILENAME);

        // Priority 1: User-curated file
        if curated_path.exists() {
            let text = std::fs::read_to_string(&curated_path)
                .with_context(|| format!("read curated workstreams from {curated_path:?}"))?;
            let ws: WorkstreamsFile = serde_yaml::from_str(&text)
                .with_context(|| format!("parse curated workstreams yaml {curated_path:?}"))?;
            return Ok(ws);
        }

        // Priority 2: Machine-suggested file
        if suggested_path.exists() {
            let text = std::fs::read_to_string(&suggested_path)
                .with_context(|| format!("read suggested workstreams from {suggested_path:?}"))?;
            let ws: WorkstreamsFile = serde_yaml::from_str(&text)
                .with_context(|| format!("parse suggested workstreams yaml {suggested_path:?}"))?;
            return Ok(ws);
        }

        // Priority 3: Generate new suggestions
        let ws = clusterer.cluster(events)?;
        write_workstreams(&suggested_path, &ws)?;
        Ok(ws)
    }

    /// Write machine-generated suggestions.
    /// This always overwrites `workstreams.suggested.yaml`.
    pub fn write_suggested(out_dir: &Path, workstreams: &WorkstreamsFile) -> Result<()> {
        let path = out_dir.join(Self::SUGGESTED_FILENAME);
        write_workstreams(&path, workstreams)
    }

    /// Check if user-curated workstreams exist.
    pub fn has_curated(out_dir: &Path) -> bool {
        out_dir.join(Self::CURATED_FILENAME).exists()
    }

    /// Get the path to the curated file.
    pub fn curated_path(out_dir: &Path) -> std::path::PathBuf {
        out_dir.join(Self::CURATED_FILENAME)
    }

    /// Get the path to the suggested file.
    pub fn suggested_path(out_dir: &Path) -> std::path::PathBuf {
        out_dir.join(Self::SUGGESTED_FILENAME)
    }

    /// Try to load workstreams from a directory. Returns None if no workstream files exist.
    /// Checks curated first, then suggested.
    pub fn try_load(dir: &Path) -> Result<Option<WorkstreamsFile>> {
        let curated_path = dir.join(Self::CURATED_FILENAME);
        if curated_path.exists() {
            let text = std::fs::read_to_string(&curated_path)
                .with_context(|| format!("read curated workstreams from {curated_path:?}"))?;
            let ws: WorkstreamsFile = serde_yaml::from_str(&text)
                .with_context(|| format!("parse curated workstreams yaml {curated_path:?}"))?;
            return Ok(Some(ws));
        }
        let suggested_path = dir.join(Self::SUGGESTED_FILENAME);
        if suggested_path.exists() {
            let text = std::fs::read_to_string(&suggested_path)
                .with_context(|| format!("read suggested workstreams from {suggested_path:?}"))?;
            let ws: WorkstreamsFile = serde_yaml::from_str(&text)
                .with_context(|| format!("parse suggested workstreams yaml {suggested_path:?}"))?;
            return Ok(Some(ws));
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shiplog_ids::EventId;
    use shiplog_schema::event::*;

    fn make_test_event(repo_name: &str, event_id: &str) -> EventEnvelope {
        EventEnvelope {
            id: EventId::from_parts(["x", event_id]),
            kind: EventKind::PullRequest,
            occurred_at: Utc::now(),
            actor: Actor {
                login: "a".into(),
                id: None,
            },
            repo: RepoRef {
                full_name: repo_name.into(),
                html_url: Some(format!("https://github.com/{repo_name}")),
                visibility: RepoVisibility::Unknown,
            },
            payload: EventPayload::PullRequest(PullRequestEvent {
                number: 1,
                title: "t".into(),
                state: PullRequestState::Merged,
                created_at: Utc::now(),
                merged_at: Some(Utc::now()),
                additions: Some(1),
                deletions: Some(0),
                changed_files: Some(1),
                touched_paths_hint: vec![],
                window: None,
            }),
            tags: vec![],
            links: vec![],
            source: SourceRef {
                system: SourceSystem::Unknown,
                url: None,
                opaque_id: None,
            },
        }
    }

    #[test]
    fn clusters_by_repo() {
        let ev1 = make_test_event("o/r1", "1");
        let ev2 = make_test_event("o/r2", "2");
        let ws = RepoClusterer.cluster(&[ev1, ev2]).unwrap();
        assert_eq!(ws.workstreams.len(), 2);
    }

    #[test]
    fn workstream_manager_prefers_curated() {
        let temp = tempfile::tempdir().unwrap();
        let out_dir = temp.path();

        // Create a curated file
        let curated = WorkstreamsFile {
            version: 1,
            generated_at: Utc::now(),
            workstreams: vec![Workstream {
                id: WorkstreamId::from_parts(["curated"]),
                title: "My Curated Workstream".into(),
                summary: Some("User-edited".into()),
                tags: vec![],
                stats: WorkstreamStats::zero(),
                events: vec![],
                receipts: vec![],
            }],
        };
        write_workstreams(&out_dir.join("workstreams.yaml"), &curated).unwrap();

        // Load effective — should return curated even with different events
        let ev = make_test_event("o/r1", "1");
        let loaded = WorkstreamManager::load_effective(out_dir, &RepoClusterer, &[ev]).unwrap();

        assert_eq!(loaded.workstreams.len(), 1);
        assert_eq!(loaded.workstreams[0].title, "My Curated Workstream");
    }

    #[test]
    fn workstream_manager_falls_back_to_suggested() {
        let temp = tempfile::tempdir().unwrap();
        let out_dir = temp.path();

        // Create only a suggested file
        let suggested = WorkstreamsFile {
            version: 1,
            generated_at: Utc::now(),
            workstreams: vec![Workstream {
                id: WorkstreamId::from_parts(["suggested"]),
                title: "Suggested Workstream".into(),
                summary: None,
                tags: vec![],
                stats: WorkstreamStats::zero(),
                events: vec![],
                receipts: vec![],
            }],
        };
        write_workstreams(&out_dir.join("workstreams.suggested.yaml"), &suggested).unwrap();

        // Load effective — should return suggested
        let ev = make_test_event("o/r1", "1");
        let loaded = WorkstreamManager::load_effective(out_dir, &RepoClusterer, &[ev]).unwrap();

        assert_eq!(loaded.workstreams.len(), 1);
        assert_eq!(loaded.workstreams[0].title, "Suggested Workstream");
    }

    #[test]
    fn try_load_empty_dir_returns_none() {
        let temp = tempfile::tempdir().unwrap();
        let result = WorkstreamManager::try_load(temp.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn try_load_with_curated_returns_curated() {
        let temp = tempfile::tempdir().unwrap();
        let ws = WorkstreamsFile {
            version: 1,
            generated_at: Utc::now(),
            workstreams: vec![Workstream {
                id: WorkstreamId::from_parts(["curated"]),
                title: "Curated".into(),
                summary: None,
                tags: vec![],
                stats: WorkstreamStats::zero(),
                events: vec![],
                receipts: vec![],
            }],
        };
        write_workstreams(&temp.path().join("workstreams.yaml"), &ws).unwrap();

        let loaded = WorkstreamManager::try_load(temp.path()).unwrap().unwrap();
        assert_eq!(loaded.workstreams[0].title, "Curated");
    }

    #[test]
    fn try_load_with_only_suggested_returns_suggested() {
        let temp = tempfile::tempdir().unwrap();
        let ws = WorkstreamsFile {
            version: 1,
            generated_at: Utc::now(),
            workstreams: vec![Workstream {
                id: WorkstreamId::from_parts(["suggested"]),
                title: "Suggested".into(),
                summary: None,
                tags: vec![],
                stats: WorkstreamStats::zero(),
                events: vec![],
                receipts: vec![],
            }],
        };
        write_workstreams(&temp.path().join("workstreams.suggested.yaml"), &ws).unwrap();

        let loaded = WorkstreamManager::try_load(temp.path()).unwrap().unwrap();
        assert_eq!(loaded.workstreams[0].title, "Suggested");
    }

    #[test]
    fn try_load_prefers_curated_over_suggested() {
        let temp = tempfile::tempdir().unwrap();

        let curated = WorkstreamsFile {
            version: 1,
            generated_at: Utc::now(),
            workstreams: vec![Workstream {
                id: WorkstreamId::from_parts(["curated"]),
                title: "Curated".into(),
                summary: None,
                tags: vec![],
                stats: WorkstreamStats::zero(),
                events: vec![],
                receipts: vec![],
            }],
        };
        write_workstreams(&temp.path().join("workstreams.yaml"), &curated).unwrap();

        let suggested = WorkstreamsFile {
            version: 1,
            generated_at: Utc::now(),
            workstreams: vec![Workstream {
                id: WorkstreamId::from_parts(["suggested"]),
                title: "Suggested".into(),
                summary: None,
                tags: vec![],
                stats: WorkstreamStats::zero(),
                events: vec![],
                receipts: vec![],
            }],
        };
        write_workstreams(&temp.path().join("workstreams.suggested.yaml"), &suggested).unwrap();

        let loaded = WorkstreamManager::try_load(temp.path()).unwrap().unwrap();
        assert_eq!(loaded.workstreams[0].title, "Curated");
    }

    #[test]
    fn workstream_manager_generates_when_missing() {
        let temp = tempfile::tempdir().unwrap();
        let out_dir = temp.path();

        // No files exist — should generate from events
        let ev1 = make_test_event("o/r1", "1");
        let ev2 = make_test_event("o/r2", "2");
        let loaded =
            WorkstreamManager::load_effective(out_dir, &RepoClusterer, &[ev1, ev2]).unwrap();

        // Should have generated workstreams based on repos
        assert_eq!(loaded.workstreams.len(), 2);

        // Should have written suggested file
        assert!(out_dir.join("workstreams.suggested.yaml").exists());
    }
}
