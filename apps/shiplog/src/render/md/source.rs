//! Source identity and display helpers for Markdown rendering.
//!
//! This module keeps source normalization separate from packet section
//! rendering so every renderer path uses the same source matching and labels.

use shiplog::schema::event::EventEnvelope;

#[derive(Clone, Copy, Debug)]
pub(crate) struct SkippedSource<'a> {
    pub(crate) source: &'a str,
    pub(crate) reason: &'a str,
}

pub(crate) fn skipped_source_warnings(warnings: &[String]) -> Vec<SkippedSource<'_>> {
    warnings
        .iter()
        .filter_map(|warning| skipped_source_warning(warning))
        .collect()
}

pub(crate) fn skipped_source_warning(warning: &str) -> Option<SkippedSource<'_>> {
    const PREFIX: &str = "Configured source ";
    const INFIX: &str = " was skipped: ";

    let rest = warning.strip_prefix(PREFIX)?;
    let (source, reason) = rest.split_once(INFIX)?;
    Some(SkippedSource { source, reason })
}

pub(crate) fn included_source_summary(
    manifest_sources: &[String],
    events: &[EventEnvelope],
    skipped_sources: &[SkippedSource<'_>],
) -> Vec<String> {
    let mut sources = Vec::new();
    for source in manifest_sources {
        push_manifest_source(&mut sources, source, skipped_sources);
    }
    for event in events {
        push_source(&mut sources, event.source.system.as_str());
    }
    sources
}

fn push_manifest_source(
    sources: &mut Vec<String>,
    candidate: &str,
    skipped_sources: &[SkippedSource<'_>],
) {
    if skipped_sources
        .iter()
        .any(|skipped| source_matches(skipped.source, candidate))
    {
        return;
    }

    push_source(sources, candidate);
}

fn push_source(sources: &mut Vec<String>, candidate: &str) {
    if sources
        .iter()
        .any(|source| source_matches(source, candidate))
    {
        return;
    }

    sources.push(candidate.to_string());
}

pub(crate) fn source_event_count(events: &[EventEnvelope], source: &str) -> usize {
    events
        .iter()
        .filter(|event| source_matches(event.source.system.as_str(), source))
        .count()
}

pub(crate) fn source_present(sources: &[String], needle: &str) -> bool {
    sources.iter().any(|source| source_matches(source, needle))
}

pub(crate) fn event_source_present(events: &[EventEnvelope], needle: &str) -> bool {
    events
        .iter()
        .any(|event| source_matches(event.source.system.as_str(), needle))
}

fn source_matches(left: &str, right: &str) -> bool {
    canonical_source_key(left) == canonical_source_key(right)
}

fn canonical_source_key(source: &str) -> String {
    let key = source.trim().to_lowercase();

    match key.as_str() {
        "json" | "json_import" | "json import" | "json-import" => "json_import".to_string(),
        "git" | "local_git" | "local git" | "local-git" => "local_git".to_string(),
        _ => key,
    }
}

pub(crate) fn display_source_label(source: &str) -> String {
    if source.eq_ignore_ascii_case("json") {
        return "JSON".to_string();
    }

    match canonical_source_key(source).as_str() {
        "github" => "GitHub".to_string(),
        "gitlab" => "GitLab".to_string(),
        "jira" => "Jira".to_string(),
        "linear" => "Linear".to_string(),
        "json_import" => "JSON import".to_string(),
        "local_git" => "Local git".to_string(),
        "manual" => "Manual".to_string(),
        "unknown" => "Unknown".to_string(),
        _ => source.to_string(),
    }
}

pub(crate) fn display_source_list(sources: &[String]) -> String {
    if sources.is_empty() {
        return "none recorded".to_string();
    }

    sources
        .iter()
        .map(|source| display_source_label(source))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, TimeZone, Utc};
    use shiplog::ids::EventId;
    use shiplog::schema::coverage::TimeWindow;
    use shiplog::schema::event::{
        Actor, EventKind, EventPayload, Link, PullRequestEvent, PullRequestState, RepoRef,
        RepoVisibility, SourceRef, SourceSystem,
    };

    fn pr_event(system: SourceSystem, number: u64) -> EventEnvelope {
        EventEnvelope {
            id: EventId::from_parts(["github", "pr", "acme/foo", &number.to_string()]),
            kind: EventKind::PullRequest,
            occurred_at: Utc.timestamp_opt(0, 0).single().unwrap_or_default(),
            actor: Actor {
                login: "user".into(),
                id: None,
            },
            repo: RepoRef {
                full_name: "acme/foo".into(),
                html_url: Some("https://github.com/acme/foo".into()),
                visibility: RepoVisibility::Unknown,
            },
            payload: EventPayload::PullRequest(PullRequestEvent {
                number,
                title: format!("PR {number}"),
                state: PullRequestState::Merged,
                created_at: Utc.timestamp_opt(0, 0).single().unwrap_or_default(),
                merged_at: Some(Utc.timestamp_opt(0, 0).single().unwrap_or_default()),
                additions: Some(1),
                deletions: Some(0),
                changed_files: Some(1),
                touched_paths_hint: vec![],
                window: Some(TimeWindow {
                    since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap_or_default(),
                    until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap_or_default(),
                }),
            }),
            tags: vec![],
            links: vec![Link {
                label: "pr".into(),
                url: format!("https://github.com/acme/foo/pull/{number}"),
            }],
            source: SourceRef {
                system,
                url: None,
                opaque_id: None,
            },
        }
    }

    #[test]
    fn skipped_source_warning_parses_source_and_reason() {
        let parsed = skipped_source_warning("Configured source github was skipped: rate limit");
        let parsed = parsed.unwrap_or(SkippedSource {
            source: "",
            reason: "",
        });
        assert_eq!(parsed.source, "github");
        assert_eq!(parsed.reason, "rate limit");
    }

    #[test]
    fn skipped_source_warning_returns_none_without_prefix() {
        assert!(skipped_source_warning("source github was skipped: rate limit").is_none());
    }

    #[test]
    fn skipped_source_warning_returns_none_without_infix() {
        assert!(
            skipped_source_warning("Configured source github failed for unknown reasons").is_none()
        );
    }

    #[test]
    fn skipped_source_warnings_filters_matching_entries() {
        let warnings = vec![
            "Configured source github was skipped: rate limit".to_string(),
            "Unrelated message".to_string(),
            "Configured source gitlab was skipped: auth".to_string(),
        ];
        let parsed = skipped_source_warnings(&warnings);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].source, "github");
        assert_eq!(parsed[0].reason, "rate limit");
        assert_eq!(parsed[1].source, "gitlab");
        assert_eq!(parsed[1].reason, "auth");
    }

    #[test]
    fn included_source_summary_dedups_manifest_entries() {
        let manifest = vec![
            "github".to_string(),
            "GitHub".to_string(),
            "github".to_string(),
        ];
        let events: Vec<EventEnvelope> = vec![];
        let summary = included_source_summary(&manifest, &events, &[]);
        assert_eq!(summary, vec!["github".to_string()]);
    }

    #[test]
    fn included_source_summary_dedups_across_manifest_and_events() {
        let manifest = vec!["github".to_string()];
        let events = vec![pr_event(SourceSystem::Github, 1)];
        let summary = included_source_summary(&manifest, &events, &[]);
        assert_eq!(summary, vec!["github".to_string()]);
    }

    #[test]
    fn included_source_summary_excludes_skipped_from_manifest_but_keeps_event_sources() {
        let manifest = vec!["github".to_string(), "gitlab".to_string()];
        let events = vec![pr_event(SourceSystem::Github, 1)];
        let skipped = vec![SkippedSource {
            source: "GitHub",
            reason: "rate limit",
        }];
        let summary = included_source_summary(&manifest, &events, &skipped);
        // gitlab survives (not skipped); github not in manifest, but still added via event.
        assert_eq!(summary, vec!["gitlab".to_string(), "github".to_string()]);
    }

    #[test]
    fn included_source_summary_preserves_first_seen_order() {
        let manifest = vec![
            "jira".to_string(),
            "github".to_string(),
            "manual".to_string(),
        ];
        let events: Vec<EventEnvelope> = vec![];
        let summary = included_source_summary(&manifest, &events, &[]);
        assert_eq!(
            summary,
            vec![
                "jira".to_string(),
                "github".to_string(),
                "manual".to_string()
            ]
        );
    }

    #[test]
    fn included_source_summary_canonical_key_matches_json_aliases() {
        let manifest = vec![
            "json".to_string(),
            "json_import".to_string(),
            "json-import".to_string(),
        ];
        let events: Vec<EventEnvelope> = vec![];
        let summary = included_source_summary(&manifest, &events, &[]);
        // All three canonicalize to "json_import"; first-seen ("json") wins.
        assert_eq!(summary, vec!["json".to_string()]);
    }

    #[test]
    fn source_event_count_uses_canonical_key() {
        let events = vec![
            pr_event(SourceSystem::Github, 1),
            pr_event(SourceSystem::Github, 2),
            pr_event(SourceSystem::Manual, 3),
        ];
        assert_eq!(source_event_count(&events, "GitHub"), 2);
        assert_eq!(source_event_count(&events, "manual"), 1);
    }

    #[test]
    fn source_event_count_zero_when_absent() {
        let events = vec![pr_event(SourceSystem::Github, 1)];
        assert_eq!(source_event_count(&events, "gitlab"), 0);
    }

    #[test]
    fn source_present_matches_canonical_aliases() {
        let sources = vec!["json".to_string(), "github".to_string()];
        assert!(source_present(&sources, "json_import"));
        assert!(source_present(&sources, "GitHub"));
        assert!(!source_present(&sources, "gitlab"));
    }

    #[test]
    fn event_source_present_matches_case_insensitively() {
        let events = vec![pr_event(SourceSystem::Github, 1)];
        assert!(event_source_present(&events, "GITHUB"));
        assert!(!event_source_present(&events, "gitlab"));
    }

    #[test]
    fn display_source_label_maps_known_sources() {
        assert_eq!(display_source_label("github"), "GitHub");
        assert_eq!(display_source_label("GITHUB"), "GitHub");
        assert_eq!(display_source_label("gitlab"), "GitLab");
        assert_eq!(display_source_label("jira"), "Jira");
        assert_eq!(display_source_label("linear"), "Linear");
        assert_eq!(display_source_label("json_import"), "JSON import");
        assert_eq!(display_source_label("local_git"), "Local git");
        assert_eq!(display_source_label("manual"), "Manual");
        assert_eq!(display_source_label("unknown"), "Unknown");
    }

    #[test]
    fn display_source_label_special_cases_json() {
        assert_eq!(display_source_label("json"), "JSON");
        assert_eq!(display_source_label("JSON"), "JSON");
        assert_eq!(display_source_label("Json"), "JSON");
    }

    #[test]
    fn display_source_label_falls_through_for_unknown_inputs() {
        assert_eq!(display_source_label("custom-source"), "custom-source");
        assert_eq!(display_source_label("notion"), "notion");
    }

    #[test]
    fn display_source_list_empty_returns_none_recorded() {
        let sources: Vec<String> = vec![];
        assert_eq!(display_source_list(&sources), "none recorded");
    }

    #[test]
    fn display_source_list_single_source() {
        let sources = vec!["github".to_string()];
        assert_eq!(display_source_list(&sources), "GitHub");
    }

    #[test]
    fn display_source_list_joins_multiple_with_comma_space() {
        let sources = vec![
            "github".to_string(),
            "json".to_string(),
            "manual".to_string(),
        ];
        assert_eq!(display_source_list(&sources), "GitHub, JSON, Manual");
    }
}
