//! Coverage section rendering for Markdown packets.
//!
//! The coverage section has several independent responsibilities (included
//! sources, skipped sources, known gaps, and detailed query metadata). Keeping
//! each responsibility in a focused helper makes the packet contract easier to
//! review without changing the emitted Markdown shape.

use shiplog::schema::coverage::{Completeness, CoverageManifest, CoverageSlice};
use shiplog::schema::event::EventEnvelope;

use super::source::{
    SkippedSource, display_source_label, display_source_list, event_source_present,
    included_source_summary, skipped_source_warning, skipped_source_warnings, source_event_count,
    source_present,
};

pub(crate) fn render_coverage(
    out: &mut String,
    coverage: &CoverageManifest,
    events: &[EventEnvelope],
) {
    out.push_str("## Coverage and Limits\n\n");

    let skipped_sources = skipped_source_warnings(&coverage.warnings);
    render_included_sources(out, coverage, events, &skipped_sources);
    render_skipped_sources(out, &skipped_sources);
    render_known_gaps(out, coverage, events);
    render_coverage_details(out, coverage);
}

fn render_included_sources(
    out: &mut String,
    coverage: &CoverageManifest,
    events: &[EventEnvelope],
    skipped_sources: &[SkippedSource<'_>],
) {
    out.push_str("Included:\n");
    let included_sources = included_source_summary(&coverage.sources, events, skipped_sources);
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

    render_query_slice_summary(out, &coverage.slices);
    out.push('\n');
}

fn render_query_slice_summary(out: &mut String, slices: &[CoverageSlice]) {
    if slices.is_empty() {
        out.push_str("- Fetched events: not reported by query slices\n");
        return;
    }

    let fetched: u64 = slices.iter().map(|slice| slice.fetched).sum();
    let total: u64 = slices.iter().map(|slice| slice.total_count).sum();
    let slice_label = if slices.len() == 1 { "slice" } else { "slices" };
    out.push_str(&format!(
        "- Query slices: {} {}, fetched {} of {} reported results\n",
        slices.len(),
        slice_label,
        fetched,
        total
    ));
}

fn render_skipped_sources(out: &mut String, skipped_sources: &[SkippedSource<'_>]) {
    out.push_str("Skipped:\n");
    if skipped_sources.is_empty() {
        out.push_str("- None recorded\n");
    } else {
        for skipped in skipped_sources {
            out.push_str(&format!(
                "- {}: {}\n",
                display_source_label(skipped.source),
                skipped.reason
            ));
        }
    }
    out.push('\n');
}

fn render_known_gaps(out: &mut String, coverage: &CoverageManifest, events: &[EventEnvelope]) {
    out.push_str("Known gaps:\n");
    let mut has_gap = render_completeness_gap(out, &coverage.completeness);
    has_gap |= render_warning_gaps(out, &coverage.warnings);
    has_gap |= render_manual_source_gap(out, coverage, events);
    has_gap |= render_slice_quality_gaps(out, &coverage.slices);

    if !has_gap {
        out.push_str("- None recorded\n");
    }
    out.push('\n');
}

fn render_completeness_gap(out: &mut String, completeness: &Completeness) -> bool {
    if matches!(completeness, Completeness::Complete) {
        return false;
    }

    out.push_str(&format!("- Overall completeness is {}\n", completeness));
    true
}

fn render_warning_gaps(out: &mut String, warnings: &[String]) -> bool {
    let mut has_gap = false;
    for warning in warnings {
        if skipped_source_warning(warning).is_none() {
            has_gap = true;
            out.push_str(&format!("- {}\n", warning));
        }
    }
    has_gap
}

fn render_manual_source_gap(
    out: &mut String,
    coverage: &CoverageManifest,
    events: &[EventEnvelope],
) -> bool {
    if !source_present(&coverage.sources, "manual") && !event_source_present(events, "manual") {
        return false;
    }

    out.push_str("- Manual events are user-provided\n");
    true
}

fn render_slice_quality_gaps(out: &mut String, slices: &[CoverageSlice]) -> bool {
    let mut has_gap = false;

    let incomplete_count = slices
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

    let capped_count = slices
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

    has_gap
}

fn render_coverage_details(out: &mut String, coverage: &CoverageManifest) {
    out.push_str("Details:\n");
    out.push_str(&format!(
        "- **Date window:** {} to {}\n",
        coverage.window.since, coverage.window.until
    ));
    out.push_str(&format!("- **Mode:** {}\n", coverage.mode));
    out.push_str(&format!(
        "- **Sources:** {}\n",
        display_source_list(&coverage.sources)
    ));
    out.push_str(&format!(
        "- **Completeness:** {:?}\n",
        coverage.completeness
    ));
    render_query_slice_details(out, &coverage.slices);
    out.push('\n');
}

fn render_query_slice_details(out: &mut String, slices: &[CoverageSlice]) {
    if slices.is_empty() {
        return;
    }

    out.push_str(&format!("- **Query slices:** {}\n", slices.len()));
    render_incomplete_slice_detail(out, slices);
    render_capped_slice_details(out, slices);
}

fn render_incomplete_slice_detail(out: &mut String, slices: &[CoverageSlice]) {
    let partial_count = slices
        .iter()
        .filter(|s| s.incomplete_results.unwrap_or(false))
        .count();
    if partial_count > 0 {
        out.push_str(&format!(
            "  - ⚠️ {} slices had incomplete results\n",
            partial_count
        ));
    }
}

fn render_capped_slice_details(out: &mut String, slices: &[CoverageSlice]) {
    let capped_slices: Vec<_> = slices
        .iter()
        .filter(|s| s.total_count > s.fetched)
        .collect();
    if capped_slices.is_empty() {
        return;
    }

    out.push_str("  - **Slicing applied (API caps):**\n");
    for slice in capped_slices.iter().take(3) {
        out.push_str(&format!(
            "    - {}: fetched {}/{} ({}%)\n",
            slice.query,
            slice.fetched,
            slice.total_count,
            fetched_percent(slice)
        ));
    }
    if capped_slices.len() > 3 {
        out.push_str(&format!("    - ... and {} more\n", capped_slices.len() - 3));
    }
}

fn fetched_percent(slice: &CoverageSlice) -> u64 {
    if slice.total_count > 0 {
        (slice.fetched as f64 / slice.total_count as f64 * 100.0) as u64
    } else {
        100
    }
}
