//! `shiplog` CLI entrypoint.
//!
//! Exposes `collect`, `render`, `refresh`, `import`, and `run` commands over
//! the workspace engine and adapter crates.

use anyhow::{Context, Result};
use chrono::{Datelike, Months, NaiveDate, Utc};
use clap::{Args, Parser, Subcommand, ValueEnum};
use reqwest::blocking::Client;
use serde::Deserialize;
use shiplog_engine::{Engine, WorkstreamSource};
use shiplog_ingest_git::LocalGitIngestor;
use shiplog_ingest_github::GithubIngestor;
use shiplog_ingest_gitlab::{GitlabIngestor, MrState};
use shiplog_ingest_jira::{IssueStatus, JiraIngestor};
use shiplog_ingest_json::JsonIngestor;
use shiplog_ingest_linear::{IssueStatus as LinearIssueStatus, LinearIngestor};
use shiplog_ingest_manual::ManualIngestor;
use shiplog_ports::Ingestor;
use shiplog_redact::DeterministicRedactor;
use shiplog_render_md::MarkdownRenderer;
use shiplog_schema::bundle::BundleProfile;
use shiplog_workstreams::RepoClusterer;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(name = "shiplog", version)]
#[command(about = "Generate self-review packets with receipts + coverage.", long_about = None)]
struct Cli {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Create a local shiplog.toml and manual_events.yaml scaffold.
    Init {
        /// Sources to enable in the generated config.
        #[arg(long = "source", value_enum)]
        sources: Vec<InitSource>,
        /// Print generated files instead of writing them.
        #[arg(long)]
        dry_run: bool,
        /// Overwrite existing shiplog.toml or manual_events.yaml.
        #[arg(long)]
        force: bool,
    },

    /// Collect events from a source and generate workstream suggestions.
    ///
    /// This creates `workstreams.suggested.yaml` which you can rename to
    /// `workstreams.yaml` and edit to curate your narrative.
    Collect {
        #[command(subcommand)]
        source: Source,
        /// Output directory (a run folder will be created inside).
        #[arg(long, default_value = "./out")]
        out: PathBuf,
        /// Also write a zip next to the run folder.
        #[arg(long)]
        zip: bool,
        /// Redaction key. If omitted, SHIPLOG_REDACT_KEY is used.
        #[arg(long)]
        redact_key: Option<String>,
        /// Bundle profile: internal (full), manager, or public.
        #[arg(long, default_value = "internal")]
        bundle_profile: BundleProfile,
        /// Regenerate workstreams even if workstreams.yaml exists.
        /// WARNING: This will not overwrite workstreams.yaml, but will
        /// regenerate workstreams.suggested.yaml.
        #[arg(long)]
        regen: bool,
        /// Use LLM-assisted workstream clustering instead of repo-based.
        #[arg(long)]
        llm_cluster: bool,
        /// LLM API endpoint (OpenAI-compatible).
        #[arg(long, default_value = "https://api.openai.com/v1/chat/completions")]
        llm_api_endpoint: String,
        /// LLM model name.
        #[arg(long, default_value = "gpt-4o-mini")]
        llm_model: String,
        /// LLM API key (or set SHIPLOG_LLM_API_KEY).
        #[arg(long)]
        llm_api_key: Option<String>,
    },

    /// Render packets from existing events and workstreams.
    ///
    /// Use this after editing workstreams.yaml to regenerate the packet
    /// without fetching new data.
    Render {
        /// Directory containing ledger.events.jsonl and workstreams.yaml
        #[arg(long, default_value = "./out")]
        out: PathBuf,
        /// Run ID to render (uses most recent if not specified)
        #[arg(long)]
        run: Option<String>,
        /// Render the most recent run explicitly.
        #[arg(long)]
        latest: bool,
        /// User label for rendering.
        #[arg(long, default_value = "user")]
        user: String,
        /// Window label for rendering.
        #[arg(long, default_value = "window")]
        window_label: String,
        /// Redaction key. If omitted, SHIPLOG_REDACT_KEY is used.
        #[arg(long)]
        redact_key: Option<String>,
        /// Bundle profile: internal (full), manager, or public.
        #[arg(long, default_value = "internal")]
        bundle_profile: BundleProfile,
        /// Also write a zip next to the run folder.
        #[arg(long)]
        zip: bool,
    },

    /// Refresh event data while preserving workstream curation.
    ///
    /// This re-fetches events from the source and updates receipts/stats,
    /// but preserves your workstreams.yaml edits.
    Refresh {
        #[command(subcommand)]
        source: Source,
        /// Output directory containing existing workstreams.yaml
        #[arg(long, default_value = "./out")]
        out: PathBuf,
        /// Explicit run directory to refresh into (overrides auto-detection).
        #[arg(long)]
        run_dir: Option<PathBuf>,
        /// Also write a zip next to the run folder.
        #[arg(long)]
        zip: bool,
        /// Redaction key. If omitted, SHIPLOG_REDACT_KEY is used.
        #[arg(long)]
        redact_key: Option<String>,
        /// Bundle profile: internal (full), manager, or public.
        #[arg(long, default_value = "internal")]
        bundle_profile: BundleProfile,
    },

    /// Import a pre-built ledger directory and run the full render pipeline.
    ///
    /// Use this to consume output from an upstream system or a previous
    /// shiplog run and re-render it.
    Import {
        /// Directory containing ledger.events.jsonl and coverage.manifest.json
        #[arg(long)]
        dir: PathBuf,
        /// Output directory (a run folder will be created inside).
        #[arg(long, default_value = "./out")]
        out: PathBuf,
        /// User label for rendering.
        #[arg(long, default_value = "user")]
        user: String,
        /// Window label for rendering.
        #[arg(long, default_value = "window")]
        window_label: String,
        /// Redaction key. If omitted, SHIPLOG_REDACT_KEY is used.
        #[arg(long)]
        redact_key: Option<String>,
        /// Bundle profile: internal (full), manager, or public.
        #[arg(long, default_value = "internal")]
        bundle_profile: BundleProfile,
        /// Also write a zip next to the run folder.
        #[arg(long)]
        zip: bool,
        /// Ignore imported workstreams and re-cluster from events.
        #[arg(long)]
        regen: bool,
        /// Use LLM-assisted workstream clustering instead of repo-based.
        #[arg(long)]
        llm_cluster: bool,
        /// LLM API endpoint (OpenAI-compatible).
        #[arg(long, default_value = "https://api.openai.com/v1/chat/completions")]
        llm_api_endpoint: String,
        /// LLM model name.
        #[arg(long, default_value = "gpt-4o-mini")]
        llm_model: String,
        /// LLM API key (or set SHIPLOG_LLM_API_KEY).
        #[arg(long)]
        llm_api_key: Option<String>,
    },

    /// Run the full pipeline (collect + render) in one command.
    ///
    /// This is the legacy mode that combines collect and render.
    /// For more control, use `collect` and `render` separately.
    Run {
        #[command(subcommand)]
        source: Source,
        /// Output directory (a run folder will be created inside).
        #[arg(long, default_value = "./out")]
        out: PathBuf,
        /// Also write a zip next to the run folder.
        #[arg(long)]
        zip: bool,
        /// Redaction key. If omitted, SHIPLOG_REDACT_KEY is used.
        #[arg(long)]
        redact_key: Option<String>,
        /// Bundle profile: internal (full), manager, or public.
        #[arg(long, default_value = "internal")]
        bundle_profile: BundleProfile,
        /// Use LLM-assisted workstream clustering instead of repo-based.
        #[arg(long)]
        llm_cluster: bool,
        /// LLM API endpoint (OpenAI-compatible).
        #[arg(long, default_value = "https://api.openai.com/v1/chat/completions")]
        llm_api_endpoint: String,
        /// LLM model name.
        #[arg(long, default_value = "gpt-4o-mini")]
        llm_model: String,
        /// LLM API key (or set SHIPLOG_LLM_API_KEY).
        #[arg(long)]
        llm_api_key: Option<String>,
    },
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
enum InitSource {
    Github,
    Gitlab,
    Jira,
    Linear,
    Git,
    Json,
    Manual,
}

#[derive(Subcommand, Debug, Clone)]
enum Source {
    /// Ingest from GitHub (public + authenticated private).
    Github {
        /// GitHub login to report on.
        #[arg(long)]
        user: Option<String>,
        /// Infer the GitHub login from the authenticated token.
        #[arg(long)]
        me: bool,
        #[command(flatten)]
        window: DateArgs,
        /// "merged" (default) or "created"
        #[arg(long, default_value = "merged")]
        mode: String,
        /// Include review activity (best-effort).
        #[arg(long)]
        include_reviews: bool,
        /// Don't fetch per-PR details (additions/deletions/changed_files).
        #[arg(long)]
        no_details: bool,
        /// Milliseconds to sleep between requests.
        #[arg(long, default_value_t = 0)]
        throttle_ms: u64,
        /// GitHub token (or set GITHUB_TOKEN).
        #[arg(long)]
        token: Option<String>,
        /// API base for GHES.
        #[arg(long, default_value = "https://api.github.com")]
        api_base: String,
        /// Override GitHub API cache directory (defaults to `<out>/.cache`).
        #[arg(long)]
        cache_dir: Option<PathBuf>,
        /// Disable GitHub API caching.
        #[arg(long)]
        no_cache: bool,
    },

    /// Ingest from GitLab merge requests and review notes.
    Gitlab {
        /// GitLab username to report on.
        #[arg(long)]
        user: Option<String>,
        /// Infer the GitLab username from the authenticated token.
        #[arg(long)]
        me: bool,
        #[command(flatten)]
        window: DateArgs,
        /// Merge request state: opened, merged, closed, or all.
        #[arg(long, default_value = "merged")]
        state: String,
        /// GitLab instance hostname or URL.
        #[arg(long, default_value = "gitlab.com")]
        instance: String,
        /// Include review activity from merge request notes (best-effort).
        #[arg(long)]
        include_reviews: bool,
        /// Milliseconds to sleep between requests.
        #[arg(long, default_value_t = 0)]
        throttle_ms: u64,
        /// GitLab token (or set GITLAB_TOKEN).
        #[arg(long)]
        token: Option<String>,
        /// Override GitLab API cache directory (defaults to `<out>/.cache`).
        #[arg(long)]
        cache_dir: Option<PathBuf>,
        /// Disable GitLab API caching.
        #[arg(long)]
        no_cache: bool,
    },

    /// Ingest from Jira issues assigned to a Jira identity.
    Jira {
        /// Jira assignee JQL value to report on, usually an account ID or email.
        #[arg(long)]
        user: String,
        /// Jira Basic Auth username/email. Defaults to `--user`.
        #[arg(long)]
        auth_user: Option<String>,
        #[command(flatten)]
        window: DateArgs,
        /// Issue status: open, in_progress, done, closed, or all.
        #[arg(long, default_value = "done")]
        status: String,
        /// Jira instance hostname or URL.
        #[arg(long)]
        instance: String,
        /// Milliseconds to sleep between requests.
        #[arg(long, default_value_t = 0)]
        throttle_ms: u64,
        /// Jira API token (or set JIRA_TOKEN).
        #[arg(long)]
        token: Option<String>,
        /// Override Jira API cache directory (defaults to `<out>/.cache`).
        #[arg(long)]
        cache_dir: Option<PathBuf>,
        /// Disable Jira API caching.
        #[arg(long)]
        no_cache: bool,
    },

    /// Ingest from Linear issues assigned to a Linear user ID.
    Linear {
        /// Linear user UUID to report on.
        #[arg(long)]
        user_id: String,
        #[command(flatten)]
        window: DateArgs,
        /// Issue status: backlog, todo, in_progress, done, cancelled, or all.
        #[arg(long, default_value = "done")]
        status: String,
        /// Optional Linear project key filter.
        #[arg(long)]
        project: Option<String>,
        /// Milliseconds to sleep between requests.
        #[arg(long, default_value_t = 0)]
        throttle_ms: u64,
        /// Linear API key (or set LINEAR_API_KEY).
        #[arg(long)]
        api_key: Option<String>,
        /// Override Linear API cache directory (defaults to `<out>/.cache`).
        #[arg(long)]
        cache_dir: Option<PathBuf>,
        /// Disable Linear API caching.
        #[arg(long)]
        no_cache: bool,
    },

    /// Ingest from JSONL events + a coverage manifest.
    Json {
        #[arg(long)]
        events: PathBuf,
        #[arg(long)]
        coverage: PathBuf,
        /// Optional user label for rendering.
        #[arg(long, default_value = "user")]
        user: String,
        /// Optional window label for rendering.
        #[arg(long, default_value = "window")]
        window_label: String,
    },

    /// Ingest manual non-GitHub events from YAML.
    Manual {
        /// Path to manual events YAML file.
        #[arg(long)]
        events: PathBuf,
        /// User label for rendering.
        #[arg(long, default_value = "user")]
        user: String,
        #[command(flatten)]
        window: DateArgs,
    },

    /// Ingest from local git repository.
    Git {
        /// Path to git repository.
        #[arg(long)]
        repo: PathBuf,
        #[command(flatten)]
        window: DateArgs,
        /// Filter commits by author email.
        #[arg(long)]
        author: Option<String>,
        /// Include merge commits.
        #[arg(long)]
        include_merges: bool,
    },
}

#[derive(Args, Debug, Clone)]
struct DateArgs {
    /// Start date (inclusive), YYYY-MM-DD.
    #[arg(long)]
    since: Option<NaiveDate>,
    /// End date (exclusive), YYYY-MM-DD.
    #[arg(long)]
    until: Option<NaiveDate>,
    /// Use the last six months, ending today.
    #[arg(long)]
    last_6_months: bool,
    /// Use the previous calendar quarter.
    #[arg(long)]
    last_quarter: bool,
    /// Use a calendar year.
    #[arg(long)]
    year: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ResolvedWindow {
    since: NaiveDate,
    until: NaiveDate,
    label: WindowLabel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowLabel {
    Explicit,
    LastSixMonths,
    LastQuarter,
    Year(i32),
}

const CONFIG_FILENAME: &str = "shiplog.toml";
const MANUAL_EVENTS_FILENAME: &str = "manual_events.yaml";

fn get_redact_key(redact_key: Option<String>) -> String {
    redact_key
        .or_else(|| std::env::var("SHIPLOG_REDACT_KEY").ok())
        .unwrap_or_else(|| {
            eprintln!("WARN: no redaction key provided; using a default dev key. Don't share public packets like this.");
            "dev-key".to_string()
        })
}

impl ResolvedWindow {
    fn window_label(&self) -> String {
        match self.label {
            WindowLabel::Explicit => format!("{}..{}", self.since, self.until),
            WindowLabel::LastSixMonths => {
                format!("last-6-months ({}..{})", self.since, self.until)
            }
            WindowLabel::LastQuarter => {
                format!("last-quarter ({}..{})", self.since, self.until)
            }
            WindowLabel::Year(year) => format!("{year} ({}..{})", self.since, self.until),
        }
    }
}

fn resolve_date_window(args: DateArgs) -> Result<ResolvedWindow> {
    resolve_date_window_for_today(args, Utc::now().date_naive())
}

fn resolve_date_window_for_today(args: DateArgs, today: NaiveDate) -> Result<ResolvedWindow> {
    match (args.since, args.until) {
        (Some(since), Some(until)) => return checked_window(since, until, WindowLabel::Explicit),
        (Some(_), None) | (None, Some(_)) => {
            anyhow::bail!("provide both --since and --until, or use a date preset")
        }
        (None, None) => {}
    }

    let preset_count = usize::from(args.last_6_months)
        + usize::from(args.last_quarter)
        + usize::from(args.year.is_some());
    if preset_count > 1 {
        anyhow::bail!("choose only one date preset: --last-6-months, --last-quarter, or --year")
    }

    if let Some(year) = args.year {
        let since = NaiveDate::from_ymd_opt(year, 1, 1)
            .ok_or_else(|| anyhow::anyhow!("invalid --year value: {year}"))?;
        let until = NaiveDate::from_ymd_opt(year + 1, 1, 1)
            .ok_or_else(|| anyhow::anyhow!("invalid --year value: {year}"))?;
        return checked_window(since, until, WindowLabel::Year(year));
    }

    if args.last_quarter {
        let start_of_current_quarter = quarter_start(today.year(), today.month())?;
        let previous_quarter_anchor = start_of_current_quarter
            .checked_sub_months(Months::new(3))
            .ok_or_else(|| anyhow::anyhow!("could not resolve --last-quarter"))?;
        return checked_window(
            previous_quarter_anchor,
            start_of_current_quarter,
            WindowLabel::LastQuarter,
        );
    }

    let since = today
        .checked_sub_months(Months::new(6))
        .ok_or_else(|| anyhow::anyhow!("could not resolve --last-6-months"))?;
    checked_window(since, today, WindowLabel::LastSixMonths)
}

fn checked_window(
    since: NaiveDate,
    until: NaiveDate,
    label: WindowLabel,
) -> Result<ResolvedWindow> {
    if since >= until {
        anyhow::bail!("date window must satisfy --since < --until")
    }
    Ok(ResolvedWindow {
        since,
        until,
        label,
    })
}

fn quarter_start(year: i32, month: u32) -> Result<NaiveDate> {
    let start_month = match month {
        1..=3 => 1,
        4..=6 => 4,
        7..=9 => 7,
        10..=12 => 10,
        _ => anyhow::bail!("invalid month while resolving quarter: {month}"),
    };
    NaiveDate::from_ymd_opt(year, start_month, 1)
        .ok_or_else(|| anyhow::anyhow!("invalid quarter start for {year}-{start_month:02}"))
}

fn run_init(sources: Vec<InitSource>, dry_run: bool, force: bool) -> Result<()> {
    let selected = selected_init_sources(&sources);
    let config = render_init_config(&selected);
    let manual_events = render_manual_events_template();

    if dry_run {
        println!("Would write {CONFIG_FILENAME}:\n\n{config}");
        println!("Would write {MANUAL_EVENTS_FILENAME}:\n\n{manual_events}");
        return Ok(());
    }

    let config_path = Path::new(CONFIG_FILENAME);
    let manual_events_path = Path::new(MANUAL_EVENTS_FILENAME);
    ensure_init_files_available(&[config_path, manual_events_path], force)?;

    write_init_file(config_path, &config)?;
    write_init_file(manual_events_path, &manual_events)?;

    println!("Initialized shiplog:");
    println!("  {CONFIG_FILENAME}");
    println!("  {MANUAL_EVENTS_FILENAME}");
    println!();
    println!("Next:");
    println!("  edit {CONFIG_FILENAME}");
    for env_var in init_env_vars(&selected) {
        println!("  export {env_var}=...");
    }
    println!("  {}", init_next_command(&selected));

    Ok(())
}

fn selected_init_sources(sources: &[InitSource]) -> Vec<InitSource> {
    if sources.is_empty() {
        return vec![InitSource::Github, InitSource::Manual];
    }

    let mut selected = Vec::new();
    for source in sources {
        if !selected.contains(source) {
            selected.push(*source);
        }
    }
    selected
}

fn init_source_enabled(selected: &[InitSource], source: InitSource) -> bool {
    selected.contains(&source)
}

fn init_env_vars(selected: &[InitSource]) -> Vec<&'static str> {
    let mut vars = Vec::new();
    if init_source_enabled(selected, InitSource::Github) {
        vars.push("GITHUB_TOKEN");
    }
    if init_source_enabled(selected, InitSource::Gitlab) {
        vars.push("GITLAB_TOKEN");
    }
    if init_source_enabled(selected, InitSource::Jira) {
        vars.push("JIRA_TOKEN");
    }
    if init_source_enabled(selected, InitSource::Linear) {
        vars.push("LINEAR_API_KEY");
    }
    vars
}

fn init_next_command(selected: &[InitSource]) -> &'static str {
    if init_source_enabled(selected, InitSource::Github) {
        "shiplog collect github --me --last-6-months"
    } else if init_source_enabled(selected, InitSource::Gitlab) {
        "shiplog collect gitlab --me --last-6-months"
    } else if init_source_enabled(selected, InitSource::Jira) {
        "shiplog collect jira --user <account-id-or-email> --auth-user <email> --last-6-months"
    } else if init_source_enabled(selected, InitSource::Linear) {
        "shiplog collect linear --user-id <linear-user-id> --last-6-months"
    } else if init_source_enabled(selected, InitSource::Git) {
        "shiplog collect git --repo . --last-6-months"
    } else if init_source_enabled(selected, InitSource::Json) {
        "shiplog collect json --events <events.jsonl> --coverage <coverage.manifest.json>"
    } else {
        "shiplog collect manual --events manual_events.yaml --user <label> --last-6-months"
    }
}

fn ensure_init_files_available(paths: &[&Path], force: bool) -> Result<()> {
    if force {
        return Ok(());
    }

    for path in paths {
        if path.exists() {
            anyhow::bail!(
                "{} already exists; use --force to overwrite",
                path.display()
            );
        }
    }

    Ok(())
}

fn write_init_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }

    std::fs::write(path, contents).with_context(|| format!("write {}", path.display()))
}

fn render_init_config(selected: &[InitSource]) -> String {
    let github = init_source_enabled(selected, InitSource::Github);
    let gitlab = init_source_enabled(selected, InitSource::Gitlab);
    let jira = init_source_enabled(selected, InitSource::Jira);
    let linear = init_source_enabled(selected, InitSource::Linear);
    let git = init_source_enabled(selected, InitSource::Git);
    let json = init_source_enabled(selected, InitSource::Json);
    let manual = init_source_enabled(selected, InitSource::Manual);

    format!(
        r#"# shiplog local configuration.
# Tokens stay in environment variables:
# GITHUB_TOKEN, GITLAB_TOKEN, JIRA_TOKEN, LINEAR_API_KEY, SHIPLOG_REDACT_KEY.

[defaults]
out = "./out"
window = "last-6-months"
profile = "internal"
include_reviews = true

[user]
label = "Your Name"

[sources.github]
enabled = {github}
user = "your-github-username"
mode = "merged"
include_reviews = true

[sources.gitlab]
enabled = {gitlab}
user = "your-gitlab-username"
instance = "gitlab.com"
state = "merged"
include_reviews = true

[sources.jira]
enabled = {jira}
user = "your-jira-account-id-or-email"
auth_user_env = "JIRA_AUTH_USER"
instance = "company.atlassian.net"
status = "done"

[sources.linear]
enabled = {linear}
user_id = "your-linear-user-id"
status = "done"
project = ""

[sources.git]
enabled = {git}
repo = "."
author = ""
include_merges = false

[sources.json]
enabled = {json}
events = "./ledger.events.jsonl"
coverage = "./coverage.manifest.json"

[sources.manual]
enabled = {manual}
events = "./manual_events.yaml"
user = "Your Name"

[redaction]
key_env = "SHIPLOG_REDACT_KEY"
"#
    )
}

fn render_manual_events_template() -> String {
    let generated_at = Utc::now().to_rfc3339();
    format!(
        r#"# Manual evidence entries for shiplog.
# Add events for work that is not visible in code-hosting or issue systems.
# Keep entries factual and receipt-oriented.
#
# Example:
# events:
#   - id: manual-2026-01-15-incident-follow-up
#     type: Note
#     date: "2026-01-15"
#     title: "Incident follow-up"
#     description: "Summarize the work, not the performance narrative."
#     workstream: "platform reliability"
#     tags: ["review-cycle"]
#     receipts:
#       - label: "runbook"
#         url: "https://example.com/runbook"
#     impact: "Reduced operational risk."

version: 1
generated_at: "{generated_at}"
events: []
"#
    )
}

fn resolve_user_or_me(
    source: &str,
    explicit_user: Option<String>,
    me: bool,
    discover: impl FnOnce() -> Result<String>,
) -> Result<String> {
    match (explicit_user, me) {
        (Some(_), true) => anyhow::bail!("use either --user or --me for {source}, not both"),
        (Some(user), false) => Ok(user),
        (None, true) => discover(),
        (None, false) => {
            anyhow::bail!("provide --user <username> or --me for {source}")
        }
    }
}

#[derive(Deserialize)]
struct GithubAuthenticatedUser {
    login: String,
}

#[derive(Deserialize)]
struct GitlabAuthenticatedUser {
    username: String,
}

fn discover_github_user(api_base: &str, token: Option<&str>) -> Result<String> {
    let token = token
        .map(ToOwned::to_owned)
        .or_else(|| std::env::var("GITHUB_TOKEN").ok())
        .ok_or_else(|| {
            anyhow::anyhow!("Could not infer GitHub user: --me requires --token or GITHUB_TOKEN")
        })?;

    let client = identity_client()?;
    let url = format!("{}/user", api_base.trim_end_matches('/'));
    let resp = client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .bearer_auth(token)
        .send()
        .with_context(|| format!("GET {url}"))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().unwrap_or_default();
        anyhow::bail!("Could not infer GitHub user: GitHub API error {status}: {body}");
    }

    let user = resp
        .json::<GithubAuthenticatedUser>()
        .with_context(|| format!("parse GitHub authenticated user from {url}"))?;

    if user.login.trim().is_empty() {
        anyhow::bail!("Could not infer GitHub user: authenticated user response had empty login");
    }

    Ok(user.login)
}

fn discover_gitlab_user(instance: &str, token: Option<&str>) -> Result<String> {
    let token = token
        .map(ToOwned::to_owned)
        .or_else(|| std::env::var("GITLAB_TOKEN").ok())
        .ok_or_else(|| {
            anyhow::anyhow!("Could not infer GitLab user: --me requires --token or GITLAB_TOKEN")
        })?;

    let client = identity_client()?;
    let url = format!("{}/user", gitlab_api_base(instance)?);
    let resp = client
        .get(&url)
        .header("Accept", "application/json")
        .header("PRIVATE-TOKEN", token)
        .send()
        .with_context(|| format!("GET {url}"))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().unwrap_or_default();
        anyhow::bail!("Could not infer GitLab user: GitLab API error {status}: {body}");
    }

    let user = resp
        .json::<GitlabAuthenticatedUser>()
        .with_context(|| format!("parse GitLab authenticated user from {url}"))?;

    if user.username.trim().is_empty() {
        anyhow::bail!(
            "Could not infer GitLab user: authenticated user response had empty username"
        );
    }

    Ok(user.username)
}

fn gitlab_api_base(instance: &str) -> Result<String> {
    if instance.trim().is_empty() {
        anyhow::bail!("GitLab instance cannot be empty");
    }

    let hostname = if instance.contains("://") {
        reqwest::Url::parse(instance)
            .ok()
            .and_then(|u| u.host_str().map(|s| s.to_string()))
            .ok_or_else(|| anyhow::anyhow!("Invalid GitLab instance URL: {instance}"))?
    } else {
        instance.to_string()
    };

    Ok(format!("https://{hostname}/api/v4"))
}

fn identity_client() -> Result<Client> {
    Client::builder()
        .user_agent(concat!("shiplog/", env!("CARGO_PKG_VERSION")))
        .build()
        .context("build identity discovery HTTP client")
}

fn create_engine(
    redact_key: &str,
    clusterer: Box<dyn shiplog_ports::WorkstreamClusterer>,
) -> (Engine<'static>, &'static DeterministicRedactor) {
    let renderer = Box::new(MarkdownRenderer::default());
    let redactor = DeterministicRedactor::new(redact_key.as_bytes());

    // We need to leak these to give them 'static lifetime
    // This is acceptable for a CLI tool that runs once
    let renderer: &'static dyn shiplog_ports::Renderer = Box::leak(renderer);
    let clusterer: &'static dyn shiplog_ports::WorkstreamClusterer = Box::leak(clusterer);
    let redactor_box = Box::new(redactor);
    let redactor_ref: &'static DeterministicRedactor = Box::leak(redactor_box);
    let redactor_trait: &'static dyn shiplog_ports::Redactor = redactor_ref;

    (
        Engine::new(renderer, clusterer, redactor_trait),
        redactor_ref,
    )
}

fn build_clusterer(
    llm_cluster: bool,
    llm_api_endpoint: &str,
    llm_model: &str,
    llm_api_key: Option<String>,
) -> Box<dyn shiplog_ports::WorkstreamClusterer> {
    if llm_cluster {
        #[cfg(feature = "llm")]
        {
            eprintln!(
                "WARN: --llm-cluster sends event summaries (PR titles, repo names) to {llm_api_endpoint}"
            );
            let api_key = llm_api_key
                .or_else(|| std::env::var("SHIPLOG_LLM_API_KEY").ok())
                .unwrap_or_else(|| {
                    eprintln!("ERROR: --llm-cluster requires --llm-api-key or SHIPLOG_LLM_API_KEY");
                    std::process::exit(1);
                });

            let backend = shiplog_cluster_llm::OpenAiCompatibleBackend {
                endpoint: llm_api_endpoint.to_string(),
                api_key,
                model: llm_model.to_string(),
                temperature: 0.2,
                timeout_secs: 60,
            };
            let config = shiplog_cluster_llm::LlmConfig {
                api_endpoint: llm_api_endpoint.to_string(),
                api_key: String::new(),
                model: llm_model.to_string(),
                ..Default::default()
            };
            let llm = shiplog_cluster_llm::LlmClusterer::new(Box::new(backend), config);
            Box::new(shiplog_cluster_llm::LlmWithFallback::new(llm))
        }
        #[cfg(not(feature = "llm"))]
        {
            let _ = (llm_api_endpoint, llm_model, llm_api_key);
            eprintln!(
                "ERROR: --llm-cluster requires the 'llm' feature. Rebuild with: cargo build -p shiplog --features llm"
            );
            std::process::exit(1);
        }
    } else {
        Box::new(RepoClusterer)
    }
}

fn resolve_cache_dir(
    out_root: &Path,
    explicit_cache_dir: Option<PathBuf>,
    no_cache: bool,
) -> Option<PathBuf> {
    if no_cache {
        None
    } else {
        Some(explicit_cache_dir.unwrap_or_else(|| out_root.join(".cache")))
    }
}

#[allow(clippy::too_many_arguments)]
fn make_github_ingestor(
    user: &str,
    since: NaiveDate,
    until: NaiveDate,
    mode: &str,
    include_reviews: bool,
    no_details: bool,
    throttle_ms: u64,
    token: Option<String>,
    api_base: &str,
    cache_dir: Option<PathBuf>,
) -> Result<GithubIngestor> {
    let token = token.or_else(|| std::env::var("GITHUB_TOKEN").ok());

    let mut ing = GithubIngestor::new(user.to_string(), since, until);
    ing.mode = mode.to_string();
    ing.include_reviews = include_reviews;
    ing.fetch_details = !no_details;
    ing.throttle_ms = throttle_ms;
    ing.token = token;
    ing.api_base = api_base.to_string();

    if let Some(cache_dir) = cache_dir {
        ing = ing
            .with_cache(cache_dir)
            .context("configure GitHub API cache")?;
    }

    Ok(ing)
}

#[allow(clippy::too_many_arguments)]
fn make_gitlab_ingestor(
    user: &str,
    since: NaiveDate,
    until: NaiveDate,
    state: &str,
    instance: &str,
    include_reviews: bool,
    throttle_ms: u64,
    token: Option<String>,
    cache_dir: Option<PathBuf>,
) -> Result<GitlabIngestor> {
    let token = token.or_else(|| std::env::var("GITLAB_TOKEN").ok());
    let state = state
        .parse::<MrState>()
        .with_context(|| format!("parse GitLab MR state {state:?}"))?;

    let mut ing = GitlabIngestor::new(user.to_string(), since, until)
        .with_state(state)
        .with_include_reviews(include_reviews)
        .with_throttle(throttle_ms)
        .with_instance(instance.to_string())
        .context("configure GitLab instance")?;

    if let Some(token) = token {
        ing = ing.with_token(token).context("configure GitLab token")?;
    }

    if let Some(cache_dir) = cache_dir {
        ing = ing
            .with_cache(cache_dir)
            .context("configure GitLab API cache")?;
    }

    Ok(ing)
}

#[allow(clippy::too_many_arguments)]
fn make_jira_ingestor(
    user: &str,
    auth_user: Option<String>,
    since: NaiveDate,
    until: NaiveDate,
    status: &str,
    instance: &str,
    throttle_ms: u64,
    token: Option<String>,
    cache_dir: Option<PathBuf>,
) -> Result<JiraIngestor> {
    let token = token.or_else(|| std::env::var("JIRA_TOKEN").ok());
    let status = status
        .parse::<IssueStatus>()
        .with_context(|| format!("parse Jira issue status {status:?}"))?;

    let mut ing = JiraIngestor::new(user.to_string(), since, until)
        .with_status(status)
        .with_throttle(throttle_ms)
        .with_instance(instance.to_string())
        .context("configure Jira instance")?;

    if let Some(auth_user) = auth_user {
        ing = ing
            .with_auth_user(auth_user)
            .context("configure Jira auth user")?;
    }

    if let Some(token) = token {
        ing = ing.with_token(token).context("configure Jira token")?;
    }

    if let Some(cache_dir) = cache_dir {
        ing = ing
            .with_cache(cache_dir)
            .context("configure Jira API cache")?;
    }

    Ok(ing)
}

#[allow(clippy::too_many_arguments)]
fn make_linear_ingestor(
    user_id: &str,
    since: NaiveDate,
    until: NaiveDate,
    status: &str,
    project: Option<String>,
    throttle_ms: u64,
    api_key: Option<String>,
    cache_dir: Option<PathBuf>,
) -> Result<LinearIngestor> {
    let api_key = api_key.or_else(|| std::env::var("LINEAR_API_KEY").ok());
    let status = status
        .parse::<LinearIssueStatus>()
        .with_context(|| format!("parse Linear issue status {status:?}"))?;

    let mut ing = LinearIngestor::new(user_id.to_string(), since, until)
        .with_status(status)
        .with_throttle(throttle_ms);

    if let Some(project) = project {
        ing = ing.with_project(project);
    }

    if let Some(api_key) = api_key {
        ing = ing
            .with_api_key(api_key)
            .context("configure Linear API key")?;
    }

    if let Some(cache_dir) = cache_dir {
        ing = ing
            .with_cache(cache_dir)
            .context("configure Linear API cache")?;
    }

    Ok(ing)
}

fn make_git_ingestor(
    repo: &Path,
    since: NaiveDate,
    until: NaiveDate,
    author: Option<String>,
    include_merges: bool,
) -> LocalGitIngestor {
    let mut ing = LocalGitIngestor::new(repo, since, until);
    if let Some(author) = author {
        ing = ing.with_author(author);
    }
    if include_merges {
        ing = ing.with_merges(true);
    }
    ing
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.cmd {
        Command::Init {
            sources,
            dry_run,
            force,
        } => {
            run_init(sources, dry_run, force)?;
        }

        Command::Collect {
            source,
            out,
            zip,
            redact_key,
            bundle_profile,
            regen,
            llm_cluster,
            llm_api_endpoint,
            llm_model,
            llm_api_key,
        } => {
            let key = get_redact_key(redact_key);
            let clusterer =
                build_clusterer(llm_cluster, &llm_api_endpoint, &llm_model, llm_api_key);
            let (engine, redactor) = create_engine(&key, clusterer);

            match source {
                Source::Github {
                    user,
                    me,
                    window,
                    mode,
                    include_reviews,
                    no_details,
                    throttle_ms,
                    token,
                    api_base,
                    cache_dir,
                    no_cache,
                } => {
                    let user = resolve_user_or_me("GitHub", user, me, || {
                        discover_github_user(&api_base, token.as_deref())
                    })?;
                    let window = resolve_date_window(window)?;
                    let cache_dir = resolve_cache_dir(&out, cache_dir, no_cache);
                    let ing = make_github_ingestor(
                        &user,
                        window.since,
                        window.until,
                        &mode,
                        include_reviews,
                        no_details,
                        throttle_ms,
                        token,
                        &api_base,
                        cache_dir,
                    )
                    .context("create GitHub ingestor")?;
                    let ingest = ing.ingest().context("ingest events")?;
                    let run_id = ingest.coverage.run_id.to_string();
                    let run_dir = out.join(&run_id);

                    let window_label = window.window_label();

                    // Check if user has curated workstreams and warn
                    if !regen && shiplog_workstreams::WorkstreamManager::has_curated(&run_dir) {
                        eprintln!("Note: Using existing workstreams.yaml (user-curated).");
                        eprintln!("      Use --regen to regenerate suggestions.");
                    }

                    // If --regen, delete existing suggested workstreams so the engine regenerates them
                    if regen {
                        let suggested =
                            shiplog_workstreams::WorkstreamManager::suggested_path(&run_dir);
                        if suggested.exists() {
                            std::fs::remove_file(&suggested)
                                .with_context(|| format!("remove {:?} for --regen", suggested))?;
                        }
                    }

                    let cache_path = DeterministicRedactor::cache_path(&run_dir);
                    let _ = redactor.load_cache(&cache_path);

                    let (outputs, ws_source) = engine
                        .run(ingest, &user, &window_label, &run_dir, zip, &bundle_profile)
                        .context("run engine pipeline")?;

                    redactor
                        .save_cache(&cache_path)
                        .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

                    println!("Collected and wrote:");
                    print_outputs(&outputs, ws_source);
                }

                Source::Gitlab {
                    user,
                    me,
                    window,
                    state,
                    instance,
                    include_reviews,
                    throttle_ms,
                    token,
                    cache_dir,
                    no_cache,
                } => {
                    let user = resolve_user_or_me("GitLab", user, me, || {
                        discover_gitlab_user(&instance, token.as_deref())
                    })?;
                    let window = resolve_date_window(window)?;
                    let cache_dir = resolve_cache_dir(&out, cache_dir, no_cache);
                    let ing = make_gitlab_ingestor(
                        &user,
                        window.since,
                        window.until,
                        &state,
                        &instance,
                        include_reviews,
                        throttle_ms,
                        token,
                        cache_dir,
                    )
                    .context("create GitLab ingestor")?;
                    let ingest = ing.ingest().context("ingest events")?;
                    let run_id = ingest.coverage.run_id.to_string();
                    let run_dir = out.join(&run_id);

                    let window_label = window.window_label();

                    // Check if user has curated workstreams and warn
                    if !regen && shiplog_workstreams::WorkstreamManager::has_curated(&run_dir) {
                        eprintln!("Note: Using existing workstreams.yaml (user-curated).");
                        eprintln!("      Use --regen to regenerate suggestions.");
                    }

                    // If --regen, delete existing suggested workstreams so the engine regenerates them
                    if regen {
                        let suggested =
                            shiplog_workstreams::WorkstreamManager::suggested_path(&run_dir);
                        if suggested.exists() {
                            std::fs::remove_file(&suggested)
                                .with_context(|| format!("remove {:?} for --regen", suggested))?;
                        }
                    }

                    let cache_path = DeterministicRedactor::cache_path(&run_dir);
                    let _ = redactor.load_cache(&cache_path);

                    let (outputs, ws_source) = engine
                        .run(ingest, &user, &window_label, &run_dir, zip, &bundle_profile)
                        .context("run engine pipeline")?;

                    redactor
                        .save_cache(&cache_path)
                        .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

                    println!("Collected and wrote:");
                    print_outputs(&outputs, ws_source);
                }

                Source::Jira {
                    user,
                    auth_user,
                    window,
                    status,
                    instance,
                    throttle_ms,
                    token,
                    cache_dir,
                    no_cache,
                } => {
                    let window = resolve_date_window(window)?;
                    let cache_dir = resolve_cache_dir(&out, cache_dir, no_cache);
                    let ing = make_jira_ingestor(
                        &user,
                        auth_user,
                        window.since,
                        window.until,
                        &status,
                        &instance,
                        throttle_ms,
                        token,
                        cache_dir,
                    )
                    .context("create Jira ingestor")?;
                    let ingest = ing.ingest().context("ingest events")?;
                    let run_id = ingest.coverage.run_id.to_string();
                    let run_dir = out.join(&run_id);

                    let window_label = window.window_label();

                    // Check if user has curated workstreams and warn
                    if !regen && shiplog_workstreams::WorkstreamManager::has_curated(&run_dir) {
                        eprintln!("Note: Using existing workstreams.yaml (user-curated).");
                        eprintln!("      Use --regen to regenerate suggestions.");
                    }

                    // If --regen, delete existing suggested workstreams so the engine regenerates them
                    if regen {
                        let suggested =
                            shiplog_workstreams::WorkstreamManager::suggested_path(&run_dir);
                        if suggested.exists() {
                            std::fs::remove_file(&suggested)
                                .with_context(|| format!("remove {:?} for --regen", suggested))?;
                        }
                    }

                    let cache_path = DeterministicRedactor::cache_path(&run_dir);
                    let _ = redactor.load_cache(&cache_path);

                    let (outputs, ws_source) = engine
                        .run(ingest, &user, &window_label, &run_dir, zip, &bundle_profile)
                        .context("run engine pipeline")?;

                    redactor
                        .save_cache(&cache_path)
                        .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

                    println!("Collected and wrote:");
                    print_outputs(&outputs, ws_source);
                }

                Source::Linear {
                    user_id,
                    window,
                    status,
                    project,
                    throttle_ms,
                    api_key,
                    cache_dir,
                    no_cache,
                } => {
                    let window = resolve_date_window(window)?;
                    let cache_dir = resolve_cache_dir(&out, cache_dir, no_cache);
                    let ing = make_linear_ingestor(
                        &user_id,
                        window.since,
                        window.until,
                        &status,
                        project,
                        throttle_ms,
                        api_key,
                        cache_dir,
                    )
                    .context("create Linear ingestor")?;
                    let ingest = ing.ingest().context("ingest events")?;
                    let run_id = ingest.coverage.run_id.to_string();
                    let run_dir = out.join(&run_id);

                    let window_label = window.window_label();

                    // Check if user has curated workstreams and warn
                    if !regen && shiplog_workstreams::WorkstreamManager::has_curated(&run_dir) {
                        eprintln!("Note: Using existing workstreams.yaml (user-curated).");
                        eprintln!("      Use --regen to regenerate suggestions.");
                    }

                    // If --regen, delete existing suggested workstreams so the engine regenerates them
                    if regen {
                        let suggested =
                            shiplog_workstreams::WorkstreamManager::suggested_path(&run_dir);
                        if suggested.exists() {
                            std::fs::remove_file(&suggested)
                                .with_context(|| format!("remove {:?} for --regen", suggested))?;
                        }
                    }

                    let cache_path = DeterministicRedactor::cache_path(&run_dir);
                    let _ = redactor.load_cache(&cache_path);

                    let (outputs, ws_source) = engine
                        .run(
                            ingest,
                            &user_id,
                            &window_label,
                            &run_dir,
                            zip,
                            &bundle_profile,
                        )
                        .context("run engine pipeline")?;

                    redactor
                        .save_cache(&cache_path)
                        .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

                    println!("Collected and wrote:");
                    print_outputs(&outputs, ws_source);
                }

                Source::Json {
                    events,
                    coverage,
                    user,
                    window_label,
                } => {
                    let ing = JsonIngestor {
                        events_path: events,
                        coverage_path: coverage,
                    };
                    let ingest = ing.ingest().context("ingest events")?;
                    let run_id = ingest.coverage.run_id.to_string();
                    let run_dir = out.join(&run_id);

                    // Check if user has curated workstreams and warn
                    if !regen && shiplog_workstreams::WorkstreamManager::has_curated(&run_dir) {
                        eprintln!("Note: Using existing workstreams.yaml (user-curated).");
                        eprintln!("      Use --regen to regenerate suggestions.");
                    }

                    // If --regen, delete existing suggested workstreams so the engine regenerates them
                    if regen {
                        let suggested =
                            shiplog_workstreams::WorkstreamManager::suggested_path(&run_dir);
                        if suggested.exists() {
                            std::fs::remove_file(&suggested)
                                .with_context(|| format!("remove {:?} for --regen", suggested))?;
                        }
                    }

                    let cache_path = DeterministicRedactor::cache_path(&run_dir);
                    let _ = redactor.load_cache(&cache_path);

                    let (outputs, ws_source) = engine
                        .run(ingest, &user, &window_label, &run_dir, zip, &bundle_profile)
                        .context("run engine pipeline")?;

                    redactor
                        .save_cache(&cache_path)
                        .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

                    println!("Collected and wrote:");
                    print_outputs(&outputs, ws_source);
                }

                Source::Manual {
                    events,
                    user,
                    window,
                } => {
                    let window = resolve_date_window(window)?;
                    let ing =
                        ManualIngestor::new(&events, user.clone(), window.since, window.until);
                    let ingest = ing.ingest().context("ingest events")?;
                    let run_id = ingest.coverage.run_id.to_string();
                    let run_dir = out.join(&run_id);
                    let window_label = window.window_label();

                    // Check if user has curated workstreams and warn
                    if !regen && shiplog_workstreams::WorkstreamManager::has_curated(&run_dir) {
                        eprintln!("Note: Using existing workstreams.yaml (user-curated).");
                        eprintln!("      Use --regen to regenerate suggestions.");
                    }

                    // If --regen, delete existing suggested workstreams so the engine regenerates them
                    if regen {
                        let suggested =
                            shiplog_workstreams::WorkstreamManager::suggested_path(&run_dir);
                        if suggested.exists() {
                            std::fs::remove_file(&suggested)
                                .with_context(|| format!("remove {:?} for --regen", suggested))?;
                        }
                    }

                    let cache_path = DeterministicRedactor::cache_path(&run_dir);
                    let _ = redactor.load_cache(&cache_path);

                    let (outputs, ws_source) = engine
                        .run(ingest, &user, &window_label, &run_dir, zip, &bundle_profile)
                        .context("run engine pipeline")?;

                    redactor
                        .save_cache(&cache_path)
                        .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

                    println!("Collected and wrote:");
                    print_outputs(&outputs, ws_source);
                }

                Source::Git {
                    repo,
                    window,
                    author,
                    include_merges,
                } => {
                    let window = resolve_date_window(window)?;
                    let ing = make_git_ingestor(
                        &repo,
                        window.since,
                        window.until,
                        author,
                        include_merges,
                    );
                    let ingest = ing.ingest().context("ingest events")?;
                    let run_id = ingest.coverage.run_id.to_string();
                    let run_dir = out.join(&run_id);
                    let window_label = window.window_label();

                    // Check if user has curated workstreams and warn
                    if !regen && shiplog_workstreams::WorkstreamManager::has_curated(&run_dir) {
                        eprintln!("Note: Using existing workstreams.yaml (user-curated).");
                        eprintln!("      Use --regen to regenerate suggestions.");
                    }

                    // If --regen, delete existing suggested workstreams so the engine regenerates them
                    if regen {
                        let suggested =
                            shiplog_workstreams::WorkstreamManager::suggested_path(&run_dir);
                        if suggested.exists() {
                            std::fs::remove_file(&suggested)
                                .with_context(|| format!("remove {:?} for --regen", suggested))?;
                        }
                    }

                    let cache_path = DeterministicRedactor::cache_path(&run_dir);
                    let _ = redactor.load_cache(&cache_path);

                    let (outputs, ws_source) = engine
                        .run(
                            ingest,
                            "local",
                            &window_label,
                            &run_dir,
                            zip,
                            &bundle_profile,
                        )
                        .context("run engine pipeline")?;

                    redactor
                        .save_cache(&cache_path)
                        .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

                    println!("Collected and wrote:");
                    print_outputs(&outputs, ws_source);
                }
            }
        }

        Command::Render {
            out,
            run,
            latest,
            user,
            window_label,
            redact_key,
            bundle_profile,
            zip,
        } => {
            let key = get_redact_key(redact_key);
            let clusterer: Box<dyn shiplog_ports::WorkstreamClusterer> = Box::new(RepoClusterer);
            let (engine, redactor) = create_engine(&key, clusterer);

            // Determine which run to render
            let run_dir = resolve_render_run_dir(&out, run, latest)?;

            // Read existing events and coverage
            let events_path = run_dir.join("ledger.events.jsonl");
            let coverage_path = run_dir.join("coverage.manifest.json");

            if !events_path.exists() {
                anyhow::bail!(
                    "No ledger.events.jsonl found in {:?}. Run `shiplog collect` first.",
                    run_dir
                );
            }

            let ing = JsonIngestor {
                events_path,
                coverage_path,
            };
            let ingest = ing.ingest().context("ingest events")?;

            let cache_path = DeterministicRedactor::cache_path(&run_dir);
            let _ = redactor.load_cache(&cache_path);

            let outputs = engine
                .refresh(ingest, &user, &window_label, &run_dir, zip, &bundle_profile)
                .context("refresh engine pipeline")?;

            redactor
                .save_cache(&cache_path)
                .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

            println!("Rendered from existing events:");
            print_outputs(&outputs, WorkstreamSource::Curated);
        }

        Command::Refresh {
            source,
            out,
            run_dir: explicit_run_dir,
            zip,
            redact_key,
            bundle_profile,
        } => {
            let key = get_redact_key(redact_key);
            let clusterer: Box<dyn shiplog_ports::WorkstreamClusterer> = Box::new(RepoClusterer);
            let (engine, redactor) = create_engine(&key, clusterer);

            // Resolve run directory: explicit --run-dir, or find most recent
            let run_dir = if let Some(rd) = explicit_run_dir {
                if rd == Path::new("latest") {
                    find_most_recent_run(&out)?
                } else {
                    rd
                }
            } else {
                find_most_recent_run(&out)?
            };

            let cache_path = DeterministicRedactor::cache_path(&run_dir);
            let _ = redactor.load_cache(&cache_path);

            match source {
                Source::Git {
                    repo,
                    window,
                    author,
                    include_merges,
                } => {
                    let window = resolve_date_window(window)?;
                    let ing = make_git_ingestor(
                        &repo,
                        window.since,
                        window.until,
                        author,
                        include_merges,
                    );
                    let ingest = ing.ingest().context("ingest events")?;
                    let window_label = window.window_label();

                    if !shiplog_workstreams::WorkstreamManager::has_curated(&run_dir)
                        && !shiplog_workstreams::WorkstreamManager::suggested_path(&run_dir)
                            .exists()
                    {
                        anyhow::bail!(
                            "No workstreams found in {:?}. Run `shiplog collect` first.",
                            run_dir
                        );
                    }

                    let outputs = engine
                        .refresh(
                            ingest,
                            "local",
                            &window_label,
                            &run_dir,
                            zip,
                            &bundle_profile,
                        )
                        .context("refresh engine pipeline")?;

                    redactor
                        .save_cache(&cache_path)
                        .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

                    println!("Refreshed while preserving workstream curation:");
                    print_outputs_simple(&outputs);
                }
                Source::Github {
                    user,
                    me,
                    window,
                    mode,
                    include_reviews,
                    no_details,
                    throttle_ms,
                    token,
                    api_base,
                    cache_dir,
                    no_cache,
                } => {
                    let user = resolve_user_or_me("GitHub", user, me, || {
                        discover_github_user(&api_base, token.as_deref())
                    })?;
                    let window = resolve_date_window(window)?;
                    let cache_root = run_dir
                        .parent()
                        .map(|p| p.to_path_buf())
                        .unwrap_or_else(|| out.clone());
                    let cache_dir = resolve_cache_dir(&cache_root, cache_dir, no_cache);
                    let ing = make_github_ingestor(
                        &user,
                        window.since,
                        window.until,
                        &mode,
                        include_reviews,
                        no_details,
                        throttle_ms,
                        token,
                        &api_base,
                        cache_dir,
                    )
                    .context("create GitHub ingestor")?;
                    let ingest = ing.ingest().context("ingest events")?;

                    let window_label = window.window_label();

                    if !shiplog_workstreams::WorkstreamManager::has_curated(&run_dir)
                        && !shiplog_workstreams::WorkstreamManager::suggested_path(&run_dir)
                            .exists()
                    {
                        anyhow::bail!(
                            "No workstreams found in {:?}. Run `shiplog collect` first.",
                            run_dir
                        );
                    }

                    let outputs = engine
                        .refresh(ingest, &user, &window_label, &run_dir, zip, &bundle_profile)
                        .context("refresh engine pipeline")?;

                    redactor
                        .save_cache(&cache_path)
                        .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

                    println!("Refreshed while preserving workstream curation:");
                    print_outputs_simple(&outputs);
                }

                Source::Gitlab {
                    user,
                    me,
                    window,
                    state,
                    instance,
                    include_reviews,
                    throttle_ms,
                    token,
                    cache_dir,
                    no_cache,
                } => {
                    let user = resolve_user_or_me("GitLab", user, me, || {
                        discover_gitlab_user(&instance, token.as_deref())
                    })?;
                    let window = resolve_date_window(window)?;
                    let cache_root = run_dir
                        .parent()
                        .map(|p| p.to_path_buf())
                        .unwrap_or_else(|| out.clone());
                    let cache_dir = resolve_cache_dir(&cache_root, cache_dir, no_cache);
                    let ing = make_gitlab_ingestor(
                        &user,
                        window.since,
                        window.until,
                        &state,
                        &instance,
                        include_reviews,
                        throttle_ms,
                        token,
                        cache_dir,
                    )
                    .context("create GitLab ingestor")?;
                    let ingest = ing.ingest().context("ingest events")?;

                    let window_label = window.window_label();

                    if !shiplog_workstreams::WorkstreamManager::has_curated(&run_dir)
                        && !shiplog_workstreams::WorkstreamManager::suggested_path(&run_dir)
                            .exists()
                    {
                        anyhow::bail!(
                            "No workstreams found in {:?}. Run `shiplog collect` first.",
                            run_dir
                        );
                    }

                    let outputs = engine
                        .refresh(ingest, &user, &window_label, &run_dir, zip, &bundle_profile)
                        .context("refresh engine pipeline")?;

                    redactor
                        .save_cache(&cache_path)
                        .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

                    println!("Refreshed while preserving workstream curation:");
                    print_outputs_simple(&outputs);
                }

                Source::Jira {
                    user,
                    auth_user,
                    window,
                    status,
                    instance,
                    throttle_ms,
                    token,
                    cache_dir,
                    no_cache,
                } => {
                    let window = resolve_date_window(window)?;
                    let cache_root = run_dir
                        .parent()
                        .map(|p| p.to_path_buf())
                        .unwrap_or_else(|| out.clone());
                    let cache_dir = resolve_cache_dir(&cache_root, cache_dir, no_cache);
                    let ing = make_jira_ingestor(
                        &user,
                        auth_user,
                        window.since,
                        window.until,
                        &status,
                        &instance,
                        throttle_ms,
                        token,
                        cache_dir,
                    )
                    .context("create Jira ingestor")?;
                    let ingest = ing.ingest().context("ingest events")?;

                    let window_label = window.window_label();

                    if !shiplog_workstreams::WorkstreamManager::has_curated(&run_dir)
                        && !shiplog_workstreams::WorkstreamManager::suggested_path(&run_dir)
                            .exists()
                    {
                        anyhow::bail!(
                            "No workstreams found in {:?}. Run `shiplog collect` first.",
                            run_dir
                        );
                    }

                    let outputs = engine
                        .refresh(ingest, &user, &window_label, &run_dir, zip, &bundle_profile)
                        .context("refresh engine pipeline")?;

                    redactor
                        .save_cache(&cache_path)
                        .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

                    println!("Refreshed while preserving workstream curation:");
                    print_outputs_simple(&outputs);
                }

                Source::Linear {
                    user_id,
                    window,
                    status,
                    project,
                    throttle_ms,
                    api_key,
                    cache_dir,
                    no_cache,
                } => {
                    let window = resolve_date_window(window)?;
                    let cache_root = run_dir
                        .parent()
                        .map(|p| p.to_path_buf())
                        .unwrap_or_else(|| out.clone());
                    let cache_dir = resolve_cache_dir(&cache_root, cache_dir, no_cache);
                    let ing = make_linear_ingestor(
                        &user_id,
                        window.since,
                        window.until,
                        &status,
                        project,
                        throttle_ms,
                        api_key,
                        cache_dir,
                    )
                    .context("create Linear ingestor")?;
                    let ingest = ing.ingest().context("ingest events")?;

                    let window_label = window.window_label();

                    if !shiplog_workstreams::WorkstreamManager::has_curated(&run_dir)
                        && !shiplog_workstreams::WorkstreamManager::suggested_path(&run_dir)
                            .exists()
                    {
                        anyhow::bail!(
                            "No workstreams found in {:?}. Run `shiplog collect` first.",
                            run_dir
                        );
                    }

                    let outputs = engine
                        .refresh(
                            ingest,
                            &user_id,
                            &window_label,
                            &run_dir,
                            zip,
                            &bundle_profile,
                        )
                        .context("refresh engine pipeline")?;

                    redactor
                        .save_cache(&cache_path)
                        .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

                    println!("Refreshed while preserving workstream curation:");
                    print_outputs_simple(&outputs);
                }

                Source::Json {
                    events,
                    coverage,
                    user,
                    window_label,
                } => {
                    if !shiplog_workstreams::WorkstreamManager::has_curated(&run_dir)
                        && !shiplog_workstreams::WorkstreamManager::suggested_path(&run_dir)
                            .exists()
                    {
                        anyhow::bail!(
                            "No workstreams found in {:?}. Run `shiplog collect` first.",
                            run_dir
                        );
                    }

                    let ing = JsonIngestor {
                        events_path: events,
                        coverage_path: coverage,
                    };
                    let ingest = ing.ingest().context("ingest events")?;

                    let outputs = engine
                        .refresh(ingest, &user, &window_label, &run_dir, zip, &bundle_profile)
                        .context("refresh engine pipeline")?;

                    redactor
                        .save_cache(&cache_path)
                        .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

                    println!("Refreshed while preserving workstream curation:");
                    print_outputs_simple(&outputs);
                }

                Source::Manual {
                    events,
                    user,
                    window,
                } => {
                    let window = resolve_date_window(window)?;
                    if !shiplog_workstreams::WorkstreamManager::has_curated(&run_dir)
                        && !shiplog_workstreams::WorkstreamManager::suggested_path(&run_dir)
                            .exists()
                    {
                        anyhow::bail!(
                            "No workstreams found in {:?}. Run `shiplog collect` first.",
                            run_dir
                        );
                    }

                    let ing =
                        ManualIngestor::new(&events, user.clone(), window.since, window.until);
                    let ingest = ing.ingest().context("ingest events")?;
                    let window_label = window.window_label();

                    let outputs = engine
                        .refresh(ingest, &user, &window_label, &run_dir, zip, &bundle_profile)
                        .context("refresh engine pipeline")?;

                    redactor
                        .save_cache(&cache_path)
                        .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

                    println!("Refreshed while preserving workstream curation:");
                    print_outputs_simple(&outputs);
                }
            }
        }
        Command::Import {
            dir,
            out,
            user,
            window_label,
            redact_key,
            bundle_profile,
            zip,
            regen,
            llm_cluster,
            llm_api_endpoint,
            llm_model,
            llm_api_key,
        } => {
            let events_path = dir.join("ledger.events.jsonl");
            let coverage_path = dir.join("coverage.manifest.json");

            if !events_path.exists() {
                anyhow::bail!(
                    "No ledger.events.jsonl found in {:?}. Expected import directory.",
                    dir
                );
            }
            if !coverage_path.exists() {
                anyhow::bail!(
                    "No coverage.manifest.json found in {:?}. Expected import directory.",
                    dir
                );
            }

            let key = get_redact_key(redact_key);
            let clusterer =
                build_clusterer(llm_cluster, &llm_api_endpoint, &llm_model, llm_api_key);
            let (engine, redactor) = create_engine(&key, clusterer);

            let ing = JsonIngestor {
                events_path,
                coverage_path,
            };
            let ingest = ing.ingest().context("ingest events")?;
            let run_id = ingest.coverage.run_id.to_string();
            let run_dir = out.join(&run_id);

            // If --regen, delete stale workstream files so the engine reclusters
            if regen {
                let curated = run_dir.join("workstreams.yaml");
                let suggested = run_dir.join("workstreams.suggested.yaml");
                let _ = std::fs::remove_file(&curated);
                let _ = std::fs::remove_file(&suggested);
            }

            // Load workstreams from import dir (unless --regen)
            let workstreams = if regen {
                None
            } else {
                shiplog_workstreams::WorkstreamManager::try_load(&dir)
                    .context("load workstreams from import directory")?
            };

            let cache_path = DeterministicRedactor::cache_path(&run_dir);
            let _ = redactor.load_cache(&cache_path);

            let (outputs, ws_source) = engine
                .import(
                    ingest,
                    &user,
                    &window_label,
                    &run_dir,
                    zip,
                    workstreams,
                    &bundle_profile,
                )
                .context("import engine pipeline")?;

            redactor
                .save_cache(&cache_path)
                .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

            println!("Imported and wrote:");
            print_outputs(&outputs, ws_source);
        }

        Command::Run {
            source,
            out,
            zip,
            redact_key,
            bundle_profile,
            llm_cluster,
            llm_api_endpoint,
            llm_model,
            llm_api_key,
        } => {
            // Legacy mode: just do collect
            let key = get_redact_key(redact_key);
            let clusterer =
                build_clusterer(llm_cluster, &llm_api_endpoint, &llm_model, llm_api_key);
            let (engine, redactor) = create_engine(&key, clusterer);

            match source {
                Source::Git {
                    repo,
                    window,
                    author,
                    include_merges,
                } => {
                    let window = resolve_date_window(window)?;
                    let ing = make_git_ingestor(
                        &repo,
                        window.since,
                        window.until,
                        author,
                        include_merges,
                    );
                    let ingest = ing.ingest().context("ingest events")?;
                    let run_id = ingest.coverage.run_id.to_string();
                    let run_dir = out.join(&run_id);

                    let cache_path = DeterministicRedactor::cache_path(&run_dir);
                    let _ = redactor.load_cache(&cache_path);

                    let window_label = window.window_label();
                    let (outputs, ws_source) = engine
                        .run(
                            ingest,
                            "local",
                            &window_label,
                            &run_dir,
                            zip,
                            &bundle_profile,
                        )
                        .context("run engine pipeline")?;

                    redactor
                        .save_cache(&cache_path)
                        .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

                    println!("Wrote:");
                    print_outputs(&outputs, ws_source);
                }
                Source::Github {
                    user,
                    me,
                    window,
                    mode,
                    include_reviews,
                    no_details,
                    throttle_ms,
                    token,
                    api_base,
                    cache_dir,
                    no_cache,
                } => {
                    let user = resolve_user_or_me("GitHub", user, me, || {
                        discover_github_user(&api_base, token.as_deref())
                    })?;
                    let window = resolve_date_window(window)?;
                    let cache_dir = resolve_cache_dir(&out, cache_dir, no_cache);
                    let ing = make_github_ingestor(
                        &user,
                        window.since,
                        window.until,
                        &mode,
                        include_reviews,
                        no_details,
                        throttle_ms,
                        token,
                        &api_base,
                        cache_dir,
                    )
                    .context("create GitHub ingestor")?;
                    let ingest = ing.ingest().context("ingest events")?;
                    let run_id = ingest.coverage.run_id.to_string();
                    let run_dir = out.join(&run_id);

                    let cache_path = DeterministicRedactor::cache_path(&run_dir);
                    let _ = redactor.load_cache(&cache_path);

                    let window_label = window.window_label();
                    let (outputs, ws_source) = engine
                        .run(ingest, &user, &window_label, &run_dir, zip, &bundle_profile)
                        .context("run engine pipeline")?;

                    redactor
                        .save_cache(&cache_path)
                        .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

                    println!("Wrote:");
                    print_outputs(&outputs, ws_source);
                }

                Source::Gitlab {
                    user,
                    me,
                    window,
                    state,
                    instance,
                    include_reviews,
                    throttle_ms,
                    token,
                    cache_dir,
                    no_cache,
                } => {
                    let user = resolve_user_or_me("GitLab", user, me, || {
                        discover_gitlab_user(&instance, token.as_deref())
                    })?;
                    let window = resolve_date_window(window)?;
                    let cache_dir = resolve_cache_dir(&out, cache_dir, no_cache);
                    let ing = make_gitlab_ingestor(
                        &user,
                        window.since,
                        window.until,
                        &state,
                        &instance,
                        include_reviews,
                        throttle_ms,
                        token,
                        cache_dir,
                    )
                    .context("create GitLab ingestor")?;
                    let ingest = ing.ingest().context("ingest events")?;
                    let run_id = ingest.coverage.run_id.to_string();
                    let run_dir = out.join(&run_id);

                    let cache_path = DeterministicRedactor::cache_path(&run_dir);
                    let _ = redactor.load_cache(&cache_path);

                    let window_label = window.window_label();
                    let (outputs, ws_source) = engine
                        .run(ingest, &user, &window_label, &run_dir, zip, &bundle_profile)
                        .context("run engine pipeline")?;

                    redactor
                        .save_cache(&cache_path)
                        .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

                    println!("Wrote:");
                    print_outputs(&outputs, ws_source);
                }

                Source::Jira {
                    user,
                    auth_user,
                    window,
                    status,
                    instance,
                    throttle_ms,
                    token,
                    cache_dir,
                    no_cache,
                } => {
                    let window = resolve_date_window(window)?;
                    let cache_dir = resolve_cache_dir(&out, cache_dir, no_cache);
                    let ing = make_jira_ingestor(
                        &user,
                        auth_user,
                        window.since,
                        window.until,
                        &status,
                        &instance,
                        throttle_ms,
                        token,
                        cache_dir,
                    )
                    .context("create Jira ingestor")?;
                    let ingest = ing.ingest().context("ingest events")?;
                    let run_id = ingest.coverage.run_id.to_string();
                    let run_dir = out.join(&run_id);

                    let cache_path = DeterministicRedactor::cache_path(&run_dir);
                    let _ = redactor.load_cache(&cache_path);

                    let window_label = window.window_label();
                    let (outputs, ws_source) = engine
                        .run(ingest, &user, &window_label, &run_dir, zip, &bundle_profile)
                        .context("run engine pipeline")?;

                    redactor
                        .save_cache(&cache_path)
                        .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

                    println!("Wrote:");
                    print_outputs(&outputs, ws_source);
                }

                Source::Linear {
                    user_id,
                    window,
                    status,
                    project,
                    throttle_ms,
                    api_key,
                    cache_dir,
                    no_cache,
                } => {
                    let window = resolve_date_window(window)?;
                    let cache_dir = resolve_cache_dir(&out, cache_dir, no_cache);
                    let ing = make_linear_ingestor(
                        &user_id,
                        window.since,
                        window.until,
                        &status,
                        project,
                        throttle_ms,
                        api_key,
                        cache_dir,
                    )
                    .context("create Linear ingestor")?;
                    let ingest = ing.ingest().context("ingest events")?;
                    let run_id = ingest.coverage.run_id.to_string();
                    let run_dir = out.join(&run_id);

                    let cache_path = DeterministicRedactor::cache_path(&run_dir);
                    let _ = redactor.load_cache(&cache_path);

                    let window_label = window.window_label();
                    let (outputs, ws_source) = engine
                        .run(
                            ingest,
                            &user_id,
                            &window_label,
                            &run_dir,
                            zip,
                            &bundle_profile,
                        )
                        .context("run engine pipeline")?;

                    redactor
                        .save_cache(&cache_path)
                        .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

                    println!("Wrote:");
                    print_outputs(&outputs, ws_source);
                }

                Source::Json {
                    events,
                    coverage,
                    user,
                    window_label,
                } => {
                    let ing = JsonIngestor {
                        events_path: events,
                        coverage_path: coverage,
                    };
                    let ingest = ing.ingest().context("ingest events")?;
                    let run_id = ingest.coverage.run_id.to_string();
                    let run_dir = out.join(&run_id);

                    let cache_path = DeterministicRedactor::cache_path(&run_dir);
                    let _ = redactor.load_cache(&cache_path);

                    let (outputs, ws_source) = engine
                        .run(ingest, &user, &window_label, &run_dir, zip, &bundle_profile)
                        .context("run engine pipeline")?;

                    redactor
                        .save_cache(&cache_path)
                        .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

                    println!("Wrote:");
                    print_outputs(&outputs, ws_source);
                }

                Source::Manual {
                    events,
                    user,
                    window,
                } => {
                    let window = resolve_date_window(window)?;
                    let ing =
                        ManualIngestor::new(&events, user.clone(), window.since, window.until);
                    let ingest = ing.ingest().context("ingest events")?;
                    let run_id = ingest.coverage.run_id.to_string();
                    let run_dir = out.join(&run_id);
                    let window_label = window.window_label();

                    let cache_path = DeterministicRedactor::cache_path(&run_dir);
                    let _ = redactor.load_cache(&cache_path);

                    let (outputs, ws_source) = engine
                        .run(ingest, &user, &window_label, &run_dir, zip, &bundle_profile)
                        .context("run engine pipeline")?;

                    redactor
                        .save_cache(&cache_path)
                        .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

                    println!("Wrote:");
                    print_outputs(&outputs, ws_source);
                }
            }
        }
    }

    Ok(())
}

fn print_outputs(outputs: &shiplog_engine::RunOutputs, ws_source: WorkstreamSource) {
    println!(
        "- {} ({})",
        outputs.packet_md.display(),
        match ws_source {
            WorkstreamSource::Curated => "using your curated workstreams.yaml",
            WorkstreamSource::Suggested =>
                "using suggested workstreams (edit and rename to workstreams.yaml)",
            WorkstreamSource::Generated => "newly generated",
        }
    );
    println!("- {}", outputs.workstreams_yaml.display());
    println!("- {}", outputs.ledger_events_jsonl.display());
    println!("- {}", outputs.coverage_manifest_json.display());
    println!("- {}", outputs.bundle_manifest_json.display());
    if let Some(ref z) = outputs.zip_path {
        println!("- {}", z.display());
    }
}

fn print_outputs_simple(outputs: &shiplog_engine::RunOutputs) {
    println!("- {}", outputs.packet_md.display());
    println!("- {}", outputs.workstreams_yaml.display());
    println!("- {}", outputs.ledger_events_jsonl.display());
    println!("- {}", outputs.coverage_manifest_json.display());
    println!("- {}", outputs.bundle_manifest_json.display());
    if let Some(ref z) = outputs.zip_path {
        println!("- {}", z.display());
    }
}

fn find_most_recent_run(out_dir: &Path) -> Result<PathBuf> {
    if !out_dir.exists() {
        anyhow::bail!("Output directory {:?} does not exist.", out_dir);
    }

    let mut runs: Vec<_> = std::fs::read_dir(out_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter(|e| e.path().join("ledger.events.jsonl").exists())
        .collect();

    // Sort by modified time, most recent first
    runs.sort_by(|a, b| {
        let a_meta = a.metadata().and_then(|m| m.modified()).ok();
        let b_meta = b.metadata().and_then(|m| m.modified()).ok();
        b_meta.cmp(&a_meta)
    });

    runs.into_iter()
        .next()
        .map(|e| e.path())
        .ok_or_else(|| anyhow::anyhow!("No run directories found in {:?}", out_dir))
}

fn resolve_render_run_dir(out_dir: &Path, run: Option<String>, latest: bool) -> Result<PathBuf> {
    if latest && run.is_some() {
        anyhow::bail!("use either --latest or --run, not both")
    }

    match run.as_deref() {
        Some("latest") | None => find_most_recent_run(out_dir),
        Some(run_id) => Ok(out_dir.join(run_id)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date_args() -> DateArgs {
        DateArgs {
            since: None,
            until: None,
            last_6_months: false,
            last_quarter: false,
            year: None,
        }
    }

    #[test]
    fn resolve_cache_dir_uses_default_out_cache() {
        let out_root = Path::new("C:/tmp/shiplog-out");
        let resolved = resolve_cache_dir(out_root, None, false);
        assert_eq!(resolved, Some(out_root.join(".cache")));
    }

    #[test]
    fn resolve_cache_dir_uses_explicit_cache_path() {
        let out_root = Path::new("C:/tmp/shiplog-out");
        let explicit = PathBuf::from("D:/cache-root");
        let resolved = resolve_cache_dir(out_root, Some(explicit.clone()), false);
        assert_eq!(resolved, Some(explicit));
    }

    #[test]
    fn resolve_cache_dir_disables_cache_when_requested() {
        let out_root = Path::new("C:/tmp/shiplog-out");
        let explicit = PathBuf::from("D:/cache-root");
        let resolved = resolve_cache_dir(out_root, Some(explicit), true);
        assert_eq!(resolved, None);
    }

    #[test]
    fn resolve_date_window_uses_explicit_dates() {
        let mut args = date_args();
        args.since = Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());
        args.until = Some(NaiveDate::from_ymd_opt(2025, 2, 1).unwrap());
        args.last_6_months = true;

        let window =
            resolve_date_window_for_today(args, NaiveDate::from_ymd_opt(2026, 5, 7).unwrap())
                .unwrap();

        assert_eq!(window.since, NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());
        assert_eq!(window.until, NaiveDate::from_ymd_opt(2025, 2, 1).unwrap());
        assert_eq!(window.label, WindowLabel::Explicit);
    }

    #[test]
    fn resolve_date_window_defaults_to_last_six_months() {
        let window = resolve_date_window_for_today(
            date_args(),
            NaiveDate::from_ymd_opt(2026, 5, 7).unwrap(),
        )
        .unwrap();

        assert_eq!(window.since, NaiveDate::from_ymd_opt(2025, 11, 7).unwrap());
        assert_eq!(window.until, NaiveDate::from_ymd_opt(2026, 5, 7).unwrap());
        assert_eq!(window.label, WindowLabel::LastSixMonths);
        assert_eq!(
            window.window_label(),
            "last-6-months (2025-11-07..2026-05-07)"
        );
    }

    #[test]
    fn resolve_date_window_uses_last_quarter() {
        let mut args = date_args();
        args.last_quarter = true;

        let window =
            resolve_date_window_for_today(args, NaiveDate::from_ymd_opt(2026, 5, 7).unwrap())
                .unwrap();

        assert_eq!(window.since, NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        assert_eq!(window.until, NaiveDate::from_ymd_opt(2026, 4, 1).unwrap());
        assert_eq!(window.label, WindowLabel::LastQuarter);
    }

    #[test]
    fn resolve_date_window_uses_year() {
        let mut args = date_args();
        args.year = Some(2025);

        let window =
            resolve_date_window_for_today(args, NaiveDate::from_ymd_opt(2026, 5, 7).unwrap())
                .unwrap();

        assert_eq!(window.since, NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());
        assert_eq!(window.until, NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        assert_eq!(window.label, WindowLabel::Year(2025));
    }

    #[test]
    fn resolve_date_window_rejects_partial_explicit_dates() {
        let mut args = date_args();
        args.since = Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());

        let err = resolve_date_window_for_today(args, NaiveDate::from_ymd_opt(2026, 5, 7).unwrap())
            .unwrap_err();

        assert!(err.to_string().contains("provide both --since"));
    }

    #[test]
    fn resolve_date_window_rejects_multiple_presets() {
        let mut args = date_args();
        args.last_6_months = true;
        args.last_quarter = true;

        let err = resolve_date_window_for_today(args, NaiveDate::from_ymd_opt(2026, 5, 7).unwrap())
            .unwrap_err();

        assert!(err.to_string().contains("choose only one date preset"));
    }

    #[test]
    fn resolve_render_run_dir_rejects_latest_and_run() {
        let err = resolve_render_run_dir(Path::new("out"), Some("run_fixture".to_string()), true)
            .unwrap_err();
        assert!(err.to_string().contains("either --latest or --run"));
    }

    #[test]
    fn resolve_render_run_dir_treats_run_latest_as_most_recent() {
        let tmp = tempfile::tempdir().unwrap();
        let run_dir = tmp.path().join("run_fixture");
        std::fs::create_dir(&run_dir).unwrap();
        std::fs::write(run_dir.join("ledger.events.jsonl"), "").unwrap();

        let resolved =
            resolve_render_run_dir(tmp.path(), Some("latest".to_string()), false).unwrap();

        assert_eq!(resolved, run_dir);
    }

    #[test]
    fn make_gitlab_ingestor_configures_cli_options() {
        let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let until = NaiveDate::from_ymd_opt(2025, 2, 1).unwrap();
        let cache_dir = tempfile::tempdir().unwrap();

        let ing = make_gitlab_ingestor(
            "alice",
            since,
            until,
            "closed",
            "https://gitlab.example.com",
            true,
            25,
            Some("glpat-token".to_string()),
            Some(cache_dir.path().to_path_buf()),
        )
        .unwrap();

        assert_eq!(ing.user, "alice");
        assert_eq!(ing.since, since);
        assert_eq!(ing.until, until);
        assert_eq!(ing.state, MrState::Closed);
        assert_eq!(ing.instance, "gitlab.example.com");
        assert!(ing.include_reviews);
        assert_eq!(ing.throttle_ms, 25);
        assert_eq!(ing.token.as_deref(), Some("glpat-token"));
        assert!(ing.cache.is_some());
        assert!(cache_dir.path().join("gitlab-api-cache.db").exists());
    }

    #[test]
    fn make_gitlab_ingestor_rejects_invalid_state() {
        let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let until = NaiveDate::from_ymd_opt(2025, 2, 1).unwrap();

        let err = make_gitlab_ingestor(
            "alice",
            since,
            until,
            "invalid",
            "gitlab.com",
            false,
            0,
            Some("glpat-token".to_string()),
            None,
        )
        .unwrap_err();

        assert!(
            err.to_string().contains("parse GitLab MR state"),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn make_jira_ingestor_configures_cli_options() {
        let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let until = NaiveDate::from_ymd_opt(2025, 2, 1).unwrap();
        let cache_dir = tempfile::tempdir().unwrap();

        let ing = make_jira_ingestor(
            "712020:account-id",
            Some("alice@example.com".to_string()),
            since,
            until,
            "done",
            "https://company.atlassian.net",
            50,
            Some("jira-token".to_string()),
            Some(cache_dir.path().to_path_buf()),
        )
        .unwrap();

        assert_eq!(ing.user, "712020:account-id");
        assert_eq!(ing.auth_user.as_deref(), Some("alice@example.com"));
        assert_eq!(ing.since, since);
        assert_eq!(ing.until, until);
        assert_eq!(ing.status, IssueStatus::Done);
        assert_eq!(ing.instance, "company.atlassian.net");
        assert_eq!(ing.throttle_ms, 50);
        assert_eq!(ing.token.as_deref(), Some("jira-token"));
        assert!(ing.cache.is_some());
        assert!(cache_dir.path().join("jira-api-cache.db").exists());
    }

    #[test]
    fn make_jira_ingestor_rejects_invalid_status() {
        let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let until = NaiveDate::from_ymd_opt(2025, 2, 1).unwrap();

        let err = make_jira_ingestor(
            "alice@example.com",
            None,
            since,
            until,
            "invalid",
            "company.atlassian.net",
            0,
            Some("jira-token".to_string()),
            None,
        )
        .unwrap_err();

        assert!(
            err.to_string().contains("parse Jira issue status"),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn make_linear_ingestor_configures_cli_options() {
        let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let until = NaiveDate::from_ymd_opt(2025, 2, 1).unwrap();
        let cache_dir = tempfile::tempdir().unwrap();

        let ing = make_linear_ingestor(
            "linear-user-id",
            since,
            until,
            "in_progress",
            Some("OPS".to_string()),
            75,
            Some("linear-key".to_string()),
            Some(cache_dir.path().to_path_buf()),
        )
        .unwrap();

        assert_eq!(ing.user, "linear-user-id");
        assert_eq!(ing.since, since);
        assert_eq!(ing.until, until);
        assert_eq!(ing.status, LinearIssueStatus::InProgress);
        assert_eq!(ing.project.as_deref(), Some("OPS"));
        assert_eq!(ing.throttle_ms, 75);
        assert_eq!(ing.api_key.as_deref(), Some("linear-key"));
        assert!(ing.cache.is_some());
        assert!(cache_dir.path().join("linear-api-cache.db").exists());
    }

    #[test]
    fn make_linear_ingestor_rejects_invalid_status() {
        let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let until = NaiveDate::from_ymd_opt(2025, 2, 1).unwrap();

        let err = make_linear_ingestor(
            "linear-user-id",
            since,
            until,
            "invalid",
            None,
            0,
            Some("linear-key".to_string()),
            None,
        )
        .unwrap_err();

        assert!(
            err.to_string().contains("parse Linear issue status"),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn make_git_ingestor_configures_cli_options() {
        let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let until = NaiveDate::from_ymd_opt(2025, 2, 1).unwrap();
        let repo = Path::new(".");

        let ing = make_git_ingestor(
            repo,
            since,
            until,
            Some("dev@example.com".to_string()),
            true,
        );

        assert_eq!(ing.repo_path, repo);
        assert_eq!(ing.since, since);
        assert_eq!(ing.until, until);
        assert_eq!(ing.author.as_deref(), Some("dev@example.com"));
        assert!(ing.include_merges);
    }
}
