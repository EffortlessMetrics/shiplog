//! Workstream artifact path contracts and file precedence logic.
//!
//! This microcrate owns the loading/saving rules for workstream files:
//! - `workstreams.yaml` (curated, user-owned)
//! - `workstreams.suggested.yaml` (machine-generated)
//!
//! It intentionally has one responsibility: stable workstream-file semantics.

use anyhow::{Context, Result};
use shiplog_ports::WorkstreamClusterer;
use shiplog_schema::event::EventEnvelope;
use shiplog_schema::workstream::WorkstreamsFile;
use std::path::{Path, PathBuf};

/// User-curated workstream file.
pub const CURATED_FILENAME: &str = "workstreams.yaml";

/// Machine-generated suggested workstream file.
pub const SUGGESTED_FILENAME: &str = "workstreams.suggested.yaml";

/// Load an existing YAML file if present, otherwise run clustering.
pub fn load_or_cluster(
    maybe_yaml: Option<&Path>,
    clusterer: &dyn WorkstreamClusterer,
    events: &[EventEnvelope],
) -> Result<WorkstreamsFile> {
    if let Some(path) = maybe_yaml.filter(|path| path.exists()) {
        return read_workstreams(path);
    }
    clusterer.cluster(events)
}

/// Write a `WorkstreamsFile` as YAML.
pub fn write_workstreams(path: &Path, workstreams: &WorkstreamsFile) -> Result<()> {
    let yaml = serde_yaml::to_string(workstreams)?;
    std::fs::write(path, yaml).with_context(|| format!("write workstreams to {path:?}"))?;
    Ok(())
}

/// Two-layer workstream file lifecycle:
///
/// 1. `workstreams.yaml` = curated (user-owned, highest priority)
/// 2. `workstreams.suggested.yaml` = generated suggestions
/// 3. If neither exists, generate from events and write suggestions
pub struct WorkstreamManager;

impl WorkstreamManager {
    /// Suggested file name (machine-generated proposals)
    pub const SUGGESTED_FILENAME: &'static str = SUGGESTED_FILENAME;

    /// Curated file name (user-owned source of truth)
    pub const CURATED_FILENAME: &'static str = CURATED_FILENAME;

    /// Load the effective workstreams for rendering.
    ///
    /// Priority:
    /// 1. `workstreams.yaml` if present
    /// 2. `workstreams.suggested.yaml` if present
    /// 3. generate via clusterer and persist suggested file
    pub fn load_effective(
        out_dir: &Path,
        clusterer: &dyn WorkstreamClusterer,
        events: &[EventEnvelope],
    ) -> Result<WorkstreamsFile> {
        let curated_path = Self::curated_path(out_dir);
        if curated_path.exists() {
            return read_workstreams(&curated_path);
        }

        let suggested_path = Self::suggested_path(out_dir);
        if suggested_path.exists() {
            return read_workstreams(&suggested_path);
        }

        let ws = clusterer.cluster(events)?;
        write_workstreams(&suggested_path, &ws)?;
        Ok(ws)
    }

    /// Write machine-generated suggested workstreams.
    /// This always overwrites `workstreams.suggested.yaml`.
    pub fn write_suggested(out_dir: &Path, workstreams: &WorkstreamsFile) -> Result<()> {
        let path = Self::suggested_path(out_dir);
        write_workstreams(&path, workstreams)
    }

    /// Check whether the curated workstreams file exists.
    pub fn has_curated(out_dir: &Path) -> bool {
        Self::curated_path(out_dir).exists()
    }

    /// Get the curated file path.
    pub fn curated_path(out_dir: &Path) -> PathBuf {
        out_dir.join(Self::CURATED_FILENAME)
    }

    /// Get the suggested file path.
    pub fn suggested_path(out_dir: &Path) -> PathBuf {
        out_dir.join(Self::SUGGESTED_FILENAME)
    }

    /// Try to load curated then suggested workstreams.
    pub fn try_load(out_dir: &Path) -> Result<Option<WorkstreamsFile>> {
        let curated_path = Self::curated_path(out_dir);
        if curated_path.exists() {
            return Ok(Some(read_workstreams(&curated_path)?));
        }

        let suggested_path = Self::suggested_path(out_dir);
        if suggested_path.exists() {
            return Ok(Some(read_workstreams(&suggested_path)?));
        }

        Ok(None)
    }
}

fn read_workstreams(path: &Path) -> Result<WorkstreamsFile> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("read workstreams from {path:?}"))?;
    let workstreams: WorkstreamsFile =
        serde_yaml::from_str(&text).with_context(|| format!("parse workstreams yaml {path:?}"))?;
    Ok(workstreams)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use shiplog_ids::{EventId, WorkstreamId};
    use shiplog_ports::WorkstreamClusterer;
    use shiplog_schema::event::{
        Actor, EventEnvelope, EventKind, EventPayload, Link, RepoRef, RepoVisibility, ReviewEvent,
        SourceRef, SourceSystem,
    };
    use shiplog_schema::workstream::{Workstream, WorkstreamStats};
    use tempfile::tempdir;

    fn make_event(repo_name: &str, event_id: &str, number: u64) -> EventEnvelope {
        EventEnvelope {
            id: EventId::from_parts(["test", event_id]),
            kind: EventKind::PullRequest,
            occurred_at: Utc::now(),
            actor: Actor {
                login: "actor".into(),
                id: None,
            },
            repo: RepoRef {
                full_name: repo_name.into(),
                html_url: Some(format!("https://example.com/{repo_name}")),
                visibility: RepoVisibility::Unknown,
            },
            payload: EventPayload::Review(ReviewEvent {
                pull_number: number,
                pull_title: "A review".into(),
                submitted_at: Utc::now(),
                state: "approved".into(),
                window: None,
            }),
            tags: vec![],
            links: vec![Link {
                label: "review".into(),
                url: format!("https://example.com/{repo_name}/reviews/{number}"),
            }],
            source: SourceRef {
                system: SourceSystem::Github,
                url: Some("https://api.example.com".into()),
                opaque_id: None,
            },
        }
    }

    fn make_workstreams(title: &str, repo: &str) -> WorkstreamsFile {
        WorkstreamsFile {
            version: 1,
            generated_at: Utc::now(),
            workstreams: vec![Workstream {
                id: WorkstreamId::from_parts(["repo", repo]),
                title: title.to_string(),
                summary: Some("test".into()),
                tags: vec!["repo".into()],
                stats: WorkstreamStats::zero(),
                events: vec![],
                receipts: vec![],
            }],
        }
    }

    struct FakeClusterer;
    impl WorkstreamClusterer for FakeClusterer {
        fn cluster(&self, _events: &[EventEnvelope]) -> anyhow::Result<WorkstreamsFile> {
            Ok(make_workstreams("fallback", "fallback"))
        }
    }

    #[test]
    fn load_or_cluster_prefers_existing_yaml() {
        let temp_dir = tempdir().unwrap();
        let path = temp_dir.path().join("existing.yaml");
        let workstreams = make_workstreams("existing", "repo/ex");
        write_workstreams(&path, &workstreams).unwrap();

        let loaded = load_or_cluster(Some(&path), &FakeClusterer, &[]).unwrap();
        assert_eq!(loaded.workstreams[0].title, "existing");
    }

    #[test]
    fn load_or_cluster_falls_back_to_clusterer() {
        let loaded = load_or_cluster(None, &FakeClusterer, &[]).unwrap();
        assert_eq!(loaded.workstreams[0].title, "fallback");
    }

    #[test]
    fn load_effective_prefers_curated_over_suggested() {
        let temp_dir = tempdir().unwrap();
        let curated = temp_dir.path().join(CURATED_FILENAME);
        let suggested = temp_dir.path().join(SUGGESTED_FILENAME);
        write_workstreams(&curated, &make_workstreams("curated", "repo/c")).unwrap();
        write_workstreams(&suggested, &make_workstreams("suggested", "repo/s")).unwrap();

        let loaded =
            WorkstreamManager::load_effective(temp_dir.path(), &FakeClusterer, &[]).unwrap();
        assert_eq!(loaded.workstreams[0].title, "curated");
        assert_eq!(curated, WorkstreamManager::curated_path(temp_dir.path()));
    }

    #[test]
    fn load_effective_falls_back_to_suggested() {
        let temp_dir = tempdir().unwrap();
        let suggested = temp_dir.path().join(SUGGESTED_FILENAME);
        write_workstreams(&suggested, &make_workstreams("suggested", "repo/s")).unwrap();

        let loaded =
            WorkstreamManager::load_effective(temp_dir.path(), &FakeClusterer, &[]).unwrap();
        assert_eq!(loaded.workstreams[0].title, "suggested");
    }

    #[test]
    fn load_effective_generates_when_missing() {
        let temp_dir = tempdir().unwrap();
        let loaded = WorkstreamManager::load_effective(
            temp_dir.path(),
            &FakeClusterer,
            &[make_event("repo/a", "1", 1)],
        )
        .unwrap();
        assert_eq!(loaded.workstreams[0].title, "fallback");
        assert!(WorkstreamManager::suggested_path(temp_dir.path()).exists());
    }

    #[test]
    fn try_load_respects_precedence() {
        let temp_dir = tempdir().unwrap();
        let curated = temp_dir.path().join(CURATED_FILENAME);
        let suggested = temp_dir.path().join(SUGGESTED_FILENAME);
        write_workstreams(&suggested, &make_workstreams("suggested", "repo/s")).unwrap();
        write_workstreams(&curated, &make_workstreams("curated", "repo/c")).unwrap();

        let loaded = WorkstreamManager::try_load(temp_dir.path())
            .unwrap()
            .unwrap();
        assert_eq!(loaded.workstreams[0].title, "curated");
    }

    #[test]
    fn has_curated_checks_file_presence() {
        let temp_dir = tempdir().unwrap();
        assert!(!WorkstreamManager::has_curated(temp_dir.path()));

        write_workstreams(
            &WorkstreamManager::curated_path(temp_dir.path()),
            &make_workstreams("curated", "repo"),
        )
        .unwrap();
        assert!(WorkstreamManager::has_curated(temp_dir.path()));
    }

    #[test]
    fn curated_path_uses_correct_filename() {
        let dir = Path::new("/some/dir");
        let path = WorkstreamManager::curated_path(dir);
        assert_eq!(path.file_name().unwrap(), CURATED_FILENAME);
    }

    #[test]
    fn suggested_path_uses_correct_filename() {
        let dir = Path::new("/some/dir");
        let path = WorkstreamManager::suggested_path(dir);
        assert_eq!(path.file_name().unwrap(), SUGGESTED_FILENAME);
    }

    #[test]
    fn write_suggested_writes_to_correct_path() {
        let temp_dir = tempdir().unwrap();
        let ws = make_workstreams("suggested-write", "repo/sw");
        WorkstreamManager::write_suggested(temp_dir.path(), &ws).unwrap();

        let suggested_path = WorkstreamManager::suggested_path(temp_dir.path());
        assert!(suggested_path.exists());

        let loaded = read_workstreams(&suggested_path).unwrap();
        assert_eq!(loaded.workstreams[0].title, "suggested-write");
    }

    #[test]
    fn try_load_returns_none_when_empty() {
        let temp_dir = tempdir().unwrap();
        let result = WorkstreamManager::try_load(temp_dir.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn load_or_cluster_with_nonexistent_path_falls_back() {
        let non_existent = Path::new("/does/not/exist/workstreams.yaml");
        let loaded = load_or_cluster(Some(non_existent), &FakeClusterer, &[]).unwrap();
        assert_eq!(loaded.workstreams[0].title, "fallback");
    }

    #[test]
    fn write_read_roundtrip_preserves_empty_workstreams() {
        let temp_dir = tempdir().unwrap();
        let ws = WorkstreamsFile {
            version: 1,
            generated_at: Utc::now(),
            workstreams: vec![],
        };
        let path = temp_dir.path().join("empty.yaml");
        write_workstreams(&path, &ws).unwrap();
        let loaded = read_workstreams(&path).unwrap();
        assert!(loaded.workstreams.is_empty());
        assert_eq!(loaded.version, 1);
    }

    #[test]
    fn write_suggested_overwrites_existing() {
        let temp_dir = tempdir().unwrap();
        let ws1 = make_workstreams("first", "repo/first");
        let ws2 = make_workstreams("second", "repo/second");

        WorkstreamManager::write_suggested(temp_dir.path(), &ws1).unwrap();
        WorkstreamManager::write_suggested(temp_dir.path(), &ws2).unwrap();

        let loaded =
            read_workstreams(&WorkstreamManager::suggested_path(temp_dir.path())).unwrap();
        assert_eq!(loaded.workstreams[0].title, "second");
    }
}
