use super::{
    ConfiguredSourceSkip, configured_source_skip, display_source_label, journal_add_next_step,
};
use crate::review_quality_signals::{
    LARGE_MISC_WORKSTREAM_EVENT_THRESHOLD, TOO_MANY_SELECTED_RECEIPTS_THRESHOLD,
    WorkstreamQualitySignals,
};
use shiplog::schema::{
    coverage::CoverageManifest,
    event::EventEnvelope,
    workstream::{Workstream, WorkstreamsFile},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum EvidenceDebtSeverity {
    Info,
    Warning,
    Blocking,
}

impl EvidenceDebtSeverity {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Blocking => "blocking",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum EvidenceDebtKind {
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
    pub(super) fn label(self) -> &'static str {
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
pub(super) struct EvidenceDebt {
    pub(super) severity: EvidenceDebtSeverity,
    pub(super) kind: EvidenceDebtKind,
    pub(super) summary: String,
    pub(super) detail: Option<String>,
    pub(super) next_step: Option<String>,
}

pub(super) struct EvidenceDebtInput<'a> {
    pub(super) run_id: &'a str,
    pub(super) coverage: &'a CoverageManifest,
    pub(super) events: &'a [EventEnvelope],
    pub(super) skipped_sources: &'a [ConfiguredSourceSkip],
    pub(super) workstreams: &'a WorkstreamsFile,
    pub(super) validation_errors: &'a [String],
    pub(super) signals: &'a WorkstreamQualitySignals<'a>,
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

pub(super) fn detect_evidence_debt(input: EvidenceDebtInput<'_>) -> Vec<EvidenceDebt> {
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

pub(super) fn print_evidence_debt(debt: &[EvidenceDebt]) {
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
