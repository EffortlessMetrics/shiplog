//! CLI surface for `cargo xtask`.

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::tasks;

/// shiplog xtask: Rust-native policy runner and release-proof aggregator.
#[derive(Debug, Parser)]
#[command(name = "xtask", version, about, long_about = None)]
pub struct Cli {
    /// Override workspace root (defaults to the parent of `CARGO_MANIFEST_DIR`).
    #[arg(long, global = true, env = "SHIPLOG_XTASK_WORKSPACE_ROOT")]
    pub workspace_root: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Validate `policy/*.toml` common headers and well-formed structure.
    CheckPolicySchemas,

    /// Verify published vs dev-only crate classification.
    ///
    /// Currently delegates to `scripts/package-boundary-audit.sh` until
    /// Rust parity is proven (see `docs/FILE_POLICY.md`
    /// shell-script-as-wrapper rule).
    PackageBoundary,

    /// Verify workspace package version alignment.
    ///
    /// Currently delegates to `scripts/package-version-audit.sh` until
    /// Rust parity is proven.
    PackageVersion,

    /// Print a human summary of every policy ledger.
    PolicyReport,
}

impl Cli {
    pub fn run(self) -> Result<()> {
        let workspace_root = match self.workspace_root {
            Some(path) => path,
            None => crate::policy::detect_workspace_root()?,
        };
        match self.command {
            Command::CheckPolicySchemas => tasks::check_policy_schemas::run(&workspace_root),
            Command::PackageBoundary => tasks::package_boundary::run(&workspace_root),
            Command::PackageVersion => tasks::package_version::run(&workspace_root),
            Command::PolicyReport => tasks::policy_report::run(&workspace_root),
        }
    }
}
