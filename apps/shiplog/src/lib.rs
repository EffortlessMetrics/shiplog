//! Internal library modules for the `shiplog` package.

extern crate self as shiplog;

pub mod bundle;
pub mod cache;
#[cfg(feature = "llm")]
pub mod cluster_llm;
pub mod coverage;
pub mod engine;
pub mod ids;
pub mod ingest;
pub mod merge;
pub mod ports;
pub mod redact;
pub mod render;
pub mod schema;
pub mod team;
pub mod workstreams;
