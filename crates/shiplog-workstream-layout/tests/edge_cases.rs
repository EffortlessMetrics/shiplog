//! Edge-case tests for shiplog-workstream-layout.
//!
//! Covers layout calculation with various workstream counts, output path
//! generation, profile-specific layouts, invalid YAML handling, and
//! overwrite semantics.

use chrono::Utc;
use shiplog_ids::{EventId, WorkstreamId};
use shiplog_ports::WorkstreamClusterer;
use shiplog_schema::event::EventEnvelope;
use shiplog_schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};
use shiplog_workstream_layout::{
    CURATED_FILENAME, SUGGESTED_FILENAME, WorkstreamManager, load_or_cluster, write_workstreams,
};
use tempfile::tempdir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_ws_file_with_n(n: usize) -> WorkstreamsFile {
    let workstreams = (0..n)
        .map(|i| Workstream {
            id: WorkstreamId::from_parts(["ws", &format!("{i}")]),
            title: format!("workstream-{i}"),
            summary: if i % 2 == 0 {
                Some(format!("Summary {i}"))
            } else {
                None
            },
            tags: vec!["repo".into()],
            stats: WorkstreamStats {
                pull_requests: i,
                reviews: 0,
                manual_events: 0,
            },
            events: vec![EventId::from_parts(["e", &format!("{i}")])],
            receipts: vec![],
        })
        .collect();

    WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams,
    }
}

struct CountingClusterer {
    count: usize,
}

impl WorkstreamClusterer for CountingClusterer {
    fn cluster(&self, _events: &[EventEnvelope]) -> anyhow::Result<WorkstreamsFile> {
        Ok(make_ws_file_with_n(self.count))
    }
}

// ===========================================================================
// Layout with various workstream counts
// ===========================================================================

#[test]
fn zero_workstreams_roundtrip() {
    let dir = tempdir().unwrap();
    let ws = make_ws_file_with_n(0);
    let path = dir.path().join("zero.yaml");
    write_workstreams(&path, &ws).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    let loaded: WorkstreamsFile = serde_yaml::from_str(&text).unwrap();
    assert!(loaded.workstreams.is_empty());
}

#[test]
fn one_workstream_roundtrip() {
    let dir = tempdir().unwrap();
    let ws = make_ws_file_with_n(1);
    let path = dir.path().join("one.yaml");
    write_workstreams(&path, &ws).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    let loaded: WorkstreamsFile = serde_yaml::from_str(&text).unwrap();
    assert_eq!(loaded.workstreams.len(), 1);
    assert_eq!(loaded.workstreams[0].title, "workstream-0");
}

#[test]
fn many_workstreams_roundtrip() {
    let dir = tempdir().unwrap();
    let ws = make_ws_file_with_n(100);
    let path = dir.path().join("many.yaml");
    write_workstreams(&path, &ws).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    let loaded: WorkstreamsFile = serde_yaml::from_str(&text).unwrap();
    assert_eq!(loaded.workstreams.len(), 100);

    for (i, w) in loaded.workstreams.iter().enumerate() {
        assert_eq!(w.title, format!("workstream-{i}"));
    }
}

// ===========================================================================
// Output path generation
// ===========================================================================

#[test]
fn curated_path_joins_correctly() {
    let base = std::path::Path::new("/output/run-001");
    let path = WorkstreamManager::curated_path(base);
    assert_eq!(path, base.join(CURATED_FILENAME));
}

#[test]
fn suggested_path_joins_correctly() {
    let base = std::path::Path::new("/output/run-001");
    let path = WorkstreamManager::suggested_path(base);
    assert_eq!(path, base.join(SUGGESTED_FILENAME));
}

#[test]
fn paths_with_spaces_work() {
    let dir = tempdir().unwrap();
    let spaced_dir = dir.path().join("path with spaces");
    std::fs::create_dir_all(&spaced_dir).unwrap();

    let ws = make_ws_file_with_n(1);
    write_workstreams(&WorkstreamManager::curated_path(&spaced_dir), &ws).unwrap();

    let loaded = WorkstreamManager::try_load(&spaced_dir).unwrap().unwrap();
    assert_eq!(loaded.workstreams.len(), 1);
}

#[test]
fn nested_output_directory() {
    let dir = tempdir().unwrap();
    let nested = dir.path().join("a").join("b").join("c");
    std::fs::create_dir_all(&nested).unwrap();

    let ws = make_ws_file_with_n(2);
    WorkstreamManager::write_suggested(&nested, &ws).unwrap();

    assert!(WorkstreamManager::suggested_path(&nested).exists());
    let loaded = WorkstreamManager::try_load(&nested).unwrap().unwrap();
    assert_eq!(loaded.workstreams.len(), 2);
}

// ===========================================================================
// Profile-specific layouts: curated vs suggested precedence
// ===========================================================================

#[test]
fn has_curated_is_false_when_only_suggested_exists() {
    let dir = tempdir().unwrap();
    WorkstreamManager::write_suggested(dir.path(), &make_ws_file_with_n(1)).unwrap();

    assert!(!WorkstreamManager::has_curated(dir.path()));
}

#[test]
fn has_curated_is_true_when_curated_exists() {
    let dir = tempdir().unwrap();
    write_workstreams(
        &WorkstreamManager::curated_path(dir.path()),
        &make_ws_file_with_n(1),
    )
    .unwrap();

    assert!(WorkstreamManager::has_curated(dir.path()));
}

#[test]
fn load_effective_generates_and_persists_suggested() {
    let dir = tempdir().unwrap();
    let clusterer = CountingClusterer { count: 3 };

    let loaded = WorkstreamManager::load_effective(dir.path(), &clusterer, &[]).unwrap();
    assert_eq!(loaded.workstreams.len(), 3);

    // Suggested file should now exist on disk
    let persisted = WorkstreamManager::try_load(dir.path()).unwrap().unwrap();
    assert_eq!(persisted.workstreams.len(), 3);
}

#[test]
fn load_effective_does_not_overwrite_curated_with_generated() {
    let dir = tempdir().unwrap();
    write_workstreams(
        &WorkstreamManager::curated_path(dir.path()),
        &make_ws_file_with_n(2),
    )
    .unwrap();

    let clusterer = CountingClusterer { count: 5 };
    let loaded = WorkstreamManager::load_effective(dir.path(), &clusterer, &[]).unwrap();

    // Should load curated (2), not generate (5)
    assert_eq!(loaded.workstreams.len(), 2);
}

// ===========================================================================
// load_or_cluster edge cases
// ===========================================================================

#[test]
fn load_or_cluster_with_none_path_delegates_to_clusterer() {
    let clusterer = CountingClusterer { count: 4 };
    let loaded = load_or_cluster(None, &clusterer, &[]).unwrap();
    assert_eq!(loaded.workstreams.len(), 4);
}

#[test]
fn load_or_cluster_with_nonexistent_path_delegates_to_clusterer() {
    let clusterer = CountingClusterer { count: 7 };
    let path = std::path::Path::new("/nonexistent/path/workstreams.yaml");
    let loaded = load_or_cluster(Some(path), &clusterer, &[]).unwrap();
    assert_eq!(loaded.workstreams.len(), 7);
}

#[test]
fn load_or_cluster_with_existing_path_reads_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("custom.yaml");
    write_workstreams(&path, &make_ws_file_with_n(3)).unwrap();

    let clusterer = CountingClusterer { count: 99 };
    let loaded = load_or_cluster(Some(&path), &clusterer, &[]).unwrap();
    assert_eq!(loaded.workstreams.len(), 3);
}

// ===========================================================================
// Overwrite semantics
// ===========================================================================

#[test]
fn write_suggested_overwrites_previous() {
    let dir = tempdir().unwrap();
    WorkstreamManager::write_suggested(dir.path(), &make_ws_file_with_n(5)).unwrap();
    WorkstreamManager::write_suggested(dir.path(), &make_ws_file_with_n(2)).unwrap();

    let loaded = WorkstreamManager::try_load(dir.path()).unwrap().unwrap();
    assert_eq!(loaded.workstreams.len(), 2);
}

#[test]
fn write_workstreams_to_same_path_overwrites() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("overwrite.yaml");

    write_workstreams(&path, &make_ws_file_with_n(10)).unwrap();
    write_workstreams(&path, &make_ws_file_with_n(1)).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    let loaded: WorkstreamsFile = serde_yaml::from_str(&text).unwrap();
    assert_eq!(loaded.workstreams.len(), 1);
}

// ===========================================================================
// Invalid YAML error handling
// ===========================================================================

#[test]
fn read_invalid_yaml_returns_error() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("invalid.yaml");
    std::fs::write(&path, "not: valid: yaml: [[[").unwrap();

    let result = load_or_cluster(Some(&path), &CountingClusterer { count: 1 }, &[]);
    assert!(result.is_err());
}

#[test]
fn read_empty_file_returns_error() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("empty.yaml");
    std::fs::write(&path, "").unwrap();

    let result = load_or_cluster(Some(&path), &CountingClusterer { count: 1 }, &[]);
    assert!(result.is_err());
}

// ===========================================================================
// Filename constants
// ===========================================================================

#[test]
fn filename_constants_are_yaml_extensions() {
    assert!(CURATED_FILENAME.ends_with(".yaml"));
    assert!(SUGGESTED_FILENAME.ends_with(".yaml"));
}

#[test]
fn curated_and_suggested_filenames_differ() {
    assert_ne!(CURATED_FILENAME, SUGGESTED_FILENAME);
}

#[test]
fn manager_constants_match_module_constants() {
    assert_eq!(WorkstreamManager::CURATED_FILENAME, CURATED_FILENAME);
    assert_eq!(WorkstreamManager::SUGGESTED_FILENAME, SUGGESTED_FILENAME);
}
