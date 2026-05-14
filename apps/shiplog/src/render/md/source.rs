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
