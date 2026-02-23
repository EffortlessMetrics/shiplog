//! Team packet rendering contracts for team aggregation mode.
//!
//! This crate isolates rendering behavior from team ledger aggregation so
//! callers can evolve rendering independently from ingest/merge logic.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use shiplog_schema::coverage::CoverageManifest;
use shiplog_schema::event::EventEnvelope;
use shiplog_team_core::TeamConfig;
use shiplog_template::{TemplateContext, TemplateEngine};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

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

/// Render a markdown packet for team-level output.
pub fn render_packet_markdown(cfg: &TeamConfig, output: &TeamAggregateResult) -> Result<String> {
    if let Some(template) = &cfg.template {
        let text = fs::read_to_string(template)
            .with_context(|| format!("read team template {:?}", template))?;
        return render_with_template(&text, cfg, output);
    }

    Ok(render_default_packet(cfg, output))
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
        out.push_str(&format!(
            "- Coverage slices: {}\n",
            output.coverage.slices.len()
        ));
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
                out.push_str(&format!("- WARNING: {warning}\n"));
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
    use shiplog_ids::{EventId, RunId};
    use shiplog_schema::coverage::{Completeness, CoverageManifest, TimeWindow};
    use shiplog_schema::event::{
        Actor, EventEnvelope, EventKind, EventPayload, PullRequestEvent, PullRequestState, RepoRef,
        RepoVisibility, SourceRef, SourceSystem,
    };

    fn event(id: &str, repo: &str, when: chrono::DateTime<Utc>) -> EventEnvelope {
        EventEnvelope {
            id: EventId::from_parts(["team", id]),
            kind: EventKind::PullRequest,
            occurred_at: when,
            actor: Actor {
                login: "alice".to_string(),
                id: None,
            },
            repo: RepoRef {
                full_name: repo.to_string(),
                html_url: None,
                visibility: RepoVisibility::Public,
            },
            payload: EventPayload::PullRequest(PullRequestEvent {
                number: 1,
                title: format!("Event {id}"),
                state: PullRequestState::Merged,
                created_at: when,
                merged_at: Some(when),
                additions: Some(1),
                deletions: Some(0),
                changed_files: Some(1),
                touched_paths_hint: vec![],
                window: Some(TimeWindow {
                    since: NaiveDate::from_ymd_opt(2025, 1, 1).expect("valid date"),
                    until: NaiveDate::from_ymd_opt(2025, 2, 1).expect("valid date"),
                }),
            }),
            tags: vec![],
            links: vec![],
            source: SourceRef {
                system: SourceSystem::Github,
                url: None,
                opaque_id: None,
            },
        }
    }

    fn coverage() -> CoverageManifest {
        CoverageManifest {
            run_id: RunId::now("team"),
            generated_at: Utc::now(),
            user: "alice".to_string(),
            window: TimeWindow {
                since: NaiveDate::from_ymd_opt(2025, 1, 1).expect("valid date"),
                until: NaiveDate::from_ymd_opt(2025, 2, 1).expect("valid date"),
            },
            mode: "merged".to_string(),
            sources: vec!["github".to_string()],
            slices: vec![],
            warnings: vec![],
            completeness: Completeness::Complete,
        }
    }

    fn sample_result() -> TeamAggregateResult {
        let when = Utc.with_ymd_and_hms(2025, 1, 2, 0, 0, 0).unwrap();
        TeamAggregateResult {
            requested_members: vec!["alice".to_string(), "bob".to_string()],
            included_members: vec!["alice".to_string()],
            missing_members: vec!["bob".to_string()],
            incompatible_members: vec![],
            members: vec![
                TeamMemberSummary {
                    member: "alice".to_string(),
                    display_name: "Alice".to_string(),
                    included: true,
                    ledger_dir: PathBuf::from("alice"),
                    event_count: 2,
                    warnings: vec![],
                },
                TeamMemberSummary {
                    member: "bob".to_string(),
                    display_name: "Bob".to_string(),
                    included: false,
                    ledger_dir: PathBuf::from("bob"),
                    event_count: 0,
                    warnings: vec!["Missing ledger".to_string()],
                },
            ],
            coverage: coverage(),
            events: vec![event("1", "repo/one", when), event("2", "repo/one", when)],
            warnings: vec!["Missing ledger".to_string()],
        }
    }

    #[test]
    fn renders_default_packet_with_expected_sections() {
        let cfg = TeamConfig::default();
        let packet = render_packet_markdown(&cfg, &sample_result()).expect("render packet");

        assert!(packet.contains("# Team Shipping Packet"));
        assert!(packet.contains("Members configured: 2"));
        assert!(packet.contains("## Summary"));
        assert!(packet.contains("## Coverage"));
        assert!(packet.contains("## Workstreams"));
        assert!(packet.contains("## Receipts"));
        assert!(packet.contains("### Alice"));
    }

    #[test]
    fn renders_custom_template() {
        let temp = tempfile::tempdir().expect("tempdir");
        let template = temp.path().join("team-template.md");
        std::fs::write(
            &template,
            "# Team\nMembers={{ member_count }}\nIncluded={{ included_member_count }}",
        )
        .expect("write template");

        let cfg = TeamConfig {
            template: Some(template),
            ..TeamConfig::default()
        };
        let packet = render_packet_markdown(&cfg, &sample_result()).expect("render packet");
        assert!(packet.contains("# Team"));
        assert!(packet.contains("Members=2"));
        assert!(packet.contains("Included=1"));
    }
}
