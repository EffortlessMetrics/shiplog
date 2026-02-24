//! Repository-based workstream clustering and workstream file contracts.
//!
//! Clustering stays in `shiplog-workstream-cluster`; file lifecycle policies are
//! delegated to `shiplog-workstream-layout`.

pub use shiplog_workstream_cluster::RepoClusterer;
pub use shiplog_workstream_layout::{
    CURATED_FILENAME, SUGGESTED_FILENAME, WorkstreamManager, load_or_cluster, write_workstreams,
};
