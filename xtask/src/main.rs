//! `xtask` — shiplog's Rust-native policy runner and release-proof
//! aggregator.
//!
//! See [`docs/xtask.md`](../../docs/xtask.md) and
//! [`docs/ci/policy-ledgers.md`](../../docs/ci/policy-ledgers.md).

use anyhow::Result;
use clap::Parser;

mod cli;
mod policy;
mod tasks;

fn main() -> Result<()> {
    cli::Cli::parse().run()
}
