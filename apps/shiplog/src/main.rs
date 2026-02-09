use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use chrono::NaiveDate;
use shiplog_engine::Engine;
use shiplog_ingest_github::GithubIngestor;
use shiplog_ingest_json::JsonIngestor;
use shiplog_redact::DeterministicRedactor;
use shiplog_render_md::MarkdownRenderer;
use shiplog_workstreams::RepoClusterer;
use shiplog_ports::Ingestor;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "shiplog")]
#[command(about = "Generate self-review packets with receipts + coverage.", long_about = None)]
struct Cli {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run an ingestion + render pipeline.
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
    },
}

#[derive(Subcommand, Debug)]
enum Source {
    /// Ingest from GitHub (public + authenticated private).
    Github {
        /// GitHub login to report on.
        #[arg(long)]
        user: String,
        /// Start date (inclusive), YYYY-MM-DD
        #[arg(long)]
        since: NaiveDate,
        /// End date (exclusive), YYYY-MM-DD
        #[arg(long)]
        until: NaiveDate,
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
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.cmd {
        Command::Run { source, out, zip, redact_key } => {
            let key = redact_key
                .or_else(|| std::env::var("SHIPLOG_REDACT_KEY").ok())
                .unwrap_or_else(|| {
                    eprintln!("WARN: no redaction key provided; using a default dev key. Don't share public packets like this.");
                    "dev-key".to_string()
                });

            let renderer = MarkdownRenderer;
            let clusterer = RepoClusterer;
            let redactor = DeterministicRedactor::new(key.as_bytes());
            let engine = Engine::new(&renderer, &clusterer, &redactor);

            match source {
                Source::Github { user, since, until, mode, include_reviews, no_details, throttle_ms, token, api_base } => {
                    let token = token.or_else(|| std::env::var("GITHUB_TOKEN").ok());

                    let mut ing = GithubIngestor::new(user.clone(), since, until);
                    ing.mode = mode;
                    ing.include_reviews = include_reviews;
                    ing.fetch_details = !no_details;
                    ing.throttle_ms = throttle_ms;
                    ing.token = token;
                    ing.api_base = api_base;

                    let ingest = ing.ingest()?;
                    let run_id = ingest.coverage.run_id.to_string();
                    let run_dir = out.join(run_id);

                    let window_label = format!("{}..{}", since, until);
                    let outputs = engine.run(ingest, &user, &window_label, &run_dir, zip)?;

                    println!("wrote:");
                    println!("- {}", outputs.packet_md.display());
                    println!("- {}", outputs.workstreams_yaml.display());
                    println!("- {}", outputs.ledger_events_jsonl.display());
                    println!("- {}", outputs.coverage_manifest_json.display());
                    println!("- {}", outputs.bundle_manifest_json.display());
                    if let Some(z) = outputs.zip_path {
                        println!("- {}", z.display());
                    }
                }

                Source::Json { events, coverage, user, window_label } => {
                    let ing = JsonIngestor { events_path: events, coverage_path: coverage };
                    let ingest = ing.ingest()?;
                    let run_id = ingest.coverage.run_id.to_string();
                    let run_dir = out.join(run_id);
                    let outputs = engine.run(ingest, &user, &window_label, &run_dir, zip)?;

                    println!("wrote:");
                    println!("- {}", outputs.packet_md.display());
                }
            }
        }
    }

    Ok(())
}
