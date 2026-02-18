//! BDD scenarios for v0.3.x (Next) features
//!
//! This module implements BDD scenarios for the v0.3.x features:
//! 1. GitLab Ingest Adapter
//! 2. Jira/Linear Ingest Adapter
//! 3. Multi-Source Merging
//! 4. Configurable Packet Templates
//! 5. LLM Clustering as Default Feature
//!
//! These scenarios follow the Given/When/Then pattern and can be used
//! to verify the behavior of these features.

pub mod gitlab_ingest;
pub mod jira_linear_ingest;
pub mod multi_source_merging;
pub mod configurable_templates;
pub mod llm_clustering;

// Re-export all scenarios for convenience
pub use gitlab_ingest::*;
pub use jira_linear_ingest::*;
pub use multi_source_merging::*;
pub use configurable_templates::*;
pub use llm_clustering::*;
