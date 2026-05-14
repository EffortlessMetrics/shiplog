//! Team aggregation mode for generating team-level shipping summaries.
//!
//! Team support lives inside the `shiplog` package so config resolution,
//! ledger aggregation, and packet rendering do not become separate crates.io
//! package contracts.

pub mod aggregate;
pub mod core;
pub mod render;
mod template;

pub use aggregate::{TeamAggregator, TeamOutputFiles, write_team_outputs};
pub use core::{TeamConfig, parse_alias_list, parse_csv_list, resolve_team_config};
pub use render::{TeamAggregateResult, TeamMemberSummary, render_packet_markdown};
