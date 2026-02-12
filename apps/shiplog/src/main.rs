use anyhow::Result;
use chrono::NaiveDate;
use clap::{Parser, Subcommand};
use shiplog_engine::{Engine, WorkstreamSource};
use shiplog_ingest_github::GithubIngestor;
use shiplog_ingest_json::JsonIngestor;
use shiplog_ports::Ingestor;
use shiplog_redact::DeterministicRedactor;
use shiplog_render_md::MarkdownRenderer;
use shiplog_workstreams::RepoClusterer;
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
        /// Regenerate workstreams even if workstreams.yaml exists.
        /// WARNING: This will not overwrite workstreams.yaml, but will
        /// regenerate workstreams.suggested.yaml.
        #[arg(long)]
        regen: bool,
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
        /// User label for rendering.
        #[arg(long, default_value = "user")]
        user: String,
        /// Window label for rendering.
        #[arg(long, default_value = "window")]
        window_label: String,
        /// Redaction key. If omitted, SHIPLOG_REDACT_KEY is used.
        #[arg(long)]
        redact_key: Option<String>,
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
        /// Also write a zip next to the run folder.
        #[arg(long)]
        zip: bool,
        /// Redaction key. If omitted, SHIPLOG_REDACT_KEY is used.
        #[arg(long)]
        redact_key: Option<String>,
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
    },
}

#[derive(Subcommand, Debug, Clone)]
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

fn get_redact_key(redact_key: Option<String>) -> String {
    redact_key
        .or_else(|| std::env::var("SHIPLOG_REDACT_KEY").ok())
        .unwrap_or_else(|| {
            eprintln!("WARN: no redaction key provided; using a default dev key. Don't share public packets like this.");
            "dev-key".to_string()
        })
}

fn create_engine(redact_key: &str) -> Engine<'static> {
    let renderer = MarkdownRenderer;
    let clusterer = RepoClusterer;
    let redactor = DeterministicRedactor::new(redact_key.as_bytes());

    // We need to leak these to give them 'static lifetime
    // This is acceptable for a CLI tool that runs once
    let renderer: &'static dyn shiplog_ports::Renderer = Box::leak(Box::new(renderer));
    let clusterer: &'static dyn shiplog_ports::WorkstreamClusterer = Box::leak(Box::new(clusterer));
    let redactor: &'static dyn shiplog_ports::Redactor = Box::leak(Box::new(redactor));

    Engine::new(renderer, clusterer, redactor)
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
) -> GithubIngestor {
    let token = token.or_else(|| std::env::var("GITHUB_TOKEN").ok());

    let mut ing = GithubIngestor::new(user.to_string(), since, until);
    ing.mode = mode.to_string();
    ing.include_reviews = include_reviews;
    ing.fetch_details = !no_details;
    ing.throttle_ms = throttle_ms;
    ing.token = token;
    ing.api_base = api_base.to_string();

    ing
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.cmd {
        Command::Collect {
            source,
            out,
            zip,
            redact_key,
            regen,
        } => {
            let key = get_redact_key(redact_key);
            let engine = create_engine(&key);

            match source {
                Source::Github {
                    user,
                    since,
                    until,
                    mode,
                    include_reviews,
                    no_details,
                    throttle_ms,
                    token,
                    api_base,
                } => {
                    let ing = make_github_ingestor(
                        &user,
                        since,
                        until,
                        &mode,
                        include_reviews,
                        no_details,
                        throttle_ms,
                        token,
                        &api_base,
                    );
                    let ingest = ing.ingest()?;
                    let run_id = ingest.coverage.run_id.to_string();
                    let run_dir = out.join(&run_id);

                    let window_label = format!("{}..{}", since, until);

                    // Check if user has curated workstreams and warn
                    if !regen && shiplog_workstreams::WorkstreamManager::has_curated(&run_dir) {
                        eprintln!("Note: Using existing workstreams.yaml (user-curated).");
                        eprintln!("      Use --regen to regenerate suggestions.");
                    }

                    let (outputs, ws_source) =
                        engine.run(ingest, &user, &window_label, &run_dir, zip)?;

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
                    let ingest = ing.ingest()?;
                    let run_id = ingest.coverage.run_id.to_string();
                    let run_dir = out.join(&run_id);

                    let (outputs, ws_source) =
                        engine.run(ingest, &user, &window_label, &run_dir, zip)?;

                    println!("Collected and wrote:");
                    print_outputs(&outputs, ws_source);
                }
            }
        }

        Command::Render {
            out,
            run,
            user,
            window_label,
            redact_key,
            zip,
        } => {
            let key = get_redact_key(redact_key);
            let engine = create_engine(&key);

            // Determine which run to render
            let run_dir = if let Some(run_id) = run {
                out.join(run_id)
            } else {
                // Find most recent run directory
                find_most_recent_run(&out)?
            };

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
            let ingest = ing.ingest()?;

            let outputs = engine.refresh(ingest, &user, &window_label, &run_dir, zip)?;

            println!("Rendered from existing events:");
            print_outputs(&outputs, WorkstreamSource::Curated);
        }

        Command::Refresh {
            source,
            out,
            zip,
            redact_key,
        } => {
            let key = get_redact_key(redact_key);
            let engine = create_engine(&key);

            match source {
                Source::Github {
                    user,
                    since,
                    until,
                    mode,
                    include_reviews,
                    no_details,
                    throttle_ms,
                    token,
                    api_base,
                } => {
                    let ing = make_github_ingestor(
                        &user,
                        since,
                        until,
                        &mode,
                        include_reviews,
                        no_details,
                        throttle_ms,
                        token,
                        &api_base,
                    );
                    let ingest = ing.ingest()?;
                    let run_id = ingest.coverage.run_id.to_string();
                    let run_dir = out.join(&run_id);

                    let window_label = format!("{}..{}", since, until);

                    if !shiplog_workstreams::WorkstreamManager::has_curated(&run_dir)
                        && !shiplog_workstreams::WorkstreamManager::suggested_path(&run_dir)
                            .exists()
                    {
                        anyhow::bail!(
                            "No workstreams found in {:?}. Run `shiplog collect` first.",
                            run_dir
                        );
                    }

                    let outputs = engine.refresh(ingest, &user, &window_label, &run_dir, zip)?;

                    println!("Refreshed while preserving workstream curation:");
                    print_outputs_simple(&outputs);
                }

                Source::Json {
                    events,
                    coverage,
                    user,
                    window_label,
                } => {
                    let run_dir = out.join("refresh_run");

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
                    let ingest = ing.ingest()?;

                    let outputs = engine.refresh(ingest, &user, &window_label, &run_dir, zip)?;

                    println!("Refreshed while preserving workstream curation:");
                    print_outputs_simple(&outputs);
                }
            }
        }

        Command::Run {
            source,
            out,
            zip,
            redact_key,
        } => {
            // Legacy mode: just do collect
            let key = get_redact_key(redact_key);
            let engine = create_engine(&key);

            match source {
                Source::Github {
                    user,
                    since,
                    until,
                    mode,
                    include_reviews,
                    no_details,
                    throttle_ms,
                    token,
                    api_base,
                } => {
                    let ing = make_github_ingestor(
                        &user,
                        since,
                        until,
                        &mode,
                        include_reviews,
                        no_details,
                        throttle_ms,
                        token,
                        &api_base,
                    );
                    let ingest = ing.ingest()?;
                    let run_id = ingest.coverage.run_id.to_string();
                    let run_dir = out.join(&run_id);

                    let window_label = format!("{}..{}", since, until);
                    let (outputs, ws_source) =
                        engine.run(ingest, &user, &window_label, &run_dir, zip)?;

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
                    let ingest = ing.ingest()?;
                    let run_id = ingest.coverage.run_id.to_string();
                    let run_dir = out.join(&run_id);
                    let (outputs, ws_source) =
                        engine.run(ingest, &user, &window_label, &run_dir, zip)?;

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

fn find_most_recent_run(out_dir: &PathBuf) -> Result<PathBuf> {
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
