use anyhow::{Context, Result};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_SECTIONS: &[&str] = &["summary", "workstreams", "coverage", "receipts"];

/// Team aggregation configuration.
///
/// Values in this struct are intended to be persisted in config files and passed
/// across team boundaries.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TeamConfig {
    /// Team member IDs. CLI `--members` maps into this field.
    pub members: Vec<String>,
    /// Optional alias map for consistent display names.
    #[serde(default)]
    pub aliases: HashMap<String, String>,
    /// Ordered list of rendered sections.
    /// Supported names:
    /// - summary
    /// - workstreams
    /// - coverage
    /// - receipts
    #[serde(default)]
    pub sections: Vec<String>,
    /// Optional custom template used for final packet rendering.
    #[serde(default)]
    pub template: Option<PathBuf>,
    /// Optional date range filter (inclusive start, exclusive end).
    #[serde(default)]
    pub since: Option<NaiveDate>,
    #[serde(default)]
    pub until: Option<NaiveDate>,
    /// Optional schema compatibility gate.
    /// If present and member coverage has `schema_version`, incompatibility
    /// causes the member to be skipped with a warning.
    #[serde(default)]
    pub required_schema_version: Option<String>,
}

impl TeamConfig {
    /// Load config from YAML.
    pub fn load(path: &Path) -> Result<Self> {
        let text =
            fs::read_to_string(path).with_context(|| format!("read team config {path:?}"))?;
        let cfg: Self =
            serde_yaml::from_str(&text).with_context(|| format!("parse team config {path:?}"))?;
        Ok(cfg)
    }

    /// Normalize requested sections into a deterministic, deduplicated list.
    pub fn normalized_sections(&self) -> Vec<String> {
        if self.sections.is_empty() {
            DEFAULT_SECTIONS.iter().map(|s| s.to_string()).collect()
        } else {
            let mut seen = HashSet::new();
            let mut out = Vec::new();
            for section in &self.sections {
                let section = section.trim().to_ascii_lowercase();
                if section.is_empty() {
                    continue;
                }
                if seen.insert(section.clone()) {
                    out.push(section);
                }
            }
            if out.is_empty() {
                DEFAULT_SECTIONS.iter().map(|s| s.to_string()).collect()
            } else {
                out
            }
        }
    }

    /// Check if a section is enabled after normalization and deduplication.
    pub fn section_enabled(&self, section: &str) -> bool {
        self.normalized_sections()
            .iter()
            .any(|value| value == section)
    }
}

/// Parse a comma-delimited CSV-like list into a stable, deduplicated vector.
pub fn parse_csv_list(raw: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut seen = HashSet::new();

    for raw_value in raw.split(',') {
        let value = raw_value.trim();
        if value.is_empty() {
            continue;
        }
        if seen.insert(value.to_string()) {
            values.push(value.to_string());
        }
    }
    values
}

/// Parse member alias entries from `member=Display Name` CLI values.
pub fn parse_alias_list(alias_args: &[String]) -> Result<HashMap<String, String>> {
    let mut aliases = HashMap::new();

    for entry in alias_args {
        let raw = entry.trim();
        let mut parts = raw.splitn(2, '=');
        let member = parts.next().unwrap_or_default().trim().to_string();
        let display = parts.next().unwrap_or_default().trim().to_string();

        if member.is_empty() {
            anyhow::bail!("Invalid alias '{raw}': expected member=Display Name");
        }
        if display.is_empty() {
            anyhow::bail!("Invalid alias '{raw}': display name cannot be empty");
        }

        aliases.insert(member, display);
    }

    Ok(aliases)
}

/// Resolve team command flags and optional config file into a normalized `TeamConfig`.
#[allow(clippy::too_many_arguments)]
pub fn resolve_team_config(
    config: Option<PathBuf>,
    members: Option<String>,
    since: Option<NaiveDate>,
    until: Option<NaiveDate>,
    sections: Option<String>,
    template: Option<PathBuf>,
    required_schema_version: Option<String>,
    alias: Vec<String>,
) -> Result<TeamConfig> {
    let mut cfg = match config {
        Some(path) => TeamConfig::load(&path)?,
        None => TeamConfig::default(),
    };

    if let Some(raw_members) = members {
        let parsed_members = parse_csv_list(&raw_members);
        if !parsed_members.is_empty() {
            cfg.members = parsed_members;
        }
    }

    if let Some(raw_sections) = sections {
        cfg.sections = parse_csv_list(&raw_sections);
    }

    if let Some(template) = template {
        cfg.template = Some(template);
    }

    if let Some(since) = since {
        cfg.since = Some(since);
    }

    if let Some(until) = until {
        cfg.until = Some(until);
    }

    if let Some(version) = required_schema_version {
        cfg.required_schema_version = Some(version);
    }

    let alias_entries = parse_alias_list(&alias)?;
    if !alias_entries.is_empty() {
        cfg.aliases.extend(alias_entries);
    }

    if let (Some(since), Some(until)) = (cfg.since, cfg.until)
        && until <= since
    {
        anyhow::bail!("Invalid date range: until ({until}) must be after since ({since})");
    }

    Ok(cfg)
}
