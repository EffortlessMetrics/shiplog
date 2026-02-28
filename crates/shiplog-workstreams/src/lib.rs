//! Repository-based workstream clustering and workstream file contracts.
//!
//! Clustering stays in `shiplog-workstream-cluster`; file lifecycle policies are
//! delegated to `shiplog-workstream-layout`.
//!
//! # Examples
//!
//! Cluster events by repository using the default strategy:
//!
//! ```
//! use shiplog_workstreams::RepoClusterer;
//! use shiplog_ports::WorkstreamClusterer;
//!
//! let clusterer = RepoClusterer;
//! let ws = clusterer.cluster(&[]).unwrap();
//! assert!(ws.workstreams.is_empty());
//! ```
//!
//! Resolve workstream file paths:
//!
//! ```
//! use shiplog_workstreams::WorkstreamManager;
//! use std::path::Path;
//!
//! let dir = Path::new("./out/run_123");
//! let curated = WorkstreamManager::curated_path(dir);
//! let suggested = WorkstreamManager::suggested_path(dir);
//! assert!(curated.ends_with("workstreams.yaml"));
//! assert!(suggested.ends_with("workstreams.suggested.yaml"));
//! ```

pub use shiplog_workstream_cluster::RepoClusterer;
pub use shiplog_workstream_layout::{
    CURATED_FILENAME, SUGGESTED_FILENAME, WorkstreamManager, load_or_cluster, write_workstreams,
};
