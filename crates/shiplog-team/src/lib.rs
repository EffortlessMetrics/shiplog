//! Team aggregation mode for generating team-level shipping summaries.
//!
//! `shiplog-team` is the optional public boundary. Its internal phases live as
//! modules so config resolution, ledger aggregation, and packet rendering do
//! not become separate package contracts.

pub mod aggregate;
pub mod core;
pub mod render;
mod template;

pub use aggregate::{TeamAggregator, TeamOutputFiles, write_team_outputs};
pub use core::{TeamConfig, parse_alias_list, parse_csv_list, resolve_team_config};
pub use render::{TeamAggregateResult, TeamMemberSummary, render_packet_markdown};
