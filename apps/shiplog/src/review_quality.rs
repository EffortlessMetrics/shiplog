//! Review quality signal and evidence debt helpers.

use chrono::Utc;
use shiplog::ids::EventId;
use shiplog::schema::{
    coverage::CoverageManifest,
    event::{EventEnvelope, EventPayload},
    workstream::{Workstream, WorkstreamsFile},
};
use std::collections::HashMap;

pub(crate) struct ConfiguredSourceSkip {
    pub(crate) source: String,
    pub(crate) reason: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EvidenceDebtSeverity {
    Info,
    Warning,
    Blocking,
}

impl EvidenceDebtSeverity {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Blocking => "blocking",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EvidenceDebtKind {
    MissingSource,
    PartialCoverage,
    CoverageWarning,
    IncompleteQuery,
    ManualContext,
    MissingReceiptAnchors,
    ThinWorkstream,
    LargeMiscWorkstream,
    CodeOnlyWorkstream,
    TicketOnlyWorkstream,
    ManualOnlyWorkstream,
    TooManySelectedReceipts,
    BroadWorkstream,
    WorkstreamValidation,
}

impl EvidenceDebtKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::MissingSource => "missing-source",
            Self::PartialCoverage => "partial-coverage",
            Self::CoverageWarning => "coverage-warning",
            Self::IncompleteQuery => "incomplete-query",
            Self::ManualContext => "manual-context",
            Self::MissingReceiptAnchors => "no-selected-receipts",
            Self::ThinWorkstream => "thin-workstream",
            Self::LargeMiscWorkstream => "large-misc-workstream",
            Self::CodeOnlyWorkstream => "code-only-workstream",
            Self::TicketOnlyWorkstream => "ticket-only-workstream",
            Self::ManualOnlyWorkstream => "manual-only-workstream",
            Self::TooManySelectedReceipts => "too-many-selected-receipts",
            Self::BroadWorkstream => "broad-workstream",
            Self::WorkstreamValidation => "workstream-validation",
        }
    }
}

#[derive(Debug)]
pub(crate) struct EvidenceDebt {
    pub(crate) severity: EvidenceDebtSeverity,
    pub(crate) kind: EvidenceDebtKind,
    pub(crate) summary: String,
    pub(crate) detail: Option<String>,
    pub(crate) next_step: Option<String>,
}

#[derive(Default)]
pub(crate) struct WorkstreamQualitySignals<'a> {
    pub(crate) no_receipt_workstreams: Vec<&'a Workstream>,
    pub(crate) broad_workstreams: Vec<&'a Workstream>,
    pub(crate) manual_context_workstreams: Vec<&'a Workstream>,
    pub(crate) thin_workstreams: Vec<&'a Workstream>,
    pub(crate) large_misc_workstreams: Vec<&'a Workstream>,
    pub(crate) code_only_workstreams: Vec<&'a Workstream>,
    pub(crate) ticket_only_workstreams: Vec<&'a Workstream>,
    pub(crate) manual_only_workstreams: Vec<&'a Workstream>,
    pub(crate) too_many_receipt_workstreams: Vec<&'a Workstream>,
    pub(crate) manual_events: usize,
}

#[derive(Default)]
struct WorkstreamEvidenceProfile {
    code: usize,
    tickets: usize,
    manual: usize,
}

pub(crate) struct EvidenceDebtInput<'a> {
    pub(crate) run_id: &'a str,
    pub(crate) coverage: &'a CoverageManifest,
    pub(crate) events: &'a [EventEnvelope],
    pub(crate) skipped_sources: &'a [ConfiguredSourceSkip],
    pub(crate) workstreams: &'a WorkstreamsFile,
    pub(crate) validation_errors: &'a [String],
    pub(crate) signals: &'a WorkstreamQualitySignals<'a>,
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

const BROAD_WORKSTREAM_EVENT_THRESHOLD: usize = 10;

const LARGE_MISC_WORKSTREAM_EVENT_THRESHOLD: usize = 5;

const TOO_MANY_SELECTED_RECEIPTS_THRESHOLD: usize = 5;

const SINGLE_SOURCE_WORKSTREAM_EVENT_THRESHOLD: usize = 5;

pub(crate) fn workstream_quality_signals<'a>(
    workstreams: &'a WorkstreamsFile,
    events: &[EventEnvelope],
) -> WorkstreamQualitySignals<'a> {
    let events_by_id: HashMap<EventId, &EventEnvelope> = events
        .iter()
        .map(|event| (event.id.clone(), event))
        .collect();
    let manual_events = events
        .iter()
        .filter(|event| matches!(event.payload, EventPayload::Manual(_)))
        .count();
    let mut signals = WorkstreamQualitySignals {
        manual_events,
        ..WorkstreamQualitySignals::default()
    };

    for workstream in &workstreams.workstreams {
        let event_count = workstream.events.len();
        let profile = workstream_evidence_profile(workstream, &events_by_id);

        if workstream.receipts.is_empty() {
            signals.no_receipt_workstreams.push(workstream);
        }
        if event_count >= BROAD_WORKSTREAM_EVENT_THRESHOLD {
            signals.broad_workstreams.push(workstream);
        }
        if event_count == 1 && workstream.receipts.is_empty() {
            signals.thin_workstreams.push(workstream);
        }
        if event_count >= LARGE_MISC_WORKSTREAM_EVENT_THRESHOLD
            && is_misc_workstream_title(workstream)
        {
            signals.large_misc_workstreams.push(workstream);
        }
        if workstream.receipts.len() > TOO_MANY_SELECTED_RECEIPTS_THRESHOLD {
            signals.too_many_receipt_workstreams.push(workstream);
        }
        if event_count >= SINGLE_SOURCE_WORKSTREAM_EVENT_THRESHOLD
            && profile.code > 0
            && profile.tickets == 0
            && profile.manual == 0
        {
            signals.code_only_workstreams.push(workstream);
        }
        if event_count >= SINGLE_SOURCE_WORKSTREAM_EVENT_THRESHOLD
            && profile.tickets > 0
            && profile.code == 0
            && profile.manual == 0
        {
            signals.ticket_only_workstreams.push(workstream);
        }
        if event_count >= 1 && profile.manual > 0 && profile.code == 0 && profile.tickets == 0 {
            signals.manual_only_workstreams.push(workstream);
        }
        if event_count >= BROAD_WORKSTREAM_EVENT_THRESHOLD && profile.manual == 0 {
            signals.manual_context_workstreams.push(workstream);
        }
    }

    signals
}

fn workstream_evidence_profile(
    workstream: &Workstream,
    events_by_id: &HashMap<EventId, &EventEnvelope>,
) -> WorkstreamEvidenceProfile {
    let mut profile = WorkstreamEvidenceProfile::default();

    for event in workstream
        .events
        .iter()
        .filter_map(|event_id| events_by_id.get(event_id).copied())
    {
        match event_source_bucket(event) {
            WorkstreamSourceBucket::Code => profile.code += 1,
            WorkstreamSourceBucket::Ticket => profile.tickets += 1,
            WorkstreamSourceBucket::Manual => profile.manual += 1,
        }
    }

    profile
}

enum WorkstreamSourceBucket {
    Code,
    Ticket,
    Manual,
}

fn event_source_bucket(event: &EventEnvelope) -> WorkstreamSourceBucket {
    let source = event.source.system.as_str().to_ascii_lowercase();
    match source.as_str() {
        "jira" | "linear" => WorkstreamSourceBucket::Ticket,
        "manual" => WorkstreamSourceBucket::Manual,
        "github" | "gitlab" | "local_git" | "localgit" => WorkstreamSourceBucket::Code,
        _ => match event.payload {
            EventPayload::PullRequest(_) | EventPayload::Review(_) => WorkstreamSourceBucket::Code,
            EventPayload::Manual(_) => WorkstreamSourceBucket::Manual,
        },
    }
}

fn is_misc_workstream_title(workstream: &Workstream) -> bool {
    let normalized = workstream.title.trim().to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "misc" | "miscellaneous" | "other" | "uncategorized" | "untriaged"
    )
}

pub(crate) fn detect_evidence_debt(input: EvidenceDebtInput<'_>) -> Vec<EvidenceDebt> {
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

    if input.coverage.completeness != shiplog::schema::coverage::Completeness::Complete {
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
        debt.push(
            EvidenceDebt::new(
                EvidenceDebtSeverity::Warning,
                EvidenceDebtKind::CoverageWarning,
                warning.clone(),
            )
            .next_step(format!(
                "Run `shiplog runs show --run {}` to inspect this run.",
                input.run_id
            )),
        );
    }

    for slice in input.coverage.slices.iter().filter(|slice| {
        slice.incomplete_results.unwrap_or(false) || slice.fetched < slice.total_count
    }) {
        debt.push(
            EvidenceDebt::new(
                EvidenceDebtSeverity::Warning,
                EvidenceDebtKind::IncompleteQuery,
                format!(
                    "Query {:?} fetched {}/{} result(s).",
                    slice.query, slice.fetched, slice.total_count
                ),
            )
            .next_step(
                "Run `shiplog intake --last-6-months --explain` after repairing source setup.",
            ),
        );
    }

    if input.signals.manual_events > 0 {
        debt.push(
            EvidenceDebt::new(
                EvidenceDebtSeverity::Info,
                EvidenceDebtKind::ManualContext,
                "Manual events are user-provided; keep context current before sharing.",
            )
            .next_step("Run `shiplog journal list` to inspect manual evidence."),
        );
    }

    if !input.signals.manual_context_workstreams.is_empty() {
        let first = input.signals.manual_context_workstreams[0];
        debt.push(
            EvidenceDebt::new(
                EvidenceDebtSeverity::Info,
                EvidenceDebtKind::ManualContext,
                format!(
                    "{} broad workstream(s) have no manual outcome note.",
                    input.signals.manual_context_workstreams.len()
                ),
            )
            .detail(workstream_title_sample(
                &input.signals.manual_context_workstreams,
            ))
            .next_step(journal_add_next_step(&first.title)),
        );
    }

    if !input.signals.no_receipt_workstreams.is_empty() {
        debt.push(
            EvidenceDebt::new(
                EvidenceDebtSeverity::Warning,
                EvidenceDebtKind::MissingReceiptAnchors,
                format!(
                    "{} workstream(s) have no selected receipt anchors.",
                    input.signals.no_receipt_workstreams.len()
                ),
            )
            .detail(workstream_title_sample(
                &input.signals.no_receipt_workstreams,
            ))
            .next_step(format!(
                "Run `shiplog workstreams receipts --run {} --workstream <title>`.",
                input.run_id
            )),
        );
    }

    if !input.signals.too_many_receipt_workstreams.is_empty() {
        debt.push(
            EvidenceDebt::new(
                EvidenceDebtSeverity::Info,
                EvidenceDebtKind::TooManySelectedReceipts,
                format!(
                    "{} workstream(s) have more than {} selected receipt anchors.",
                    input.signals.too_many_receipt_workstreams.len(),
                    TOO_MANY_SELECTED_RECEIPTS_THRESHOLD
                ),
            )
            .detail(workstream_title_sample(&input.signals.too_many_receipt_workstreams))
            .next_step(format!(
                "Run `shiplog workstreams receipts --run {} --workstream <title>` and keep the strongest anchors.",
                input.run_id
            )),
        );
    }

    if !input.signals.thin_workstreams.is_empty() {
        debt.push(
            EvidenceDebt::new(
                EvidenceDebtSeverity::Info,
                EvidenceDebtKind::ThinWorkstream,
                format!(
                    "{} workstream(s) have only one assigned event.",
                    input.signals.thin_workstreams.len()
                ),
            )
            .detail(workstream_title_sample(&input.signals.thin_workstreams))
            .next_step(format!(
                "Run `shiplog workstreams receipts --run {} --workstream <title>` to confirm the anchor.",
                input.run_id
            )),
        );
    }

    if !input.signals.large_misc_workstreams.is_empty() {
        debt.push(
            EvidenceDebt::new(
                EvidenceDebtSeverity::Warning,
                EvidenceDebtKind::LargeMiscWorkstream,
                format!(
                    "{} miscellaneous workstream(s) have {}+ events.",
                    input.signals.large_misc_workstreams.len(),
                    LARGE_MISC_WORKSTREAM_EVENT_THRESHOLD
                ),
            )
            .detail(workstream_title_sample(&input.signals.large_misc_workstreams))
            .next_step(format!(
                "Run `shiplog workstreams split --run {} --from <title> --to \"<new workstream>\" --matching \"<pattern>\" --create`.",
                input.run_id
            )),
        );
    }

    if !input.signals.code_only_workstreams.is_empty() {
        debt.push(
            EvidenceDebt::new(
                EvidenceDebtSeverity::Info,
                EvidenceDebtKind::CodeOnlyWorkstream,
                format!(
                    "{} workstream(s) only have code or review receipts.",
                    input.signals.code_only_workstreams.len()
                ),
            )
            .detail(workstream_title_sample(
                &input.signals.code_only_workstreams,
            ))
            .next_step(journal_add_next_step(
                &input.signals.code_only_workstreams[0].title,
            )),
        );
    }

    if !input.signals.ticket_only_workstreams.is_empty() {
        debt.push(
            EvidenceDebt::new(
                EvidenceDebtSeverity::Info,
                EvidenceDebtKind::TicketOnlyWorkstream,
                format!(
                    "{} workstream(s) only have ticket receipts.",
                    input.signals.ticket_only_workstreams.len()
                ),
            )
            .detail(workstream_title_sample(
                &input.signals.ticket_only_workstreams,
            ))
            .next_step(journal_add_next_step(
                &input.signals.ticket_only_workstreams[0].title,
            )),
        );
    }

    if !input.signals.manual_only_workstreams.is_empty() {
        debt.push(
            EvidenceDebt::new(
                EvidenceDebtSeverity::Info,
                EvidenceDebtKind::ManualOnlyWorkstream,
                format!(
                    "{} workstream(s) only have manual evidence.",
                    input.signals.manual_only_workstreams.len()
                ),
            )
            .detail(workstream_title_sample(
                &input.signals.manual_only_workstreams,
            ))
            .next_step("Run `shiplog journal list` and attach external receipts where available."),
        );
    }

    if !input.signals.broad_workstreams.is_empty() {
        debt.push(
            EvidenceDebt::new(
                EvidenceDebtSeverity::Info,
                EvidenceDebtKind::BroadWorkstream,
                format!(
                    "{} workstream(s) have 10+ events; consider splitting broad buckets.",
                    input.signals.broad_workstreams.len()
                ),
            )
            .detail(workstream_title_sample(&input.signals.broad_workstreams))
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
        debt.push(
            EvidenceDebt::new(
                EvidenceDebtSeverity::Blocking,
                EvidenceDebtKind::WorkstreamValidation,
                "Ledger has events but no workstream assignments.",
            )
            .next_step(format!(
                "Run `shiplog workstreams validate --run {}`.",
                input.run_id
            )),
        );
    }

    debt
}

pub(crate) fn print_evidence_debt(debt: &[EvidenceDebt]) {
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

pub(crate) fn journal_add_next_step(workstream_title: &str) -> String {
    format!(
        "shiplog journal add --date {} --title {} --workstream {}",
        Utc::now().date_naive(),
        quote_cli_value(&format!("Outcome note for {workstream_title}")),
        quote_cli_value(workstream_title)
    )
}

pub(crate) fn journal_add_template_next_step(workstream_title: &str) -> String {
    format!(
        "shiplog journal add --date {} --title {} --workstream {} --description {}",
        Utc::now().date_naive(),
        quote_cli_value(&format!("Outcome note for {workstream_title}")),
        quote_cli_value(workstream_title),
        quote_cli_value("<replace with factual context or outcome>")
    )
}

pub(crate) fn quote_cli_value(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\\\""))
}

pub(crate) fn review_source_event_counts(
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

pub(crate) fn configured_source_skips(warnings: &[String]) -> Vec<ConfiguredSourceSkip> {
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

pub(crate) fn sources_match(left: &str, right: &str) -> bool {
    normalized_source_key(left) == normalized_source_key(right)
}

pub(crate) fn normalized_source_key(source: &str) -> String {
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

pub(crate) fn display_source_label(source: &str) -> String {
    match normalized_source_key(source).as_str() {
        "github" => "GitHub".to_string(),
        "gitlab" => "GitLab".to_string(),
        "jira" => "Jira".to_string(),
        "linear" => "Linear".to_string(),
        "manual" => "Manual".to_string(),
        "json" => "JSON".to_string(),
        "git" => "Local git".to_string(),
        "redaction" => "Redaction".to_string(),
        "unknown" => "Unknown".to_string(),
        other => other.to_string(),
    }
}
