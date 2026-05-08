//! `shiplog` CLI entrypoint.
//!
//! Exposes `init`, `doctor`, `intake`, `config`, `collect`, `render`,
//! `refresh`, `workstreams`, `runs`, `review`, `journal`, `open`, `merge`,
//! `import`, and `run` commands over the workspace engine and adapter crates.

use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, Duration, Months, NaiveDate, Utc};
use clap::{Args, Parser, Subcommand, ValueEnum};
use regex::{Regex, RegexBuilder};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use shiplog_cache::ApiCache;
use shiplog_engine::{ConflictResolution, Engine, WorkstreamSource};
use shiplog_ids::{EventId, WorkstreamId};
use shiplog_ingest_git::LocalGitIngestor;
use shiplog_ingest_github::GithubIngestor;
use shiplog_ingest_gitlab::{GitlabIngestor, MrState};
use shiplog_ingest_jira::{IssueStatus, JiraIngestor};
use shiplog_ingest_json::JsonIngestor;
use shiplog_ingest_linear::{IssueStatus as LinearIssueStatus, LinearIngestor};
use shiplog_ingest_manual::{
    ManualIngestor, create_empty_file, read_manual_events, write_manual_events,
};
use shiplog_ports::{IngestOutput, Ingestor, Renderer};
use shiplog_redact::DeterministicRedactor;
use shiplog_render_md::{
    AppendixMode, MarkdownRenderOptions, MarkdownRenderer, SectionOrder, format_receipt_markdown,
};
use shiplog_schema::{
    bundle::BundleProfile,
    coverage::{CoverageManifest, TimeWindow},
    event::{EventEnvelope, EventPayload},
    event::{Link, ManualDate, ManualEventEntry, ManualEventType},
    workstream::{Workstream, WorkstreamStats, WorkstreamsFile},
};
use shiplog_workstreams::{RepoClusterer, WORKSTREAM_RECEIPT_RENDER_LIMIT};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
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

    /// Check local config, source setup, tokens, and output safety.
    Doctor {
        /// Path to shiplog.toml.
        #[arg(long, default_value = CONFIG_FILENAME)]
        config: PathBuf,
        /// Limit checks to one or more sources.
        #[arg(long = "source", value_enum)]
        sources: Vec<InitSource>,
    },

    /// Run a guided best-effort review intake and print next steps.
    Intake(IntakeArgs),

    /// Validate and explain shiplog.toml without collecting data.
    Config {
        #[command(subcommand)]
        cmd: ConfigCommand,
    },

    /// Inspect and clean source API caches.
    Cache {
        #[command(subcommand)]
        cmd: CacheCommand,
    },

    /// Discover provider identities for source configuration.
    Identify {
        #[command(subcommand)]
        cmd: IdentifyCommand,
    },

    /// Add factual manual evidence without hand-editing YAML.
    Journal {
        #[command(subcommand)]
        cmd: JournalCommand,
    },

    /// Collect events from a source and generate workstream suggestions.
    ///
    /// This creates `workstreams.suggested.yaml` which you can rename to
    /// `workstreams.yaml` and edit to curate your narrative.
    Collect {
        #[command(subcommand)]
        source: CollectSource,
        /// Output directory (a run folder will be created inside).
        #[arg(long)]
        out: Option<PathBuf>,
        /// Also write a zip next to the run folder.
        #[arg(long)]
        zip: bool,
        /// Redaction key. Required for manager/public profiles.
        /// If omitted, SHIPLOG_REDACT_KEY is used.
        #[arg(long)]
        redact_key: Option<String>,
        /// Bundle profile: internal (full), manager, or public.
        #[arg(long)]
        bundle_profile: Option<BundleProfile>,
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
        /// Redaction key. Required for manager/public profiles.
        /// If omitted, SHIPLOG_REDACT_KEY is used.
        #[arg(long)]
        redact_key: Option<String>,
        /// Bundle profile: internal (full), manager, or public.
        #[arg(long, default_value = "internal")]
        bundle_profile: BundleProfile,
        /// Packet output mode.
        #[arg(long, value_enum, default_value = "packet")]
        mode: RenderPacketMode,
        /// Maximum curated receipts to show per workstream in the main receipts section.
        ///
        /// Defaults depend on the selected bundle profile.
        #[arg(long)]
        receipt_limit: Option<usize>,
        /// Appendix density for receipt detail.
        ///
        /// Defaults depend on output mode and selected bundle profile.
        #[arg(long, value_enum)]
        appendix: Option<RenderAppendixMode>,
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
        /// Redaction key. Required for manager/public profiles.
        /// If omitted, SHIPLOG_REDACT_KEY is used.
        #[arg(long)]
        redact_key: Option<String>,
        /// Bundle profile: internal (full), manager, or public.
        #[arg(long, default_value = "internal")]
        bundle_profile: BundleProfile,
    },

    /// Inspect and validate workstream curation for an existing run.
    Workstreams {
        #[command(subcommand)]
        cmd: WorkstreamsCommand,
    },

    /// List and inspect generated run directories.
    Runs {
        #[command(subcommand)]
        cmd: RunsCommand,
    },

    /// Inspect a run and suggest review-prep next steps.
    Review {
        #[command(subcommand)]
        cmd: Option<ReviewCommand>,
        #[command(flatten)]
        options: ReviewOptions,
    },

    /// Open generated artifacts for a run, or print their paths when unavailable.
    Open {
        #[command(subcommand)]
        cmd: OpenCommand,
    },

    /// Merge existing run directories into one packet.
    Merge {
        /// Input run directory containing ledger.events.jsonl and coverage.manifest.json.
        #[arg(long = "input", required = true)]
        inputs: Vec<PathBuf>,
        /// Output directory (a merged run folder will be created inside).
        #[arg(long, default_value = "./out")]
        out: PathBuf,
        /// Duplicate event conflict policy.
        #[arg(long, value_enum, default_value = "prefer-most-recent")]
        conflict: MergeConflict,
        /// Optional user label for rendering. Defaults to the first input coverage user.
        #[arg(long)]
        user: Option<String>,
        /// Optional window label for rendering. Defaults to the merged coverage date window.
        #[arg(long)]
        window_label: Option<String>,
        /// Also write a zip next to the merged run folder.
        #[arg(long)]
        zip: bool,
        /// Redaction key. Required for manager/public profiles.
        /// If omitted, SHIPLOG_REDACT_KEY is used.
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
        /// Redaction key. Required for manager/public profiles.
        /// If omitted, SHIPLOG_REDACT_KEY is used.
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
        /// Redaction key. Required for manager/public profiles.
        /// If omitted, SHIPLOG_REDACT_KEY is used.
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

#[derive(Args, Debug)]
struct ReviewOptions {
    /// Output directory containing run folders.
    #[arg(long, default_value = "./out")]
    out: PathBuf,
    /// Run ID to review (uses most recent if not specified).
    #[arg(long)]
    run: Option<String>,
    /// Review the most recent run explicitly.
    #[arg(long)]
    latest: bool,
    /// Exit with an error when review finds evidence debt.
    #[arg(long)]
    strict: bool,
}

#[derive(Subcommand, Debug)]
enum ReviewCommand {
    /// Inspect the latest weekly evidence and suggest next steps.
    Weekly {
        /// Output directory containing run folders.
        #[arg(long, default_value = "./out")]
        out: PathBuf,
        /// Run ID to review (uses most recent if not specified).
        #[arg(long)]
        run: Option<String>,
        /// Review the most recent run explicitly.
        #[arg(long)]
        latest: bool,
        /// Exit with an error when review finds evidence debt.
        #[arg(long)]
        strict: bool,
    },
}

#[derive(Subcommand, Debug)]
enum ConfigCommand {
    /// Validate shiplog.toml structure and enabled source settings.
    Validate {
        /// Path to shiplog.toml.
        #[arg(long, default_value = CONFIG_FILENAME)]
        config: PathBuf,
    },

    /// Print resolved defaults and enabled source settings.
    Explain {
        /// Path to shiplog.toml.
        #[arg(long, default_value = CONFIG_FILENAME)]
        config: PathBuf,
    },

    /// Add missing version metadata to shiplog.toml.
    Migrate {
        /// Path to shiplog.toml.
        #[arg(long, default_value = CONFIG_FILENAME)]
        config: PathBuf,
        /// Print the migration action without writing.
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand, Debug)]
enum CacheCommand {
    /// Show cache entry counts and size.
    Stats(CacheArgs),

    /// Show cache entry counts plus timestamp bounds.
    Inspect(CacheArgs),

    /// Remove expired, old, or all cache entries without deleting outputs.
    Clean(CacheCleanArgs),
}

#[derive(Subcommand, Debug)]
enum IdentifyCommand {
    /// Show the authenticated Jira account ID for use with `--user`.
    Jira {
        /// Jira instance hostname or URL.
        #[arg(long)]
        instance: String,
        /// Jira Basic Auth username/email. Defaults to JIRA_AUTH_USER.
        #[arg(long)]
        auth_user: Option<String>,
        /// Jira API token. Defaults to JIRA_TOKEN.
        #[arg(long)]
        token: Option<String>,
    },

    /// Show the authenticated Linear user ID for use with `--user-id`.
    Linear {
        /// Linear API key. Defaults to LINEAR_API_KEY.
        #[arg(long)]
        api_key: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum JournalCommand {
    /// Append one manual evidence entry to manual_events.yaml.
    Add(JournalAddArgs),
}

#[derive(Args, Debug)]
struct JournalAddArgs {
    /// Manual events YAML file to create or append to.
    #[arg(long, default_value = MANUAL_EVENTS_FILENAME)]
    events: PathBuf,
    /// Stable event ID. Defaults to `manual-<date>-<slugified-title>`.
    #[arg(long)]
    id: Option<String>,
    /// Manual event type.
    #[arg(long = "type", value_enum, default_value = "note")]
    event_type: JournalEventType,
    /// Single event date, in YYYY-MM-DD format.
    #[arg(long)]
    date: Option<NaiveDate>,
    /// Inclusive start date for a multi-day event.
    #[arg(long)]
    start: Option<NaiveDate>,
    /// Inclusive end date for a multi-day event.
    #[arg(long)]
    end: Option<NaiveDate>,
    /// Factual title for the work.
    #[arg(long)]
    title: String,
    /// Optional context. Keep this factual, not performance-review prose.
    #[arg(long)]
    description: Option<String>,
    /// Workstream to associate with this evidence.
    #[arg(long)]
    workstream: Option<String>,
    /// Tag to attach. Repeat for multiple tags.
    #[arg(long = "tag")]
    tags: Vec<String>,
    /// Receipt link as LABEL=URL. Repeat for multiple receipts.
    #[arg(long = "receipt", value_name = "LABEL=URL")]
    receipts: Vec<String>,
    /// Optional outcome or impact note.
    #[arg(long)]
    impact: Option<String>,
    /// Print the entry that would be added without writing.
    #[arg(long)]
    dry_run: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum JournalEventType {
    Note,
    Incident,
    Design,
    Mentoring,
    Launch,
    Migration,
    Review,
    Other,
}

impl From<JournalEventType> for ManualEventType {
    fn from(value: JournalEventType) -> Self {
        match value {
            JournalEventType::Note => Self::Note,
            JournalEventType::Incident => Self::Incident,
            JournalEventType::Design => Self::Design,
            JournalEventType::Mentoring => Self::Mentoring,
            JournalEventType::Launch => Self::Launch,
            JournalEventType::Migration => Self::Migration,
            JournalEventType::Review => Self::Review,
            JournalEventType::Other => Self::Other,
        }
    }
}

#[derive(Args, Debug)]
struct CacheArgs {
    /// Output directory whose `.cache` directory should be inspected.
    #[arg(long, default_value = "./out")]
    out: PathBuf,
    /// Cache directory to inspect instead of `<out>/.cache`.
    #[arg(long)]
    cache_dir: Option<PathBuf>,
    /// Limit to one or more source caches.
    #[arg(long = "source", value_enum)]
    sources: Vec<CacheSource>,
}

#[derive(Args, Debug)]
struct CacheCleanArgs {
    /// Output directory whose `.cache` directory should be cleaned.
    #[arg(long, default_value = "./out")]
    out: PathBuf,
    /// Cache directory to clean instead of `<out>/.cache`.
    #[arg(long)]
    cache_dir: Option<PathBuf>,
    /// Limit to one or more source caches.
    #[arg(long = "source", value_enum)]
    sources: Vec<CacheSource>,
    /// Remove entries cached before this age, such as 30d, 12h, or 90m.
    #[arg(long)]
    older_than: Option<String>,
    /// Remove every entry in the selected caches. Requires --yes unless --dry-run is set.
    #[arg(long)]
    all: bool,
    /// Print what would be removed without modifying cache databases.
    #[arg(long)]
    dry_run: bool,
    /// Confirm destructive --all cleanup.
    #[arg(long)]
    yes: bool,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum CacheSource {
    Github,
    Gitlab,
    Jira,
    Linear,
}

impl CacheSource {
    fn all() -> [Self; 4] {
        [Self::Github, Self::Gitlab, Self::Jira, Self::Linear]
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Github => "github",
            Self::Gitlab => "gitlab",
            Self::Jira => "jira",
            Self::Linear => "linear",
        }
    }

    fn db_filename(self) -> &'static str {
        match self {
            Self::Github => "github-api-cache.db",
            Self::Gitlab => "gitlab-api-cache.db",
            Self::Jira => "jira-api-cache.db",
            Self::Linear => "linear-api-cache.db",
        }
    }
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

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
enum MergeConflict {
    #[value(name = "prefer-first")]
    First,
    #[value(name = "prefer-most-recent")]
    MostRecent,
    #[value(name = "prefer-most-complete")]
    MostComplete,
}

impl From<MergeConflict> for ConflictResolution {
    fn from(value: MergeConflict) -> Self {
        match value {
            MergeConflict::First => Self::PreferFirst,
            MergeConflict::MostRecent => Self::PreferMostRecent,
            MergeConflict::MostComplete => Self::PreferMostComplete,
        }
    }
}

impl MergeConflict {
    fn as_str(self) -> &'static str {
        match self {
            Self::First => "prefer-first",
            Self::MostRecent => "prefer-most-recent",
            Self::MostComplete => "prefer-most-complete",
        }
    }
}

#[derive(Subcommand, Debug)]
enum RunsCommand {
    /// List discovered runs under an output directory.
    List {
        /// Output directory containing shiplog runs.
        #[arg(long, default_value = "./out")]
        out: PathBuf,
    },

    /// Show details for one run.
    Show {
        /// Output directory containing shiplog runs.
        #[arg(long, default_value = "./out")]
        out: PathBuf,
        /// Run ID to inspect (uses most recent if not specified).
        #[arg(long)]
        run: Option<String>,
        /// Inspect the most recent run explicitly.
        #[arg(long)]
        latest: bool,
    },

    /// Compare two existing runs.
    Compare {
        /// Output directory containing shiplog runs.
        #[arg(long, default_value = "./out")]
        out: PathBuf,
        /// Earlier run ID to compare from. Use "latest" for the most recent run.
        #[arg(long)]
        from: String,
        /// Later run ID to compare to. Use "latest" for the most recent run.
        #[arg(long)]
        to: String,
    },
}

#[derive(Subcommand, Debug)]
enum OpenCommand {
    /// Open the rendered packet for a run.
    Packet {
        /// Output directory containing shiplog runs.
        #[arg(long, default_value = "./out")]
        out: PathBuf,
        /// Run ID to open (uses most recent if not specified).
        #[arg(long)]
        run: Option<String>,
        /// Open the most recent run explicitly.
        #[arg(long)]
        latest: bool,
        /// Print the path without launching a platform opener.
        #[arg(long)]
        print_path: bool,
    },

    /// Open the effective workstreams file for a run.
    Workstreams {
        /// Output directory containing shiplog runs.
        #[arg(long, default_value = "./out")]
        out: PathBuf,
        /// Run ID to open (uses most recent if not specified).
        #[arg(long)]
        run: Option<String>,
        /// Open the most recent run explicitly.
        #[arg(long)]
        latest: bool,
        /// Print the path without launching a platform opener.
        #[arg(long)]
        print_path: bool,
    },

    /// Open the run output directory.
    Out {
        /// Output directory containing shiplog runs.
        #[arg(long, default_value = "./out")]
        out: PathBuf,
        /// Run ID to open (uses most recent if not specified).
        #[arg(long)]
        run: Option<String>,
        /// Open the most recent run explicitly.
        #[arg(long)]
        latest: bool,
        /// Print the path without launching a platform opener.
        #[arg(long)]
        print_path: bool,
    },
}

#[derive(Subcommand, Debug)]
enum WorkstreamsCommand {
    /// List workstreams and their event/receipt counts.
    List {
        /// Output directory containing shiplog runs.
        #[arg(long, default_value = "./out")]
        out: PathBuf,
        /// Run ID to inspect (uses most recent if not specified).
        #[arg(long)]
        run: Option<String>,
        /// Inspect the most recent run explicitly.
        #[arg(long)]
        latest: bool,
    },

    /// Validate the effective workstreams file against the run ledger.
    Validate {
        /// Output directory containing shiplog runs.
        #[arg(long, default_value = "./out")]
        out: PathBuf,
        /// Run ID to validate (uses most recent if not specified).
        #[arg(long)]
        run: Option<String>,
        /// Validate the most recent run explicitly.
        #[arg(long)]
        latest: bool,
    },

    /// Rename a workstream in the curated workstreams file.
    Rename {
        /// Output directory containing shiplog runs.
        #[arg(long, default_value = "./out")]
        out: PathBuf,
        /// Run ID to edit (uses most recent if not specified).
        #[arg(long)]
        run: Option<String>,
        /// Edit the most recent run explicitly.
        #[arg(long)]
        latest: bool,
        /// Existing workstream title or ID.
        #[arg(long)]
        from: String,
        /// New workstream title.
        #[arg(long)]
        to: String,
    },

    /// Move or assign one event to a workstream.
    Move {
        /// Output directory containing shiplog runs.
        #[arg(long, default_value = "./out")]
        out: PathBuf,
        /// Run ID to edit (uses most recent if not specified).
        #[arg(long)]
        run: Option<String>,
        /// Edit the most recent run explicitly.
        #[arg(long)]
        latest: bool,
        /// Event ID from ledger.events.jsonl.
        #[arg(long)]
        event: String,
        /// Target workstream title or ID.
        #[arg(long)]
        to: String,
    },

    /// List curated receipt anchors for a workstream.
    Receipts {
        /// Output directory containing shiplog runs.
        #[arg(long, default_value = "./out")]
        out: PathBuf,
        /// Run ID to inspect (uses most recent if not specified).
        #[arg(long)]
        run: Option<String>,
        /// Inspect the most recent run explicitly.
        #[arg(long)]
        latest: bool,
        /// Workstream title or ID.
        #[arg(long)]
        workstream: String,
    },

    /// Add or remove curated receipt anchors.
    Receipt {
        #[command(subcommand)]
        cmd: WorkstreamReceiptCommand,
    },

    /// Create an empty curated workstream.
    Create {
        /// Output directory containing shiplog runs.
        #[arg(long, default_value = "./out")]
        out: PathBuf,
        /// Run ID to edit (uses most recent if not specified).
        #[arg(long)]
        run: Option<String>,
        /// Edit the most recent run explicitly.
        #[arg(long)]
        latest: bool,
        /// New workstream title.
        #[arg(long)]
        title: String,
    },

    /// Delete a workstream, optionally moving its events to another workstream.
    Delete {
        /// Output directory containing shiplog runs.
        #[arg(long, default_value = "./out")]
        out: PathBuf,
        /// Run ID to edit (uses most recent if not specified).
        #[arg(long)]
        run: Option<String>,
        /// Edit the most recent run explicitly.
        #[arg(long)]
        latest: bool,
        /// Workstream title or ID to delete.
        #[arg(long)]
        workstream: String,
        /// Move events and receipts to this existing workstream before deleting.
        #[arg(long)]
        move_to: Option<String>,
        /// Delete a non-empty workstream without preserving its event assignments.
        #[arg(long)]
        force: bool,
    },

    /// Split matching events out of one workstream into another.
    Split {
        /// Output directory containing shiplog runs.
        #[arg(long, default_value = "./out")]
        out: PathBuf,
        /// Run ID to edit (uses most recent if not specified).
        #[arg(long)]
        run: Option<String>,
        /// Edit the most recent run explicitly.
        #[arg(long)]
        latest: bool,
        /// Source workstream title or ID.
        #[arg(long)]
        from: String,
        /// Target workstream title or ID.
        #[arg(long)]
        to: String,
        /// Regex matched against event title, repo, tags, and source URLs.
        #[arg(long)]
        matching: String,
        /// Create the target workstream if it does not exist.
        #[arg(long)]
        create: bool,
    },
}

#[derive(Subcommand, Debug)]
enum WorkstreamReceiptCommand {
    /// Add an assigned event as a receipt anchor for one workstream.
    Add {
        /// Output directory containing shiplog runs.
        #[arg(long, default_value = "./out")]
        out: PathBuf,
        /// Run ID to edit (uses most recent if not specified).
        #[arg(long)]
        run: Option<String>,
        /// Edit the most recent run explicitly.
        #[arg(long)]
        latest: bool,
        /// Workstream title or ID.
        #[arg(long)]
        workstream: String,
        /// Event ID from ledger.events.jsonl.
        #[arg(long)]
        event: String,
    },

    /// Remove a receipt anchor from one workstream.
    Remove {
        /// Output directory containing shiplog runs.
        #[arg(long, default_value = "./out")]
        out: PathBuf,
        /// Run ID to edit (uses most recent if not specified).
        #[arg(long)]
        run: Option<String>,
        /// Edit the most recent run explicitly.
        #[arg(long)]
        latest: bool,
        /// Workstream title or ID.
        #[arg(long)]
        workstream: String,
        /// Event ID from ledger.events.jsonl.
        #[arg(long)]
        event: String,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum RenderPacketMode {
    /// Review-ready packet with coverage, workstreams, receipts, and appendix.
    Packet,
    /// Writing scaffold with coverage, workstream prompts, and evidence anchors.
    Scaffold,
    /// Dense receipts and appendix view.
    Receipts,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum RenderAppendixMode {
    /// Render every assigned event in the appendix.
    Full,
    /// Render per-workstream receipt counts instead of every event.
    Summary,
    /// Omit the appendix.
    None,
}

impl From<RenderAppendixMode> for AppendixMode {
    fn from(value: RenderAppendixMode) -> Self {
        match value {
            RenderAppendixMode::Full => Self::Full,
            RenderAppendixMode::Summary => Self::Summary,
            RenderAppendixMode::None => Self::None,
        }
    }
}

#[derive(Subcommand, Debug, Clone)]
enum CollectSource {
    /// Collect all enabled sources from shiplog.toml and render one merged packet.
    Multi {
        /// Path to shiplog.toml.
        #[arg(long, default_value = CONFIG_FILENAME)]
        config: PathBuf,
        #[command(flatten)]
        window: DateArgs,
        /// Duplicate event conflict policy.
        #[arg(long, value_enum, default_value = "prefer-most-recent")]
        conflict: MergeConflict,
    },

    #[command(flatten)]
    Source(Source),
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

#[derive(Args, Debug, Clone, Default)]
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

#[derive(Args, Debug, Clone)]
struct IntakeArgs {
    /// Path to shiplog.toml. Created with rescue-mode defaults if missing.
    #[arg(long, default_value = CONFIG_FILENAME)]
    config: PathBuf,
    /// Output directory (a run folder will be created inside).
    #[arg(long)]
    out: Option<PathBuf>,
    /// Limit intake to one or more sources.
    #[arg(long = "source", value_enum)]
    sources: Vec<InitSource>,
    /// Bundle profile to render. Internal is default unless config says otherwise.
    #[arg(long)]
    profile: Option<BundleProfile>,
    /// Redaction key. Required for manager/public profiles.
    /// If omitted, SHIPLOG_REDACT_KEY or the configured redaction env var is used.
    #[arg(long)]
    redact_key: Option<String>,
    /// Do not launch the packet after intake; print paths only.
    #[arg(long)]
    no_open: bool,
    /// Explain why intake used or skipped each source.
    #[arg(long)]
    explain: bool,
    /// Duplicate event conflict policy.
    #[arg(long, value_enum, default_value = "prefer-most-recent")]
    conflict: MergeConflict,
    #[command(flatten)]
    window: DateArgs,
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
const CURRENT_CONFIG_VERSION: i64 = 1;

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
struct ShiplogConfig {
    shiplog: ConfigMetadata,
    defaults: ConfigDefaults,
    user: ConfigUser,
    sources: ConfigSources,
    redaction: ConfigRedaction,
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
struct ConfigMetadata {
    config_version: Option<i64>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
struct ConfigDefaults {
    out: Option<PathBuf>,
    window: Option<String>,
    profile: Option<String>,
    include_reviews: Option<bool>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
struct ConfigUser {
    label: Option<String>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
struct ConfigSources {
    github: Option<ConfigGithubSource>,
    gitlab: Option<ConfigGitlabSource>,
    jira: Option<ConfigJiraSource>,
    linear: Option<ConfigLinearSource>,
    git: Option<ConfigGitSource>,
    json: Option<ConfigJsonSource>,
    manual: Option<ConfigManualSource>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
struct ConfigGithubSource {
    enabled: bool,
    user: Option<String>,
    me: bool,
    mode: Option<String>,
    include_reviews: Option<bool>,
    no_details: bool,
    throttle_ms: u64,
    api_base: Option<String>,
    cache_dir: Option<PathBuf>,
    no_cache: bool,
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
struct ConfigGitlabSource {
    enabled: bool,
    user: Option<String>,
    me: bool,
    instance: Option<String>,
    state: Option<String>,
    include_reviews: Option<bool>,
    throttle_ms: u64,
    cache_dir: Option<PathBuf>,
    no_cache: bool,
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
struct ConfigJiraSource {
    enabled: bool,
    user: Option<String>,
    auth_user: Option<String>,
    auth_user_env: Option<String>,
    instance: Option<String>,
    status: Option<String>,
    throttle_ms: u64,
    cache_dir: Option<PathBuf>,
    no_cache: bool,
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
struct ConfigLinearSource {
    enabled: bool,
    user_id: Option<String>,
    status: Option<String>,
    project: Option<String>,
    throttle_ms: u64,
    cache_dir: Option<PathBuf>,
    no_cache: bool,
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
struct ConfigGitSource {
    enabled: bool,
    repo: Option<PathBuf>,
    author: Option<String>,
    include_merges: bool,
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
struct ConfigJsonSource {
    enabled: bool,
    events: Option<PathBuf>,
    coverage: Option<PathBuf>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
struct ConfigManualSource {
    enabled: bool,
    events: Option<PathBuf>,
    user: Option<String>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
struct ConfigRedaction {
    key_env: Option<String>,
}

#[derive(Debug)]
struct ConfiguredSourceFailure {
    name: String,
    error: String,
}

#[derive(Debug, Default)]
struct IntakeSourcePlan {
    failures: Vec<ConfiguredSourceFailure>,
    explanations: Vec<IntakeSourceExplanation>,
}

#[derive(Debug)]
struct IntakeSourceExplanation {
    name: String,
    decision: IntakeSourceDecision,
    reason: String,
}

#[derive(Debug, Clone, Copy)]
enum IntakeSourceDecision {
    Included,
    Skipped,
}

#[derive(Debug)]
struct ConfiguredSourceOutputs {
    successes: Vec<(String, IngestOutput)>,
    failures: Vec<ConfiguredSourceFailure>,
}

#[derive(Debug)]
struct ConfiguredRunResult {
    configured: ConfiguredSourceOutputs,
    outputs: shiplog_engine::RunOutputs,
    ws_source: WorkstreamSource,
    run_id: String,
}

#[derive(Debug, Clone)]
struct RedactionKey {
    key: Option<String>,
}

impl RedactionKey {
    fn resolve(redact_key: Option<String>, bundle_profile: &BundleProfile) -> Result<Self> {
        Self::resolve_with_env(redact_key, bundle_profile, "SHIPLOG_REDACT_KEY")
    }

    fn resolve_with_env(
        redact_key: Option<String>,
        bundle_profile: &BundleProfile,
        key_env: &str,
    ) -> Result<Self> {
        let key = redact_key.or_else(|| std::env::var(key_env).ok());
        if key.is_none() && !matches!(bundle_profile, BundleProfile::Internal) {
            anyhow::bail!(
                "{} profile requires --redact-key or {key_env}",
                bundle_profile
            );
        }
        Ok(Self { key })
    }

    fn engine_key(&self) -> &str {
        self.key.as_deref().unwrap_or("")
    }

    fn render_profiles(&self) -> bool {
        self.key.is_some()
    }
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

fn run_intake(args: IntakeArgs) -> Result<()> {
    let config_setup = ensure_intake_config(&args.config, &args.sources)?;
    let mut config_model = load_shiplog_config(&args.config)?;
    ensure_supported_config_version(&config_model)?;

    let base_dir = config_base_dir(&args.config);
    let out = args
        .out
        .clone()
        .unwrap_or_else(|| config_default_out(&config_model, &base_dir));
    let bundle_profile = args
        .profile
        .clone()
        .map(Ok)
        .unwrap_or_else(|| doctor_config_profile(config_model.defaults.profile.as_deref()))?;
    let redaction_key = RedactionKey::resolve_with_env(
        args.redact_key.clone(),
        &bundle_profile,
        &config_redaction_key_env(&config_model),
    )?;
    let explicit_sources = dedupe_sources(&args.sources);
    let mut intake_plan =
        prepare_intake_sources(&args.config, &mut config_model, &explicit_sources)?;
    intake_plan
        .explanations
        .splice(0..0, config_setup.source_explanations);
    let window = resolve_multi_window(args.window.clone(), &config_model)?;
    let mut configured = collect_configured_sources(&args.config, &config_model, window, &out)
        .with_context(|| {
            format!(
                "collect usable intake sources from {}",
                args.config.display()
            )
        })?;
    intake_plan.failures.append(&mut configured.failures);
    configured.failures = intake_plan.failures;

    let clusterer = build_clusterer(false, "", "", None);
    let (engine, redactor) = create_engine(redaction_key.engine_key(), clusterer, &bundle_profile);
    let engine = engine.with_profile_rendering(redaction_key.render_profiles());
    let result = run_configured_multi_pipeline(
        &config_model,
        &out,
        window,
        &bundle_profile,
        args.conflict,
        configured,
        false,
        false,
        &engine,
        redactor,
    )?;

    println!("Review intake complete.");
    if config_setup.created {
        println!("Config: created {}", args.config.display());
    } else {
        println!("Config: {}", args.config.display());
    }
    println!("Run: {}", result.run_id);
    println!("Packet: {}", result.outputs.packet_md.display());
    println!();

    println!("Collected:");
    for (name, ingest) in &result.configured.successes {
        println!(
            "- {}: success, {}",
            display_source_label(name),
            event_count_phrase(ingest.events.len())
        );
    }
    if result.configured.failures.is_empty() {
        println!("Skipped:");
        println!("- None");
    } else {
        println!("Skipped:");
        for failure in &result.configured.failures {
            println!(
                "- {}: {}",
                display_source_label(&failure.name),
                failure.error
            );
        }
    }
    if args.explain {
        println!();
        print_intake_explanations(&intake_plan.explanations);
    }
    println!();

    println!("Artifacts:");
    print_outputs(&result.outputs, result.ws_source.clone());
    println!();
    print_review(&result.outputs.out_dir, false)?;

    if args.no_open {
        println!();
        println!("Open later:");
        println!(
            "1. shiplog open packet --out {} --run {} --print-path",
            quote_cli_value(&out.display().to_string()),
            result.run_id
        );
    } else {
        println!();
        open_or_print_path(&result.outputs.packet_md, false)?;
    }

    Ok(())
}

#[derive(Debug, Default)]
struct IntakeConfigSetup {
    created: bool,
    source_explanations: Vec<IntakeSourceExplanation>,
}

fn ensure_intake_config(
    config_path: &Path,
    requested_sources: &[InitSource],
) -> Result<IntakeConfigSetup> {
    if config_path.exists() {
        return Ok(IntakeConfigSetup::default());
    }

    let selected = selected_intake_sources(requested_sources);
    let source_explanations = intake_autodetect_explanations(requested_sources, &selected);
    let config = render_init_config(&selected);
    write_init_file(config_path, &config)?;

    if init_source_enabled(&selected, InitSource::Manual) {
        let manual_path = config_base_dir(config_path).join(MANUAL_EVENTS_FILENAME);
        if !manual_path.exists() {
            write_init_file(&manual_path, &render_manual_events_template())?;
        }
    }

    Ok(IntakeConfigSetup {
        created: true,
        source_explanations,
    })
}

fn selected_intake_sources(requested_sources: &[InitSource]) -> Vec<InitSource> {
    if !requested_sources.is_empty() {
        return dedupe_sources(requested_sources);
    }

    let mut selected = Vec::new();
    if env_var_present("GITHUB_TOKEN") {
        selected.push(InitSource::Github);
    }
    if env_var_present("GITLAB_TOKEN") {
        selected.push(InitSource::Gitlab);
    }
    if Path::new(".git").exists() {
        selected.push(InitSource::Git);
    }
    if Path::new("ledger.events.jsonl").exists() && Path::new("coverage.manifest.json").exists() {
        selected.push(InitSource::Json);
    }
    selected.push(InitSource::Manual);
    dedupe_sources(&selected)
}

fn dedupe_sources(sources: &[InitSource]) -> Vec<InitSource> {
    let mut selected = Vec::new();
    for source in sources {
        if !selected.contains(source) {
            selected.push(*source);
        }
    }
    selected
}

fn intake_autodetect_explanations(
    requested_sources: &[InitSource],
    selected: &[InitSource],
) -> Vec<IntakeSourceExplanation> {
    if !requested_sources.is_empty() {
        return Vec::new();
    }

    let mut explanations = Vec::new();
    if !init_source_enabled(selected, InitSource::Github) {
        push_intake_explanation(
            &mut explanations,
            "github",
            IntakeSourceDecision::Skipped,
            "GITHUB_TOKEN not found",
        );
    }
    if !init_source_enabled(selected, InitSource::Gitlab) {
        push_intake_explanation(
            &mut explanations,
            "gitlab",
            IntakeSourceDecision::Skipped,
            "GITLAB_TOKEN not found",
        );
    }
    if !init_source_enabled(selected, InitSource::Jira) {
        push_intake_explanation(
            &mut explanations,
            "jira",
            IntakeSourceDecision::Skipped,
            "JIRA_TOKEN not found or Jira source config missing",
        );
    }
    if !init_source_enabled(selected, InitSource::Linear) {
        push_intake_explanation(
            &mut explanations,
            "linear",
            IntakeSourceDecision::Skipped,
            "LINEAR_API_KEY not found or Linear source config missing",
        );
    }
    if !init_source_enabled(selected, InitSource::Git) {
        push_intake_explanation(
            &mut explanations,
            "git",
            IntakeSourceDecision::Skipped,
            "current directory is not a git repo",
        );
    }
    if !init_source_enabled(selected, InitSource::Json) {
        push_intake_explanation(
            &mut explanations,
            "json",
            IntakeSourceDecision::Skipped,
            "ledger.events.jsonl and coverage.manifest.json not found",
        );
    }

    explanations
}

fn prepare_intake_sources(
    config_path: &Path,
    config: &mut ShiplogConfig,
    explicit_sources: &[InitSource],
) -> Result<IntakeSourcePlan> {
    let base_dir = config_base_dir(config_path);
    let mut plan = IntakeSourcePlan::default();

    if let Some(source) = config.sources.github.as_mut() {
        if !intake_source_in_scope(explicit_sources, InitSource::Github) {
            source.enabled = false;
        } else if source.enabled {
            if optional_config_string(source.user.as_deref()).is_some() && source.me {
                source.enabled = false;
                push_intake_skip(&mut plan, "github", "configured both user and me");
            } else if !env_var_present("GITHUB_TOKEN") {
                source.enabled = false;
                push_intake_skip(&mut plan, "github", "missing GITHUB_TOKEN");
            } else if source.me {
                let api_base = optional_config_string(source.api_base.as_deref())
                    .unwrap_or_else(|| "https://api.github.com".to_string());
                match discover_github_user(&api_base, None) {
                    Ok(user) => {
                        push_intake_include(
                            &mut plan,
                            "github",
                            format!("GITHUB_TOKEN found; --me resolved as {user}"),
                        );
                        source.user = Some(user);
                        source.me = false;
                    }
                    Err(err) => {
                        source.enabled = false;
                        push_intake_skip(&mut plan, "github", err.to_string());
                    }
                }
            } else if optional_config_string(source.user.as_deref()).is_none() && !source.me {
                source.enabled = false;
                push_intake_skip(&mut plan, "github", "set sources.github.user or me = true");
            } else if let Some(user) = optional_config_string(source.user.as_deref()) {
                push_intake_include(
                    &mut plan,
                    "github",
                    format!("GITHUB_TOKEN found; user configured as {user}"),
                );
            }
        }
    }

    if let Some(source) = config.sources.gitlab.as_mut() {
        if !intake_source_in_scope(explicit_sources, InitSource::Gitlab) {
            source.enabled = false;
        } else if source.enabled {
            if optional_config_string(source.user.as_deref()).is_some() && source.me {
                source.enabled = false;
                push_intake_skip(&mut plan, "gitlab", "configured both user and me");
            } else if !env_var_present("GITLAB_TOKEN") {
                source.enabled = false;
                push_intake_skip(&mut plan, "gitlab", "missing GITLAB_TOKEN");
            } else if source.me {
                let instance = optional_config_string(source.instance.as_deref())
                    .unwrap_or_else(|| "gitlab.com".to_string());
                match discover_gitlab_user(&instance, None) {
                    Ok(user) => {
                        push_intake_include(
                            &mut plan,
                            "gitlab",
                            format!("GITLAB_TOKEN found; --me resolved as {user}"),
                        );
                        source.user = Some(user);
                        source.me = false;
                    }
                    Err(err) => {
                        source.enabled = false;
                        push_intake_skip(&mut plan, "gitlab", err.to_string());
                    }
                }
            } else if optional_config_string(source.user.as_deref()).is_none() && !source.me {
                source.enabled = false;
                push_intake_skip(&mut plan, "gitlab", "set sources.gitlab.user or me = true");
            } else if let Some(user) = optional_config_string(source.user.as_deref()) {
                push_intake_include(
                    &mut plan,
                    "gitlab",
                    format!("GITLAB_TOKEN found; user configured as {user}"),
                );
            }
        }
    }

    if let Some(source) = config.sources.jira.as_mut() {
        if !intake_source_in_scope(explicit_sources, InitSource::Jira) {
            source.enabled = false;
        } else if source.enabled {
            let user = optional_config_string(source.user.as_deref());
            let instance = optional_config_string(source.instance.as_deref());
            if !env_var_present("JIRA_TOKEN") {
                source.enabled = false;
                push_intake_skip(&mut plan, "jira", "missing JIRA_TOKEN");
            } else if user.as_deref().is_none_or(is_unfilled_placeholder) {
                source.enabled = false;
                push_intake_skip(&mut plan, "jira", "set sources.jira.user");
            } else if instance.as_deref().is_none_or(is_unfilled_placeholder) {
                source.enabled = false;
                push_intake_skip(&mut plan, "jira", "set sources.jira.instance");
            } else {
                push_intake_include(
                    &mut plan,
                    "jira",
                    "JIRA_TOKEN found; assignee and instance configured",
                );
            }
        }
    }

    if let Some(source) = config.sources.linear.as_mut() {
        if !intake_source_in_scope(explicit_sources, InitSource::Linear) {
            source.enabled = false;
        } else if source.enabled {
            let user_id = optional_config_string(source.user_id.as_deref());
            if !env_var_present("LINEAR_API_KEY") {
                source.enabled = false;
                push_intake_skip(&mut plan, "linear", "missing LINEAR_API_KEY");
            } else if user_id.as_deref().is_none_or(is_unfilled_placeholder) {
                source.enabled = false;
                push_intake_skip(&mut plan, "linear", "set sources.linear.user_id");
            } else {
                push_intake_include(
                    &mut plan,
                    "linear",
                    "LINEAR_API_KEY found; user id configured",
                );
            }
        }
    }

    if let Some(source) = config.sources.git.as_mut() {
        if !intake_source_in_scope(explicit_sources, InitSource::Git) {
            source.enabled = false;
        } else if source.enabled {
            match source
                .repo
                .as_ref()
                .filter(|path| !path.as_os_str().is_empty())
                .map(|path| resolve_config_path(&base_dir, path))
            {
                Some(repo) if repo.exists() => {
                    push_intake_include(&mut plan, "git", format!("repo {} found", repo.display()));
                }
                Some(repo) => {
                    source.enabled = false;
                    push_intake_skip(
                        &mut plan,
                        "git",
                        format!("repo {} not found", repo.display()),
                    );
                }
                None => {
                    source.enabled = false;
                    push_intake_skip(&mut plan, "git", "set sources.git.repo");
                }
            }
        }
    }

    if let Some(source) = config.sources.json.as_mut() {
        if !intake_source_in_scope(explicit_sources, InitSource::Json) {
            source.enabled = false;
        } else if source.enabled {
            let events = source
                .events
                .as_ref()
                .filter(|path| !path.as_os_str().is_empty())
                .map(|path| resolve_config_path(&base_dir, path));
            let coverage = source
                .coverage
                .as_ref()
                .filter(|path| !path.as_os_str().is_empty())
                .map(|path| resolve_config_path(&base_dir, path));
            match (events, coverage) {
                (Some(events), Some(coverage)) if events.exists() && coverage.exists() => {
                    push_intake_include(&mut plan, "json", "events and coverage files found");
                }
                (Some(events), Some(coverage)) => {
                    source.enabled = false;
                    push_intake_skip(
                        &mut plan,
                        "json",
                        format!("missing {} or {}", events.display(), coverage.display()),
                    );
                }
                _ => {
                    source.enabled = false;
                    push_intake_skip(
                        &mut plan,
                        "json",
                        "set sources.json.events and sources.json.coverage",
                    );
                }
            }
        }
    }

    if let Some(source) = config.sources.manual.as_mut() {
        if !intake_source_in_scope(explicit_sources, InitSource::Manual) {
            source.enabled = false;
        } else if source.enabled {
            if source
                .events
                .as_ref()
                .is_none_or(|path| path.as_os_str().is_empty())
            {
                source.events = Some(PathBuf::from(MANUAL_EVENTS_FILENAME));
            }
            let events = source
                .events
                .as_ref()
                .map(|path| resolve_config_path(&base_dir, path))
                .expect("manual events path set above");
            let existed = events.exists();
            if !events.exists() {
                write_init_file(&events, &render_manual_events_template())?;
            }
            if existed {
                push_intake_include(&mut plan, "manual", "manual_events.yaml found");
            } else {
                push_intake_include(&mut plan, "manual", "manual_events.yaml created");
            }
        }
    }

    Ok(plan)
}

fn intake_source_in_scope(explicit_sources: &[InitSource], source: InitSource) -> bool {
    explicit_sources.is_empty() || explicit_sources.contains(&source)
}

fn push_intake_include(plan: &mut IntakeSourcePlan, name: &str, reason: impl Into<String>) {
    push_intake_explanation(
        &mut plan.explanations,
        name,
        IntakeSourceDecision::Included,
        reason,
    );
}

fn push_intake_skip(plan: &mut IntakeSourcePlan, name: &str, error: impl Into<String>) {
    let error = error.into();
    plan.failures.push(ConfiguredSourceFailure {
        name: name.to_string(),
        error: error.clone(),
    });
    push_intake_explanation(
        &mut plan.explanations,
        name,
        IntakeSourceDecision::Skipped,
        error,
    );
}

fn push_intake_explanation(
    explanations: &mut Vec<IntakeSourceExplanation>,
    name: &str,
    decision: IntakeSourceDecision,
    reason: impl Into<String>,
) {
    explanations.push(IntakeSourceExplanation {
        name: name.to_string(),
        decision,
        reason: reason.into(),
    });
}

fn print_intake_explanations(explanations: &[IntakeSourceExplanation]) {
    println!("Source decisions:");
    if explanations.is_empty() {
        println!("- None");
        return;
    }

    for explanation in explanations {
        let decision = match explanation.decision {
            IntakeSourceDecision::Included => "included",
            IntakeSourceDecision::Skipped => "skipped",
        };
        println!(
            "- {}: {}, {}",
            display_source_label(&explanation.name),
            decision,
            explanation.reason
        );
    }
}

fn is_unfilled_placeholder(value: &str) -> bool {
    let value = value.trim().to_ascii_lowercase();
    value.is_empty() || value.starts_with("your-") || value == "company.atlassian.net"
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

fn init_next_command(_selected: &[InitSource]) -> &'static str {
    "shiplog collect multi --last-6-months"
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

[shiplog]
config_version = 1

[defaults]
out = "./out"
window = "last-6-months"
profile = "internal"
include_reviews = true

[user]
label = "Your Name"

[sources.github]
enabled = {github}
user = ""
me = {github}
mode = "merged"
include_reviews = true

[sources.gitlab]
enabled = {gitlab}
user = ""
me = {gitlab}
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

fn run_journal_add(args: JournalAddArgs) -> Result<()> {
    let date = resolve_journal_date(args.date, args.start, args.end)?;
    let id = match args.id {
        Some(id) => required_text_arg("--id", &id)?,
        None => generated_journal_id(&date, &args.title),
    };
    validate_journal_id(&id)?;

    let mut file = if args.events.exists() {
        read_manual_events(&args.events)?
    } else {
        create_empty_file()
    };
    if file.version != 1 {
        anyhow::bail!(
            "unsupported manual events version {}; expected 1",
            file.version
        );
    }
    if file.events.iter().any(|entry| entry.id == id) {
        anyhow::bail!(
            "manual event id {id:?} already exists in {}; use --id for a distinct entry",
            args.events.display()
        );
    }

    let entry = ManualEventEntry {
        id,
        event_type: args.event_type.into(),
        date,
        title: required_text_arg("--title", &args.title)?,
        description: optional_text_arg(args.description),
        workstream: optional_text_arg(args.workstream),
        tags: normalize_journal_tags(args.tags)?,
        receipts: parse_journal_receipts(&args.receipts)?,
        impact: optional_text_arg(args.impact),
    };

    if args.dry_run {
        print_journal_entry("Would add manual event", &args.events, &entry);
        return Ok(());
    }

    if let Some(parent) = args
        .events
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create manual events directory {}", parent.display()))?;
    }

    file.events.push(entry.clone());
    write_manual_events(&args.events, &file)?;

    print_journal_entry("Added manual event", &args.events, &entry);
    println!("Next:");
    println!("  shiplog collect multi --last-6-months");

    Ok(())
}

fn resolve_journal_date(
    date: Option<NaiveDate>,
    start: Option<NaiveDate>,
    end: Option<NaiveDate>,
) -> Result<ManualDate> {
    match (date, start, end) {
        (Some(date), None, None) => Ok(ManualDate::Single(date)),
        (None, Some(start), Some(end)) if start <= end => Ok(ManualDate::Range { start, end }),
        (None, Some(start), Some(end)) => {
            anyhow::bail!("journal date range must satisfy --start {start} <= --end {end}")
        }
        (None, None, None) => anyhow::bail!("journal add requires --date or --start/--end"),
        _ => anyhow::bail!("use either --date or --start/--end, not both"),
    }
}

fn generated_journal_id(date: &ManualDate, title: &str) -> String {
    let date_label = match date {
        ManualDate::Single(date) => date.to_string(),
        ManualDate::Range { start, end } => format!("{start}-to-{end}"),
    };
    let slug = slugify_journal_title(title);
    format!("manual-{date_label}-{slug}")
}

fn slugify_journal_title(title: &str) -> String {
    let mut slug = String::new();
    let mut pending_dash = false;
    for ch in title.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            if pending_dash && !slug.is_empty() {
                slug.push('-');
            }
            slug.push(ch);
            pending_dash = false;
        } else {
            pending_dash = true;
        }
    }

    if slug.is_empty() {
        "event".to_string()
    } else {
        slug
    }
}

fn validate_journal_id(id: &str) -> Result<()> {
    if id.trim().is_empty() {
        anyhow::bail!("manual event id cannot be blank");
    }
    if id.chars().any(char::is_whitespace) {
        anyhow::bail!("manual event id cannot contain whitespace: {id:?}");
    }
    Ok(())
}

fn required_text_arg(name: &str, value: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        anyhow::bail!("{name} cannot be blank");
    }
    Ok(trimmed.to_string())
}

fn optional_text_arg(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn normalize_journal_tags(tags: Vec<String>) -> Result<Vec<String>> {
    let mut normalized = Vec::new();
    for tag in tags {
        let tag = tag.trim();
        if tag.is_empty() {
            anyhow::bail!("journal tag cannot be blank");
        }
        if !normalized.iter().any(|existing| existing == tag) {
            normalized.push(tag.to_string());
        }
    }
    Ok(normalized)
}

fn parse_journal_receipts(values: &[String]) -> Result<Vec<Link>> {
    let mut receipts = Vec::new();
    for value in values {
        let (label, url) = match value.split_once('=') {
            Some((label, url)) => (label.trim(), url.trim()),
            None => ("receipt", value.trim()),
        };
        if label.is_empty() {
            anyhow::bail!("journal receipt label cannot be blank in {value:?}");
        }
        if url.is_empty() {
            anyhow::bail!("journal receipt URL cannot be blank in {value:?}");
        }
        receipts.push(Link {
            label: label.to_string(),
            url: url.to_string(),
        });
    }
    Ok(receipts)
}

fn print_journal_entry(label: &str, path: &Path, entry: &ManualEventEntry) {
    println!("{label}: {}", entry.id);
    println!("File: {}", path.display());
    println!("Type: {}", entry.event_type);
    println!("Date: {}", journal_date_label(&entry.date));
    println!("Title: {}", entry.title);
    if let Some(workstream) = &entry.workstream {
        println!("Workstream: {workstream}");
    }
    if !entry.tags.is_empty() {
        println!("Tags: {}", entry.tags.join(", "));
    }
    if !entry.receipts.is_empty() {
        println!("Receipts: {}", entry.receipts.len());
    }
}

fn journal_date_label(date: &ManualDate) -> String {
    match date {
        ManualDate::Single(date) => date.to_string(),
        ManualDate::Range { start, end } => format!("{start}..{end}"),
    }
}

fn load_shiplog_config(config_path: &Path) -> Result<ShiplogConfig> {
    let text = std::fs::read_to_string(config_path)
        .with_context(|| format!("read {}", config_path.display()))?;
    toml::from_str(&text).with_context(|| format!("parse {}", config_path.display()))
}

#[derive(Default)]
struct DoctorReport {
    errors: usize,
}

impl DoctorReport {
    fn ok(&self, label: &str, detail: impl AsRef<str>) {
        let detail = detail.as_ref();
        if detail.is_empty() {
            println!("{label}: ok");
        } else {
            println!("{label}: ok, {detail}");
        }
    }

    fn disabled(&self, label: &str) {
        println!("{label}: disabled");
    }

    fn error(&mut self, label: &str, detail: impl AsRef<str>) {
        self.errors += 1;
        println!("{label}: error, {}", detail.as_ref());
    }
}

fn run_doctor(config_path: &Path, sources: &[InitSource]) -> Result<()> {
    let mut report = DoctorReport::default();

    if !config_path.exists() {
        report.error(
            "Config",
            format!(
                "{} not found; run `shiplog init` first",
                config_path.display()
            ),
        );
        anyhow::bail!("doctor found {} issue(s)", report.errors);
    }

    let config = match load_shiplog_config(config_path) {
        Ok(config) => {
            report.ok("Config", config_path.display().to_string());
            config
        }
        Err(err) => {
            report.error("Config", err.to_string());
            anyhow::bail!("doctor found {} issue(s)", report.errors);
        }
    };

    let base_dir = config_base_dir(config_path);
    doctor_config_version(&mut report, &config);
    if report.errors > 0 {
        anyhow::bail!("doctor found {} issue(s)", report.errors);
    }
    doctor_defaults(&mut report, &config, &base_dir);
    doctor_sources(&mut report, &config, &base_dir, sources);

    if report.errors > 0 {
        anyhow::bail!("doctor found {} issue(s)", report.errors);
    }

    Ok(())
}

fn doctor_config_version(report: &mut DoctorReport, config: &ShiplogConfig) {
    match config_version_state(config) {
        Ok(version) => report.ok("Config version", version.label()),
        Err(err) => report.error("Config version", err.to_string()),
    }
}

#[derive(Debug)]
struct ConfigIssue {
    label: &'static str,
    detail: String,
}

#[derive(Debug, Clone, Copy)]
struct ConfigVersionState {
    version: i64,
    explicit: bool,
}

enum ConfigMigration {
    Current(ConfigVersionState),
    AddVersion { text: String },
}

fn config_issue(label: &'static str, detail: impl Into<String>) -> ConfigIssue {
    ConfigIssue {
        label,
        detail: detail.into(),
    }
}

fn run_config_validate(config_path: &Path) -> Result<()> {
    let config = load_config_for_command(config_path)?;
    let base_dir = config_base_dir(config_path);
    let issues = validate_shiplog_config(&config, &base_dir);

    println!("Config: ok, {}", config_path.display());
    if issues.is_empty() {
        print_config_validation_summary(&config, &base_dir)?;
        println!("Config valid");
        return Ok(());
    }

    for issue in &issues {
        println!("{}: error, {}", issue.label, issue.detail);
    }
    anyhow::bail!("config validate found {} issue(s)", issues.len())
}

fn run_config_explain(config_path: &Path) -> Result<()> {
    let config = load_config_for_command(config_path)?;
    let base_dir = config_base_dir(config_path);
    let issues = validate_shiplog_config(&config, &base_dir);
    if !issues.is_empty() {
        println!("Config: ok, {}", config_path.display());
        for issue in &issues {
            println!("{}: error, {}", issue.label, issue.detail);
        }
        anyhow::bail!("config explain requires a valid config")
    }

    print_config_explanation(config_path, &config, &base_dir)
}

fn run_config_migrate(config_path: &Path, dry_run: bool) -> Result<()> {
    let text = read_config_for_command(config_path)?;
    let migration = plan_config_migration(config_path, &text)?;

    println!("Config: ok, {}", config_path.display());
    match migration {
        ConfigMigration::Current(version) => {
            println!("Migration: already current ({})", version.label());
            Ok(())
        }
        ConfigMigration::AddVersion { text } => {
            if dry_run {
                println!(
                    "Migration: would add [shiplog] config_version = {CURRENT_CONFIG_VERSION}"
                );
            } else {
                std::fs::write(config_path, text)
                    .with_context(|| format!("write {}", config_path.display()))?;
                println!("Migration: added [shiplog] config_version = {CURRENT_CONFIG_VERSION}");
                println!("Config migrated");
            }
            Ok(())
        }
    }
}

fn load_config_for_command(config_path: &Path) -> Result<ShiplogConfig> {
    if !config_path.exists() {
        anyhow::bail!(
            "{} not found; run `shiplog init` first",
            config_path.display()
        );
    }
    load_shiplog_config(config_path)
}

fn read_config_for_command(config_path: &Path) -> Result<String> {
    if !config_path.exists() {
        anyhow::bail!(
            "{} not found; run `shiplog init` first",
            config_path.display()
        );
    }
    std::fs::read_to_string(config_path).with_context(|| format!("read {}", config_path.display()))
}

fn validate_shiplog_config(config: &ShiplogConfig, base_dir: &Path) -> Vec<ConfigIssue> {
    let mut issues = Vec::new();

    if let Err(err) = config_version_state(config) {
        issues.push(config_issue("Version", err.to_string()));
    }
    if let Err(err) = resolve_multi_window(DateArgs::default(), config) {
        issues.push(config_issue("Window", err.to_string()));
    }
    if let Err(err) = doctor_config_profile(config.defaults.profile.as_deref()) {
        issues.push(config_issue("Profile", err.to_string()));
    }
    if let Err(err) = validate_config_output_path(&config_default_out(config, base_dir)) {
        issues.push(config_issue("Output", err.to_string()));
    }
    if config_enabled_source_names(config).is_empty() {
        issues.push(config_issue(
            "Sources",
            "enable at least one [sources.<name>] section",
        ));
    }

    validate_config_github(config, &mut issues);
    validate_config_gitlab(config, &mut issues);
    validate_config_jira(config, &mut issues);
    validate_config_linear(config, &mut issues);
    validate_config_git(config, base_dir, &mut issues);
    validate_config_json(config, base_dir, &mut issues);
    validate_config_manual(config, base_dir, &mut issues);

    issues
}

impl ConfigVersionState {
    fn label(self) -> String {
        if self.explicit {
            self.version.to_string()
        } else {
            format!("{} (implicit)", self.version)
        }
    }
}

fn config_version_state(config: &ShiplogConfig) -> Result<ConfigVersionState> {
    match config.shiplog.config_version {
        Some(CURRENT_CONFIG_VERSION) => Ok(ConfigVersionState {
            version: CURRENT_CONFIG_VERSION,
            explicit: true,
        }),
        Some(version) => anyhow::bail!(
            "unsupported config_version {version}; expected {CURRENT_CONFIG_VERSION}; run `shiplog config migrate` or update shiplog"
        ),
        None => Ok(ConfigVersionState {
            version: CURRENT_CONFIG_VERSION,
            explicit: false,
        }),
    }
}

fn ensure_supported_config_version(config: &ShiplogConfig) -> Result<()> {
    config_version_state(config).map(|_| ())
}

fn plan_config_migration(config_path: &Path, text: &str) -> Result<ConfigMigration> {
    let config = load_shiplog_config(config_path)?;
    let version = config_version_state(&config)?;
    if version.explicit {
        return Ok(ConfigMigration::Current(version));
    }

    Ok(ConfigMigration::AddVersion {
        text: add_config_version_to_text(text)?,
    })
}

fn add_config_version_to_text(text: &str) -> Result<String> {
    let has_shiplog_header = text.lines().any(is_shiplog_table_header);
    if !has_shiplog_header {
        let parsed: toml::Table = toml::from_str(text).context("parse shiplog.toml")?;
        if parsed.contains_key("shiplog") {
            anyhow::bail!(
                "found a shiplog table but could not locate a [shiplog] header; add config_version manually"
            );
        }
    }
    if !has_shiplog_header {
        return Ok(format!(
            "[shiplog]\nconfig_version = {CURRENT_CONFIG_VERSION}\n\n{text}"
        ));
    }

    let mut migrated = String::with_capacity(text.len() + 32);
    let mut inserted = false;
    for line in text.split_inclusive('\n') {
        migrated.push_str(line);
        if !inserted && is_shiplog_table_header(line) {
            migrated.push_str(&format!("config_version = {CURRENT_CONFIG_VERSION}\n"));
            inserted = true;
        }
    }
    if !inserted && is_shiplog_table_header(text) {
        migrated.push_str(&format!("\nconfig_version = {CURRENT_CONFIG_VERSION}\n"));
        inserted = true;
    }
    if !inserted {
        anyhow::bail!(
            "found a shiplog table but could not locate a [shiplog] header; add config_version manually"
        );
    }
    Ok(migrated)
}

fn is_shiplog_table_header(line: &str) -> bool {
    let before_comment = line.split('#').next().unwrap_or("").trim();
    before_comment == "[shiplog]"
}

fn validate_config_output_path(out: &Path) -> Result<()> {
    if out.exists() {
        if !out.is_dir() {
            anyhow::bail!("{} exists but is not a directory", out.display());
        }
        return Ok(());
    }

    let parent = out
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    if !parent.exists() {
        anyhow::bail!(
            "{} does not exist; create it or choose a different defaults.out",
            parent.display()
        );
    }
    if !parent.is_dir() {
        anyhow::bail!("{} exists but is not a directory", parent.display());
    }
    Ok(())
}

fn validate_config_github(config: &ShiplogConfig, issues: &mut Vec<ConfigIssue>) {
    let Some(source) = config
        .sources
        .github
        .as_ref()
        .filter(|source| source.enabled)
    else {
        return;
    };

    validate_config_user_or_me(
        "GitHub",
        "sources.github.user",
        source.user.as_deref(),
        source.me,
        issues,
    );
    let mode = optional_config_string(source.mode.as_deref()).unwrap_or_else(|| "merged".into());
    if !matches!(mode.as_str(), "merged" | "created") {
        issues.push(config_issue(
            "GitHub",
            format!("sources.github.mode must be merged or created, got {mode:?}"),
        ));
    }
}

fn validate_config_gitlab(config: &ShiplogConfig, issues: &mut Vec<ConfigIssue>) {
    let Some(source) = config
        .sources
        .gitlab
        .as_ref()
        .filter(|source| source.enabled)
    else {
        return;
    };

    validate_config_user_or_me(
        "GitLab",
        "sources.gitlab.user",
        source.user.as_deref(),
        source.me,
        issues,
    );
    let instance =
        optional_config_string(source.instance.as_deref()).unwrap_or_else(|| "gitlab.com".into());
    if let Err(err) = gitlab_api_base(&instance) {
        issues.push(config_issue("GitLab", err.to_string()));
    }
    if let Some(state) = non_empty_string(source.state.as_deref())
        && let Err(err) = state.parse::<MrState>()
    {
        issues.push(config_issue(
            "GitLab",
            format!("parse state {state:?}: {err}"),
        ));
    }
}

fn validate_config_jira(config: &ShiplogConfig, issues: &mut Vec<ConfigIssue>) {
    let Some(source) = config.sources.jira.as_ref().filter(|source| source.enabled) else {
        return;
    };

    if let Err(err) = required_config_string("jira", "user", source.user.as_deref()) {
        issues.push(config_issue("Jira", err.to_string()));
    }
    if let Err(err) = required_config_string("jira", "instance", source.instance.as_deref()) {
        issues.push(config_issue("Jira", err.to_string()));
    }
    let status = source.status.as_deref().unwrap_or("done");
    if let Err(err) = status.parse::<IssueStatus>() {
        issues.push(config_issue(
            "Jira",
            format!("parse status {status:?}: {err}"),
        ));
    }
}

fn validate_config_linear(config: &ShiplogConfig, issues: &mut Vec<ConfigIssue>) {
    let Some(source) = config
        .sources
        .linear
        .as_ref()
        .filter(|source| source.enabled)
    else {
        return;
    };

    if let Err(err) = required_config_string("linear", "user_id", source.user_id.as_deref()) {
        issues.push(config_issue("Linear", err.to_string()));
    }
    let status = source.status.as_deref().unwrap_or("done");
    if let Err(err) = status.parse::<LinearIssueStatus>() {
        issues.push(config_issue(
            "Linear",
            format!("parse status {status:?}: {err}"),
        ));
    }
}

fn validate_config_git(config: &ShiplogConfig, base_dir: &Path, issues: &mut Vec<ConfigIssue>) {
    let Some(source) = config.sources.git.as_ref().filter(|source| source.enabled) else {
        return;
    };

    match required_config_path(base_dir, "git", "repo", source.repo.as_ref()) {
        Ok(repo) if repo.is_dir() => {}
        Ok(repo) => issues.push(config_issue(
            "Git",
            format!("{} is not a directory", repo.display()),
        )),
        Err(err) => issues.push(config_issue("Git", err.to_string())),
    }
}

fn validate_config_json(config: &ShiplogConfig, base_dir: &Path, issues: &mut Vec<ConfigIssue>) {
    let Some(source) = config.sources.json.as_ref().filter(|source| source.enabled) else {
        return;
    };

    match required_config_path(base_dir, "json", "events", source.events.as_ref()) {
        Ok(events) if events.exists() => {}
        Ok(events) => issues.push(config_issue(
            "JSON",
            format!("{} not found", events.display()),
        )),
        Err(err) => issues.push(config_issue("JSON", err.to_string())),
    }
    match required_config_path(base_dir, "json", "coverage", source.coverage.as_ref()) {
        Ok(coverage) if coverage.exists() => {}
        Ok(coverage) => issues.push(config_issue(
            "JSON",
            format!("{} not found", coverage.display()),
        )),
        Err(err) => issues.push(config_issue("JSON", err.to_string())),
    }
}

fn validate_config_manual(config: &ShiplogConfig, base_dir: &Path, issues: &mut Vec<ConfigIssue>) {
    let Some(source) = config
        .sources
        .manual
        .as_ref()
        .filter(|source| source.enabled)
    else {
        return;
    };

    match required_config_path(base_dir, "manual", "events", source.events.as_ref()) {
        Ok(events) if events.exists() => {}
        Ok(events) => issues.push(config_issue(
            "Manual",
            format!("{} not found", events.display()),
        )),
        Err(err) => issues.push(config_issue("Manual", err.to_string())),
    }
}

fn validate_config_user_or_me(
    label: &'static str,
    field: &str,
    user: Option<&str>,
    me: bool,
    issues: &mut Vec<ConfigIssue>,
) {
    match (optional_config_string(user), me) {
        (Some(_), true) => issues.push(config_issue(
            label,
            format!("use either {field} or me = true, not both"),
        )),
        (None, false) => issues.push(config_issue(label, format!("set {field} or me = true"))),
        _ => {}
    }
}

fn print_config_validation_summary(config: &ShiplogConfig, base_dir: &Path) -> Result<()> {
    let version = config_version_state(config)?;
    let window = resolve_multi_window(DateArgs::default(), config)?;
    let out = config_default_out(config, base_dir);
    let profile = doctor_config_profile(config.defaults.profile.as_deref())?;
    println!("Version: ok, {}", version.label());
    println!("Window: ok, {}", window.window_label());
    println!("Output: ok, {}", out.display());
    println!("Profile: ok, {}", profile.as_str());
    println!(
        "Sources: ok, {}",
        config_enabled_source_names(config).join(", ")
    );
    Ok(())
}

fn print_config_explanation(
    config_path: &Path,
    config: &ShiplogConfig,
    base_dir: &Path,
) -> Result<()> {
    let version = config_version_state(config)?;
    let window = resolve_multi_window(DateArgs::default(), config)?;
    let out = config_default_out(config, base_dir);
    let profile = doctor_config_profile(config.defaults.profile.as_deref())?;
    let window_setting = non_empty_string(config.defaults.window.as_deref())
        .unwrap_or_else(|| "last-6-months (default)".to_string());
    let out_setting = config
        .defaults
        .out
        .as_ref()
        .filter(|path| !path.as_os_str().is_empty())
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "./out (default)".to_string());
    let include_reviews = config.defaults.include_reviews.unwrap_or(false);

    println!("Config: {}", config_path.display());
    println!("Config metadata:");
    println!("- config_version: {}", version.label());
    println!("Resolved defaults:");
    println!(
        "- window: {} -> {}..{}",
        window_setting, window.since, window.until
    );
    println!("- out: {} -> {}", out_setting, out.display());
    println!("- profile: {}", profile.as_str());
    println!("- include_reviews: {}", include_reviews);
    println!(
        "- user.label: {}",
        optional_config_string(config.user.label.as_deref()).unwrap_or_else(|| "-".to_string())
    );
    println!("- redaction.key_env: {}", config_redaction_key_env(config));
    println!("Enabled sources:");
    for line in config_enabled_source_explanations(config, base_dir, &out) {
        println!("- {line}");
    }
    Ok(())
}

fn config_default_out(config: &ShiplogConfig, base_dir: &Path) -> PathBuf {
    config
        .defaults
        .out
        .as_ref()
        .filter(|path| !path.as_os_str().is_empty())
        .map(|path| resolve_config_path(base_dir, path))
        .unwrap_or_else(|| resolve_config_path(base_dir, Path::new("./out")))
}

fn config_redaction_key_env(config: &ShiplogConfig) -> String {
    optional_config_string(config.redaction.key_env.as_deref())
        .unwrap_or_else(|| "SHIPLOG_REDACT_KEY".to_string())
}

fn config_enabled_source_names(config: &ShiplogConfig) -> Vec<&'static str> {
    let mut names = Vec::new();
    if config
        .sources
        .github
        .as_ref()
        .is_some_and(|source| source.enabled)
    {
        names.push("github");
    }
    if config
        .sources
        .gitlab
        .as_ref()
        .is_some_and(|source| source.enabled)
    {
        names.push("gitlab");
    }
    if config
        .sources
        .jira
        .as_ref()
        .is_some_and(|source| source.enabled)
    {
        names.push("jira");
    }
    if config
        .sources
        .linear
        .as_ref()
        .is_some_and(|source| source.enabled)
    {
        names.push("linear");
    }
    if config
        .sources
        .git
        .as_ref()
        .is_some_and(|source| source.enabled)
    {
        names.push("git");
    }
    if config
        .sources
        .json
        .as_ref()
        .is_some_and(|source| source.enabled)
    {
        names.push("json");
    }
    if config
        .sources
        .manual
        .as_ref()
        .is_some_and(|source| source.enabled)
    {
        names.push("manual");
    }
    names
}

fn config_enabled_source_explanations(
    config: &ShiplogConfig,
    base_dir: &Path,
    out: &Path,
) -> Vec<String> {
    let mut lines = Vec::new();
    let default_include_reviews = config.defaults.include_reviews.unwrap_or(false);

    if let Some(source) = config
        .sources
        .github
        .as_ref()
        .filter(|source| source.enabled)
    {
        let identity = config_identity_label(source.user.as_deref(), source.me, "--me");
        let mode =
            optional_config_string(source.mode.as_deref()).unwrap_or_else(|| "merged".into());
        let include_reviews = source.include_reviews.unwrap_or(default_include_reviews);
        let cache = config_cache_label(
            resolve_config_cache_dir(base_dir, out, source.cache_dir.as_ref(), source.no_cache)
                .as_deref(),
        );
        lines.push(format!(
            "github: {identity}, mode {mode}, include_reviews {include_reviews}, cache {cache}"
        ));
    }
    if let Some(source) = config
        .sources
        .gitlab
        .as_ref()
        .filter(|source| source.enabled)
    {
        let identity = config_identity_label(source.user.as_deref(), source.me, "--me");
        let instance = optional_config_string(source.instance.as_deref())
            .unwrap_or_else(|| "gitlab.com".into());
        let state =
            optional_config_string(source.state.as_deref()).unwrap_or_else(|| "merged".into());
        let include_reviews = source.include_reviews.unwrap_or(default_include_reviews);
        let cache = config_cache_label(
            resolve_config_cache_dir(base_dir, out, source.cache_dir.as_ref(), source.no_cache)
                .as_deref(),
        );
        lines.push(format!(
            "gitlab: {identity}, instance {instance}, state {state}, include_reviews {include_reviews}, cache {cache}"
        ));
    }
    if let Some(source) = config.sources.jira.as_ref().filter(|source| source.enabled) {
        let user = optional_config_string(source.user.as_deref()).unwrap_or_else(|| "-".into());
        let instance =
            optional_config_string(source.instance.as_deref()).unwrap_or_else(|| "-".into());
        let status =
            optional_config_string(source.status.as_deref()).unwrap_or_else(|| "done".into());
        let auth_user = config_jira_auth_user_label(source);
        let cache = config_cache_label(
            resolve_config_cache_dir(base_dir, out, source.cache_dir.as_ref(), source.no_cache)
                .as_deref(),
        );
        lines.push(format!(
            "jira: user {user}, auth {auth_user}, instance {instance}, status {status}, cache {cache}"
        ));
    }
    if let Some(source) = config
        .sources
        .linear
        .as_ref()
        .filter(|source| source.enabled)
    {
        let user_id =
            optional_config_string(source.user_id.as_deref()).unwrap_or_else(|| "-".into());
        let status =
            optional_config_string(source.status.as_deref()).unwrap_or_else(|| "done".into());
        let project =
            optional_config_string(source.project.as_deref()).unwrap_or_else(|| "-".into());
        let cache = config_cache_label(
            resolve_config_cache_dir(base_dir, out, source.cache_dir.as_ref(), source.no_cache)
                .as_deref(),
        );
        lines.push(format!(
            "linear: user_id {user_id}, status {status}, project {project}, cache {cache}"
        ));
    }
    if let Some(source) = config.sources.git.as_ref().filter(|source| source.enabled) {
        let repo = source
            .repo
            .as_ref()
            .map(|path| resolve_config_path(base_dir, path).display().to_string())
            .unwrap_or_else(|| "-".to_string());
        let author = optional_config_string(source.author.as_deref()).unwrap_or_else(|| "-".into());
        lines.push(format!(
            "git: repo {repo}, author {author}, include_merges {}",
            source.include_merges
        ));
    }
    if let Some(source) = config.sources.json.as_ref().filter(|source| source.enabled) {
        let events = source
            .events
            .as_ref()
            .map(|path| resolve_config_path(base_dir, path).display().to_string())
            .unwrap_or_else(|| "-".to_string());
        let coverage = source
            .coverage
            .as_ref()
            .map(|path| resolve_config_path(base_dir, path).display().to_string())
            .unwrap_or_else(|| "-".to_string());
        lines.push(format!("json: events {events}, coverage {coverage}"));
    }
    if let Some(source) = config
        .sources
        .manual
        .as_ref()
        .filter(|source| source.enabled)
    {
        let events = source
            .events
            .as_ref()
            .map(|path| resolve_config_path(base_dir, path).display().to_string())
            .unwrap_or_else(|| "-".to_string());
        let user = optional_config_string(source.user.as_deref())
            .or_else(|| optional_config_string(config.user.label.as_deref()))
            .unwrap_or_else(|| "-".into());
        lines.push(format!("manual: events {events}, user {user}"));
    }

    lines
}

fn config_identity_label(user: Option<&str>, me: bool, me_label: &str) -> String {
    match (optional_config_string(user), me) {
        (Some(user), false) => format!("user {user}"),
        (None, true) => format!("identity {me_label}"),
        (Some(user), true) => format!("user {user} and {me_label}"),
        (None, false) => "identity -".to_string(),
    }
}

fn config_jira_auth_user_label(source: &ConfigJiraSource) -> String {
    if let Some(auth_user) = optional_config_string(source.auth_user.as_deref()) {
        return format!("user {auth_user}");
    }
    if let Some(env_var) = optional_config_string(source.auth_user_env.as_deref()) {
        return format!("env {env_var}");
    }
    "defaults to assignee".to_string()
}

fn config_cache_label(cache_dir: Option<&Path>) -> String {
    cache_dir
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "disabled".to_string())
}

fn doctor_defaults(report: &mut DoctorReport, config: &ShiplogConfig, base_dir: &Path) {
    match resolve_multi_window(DateArgs::default(), config) {
        Ok(window) => report.ok("Window", window.window_label()),
        Err(err) => report.error("Window", err.to_string()),
    }

    let out = config
        .defaults
        .out
        .as_ref()
        .filter(|path| !path.as_os_str().is_empty())
        .map(|path| resolve_config_path(base_dir, path))
        .unwrap_or_else(|| resolve_config_path(base_dir, Path::new("./out")));
    match doctor_check_output_writable(&out) {
        Ok(()) => report.ok("Output", format!("{} writable", out.display())),
        Err(err) => report.error("Output", err.to_string()),
    }

    let profile = match doctor_config_profile(config.defaults.profile.as_deref()) {
        Ok(profile) => {
            report.ok("Profile", profile.as_str());
            profile
        }
        Err(err) => {
            report.error("Profile", err.to_string());
            BundleProfile::Internal
        }
    };
    let key_env = optional_config_string(config.redaction.key_env.as_deref())
        .unwrap_or_else(|| "SHIPLOG_REDACT_KEY".to_string());
    if matches!(profile, BundleProfile::Internal) {
        report.ok("Redaction", "internal profile, no key required");
    } else if env_var_present(&key_env) {
        report.ok(
            "Redaction",
            format!("{} profile, {key_env} present", profile.as_str()),
        );
    } else {
        report.error(
            "Redaction",
            format!(
                "{} profile requires {key_env}; set it or use profile = \"internal\"",
                profile.as_str()
            ),
        );
    }
}

fn doctor_config_profile(profile: Option<&str>) -> Result<BundleProfile> {
    let profile = non_empty_string(profile).unwrap_or_else(|| "internal".to_string());
    profile
        .parse::<BundleProfile>()
        .map_err(anyhow::Error::msg)
        .with_context(|| format!("parse defaults.profile {profile:?}"))
}

fn doctor_check_output_writable(out: &Path) -> Result<()> {
    let probe_dir = if out.exists() {
        if !out.is_dir() {
            anyhow::bail!("{} exists but is not a directory", out.display());
        }
        out.to_path_buf()
    } else {
        out.parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    };

    if !probe_dir.exists() {
        anyhow::bail!(
            "{} does not exist; create it or choose a different defaults.out",
            probe_dir.display()
        );
    }

    let probe = probe_dir.join(format!(".shiplog-doctor-write-test-{}", std::process::id()));
    std::fs::write(&probe, b"").with_context(|| format!("write test file {}", probe.display()))?;
    std::fs::remove_file(&probe)
        .with_context(|| format!("remove test file {}", probe.display()))?;
    Ok(())
}

fn doctor_sources(
    report: &mut DoctorReport,
    config: &ShiplogConfig,
    base_dir: &Path,
    selected_sources: &[InitSource],
) {
    doctor_github(report, config, selected_sources);
    doctor_gitlab(report, config, selected_sources);
    doctor_jira(report, config, selected_sources);
    doctor_linear(report, config, selected_sources);
    doctor_git(report, config, base_dir, selected_sources);
    doctor_json(report, config, base_dir, selected_sources);
    doctor_manual(report, config, base_dir, selected_sources);
}

fn doctor_should_check(selected_sources: &[InitSource], source: InitSource) -> bool {
    selected_sources.is_empty() || selected_sources.contains(&source)
}

fn doctor_github(report: &mut DoctorReport, config: &ShiplogConfig, selected: &[InitSource]) {
    if !doctor_should_check(selected, InitSource::Github) {
        return;
    }

    let Some(source) = config.sources.github.as_ref() else {
        report.disabled("GitHub");
        return;
    };
    if !source.enabled {
        report.disabled("GitHub");
        return;
    }

    let api_base = optional_config_string(source.api_base.as_deref())
        .unwrap_or_else(|| "https://api.github.com".to_string());
    let token_present = env_var_present("GITHUB_TOKEN");
    match (
        optional_config_string(source.user.as_deref()),
        source.me,
        token_present,
    ) {
        (Some(_), true, _) => report.error("GitHub", "use either user or me, not both"),
        (Some(user), false, true) => report.ok("GitHub", format!("token found, user {user}")),
        (Some(user), false, false) => report.error(
            "GitHub",
            format!("missing GITHUB_TOKEN for configured user {user}"),
        ),
        (None, true, true) => match discover_github_user(&api_base, None) {
            Ok(user) => report.ok("GitHub", format!("token found, user inferred as {user}")),
            Err(err) => report.error("GitHub", err.to_string()),
        },
        (None, true, false) => {
            report.error("GitHub", "missing GITHUB_TOKEN for me identity discovery");
        }
        (None, false, _) => report.error("GitHub", "set sources.github.user or me = true"),
    }
}

fn doctor_gitlab(report: &mut DoctorReport, config: &ShiplogConfig, selected: &[InitSource]) {
    if !doctor_should_check(selected, InitSource::Gitlab) {
        return;
    }

    let Some(source) = config.sources.gitlab.as_ref() else {
        report.disabled("GitLab");
        return;
    };
    if !source.enabled {
        report.disabled("GitLab");
        return;
    }

    let instance = optional_config_string(source.instance.as_deref())
        .unwrap_or_else(|| "gitlab.com".to_string());
    if let Err(err) = gitlab_api_base(&instance) {
        report.error("GitLab", err.to_string());
        return;
    }
    if let Some(state) = non_empty_string(source.state.as_deref())
        && let Err(err) = state.parse::<MrState>()
    {
        report.error("GitLab", format!("parse state {state:?}: {err}"));
        return;
    }

    let token_present = env_var_present("GITLAB_TOKEN");
    match (
        optional_config_string(source.user.as_deref()),
        source.me,
        token_present,
    ) {
        (Some(_), true, _) => report.error("GitLab", "use either user or me, not both"),
        (Some(user), false, true) => {
            report.ok(
                "GitLab",
                format!("token found, user {user}, instance {instance}"),
            );
        }
        (Some(user), false, false) => report.error(
            "GitLab",
            format!("missing GITLAB_TOKEN for configured user {user}"),
        ),
        (None, true, true) => match discover_gitlab_user(&instance, None) {
            Ok(user) => report.ok("GitLab", format!("token found, user inferred as {user}")),
            Err(err) => report.error("GitLab", err.to_string()),
        },
        (None, true, false) => {
            report.error("GitLab", "missing GITLAB_TOKEN for me identity discovery");
        }
        (None, false, _) => report.error("GitLab", "set sources.gitlab.user or me = true"),
    }
}

fn doctor_jira(report: &mut DoctorReport, config: &ShiplogConfig, selected: &[InitSource]) {
    if !doctor_should_check(selected, InitSource::Jira) {
        return;
    }

    let Some(source) = config.sources.jira.as_ref() else {
        report.disabled("Jira");
        return;
    };
    if !source.enabled {
        report.disabled("Jira");
        return;
    }

    let start_errors = report.errors;
    let user = required_config_string("jira", "user", source.user.as_deref());
    let instance = required_config_string("jira", "instance", source.instance.as_deref());
    let status = source.status.as_deref().unwrap_or("done");
    let mut details = Vec::new();

    match user {
        Ok(user) => details.push(format!("assignee {user}")),
        Err(err) => report.error("Jira", err.to_string()),
    }
    match instance {
        Ok(instance) => details.push(format!("instance {instance}")),
        Err(err) => report.error("Jira", err.to_string()),
    }
    if let Err(err) = status.parse::<IssueStatus>() {
        report.error("Jira", format!("parse status {status:?}: {err}"));
    }
    if env_var_present("JIRA_TOKEN") {
        details.push("JIRA_TOKEN present".to_string());
    } else {
        report.error("Jira", "missing JIRA_TOKEN");
    }
    if let Some(auth_user) = optional_config_string(source.auth_user.as_deref()) {
        details.push(format!("auth user {auth_user}"));
    } else if let Some(env_var) = optional_config_string(source.auth_user_env.as_deref()) {
        if env_var_present(&env_var) {
            details.push(format!("auth user from {env_var}"));
        } else {
            details.push(format!("auth user defaults to assignee; {env_var} not set"));
        }
    } else {
        details.push("auth user defaults to assignee".to_string());
    }

    if report.errors == start_errors {
        report.ok("Jira", details.join(", "));
    }
}

fn doctor_linear(report: &mut DoctorReport, config: &ShiplogConfig, selected: &[InitSource]) {
    if !doctor_should_check(selected, InitSource::Linear) {
        return;
    }

    let Some(source) = config.sources.linear.as_ref() else {
        report.disabled("Linear");
        return;
    };
    if !source.enabled {
        report.disabled("Linear");
        return;
    }

    let start_errors = report.errors;
    let mut details = Vec::new();
    match required_config_string("linear", "user_id", source.user_id.as_deref()) {
        Ok(user_id) => details.push(format!("user_id {user_id}")),
        Err(err) => report.error("Linear", err.to_string()),
    }
    let status = source.status.as_deref().unwrap_or("done");
    if let Err(err) = status.parse::<LinearIssueStatus>() {
        report.error("Linear", format!("parse status {status:?}: {err}"));
    }
    if env_var_present("LINEAR_API_KEY") {
        details.push("LINEAR_API_KEY present".to_string());
    } else {
        report.error("Linear", "missing LINEAR_API_KEY");
    }

    if report.errors == start_errors {
        report.ok("Linear", details.join(", "));
    }
}

fn doctor_git(
    report: &mut DoctorReport,
    config: &ShiplogConfig,
    base_dir: &Path,
    selected: &[InitSource],
) {
    if !doctor_should_check(selected, InitSource::Git) {
        return;
    }

    let Some(source) = config.sources.git.as_ref() else {
        report.disabled("Git");
        return;
    };
    if !source.enabled {
        report.disabled("Git");
        return;
    }

    match required_config_path(base_dir, "git", "repo", source.repo.as_ref()) {
        Ok(repo) if repo.is_dir() => report.ok("Git", format!("repo {}", repo.display())),
        Ok(repo) => report.error("Git", format!("{} is not a directory", repo.display())),
        Err(err) => report.error("Git", err.to_string()),
    }
}

fn doctor_json(
    report: &mut DoctorReport,
    config: &ShiplogConfig,
    base_dir: &Path,
    selected: &[InitSource],
) {
    if !doctor_should_check(selected, InitSource::Json) {
        return;
    }

    let Some(source) = config.sources.json.as_ref() else {
        report.disabled("JSON");
        return;
    };
    if !source.enabled {
        report.disabled("JSON");
        return;
    }

    let events = required_config_path(base_dir, "json", "events", source.events.as_ref());
    let coverage = required_config_path(base_dir, "json", "coverage", source.coverage.as_ref());
    match (events, coverage) {
        (Ok(events), Ok(coverage)) if events.exists() && coverage.exists() => report.ok(
            "JSON",
            format!(
                "events {}, coverage {}",
                events.display(),
                coverage.display()
            ),
        ),
        (Ok(events), Ok(_)) if !events.exists() => {
            report.error("JSON", format!("{} not found", events.display()));
        }
        (Ok(_), Ok(coverage)) if !coverage.exists() => {
            report.error("JSON", format!("{} not found", coverage.display()));
        }
        (Err(err), _) | (_, Err(err)) => report.error("JSON", err.to_string()),
        _ => {}
    }
}

fn doctor_manual(
    report: &mut DoctorReport,
    config: &ShiplogConfig,
    base_dir: &Path,
    selected: &[InitSource],
) {
    if !doctor_should_check(selected, InitSource::Manual) {
        return;
    }

    let Some(source) = config.sources.manual.as_ref() else {
        report.disabled("Manual");
        return;
    };
    if !source.enabled {
        report.disabled("Manual");
        return;
    }

    match required_config_path(base_dir, "manual", "events", source.events.as_ref()) {
        Ok(events) if events.exists() => {
            report.ok("Manual", format!("{} found", events.display()));
        }
        Ok(events) => report.error("Manual", format!("{} not found", events.display())),
        Err(err) => report.error("Manual", err.to_string()),
    }
}

fn env_var_present(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .is_some_and(|value| !value.trim().is_empty())
}

fn resolve_multi_window(args: DateArgs, config: &ShiplogConfig) -> Result<ResolvedWindow> {
    if date_args_has_any(&args) {
        return resolve_date_window(args);
    }

    if let Some(window) = non_empty_string(config.defaults.window.as_deref()) {
        return resolve_date_window(date_args_from_config_window(&window)?);
    }

    resolve_date_window(DateArgs::default())
}

fn date_args_has_any(args: &DateArgs) -> bool {
    args.since.is_some()
        || args.until.is_some()
        || args.last_6_months
        || args.last_quarter
        || args.year.is_some()
}

fn date_args_from_config_window(window: &str) -> Result<DateArgs> {
    let value = window.trim();
    if value.eq_ignore_ascii_case("last-6-months") {
        return Ok(DateArgs {
            last_6_months: true,
            ..DateArgs::default()
        });
    }
    if value.eq_ignore_ascii_case("last-quarter") {
        return Ok(DateArgs {
            last_quarter: true,
            ..DateArgs::default()
        });
    }

    let year = value
        .strip_prefix("year:")
        .or_else(|| value.strip_prefix("year="))
        .unwrap_or(value);
    if year.len() == 4 && year.chars().all(|c| c.is_ascii_digit()) {
        let year = year
            .parse::<i32>()
            .with_context(|| format!("parse configured window year {year:?}"))?;
        return Ok(DateArgs {
            year: Some(year),
            ..DateArgs::default()
        });
    }

    anyhow::bail!(
        "unsupported configured window {window:?}; use last-6-months, last-quarter, or year:YYYY"
    )
}

fn non_empty_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn required_config_string(source: &str, field: &str, value: Option<&str>) -> Result<String> {
    non_empty_string(value)
        .ok_or_else(|| anyhow::anyhow!("{source} is enabled but sources.{source}.{field} is empty"))
}

fn optional_config_string(value: Option<&str>) -> Option<String> {
    non_empty_string(value)
}

fn required_config_path(
    base_dir: &Path,
    source: &str,
    field: &str,
    value: Option<&PathBuf>,
) -> Result<PathBuf> {
    let path = value
        .filter(|path| !path.as_os_str().is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!("{source} is enabled but sources.{source}.{field} is empty")
        })?;
    Ok(resolve_config_path(base_dir, path))
}

fn optional_config_path(base_dir: &Path, value: Option<&PathBuf>) -> Option<PathBuf> {
    value
        .filter(|path| !path.as_os_str().is_empty())
        .map(|path| resolve_config_path(base_dir, path))
}

fn resolve_config_path(base_dir: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}

fn config_base_dir(config_path: &Path) -> PathBuf {
    config_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf()
}

fn resolve_config_cache_dir(
    base_dir: &Path,
    out_root: &Path,
    explicit_cache_dir: Option<&PathBuf>,
    no_cache: bool,
) -> Option<PathBuf> {
    if no_cache {
        None
    } else {
        optional_config_path(base_dir, explicit_cache_dir).or_else(|| Some(out_root.join(".cache")))
    }
}

fn push_configured_source_result(
    successes: &mut Vec<(String, IngestOutput)>,
    failures: &mut Vec<ConfiguredSourceFailure>,
    name: &str,
    result: Result<IngestOutput>,
) {
    match result {
        Ok(ingest) => successes.push((name.to_string(), ingest)),
        Err(err) => failures.push(ConfiguredSourceFailure {
            name: name.to_string(),
            error: err.to_string(),
        }),
    }
}

fn collect_configured_sources(
    config_path: &Path,
    config: &ShiplogConfig,
    window: ResolvedWindow,
    out_root: &Path,
) -> Result<ConfiguredSourceOutputs> {
    let base_dir = config_base_dir(config_path);
    let default_include_reviews = config.defaults.include_reviews.unwrap_or(false);
    let mut successes = Vec::new();
    let mut failures = Vec::new();

    if let Some(source) = config
        .sources
        .github
        .as_ref()
        .filter(|source| source.enabled)
    {
        let api_base = optional_config_string(source.api_base.as_deref())
            .unwrap_or_else(|| "https://api.github.com".to_string());
        let user = resolve_user_or_me(
            "GitHub",
            optional_config_string(source.user.as_deref()),
            source.me,
            || discover_github_user(&api_base, None),
        )?;
        let cache_dir = resolve_config_cache_dir(
            &base_dir,
            out_root,
            source.cache_dir.as_ref(),
            source.no_cache,
        );
        let ing = make_github_ingestor(
            &user,
            window.since,
            window.until,
            source.mode.as_deref().unwrap_or("merged"),
            source.include_reviews.unwrap_or(default_include_reviews),
            source.no_details,
            source.throttle_ms,
            None,
            &api_base,
            cache_dir,
        )
        .context("create configured GitHub ingestor")
        .and_then(|ing| ing.ingest().context("collect configured GitHub source"));
        push_configured_source_result(&mut successes, &mut failures, "github", ing);
    }

    if let Some(source) = config
        .sources
        .gitlab
        .as_ref()
        .filter(|source| source.enabled)
    {
        let instance = optional_config_string(source.instance.as_deref())
            .unwrap_or_else(|| "gitlab.com".to_string());
        let user = resolve_user_or_me(
            "GitLab",
            optional_config_string(source.user.as_deref()),
            source.me,
            || discover_gitlab_user(&instance, None),
        )?;
        let cache_dir = resolve_config_cache_dir(
            &base_dir,
            out_root,
            source.cache_dir.as_ref(),
            source.no_cache,
        );
        let ing = make_gitlab_ingestor(
            &user,
            window.since,
            window.until,
            source.state.as_deref().unwrap_or("merged"),
            &instance,
            source.include_reviews.unwrap_or(default_include_reviews),
            source.throttle_ms,
            None,
            cache_dir,
        )
        .context("create configured GitLab ingestor")
        .and_then(|ing| ing.ingest().context("collect configured GitLab source"));
        push_configured_source_result(&mut successes, &mut failures, "gitlab", ing);
    }

    if let Some(source) = config.sources.jira.as_ref().filter(|source| source.enabled) {
        let user = required_config_string("jira", "user", source.user.as_deref())?;
        let instance = required_config_string("jira", "instance", source.instance.as_deref())?;
        let auth_user = optional_config_string(source.auth_user.as_deref()).or_else(|| {
            source
                .auth_user_env
                .as_deref()
                .and_then(|env_var| non_empty_string(Some(env_var)))
                .and_then(|env_var| std::env::var(env_var).ok())
        });
        let cache_dir = resolve_config_cache_dir(
            &base_dir,
            out_root,
            source.cache_dir.as_ref(),
            source.no_cache,
        );
        let ing = make_jira_ingestor(
            &user,
            auth_user,
            window.since,
            window.until,
            source.status.as_deref().unwrap_or("done"),
            &instance,
            source.throttle_ms,
            None,
            cache_dir,
        )
        .context("create configured Jira ingestor")
        .and_then(|ing| ing.ingest().context("collect configured Jira source"));
        push_configured_source_result(&mut successes, &mut failures, "jira", ing);
    }

    if let Some(source) = config
        .sources
        .linear
        .as_ref()
        .filter(|source| source.enabled)
    {
        let user_id = required_config_string("linear", "user_id", source.user_id.as_deref())?;
        let project = optional_config_string(source.project.as_deref());
        let cache_dir = resolve_config_cache_dir(
            &base_dir,
            out_root,
            source.cache_dir.as_ref(),
            source.no_cache,
        );
        let ing = make_linear_ingestor(
            &user_id,
            window.since,
            window.until,
            source.status.as_deref().unwrap_or("done"),
            project,
            source.throttle_ms,
            None,
            cache_dir,
        )
        .context("create configured Linear ingestor")
        .and_then(|ing| ing.ingest().context("collect configured Linear source"));
        push_configured_source_result(&mut successes, &mut failures, "linear", ing);
    }

    if let Some(source) = config.sources.git.as_ref().filter(|source| source.enabled) {
        let repo = required_config_path(&base_dir, "git", "repo", source.repo.as_ref())?;
        let result = make_git_ingestor(
            &repo,
            window.since,
            window.until,
            optional_config_string(source.author.as_deref()),
            source.include_merges,
        )
        .ingest()
        .context("collect configured git source");
        push_configured_source_result(&mut successes, &mut failures, "git", result);
    }

    if let Some(source) = config.sources.json.as_ref().filter(|source| source.enabled) {
        let events = required_config_path(&base_dir, "json", "events", source.events.as_ref())?;
        let coverage =
            required_config_path(&base_dir, "json", "coverage", source.coverage.as_ref())?;
        let ing = JsonIngestor {
            events_path: events,
            coverage_path: coverage,
        };
        let result = ing.ingest().context("collect configured JSON source");
        push_configured_source_result(&mut successes, &mut failures, "json", result);
    }

    if let Some(source) = config
        .sources
        .manual
        .as_ref()
        .filter(|source| source.enabled)
    {
        let events = required_config_path(&base_dir, "manual", "events", source.events.as_ref())?;
        let user = optional_config_string(source.user.as_deref())
            .or_else(|| optional_config_string(config.user.label.as_deref()))
            .unwrap_or_else(|| "user".to_string());
        let ing = ManualIngestor::new(&events, user, window.since, window.until);
        let result = ing.ingest().context("collect configured manual source");
        push_configured_source_result(&mut successes, &mut failures, "manual", result);
    }

    if successes.is_empty() && failures.is_empty() {
        anyhow::bail!(
            "No enabled sources found in {}; enable at least one [sources.<name>] section",
            config_path.display()
        );
    }

    if successes.is_empty() {
        let failed = failures
            .iter()
            .map(|failure| format!("{}: {}", failure.name, failure.error))
            .collect::<Vec<_>>()
            .join("; ");
        anyhow::bail!("No configured sources collected successfully: {failed}");
    }

    Ok(ConfiguredSourceOutputs {
        successes,
        failures,
    })
}

#[allow(clippy::too_many_arguments)]
fn run_configured_multi_pipeline(
    config: &ShiplogConfig,
    out: &Path,
    window: ResolvedWindow,
    bundle_profile: &BundleProfile,
    conflict: MergeConflict,
    configured: ConfiguredSourceOutputs,
    regen: bool,
    zip: bool,
    engine: &Engine<'_>,
    redactor: &DeterministicRedactor,
) -> Result<ConfiguredRunResult> {
    let ingest_outputs = configured
        .successes
        .iter()
        .map(|(_, ingest)| ingest.clone())
        .collect::<Vec<_>>();

    let mut merged = engine
        .merge(ingest_outputs, conflict.into())
        .context("merge configured source outputs")?;
    let merge_user = config_user_label(config).unwrap_or_else(|| merged.coverage.user.clone());
    let window_label = window.window_label();
    merged.coverage.user = merge_user.clone();
    merged.coverage.window = TimeWindow {
        since: window.since,
        until: window.until,
    };
    if !configured.failures.is_empty() {
        for failure in &configured.failures {
            if !merged.coverage.sources.contains(&failure.name) {
                merged.coverage.sources.push(failure.name.clone());
            }
            merged.coverage.warnings.push(format!(
                "Configured source {} was skipped: {}",
                failure.name, failure.error
            ));
        }
        merged.coverage.sources.sort();
        merged.coverage.sources.dedup();
        merged.coverage.completeness = shiplog_schema::coverage::Completeness::Partial;
    }

    let run_id = merged.coverage.run_id.to_string();
    let run_dir = out.join(&run_id);

    if regen {
        let suggested = shiplog_workstreams::WorkstreamManager::suggested_path(&run_dir);
        if suggested.exists() {
            std::fs::remove_file(&suggested)
                .with_context(|| format!("remove {:?} for --regen", suggested))?;
        }
    }

    let cache_path = DeterministicRedactor::cache_path(&run_dir);
    let _ = redactor.load_cache(&cache_path);

    let (outputs, ws_source) = engine
        .run(
            merged,
            &merge_user,
            &window_label,
            &run_dir,
            zip,
            bundle_profile,
        )
        .context("run configured multi-source pipeline")?;

    redactor
        .save_cache(&cache_path)
        .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

    Ok(ConfiguredRunResult {
        configured,
        outputs,
        ws_source,
        run_id,
    })
}

fn config_user_label(config: &ShiplogConfig) -> Option<String> {
    optional_config_string(config.user.label.as_deref())
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

#[derive(Deserialize)]
struct JiraAuthenticatedUser {
    #[serde(rename = "accountId")]
    account_id: String,
    #[serde(rename = "displayName")]
    display_name: Option<String>,
    #[serde(rename = "emailAddress")]
    email_address: Option<String>,
    active: Option<bool>,
}

#[derive(Serialize)]
struct LinearGraphqlRequest<'a> {
    query: &'a str,
}

#[derive(Deserialize)]
struct LinearGraphqlResponse<T> {
    data: Option<T>,
    errors: Option<Vec<LinearGraphqlError>>,
}

#[derive(Deserialize)]
struct LinearGraphqlError {
    message: Option<String>,
}

#[derive(Deserialize)]
struct LinearViewerData {
    viewer: LinearViewer,
}

#[derive(Deserialize)]
struct LinearViewer {
    id: String,
    name: Option<String>,
    email: Option<String>,
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

fn run_identify_jira(
    instance: String,
    auth_user: Option<String>,
    token: Option<String>,
) -> Result<()> {
    let auth_user = arg_or_env(
        "Jira identity lookup",
        "--auth-user",
        auth_user.as_deref(),
        "JIRA_AUTH_USER",
    )?;
    let token = arg_or_env(
        "Jira identity lookup",
        "--token",
        token.as_deref(),
        "JIRA_TOKEN",
    )?;

    let identity = identify_jira_user(&instance, &auth_user, &token)?;
    println!("Jira identity:");
    println!("- account_id: {}", identity.account_id);
    println!(
        "- display_name: {}",
        optional_output(identity.display_name.as_deref())
    );
    println!(
        "- email: {}",
        optional_output(identity.email_address.as_deref())
    );
    if let Some(active) = identity.active {
        println!("- active: {active}");
    }
    println!();
    println!("Use for collection:");
    println!(
        "  shiplog collect jira --instance {} --user {} --auth-user {}",
        normalize_jira_instance(&instance)?,
        identity.account_id,
        auth_user
    );
    Ok(())
}

fn run_identify_linear(api_key: Option<String>) -> Result<()> {
    let api_key = arg_or_env(
        "Linear identity lookup",
        "--api-key",
        api_key.as_deref(),
        "LINEAR_API_KEY",
    )?;

    let viewer = identify_linear_user(&api_key)?;
    println!("Linear identity:");
    println!("- user_id: {}", viewer.id);
    println!("- name: {}", optional_output(viewer.name.as_deref()));
    println!("- email: {}", optional_output(viewer.email.as_deref()));
    println!();
    println!("Use for collection:");
    println!("  shiplog collect linear --user-id {}", viewer.id);
    Ok(())
}

fn identify_jira_user(
    instance: &str,
    auth_user: &str,
    token: &str,
) -> Result<JiraAuthenticatedUser> {
    let client = identity_client()?;
    let url = jira_api_url(instance, "/myself")?;
    let resp = client
        .get(&url)
        .header("Accept", "application/json")
        .basic_auth(auth_user, Some(token))
        .send()
        .with_context(|| format!("GET {url}"))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().unwrap_or_default();
        if status.as_u16() == 401 {
            anyhow::bail!("Could not identify Jira user: Jira authentication failed");
        }
        anyhow::bail!("Could not identify Jira user: Jira API error {status}: {body}");
    }

    let user = resp
        .json::<JiraAuthenticatedUser>()
        .with_context(|| format!("parse Jira authenticated user from {url}"))?;
    if user.account_id.trim().is_empty() {
        anyhow::bail!("Could not identify Jira user: response had empty accountId");
    }
    Ok(user)
}

fn identify_linear_user(api_key: &str) -> Result<LinearViewer> {
    let client = identity_client()?;
    let query = "query Me { viewer { id name email } }";
    let resp = client
        .post(linear_graphql_url())
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .header("Authorization", api_key)
        .json(&LinearGraphqlRequest { query })
        .send()
        .context("POST Linear viewer query")?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().unwrap_or_default();
        if status.as_u16() == 401 {
            anyhow::bail!("Could not identify Linear user: Linear authentication failed");
        }
        anyhow::bail!("Could not identify Linear user: Linear API error {status}: {body}");
    }

    let response = resp
        .json::<LinearGraphqlResponse<LinearViewerData>>()
        .context("parse Linear viewer response")?;
    if let Some(errors) = response.errors {
        anyhow::bail!(
            "Could not identify Linear user: Linear GraphQL errors: {}",
            errors
                .iter()
                .map(|error| error.message.as_deref().unwrap_or("unknown error"))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    let viewer = response
        .data
        .ok_or_else(|| anyhow::anyhow!("Could not identify Linear user: response missing data"))?
        .viewer;
    if viewer.id.trim().is_empty() {
        anyhow::bail!("Could not identify Linear user: response had empty viewer id");
    }
    Ok(viewer)
}

fn arg_or_env(
    command: &str,
    arg_name: &str,
    explicit: Option<&str>,
    env_name: &str,
) -> Result<String> {
    optional_config_string(explicit)
        .or_else(|| {
            std::env::var(env_name)
                .ok()
                .and_then(|value| non_empty_string(Some(&value)))
        })
        .ok_or_else(|| anyhow::anyhow!("{command} requires {arg_name} or {env_name}"))
}

fn optional_output(value: Option<&str>) -> &str {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("-")
}

fn jira_api_url(instance: &str, path: &str) -> Result<String> {
    Ok(format!(
        "https://{}/rest/api/3{}",
        normalize_jira_instance(instance)?,
        path
    ))
}

fn normalize_jira_instance(instance: &str) -> Result<String> {
    if instance.trim().is_empty() {
        anyhow::bail!("Jira instance cannot be empty");
    }

    if instance.contains("://") {
        return reqwest::Url::parse(instance)
            .ok()
            .and_then(|url| url.host_str().map(ToOwned::to_owned))
            .ok_or_else(|| anyhow::anyhow!("Invalid Jira instance URL: {instance}"));
    }

    Ok(instance.trim().trim_end_matches('/').to_string())
}

fn linear_graphql_url() -> &'static str {
    "https://api.linear.app/graphql"
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
    bundle_profile: &BundleProfile,
) -> (Engine<'static>, &'static DeterministicRedactor) {
    create_engine_with_renderer(
        redact_key,
        clusterer,
        Box::new(ModeMarkdownRenderer::new(
            RenderPacketMode::Packet,
            cli_render_options(RenderPacketMode::Packet, None, None, bundle_profile),
        )),
    )
}

fn create_engine_with_renderer(
    redact_key: &str,
    clusterer: Box<dyn shiplog_ports::WorkstreamClusterer>,
    renderer: Box<dyn Renderer>,
) -> (Engine<'static>, &'static DeterministicRedactor) {
    let redactor = DeterministicRedactor::new(redact_key.as_bytes());

    // We need to leak these to give them 'static lifetime
    // This is acceptable for a CLI tool that runs once
    let renderer: &'static dyn Renderer = Box::leak(renderer);
    let clusterer: &'static dyn shiplog_ports::WorkstreamClusterer = Box::leak(clusterer);
    let redactor_box = Box::new(redactor);
    let redactor_ref: &'static DeterministicRedactor = Box::leak(redactor_box);
    let redactor_trait: &'static dyn shiplog_ports::Redactor = redactor_ref;

    (
        Engine::new(renderer, clusterer, redactor_trait),
        redactor_ref,
    )
}

struct ModeMarkdownRenderer {
    mode: RenderPacketMode,
    options: MarkdownRenderOptions,
    inner: MarkdownRenderer,
}

impl ModeMarkdownRenderer {
    fn new(mode: RenderPacketMode, options: MarkdownRenderOptions) -> Self {
        Self {
            mode,
            options,
            inner: cli_packet_renderer(),
        }
    }
}

fn cli_packet_renderer() -> MarkdownRenderer {
    MarkdownRenderer::new().with_section_order(SectionOrder::CoverageFirst)
}

const MANAGER_RECEIPT_RENDER_LIMIT: usize = 3;
const PUBLIC_RECEIPT_RENDER_LIMIT: usize = 1;

fn cli_render_options(
    mode: RenderPacketMode,
    receipt_limit: Option<usize>,
    appendix: Option<RenderAppendixMode>,
    bundle_profile: &BundleProfile,
) -> MarkdownRenderOptions {
    MarkdownRenderOptions {
        receipt_limit: receipt_limit
            .unwrap_or_else(|| default_receipt_limit_for_profile(mode, bundle_profile)),
        appendix_mode: appendix
            .unwrap_or_else(|| default_appendix_for_profile(mode, bundle_profile))
            .into(),
    }
}

fn default_receipt_limit_for_profile(
    mode: RenderPacketMode,
    bundle_profile: &BundleProfile,
) -> usize {
    match (mode, bundle_profile) {
        (RenderPacketMode::Packet, BundleProfile::Manager) => MANAGER_RECEIPT_RENDER_LIMIT,
        (RenderPacketMode::Packet, BundleProfile::Public) => PUBLIC_RECEIPT_RENDER_LIMIT,
        _ => WORKSTREAM_RECEIPT_RENDER_LIMIT,
    }
}

fn default_appendix_for_profile(
    mode: RenderPacketMode,
    bundle_profile: &BundleProfile,
) -> RenderAppendixMode {
    match mode {
        RenderPacketMode::Packet => match bundle_profile {
            BundleProfile::Internal | BundleProfile::Manager => RenderAppendixMode::Summary,
            BundleProfile::Public => RenderAppendixMode::None,
        },
        RenderPacketMode::Scaffold => RenderAppendixMode::None,
        RenderPacketMode::Receipts => RenderAppendixMode::Full,
    }
}

impl Renderer for ModeMarkdownRenderer {
    fn render_packet_markdown(
        &self,
        user: &str,
        window_label: &str,
        events: &[EventEnvelope],
        workstreams: &WorkstreamsFile,
        coverage: &shiplog_schema::coverage::CoverageManifest,
    ) -> Result<String> {
        match self.mode {
            RenderPacketMode::Packet => self.inner.render_packet_markdown_with_options(
                user,
                window_label,
                events,
                workstreams,
                coverage,
                self.options,
            ),
            RenderPacketMode::Scaffold => self.inner.render_scaffold_markdown_with_options(
                user,
                window_label,
                events,
                workstreams,
                coverage,
                self.options,
            ),
            RenderPacketMode::Receipts => self.inner.render_receipts_markdown_with_options(
                user,
                window_label,
                events,
                workstreams,
                coverage,
                self.options,
            ),
        }
    }
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

#[derive(Debug, Clone)]
struct CacheDbTarget {
    source: CacheSource,
    path: PathBuf,
}

#[derive(Debug)]
enum CacheCleanMode {
    Expired,
    OlderThan(DateTime<Utc>),
    All,
}

fn run_cache_stats(args: CacheArgs) -> Result<()> {
    let root = cache_command_root(&args.out, args.cache_dir.as_ref());
    println!("Cache root: {}", root.display());
    let targets = cache_db_targets(&root, &args.sources);
    let mut found = 0usize;
    for target in targets {
        if !target.path.exists() {
            println!(
                "{}: missing, {}",
                target.source.as_str(),
                target.path.display()
            );
            continue;
        }
        found += 1;
        let cache = ApiCache::open_read_only(&target.path)
            .with_context(|| format!("open cache {}", target.path.display()))?;
        let stats = cache
            .stats()
            .with_context(|| format!("read cache stats {}", target.path.display()))?;
        print_cache_stats(target.source, &target.path, &stats);
    }
    if found == 0 {
        println!("No cache databases found");
    }
    Ok(())
}

fn run_cache_inspect(args: CacheArgs) -> Result<()> {
    let root = cache_command_root(&args.out, args.cache_dir.as_ref());
    println!("Cache root: {}", root.display());
    let targets = cache_db_targets(&root, &args.sources);
    let mut found = 0usize;
    for target in targets {
        if !target.path.exists() {
            println!(
                "{}: missing, {}",
                target.source.as_str(),
                target.path.display()
            );
            continue;
        }
        found += 1;
        let cache = ApiCache::open_read_only(&target.path)
            .with_context(|| format!("open cache {}", target.path.display()))?;
        let inspection = cache
            .inspect()
            .with_context(|| format!("inspect cache {}", target.path.display()))?;
        print_cache_stats(target.source, &target.path, &inspection.stats);
        println!(
            "  oldest: {}",
            inspection.oldest_cached_at.as_deref().unwrap_or("-")
        );
        println!(
            "  newest: {}",
            inspection.newest_cached_at.as_deref().unwrap_or("-")
        );
    }
    if found == 0 {
        println!("No cache databases found");
    }
    Ok(())
}

fn run_cache_clean(args: CacheCleanArgs) -> Result<()> {
    let mode = cache_clean_mode(&args)?;
    if matches!(mode, CacheCleanMode::All) && !args.yes && !args.dry_run {
        anyhow::bail!("cache clean --all requires --yes");
    }

    let root = cache_command_root(&args.out, args.cache_dir.as_ref());
    println!("Cache root: {}", root.display());
    let targets = cache_db_targets(&root, &args.sources);
    let mut found = 0usize;
    for target in targets {
        if !target.path.exists() {
            println!(
                "{}: missing, {}",
                target.source.as_str(),
                target.path.display()
            );
            continue;
        }
        found += 1;
        let cache = ApiCache::open(&target.path)
            .with_context(|| format!("open cache {}", target.path.display()))?;
        let planned = cache_clean_count(&cache, &mode)?;
        if args.dry_run {
            println!(
                "{}: would remove {} entries from {}",
                target.source.as_str(),
                planned,
                target.path.display()
            );
            continue;
        }
        let removed = cache_clean_apply(&cache, &mode, planned)?;
        println!(
            "{}: removed {} entries from {}",
            target.source.as_str(),
            removed,
            target.path.display()
        );
    }
    if found == 0 {
        println!("No cache databases found");
    }
    Ok(())
}

fn cache_command_root(out: &Path, cache_dir: Option<&PathBuf>) -> PathBuf {
    cache_dir.cloned().unwrap_or_else(|| out.join(".cache"))
}

fn cache_db_targets(root: &Path, sources: &[CacheSource]) -> Vec<CacheDbTarget> {
    selected_cache_sources(sources)
        .into_iter()
        .map(|source| CacheDbTarget {
            source,
            path: root.join(source.db_filename()),
        })
        .collect()
}

fn selected_cache_sources(sources: &[CacheSource]) -> Vec<CacheSource> {
    if sources.is_empty() {
        return CacheSource::all().to_vec();
    }
    let mut selected = Vec::new();
    for source in sources {
        if !selected.contains(source) {
            selected.push(*source);
        }
    }
    selected
}

fn print_cache_stats(source: CacheSource, path: &Path, stats: &shiplog_cache::CacheStats) {
    println!("{}:", source.as_str());
    println!("  path: {}", path.display());
    println!(
        "  entries: total {}, valid {}, expired {}",
        stats.total_entries, stats.valid_entries, stats.expired_entries
    );
    println!("  size: {} MB", stats.cache_size_mb);
}

fn cache_clean_mode(args: &CacheCleanArgs) -> Result<CacheCleanMode> {
    if args.all && args.older_than.is_some() {
        anyhow::bail!("use either --all or --older-than, not both");
    }
    if args.all {
        return Ok(CacheCleanMode::All);
    }
    if let Some(age) = args.older_than.as_deref() {
        let duration = parse_cache_age(age)?;
        return Ok(CacheCleanMode::OlderThan(Utc::now() - duration));
    }
    Ok(CacheCleanMode::Expired)
}

fn parse_cache_age(value: &str) -> Result<Duration> {
    let value = value.trim();
    let Some(unit) = value.chars().last() else {
        anyhow::bail!("--older-than must use a duration like 30d, 12h, or 90m");
    };
    let amount = &value[..value.len() - unit.len_utf8()];
    let amount: i64 = amount
        .parse()
        .with_context(|| format!("parse --older-than duration {value:?}"))?;
    if amount < 0 {
        anyhow::bail!("--older-than must not be negative");
    }
    match unit {
        'd' => Ok(Duration::days(amount)),
        'h' => Ok(Duration::hours(amount)),
        'm' => Ok(Duration::minutes(amount)),
        _ => anyhow::bail!("--older-than must use d, h, or m, got {unit:?}"),
    }
}

fn cache_clean_count(cache: &ApiCache, mode: &CacheCleanMode) -> Result<usize> {
    match mode {
        CacheCleanMode::Expired => Ok(cache.stats()?.expired_entries),
        CacheCleanMode::OlderThan(cutoff) => cache.count_older_than(*cutoff),
        CacheCleanMode::All => Ok(cache.stats()?.total_entries),
    }
}

fn cache_clean_apply(cache: &ApiCache, mode: &CacheCleanMode, planned: usize) -> Result<usize> {
    match mode {
        CacheCleanMode::Expired => cache.cleanup_expired(),
        CacheCleanMode::OlderThan(cutoff) => cache.cleanup_older_than(*cutoff),
        CacheCleanMode::All => {
            cache.clear()?;
            Ok(planned)
        }
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

        Command::Doctor { config, sources } => {
            run_doctor(&config, &sources)?;
        }

        Command::Intake(args) => {
            run_intake(args)?;
        }

        Command::Config { cmd } => match cmd {
            ConfigCommand::Validate { config } => {
                run_config_validate(&config)?;
            }
            ConfigCommand::Explain { config } => {
                run_config_explain(&config)?;
            }
            ConfigCommand::Migrate { config, dry_run } => {
                run_config_migrate(&config, dry_run)?;
            }
        },

        Command::Cache { cmd } => match cmd {
            CacheCommand::Stats(args) => run_cache_stats(args)?,
            CacheCommand::Inspect(args) => run_cache_inspect(args)?,
            CacheCommand::Clean(args) => run_cache_clean(args)?,
        },

        Command::Identify { cmd } => match cmd {
            IdentifyCommand::Jira {
                instance,
                auth_user,
                token,
            } => run_identify_jira(instance, auth_user, token)?,
            IdentifyCommand::Linear { api_key } => run_identify_linear(api_key)?,
        },

        Command::Journal { cmd } => match cmd {
            JournalCommand::Add(args) => run_journal_add(args)?,
        },

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
            let source = match source {
                CollectSource::Multi {
                    config,
                    window,
                    conflict,
                } => {
                    let config_model = load_shiplog_config(&config)?;
                    ensure_supported_config_version(&config_model)?;
                    let base_dir = config_base_dir(&config);
                    let out = out
                        .clone()
                        .unwrap_or_else(|| config_default_out(&config_model, &base_dir));
                    let bundle_profile = bundle_profile.clone().map(Ok).unwrap_or_else(|| {
                        doctor_config_profile(config_model.defaults.profile.as_deref())
                    })?;
                    let redaction_key = RedactionKey::resolve_with_env(
                        redact_key.clone(),
                        &bundle_profile,
                        &config_redaction_key_env(&config_model),
                    )?;
                    let clusterer = build_clusterer(
                        llm_cluster,
                        &llm_api_endpoint,
                        &llm_model,
                        llm_api_key.clone(),
                    );
                    let (engine, redactor) =
                        create_engine(redaction_key.engine_key(), clusterer, &bundle_profile);
                    let engine = engine.with_profile_rendering(redaction_key.render_profiles());
                    let window = resolve_multi_window(window, &config_model)?;
                    let configured =
                        collect_configured_sources(&config, &config_model, window, &out)?;
                    let result = run_configured_multi_pipeline(
                        &config_model,
                        &out,
                        window,
                        &bundle_profile,
                        conflict,
                        configured,
                        regen,
                        zip,
                        &engine,
                        redactor,
                    )?;

                    println!("Collected configured sources:");
                    for (name, ingest) in &result.configured.successes {
                        println!(
                            "- {name}: success, {}",
                            event_count_phrase(ingest.events.len())
                        );
                    }
                    for failure in &result.configured.failures {
                        println!("- {}: skipped, {}", failure.name, failure.error);
                    }
                    println!("Merged and wrote:");
                    println!("- inputs: {}", result.configured.successes.len());
                    println!("- conflict: {}", conflict.as_str());
                    print_outputs(&result.outputs, result.ws_source);
                    return Ok(());
                }

                CollectSource::Source(source) => source,
            };
            let out = out.unwrap_or_else(|| PathBuf::from("./out"));
            let bundle_profile = bundle_profile.unwrap_or_default();
            let redaction_key = RedactionKey::resolve(redact_key, &bundle_profile)?;
            let clusterer =
                build_clusterer(llm_cluster, &llm_api_endpoint, &llm_model, llm_api_key);
            let (engine, redactor) =
                create_engine(redaction_key.engine_key(), clusterer, &bundle_profile);
            let engine = engine.with_profile_rendering(redaction_key.render_profiles());

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
            mode,
            receipt_limit,
            appendix,
            zip,
        } => {
            let redaction_key = RedactionKey::resolve(redact_key, &bundle_profile)?;
            let clusterer: Box<dyn shiplog_ports::WorkstreamClusterer> = Box::new(RepoClusterer);
            let renderer = Box::new(ModeMarkdownRenderer::new(
                mode,
                cli_render_options(mode, receipt_limit, appendix, &bundle_profile),
            ));
            let (engine, redactor) =
                create_engine_with_renderer(redaction_key.engine_key(), clusterer, renderer);
            let engine = engine.with_profile_rendering(redaction_key.render_profiles());

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
            let redaction_key = RedactionKey::resolve(redact_key, &bundle_profile)?;
            let clusterer: Box<dyn shiplog_ports::WorkstreamClusterer> = Box::new(RepoClusterer);
            let (engine, redactor) =
                create_engine(redaction_key.engine_key(), clusterer, &bundle_profile);
            let engine = engine.with_profile_rendering(redaction_key.render_profiles());

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
        Command::Workstreams { cmd } => match cmd {
            WorkstreamsCommand::List { out, run, latest } => {
                let run_dir = resolve_render_run_dir(&out, run, latest)?;
                let (workstreams, source, path) = load_effective_workstreams_for_run(&run_dir)?;
                print_workstreams_list(&run_dir, &path, source, &workstreams);
            }
            WorkstreamsCommand::Validate { out, run, latest } => {
                let run_dir = resolve_render_run_dir(&out, run, latest)?;
                let (workstreams, source, path) = load_effective_workstreams_for_run(&run_dir)?;
                let errors = validate_workstreams_for_run(&run_dir, &workstreams)?;
                if errors.is_empty() {
                    println!(
                        "Workstreams valid: {} ({})",
                        path.display(),
                        workstream_source_label(source)
                    );
                    println!("- {} workstreams", workstreams.workstreams.len());
                    println!(
                        "- {} assigned events",
                        workstreams
                            .workstreams
                            .iter()
                            .map(|workstream| workstream.events.len())
                            .sum::<usize>()
                    );
                    println!(
                        "- {} receipts",
                        workstreams
                            .workstreams
                            .iter()
                            .map(|workstream| workstream.receipts.len())
                            .sum::<usize>()
                    );
                } else {
                    for error in &errors {
                        eprintln!("- {error}");
                    }
                    anyhow::bail!("{} workstream validation error(s)", errors.len());
                }
            }
            WorkstreamsCommand::Rename {
                out,
                run,
                latest,
                from,
                to,
            } => {
                let run_dir = resolve_render_run_dir(&out, run, latest)?;
                let (mut workstreams, source, _) = load_effective_workstreams_for_run(&run_dir)?;
                let old_title = rename_workstream(&mut workstreams, &from, &to)?;
                write_curated_workstreams(&run_dir, &workstreams)?;
                println!("Renamed workstream: {old_title} -> {}", to.trim());
                println!(
                    "Updated: {}",
                    shiplog_workstreams::WorkstreamManager::curated_path(&run_dir).display()
                );
                if matches!(source, WorkstreamsFileSource::Suggested) {
                    println!("Created curated workstreams.yaml from suggested workstreams.");
                }
            }
            WorkstreamsCommand::Move {
                out,
                run,
                latest,
                event,
                to,
            } => {
                let run_dir = resolve_render_run_dir(&out, run, latest)?;
                let (mut workstreams, source, _) = load_effective_workstreams_for_run(&run_dir)?;
                let ledger_events = load_run_events(&run_dir)?;
                let result =
                    move_event_to_workstream(&mut workstreams, &event, &to, &ledger_events)?;
                let errors = validate_workstreams_against_events(&workstreams, &ledger_events);
                if !errors.is_empty() {
                    for error in &errors {
                        eprintln!("- {error}");
                    }
                    anyhow::bail!("{} workstream validation error(s)", errors.len());
                }

                write_curated_workstreams(&run_dir, &workstreams)?;
                println!("Moved event {} to {}", result.event_id, result.to_title);
                if result.from_titles.is_empty() {
                    println!("Source: unassigned");
                } else {
                    println!("Source: {}", result.from_titles.join(", "));
                }
                if result.receipt_preserved {
                    println!("Receipt anchor preserved in target workstream.");
                }
                println!(
                    "Updated: {}",
                    shiplog_workstreams::WorkstreamManager::curated_path(&run_dir).display()
                );
                if matches!(source, WorkstreamsFileSource::Suggested) {
                    println!("Created curated workstreams.yaml from suggested workstreams.");
                }
            }
            WorkstreamsCommand::Receipts {
                out,
                run,
                latest,
                workstream,
            } => {
                let run_dir = resolve_render_run_dir(&out, run, latest)?;
                let (workstreams, source, path) = load_effective_workstreams_for_run(&run_dir)?;
                let ledger_events = load_run_events(&run_dir)?;
                print_workstream_receipts(
                    &run_dir,
                    &path,
                    source,
                    &workstreams,
                    &ledger_events,
                    &workstream,
                )?;
            }
            WorkstreamsCommand::Receipt { cmd } => match cmd {
                WorkstreamReceiptCommand::Add {
                    out,
                    run,
                    latest,
                    workstream,
                    event,
                } => {
                    let run_dir = resolve_render_run_dir(&out, run, latest)?;
                    let (mut workstreams, source, _) =
                        load_effective_workstreams_for_run(&run_dir)?;
                    let ledger_events = load_run_events(&run_dir)?;
                    let result = add_workstream_receipt(
                        &mut workstreams,
                        &workstream,
                        &event,
                        &ledger_events,
                    )?;
                    let errors = validate_workstreams_against_events(&workstreams, &ledger_events);
                    if !errors.is_empty() {
                        for error in &errors {
                            eprintln!("- {error}");
                        }
                        anyhow::bail!("{} workstream validation error(s)", errors.len());
                    }

                    write_curated_workstreams(&run_dir, &workstreams)?;
                    println!(
                        "Added receipt anchor {} to {}",
                        result.event_id, result.workstream_title
                    );
                    println!("Receipt: {}", result.event_title);
                    println!(
                        "Updated: {}",
                        shiplog_workstreams::WorkstreamManager::curated_path(&run_dir).display()
                    );
                    if matches!(source, WorkstreamsFileSource::Suggested) {
                        println!("Created curated workstreams.yaml from suggested workstreams.");
                    }
                }
                WorkstreamReceiptCommand::Remove {
                    out,
                    run,
                    latest,
                    workstream,
                    event,
                } => {
                    let run_dir = resolve_render_run_dir(&out, run, latest)?;
                    let (mut workstreams, source, _) =
                        load_effective_workstreams_for_run(&run_dir)?;
                    let ledger_events = load_run_events(&run_dir)?;
                    let result = remove_workstream_receipt(
                        &mut workstreams,
                        &workstream,
                        &event,
                        &ledger_events,
                    )?;
                    let errors = validate_workstreams_against_events(&workstreams, &ledger_events);
                    if !errors.is_empty() {
                        for error in &errors {
                            eprintln!("- {error}");
                        }
                        anyhow::bail!("{} workstream validation error(s)", errors.len());
                    }

                    write_curated_workstreams(&run_dir, &workstreams)?;
                    println!(
                        "Removed receipt anchor {} from {}",
                        result.event_id, result.workstream_title
                    );
                    println!("Receipt: {}", result.event_title);
                    println!(
                        "Updated: {}",
                        shiplog_workstreams::WorkstreamManager::curated_path(&run_dir).display()
                    );
                    if matches!(source, WorkstreamsFileSource::Suggested) {
                        println!("Created curated workstreams.yaml from suggested workstreams.");
                    }
                }
            },
            WorkstreamsCommand::Create {
                out,
                run,
                latest,
                title,
            } => {
                let run_dir = resolve_render_run_dir(&out, run, latest)?;
                let (mut workstreams, source, _) = load_effective_workstreams_for_run(&run_dir)?;
                let result = create_workstream(&mut workstreams, &title)?;
                let ledger_events = load_run_events(&run_dir)?;
                let errors = validate_workstreams_against_events(&workstreams, &ledger_events);
                if !errors.is_empty() {
                    for error in &errors {
                        eprintln!("- {error}");
                    }
                    anyhow::bail!("{} workstream validation error(s)", errors.len());
                }

                write_curated_workstreams(&run_dir, &workstreams)?;
                println!("Created workstream: {}", result.title);
                println!("ID: {}", result.id);
                println!(
                    "Updated: {}",
                    shiplog_workstreams::WorkstreamManager::curated_path(&run_dir).display()
                );
                if matches!(source, WorkstreamsFileSource::Suggested) {
                    println!("Created curated workstreams.yaml from suggested workstreams.");
                }
            }
            WorkstreamsCommand::Delete {
                out,
                run,
                latest,
                workstream,
                move_to,
                force,
            } => {
                let run_dir = resolve_render_run_dir(&out, run, latest)?;
                let (mut workstreams, source, _) = load_effective_workstreams_for_run(&run_dir)?;
                let ledger_events = load_run_events(&run_dir)?;
                let result = delete_workstream(
                    &mut workstreams,
                    &workstream,
                    move_to.as_deref(),
                    force,
                    &ledger_events,
                )?;
                let errors = validate_workstreams_against_events(&workstreams, &ledger_events);
                if !errors.is_empty() {
                    for error in &errors {
                        eprintln!("- {error}");
                    }
                    anyhow::bail!("{} workstream validation error(s)", errors.len());
                }

                write_curated_workstreams(&run_dir, &workstreams)?;
                println!("Deleted workstream: {}", result.deleted_title);
                if let Some(target) = result.moved_to_title {
                    println!(
                        "Moved {} event(s) and {} receipt anchor(s) to {}.",
                        result.event_count, result.receipt_count, target
                    );
                } else if result.event_count > 0 || result.receipt_count > 0 {
                    println!(
                        "Discarded {} event assignment(s) and {} receipt anchor(s).",
                        result.event_count, result.receipt_count
                    );
                }
                println!(
                    "Updated: {}",
                    shiplog_workstreams::WorkstreamManager::curated_path(&run_dir).display()
                );
                if matches!(source, WorkstreamsFileSource::Suggested) {
                    println!("Created curated workstreams.yaml from suggested workstreams.");
                }
            }
            WorkstreamsCommand::Split {
                out,
                run,
                latest,
                from,
                to,
                matching,
                create,
            } => {
                let run_dir = resolve_render_run_dir(&out, run, latest)?;
                let (mut workstreams, source, _) = load_effective_workstreams_for_run(&run_dir)?;
                let ledger_events = load_run_events(&run_dir)?;
                let result = split_workstream(
                    &mut workstreams,
                    &from,
                    &to,
                    &matching,
                    create,
                    &ledger_events,
                )?;
                let errors = validate_workstreams_against_events(&workstreams, &ledger_events);
                if !errors.is_empty() {
                    for error in &errors {
                        eprintln!("- {error}");
                    }
                    anyhow::bail!("{} workstream validation error(s)", errors.len());
                }

                write_curated_workstreams(&run_dir, &workstreams)?;
                println!(
                    "Split {} event(s) from {} to {}",
                    result.event_count, result.from_title, result.to_title
                );
                println!("Matched: {}", result.pattern);
                if result.receipt_count > 0 {
                    println!("Moved {} receipt anchor(s).", result.receipt_count);
                }
                if result.created_target {
                    println!("Created target workstream: {}", result.to_title);
                }
                println!(
                    "Updated: {}",
                    shiplog_workstreams::WorkstreamManager::curated_path(&run_dir).display()
                );
                if matches!(source, WorkstreamsFileSource::Suggested) {
                    println!("Created curated workstreams.yaml from suggested workstreams.");
                }
            }
        },
        Command::Runs { cmd } => match cmd {
            RunsCommand::List { out } => {
                let summaries = load_run_summaries(&out)?;
                print_runs_list(&out, &summaries);
            }
            RunsCommand::Show { out, run, latest } => {
                let run_dir = resolve_render_run_dir(&out, run, latest)?;
                let summary = load_run_summary(&run_dir)?;
                print_run_show(&summary);
            }
            RunsCommand::Compare { out, from, to } => {
                let from_dir = resolve_run_selector(&out, &from)?;
                let to_dir = resolve_run_selector(&out, &to)?;
                let comparison = compare_runs(&from_dir, &to_dir)?;
                print_run_compare(&comparison);
            }
        },
        Command::Review { cmd, options } => match cmd {
            Some(ReviewCommand::Weekly {
                out,
                run,
                latest,
                strict,
            }) => {
                let run_dir = resolve_render_run_dir(&out, run, latest)?;
                print_weekly_review(&run_dir, strict)?;
            }
            None => {
                let run_dir = resolve_render_run_dir(&options.out, options.run, options.latest)?;
                print_review(&run_dir, options.strict)?;
            }
        },
        Command::Open { cmd } => match cmd {
            OpenCommand::Packet {
                out,
                run,
                latest,
                print_path,
            } => {
                let run_dir = resolve_render_run_dir(&out, run, latest)?;
                let packet = run_dir.join("packet.md");
                open_existing_path(
                    &packet,
                    "Packet",
                    "Run `shiplog render --latest` to create it.",
                    print_path,
                )?;
            }
            OpenCommand::Workstreams {
                out,
                run,
                latest,
                print_path,
            } => {
                let run_dir = resolve_render_run_dir(&out, run, latest)?;
                let (_, _, path) = load_effective_workstreams_for_run(&run_dir)?;
                open_existing_path(
                    &path,
                    "Workstreams file",
                    "Run `shiplog collect` first.",
                    print_path,
                )?;
            }
            OpenCommand::Out {
                out,
                run,
                latest,
                print_path,
            } => {
                let run_dir = resolve_render_run_dir(&out, run, latest)?;
                open_existing_path(
                    &run_dir,
                    "Run directory",
                    "Run `shiplog collect` first.",
                    print_path,
                )?;
            }
        },
        Command::Merge {
            inputs,
            out,
            conflict,
            user,
            window_label,
            zip,
            redact_key,
            bundle_profile,
            regen,
        } => {
            let redaction_key = RedactionKey::resolve(redact_key, &bundle_profile)?;
            let clusterer: Box<dyn shiplog_ports::WorkstreamClusterer> = Box::new(RepoClusterer);
            let (engine, redactor) =
                create_engine(redaction_key.engine_key(), clusterer, &bundle_profile);
            let engine = engine.with_profile_rendering(redaction_key.render_profiles());

            let mut ingest_outputs = Vec::with_capacity(inputs.len());
            for input in &inputs {
                ingest_outputs.push(
                    load_run_ingest(input)
                        .with_context(|| format!("load merge input {}", input.display()))?,
                );
            }

            let merged = engine
                .merge(ingest_outputs, conflict.into())
                .context("merge input runs")?;
            let merge_user = user.unwrap_or_else(|| merged.coverage.user.clone());
            let merge_window_label = window_label.unwrap_or_else(|| {
                format!(
                    "{}..{}",
                    merged.coverage.window.since, merged.coverage.window.until
                )
            });
            let run_id = merged.coverage.run_id.to_string();
            let run_dir = out.join(&run_id);

            if regen {
                let suggested = shiplog_workstreams::WorkstreamManager::suggested_path(&run_dir);
                if suggested.exists() {
                    std::fs::remove_file(&suggested)
                        .with_context(|| format!("remove {:?} for --regen", suggested))?;
                }
            }

            let cache_path = DeterministicRedactor::cache_path(&run_dir);
            let _ = redactor.load_cache(&cache_path);

            let (outputs, ws_source) = engine
                .run(
                    merged,
                    &merge_user,
                    &merge_window_label,
                    &run_dir,
                    zip,
                    &bundle_profile,
                )
                .context("run merged engine pipeline")?;

            redactor
                .save_cache(&cache_path)
                .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

            println!("Merged and wrote:");
            println!("- inputs: {}", inputs.len());
            println!("- conflict: {}", conflict.as_str());
            print_outputs(&outputs, ws_source);
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

            let redaction_key = RedactionKey::resolve(redact_key, &bundle_profile)?;
            let clusterer =
                build_clusterer(llm_cluster, &llm_api_endpoint, &llm_model, llm_api_key);
            let (engine, redactor) =
                create_engine(redaction_key.engine_key(), clusterer, &bundle_profile);
            let engine = engine.with_profile_rendering(redaction_key.render_profiles());

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
            let redaction_key = RedactionKey::resolve(redact_key, &bundle_profile)?;
            let clusterer =
                build_clusterer(llm_cluster, &llm_api_endpoint, &llm_model, llm_api_key);
            let (engine, redactor) =
                create_engine(redaction_key.engine_key(), clusterer, &bundle_profile);
            let engine = engine.with_profile_rendering(redaction_key.render_profiles());

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

fn event_count_phrase(count: usize) -> String {
    let noun = if count == 1 { "event" } else { "events" };
    format!("{count} {noun}")
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

#[derive(Clone, Copy, Debug)]
enum WorkstreamsFileSource {
    Curated,
    Suggested,
}

fn load_effective_workstreams_for_run(
    run_dir: &Path,
) -> Result<(WorkstreamsFile, WorkstreamsFileSource, PathBuf)> {
    let curated = shiplog_workstreams::WorkstreamManager::curated_path(run_dir);
    if curated.exists() {
        let workstreams = shiplog_workstreams::WorkstreamManager::try_load(run_dir)?
            .ok_or_else(|| anyhow::anyhow!("curated workstreams disappeared from {run_dir:?}"))?;
        return Ok((workstreams, WorkstreamsFileSource::Curated, curated));
    }

    let suggested = shiplog_workstreams::WorkstreamManager::suggested_path(run_dir);
    if suggested.exists() {
        let workstreams = shiplog_workstreams::WorkstreamManager::try_load(run_dir)?
            .ok_or_else(|| anyhow::anyhow!("suggested workstreams disappeared from {run_dir:?}"))?;
        return Ok((workstreams, WorkstreamsFileSource::Suggested, suggested));
    }

    anyhow::bail!(
        "No workstreams found in {:?}. Run `shiplog collect` first.",
        run_dir
    );
}

fn workstream_source_label(source: WorkstreamsFileSource) -> &'static str {
    match source {
        WorkstreamsFileSource::Curated => "curated workstreams.yaml",
        WorkstreamsFileSource::Suggested => "suggested workstreams.suggested.yaml",
    }
}

fn print_workstreams_list(
    run_dir: &Path,
    path: &Path,
    source: WorkstreamsFileSource,
    workstreams: &WorkstreamsFile,
) {
    println!(
        "Workstreams: {} ({})",
        path.display(),
        workstream_source_label(source)
    );
    println!("Run: {}", run_dir.display());
    println!("Count: {}", workstreams.workstreams.len());

    for workstream in &workstreams.workstreams {
        let tags = if workstream.tags.is_empty() {
            "-".to_string()
        } else {
            workstream.tags.join(",")
        };

        println!("- {} [{}]", workstream.title, workstream.id);
        println!(
            "  events={} receipts={} prs={} reviews={} manual={} tags={}",
            workstream.events.len(),
            workstream.receipts.len(),
            workstream.stats.pull_requests,
            workstream.stats.reviews,
            workstream.stats.manual_events,
            tags
        );
    }
}

fn rename_workstream(workstreams: &mut WorkstreamsFile, from: &str, to: &str) -> Result<String> {
    let new_title = to.trim();
    if new_title.is_empty() {
        anyhow::bail!("new workstream title cannot be blank");
    }

    let idx = find_workstream_index(workstreams, from)?;
    if workstreams
        .workstreams
        .iter()
        .enumerate()
        .any(|(other_idx, workstream)| other_idx != idx && workstream.title == new_title)
    {
        anyhow::bail!("another workstream is already titled {new_title:?}");
    }

    let old_title = workstreams.workstreams[idx].title.clone();
    workstreams.workstreams[idx].title = new_title.to_string();
    Ok(old_title)
}

struct MoveWorkstreamResult {
    event_id: String,
    from_titles: Vec<String>,
    to_title: String,
    receipt_preserved: bool,
}

struct ReceiptEditResult {
    event_id: String,
    event_title: String,
    workstream_title: String,
}

struct CreateWorkstreamResult {
    id: WorkstreamId,
    title: String,
}

struct DeleteWorkstreamResult {
    deleted_title: String,
    moved_to_title: Option<String>,
    event_count: usize,
    receipt_count: usize,
}

struct SplitWorkstreamResult {
    event_count: usize,
    receipt_count: usize,
    from_title: String,
    to_title: String,
    pattern: String,
    created_target: bool,
}

fn move_event_to_workstream(
    workstreams: &mut WorkstreamsFile,
    event_selector: &str,
    target_selector: &str,
    ledger_events: &[EventEnvelope],
) -> Result<MoveWorkstreamResult> {
    let target_idx = find_workstream_index(workstreams, target_selector)?;
    let event = ledger_events
        .iter()
        .find(|event| event.id.to_string() == event_selector)
        .ok_or_else(|| {
            anyhow::anyhow!("event {event_selector:?} was not found in ledger.events.jsonl")
        })?;
    let event_id = event.id.clone();
    let event_key = event.id.to_string();

    let mut from_titles = Vec::new();
    let mut receipt_preserved = false;

    for (idx, workstream) in workstreams.workstreams.iter_mut().enumerate() {
        let had_event = workstream
            .events
            .iter()
            .any(|candidate| candidate.to_string() == event_key);
        if had_event && idx != target_idx {
            from_titles.push(workstream.title.clone());
        }
        workstream
            .events
            .retain(|candidate| candidate.to_string() != event_key);

        let had_receipt = workstream
            .receipts
            .iter()
            .any(|candidate| candidate.to_string() == event_key);
        receipt_preserved |= had_receipt;
        workstream
            .receipts
            .retain(|candidate| candidate.to_string() != event_key);
    }

    let target = &mut workstreams.workstreams[target_idx];
    let to_title = target.title.clone();
    target.events.push(event_id.clone());
    if receipt_preserved {
        target.receipts.push(event_id);
    }

    recompute_workstream_stats(workstreams, ledger_events);

    Ok(MoveWorkstreamResult {
        event_id: event_key,
        from_titles,
        to_title,
        receipt_preserved,
    })
}

fn print_workstream_receipts(
    run_dir: &Path,
    path: &Path,
    source: WorkstreamsFileSource,
    workstreams: &WorkstreamsFile,
    ledger_events: &[EventEnvelope],
    workstream_selector: &str,
) -> Result<()> {
    let idx = find_workstream_index(workstreams, workstream_selector)?;
    let workstream = &workstreams.workstreams[idx];
    let by_id: HashMap<_, _> = ledger_events
        .iter()
        .map(|event| (event.id.to_string(), event))
        .collect();

    println!("Receipts: {} [{}]", workstream.title, workstream.id);
    println!("Run: {}", run_dir.display());
    println!(
        "Workstreams: {} ({})",
        path.display(),
        workstream_source_label(source)
    );
    println!("Count: {}", workstream.receipts.len());

    if workstream.receipts.is_empty() {
        println!("- (none)");
        return Ok(());
    }

    for receipt_id in &workstream.receipts {
        if let Some(event) = by_id.get(&receipt_id.to_string()) {
            println!("{}", format_receipt_markdown(event));
        } else {
            println!("- {} (missing from ledger.events.jsonl)", receipt_id);
        }
    }

    Ok(())
}

fn add_workstream_receipt(
    workstreams: &mut WorkstreamsFile,
    workstream_selector: &str,
    event_selector: &str,
    ledger_events: &[EventEnvelope],
) -> Result<ReceiptEditResult> {
    let idx = find_workstream_index(workstreams, workstream_selector)?;
    let event = find_ledger_event(ledger_events, event_selector)?;
    let event_id = event.id.clone();
    let event_key = event.id.to_string();
    let event_title = event_title(event).to_string();
    let workstream = &mut workstreams.workstreams[idx];

    if !contains_event_id(&workstream.events, &event_key) {
        anyhow::bail!(
            "event {event_key:?} is not assigned to workstream {:?}; use `shiplog workstreams move` first",
            workstream.title
        );
    }
    if contains_event_id(&workstream.receipts, &event_key) {
        anyhow::bail!(
            "event {event_key:?} is already a receipt anchor for workstream {:?}",
            workstream.title
        );
    }

    workstream.receipts.push(event_id);

    Ok(ReceiptEditResult {
        event_id: event_key,
        event_title,
        workstream_title: workstream.title.clone(),
    })
}

fn remove_workstream_receipt(
    workstreams: &mut WorkstreamsFile,
    workstream_selector: &str,
    event_selector: &str,
    ledger_events: &[EventEnvelope],
) -> Result<ReceiptEditResult> {
    let idx = find_workstream_index(workstreams, workstream_selector)?;
    let event = find_ledger_event(ledger_events, event_selector)?;
    let event_key = event.id.to_string();
    let event_title = event_title(event).to_string();
    let workstream = &mut workstreams.workstreams[idx];

    let Some(receipt_idx) = workstream
        .receipts
        .iter()
        .position(|candidate| candidate.to_string() == event_key)
    else {
        anyhow::bail!(
            "event {event_key:?} is not a receipt anchor for workstream {:?}",
            workstream.title
        );
    };

    workstream.receipts.remove(receipt_idx);

    Ok(ReceiptEditResult {
        event_id: event_key,
        event_title,
        workstream_title: workstream.title.clone(),
    })
}

fn create_workstream(
    workstreams: &mut WorkstreamsFile,
    title: &str,
) -> Result<CreateWorkstreamResult> {
    let title = title.trim();
    if title.is_empty() {
        anyhow::bail!("workstream title cannot be blank");
    }
    if find_workstream_index_optional(workstreams, title)?.is_some() {
        anyhow::bail!("a workstream already matches {title:?}");
    }

    let id = WorkstreamId::from_parts(["curated", title]);
    workstreams.workstreams.push(Workstream {
        id: id.clone(),
        title: title.to_string(),
        summary: None,
        tags: vec![],
        stats: WorkstreamStats::zero(),
        events: vec![],
        receipts: vec![],
    });

    Ok(CreateWorkstreamResult {
        id,
        title: title.to_string(),
    })
}

fn delete_workstream(
    workstreams: &mut WorkstreamsFile,
    workstream_selector: &str,
    move_to: Option<&str>,
    force: bool,
    ledger_events: &[EventEnvelope],
) -> Result<DeleteWorkstreamResult> {
    if force && move_to.is_some() {
        anyhow::bail!("use either --move-to or --force, not both");
    }

    let source_idx = find_workstream_index(workstreams, workstream_selector)?;
    let event_count = workstreams.workstreams[source_idx].events.len();
    let receipt_count = workstreams.workstreams[source_idx].receipts.len();
    let is_non_empty = event_count > 0 || receipt_count > 0;

    if is_non_empty && move_to.is_none() && !force {
        anyhow::bail!(
            "workstream {:?} is not empty; use --move-to <workstream> or --force",
            workstreams.workstreams[source_idx].title
        );
    }

    let mut moved_to_title = None;
    if let Some(target_selector) = move_to {
        let target_idx = find_workstream_index(workstreams, target_selector)?;
        if source_idx == target_idx {
            anyhow::bail!("delete target and --move-to target must be different workstreams");
        }

        let events = workstreams.workstreams[source_idx].events.clone();
        let receipts = workstreams.workstreams[source_idx].receipts.clone();
        let target = &mut workstreams.workstreams[target_idx];
        moved_to_title = Some(target.title.clone());
        append_unique_event_ids(&mut target.events, events);
        append_unique_event_ids(&mut target.receipts, receipts);
    }

    let deleted = workstreams.workstreams.remove(source_idx);
    recompute_workstream_stats(workstreams, ledger_events);

    Ok(DeleteWorkstreamResult {
        deleted_title: deleted.title,
        moved_to_title,
        event_count,
        receipt_count,
    })
}

fn append_unique_event_ids(
    target: &mut Vec<shiplog_ids::EventId>,
    incoming: Vec<shiplog_ids::EventId>,
) {
    for event_id in incoming {
        let event_key = event_id.to_string();
        if !target
            .iter()
            .any(|candidate| candidate.to_string() == event_key)
        {
            target.push(event_id);
        }
    }
}

fn split_workstream(
    workstreams: &mut WorkstreamsFile,
    from_selector: &str,
    target_selector: &str,
    pattern: &str,
    create: bool,
    ledger_events: &[EventEnvelope],
) -> Result<SplitWorkstreamResult> {
    let pattern = pattern.trim();
    if pattern.is_empty() {
        anyhow::bail!("split matching pattern cannot be blank");
    }
    let matcher = RegexBuilder::new(pattern)
        .case_insensitive(true)
        .build()
        .with_context(|| format!("compile split matching regex {pattern:?}"))?;

    let from_idx = find_workstream_index(workstreams, from_selector)?;
    let from_title = workstreams.workstreams[from_idx].title.clone();
    let (target_idx, created_target) =
        resolve_split_target_index(workstreams, target_selector, create)?;
    if from_idx == target_idx {
        anyhow::bail!("split target must be different from source workstream");
    }
    let to_title = workstreams.workstreams[target_idx].title.clone();

    let events_by_id: HashMap<_, _> = ledger_events
        .iter()
        .map(|event| (event.id.to_string(), event))
        .collect();

    let mut matched_ids = HashSet::new();
    for event_id in workstreams.workstreams[from_idx].events.clone() {
        let event_key = event_id.to_string();
        let event = events_by_id.get(event_key.as_str()).ok_or_else(|| {
            anyhow::anyhow!("event {event_key:?} was not found in ledger.events.jsonl")
        })?;
        if event_matches_split_pattern(event, &matcher) {
            matched_ids.insert(event_key);
        }
    }

    if matched_ids.is_empty() {
        anyhow::bail!("no events in {from_title:?} matched {pattern:?}");
    }

    let mut moved_events = Vec::new();
    let mut moved_receipts = Vec::new();
    for workstream in &mut workstreams.workstreams {
        workstream.events.retain(|event_id| {
            let event_key = event_id.to_string();
            if matched_ids.contains(&event_key) {
                if !moved_events
                    .iter()
                    .any(|candidate: &shiplog_ids::EventId| candidate.to_string() == event_key)
                {
                    moved_events.push(event_id.clone());
                }
                false
            } else {
                true
            }
        });

        workstream.receipts.retain(|event_id| {
            let event_key = event_id.to_string();
            if matched_ids.contains(&event_key) {
                if !moved_receipts
                    .iter()
                    .any(|candidate: &shiplog_ids::EventId| candidate.to_string() == event_key)
                {
                    moved_receipts.push(event_id.clone());
                }
                false
            } else {
                true
            }
        });
    }

    let target = &mut workstreams.workstreams[target_idx];
    target.events.extend(moved_events.iter().cloned());
    target.receipts.extend(moved_receipts.iter().cloned());

    recompute_workstream_stats(workstreams, ledger_events);

    Ok(SplitWorkstreamResult {
        event_count: moved_events.len(),
        receipt_count: moved_receipts.len(),
        from_title,
        to_title,
        pattern: pattern.to_string(),
        created_target,
    })
}

fn resolve_split_target_index(
    workstreams: &mut WorkstreamsFile,
    target_selector: &str,
    create: bool,
) -> Result<(usize, bool)> {
    let target_selector = target_selector.trim();
    if target_selector.is_empty() {
        anyhow::bail!("target workstream title or ID cannot be blank");
    }

    if let Some(idx) = find_workstream_index_optional(workstreams, target_selector)? {
        return Ok((idx, false));
    }

    if !create {
        anyhow::bail!("no workstream matched {target_selector:?}; add --create to create it");
    }

    let idx = workstreams.workstreams.len();
    workstreams.workstreams.push(Workstream {
        id: WorkstreamId::from_parts(["split", target_selector]),
        title: target_selector.to_string(),
        summary: None,
        tags: vec![],
        stats: WorkstreamStats::zero(),
        events: vec![],
        receipts: vec![],
    });
    Ok((idx, true))
}

fn event_matches_split_pattern(event: &EventEnvelope, matcher: &Regex) -> bool {
    event_split_match_fields(event)
        .into_iter()
        .any(|field| matcher.is_match(&field))
}

fn event_split_match_fields(event: &EventEnvelope) -> Vec<String> {
    let mut fields = Vec::new();
    fields.push(event_title(event).to_string());
    fields.push(event.repo.full_name.clone());
    if let Some(url) = &event.repo.html_url {
        fields.push(url.clone());
    }
    fields.extend(event.tags.iter().cloned());
    if let Some(url) = &event.source.url {
        fields.push(url.clone());
    }
    fields.extend(event.links.iter().map(|link| link.url.clone()));
    fields
}

fn event_title(event: &EventEnvelope) -> &str {
    match &event.payload {
        EventPayload::PullRequest(payload) => &payload.title,
        EventPayload::Review(payload) => &payload.pull_title,
        EventPayload::Manual(payload) => &payload.title,
    }
}

fn find_ledger_event<'a>(
    ledger_events: &'a [EventEnvelope],
    event_selector: &str,
) -> Result<&'a EventEnvelope> {
    let event_selector = event_selector.trim();
    if event_selector.is_empty() {
        anyhow::bail!("event selector cannot be blank");
    }

    ledger_events
        .iter()
        .find(|event| event.id.to_string() == event_selector)
        .ok_or_else(|| {
            anyhow::anyhow!("event {event_selector:?} was not found in ledger.events.jsonl")
        })
}

fn contains_event_id(event_ids: &[EventId], event_key: &str) -> bool {
    event_ids
        .iter()
        .any(|candidate| candidate.to_string() == event_key)
}

fn find_workstream_index(workstreams: &WorkstreamsFile, selector: &str) -> Result<usize> {
    let selector = selector.trim();
    if selector.is_empty() {
        anyhow::bail!("workstream selector cannot be blank");
    }

    match find_workstream_index_optional(workstreams, selector)? {
        Some(idx) => Ok(idx),
        None => anyhow::bail!(
            "no workstream matched {selector:?}; run `shiplog workstreams list` to see available titles and IDs"
        ),
    }
}

fn find_workstream_index_optional(
    workstreams: &WorkstreamsFile,
    selector: &str,
) -> Result<Option<usize>> {
    let selector = selector.trim();
    if selector.is_empty() {
        anyhow::bail!("workstream selector cannot be blank");
    }

    let matches: Vec<_> = workstreams
        .workstreams
        .iter()
        .enumerate()
        .filter(|(_, workstream)| {
            workstream.title == selector || workstream.id.to_string() == selector
        })
        .map(|(idx, _)| idx)
        .collect();

    match matches.as_slice() {
        [idx] => Ok(Some(*idx)),
        [] => Ok(None),
        _ => anyhow::bail!(
            "multiple workstreams matched {selector:?}; use the workstream ID instead"
        ),
    }
}

fn recompute_workstream_stats(workstreams: &mut WorkstreamsFile, ledger_events: &[EventEnvelope]) {
    let event_kinds: HashMap<_, _> = ledger_events
        .iter()
        .map(|event| (event.id.to_string(), event.kind.clone()))
        .collect();

    for workstream in &mut workstreams.workstreams {
        workstream.stats = WorkstreamStats::zero();
        let event_ids = workstream.events.clone();
        for event_id in event_ids {
            if let Some(kind) = event_kinds.get(&event_id.to_string()) {
                workstream.bump_stats(kind);
            }
        }
    }
}

fn write_curated_workstreams(run_dir: &Path, workstreams: &WorkstreamsFile) -> Result<()> {
    let curated_path = shiplog_workstreams::WorkstreamManager::curated_path(run_dir);
    shiplog_workstreams::write_workstreams(&curated_path, workstreams)
        .with_context(|| format!("write curated workstreams to {curated_path:?}"))
}

fn validate_workstreams_for_run(
    run_dir: &Path,
    workstreams: &WorkstreamsFile,
) -> Result<Vec<String>> {
    let ledger_events = load_run_events(run_dir)?;
    Ok(validate_workstreams_against_events(
        workstreams,
        &ledger_events,
    ))
}

fn validate_workstreams_against_events(
    workstreams: &WorkstreamsFile,
    ledger_events: &[EventEnvelope],
) -> Vec<String> {
    let ledger_ids: HashSet<_> = ledger_events
        .iter()
        .map(|event| event.id.to_string())
        .collect();
    let mut errors = Vec::new();

    if workstreams.version != 1 {
        errors.push(format!(
            "unsupported workstreams version {}; expected 1",
            workstreams.version
        ));
    }

    let mut workstream_ids = HashSet::new();
    let mut assigned_events: HashMap<String, String> = HashMap::new();

    for (idx, workstream) in workstreams.workstreams.iter().enumerate() {
        let ordinal = idx + 1;
        let title = if workstream.title.trim().is_empty() {
            format!("#{ordinal}")
        } else {
            workstream.title.clone()
        };

        if !workstream_ids.insert(workstream.id.to_string()) {
            errors.push(format!("duplicate workstream id {}", workstream.id));
        }

        if workstream.title.trim().is_empty() {
            errors.push(format!("workstream {ordinal} has a blank title"));
        }

        let mut local_events = HashSet::new();
        for event_id in &workstream.events {
            let event_id = event_id.to_string();
            if !local_events.insert(event_id.clone()) {
                errors.push(format!(
                    "workstream {title} lists event {event_id} more than once"
                ));
                continue;
            }

            if !ledger_ids.contains(&event_id) {
                errors.push(format!(
                    "workstream {title} references event {event_id} not found in ledger.events.jsonl"
                ));
            }

            if let Some(previous) = assigned_events.insert(event_id.clone(), title.clone()) {
                errors.push(format!(
                    "event {event_id} is assigned to both {previous} and {title}"
                ));
            }
        }

        let mut local_receipts = HashSet::new();
        for receipt_id in &workstream.receipts {
            let receipt_id = receipt_id.to_string();
            if !local_receipts.insert(receipt_id.clone()) {
                errors.push(format!(
                    "workstream {title} lists receipt {receipt_id} more than once"
                ));
                continue;
            }

            if !ledger_ids.contains(&receipt_id) {
                errors.push(format!(
                    "workstream {title} references receipt {receipt_id} not found in ledger.events.jsonl"
                ));
            }

            if !local_events.contains(&receipt_id) {
                errors.push(format!(
                    "workstream {title} receipt {receipt_id} is not listed in that workstream's events"
                ));
            }
        }
    }

    errors
}

fn load_run_events(run_dir: &Path) -> Result<Vec<EventEnvelope>> {
    let ingest = load_run_ingest(run_dir).context("ingest run ledger for workstream validation")?;
    Ok(ingest.events)
}

fn load_run_ingest(run_dir: &Path) -> Result<IngestOutput> {
    let events_path = run_dir.join("ledger.events.jsonl");
    if !events_path.exists() {
        anyhow::bail!(
            "No ledger.events.jsonl found in {:?}. Run `shiplog collect` first.",
            run_dir
        );
    }

    let coverage_path = run_dir.join("coverage.manifest.json");
    if !coverage_path.exists() {
        anyhow::bail!(
            "No coverage.manifest.json found in {:?}. Run `shiplog collect` first.",
            run_dir
        );
    }

    let ingest = JsonIngestor {
        events_path,
        coverage_path,
    }
    .ingest()
    .context("ingest run ledger")?;

    Ok(ingest)
}

struct RunSummary {
    run_dir: PathBuf,
    run_id: String,
    generated_at: String,
    modified_at: String,
    user: String,
    window: String,
    mode: String,
    sources: Vec<String>,
    event_count: usize,
    completeness: String,
    gap_count: usize,
    warnings: Vec<String>,
    packet_path: PathBuf,
    ledger_path: PathBuf,
    coverage_path: PathBuf,
}

struct RunCompareData {
    summary: RunSummary,
    events: Vec<EventEnvelope>,
    workstreams: WorkstreamsFile,
}

struct RunComparison {
    from: RunCompareData,
    to: RunCompareData,
    added_sources: Vec<String>,
    removed_sources: Vec<String>,
    continued_sources: Vec<String>,
    added_workstreams: Vec<String>,
    removed_workstreams: Vec<String>,
    continued_workstreams: Vec<String>,
    expanded_workstreams: Vec<(String, isize)>,
    contracted_workstreams: Vec<(String, isize)>,
}

fn load_run_summaries(out_dir: &Path) -> Result<Vec<RunSummary>> {
    discover_run_dirs(out_dir)?
        .iter()
        .map(|run_dir| load_run_summary(run_dir))
        .collect()
}

fn load_run_summary(run_dir: &Path) -> Result<RunSummary> {
    let ingest =
        load_run_ingest(run_dir).with_context(|| format!("load run {}", run_dir.display()))?;
    let coverage = ingest.coverage;
    let run_id = coverage.run_id.to_string();
    let generated_at = coverage.generated_at.to_rfc3339();
    let modified_at = modified_time_label(run_dir);
    let window = format!("{}..{}", coverage.window.since, coverage.window.until);
    let gap_count = coverage_gap_count(&coverage);

    Ok(RunSummary {
        run_dir: run_dir.to_path_buf(),
        run_id,
        generated_at,
        modified_at,
        user: coverage.user,
        window,
        mode: coverage.mode,
        sources: coverage.sources,
        event_count: ingest.events.len(),
        completeness: coverage.completeness.to_string(),
        gap_count,
        warnings: coverage.warnings,
        packet_path: run_dir.join("packet.md"),
        ledger_path: run_dir.join("ledger.events.jsonl"),
        coverage_path: run_dir.join("coverage.manifest.json"),
    })
}

fn discover_run_dirs(out_dir: &Path) -> Result<Vec<PathBuf>> {
    if !out_dir.exists() {
        anyhow::bail!("Output directory {:?} does not exist.", out_dir);
    }

    let mut runs: Vec<_> = std::fs::read_dir(out_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .filter(|entry| entry.path().join("ledger.events.jsonl").exists())
        .collect();

    runs.sort_by(|a, b| {
        let a_meta = a.metadata().and_then(|meta| meta.modified()).ok();
        let b_meta = b.metadata().and_then(|meta| meta.modified()).ok();
        b_meta.cmp(&a_meta)
    });

    let runs: Vec<_> = runs.into_iter().map(|entry| entry.path()).collect();
    if runs.is_empty() {
        anyhow::bail!("No run directories found in {:?}", out_dir);
    }

    Ok(runs)
}

fn modified_time_label(path: &Path) -> String {
    path.metadata()
        .and_then(|meta| meta.modified())
        .map(|time| DateTime::<Utc>::from(time).to_rfc3339())
        .unwrap_or_else(|_| "unknown".to_string())
}

fn coverage_gap_count(coverage: &shiplog_schema::coverage::CoverageManifest) -> usize {
    coverage.warnings.len()
        + coverage
            .slices
            .iter()
            .filter(|slice| {
                slice.incomplete_results.unwrap_or(false) || slice.fetched < slice.total_count
            })
            .count()
}

fn print_runs_list(out_dir: &Path, summaries: &[RunSummary]) {
    println!("Runs: {}", out_dir.display());
    println!("Count: {}", summaries.len());

    for summary in summaries {
        println!("- {}", summary.run_id);
        println!("  modified: {}", summary.modified_at);
        println!("  sources: {}", source_list_label(&summary.sources));
        println!("  events: {}", summary.event_count);
        println!("  coverage: {}", summary.completeness);
        println!("  packet: {}", summary.packet_path.display());
        println!("  gaps: {}", summary.gap_count);
    }
}

fn print_run_show(summary: &RunSummary) {
    println!("Run: {}", summary.run_id);
    println!("Directory: {}", summary.run_dir.display());
    println!("Created: {}", summary.generated_at);
    println!("Modified: {}", summary.modified_at);
    println!("User: {}", summary.user);
    println!("Window: {}", summary.window);
    println!("Mode: {}", summary.mode);
    println!("Sources: {}", source_list_label(&summary.sources));
    println!("Events: {}", summary.event_count);
    println!("Coverage: {}", summary.completeness);
    println!("Gaps: {}", summary.gap_count);
    println!("Packet: {}", summary.packet_path.display());
    println!("Ledger: {}", summary.ledger_path.display());
    println!("Coverage manifest: {}", summary.coverage_path.display());
    if summary.warnings.is_empty() {
        println!("Warnings: none");
    } else {
        println!("Warnings:");
        for warning in &summary.warnings {
            println!("- {warning}");
        }
    }
}

fn resolve_run_selector(out_dir: &Path, selector: &str) -> Result<PathBuf> {
    resolve_render_run_dir(out_dir, Some(selector.to_string()), false)
}

fn compare_runs(from_dir: &Path, to_dir: &Path) -> Result<RunComparison> {
    let from = load_run_compare_data(from_dir)?;
    let to = load_run_compare_data(to_dir)?;

    let from_sources = source_set(&from.summary.sources, &from.events);
    let to_sources = source_set(&to.summary.sources, &to.events);
    let from_workstreams = workstream_event_counts(&from.workstreams);
    let to_workstreams = workstream_event_counts(&to.workstreams);
    let from_workstream_titles = from_workstreams.keys().cloned().collect::<BTreeSet<_>>();
    let to_workstream_titles = to_workstreams.keys().cloned().collect::<BTreeSet<_>>();

    let mut expanded_workstreams = Vec::new();
    let mut contracted_workstreams = Vec::new();
    for title in from_workstream_titles.intersection(&to_workstream_titles) {
        let from_count = *from_workstreams.get(title).unwrap_or(&0);
        let to_count = *to_workstreams.get(title).unwrap_or(&0);
        let delta = to_count as isize - from_count as isize;
        if delta > 0 {
            expanded_workstreams.push((title.clone(), delta));
        } else if delta < 0 {
            contracted_workstreams.push((title.clone(), delta));
        }
    }

    Ok(RunComparison {
        from,
        to,
        added_sources: display_source_set_diff(&to_sources, &from_sources),
        removed_sources: display_source_set_diff(&from_sources, &to_sources),
        continued_sources: display_source_set_intersection(&from_sources, &to_sources),
        added_workstreams: set_diff(&to_workstream_titles, &from_workstream_titles),
        removed_workstreams: set_diff(&from_workstream_titles, &to_workstream_titles),
        continued_workstreams: set_intersection(&from_workstream_titles, &to_workstream_titles),
        expanded_workstreams,
        contracted_workstreams,
    })
}

fn load_run_compare_data(run_dir: &Path) -> Result<RunCompareData> {
    let summary = load_run_summary(run_dir)?;
    let ingest =
        load_run_ingest(run_dir).with_context(|| format!("load run {}", run_dir.display()))?;
    let workstreams = shiplog_workstreams::WorkstreamManager::try_load(run_dir)?
        .unwrap_or_else(empty_workstreams_file);

    Ok(RunCompareData {
        summary,
        events: ingest.events,
        workstreams,
    })
}

fn empty_workstreams_file() -> WorkstreamsFile {
    WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: Vec::new(),
    }
}

fn source_set(summary_sources: &[String], events: &[EventEnvelope]) -> BTreeSet<String> {
    let mut sources = BTreeSet::new();
    for source in summary_sources {
        sources.insert(normalized_source_key(source));
    }
    for event in events {
        sources.insert(normalized_source_key(event.source.system.as_str()));
    }
    sources
}

fn workstream_event_counts(workstreams: &WorkstreamsFile) -> BTreeMap<String, usize> {
    workstreams
        .workstreams
        .iter()
        .map(|workstream| (workstream.title.clone(), workstream.events.len()))
        .collect()
}

fn display_source_set_diff(left: &BTreeSet<String>, right: &BTreeSet<String>) -> Vec<String> {
    left.difference(right)
        .map(|source| display_source_label(source))
        .collect()
}

fn display_source_set_intersection(
    left: &BTreeSet<String>,
    right: &BTreeSet<String>,
) -> Vec<String> {
    left.intersection(right)
        .map(|source| display_source_label(source))
        .collect()
}

fn set_diff(left: &BTreeSet<String>, right: &BTreeSet<String>) -> Vec<String> {
    left.difference(right).cloned().collect()
}

fn set_intersection(left: &BTreeSet<String>, right: &BTreeSet<String>) -> Vec<String> {
    left.intersection(right).cloned().collect()
}

fn print_run_compare(comparison: &RunComparison) {
    let from = &comparison.from.summary;
    let to = &comparison.to.summary;
    let event_delta = to.event_count as isize - from.event_count as isize;
    let gap_delta = to.gap_count as isize - from.gap_count as isize;

    println!("Compare: {} -> {}", from.run_id, to.run_id);
    println!("From: {}", from.run_dir.display());
    println!("To: {}", to.run_dir.display());
    println!();

    println!("Events:");
    println!("- from: {}", from.event_count);
    println!("- to: {}", to.event_count);
    println!("- delta: {}", signed_count(event_delta));
    println!();

    println!("Sources:");
    print_named_list("Added", &comparison.added_sources);
    print_named_list("Removed", &comparison.removed_sources);
    print_named_list("Continued", &comparison.continued_sources);
    println!();

    println!("Workstreams:");
    print_named_list("Added", &comparison.added_workstreams);
    print_named_list("Removed", &comparison.removed_workstreams);
    print_named_list("Continued", &comparison.continued_workstreams);
    print_workstream_delta_list("Expanded", &comparison.expanded_workstreams);
    print_workstream_delta_list("Contracted", &comparison.contracted_workstreams);
    println!();

    println!("Coverage:");
    println!("- from: {}, gaps: {}", from.completeness, from.gap_count);
    println!("- to: {}, gaps: {}", to.completeness, to.gap_count);
    println!("- gap delta: {}", signed_count(gap_delta));
    if from.completeness != to.completeness {
        println!(
            "- completeness changed: {} -> {}",
            from.completeness, to.completeness
        );
    } else {
        println!("- completeness changed: no");
    }
    println!();

    println!("Next:");
    println!("1. shiplog review --run {}", to.run_id);
    println!("2. shiplog render --run {} --mode scaffold", to.run_id);
}

fn print_named_list(label: &str, values: &[String]) {
    if values.is_empty() {
        println!("- {label}: None");
    } else {
        println!("- {label}: {}", values.join(", "));
    }
}

fn print_workstream_delta_list(label: &str, values: &[(String, isize)]) {
    if values.is_empty() {
        println!("- {label}: None");
        return;
    }

    println!("- {label}:");
    for (title, delta) in values {
        println!("  - {title}: {} event(s)", signed_count(*delta));
    }
}

fn signed_count(value: isize) -> String {
    if value > 0 {
        format!("+{value}")
    } else {
        value.to_string()
    }
}

struct ConfiguredSourceSkip {
    source: String,
    reason: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EvidenceDebtSeverity {
    Info,
    Warning,
    Blocking,
}

impl EvidenceDebtSeverity {
    fn label(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Blocking => "blocking",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EvidenceDebtKind {
    MissingSource,
    PartialCoverage,
    CoverageWarning,
    IncompleteQuery,
    ManualContext,
    MissingReceiptAnchors,
    BroadWorkstream,
    WorkstreamValidation,
}

impl EvidenceDebtKind {
    fn label(self) -> &'static str {
        match self {
            Self::MissingSource => "missing-source",
            Self::PartialCoverage => "partial-coverage",
            Self::CoverageWarning => "coverage-warning",
            Self::IncompleteQuery => "incomplete-query",
            Self::ManualContext => "manual-context",
            Self::MissingReceiptAnchors => "missing-receipts",
            Self::BroadWorkstream => "broad-workstream",
            Self::WorkstreamValidation => "workstream-validation",
        }
    }
}

#[derive(Debug)]
struct EvidenceDebt {
    severity: EvidenceDebtSeverity,
    kind: EvidenceDebtKind,
    summary: String,
    detail: Option<String>,
    next_step: Option<String>,
}

struct EvidenceDebtInput<'a> {
    run_id: &'a str,
    coverage: &'a CoverageManifest,
    events: &'a [EventEnvelope],
    skipped_sources: &'a [ConfiguredSourceSkip],
    workstreams: &'a WorkstreamsFile,
    validation_errors: &'a [String],
    no_receipt_workstreams: &'a [&'a Workstream],
    broad_workstreams: &'a [&'a Workstream],
    manual_context_workstreams: &'a [&'a Workstream],
    manual_events: usize,
}

impl EvidenceDebt {
    fn new(
        severity: EvidenceDebtSeverity,
        kind: EvidenceDebtKind,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            kind,
            summary: summary.into(),
            detail: None,
            next_step: None,
        }
    }

    fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    fn next_step(mut self, next_step: impl Into<String>) -> Self {
        self.next_step = Some(next_step.into());
        self
    }
}

fn print_weekly_review(run_dir: &Path, strict: bool) -> Result<()> {
    let ingest =
        load_run_ingest(run_dir).with_context(|| format!("load run {}", run_dir.display()))?;
    let coverage = ingest.coverage;
    let events = ingest.events;
    let skipped_sources = configured_source_skips(&coverage.warnings);
    let counts = review_source_event_counts(&coverage.sources, &events, &skipped_sources);

    println!("Weekly review: {}", coverage.run_id);
    println!("Directory: {}", run_dir.display());
    println!(
        "Window: {}..{}",
        coverage.window.since, coverage.window.until
    );
    println!();

    println!("New evidence:");
    println!("- total: {} event(s)", events.len());
    if counts.is_empty() {
        println!("- No included source events");
    } else {
        for (source, count) in counts {
            println!("- {}: {} event(s)", display_source_label(&source), count);
        }
    }
    if !skipped_sources.is_empty() {
        println!("Source gaps:");
        for skipped in &skipped_sources {
            println!(
                "- {}: {}",
                display_source_label(&skipped.source),
                skipped.reason
            );
        }
    }
    println!();

    print_review(run_dir, strict)
}

fn print_review(run_dir: &Path, strict: bool) -> Result<()> {
    let ingest =
        load_run_ingest(run_dir).with_context(|| format!("load run {}", run_dir.display()))?;
    let coverage = ingest.coverage;
    let events = ingest.events;
    let run_id = coverage.run_id.to_string();
    let skipped_sources = configured_source_skips(&coverage.warnings);
    let (workstreams, source, path) = load_effective_workstreams_for_run(run_dir)?;
    let validation_errors = validate_workstreams_against_events(&workstreams, &events);
    let no_receipt_workstreams: Vec<_> = workstreams
        .workstreams
        .iter()
        .filter(|workstream| workstream.receipts.is_empty())
        .collect();
    let broad_workstreams: Vec<_> = workstreams
        .workstreams
        .iter()
        .filter(|workstream| workstream.events.len() >= 10)
        .collect();
    let manual_events = events
        .iter()
        .filter(|event| matches!(event.payload, EventPayload::Manual(_)))
        .count();
    let manual_event_ids: HashSet<_> = events
        .iter()
        .filter(|event| matches!(event.payload, EventPayload::Manual(_)))
        .map(|event| event.id.clone())
        .collect();
    let manual_context_workstreams: Vec<_> = broad_workstreams
        .iter()
        .copied()
        .filter(|workstream| {
            !workstream
                .events
                .iter()
                .any(|event_id| manual_event_ids.contains(event_id))
        })
        .collect();
    let evidence_debt = detect_evidence_debt(EvidenceDebtInput {
        run_id: &run_id,
        coverage: &coverage,
        events: &events,
        skipped_sources: &skipped_sources,
        workstreams: &workstreams,
        validation_errors: &validation_errors,
        no_receipt_workstreams: &no_receipt_workstreams,
        broad_workstreams: &broad_workstreams,
        manual_context_workstreams: &manual_context_workstreams,
        manual_events,
    });

    println!("Run: {run_id}");
    println!("Directory: {}", run_dir.display());
    println!(
        "Window: {}..{}",
        coverage.window.since, coverage.window.until
    );
    println!("User: {}", coverage.user);
    println!();

    println!("Coverage:");
    let counts = review_source_event_counts(&coverage.sources, &events, &skipped_sources);
    if counts.is_empty() {
        println!("- No included source events");
    } else {
        for (source, count) in counts {
            println!("- {}: {} event(s)", display_source_label(&source), count);
        }
    }
    println!("Completeness: {}", coverage.completeness);
    println!("Gaps: {}", coverage_gap_count(&coverage));
    if !skipped_sources.is_empty() {
        println!("Skipped sources:");
        for skipped in &skipped_sources {
            println!(
                "- {}: {}",
                display_source_label(&skipped.source),
                skipped.reason
            );
        }
    }
    println!();

    println!("Curation:");
    println!(
        "- Workstreams: {} ({})",
        workstreams.workstreams.len(),
        workstream_source_label(source)
    );
    println!("- Workstreams file: {}", path.display());
    println!(
        "- Workstreams with no selected receipts: {}",
        no_receipt_workstreams.len()
    );
    println!("- Workstreams with 10+ events: {}", broad_workstreams.len());
    if validation_errors.is_empty() {
        println!("- Validation: ok");
    } else {
        println!("- Validation: {} issue(s)", validation_errors.len());
        for error in validation_errors.iter().take(5) {
            println!("  - {error}");
        }
        if validation_errors.len() > 5 {
            println!("  - ... and {} more", validation_errors.len() - 5);
        }
    }
    println!();

    print_evidence_debt(&evidence_debt);
    println!();

    print_review_next_steps(
        &run_id,
        !validation_errors.is_empty(),
        no_receipt_workstreams
            .first()
            .map(|workstream| workstream.title.as_str()),
        broad_workstreams
            .first()
            .map(|workstream| workstream.title.as_str()),
        manual_context_workstreams
            .first()
            .map(|workstream| workstream.title.as_str()),
        !skipped_sources.is_empty(),
    );

    if strict && !evidence_debt.is_empty() {
        anyhow::bail!(
            "review found {} evidence debt item(s); rerun without --strict to inspect details",
            evidence_debt.len()
        );
    }

    Ok(())
}

fn detect_evidence_debt(input: EvidenceDebtInput<'_>) -> Vec<EvidenceDebt> {
    let mut debt = Vec::new();

    for skipped in input.skipped_sources {
        debt.push(
            EvidenceDebt::new(
                EvidenceDebtSeverity::Warning,
                EvidenceDebtKind::MissingSource,
                format!(
                    "{} was skipped: {}",
                    display_source_label(&skipped.source),
                    skipped.reason
                ),
            )
            .next_step("Run `shiplog doctor` to check source configuration and tokens."),
        );
    }

    if input.coverage.completeness != shiplog_schema::coverage::Completeness::Complete {
        debt.push(
            EvidenceDebt::new(
                EvidenceDebtSeverity::Warning,
                EvidenceDebtKind::PartialCoverage,
                format!(
                    "Coverage is {}; inspect coverage.manifest.json before making strong claims.",
                    input.coverage.completeness
                ),
            )
            .next_step("Inspect coverage.manifest.json before sharing this packet."),
        );
    }

    for warning in input
        .coverage
        .warnings
        .iter()
        .filter(|warning| configured_source_skip(warning).is_none())
    {
        debt.push(EvidenceDebt::new(
            EvidenceDebtSeverity::Warning,
            EvidenceDebtKind::CoverageWarning,
            warning.clone(),
        ));
    }

    for slice in input.coverage.slices.iter().filter(|slice| {
        slice.incomplete_results.unwrap_or(false) || slice.fetched < slice.total_count
    }) {
        debt.push(EvidenceDebt::new(
            EvidenceDebtSeverity::Warning,
            EvidenceDebtKind::IncompleteQuery,
            format!(
                "Query {:?} fetched {}/{} result(s).",
                slice.query, slice.fetched, slice.total_count
            ),
        ));
    }

    if input.manual_events > 0 {
        debt.push(EvidenceDebt::new(
            EvidenceDebtSeverity::Info,
            EvidenceDebtKind::ManualContext,
            "Manual events are user-provided; keep context current before sharing.",
        ));
    }

    if !input.manual_context_workstreams.is_empty() {
        let first = input.manual_context_workstreams[0];
        debt.push(
            EvidenceDebt::new(
                EvidenceDebtSeverity::Info,
                EvidenceDebtKind::ManualContext,
                format!(
                    "{} broad workstream(s) have no manual outcome note.",
                    input.manual_context_workstreams.len()
                ),
            )
            .detail(workstream_title_sample(input.manual_context_workstreams))
            .next_step(journal_add_next_step(&first.title)),
        );
    }

    if !input.no_receipt_workstreams.is_empty() {
        debt.push(
            EvidenceDebt::new(
                EvidenceDebtSeverity::Warning,
                EvidenceDebtKind::MissingReceiptAnchors,
                format!(
                    "{} workstream(s) have no selected receipt anchors.",
                    input.no_receipt_workstreams.len()
                ),
            )
            .detail(workstream_title_sample(input.no_receipt_workstreams))
            .next_step(format!(
                "Run `shiplog workstreams receipts --run {} --workstream <title>`.",
                input.run_id
            )),
        );
    }

    if !input.broad_workstreams.is_empty() {
        debt.push(
            EvidenceDebt::new(
                EvidenceDebtSeverity::Info,
                EvidenceDebtKind::BroadWorkstream,
                format!(
                    "{} workstream(s) have 10+ events; consider splitting broad buckets.",
                    input.broad_workstreams.len()
                ),
            )
            .detail(workstream_title_sample(input.broad_workstreams))
            .next_step(format!(
                "Run `shiplog workstreams split --run {} --from <title> --to \"<new workstream>\" --matching \"<pattern>\" --create`.",
                input.run_id
            )),
        );
    }

    if !input.validation_errors.is_empty() {
        debt.push(
            EvidenceDebt::new(
                EvidenceDebtSeverity::Blocking,
                EvidenceDebtKind::WorkstreamValidation,
                "Workstream validation needs attention before rendering.",
            )
            .detail(format!(
                "{} validation issue(s) found across {} workstream(s).",
                input.validation_errors.len(),
                input.workstreams.workstreams.len()
            ))
            .next_step(format!(
                "Run `shiplog workstreams validate --run {}`.",
                input.run_id
            )),
        );
    }

    let assigned_events: usize = input
        .workstreams
        .workstreams
        .iter()
        .map(|workstream| workstream.events.len())
        .sum();
    if assigned_events == 0 && !input.events.is_empty() {
        debt.push(EvidenceDebt::new(
            EvidenceDebtSeverity::Blocking,
            EvidenceDebtKind::WorkstreamValidation,
            "Ledger has events but no workstream assignments.",
        ));
    }

    debt
}

fn print_evidence_debt(debt: &[EvidenceDebt]) {
    println!("Evidence debt:");
    if debt.is_empty() {
        println!("- No obvious evidence debt detected.");
        return;
    }

    for item in debt {
        println!(
            "- [{}] {}: {}",
            item.severity.label(),
            item.kind.label(),
            item.summary
        );
        if let Some(detail) = &item.detail {
            println!("  Detail: {detail}");
        }
        if let Some(next_step) = &item.next_step {
            println!("  Next: {next_step}");
        }
    }
}

fn workstream_title_sample(workstreams: &[&Workstream]) -> String {
    let mut titles = workstreams
        .iter()
        .take(3)
        .map(|workstream| workstream.title.as_str())
        .collect::<Vec<_>>();
    titles.sort_unstable();
    let mut detail = format!("Examples: {}", titles.join(", "));
    if workstreams.len() > titles.len() {
        detail.push_str(&format!("; and {} more", workstreams.len() - titles.len()));
    }
    detail
}

fn print_review_next_steps(
    run_id: &str,
    has_validation_errors: bool,
    first_no_receipt_workstream: Option<&str>,
    first_broad_workstream: Option<&str>,
    first_manual_context_workstream: Option<&str>,
    has_skipped_sources: bool,
) {
    println!("Next:");
    let mut step = 1usize;

    if has_validation_errors {
        println!("{step}. shiplog workstreams validate --run {run_id}");
        step += 1;
    }
    if let Some(title) = first_no_receipt_workstream {
        println!(
            "{step}. shiplog workstreams receipts --run {run_id} --workstream {}",
            quote_cli_value(title)
        );
        step += 1;
    }
    if let Some(title) = first_broad_workstream {
        println!(
            "{step}. shiplog workstreams split --run {run_id} --from {} --to \"<new workstream>\" --matching \"<pattern>\" --create",
            quote_cli_value(title)
        );
        step += 1;
    }
    if let Some(title) = first_manual_context_workstream {
        println!("{step}. {}", journal_add_next_step(title));
        step += 1;
    }
    if has_skipped_sources {
        println!("{step}. shiplog doctor");
        step += 1;
    }

    println!("{step}. shiplog render --run {run_id} --mode scaffold");
}

fn journal_add_next_step(workstream_title: &str) -> String {
    format!(
        "shiplog journal add --date {} --title {} --workstream {}",
        Utc::now().date_naive(),
        quote_cli_value(&format!("Outcome note for {workstream_title}")),
        quote_cli_value(workstream_title)
    )
}

fn quote_cli_value(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\\\""))
}

fn review_source_event_counts(
    manifest_sources: &[String],
    events: &[EventEnvelope],
    skipped_sources: &[ConfiguredSourceSkip],
) -> Vec<(String, usize)> {
    let mut ordered_sources = Vec::new();
    for source in manifest_sources {
        if !skipped_sources
            .iter()
            .any(|skipped| sources_match(&skipped.source, source))
        {
            push_review_source(&mut ordered_sources, source);
        }
    }
    for event in events {
        push_review_source(&mut ordered_sources, event.source.system.as_str());
    }

    ordered_sources
        .into_iter()
        .filter_map(|source| {
            let count = source_event_count_for_review(events, &source);
            (count > 0).then_some((source, count))
        })
        .collect()
}

fn push_review_source(sources: &mut Vec<String>, candidate: &str) {
    if sources
        .iter()
        .any(|source| sources_match(source, candidate))
    {
        return;
    }

    sources.push(candidate.to_string());
}

fn source_event_count_for_review(events: &[EventEnvelope], source: &str) -> usize {
    events
        .iter()
        .filter(|event| sources_match(event.source.system.as_str(), source))
        .count()
}

fn configured_source_skips(warnings: &[String]) -> Vec<ConfiguredSourceSkip> {
    warnings
        .iter()
        .filter_map(|warning| configured_source_skip(warning))
        .collect()
}

fn configured_source_skip(warning: &str) -> Option<ConfiguredSourceSkip> {
    const PREFIX: &str = "Configured source ";
    const INFIX: &str = " was skipped: ";

    let rest = warning.strip_prefix(PREFIX)?;
    let (source, reason) = rest.split_once(INFIX)?;
    Some(ConfiguredSourceSkip {
        source: source.to_string(),
        reason: reason.to_string(),
    })
}

fn sources_match(left: &str, right: &str) -> bool {
    normalized_source_key(left) == normalized_source_key(right)
}

fn normalized_source_key(source: &str) -> String {
    match source
        .trim()
        .to_ascii_lowercase()
        .replace('-', "_")
        .as_str()
    {
        "json_import" | "jsonimport" => "json".to_string(),
        "local_git" | "localgit" => "git".to_string(),
        other => other.to_string(),
    }
}

fn display_source_label(source: &str) -> String {
    match normalized_source_key(source).as_str() {
        "github" => "GitHub".to_string(),
        "gitlab" => "GitLab".to_string(),
        "jira" => "Jira".to_string(),
        "linear" => "Linear".to_string(),
        "manual" => "Manual".to_string(),
        "json" => "JSON".to_string(),
        "git" => "Local git".to_string(),
        "unknown" => "Unknown".to_string(),
        other => other.to_string(),
    }
}

fn source_list_label(sources: &[String]) -> String {
    if sources.is_empty() {
        "-".to_string()
    } else {
        sources.join(", ")
    }
}

fn open_existing_path(path: &Path, label: &str, next_step: &str, print_path: bool) -> Result<()> {
    if !path.exists() {
        anyhow::bail!("{label} not found: {}. {next_step}", path.display());
    }

    open_or_print_path(path, print_path)
}

fn open_or_print_path(path: &Path, print_path: bool) -> Result<()> {
    let display_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

    if print_path {
        println!("{}", display_path.display());
        return Ok(());
    }

    if try_open_path(&display_path) {
        println!("Opened: {}", display_path.display());
    } else {
        println!("{}", display_path.display());
    }

    Ok(())
}

fn try_open_path(path: &Path) -> bool {
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = std::process::Command::new("explorer.exe");
        command.arg(path);
        command
    };

    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = std::process::Command::new("open");
        command.arg(path);
        command
    };

    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut command = std::process::Command::new("xdg-open");
        command.arg(path);
        command
    };

    #[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
    {
        let _ = path;
        return false;
    }

    #[cfg(any(target_os = "windows", target_os = "macos", unix))]
    {
        command.stdin(std::process::Stdio::null());
        command.stdout(std::process::Stdio::null());
        command.stderr(std::process::Stdio::null());
        command
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
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
    fn cache_db_targets_default_to_known_api_cache_files() {
        let root = Path::new("C:/tmp/shiplog-out/.cache");
        let targets = cache_db_targets(root, &[]);
        let files: Vec<_> = targets
            .iter()
            .map(|target| {
                target
                    .path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            })
            .collect();

        assert_eq!(
            files,
            vec![
                "github-api-cache.db",
                "gitlab-api-cache.db",
                "jira-api-cache.db",
                "linear-api-cache.db",
            ]
        );
    }

    #[test]
    fn selected_cache_sources_preserve_order_and_dedupe() {
        let selected = selected_cache_sources(&[
            CacheSource::Jira,
            CacheSource::Github,
            CacheSource::Jira,
            CacheSource::Linear,
        ]);

        assert_eq!(
            selected,
            vec![CacheSource::Jira, CacheSource::Github, CacheSource::Linear]
        );
    }

    #[test]
    fn parse_cache_age_accepts_days_hours_and_minutes() {
        assert_eq!(parse_cache_age("30d").unwrap(), Duration::days(30));
        assert_eq!(parse_cache_age("12h").unwrap(), Duration::hours(12));
        assert_eq!(parse_cache_age("90m").unwrap(), Duration::minutes(90));
    }

    #[test]
    fn parse_cache_age_rejects_invalid_values() {
        assert!(parse_cache_age("30").is_err());
        assert!(parse_cache_age("-1d").is_err());
        assert!(parse_cache_age("1w").is_err());
    }

    #[test]
    fn cache_clean_mode_rejects_all_with_older_than() {
        let args = CacheCleanArgs {
            out: PathBuf::from("./out"),
            cache_dir: None,
            sources: Vec::new(),
            older_than: Some("30d".to_string()),
            all: true,
            dry_run: false,
            yes: true,
        };

        let err = cache_clean_mode(&args).unwrap_err();
        assert!(err.to_string().contains("either --all or --older-than"));
    }

    #[test]
    fn render_options_use_profile_specific_packet_defaults() {
        let internal = cli_render_options(
            RenderPacketMode::Packet,
            None,
            None,
            &BundleProfile::Internal,
        );
        assert_eq!(internal.receipt_limit, WORKSTREAM_RECEIPT_RENDER_LIMIT);
        assert_eq!(internal.appendix_mode, AppendixMode::Summary);

        let manager = cli_render_options(
            RenderPacketMode::Packet,
            None,
            None,
            &BundleProfile::Manager,
        );
        assert_eq!(manager.receipt_limit, MANAGER_RECEIPT_RENDER_LIMIT);
        assert_eq!(manager.appendix_mode, AppendixMode::Summary);

        let public =
            cli_render_options(RenderPacketMode::Packet, None, None, &BundleProfile::Public);
        assert_eq!(public.receipt_limit, PUBLIC_RECEIPT_RENDER_LIMIT);
        assert_eq!(public.appendix_mode, AppendixMode::None);
    }

    #[test]
    fn render_options_keep_explicit_receipt_and_appendix_overrides() {
        let options = cli_render_options(
            RenderPacketMode::Packet,
            Some(7),
            Some(RenderAppendixMode::Full),
            &BundleProfile::Public,
        );

        assert_eq!(options.receipt_limit, 7);
        assert_eq!(options.appendix_mode, AppendixMode::Full);
    }

    #[test]
    fn receipts_mode_keeps_audit_defaults_for_share_profiles() {
        let options = cli_render_options(
            RenderPacketMode::Receipts,
            None,
            None,
            &BundleProfile::Public,
        );

        assert_eq!(options.receipt_limit, WORKSTREAM_RECEIPT_RENDER_LIMIT);
        assert_eq!(options.appendix_mode, AppendixMode::Full);
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
    fn normalize_jira_instance_accepts_host_and_url() {
        assert_eq!(
            normalize_jira_instance("company.atlassian.net").unwrap(),
            "company.atlassian.net"
        );
        assert_eq!(
            normalize_jira_instance("https://company.atlassian.net").unwrap(),
            "company.atlassian.net"
        );
    }

    #[test]
    fn jira_api_url_uses_myself_endpoint_base() {
        assert_eq!(
            jira_api_url("https://company.atlassian.net", "/myself").unwrap(),
            "https://company.atlassian.net/rest/api/3/myself"
        );
    }

    #[test]
    fn linear_graphql_url_matches_adapter_endpoint() {
        assert_eq!(linear_graphql_url(), "https://api.linear.app/graphql");
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
