//! Markdown packet renderer for shiplog.
//!
//! Converts canonical events, workstreams, and coverage metadata into an
//! editable self-review packet with receipts and appendix sections.

use anyhow::Result;
use shiplog_ports::Renderer;
use shiplog_schema::coverage::CoverageManifest;
use shiplog_schema::event::{EventEnvelope, EventKind};
use shiplog_schema::workstream::{Workstream, WorkstreamsFile};
use shiplog_workstreams::WORKSTREAM_RECEIPT_RENDER_LIMIT;
use std::collections::HashMap;

pub mod receipt;

pub use receipt::{format_receipt_markdown, manual_type_emoji};

const WORKSTREAM_EVIDENCE_ANCHOR_LIMIT: usize = 3;

/// Section ordering configuration
///
/// # Examples
///
/// ```
/// use shiplog_render_md::SectionOrder;
///
/// let order = SectionOrder::default();
/// assert_eq!(order, SectionOrder::Default);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SectionOrder {
    /// Default order: Summary, Workstreams, Receipts, Coverage
    #[default]
    Default,
    /// Alternative order: Coverage, Summary, Workstreams, Receipts
    CoverageFirst,
}

/// Controls appendix density for rendered Markdown packets.
///
/// The default keeps the historical full appendix behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppendixMode {
    /// Render every assigned event in the appendix.
    #[default]
    Full,
    /// Render per-workstream receipt counts instead of every event.
    Summary,
    /// Omit the appendix.
    None,
}

/// Markdown packet density controls.
///
/// These options affect the human-facing packet shape without changing the
/// canonical ledger, coverage manifest, or workstream files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarkdownRenderOptions {
    /// Maximum curated receipts shown per workstream in the main receipts section.
    pub receipt_limit: usize,
    /// Appendix detail level.
    pub appendix_mode: AppendixMode,
}

impl Default for MarkdownRenderOptions {
    fn default() -> Self {
        Self {
            receipt_limit: WORKSTREAM_RECEIPT_RENDER_LIMIT,
            appendix_mode: AppendixMode::Full,
        }
    }
}

/// Minimal renderer that produces a copy-ready Markdown packet.
///
/// The output is intentionally low-magic:
/// - headings
/// - short bullets
/// - receipts with URLs when available
///
/// # Examples
///
/// Use as a [`Renderer`] trait object to render events into Markdown:
///
/// ```rust,no_run
/// use shiplog_render_md::MarkdownRenderer;
/// use shiplog_ports::Renderer;
/// use shiplog_schema::event::EventEnvelope;
/// use shiplog_schema::workstream::WorkstreamsFile;
/// use shiplog_schema::coverage::CoverageManifest;
///
/// # fn example(
/// #     events: &[EventEnvelope],
/// #     workstreams: &WorkstreamsFile,
/// #     coverage: &CoverageManifest,
/// # ) -> anyhow::Result<()> {
/// let renderer = MarkdownRenderer::new();
/// let markdown = renderer.render_packet_markdown(
///     "octocat",
///     "2025-01-01..2025-04-01",
///     events,
///     workstreams,
///     coverage,
/// )?;
/// println!("{}", markdown);
/// # Ok(())
/// # }
/// ```
pub struct MarkdownRenderer {
    /// Section ordering configuration
    pub section_order: SectionOrder,
}

impl Default for MarkdownRenderer {
    fn default() -> Self {
        Self {
            section_order: SectionOrder::Default,
        }
    }
}

impl MarkdownRenderer {
    /// Create a new renderer with default section ordering.
    ///
    /// # Examples
    ///
    /// ```
    /// use shiplog_render_md::MarkdownRenderer;
    ///
    /// let renderer = MarkdownRenderer::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new renderer with custom section ordering.
    ///
    /// # Examples
    ///
    /// ```
    /// use shiplog_render_md::{MarkdownRenderer, SectionOrder};
    ///
    /// let renderer = MarkdownRenderer::new()
    ///     .with_section_order(SectionOrder::CoverageFirst);
    /// ```
    pub fn with_section_order(mut self, order: SectionOrder) -> Self {
        self.section_order = order;
        self
    }

    /// Render a writing scaffold with coverage, workstream prompts, and evidence anchors.
    pub fn render_scaffold_markdown(
        &self,
        user: &str,
        window_label: &str,
        events: &[EventEnvelope],
        workstreams: &WorkstreamsFile,
        coverage: &CoverageManifest,
    ) -> Result<String> {
        self.render_scaffold_markdown_with_options(
            user,
            window_label,
            events,
            workstreams,
            coverage,
            MarkdownRenderOptions::default(),
        )
    }

    /// Render a writing scaffold with explicit packet density controls.
    ///
    /// Scaffold mode currently omits the full receipts and appendix sections,
    /// so appendix options are accepted for API symmetry and future expansion.
    pub fn render_scaffold_markdown_with_options(
        &self,
        user: &str,
        window_label: &str,
        events: &[EventEnvelope],
        workstreams: &WorkstreamsFile,
        coverage: &CoverageManifest,
        _options: MarkdownRenderOptions,
    ) -> Result<String> {
        let mut out = String::new();
        render_coverage(&mut out, coverage, events);
        render_summary(&mut out, user, window_label, events, workstreams, coverage);
        render_workstreams(&mut out, events, workstreams);
        render_file_artifacts(&mut out);
        Ok(out)
    }

    /// Render a dense receipts view for audit and appendix review.
    pub fn render_receipts_markdown(
        &self,
        user: &str,
        window_label: &str,
        events: &[EventEnvelope],
        workstreams: &WorkstreamsFile,
        coverage: &CoverageManifest,
    ) -> Result<String> {
        self.render_receipts_markdown_with_options(
            user,
            window_label,
            events,
            workstreams,
            coverage,
            MarkdownRenderOptions::default(),
        )
    }

    /// Render a dense receipts view with explicit packet density controls.
    pub fn render_receipts_markdown_with_options(
        &self,
        user: &str,
        window_label: &str,
        events: &[EventEnvelope],
        workstreams: &WorkstreamsFile,
        coverage: &CoverageManifest,
        options: MarkdownRenderOptions,
    ) -> Result<String> {
        let mut out = String::new();
        render_summary(&mut out, user, window_label, events, workstreams, coverage);
        render_coverage(&mut out, coverage, events);
        render_receipts(&mut out, events, workstreams, options);
        render_appendix(&mut out, events, workstreams, options.appendix_mode);
        render_file_artifacts(&mut out);
        Ok(out)
    }

    /// Render the default packet with explicit packet density controls.
    pub fn render_packet_markdown_with_options(
        &self,
        user: &str,
        window_label: &str,
        events: &[EventEnvelope],
        workstreams: &WorkstreamsFile,
        coverage: &CoverageManifest,
        options: MarkdownRenderOptions,
    ) -> Result<String> {
        let mut out = String::new();

        // Render sections based on configured order
        match self.section_order {
            SectionOrder::Default => {
                render_summary(&mut out, user, window_label, events, workstreams, coverage);
                render_workstreams(&mut out, events, workstreams);
                render_receipts(&mut out, events, workstreams, options);
                render_coverage(&mut out, coverage, events);
            }
            SectionOrder::CoverageFirst => {
                render_coverage(&mut out, coverage, events);
                render_summary(&mut out, user, window_label, events, workstreams, coverage);
                render_workstreams(&mut out, events, workstreams);
                render_receipts(&mut out, events, workstreams, options);
            }
        }

        render_appendix(&mut out, events, workstreams, options.appendix_mode);
        render_file_artifacts(&mut out);

        Ok(out)
    }
}

impl Renderer for MarkdownRenderer {
    fn render_packet_markdown(
        &self,
        user: &str,
        window_label: &str,
        events: &[EventEnvelope],
        workstreams: &WorkstreamsFile,
        coverage: &CoverageManifest,
    ) -> Result<String> {
        self.render_packet_markdown_with_options(
            user,
            window_label,
            events,
            workstreams,
            coverage,
            MarkdownRenderOptions::default(),
        )
    }
}

fn render_summary(
    out: &mut String,
    _user: &str,
    window_label: &str,
    events: &[EventEnvelope],
    workstreams: &WorkstreamsFile,
    coverage: &CoverageManifest,
) {
    out.push_str("# Summary\n\n");

    // Window
    out.push_str(&format!("**Window:** {}\n\n", window_label));

    // Workstream count
    out.push_str(&format!(
        "**Workstreams:** {}\n\n",
        workstreams.workstreams.len()
    ));

    // Event counts by type
    let pr_count = events
        .iter()
        .filter(|e| matches!(e.kind, EventKind::PullRequest))
        .count();
    let review_count = events
        .iter()
        .filter(|e| matches!(e.kind, EventKind::Review))
        .count();
    let manual_count = events
        .iter()
        .filter(|e| matches!(e.kind, EventKind::Manual))
        .count();
    out.push_str(&format!(
        "**Events:** {} PRs, {} reviews, {} manual\n\n",
        pr_count, review_count, manual_count
    ));

    // Completeness
    out.push_str(&format!("**Coverage:** {:?}\n\n", coverage.completeness));

    // Sources
    out.push_str(&format!("**Sources:** {}\n\n", coverage.sources.join(", ")));

    // Warnings
    if !coverage.warnings.is_empty() {
        out.push_str("**Warnings:**\n");
        for w in &coverage.warnings {
            out.push_str(&format!("  - ⚠️ {}\n", w));
        }
        out.push('\n');
    }
}

fn render_workstreams(out: &mut String, events: &[EventEnvelope], workstreams: &WorkstreamsFile) {
    out.push_str("## Workstreams\n\n");

    if workstreams.workstreams.is_empty() {
        out.push_str("_No workstreams found_\n\n");
        return;
    }

    let by_id: HashMap<String, &EventEnvelope> =
        events.iter().map(|e| (e.id.0.clone(), e)).collect();

    for ws in &workstreams.workstreams {
        out.push_str(&format!("### {}\n\n", ws.title));

        if let Some(s) = &ws.summary {
            out.push_str(s);
            out.push_str("\n\n");
        }

        render_evidence_anchors(out, &by_id, ws);
        render_claim_prompts(out);

        // Stats
        out.push_str(&format!(
            "_PRs: {}, Reviews: {}, Manual: {}_\n\n",
            ws.stats.pull_requests, ws.stats.reviews, ws.stats.manual_events
        ));
    }
}

fn render_evidence_anchors(
    out: &mut String,
    by_id: &HashMap<String, &EventEnvelope>,
    workstream: &Workstream,
) {
    out.push_str("**Evidence anchors**\n\n");

    let available: Vec<_> = workstream
        .receipts
        .iter()
        .filter_map(|id| by_id.get(&id.0).copied())
        .collect();

    if available.is_empty() {
        out.push_str("- (none)\n\n");
        return;
    }

    for event in available.iter().take(WORKSTREAM_EVIDENCE_ANCHOR_LIMIT) {
        out.push_str(&format!("{}\n", format_receipt_markdown(event)));
    }

    let remaining = available
        .len()
        .saturating_sub(WORKSTREAM_EVIDENCE_ANCHOR_LIMIT);
    if remaining > 0 {
        out.push_str(&format!(
            "- ... and {remaining} more in [Receipts](#receipts)\n"
        ));
    }
    out.push('\n');
}

fn render_claim_prompts(out: &mut String) {
    out.push_str("**Suggested claim prompts**\n\n");
    out.push_str("- What changed for users, operators, or maintainers?\n");
    out.push_str("- Which risk, delay, or repeated work did this reduce?\n");
    out.push_str("- Which evidence anchor best proves the change?\n");
    out.push_str("- What follow-up or gap should a reviewer know about?\n\n");
}

fn render_receipts(
    out: &mut String,
    events: &[EventEnvelope],
    workstreams: &WorkstreamsFile,
    options: MarkdownRenderOptions,
) {
    out.push_str("## Receipts\n\n");

    if workstreams.workstreams.is_empty() {
        out.push_str("_No workstreams, no receipts_\n\n");
        return;
    }

    let by_id: HashMap<String, &EventEnvelope> =
        events.iter().map(|e| (e.id.0.clone(), e)).collect();

    for ws in &workstreams.workstreams {
        out.push_str(&format!("### Workstream: {}\n\n", ws.title));

        // Split receipts into main (top N) and appendix (remainder)
        let (main_receipts, appendix_receipts): (Vec<_>, Vec<_>) = if ws.receipts.is_empty() {
            (Vec::new(), Vec::new())
        } else if ws.receipts.len() <= options.receipt_limit {
            (ws.receipts.clone(), Vec::new())
        } else if options.receipt_limit == 0 {
            (Vec::new(), ws.receipts.clone())
        } else {
            let (main, appendix) = ws.receipts.split_at(options.receipt_limit);
            (main.to_vec(), appendix.to_vec())
        };

        if main_receipts.is_empty() {
            out.push_str("- (none)\n");
        } else {
            for id in &main_receipts {
                if let Some(ev) = by_id.get(&id.0) {
                    out.push_str(&format!("{}\n", format_receipt_markdown(ev)));
                }
            }
        }

        if !appendix_receipts.is_empty() {
            out.push_str(&appendix_receipt_note(
                appendix_receipts.len(),
                options.appendix_mode,
            ));
        }
        out.push('\n');
    }
}

fn appendix_receipt_note(count: usize, mode: AppendixMode) -> String {
    match mode {
        AppendixMode::Full => {
            format!("- *... and {count} more in [Appendix](#appendix-receipts)*\n")
        }
        AppendixMode::Summary => {
            format!(
                "- *... and {count} more summarized in [Appendix](#appendix-receipt-summary)*\n"
            )
        }
        AppendixMode::None => format!("- *... and {count} more omitted by appendix settings*\n"),
    }
}

fn render_coverage(out: &mut String, coverage: &CoverageManifest, events: &[EventEnvelope]) {
    out.push_str("## Coverage and Limits\n\n");

    out.push_str("Included:\n");
    let skipped_sources = skipped_source_warnings(&coverage.warnings);
    let included_sources = included_source_summary(coverage, events, &skipped_sources);
    if included_sources.is_empty() {
        out.push_str("- No completed sources recorded\n");
    } else {
        for source in &included_sources {
            let count = source_event_count(events, source);
            let noun = if count == 1 { "event" } else { "events" };
            out.push_str(&format!(
                "- {}: {} {}\n",
                display_source_label(source),
                count,
                noun
            ));
        }
    }

    if coverage.slices.is_empty() {
        out.push_str("- Fetched events: not reported by query slices\n");
    } else {
        let fetched: u64 = coverage.slices.iter().map(|slice| slice.fetched).sum();
        let total: u64 = coverage.slices.iter().map(|slice| slice.total_count).sum();
        let slice_label = if coverage.slices.len() == 1 {
            "slice"
        } else {
            "slices"
        };
        out.push_str(&format!(
            "- Query slices: {} {}, fetched {} of {} reported results\n",
            coverage.slices.len(),
            slice_label,
            fetched,
            total
        ));
    }
    out.push('\n');

    out.push_str("Skipped:\n");
    if skipped_sources.is_empty() {
        out.push_str("- None recorded\n");
    } else {
        for skipped in &skipped_sources {
            out.push_str(&format!(
                "- {}: {}\n",
                display_source_label(skipped.source),
                skipped.reason
            ));
        }
    }
    out.push('\n');

    out.push_str("Known gaps:\n");
    let mut has_gap = false;
    if !matches!(
        coverage.completeness,
        shiplog_schema::coverage::Completeness::Complete
    ) {
        has_gap = true;
        out.push_str(&format!(
            "- Overall completeness is {}\n",
            coverage.completeness
        ));
    }
    for warning in &coverage.warnings {
        if skipped_source_warning(warning).is_none() {
            has_gap = true;
            out.push_str(&format!("- {}\n", warning));
        }
    }

    if source_present(&coverage.sources, "manual")
        || events
            .iter()
            .any(|event| source_matches(event.source.system.as_str(), "manual"))
    {
        has_gap = true;
        out.push_str("- Manual events are user-provided\n");
    }

    let incomplete_count = coverage
        .slices
        .iter()
        .filter(|slice| slice.incomplete_results.unwrap_or(false))
        .count();
    if incomplete_count > 0 {
        has_gap = true;
        let slice_label = if incomplete_count == 1 {
            "slice"
        } else {
            "slices"
        };
        out.push_str(&format!(
            "- {} query {} reported incomplete results\n",
            incomplete_count, slice_label
        ));
    }

    let capped_count = coverage
        .slices
        .iter()
        .filter(|slice| slice.total_count > slice.fetched)
        .count();
    if capped_count > 0 {
        has_gap = true;
        let slice_label = if capped_count == 1 { "slice" } else { "slices" };
        out.push_str(&format!(
            "- {} query {} fetched fewer results than reported\n",
            capped_count, slice_label
        ));
    }

    if !has_gap {
        out.push_str("- None recorded\n");
    }
    out.push('\n');

    out.push_str("Details:\n");

    // Date window
    out.push_str(&format!(
        "- **Date window:** {} to {}\n",
        coverage.window.since, coverage.window.until
    ));

    // Mode
    out.push_str(&format!("- **Mode:** {}\n", coverage.mode));

    // Sources
    let source_details = if coverage.sources.is_empty() {
        "none recorded".to_string()
    } else {
        coverage.sources.join(", ")
    };
    out.push_str(&format!("- **Sources:** {}\n", source_details));

    // Completeness
    out.push_str(&format!(
        "- **Completeness:** {:?}\n",
        coverage.completeness
    ));

    // Coverage slicing details
    if !coverage.slices.is_empty() {
        out.push_str(&format!("- **Query slices:** {}\n", coverage.slices.len()));

        // Check for partial results or caps
        let partial_count = coverage
            .slices
            .iter()
            .filter(|s| s.incomplete_results.unwrap_or(false))
            .count();
        if partial_count > 0 {
            out.push_str(&format!(
                "  - ⚠️ {} slices had incomplete results\n",
                partial_count
            ));
        }

        // Show slices that hit caps
        let capped_slices: Vec<_> = coverage
            .slices
            .iter()
            .filter(|s| s.total_count > s.fetched)
            .collect();
        if !capped_slices.is_empty() {
            out.push_str("  - **Slicing applied (API caps):**\n");
            for slice in capped_slices.iter().take(3) {
                let pct = if slice.total_count > 0 {
                    (slice.fetched as f64 / slice.total_count as f64 * 100.0) as u64
                } else {
                    100
                };
                out.push_str(&format!(
                    "    - {}: fetched {}/{} ({}%)\n",
                    slice.query, slice.fetched, slice.total_count, pct
                ));
            }
            if capped_slices.len() > 3 {
                out.push_str(&format!("    - ... and {} more\n", capped_slices.len() - 3));
            }
        }
    }
    out.push('\n');
}

#[derive(Clone, Copy, Debug)]
struct SkippedSource<'a> {
    source: &'a str,
    reason: &'a str,
}

fn skipped_source_warnings(warnings: &[String]) -> Vec<SkippedSource<'_>> {
    warnings
        .iter()
        .filter_map(|warning| skipped_source_warning(warning))
        .collect()
}

fn skipped_source_warning(warning: &str) -> Option<SkippedSource<'_>> {
    const PREFIX: &str = "Configured source ";
    const INFIX: &str = " was skipped: ";

    let rest = warning.strip_prefix(PREFIX)?;
    let (source, reason) = rest.split_once(INFIX)?;
    Some(SkippedSource { source, reason })
}

fn included_source_summary(
    coverage: &CoverageManifest,
    events: &[EventEnvelope],
    skipped_sources: &[SkippedSource<'_>],
) -> Vec<String> {
    let mut sources = Vec::new();
    for source in &coverage.sources {
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

fn source_event_count(events: &[EventEnvelope], source: &str) -> usize {
    events
        .iter()
        .filter(|event| source_matches(event.source.system.as_str(), source))
        .count()
}

fn source_present(sources: &[String], needle: &str) -> bool {
    sources.iter().any(|source| source_matches(source, needle))
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

fn display_source_label(source: &str) -> String {
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

fn render_appendix(
    out: &mut String,
    events: &[EventEnvelope],
    workstreams: &WorkstreamsFile,
    mode: AppendixMode,
) {
    match mode {
        AppendixMode::Full => render_full_appendix(out, events, workstreams),
        AppendixMode::Summary => render_appendix_summary(out, workstreams),
        AppendixMode::None => {}
    }
}

fn render_full_appendix(out: &mut String, events: &[EventEnvelope], workstreams: &WorkstreamsFile) {
    out.push_str("## Appendix: All Receipts\n\n");

    if workstreams.workstreams.is_empty() {
        return;
    }

    let by_id: HashMap<String, &EventEnvelope> =
        events.iter().map(|e| (e.id.0.clone(), e)).collect();

    for ws in &workstreams.workstreams {
        if ws.events.is_empty() {
            continue;
        }

        out.push_str(&format!("### {}\n\n", ws.title));

        // Show all events for this workstream, not just receipts
        for event_id in &ws.events {
            if let Some(ev) = by_id.get(&event_id.0) {
                out.push_str(&format!("{}\n", format_receipt_markdown(ev)));
            }
        }
        out.push('\n');
    }
    out.push_str("---\n\n");
}

fn render_appendix_summary(out: &mut String, workstreams: &WorkstreamsFile) {
    out.push_str("## Appendix: Receipt Summary\n\n");

    if workstreams.workstreams.is_empty() {
        return;
    }

    for ws in &workstreams.workstreams {
        out.push_str(&format!("### {}\n\n", ws.title));
        out.push_str(&format!("- Assigned events: {}\n", ws.events.len()));
        out.push_str(&format!(
            "- Curated receipt anchors: {}\n",
            ws.receipts.len()
        ));
        out.push_str("- Full receipt detail omitted by appendix summary mode.\n\n");
    }
    out.push_str("---\n\n");
}

fn render_file_artifacts(out: &mut String) {
    out.push_str("## File Artifacts\n\n");
    out.push_str("- `ledger.events.jsonl` (canonical events)\n");
    out.push_str("- `coverage.manifest.json` (completeness + slicing)\n");
    out.push_str("- `workstreams.yaml` (editable clustering)\n");
    out.push_str("- `manual_events.yaml` (non-GitHub work)\n");
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, TimeZone, Utc};
    use shiplog_ids::{EventId, RunId, WorkstreamId};
    use shiplog_schema::coverage::*;
    use shiplog_schema::event::*;
    use shiplog_schema::workstream::*;

    fn create_test_pr(id: &str, number: u64, title: &str) -> EventEnvelope {
        EventEnvelope {
            id: EventId::from_parts(["pr", id]),
            kind: EventKind::PullRequest,
            occurred_at: Utc.timestamp_opt(0, 0).unwrap(),
            actor: Actor {
                login: "octo".into(),
                id: None,
            },
            repo: RepoRef {
                full_name: "owner/repo".into(),
                html_url: None,
                visibility: RepoVisibility::Public,
            },
            payload: EventPayload::PullRequest(PullRequestEvent {
                number,
                title: title.into(),
                state: PullRequestState::Merged,
                created_at: Utc.timestamp_opt(0, 0).unwrap(),
                merged_at: Some(Utc.timestamp_opt(0, 0).unwrap()),
                additions: Some(10),
                deletions: Some(5),
                changed_files: Some(2),
                touched_paths_hint: vec![],
                window: None,
            }),
            tags: vec![],
            links: vec![Link {
                label: "pr".into(),
                url: format!("https://github.com/owner/repo/pull/{}", number),
            }],
            source: SourceRef {
                system: SourceSystem::Github,
                url: None,
                opaque_id: Some(id.into()),
            },
        }
    }

    fn create_test_manual(id: &str, event_type: ManualEventType, title: &str) -> EventEnvelope {
        EventEnvelope {
            id: EventId::from_parts(["manual", id]),
            kind: EventKind::Manual,
            occurred_at: Utc.timestamp_opt(0, 0).unwrap(),
            actor: Actor {
                login: "user".into(),
                id: None,
            },
            repo: RepoRef {
                full_name: "owner/repo".into(),
                html_url: None,
                visibility: RepoVisibility::Public,
            },
            payload: EventPayload::Manual(ManualEvent {
                event_type: event_type.clone(),
                title: title.into(),
                description: None,
                started_at: None,
                ended_at: None,
                impact: None,
            }),
            tags: vec![],
            links: vec![],
            source: SourceRef {
                system: SourceSystem::Manual,
                url: None,
                opaque_id: Some(id.into()),
            },
        }
    }

    #[test]
    fn test_snapshot_empty_packet() {
        let renderer = MarkdownRenderer::new();
        let events: Vec<EventEnvelope> = vec![];
        let workstreams = WorkstreamsFile {
            version: 1,
            generated_at: Utc::now(),
            workstreams: vec![],
        };
        let coverage = CoverageManifest {
            run_id: RunId::now("test"),
            generated_at: Utc::now(),
            user: "test".into(),
            window: TimeWindow {
                since: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                until: NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
            },
            mode: "test".into(),
            sources: vec![],
            slices: vec![],
            warnings: vec![],
            completeness: Completeness::Complete,
        };

        let result = renderer
            .render_packet_markdown("test", "2024", &events, &workstreams, &coverage)
            .unwrap();

        insta::assert_snapshot!(result);
    }

    #[test]
    fn test_snapshot_full_packet() {
        let renderer = MarkdownRenderer::new();
        let events = vec![
            create_test_pr("1", 1, "Fix authentication bug"),
            create_test_manual("2", ManualEventType::Incident, "Handle production incident"),
        ];
        let workstreams = WorkstreamsFile {
            version: 1,
            generated_at: Utc::now(),
            workstreams: vec![Workstream {
                id: WorkstreamId::from_parts(["ws", "1"]),
                title: "Authentication".into(),
                summary: Some("Fixed auth bugs and improved security".into()),
                tags: vec![],
                receipts: vec![EventId::from_parts(["pr", "1"])],
                events: vec![EventId::from_parts(["pr", "1"])],
                stats: WorkstreamStats {
                    pull_requests: 1,
                    reviews: 0,
                    manual_events: 0,
                },
            }],
        };
        let coverage = CoverageManifest {
            run_id: RunId::now("test"),
            generated_at: Utc::now(),
            user: "test".into(),
            window: TimeWindow {
                since: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                until: NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
            },
            mode: "test".into(),
            sources: vec!["github".into(), "manual".into()],
            slices: vec![],
            warnings: vec![],
            completeness: Completeness::Complete,
        };

        let result = renderer
            .render_packet_markdown("test", "2024", &events, &workstreams, &coverage)
            .unwrap();

        insta::assert_snapshot!(result);
    }

    #[test]
    fn test_snapshot_partial_coverage() {
        let renderer = MarkdownRenderer::new();
        let events = vec![create_test_pr("1", 1, "Add feature")];
        let workstreams = WorkstreamsFile {
            version: 1,
            generated_at: Utc::now(),
            workstreams: vec![Workstream {
                id: WorkstreamId::from_parts(["ws", "1"]),
                title: "Feature".into(),
                summary: None,
                tags: vec![],
                receipts: vec![EventId::from_parts(["pr", "1"])],
                events: vec![EventId::from_parts(["pr", "1"])],
                stats: WorkstreamStats {
                    pull_requests: 1,
                    reviews: 0,
                    manual_events: 0,
                },
            }],
        };
        let coverage = CoverageManifest {
            run_id: RunId::now("test"),
            generated_at: Utc::now(),
            user: "test".into(),
            window: TimeWindow {
                since: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                until: NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
            },
            mode: "test".into(),
            sources: vec!["github".into()],
            slices: vec![CoverageSlice {
                window: TimeWindow {
                    since: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                    until: NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
                },
                query: "test".into(),
                total_count: 100,
                fetched: 50,
                incomplete_results: Some(true),
                notes: vec![],
            }],
            warnings: vec!["API rate limit hit".into()],
            completeness: Completeness::Partial,
        };

        let result = renderer
            .render_packet_markdown("test", "2024", &events, &workstreams, &coverage)
            .unwrap();

        insta::assert_snapshot!(result);
    }

    #[test]
    fn test_snapshot_coverage_first_section_order() {
        let renderer = MarkdownRenderer::new().with_section_order(SectionOrder::CoverageFirst);
        let events = vec![
            create_test_pr("1", 1, "Fix bug"),
            create_test_pr("2", 2, "Add feature"),
        ];
        let workstreams = WorkstreamsFile {
            version: 1,
            generated_at: Utc::now(),
            workstreams: vec![Workstream {
                id: WorkstreamId::from_parts(["ws", "1"]),
                title: "Workstream 1".into(),
                summary: Some("Summary".into()),
                tags: vec![],
                receipts: vec![EventId::from_parts(["pr", "1"])],
                events: vec![EventId::from_parts(["pr", "1"])],
                stats: WorkstreamStats {
                    pull_requests: 1,
                    reviews: 0,
                    manual_events: 0,
                },
            }],
        };
        let coverage = CoverageManifest {
            run_id: RunId::now("test"),
            generated_at: Utc::now(),
            user: "test".into(),
            window: TimeWindow {
                since: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                until: NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
            },
            mode: "merged".into(),
            sources: vec!["github".into()],
            slices: vec![],
            warnings: vec![],
            completeness: Completeness::Complete,
        };

        let result = renderer
            .render_packet_markdown("testuser", "2024-W01", &events, &workstreams, &coverage)
            .unwrap();

        // Coverage should appear first
        assert!(result.starts_with("## Coverage"));
    }

    #[test]
    fn test_snapshot_events_with_reviews() {
        let renderer = MarkdownRenderer::new();
        let pr_event = create_test_pr("1", 1, "Add feature");
        let review_event = EventEnvelope {
            id: EventId::from_parts(["review", "1"]),
            kind: EventKind::Review,
            occurred_at: Utc.timestamp_opt(0, 0).unwrap(),
            actor: Actor {
                login: "reviewer".into(),
                id: None,
            },
            repo: RepoRef {
                full_name: "owner/repo".into(),
                html_url: None,
                visibility: RepoVisibility::Public,
            },
            payload: EventPayload::Review(ReviewEvent {
                pull_number: 1,
                pull_title: "Add feature".into(),
                submitted_at: Utc.timestamp_opt(0, 0).unwrap(),
                state: "approved".into(),
                window: None,
            }),
            tags: vec![],
            links: vec![Link {
                label: "pr".into(),
                url: "https://github.com/owner/repo/pull/1".into(),
            }],
            source: SourceRef {
                system: SourceSystem::Github,
                url: None,
                opaque_id: None,
            },
        };
        let events = vec![pr_event, review_event];
        let workstreams = WorkstreamsFile {
            version: 1,
            generated_at: Utc::now(),
            workstreams: vec![Workstream {
                id: WorkstreamId::from_parts(["ws", "1"]),
                title: "Feature Work".into(),
                summary: None,
                tags: vec![],
                receipts: vec![],
                events: vec![
                    EventId::from_parts(["pr", "1"]),
                    EventId::from_parts(["review", "1"]),
                ],
                stats: WorkstreamStats {
                    pull_requests: 1,
                    reviews: 1,
                    manual_events: 0,
                },
            }],
        };
        let coverage = CoverageManifest {
            run_id: RunId::now("test"),
            generated_at: Utc::now(),
            user: "test".into(),
            window: TimeWindow {
                since: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                until: NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
            },
            mode: "test".into(),
            sources: vec!["github".into()],
            slices: vec![],
            warnings: vec![],
            completeness: Completeness::Complete,
        };

        let result = renderer
            .render_packet_markdown("test", "2024", &events, &workstreams, &coverage)
            .unwrap();

        // Should show both PRs and reviews in summary
        assert!(result.contains("1 PRs, 1 reviews"));
    }

    #[test]
    fn test_snapshot_multiple_workstreams() {
        let renderer = MarkdownRenderer::new();
        let events = vec![
            create_test_pr("1", 1, "Feature A"),
            create_test_pr("2", 2, "Feature B"),
            create_test_pr("3", 3, "Fix bug"),
        ];
        let workstreams = WorkstreamsFile {
            version: 1,
            generated_at: Utc::now(),
            workstreams: vec![
                Workstream {
                    id: WorkstreamId::from_parts(["ws", "a"]),
                    title: "Feature A".into(),
                    summary: Some("Work on feature A".into()),
                    tags: vec![],
                    receipts: vec![EventId::from_parts(["pr", "1"])],
                    events: vec![EventId::from_parts(["pr", "1"])],
                    stats: WorkstreamStats {
                        pull_requests: 1,
                        reviews: 0,
                        manual_events: 0,
                    },
                },
                Workstream {
                    id: WorkstreamId::from_parts(["ws", "b"]),
                    title: "Feature B & Bugfix".into(),
                    summary: Some("Work on feature B and bugfix".into()),
                    tags: vec![],
                    receipts: vec![
                        EventId::from_parts(["pr", "2"]),
                        EventId::from_parts(["pr", "3"]),
                    ],
                    events: vec![
                        EventId::from_parts(["pr", "2"]),
                        EventId::from_parts(["pr", "3"]),
                    ],
                    stats: WorkstreamStats {
                        pull_requests: 2,
                        reviews: 0,
                        manual_events: 0,
                    },
                },
            ],
        };
        let coverage = CoverageManifest {
            run_id: RunId::now("test"),
            generated_at: Utc::now(),
            user: "test".into(),
            window: TimeWindow {
                since: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                until: NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
            },
            mode: "test".into(),
            sources: vec!["github".into()],
            slices: vec![],
            warnings: vec![],
            completeness: Completeness::Complete,
        };

        let result = renderer
            .render_packet_markdown("test", "2024", &events, &workstreams, &coverage)
            .unwrap();

        // Should show 2 workstreams
        assert!(result.contains("**Workstreams:** 2"));
    }

    fn create_test_review(id: &str, state: &str, with_link: bool) -> EventEnvelope {
        let links = if with_link {
            vec![Link {
                label: "pr".into(),
                url: "https://github.com/owner/repo/pull/42".to_string(),
            }]
        } else {
            vec![]
        };
        EventEnvelope {
            id: EventId::from_parts(["review", id]),
            kind: EventKind::Review,
            occurred_at: Utc.timestamp_opt(0, 0).unwrap(),
            actor: Actor {
                login: "reviewer".into(),
                id: None,
            },
            repo: RepoRef {
                full_name: "owner/repo".into(),
                html_url: None,
                visibility: RepoVisibility::Public,
            },
            payload: EventPayload::Review(ReviewEvent {
                pull_number: 42,
                pull_title: "Some PR".into(),
                submitted_at: Utc.timestamp_opt(0, 0).unwrap(),
                state: state.into(),
                window: None,
            }),
            tags: vec![],
            links,
            source: SourceRef {
                system: SourceSystem::Github,
                url: None,
                opaque_id: Some(id.into()),
            },
        }
    }

    fn make_coverage(slices: Vec<CoverageSlice>, warnings: Vec<String>) -> CoverageManifest {
        CoverageManifest {
            run_id: RunId::now("test"),
            generated_at: Utc::now(),
            user: "test".into(),
            window: TimeWindow {
                since: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                until: NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
            },
            mode: "test".into(),
            sources: vec!["github".into()],
            slices,
            warnings,
            completeness: Completeness::Complete,
        }
    }

    #[test]
    fn coverage_complete_slices_no_incomplete_message() {
        // All slices complete → output does NOT contain "incomplete results"
        // Kills >0 → >=0 mutation on partial_count check
        let coverage = make_coverage(
            vec![CoverageSlice {
                window: TimeWindow {
                    since: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                    until: NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
                },
                query: "test".into(),
                total_count: 10,
                fetched: 10,
                incomplete_results: Some(false),
                notes: vec![],
            }],
            vec![],
        );
        let mut out = String::new();
        render_coverage(&mut out, &coverage, &[]);
        assert!(!out.contains("incomplete results"));
    }

    #[test]
    fn coverage_with_total_equal_fetched_no_slicing_message() {
        // total_count == fetched → should NOT show "Slicing applied"
        // Kills > → >= mutation on `total_count > fetched`
        let coverage = make_coverage(
            vec![CoverageSlice {
                window: TimeWindow {
                    since: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                    until: NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
                },
                query: "test".into(),
                total_count: 50,
                fetched: 50,
                incomplete_results: Some(false),
                notes: vec![],
            }],
            vec![],
        );
        let mut out = String::new();
        render_coverage(&mut out, &coverage, &[]);
        assert!(!out.contains("Slicing applied"));
    }

    #[test]
    fn coverage_summary_complete_lists_no_known_gaps() {
        let coverage = make_coverage(vec![], vec![]);
        let mut out = String::new();
        render_coverage(&mut out, &coverage, &[]);
        assert!(out.contains("## Coverage and Limits"));
        assert!(out.contains("Included:\n- GitHub: 0 events\n"));
        assert!(out.contains("Skipped:\n- None recorded\n"));
        assert!(out.contains("Known gaps:\n- None recorded\n"));
    }

    #[test]
    fn coverage_summary_lists_source_event_counts_and_manual_gap() {
        let events = vec![
            create_test_pr("1", 1, "Ship API"),
            create_test_manual("2", ManualEventType::Incident, "Incident follow-up"),
        ];
        let mut coverage = make_coverage(vec![], vec![]);
        coverage.sources = vec!["github".into(), "manual".into()];

        let mut out = String::new();
        render_coverage(&mut out, &coverage, &events);

        assert!(out.contains("Included:\n- GitHub: 1 event\n- Manual: 1 event\n"));
        assert!(out.contains("Skipped:\n- None recorded\n"));
        assert!(out.contains("Known gaps:\n- Manual events are user-provided\n"));
    }

    #[test]
    fn coverage_summary_includes_event_provenance_not_only_manifest_sources() {
        let events = vec![
            create_test_pr("1", 1, "Ship API"),
            create_test_manual("2", ManualEventType::Incident, "Incident follow-up"),
        ];
        let mut coverage = make_coverage(vec![], vec![]);
        coverage.sources = vec!["github".into()];

        let mut out = String::new();
        render_coverage(&mut out, &coverage, &events);

        assert!(out.contains("Included:\n- GitHub: 1 event\n- Manual: 1 event\n"));
        assert!(out.contains("Known gaps:\n- Manual events are user-provided\n"));
    }

    #[test]
    fn coverage_summary_lists_skipped_configured_sources() {
        let events = vec![create_test_manual(
            "manual-1",
            ManualEventType::Note,
            "Manual note",
        )];
        let mut coverage = make_coverage(
            vec![],
            vec!["Configured source json was skipped: missing coverage".into()],
        );
        coverage.sources = vec!["json".into(), "manual".into()];
        coverage.completeness = Completeness::Partial;

        let mut out = String::new();
        render_coverage(&mut out, &coverage, &events);

        assert!(out.contains("Included:\n- Manual: 1 event\n"));
        let included = out
            .split("Included:")
            .nth(1)
            .expect("coverage should include Included block")
            .split("Skipped:")
            .next()
            .expect("coverage should include Skipped after Included");
        assert!(!included.contains("JSON"));
        assert!(out.contains("Skipped:\n- JSON: missing coverage\n"));
        let known_gaps = out
            .split("Known gaps:")
            .nth(1)
            .expect("coverage should include Known gaps block")
            .split("Details:")
            .next()
            .expect("coverage should include Details after Known gaps");
        assert!(known_gaps.contains("- Overall completeness is Partial"));
        assert!(!known_gaps.contains("Configured source json was skipped"));
    }

    #[test]
    fn coverage_summary_keeps_event_provenance_when_configured_source_skipped() {
        let events = vec![create_test_pr("1", 1, "Imported GitHub evidence")];
        let mut coverage = make_coverage(
            vec![],
            vec!["Configured source github was skipped: token missing".into()],
        );
        coverage.sources = vec!["github".into(), "json".into()];
        coverage.completeness = Completeness::Partial;

        let mut out = String::new();
        render_coverage(&mut out, &coverage, &events);

        let included = out
            .split("Included:")
            .nth(1)
            .expect("coverage should include Included block")
            .split("Skipped:")
            .next()
            .expect("coverage should include Skipped after Included");
        assert!(included.contains("- GitHub: 1 event\n"));
        assert!(out.contains("Skipped:\n- GitHub: token missing\n"));
    }

    #[test]
    fn coverage_summary_does_not_collapse_distinct_custom_sources() {
        let mut custom_slash = create_test_manual("1", ManualEventType::Note, "Slash source");
        custom_slash.source.system = SourceSystem::Other("custom/system".into());
        let mut custom_dash = create_test_manual("2", ManualEventType::Note, "Dash source");
        custom_dash.source.system = SourceSystem::Other("custom-system".into());
        let mut custom_unicode = create_test_manual("3", ManualEventType::Note, "Unicode source");
        custom_unicode.source.system = SourceSystem::Other("日本語ソース".into());
        let events = vec![custom_slash, custom_dash, custom_unicode];
        let mut coverage = make_coverage(vec![], vec![]);
        coverage.sources = vec![
            "custom/system".into(),
            "custom-system".into(),
            "日本語ソース".into(),
        ];

        let mut out = String::new();
        render_coverage(&mut out, &coverage, &events);

        assert!(out.contains("- custom/system: 1 event\n"));
        assert!(out.contains("- custom-system: 1 event\n"));
        assert!(out.contains("- 日本語ソース: 1 event\n"));
    }

    #[test]
    fn coverage_with_capped_slices_shows_slicing_applied() {
        // total_count > fetched → should show "Slicing applied"
        let coverage = make_coverage(
            vec![CoverageSlice {
                window: TimeWindow {
                    since: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                    until: NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
                },
                query: "test".into(),
                total_count: 100,
                fetched: 50,
                incomplete_results: Some(false),
                notes: vec![],
            }],
            vec![],
        );
        let mut out = String::new();
        render_coverage(&mut out, &coverage, &[]);
        assert!(out.contains("Slicing applied"));
        assert!(out.contains("fetched 50/100"));
    }

    #[test]
    fn coverage_summary_partial_lists_warnings_and_slice_limits() {
        let mut coverage = make_coverage(
            vec![CoverageSlice {
                window: TimeWindow {
                    since: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                    until: NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
                },
                query: "test".into(),
                total_count: 100,
                fetched: 50,
                incomplete_results: Some(true),
                notes: vec![],
            }],
            vec!["API returned partial results".into()],
        );
        coverage.completeness = Completeness::Partial;

        let mut out = String::new();
        render_coverage(&mut out, &coverage, &[]);
        assert!(out.contains("- Query slices: 1 slice, fetched 50 of 100 reported results"));
        assert!(out.contains("- Overall completeness is Partial"));
        assert!(out.contains("- API returned partial results"));
        assert!(out.contains("- 1 query slice reported incomplete results"));
        assert!(out.contains("- 1 query slice fetched fewer results than reported"));
    }

    #[test]
    fn coverage_with_4_plus_capped_slices_shows_and_more() {
        // 4+ capped slices → shows first 3 then "... and N more"
        // Kills >3 → >=3 mutation
        let slices: Vec<CoverageSlice> = (0..5)
            .map(|i| CoverageSlice {
                window: TimeWindow {
                    since: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                    until: NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
                },
                query: format!("query-{i}"),
                total_count: 100,
                fetched: 50,
                incomplete_results: Some(false),
                notes: vec![],
            })
            .collect();
        let coverage = make_coverage(slices, vec![]);
        let mut out = String::new();
        render_coverage(&mut out, &coverage, &[]);
        assert!(out.contains("... and 2 more"));
    }

    #[test]
    fn coverage_with_exactly_3_capped_slices_no_and_more() {
        // Exactly 3 capped slices → no "... and N more"
        // Kills >3 → >=3 (with 3 slices, > 3 is false, so no "and more")
        let slices: Vec<CoverageSlice> = (0..3)
            .map(|i| CoverageSlice {
                window: TimeWindow {
                    since: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                    until: NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
                },
                query: format!("query-{i}"),
                total_count: 100,
                fetched: 50,
                incomplete_results: Some(false),
                notes: vec![],
            })
            .collect();
        let coverage = make_coverage(slices, vec![]);
        let mut out = String::new();
        render_coverage(&mut out, &coverage, &[]);
        assert!(out.contains("Slicing applied"));
        assert!(!out.contains("... and"));
    }

    #[test]
    fn coverage_empty_slices_no_incomplete_no_slicing() {
        // No slices at all → no "incomplete results" or "Slicing applied" messages.
        // Strengthens > → >= mutation coverage on partial_count and capped checks.
        let coverage = make_coverage(vec![], vec![]);
        let mut out = String::new();
        render_coverage(&mut out, &coverage, &[]);
        assert!(!out.contains("incomplete results"));
        assert!(!out.contains("Slicing applied"));
        assert!(!out.contains("Query slices"));
    }

    #[test]
    fn coverage_none_incomplete_results_no_warning() {
        // incomplete_results: None → defaults to false → no warning.
        // Kills > → >= mutation on partial_count when None is present.
        let coverage = make_coverage(
            vec![CoverageSlice {
                window: TimeWindow {
                    since: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                    until: NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
                },
                query: "test".into(),
                total_count: 10,
                fetched: 10,
                incomplete_results: None,
                notes: vec![],
            }],
            vec![],
        );
        let mut out = String::new();
        render_coverage(&mut out, &coverage, &[]);
        assert!(!out.contains("incomplete results"));
    }

    #[test]
    fn review_event_shows_review_tag_and_state() {
        // Kills Review match arm deletion in format_receipt_markdown
        let ev = create_test_review("r1", "approved", false);
        let formatted = format_receipt_markdown(&ev);
        assert!(formatted.contains("[Review]"));
        assert!(formatted.contains("approved"));
    }

    #[test]
    fn review_with_pr_link_shows_markdown_link() {
        // Kills == → != mutation on `l.label == "pr"` check
        let ev = create_test_review("r2", "changes_requested", true);
        let formatted = format_receipt_markdown(&ev);
        assert!(formatted.contains("[Review]"));
        assert!(formatted.contains("[owner/repo]"));
        assert!(formatted.contains("(https://github.com/owner/repo/pull/42)"));
    }

    #[test]
    fn review_without_pr_link_shows_plain_repo() {
        // No "pr" link → repo name is shown without markdown link syntax
        let ev = create_test_review("r3", "approved", false);
        let formatted = format_receipt_markdown(&ev);
        assert!(formatted.contains("owner/repo"));
        assert!(!formatted.contains("]("));
    }

    #[test]
    fn test_snapshot_events_with_all_manual_types() {
        let renderer = MarkdownRenderer::new();
        let events = vec![
            create_test_manual("1", ManualEventType::Note, "Take notes"),
            create_test_manual("2", ManualEventType::Incident, "Fix outage"),
            create_test_manual("3", ManualEventType::Design, "Design review"),
            create_test_manual("4", ManualEventType::Mentoring, "Mentor junior"),
            create_test_manual("5", ManualEventType::Launch, "Launch feature"),
            create_test_manual("6", ManualEventType::Migration, "Migrate data"),
            create_test_manual("7", ManualEventType::Review, "Code review"),
            create_test_manual("8", ManualEventType::Other, "Other work"),
        ];
        let workstreams = WorkstreamsFile {
            version: 1,
            generated_at: Utc::now(),
            workstreams: vec![Workstream {
                id: WorkstreamId::from_parts(["ws", "1"]),
                title: "Mixed Work".into(),
                summary: None,
                tags: vec![],
                receipts: vec![],
                events: events.iter().map(|e| e.id.clone()).collect(),
                stats: WorkstreamStats {
                    pull_requests: 0,
                    reviews: 0,
                    manual_events: 8,
                },
            }],
        };
        let coverage = CoverageManifest {
            run_id: RunId::now("test"),
            generated_at: Utc::now(),
            user: "test".into(),
            window: TimeWindow {
                since: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                until: NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
            },
            mode: "test".into(),
            sources: vec!["manual".into()],
            slices: vec![],
            warnings: vec![],
            completeness: Completeness::Complete,
        };

        let result = renderer
            .render_packet_markdown("test", "2024", &events, &workstreams, &coverage)
            .unwrap();

        // Should show manual events in summary
        assert!(result.contains("0 PRs, 0 reviews, 8 manual"));
        // Should have all emoji types
        assert!(result.contains("📝")); // Note
        assert!(result.contains("🚨")); // Incident
        assert!(result.contains("🏗️")); // Design
        assert!(result.contains("👨‍🏫")); // Mentoring
        assert!(result.contains("🚀")); // Launch
        assert!(result.contains("🔄")); // Migration
        assert!(result.contains("👀")); // Review
        assert!(result.contains("📌")); // Other
    }
}
