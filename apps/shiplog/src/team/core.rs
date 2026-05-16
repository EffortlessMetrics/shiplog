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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use std::fs;
    use tempfile::tempdir;

    fn d(s: &str) -> Result<NaiveDate> {
        NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map_err(|e| anyhow::anyhow!("invalid date {s:?}: {e}"))
    }

    #[test]
    fn load_full_yaml_parses_all_fields() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path().join("team.yaml");
        fs::write(
            &path,
            "members:\n  - alice\n  - bob\naliases:\n  alice: Alice A.\nsections:\n  - summary\n  - workstreams\ntemplate: tmpl.md\nsince: 2026-01-01\nuntil: 2026-02-01\nrequired_schema_version: \"1.0\"\n",
        )?;

        let cfg = TeamConfig::load(&path)?;
        assert_eq!(cfg.members, vec!["alice", "bob"]);
        assert_eq!(
            cfg.aliases.get("alice").map(String::as_str),
            Some("Alice A.")
        );
        assert_eq!(cfg.sections, vec!["summary", "workstreams"]);
        assert_eq!(cfg.template, Some(PathBuf::from("tmpl.md")));
        assert_eq!(cfg.since, Some(d("2026-01-01")?));
        assert_eq!(cfg.until, Some(d("2026-02-01")?));
        assert_eq!(cfg.required_schema_version.as_deref(), Some("1.0"));
        Ok(())
    }

    #[test]
    fn load_minimal_yaml_uses_defaults_for_missing_fields() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path().join("team.yaml");
        fs::write(&path, "members: []\n")?;

        let cfg = TeamConfig::load(&path)?;
        assert!(cfg.members.is_empty());
        assert!(cfg.aliases.is_empty());
        assert!(cfg.sections.is_empty());
        assert!(cfg.template.is_none());
        assert!(cfg.since.is_none());
        assert!(cfg.until.is_none());
        assert!(cfg.required_schema_version.is_none());
        Ok(())
    }

    #[test]
    fn load_invalid_yaml_returns_error() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path().join("team.yaml");
        fs::write(&path, "members: [: bad yaml\n")?;

        let err = TeamConfig::load(&path).expect_err("should fail");
        let msg = format!("{err:#}");
        assert!(msg.contains("parse team config"), "msg was: {msg}");
        Ok(())
    }

    #[test]
    fn load_missing_file_returns_error() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path().join("does-not-exist.yaml");

        let err = TeamConfig::load(&path).expect_err("should fail");
        let msg = format!("{err:#}");
        assert!(msg.contains("read team config"), "msg was: {msg}");
        Ok(())
    }

    #[test]
    fn normalized_sections_empty_returns_defaults() {
        let cfg = TeamConfig::default();
        assert_eq!(
            cfg.normalized_sections(),
            vec!["summary", "workstreams", "coverage", "receipts"]
        );
    }

    fn cfg_with_sections(sections: Vec<String>) -> TeamConfig {
        TeamConfig {
            sections,
            ..Default::default()
        }
    }

    #[test]
    fn normalized_sections_single_item() {
        let cfg = cfg_with_sections(vec!["summary".into()]);
        assert_eq!(cfg.normalized_sections(), vec!["summary"]);
    }

    #[test]
    fn normalized_sections_multi_preserves_order() {
        let cfg = cfg_with_sections(vec!["receipts".into(), "summary".into(), "coverage".into()]);
        assert_eq!(
            cfg.normalized_sections(),
            vec!["receipts", "summary", "coverage"]
        );
    }

    #[test]
    fn normalized_sections_dedupes_preserving_first_seen() {
        let cfg = cfg_with_sections(vec![
            "summary".into(),
            "workstreams".into(),
            "summary".into(),
            "coverage".into(),
        ]);
        assert_eq!(
            cfg.normalized_sections(),
            vec!["summary", "workstreams", "coverage"]
        );
    }

    #[test]
    fn normalized_sections_lowercases_entries() {
        let cfg = cfg_with_sections(vec!["Summary".into(), "WORKSTREAMS".into()]);
        assert_eq!(cfg.normalized_sections(), vec!["summary", "workstreams"]);
    }

    #[test]
    fn normalized_sections_drops_empty_entries() {
        let cfg = cfg_with_sections(vec!["summary".into(), "".into(), "coverage".into()]);
        assert_eq!(cfg.normalized_sections(), vec!["summary", "coverage"]);
    }

    #[test]
    fn normalized_sections_all_whitespace_falls_back_to_defaults() {
        let cfg = cfg_with_sections(vec!["".into(), "   ".into(), "\t".into()]);
        assert_eq!(
            cfg.normalized_sections(),
            vec!["summary", "workstreams", "coverage", "receipts"]
        );
    }

    #[test]
    fn section_enabled_true_for_default_when_empty() {
        let cfg = TeamConfig::default();
        assert!(cfg.section_enabled("summary"));
        assert!(cfg.section_enabled("workstreams"));
        assert!(cfg.section_enabled("coverage"));
        assert!(cfg.section_enabled("receipts"));
    }

    #[test]
    fn section_enabled_false_for_unknown_section() {
        let cfg = TeamConfig::default();
        assert!(!cfg.section_enabled("nope"));
    }

    #[test]
    fn section_enabled_true_after_explicit_list() {
        let cfg = cfg_with_sections(vec!["Summary".into()]);
        assert!(cfg.section_enabled("summary"));
        assert!(!cfg.section_enabled("workstreams"));
    }

    #[test]
    fn parse_csv_list_empty_string_returns_empty() {
        assert!(parse_csv_list("").is_empty());
    }

    #[test]
    fn parse_csv_list_single_item() {
        assert_eq!(parse_csv_list("alice"), vec!["alice"]);
    }

    #[test]
    fn parse_csv_list_multi_with_trim_and_dedup() {
        assert_eq!(
            parse_csv_list(" alice , bob ,alice, carol "),
            vec!["alice", "bob", "carol"]
        );
    }

    #[test]
    fn parse_csv_list_is_case_sensitive() {
        assert_eq!(parse_csv_list("Alice,alice"), vec!["Alice", "alice"]);
    }

    #[test]
    fn parse_csv_list_drops_empty_entries() {
        assert_eq!(
            parse_csv_list("alice,,bob, ,carol"),
            vec!["alice", "bob", "carol"]
        );
    }

    #[test]
    fn parse_alias_list_empty_input_returns_empty_map() -> Result<()> {
        let aliases = parse_alias_list(&[])?;
        assert!(aliases.is_empty());
        Ok(())
    }

    #[test]
    fn parse_alias_list_single_entry() -> Result<()> {
        let aliases = parse_alias_list(&["alice=Alice A.".to_string()])?;
        assert_eq!(aliases.len(), 1);
        assert_eq!(aliases.get("alice").map(String::as_str), Some("Alice A."));
        Ok(())
    }

    #[test]
    fn parse_alias_list_multi_entries_and_trims_around_equals() -> Result<()> {
        let aliases = parse_alias_list(&[
            "  alice  =  Alice A.  ".to_string(),
            "bob=Bob B.".to_string(),
        ])?;
        assert_eq!(aliases.get("alice").map(String::as_str), Some("Alice A."));
        assert_eq!(aliases.get("bob").map(String::as_str), Some("Bob B."));
        Ok(())
    }

    #[test]
    fn parse_alias_list_errors_when_member_empty() {
        let err = parse_alias_list(&["=Alice".to_string()]).expect_err("should fail");
        let msg = format!("{err}");
        assert!(
            msg.contains("expected member=Display Name"),
            "msg was: {msg}"
        );
    }

    #[test]
    fn parse_alias_list_errors_when_display_empty() {
        let err = parse_alias_list(&["alice=".to_string()]).expect_err("should fail");
        let msg = format!("{err}");
        assert!(
            msg.contains("display name cannot be empty"),
            "msg was: {msg}"
        );
    }

    #[test]
    fn resolve_team_config_no_config_file_returns_defaults() -> Result<()> {
        let cfg = resolve_team_config(None, None, None, None, None, None, None, vec![])?;
        assert!(cfg.members.is_empty());
        assert!(cfg.sections.is_empty());
        assert!(cfg.aliases.is_empty());
        assert!(cfg.since.is_none());
        assert!(cfg.until.is_none());
        Ok(())
    }

    #[test]
    fn resolve_team_config_cli_overrides_members_and_sections_and_dates() -> Result<()> {
        let since = d("2026-01-01")?;
        let until = d("2026-02-01")?;
        let cfg = resolve_team_config(
            None,
            Some("alice,bob".into()),
            Some(since),
            Some(until),
            Some("Summary,Coverage".into()),
            Some(PathBuf::from("tmpl.md")),
            Some("1.0".into()),
            vec![],
        )?;
        assert_eq!(cfg.members, vec!["alice", "bob"]);
        assert_eq!(cfg.sections, vec!["Summary", "Coverage"]);
        assert_eq!(cfg.normalized_sections(), vec!["summary", "coverage"]);
        assert_eq!(cfg.since, Some(since));
        assert_eq!(cfg.until, Some(until));
        assert_eq!(cfg.template, Some(PathBuf::from("tmpl.md")));
        assert_eq!(cfg.required_schema_version.as_deref(), Some("1.0"));
        Ok(())
    }

    #[test]
    fn resolve_team_config_errors_when_until_equals_since() -> Result<()> {
        let date = d("2026-01-01")?;
        let err = resolve_team_config(None, None, Some(date), Some(date), None, None, None, vec![])
            .expect_err("should fail");
        let msg = format!("{err}");
        assert!(msg.contains("Invalid date range"), "msg was: {msg}");
        Ok(())
    }

    #[test]
    fn resolve_team_config_errors_when_until_before_since() -> Result<()> {
        let since = d("2026-02-01")?;
        let until = d("2026-01-15")?;
        let err = resolve_team_config(
            None,
            None,
            Some(since),
            Some(until),
            None,
            None,
            None,
            vec![],
        )
        .expect_err("should fail");
        let msg = format!("{err}");
        assert!(msg.contains("Invalid date range"), "msg was: {msg}");
        Ok(())
    }

    #[test]
    fn resolve_team_config_ok_when_until_after_since() -> Result<()> {
        let since = d("2026-01-01")?;
        let until = d("2026-01-02")?;
        let cfg = resolve_team_config(
            None,
            None,
            Some(since),
            Some(until),
            None,
            None,
            None,
            vec![],
        )?;
        assert_eq!(cfg.since, Some(since));
        assert_eq!(cfg.until, Some(until));
        Ok(())
    }

    #[test]
    fn resolve_team_config_merges_config_file_aliases_with_cli_aliases() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path().join("team.yaml");
        fs::write(
            &path,
            "members:\n  - alice\naliases:\n  alice: Alice A.\n  bob: Bob B.\n",
        )?;

        let cfg = resolve_team_config(
            Some(path),
            None,
            None,
            None,
            None,
            None,
            None,
            vec!["bob=Robert B.".to_string(), "carol=Carol C.".to_string()],
        )?;

        // bob is overridden by CLI; alice persists from config; carol is added.
        assert_eq!(
            cfg.aliases.get("alice").map(String::as_str),
            Some("Alice A.")
        );
        assert_eq!(
            cfg.aliases.get("bob").map(String::as_str),
            Some("Robert B.")
        );
        assert_eq!(
            cfg.aliases.get("carol").map(String::as_str),
            Some("Carol C.")
        );
        assert_eq!(cfg.members, vec!["alice"]);
        Ok(())
    }
}
