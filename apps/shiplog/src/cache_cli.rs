//! Cache CLI argument types and handlers.
//!
//! Keeps cache command parsing, database targeting, cleaning, and inspection
//! logic out of the top-level CLI entrypoint.

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use clap::{Args, Subcommand, ValueEnum};
use shiplog::cache::ApiCache;
use std::path::{Path, PathBuf};

#[derive(Subcommand, Debug)]
pub(crate) enum CacheCommand {
    /// Show cache entry counts and size.
    Stats(CacheArgs),

    /// Show cache entry counts plus timestamp bounds.
    Inspect(CacheArgs),

    /// Remove expired, old, or all cache entries without deleting outputs.
    Clean(CacheCleanArgs),
}

#[derive(Args, Debug)]
pub(crate) struct CacheArgs {
    /// Output directory whose `.cache` directory should be inspected.
    #[arg(long, default_value = "./out")]
    pub(crate) out: PathBuf,
    /// Cache directory to inspect instead of `<out>/.cache`.
    #[arg(long)]
    pub(crate) cache_dir: Option<PathBuf>,
    /// Limit to one or more source caches.
    #[arg(long = "source", value_enum)]
    pub(crate) sources: Vec<CacheSource>,
}

#[derive(Args, Debug)]
pub(crate) struct CacheCleanArgs {
    /// Output directory whose `.cache` directory should be cleaned.
    #[arg(long, default_value = "./out")]
    pub(crate) out: PathBuf,
    /// Cache directory to clean instead of `<out>/.cache`.
    #[arg(long)]
    pub(crate) cache_dir: Option<PathBuf>,
    /// Limit to one or more source caches.
    #[arg(long = "source", value_enum)]
    pub(crate) sources: Vec<CacheSource>,
    /// Remove entries cached before this age, such as 30d, 12h, or 90m.
    #[arg(long)]
    pub(crate) older_than: Option<String>,
    /// Remove every entry in the selected caches. Requires --yes unless --dry-run is set.
    #[arg(long)]
    pub(crate) all: bool,
    /// Print what would be removed without modifying cache databases.
    #[arg(long)]
    pub(crate) dry_run: bool,
    /// Confirm destructive --all cleanup.
    #[arg(long)]
    pub(crate) yes: bool,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CacheSource {
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

#[derive(Debug, Clone)]
pub(crate) struct CacheDbTarget {
    pub(crate) source: CacheSource,
    pub(crate) path: PathBuf,
}

#[derive(Debug)]
pub(crate) enum CacheCleanMode {
    Expired,
    OlderThan(DateTime<Utc>),
    All,
}

pub(crate) fn run_cache_stats(args: CacheArgs) -> Result<()> {
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

pub(crate) fn run_cache_inspect(args: CacheArgs) -> Result<()> {
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

pub(crate) fn run_cache_clean(args: CacheCleanArgs) -> Result<()> {
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

pub(crate) fn cache_db_targets(root: &Path, sources: &[CacheSource]) -> Vec<CacheDbTarget> {
    selected_cache_sources(sources)
        .into_iter()
        .map(|source| CacheDbTarget {
            source,
            path: root.join(source.db_filename()),
        })
        .collect()
}

pub(crate) fn selected_cache_sources(sources: &[CacheSource]) -> Vec<CacheSource> {
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

fn print_cache_stats(source: CacheSource, path: &Path, stats: &shiplog::cache::CacheStats) {
    println!("{}:", source.as_str());
    println!("  path: {}", path.display());
    println!(
        "  entries: total {}, valid {}, expired {}",
        stats.total_entries, stats.valid_entries, stats.expired_entries
    );
    println!("  size: {} MB", stats.cache_size_mb);
}

pub(crate) fn cache_clean_mode(args: &CacheCleanArgs) -> Result<CacheCleanMode> {
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

pub(crate) fn parse_cache_age(value: &str) -> Result<Duration> {
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
