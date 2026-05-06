//! Repository-based workstream clustering and workstream file contracts.
//!
//! Clustering, curated/suggested file lifecycle policies, and receipt display
//! limits live as modules under this crate so workstream phases do not become
//! separate package contracts.
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

pub mod cluster;
pub mod layout;
pub mod receipt_policy;

pub use cluster::RepoClusterer;
pub use layout::{
    CURATED_FILENAME, SUGGESTED_FILENAME, WorkstreamManager, load_or_cluster, write_workstreams,
};
pub use receipt_policy::{
    WORKSTREAM_RECEIPT_LIMIT_MANUAL, WORKSTREAM_RECEIPT_LIMIT_REVIEW,
    WORKSTREAM_RECEIPT_LIMIT_TOTAL, WORKSTREAM_RECEIPT_RENDER_LIMIT, max_cluster_receipts_for_kind,
    should_include_cluster_receipt, should_render_receipt_at, truncate_cluster_receipts,
};
