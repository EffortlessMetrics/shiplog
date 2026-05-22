//! CLI surface for `cargo xtask`.

use anyhow::{Context, Result};
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

    /// Validate `policy/doc-artifacts.toml` artifact links and file receipts.
    CheckDocArtifacts,

    /// Validate `.codex/goals/active.toml` execution-state links and proof refs.
    CheckGoals,

    /// Validate support-tier claims and proof-command refs.
    CheckSupportTiers,

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

    /// Write a source-of-truth graph report for humans and agents.
    RepoContractReport,

    /// CI economics commands (forecast, actuals).
    Ci(CiArgs),

    /// Verify the non-Rust file allowlist (`policy/non-rust-allowlist.toml`).
    CheckFilePolicy(FilePolicyModeArgs),

    /// Verify the generated-file allowlist (`policy/generated-allowlist.toml`).
    CheckGenerated(FilePolicyModeArgs),

    /// Verify the executable-file allowlist (`policy/executable-allowlist.toml`).
    CheckExecutableFiles(FilePolicyModeArgs),

    /// Verify the workflow allowlist (`policy/workflow-allowlist.toml`).
    CheckWorkflows(FilePolicyModeArgs),

    /// Verify the dependency-surface allowlist (`policy/dependency-surface-allowlist.toml`).
    CheckDependencySurfaces(FilePolicyModeArgs),

    /// Verify the process-policy allowlist (`policy/process-allowlist.toml`).
    CheckProcessPolicy(FilePolicyModeArgs),

    /// Verify the network-policy allowlist (`policy/network-allowlist.toml`).
    CheckNetworkPolicy(FilePolicyModeArgs),

    /// Verify the Clippy lint ledger (`policy/clippy-lints.toml` + `clippy-debt.toml`).
    CheckLintPolicy(FilePolicyModeArgs),

    /// Verify Clippy exceptions in source vs `policy/clippy-exceptions.toml`.
    CheckClippyExceptions(FilePolicyModeArgs),

    /// No-panic baseline commands.
    NoPanic(NoPanicArgs),

    /// Verify the panic-family baseline (`policy/no-panic-baseline.toml`).
    CheckNoPanicFamily(FilePolicyModeArgs),

    /// Verify routing labels declared in `policy/ci-budget.toml [labels]`
    /// against actual consumption by workflow job-level `if:` blocks.
    CheckLabelEnforcement(FilePolicyModeArgs),

    /// Verify `policy/ci-risk-packs.toml` referential integrity:
    /// `selected_lanes` entries resolve to `[lane.*]` tables in
    /// `policy/ci-lanes.toml`, and `labels` entries resolve to labels
    /// in `policy/ci-budget.toml [labels]`.
    CheckRiskPackIntegrity(FilePolicyModeArgs),

    /// Verify `[lane.*]` entries in `policy/ci-lanes.toml` resolve to
    /// real workflow files and (where set) real job display names.
    /// Catches stale `workflow`/`workflow_name`/`job_name` references.
    CheckLaneMappings(FilePolicyModeArgs),

    /// Verify every lane `workflow_name` in `policy/ci-lanes.toml` is
    /// subscribed by `ci-actuals.yml`'s `workflow_run.workflows` list
    /// (and vice versa). Exemptions are explicit in
    /// `[actuals_exemptions].not_subscribed`.
    CheckActualsCoverage(FilePolicyModeArgs),
}

#[derive(Debug, Args)]
pub struct NoPanicArgs {
    #[command(subcommand)]
    command: NoPanicCommand,
}

#[derive(Debug, Subcommand)]
enum NoPanicCommand {
    /// Show diff between source and the on-disk baseline; with `--reset`,
    /// regenerate the baseline file.
    Baseline(BaselineArgs),
}

#[derive(Debug, Args)]
pub struct BaselineArgs {
    /// Regenerate `policy/no-panic-baseline.toml` from a fresh source scan.
    /// This is the only documented way to mutate the baseline; do it in a
    /// dedicated PR.
    #[arg(long)]
    pub reset: bool,
}

#[derive(Debug, Args)]
pub struct FilePolicyModeArgs {
    /// Enforcement mode. `advisory` (default) reports findings without
    /// failing; `blocking-allowlist` exits non-zero on any finding.
    #[arg(long, default_value = "advisory")]
    pub mode: String,
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

    /// PR number (optional; null on push / workflow_dispatch).
    ///
    /// Falls back to `GITHUB_PR_NUMBER` when the flag is omitted; an empty
    /// env value is treated as "no PR" so the actuals lane can run on
    /// `workflow_run` after a push to `main` (where the upstream PR field
    /// resolves to an empty string).
    #[arg(long)]
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
            Command::CheckDocArtifacts => tasks::check_doc_artifacts::run(&workspace_root),
            Command::CheckGoals => tasks::check_goals::run(&workspace_root),
            Command::CheckSupportTiers => tasks::check_support_tiers::run(&workspace_root),
            Command::PackageBoundary => tasks::package_boundary::run(&workspace_root),
            Command::PackageVersion => tasks::package_version::run(&workspace_root),
            Command::PolicyReport => tasks::policy_report::run(&workspace_root),
            Command::RepoContractReport => tasks::repo_contract_report::run(&workspace_root),
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
                    let pr_number = match args.pr_number {
                        Some(n) => Some(n),
                        None => {
                            parse_optional_u32(std::env::var("GITHUB_PR_NUMBER").ok().as_deref())
                                .context("parse GITHUB_PR_NUMBER env var")?
                        }
                    };
                    tasks::ci_actuals::run(tasks::ci_actuals::ActualsInputs {
                        workspace_root,
                        run_id: args.run_id,
                        workflow_name: args.workflow_name,
                        head_sha: args.head_sha,
                        pr_number,
                        jobs_input: args.input,
                        output: args.output,
                    })
                }
            },
            Command::CheckFilePolicy(args) => {
                tasks::file_policy::check_file_policy(&workspace_root, parse_mode(&args.mode)?)
            }
            Command::CheckGenerated(args) => {
                tasks::file_policy::check_generated(&workspace_root, parse_mode(&args.mode)?)
            }
            Command::CheckExecutableFiles(args) => {
                tasks::file_policy::check_executable_files(&workspace_root, parse_mode(&args.mode)?)
            }
            Command::CheckWorkflows(args) => {
                tasks::file_policy::check_workflows(&workspace_root, parse_mode(&args.mode)?)
            }
            Command::CheckDependencySurfaces(args) => {
                tasks::file_policy::check_dependency_surfaces(
                    &workspace_root,
                    parse_mode(&args.mode)?,
                )
            }
            Command::CheckProcessPolicy(args) => {
                tasks::file_policy::check_process_policy(&workspace_root, parse_mode(&args.mode)?)
            }
            Command::CheckNetworkPolicy(args) => {
                tasks::file_policy::check_network_policy(&workspace_root, parse_mode(&args.mode)?)
            }
            Command::CheckLintPolicy(args) => {
                tasks::clippy_policy::check_lint_policy(&workspace_root, parse_mode(&args.mode)?)
            }
            Command::CheckClippyExceptions(args) => tasks::clippy_policy::check_clippy_exceptions(
                &workspace_root,
                parse_mode(&args.mode)?,
            ),
            Command::NoPanic(np) => match np.command {
                NoPanicCommand::Baseline(args) => {
                    tasks::no_panic::baseline(&workspace_root, args.reset)
                }
            },
            Command::CheckNoPanicFamily(args) => {
                tasks::no_panic::check_no_panic_family(&workspace_root, parse_mode(&args.mode)?)
            }
            Command::CheckLabelEnforcement(args) => {
                tasks::check_label_enforcement::run(&workspace_root, parse_mode(&args.mode)?)
            }
            Command::CheckRiskPackIntegrity(args) => {
                tasks::check_risk_pack_integrity::run(&workspace_root, parse_mode(&args.mode)?)
            }
            Command::CheckLaneMappings(args) => {
                tasks::check_lane_mappings::run(&workspace_root, parse_mode(&args.mode)?)
            }
            Command::CheckActualsCoverage(args) => {
                tasks::check_actuals_coverage::run(&workspace_root, parse_mode(&args.mode)?)
            }
        }
    }
}

fn parse_mode(s: &str) -> Result<tasks::file_policy::Mode> {
    match s {
        "advisory" => Ok(tasks::file_policy::Mode::Advisory),
        "blocking-allowlist" => Ok(tasks::file_policy::Mode::BlockingAllowlist),
        other => Err(anyhow::anyhow!(
            "invalid --mode {other:?}; expected `advisory` or `blocking-allowlist`"
        )),
    }
}

/// Parse an optional `u32` from a string env value, treating absent and
/// empty strings as `None`. Bridges clap's `Option<u32>` env-arg semantics
/// to GitHub Actions, where workflow_run env vars can resolve to the empty
/// string on push / workflow_dispatch (and clap rejects `""` as a u32).
fn parse_optional_u32(value: Option<&str>) -> Result<Option<u32>> {
    match value {
        None | Some("") => Ok(None),
        Some(s) => s
            .parse::<u32>()
            .map(Some)
            .with_context(|| format!("parse {s:?} as u32")),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_optional_u32;

    #[test]
    fn absent_env_returns_none() {
        assert_eq!(parse_optional_u32(None).unwrap(), None);
    }

    #[test]
    fn empty_env_returns_none() {
        assert_eq!(parse_optional_u32(Some("")).unwrap(), None);
    }

    #[test]
    fn integer_env_returns_some() {
        assert_eq!(parse_optional_u32(Some("157")).unwrap(), Some(157));
    }

    #[test]
    fn non_integer_env_errors() {
        assert!(parse_optional_u32(Some("not-a-number")).is_err());
    }
}
