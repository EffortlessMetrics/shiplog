//! Team aggregation mode for generating team-level shipping summaries.
//!
//! The team crate keeps a narrow API boundary:
//! - Read multiple member ledgers from disk
//! - Merge them with the shared `shiplog-merge` pipeline
//! - Emit a deterministic, sectioned aggregate summary

use anyhow::{Context, Result, anyhow};
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use shiplog_merge::{ConflictResolution, merge_ingest_outputs};
use shiplog_ports::IngestOutput;
use shiplog_schema::coverage::{Completeness, CoverageManifest, TimeWindow};
use shiplog_schema::event::EventEnvelope;
use shiplog_template::{TemplateContext, TemplateEngine};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub use shiplog_team_core::{TeamConfig, parse_alias_list, parse_csv_list, resolve_team_config};

/// One member's aggregation outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMemberSummary {
    pub member: String,
    pub display_name: String,
    pub included: bool,
    pub ledger_dir: PathBuf,
    pub event_count: usize,
    pub warnings: Vec<String>,
}

/// Result of one team aggregation run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamAggregateResult {
    pub requested_members: Vec<String>,
    pub included_members: Vec<String>,
    pub missing_members: Vec<String>,
    pub incompatible_members: Vec<String>,
    pub members: Vec<TeamMemberSummary>,
    pub coverage: CoverageManifest,
    pub events: Vec<EventEnvelope>,
    pub warnings: Vec<String>,
}

/// Stable output artifact locations from team packet generation.
#[derive(Debug, Clone)]
pub struct TeamOutputFiles {
    pub packet: PathBuf,
    pub events: PathBuf,
    pub coverage: PathBuf,
}

/// Build team summaries from multiple member ledgers.
#[derive(Debug)]
pub struct TeamAggregator {
    config: TeamConfig,
}

impl TeamAggregator {
    /// Create an aggregator with the given config.
    pub fn new(config: TeamConfig) -> Self {
        Self { config }
    }

    /// Aggregate every configured member into one output packet.
    pub fn aggregate(&self, member_root: &Path) -> Result<TeamAggregateResult> {
        if self.config.members.is_empty() {
            return Err(anyhow!("No team members configured"));
        }

        let mut requested_members = self.config.members.clone();
        requested_members.sort();
        requested_members.dedup();

        let mut member_outputs = Vec::new();
        let mut member_summaries = Vec::new();
        let mut included_members = Vec::new();
        let mut missing_members = Vec::new();
        let mut incompatible_members = Vec::new();
        let mut warnings = Vec::new();

        for member in &requested_members {
            let alias = self
                .config
                .aliases
                .get(member)
                .cloned()
                .unwrap_or_else(|| member.clone());
            let ledger_dir = member_root.join(member);

            match load_member_output(&ledger_dir, self.config.since, self.config.until) {
                Ok(Some((output, member_warnings))) => {
                    if is_compatible_schema(&ledger_dir, &self.config.required_schema_version)? {
                        included_members.push(member.clone());
                        let warning_count = member_warnings.len();
                        if warning_count > 0 {
                            warnings.extend(member_warnings.iter().cloned());
                        }
                        member_summaries.push(TeamMemberSummary {
                            member: member.clone(),
                            display_name: alias,
                            included: true,
                            ledger_dir,
                            event_count: output.events.len(),
                            warnings: member_warnings,
                        });
                        member_outputs.push(output);
                    } else {
                        let warning =
                            format!("Ledger for '{member}' skipped due to schema incompatibility");
                        incompatible_members.push(member.clone());
                        warnings.push(warning.clone());
                        member_summaries.push(TeamMemberSummary {
                            member: member.clone(),
                            display_name: alias,
                            included: false,
                            ledger_dir,
                            event_count: 0,
                            warnings: vec![warning],
                        });
                    }
                }
                Ok(None) => {
                    let warning = format!(
                        "Missing ledger or coverage for member '{member}' at {}",
                        ledger_dir.display()
                    );
                    missing_members.push(member.clone());
                    warnings.push(warning.clone());
                    member_summaries.push(TeamMemberSummary {
                        member: member.clone(),
                        display_name: alias,
                        included: false,
                        ledger_dir,
                        event_count: 0,
                        warnings: vec![warning],
                    });
                }
                Err(err) => {
                    let warning = format!(
                        "Failed reading ledger for '{member}' from {}: {err}",
                        ledger_dir.display()
                    );
                    missing_members.push(member.clone());
                    warnings.push(warning.clone());
                    member_summaries.push(TeamMemberSummary {
                        member: member.clone(),
                        display_name: alias,
                        included: false,
                        ledger_dir,
                        event_count: 0,
                        warnings: vec![warning],
                    });
                }
            }
        }

        let (coverage, events) = if member_outputs.is_empty() {
            (
                empty_coverage(
                    self.config.since,
                    self.config.until,
                    requested_members.clone(),
                    warnings.clone(),
                ),
                Vec::new(),
            )
        } else {
            let merged =
                merge_ingest_outputs(&member_outputs, ConflictResolution::PreferMostRecent)?;
            let mut coverage = merged.ingest_output.coverage;
            if let Some(since) = self.config.since {
                coverage.window.since = since;
            }
            if let Some(until) = self.config.until {
                coverage.window.until = until;
            }
            coverage.sources = requested_members.clone();
            coverage.warnings.extend(warnings.clone());
            (coverage, merged.ingest_output.events)
        };

        if self.config.required_schema_version.is_some()
            && member_outputs.is_empty()
            && !warnings.is_empty()
        {
            // All members were skipped for schema incompatibility — warnings already recorded.
        }

        Ok(TeamAggregateResult {
            requested_members,
            included_members,
            missing_members,
            incompatible_members,
            members: member_summaries,
            coverage,
            events,
            warnings,
        })
    }

    /// Render a markdown packet from aggregate output.
    pub fn render_packet_markdown(&self, output: &TeamAggregateResult) -> Result<String> {
        if let Some(template) = &self.config.template {
            let text = fs::read_to_string(template)
                .with_context(|| format!("read team template {:?}", template))?;
            return render_with_template(&text, &self.config, output);
        }

        Ok(render_default_packet(&self.config, output))
    }
}

/// Persist team outputs in the current directory layout used by the shiplog app.
pub fn write_team_outputs(
    out_dir: &Path,
    packet_markdown: &str,
    result: &TeamAggregateResult,
) -> Result<TeamOutputFiles> {
    std::fs::create_dir_all(out_dir)
        .with_context(|| format!("create output directory {out_dir:?}"))?;

    let packet_path = out_dir.join("team.packet.md");
    let events_path = out_dir.join("team.ledger.events.jsonl");
    let coverage_path = out_dir.join("team.coverage.manifest.json");

    std::fs::write(&packet_path, packet_markdown)?;
    let mut events_file =
        std::fs::File::create(&events_path).context("create team events output")?;
    for event in &result.events {
        let line = serde_json::to_string(event).context("serialize team event")?;
        events_file.write_all(line.as_bytes())?;
        events_file.write_all(b"\n")?;
    }

    let coverage_payload = serde_json::to_string_pretty(&result.coverage)
        .context("serialize team coverage manifest")?;
    std::fs::write(&coverage_path, coverage_payload)?;

    Ok(TeamOutputFiles {
        packet: packet_path,
        events: events_path,
        coverage: coverage_path,
    })
}

fn is_compatible_schema(ledger_dir: &Path, required_version: &Option<String>) -> Result<bool> {
    let required = required_version.as_deref();
    if required.is_none() {
        return Ok(true);
    }

    let version_path = ledger_dir.join("coverage.manifest.json");
    let text = fs::read_to_string(&version_path)
        .with_context(|| format!("read coverage manifest {version_path:?}"))?;
    let json: Value = serde_json::from_str(&text).context("parse coverage manifest")?;
    let detected = json
        .get("schema_version")
        .and_then(Value::as_str)
        .unwrap_or("0.2.0");
    Ok(detected == required.unwrap())
}

fn load_member_output(
    ledger_dir: &Path,
    since: Option<NaiveDate>,
    until: Option<NaiveDate>,
) -> Result<Option<(IngestOutput, Vec<String>)>> {
    let events_path = ledger_dir.join("ledger.events.jsonl");
    let coverage_path = ledger_dir.join("coverage.manifest.json");

    if !events_path.exists() || !coverage_path.exists() {
        return Ok(None);
    }

    let events = read_events(&events_path)?;
    let coverage = read_coverage(&coverage_path)?;
    let mut warnings = Vec::new();

    let filtered: Vec<EventEnvelope> = events
        .into_iter()
        .filter(|event| {
            if let Some(since) = since
                && event.occurred_at.date_naive() < since
            {
                return false;
            }
            if let Some(until) = until
                && event.occurred_at.date_naive() >= until
            {
                return false;
            }
            true
        })
        .collect();

    if filtered.is_empty() {
        warnings.push(format!(
            "Member at {} had zero events after filtering",
            ledger_dir.display()
        ));
    }

    Ok(Some((
        IngestOutput {
            events: filtered,
            coverage,
        },
        warnings,
    )))
}

fn read_events(path: &Path) -> Result<Vec<EventEnvelope>> {
    let text = fs::read_to_string(path).with_context(|| format!("read {path:?}"))?;
    let mut events = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let event: EventEnvelope = serde_json::from_str(line)
            .with_context(|| format!("parse event line {} in {:?}", i + 1, path))?;
        events.push(event);
    }
    Ok(events)
}

fn read_coverage(path: &Path) -> Result<CoverageManifest> {
    let text = fs::read_to_string(path).with_context(|| format!("read {path:?}"))?;
    let cov: CoverageManifest =
        serde_json::from_str(&text).with_context(|| format!("parse coverage manifest {path:?}"))?;
    Ok(cov)
}

fn empty_coverage(
    since: Option<NaiveDate>,
    until: Option<NaiveDate>,
    sources: Vec<String>,
    warnings: Vec<String>,
) -> CoverageManifest {
    let today = Utc::now().date_naive();
    CoverageManifest {
        run_id: shiplog_ids::RunId::now("team"),
        generated_at: Utc::now(),
        user: "team".to_string(),
        window: TimeWindow {
            since: since.unwrap_or(today),
            until: until.unwrap_or(today),
        },
        mode: "team".to_string(),
        sources,
        slices: Vec::new(),
        warnings,
        completeness: Completeness::Complete,
    }
}

fn render_default_packet(cfg: &TeamConfig, output: &TeamAggregateResult) -> String {
    let sections = cfg.normalized_sections();
    let included_count = output.included_members.len();
    let total_events = output.events.len();
    let mut out = String::new();

    out.push_str("# Team Shipping Packet\n\n");
    out.push_str(&format!(
        "Members configured: {}\n",
        output.requested_members.len()
    ));
    out.push_str(&format!("Members included: {}\n", included_count));
    if !output.missing_members.is_empty() {
        out.push_str(&format!(
            "Members missing ledgers: {}\n",
            output.missing_members.join(", ")
        ));
    }
    if !output.incompatible_members.is_empty() {
        out.push_str(&format!(
            "Members skipped for incompatible schema: {}\n",
            output.incompatible_members.join(", ")
        ));
    }
    out.push_str(&format!("Total events: {}\n\n", total_events));

    if sections.iter().any(|s| s == "summary") {
        out.push_str("## Summary\n\n");
        out.push_str(&format!(
            "- Sources: {}\n",
            output.coverage.sources.join(", ")
        ));
        out.push_str(&format!(
            "- Coverage completeness: {:?}\n",
            output.coverage.completeness
        ));
        if let Some(slice_count) = output.coverage.slices.len().into() {
            out.push_str(&format!("- Coverage slices: {}\n", slice_count));
        }
        out.push('\n');
    }

    if sections.iter().any(|s| s == "coverage") {
        out.push_str("## Coverage\n\n");
        out.push_str(&format!(
            "- Window: {}..{}\n",
            output.coverage.window.since, output.coverage.window.until
        ));
        if !output.coverage.warnings.is_empty() {
            out.push_str("- Warnings:\n");
            for warning in &output.coverage.warnings {
                out.push_str(&format!("  - {warning}\n"));
            }
            out.push('\n');
        }
    }

    if sections.iter().any(|s| s == "workstreams") {
        out.push_str("## Workstreams\n\n");
        let by_repo = group_events_by_repo(&output.events);
        if by_repo.is_empty() {
            out.push_str("No workstream activity.\n\n");
        } else {
            for (repo, count) in by_repo {
                out.push_str(&format!("## {repo}\n- events: {count}\n\n"));
            }
        }
    }

    if sections.iter().any(|s| s == "receipts") {
        out.push_str("## Receipts\n\n");
        for member in &output.members {
            out.push_str(&format!("### {}\n", member.display_name));
            if member.included {
                out.push_str(&format!("Included events: {}\n", member.event_count));
            } else {
                out.push_str("Not included\n");
            }
            for warning in &member.warnings {
                out.push_str(&format!("- ⚠️ {warning}\n"));
            }
            out.push('\n');
        }
    }

    if !output.warnings.is_empty() {
        out.push_str("## Warnings\n\n");
        for warning in &output.warnings {
            out.push_str(&format!("- {warning}\n"));
        }
    }

    out
}

fn render_with_template(
    template: &str,
    cfg: &TeamConfig,
    result: &TeamAggregateResult,
) -> Result<String> {
    let sections = cfg.normalized_sections();
    let mut context = TemplateContext::new();
    context.set(
        "member_count",
        i64::try_from(result.requested_members.len()).unwrap_or(i64::MAX),
    );
    context.set(
        "included_member_count",
        i64::try_from(result.included_members.len()).unwrap_or(i64::MAX),
    );
    context.set(
        "events",
        i64::try_from(result.events.len()).unwrap_or(i64::MAX),
    );
    context.set(
        "coverage_completeness",
        format!("{:?}", result.coverage.completeness),
    );
    context.set("coverage_sources", result.coverage.sources.join(", "));
    context.set("coverage_since", result.coverage.window.since.to_string());
    context.set("coverage_until", result.coverage.window.until.to_string());
    context.set("missing_members", result.missing_members.join(", "));
    context.set("sections", sections.join(", "));
    context.set(
        "warnings",
        result
            .warnings
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n"),
    );

    let engine = TemplateEngine::new();
    engine
        .render(template, &context)
        .context("render team template")
}

fn group_events_by_repo(events: &[EventEnvelope]) -> Vec<(String, usize)> {
    let mut by_repo: HashMap<String, usize> = HashMap::new();
    for event in events {
        *by_repo.entry(event.repo.full_name.clone()).or_default() += 1;
    }
    let mut entries: Vec<_> = by_repo.into_iter().collect();
    entries.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, TimeZone, Utc};
    use shiplog_ids::EventId;
    use shiplog_schema::coverage::TimeWindow;
    use std::io::Write;

    fn event(id: &str, repo: &str, when: chrono::DateTime<Utc>) -> EventEnvelope {
        EventEnvelope {
            id: EventId::from_parts(["team", id]),
            kind: shiplog_schema::event::EventKind::PullRequest,
            occurred_at: when,
            actor: shiplog_schema::event::Actor {
                login: "alice".to_string(),
                id: None,
            },
            repo: shiplog_schema::event::RepoRef {
                full_name: repo.to_string(),
                html_url: None,
                visibility: shiplog_schema::event::RepoVisibility::Public,
            },
            payload: shiplog_schema::event::EventPayload::PullRequest(
                shiplog_schema::event::PullRequestEvent {
                    number: 1,
                    title: format!("Event {id}"),
                    state: shiplog_schema::event::PullRequestState::Merged,
                    created_at: when,
                    merged_at: Some(when),
                    additions: Some(1),
                    deletions: Some(0),
                    changed_files: Some(1),
                    touched_paths_hint: vec![],
                    window: Some(TimeWindow {
                        since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                        until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
                    }),
                },
            ),
            tags: vec![],
            links: vec![],
            source: shiplog_schema::event::SourceRef {
                system: shiplog_schema::event::SourceSystem::Github,
                url: None,
                opaque_id: None,
            },
        }
    }

    fn coverage(run_id: &str) -> shiplog_schema::coverage::CoverageManifest {
        shiplog_schema::coverage::CoverageManifest {
            run_id: shiplog_ids::RunId(run_id.to_string()),
            generated_at: Utc::now(),
            user: "alice".to_string(),
            window: TimeWindow {
                since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
            },
            mode: "test".to_string(),
            sources: vec!["github".to_string()],
            slices: vec![],
            warnings: vec![],
            completeness: Completeness::Complete,
        }
    }

    fn write_run(dir: &Path, member: &str, include_coverage: bool) -> PathBuf {
        let run_dir = dir.join(member);
        std::fs::create_dir_all(&run_dir).unwrap();
        if include_coverage {
            let mut events = std::fs::File::create(run_dir.join("ledger.events.jsonl")).unwrap();
            let e1 = event(
                &format!("{member}-1"),
                "repo/one",
                Utc.with_ymd_and_hms(2025, 1, 2, 0, 0, 0).unwrap(),
            );
            let e2 = event(
                &format!("{member}-2"),
                "repo/two",
                Utc.with_ymd_and_hms(2025, 1, 3, 0, 0, 0).unwrap(),
            );
            writeln!(events, "{}", serde_json::to_string(&e1).unwrap()).unwrap();
            writeln!(events, "{}", serde_json::to_string(&e2).unwrap()).unwrap();
            std::fs::write(
                run_dir.join("coverage.manifest.json"),
                serde_json::to_string(&coverage(&format!("{member}-run"))).unwrap(),
            )
            .unwrap();
        }
        run_dir
    }

    #[test]
    fn aggregates_multiple_members() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path();
        write_run(dir, "alice", true);
        write_run(dir, "bob", true);

        let agg = TeamAggregator::new(TeamConfig {
            members: vec!["alice".into(), "bob".into()],
            ..TeamConfig::default()
        });

        let result = agg.aggregate(dir).unwrap();
        assert_eq!(result.included_members.len(), 2);
        assert_eq!(result.events.len(), 4);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn missing_member_reports_warning_and_continues() {
        let root = tempfile::tempdir().unwrap();
        write_run(root.path(), "alice", true);

        let agg = TeamAggregator::new(TeamConfig {
            members: vec!["alice".into(), "missing".into()],
            ..TeamConfig::default()
        });

        let result = agg.aggregate(root.path()).unwrap();
        assert_eq!(result.included_members.len(), 1);
        assert_eq!(result.members.len(), 2);
        assert!(
            result.warnings.iter().any(|w| w.contains("Missing ledger")),
            "missing warnings: {:?}",
            result.warnings
        );
    }

    #[test]
    fn renders_default_packet() {
        let root = tempfile::tempdir().unwrap();
        write_run(root.path(), "alice", true);
        let agg = TeamAggregator::new(TeamConfig {
            members: vec!["alice".into()],
            ..TeamConfig::default()
        });
        let output = agg.aggregate(root.path()).unwrap();
        let packet = agg.render_packet_markdown(&output).unwrap();
        assert!(packet.contains("# Team Shipping Packet"));
        assert!(packet.contains("Total events: 2"));
    }
}
