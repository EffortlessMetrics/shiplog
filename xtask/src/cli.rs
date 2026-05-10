//! CLI surface for `cargo xtask`.

use anyhow::Result;
use clap::{Args, Parser, Subcommand};
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

    /// CI economics commands (forecast, actuals).
    Ci(CiArgs),
}

#[derive(Debug, Args)]
pub struct CiArgs {
    #[command(subcommand)]
    command: CiCommand,
}

#[derive(Debug, Subcommand)]
enum CiCommand {
    /// Emit a CI Plan (forecast) against contracts/schemas/ci-plan.v1.schema.json.
    Plan(PlanArgs),
    /// Emit CI Actuals (per-job timings + drift) against contracts/schemas/ci-actuals.v1.schema.json.
    Actuals(ActualsArgs),
}

#[derive(Debug, Args)]
pub struct PlanArgs {
    /// Base SHA or git ref (PR target). Defaults to env GITHUB_BASE_SHA, then `main`.
    #[arg(long, env = "GITHUB_BASE_SHA")]
    pub base_ref: Option<String>,

    /// Head SHA or git ref (PR source). Defaults to env GITHUB_HEAD_SHA, then `HEAD`.
    #[arg(long, env = "GITHUB_HEAD_SHA")]
    pub head_ref: Option<String>,

    /// PR number (omit for non-PR runs).
    #[arg(long, env = "GITHUB_PR_NUMBER")]
    pub pr_number: Option<u32>,

    /// Comma-separated PR labels.
    #[arg(long, env = "GITHUB_PR_LABELS", value_delimiter = ',')]
    pub labels: Vec<String>,

    /// Override changed files (comma-separated). Skips git diff. Used for testing.
    #[arg(long, value_delimiter = ',')]
    pub changed_files: Vec<String>,

    /// Output path for the JSON plan.
    #[arg(long, default_value = "target/ci/ci-plan.json")]
    pub output: PathBuf,
}

#[derive(Debug, Args)]
pub struct ActualsArgs {
    /// GitHub Actions run ID being summarized.
    #[arg(long, env = "GITHUB_RUN_ID")]
    pub run_id: String,

    /// Top-level workflow name whose run produced the artifact.
    #[arg(long, env = "GITHUB_WORKFLOW")]
    pub workflow_name: String,

    /// Head SHA of the run.
    #[arg(long, env = "GITHUB_SHA")]
    pub head_sha: String,

    /// PR number (optional).
    #[arg(long, env = "GITHUB_PR_NUMBER")]
    pub pr_number: Option<u32>,

    /// Path to a JSON file with the GitHub Actions jobs response
    /// (output of `gh api repos/{owner}/{repo}/actions/runs/{run_id}/jobs`).
    #[arg(long)]
    pub input: PathBuf,

    /// Output path for the JSON actuals.
    #[arg(long, default_value = "target/ci/ci-actuals.json")]
    pub output: PathBuf,
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
            Command::Ci(ci) => match ci.command {
                CiCommand::Plan(args) => tasks::ci_plan::run(tasks::ci_plan::PlanInputs {
                    workspace_root,
                    base_ref: args.base_ref,
                    head_ref: args.head_ref,
                    pr_number: args.pr_number,
                    labels: args.labels,
                    changed_files_override: if args.changed_files.is_empty() {
                        None
                    } else {
                        Some(args.changed_files)
                    },
                    output: args.output,
                }),
                CiCommand::Actuals(args) => {
                    tasks::ci_actuals::run(tasks::ci_actuals::ActualsInputs {
                        workspace_root,
                        run_id: args.run_id,
                        workflow_name: args.workflow_name,
                        head_sha: args.head_sha,
                        pr_number: args.pr_number,
                        jobs_input: args.input,
                        output: args.output,
                    })
                }
            },
        }
    }
}
