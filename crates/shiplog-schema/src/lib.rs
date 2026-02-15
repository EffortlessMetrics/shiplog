//! Canonical event model and data types for the shiplog pipeline.
//!
//! Defines event envelopes, event payloads (pull requests, reviews, manual entries),
//! coverage manifests, workstream definitions, and bundle metadata.
//! All other crates depend on these types.

pub mod bundle;
pub mod coverage;
pub mod event;
pub mod workstream;
