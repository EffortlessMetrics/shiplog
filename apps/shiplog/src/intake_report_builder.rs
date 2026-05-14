use super::*;

pub(crate) fn build_intake_report(
    result: &ConfiguredRunResult,
    out_dir: &Path,
    config_path: &Path,
    explanations: &[IntakeSourceExplanation],
) -> Result<IntakeReport> {
    let ingest = load_run_ingest(&result.outputs.out_dir)
        .with_context(|| format!("load intake run {}", result.outputs.out_dir.display()))?;
    let coverage = ingest.coverage;
    let events = ingest.events;
    let run_id = result.run_id.clone();
    let skipped_sources = configured_source_skips(&coverage.warnings);
    let (workstreams, _, _) = load_effective_workstreams_for_run(&result.outputs.out_dir)?;
    let validation_errors = validate_workstreams_against_events(&workstreams, &events);
    let signals = workstream_quality_signals(&workstreams, &events);

    let good = build_completion_signals(result);
    let attention = build_attention_items(result, &coverage, &events, &validation_errors, &signals);
    let readiness = intake_readiness(&validation_errors, &events, &attention);
    let evidence_debt = build_evidence_debt(
        &run_id,
        &coverage,
        &events,
        &skipped_sources,
        &workstreams,
        &validation_errors,
        &signals,
    );
    let top_fixups = map_top_fixups(review_fixups(
        &run_id,
        out_dir,
        &skipped_sources,
        &validation_errors,
        &signals,
    ));
    let curation_notes = intake_curation_notes(result);
    let next_commands = build_next_commands(result, out_dir, config_path, &run_id, &signals);
    let artifacts = build_artifacts(result);
    let repair_sources = intake_repair_source_reports(explanations, &result.configured.failures);
    let journal_suggestions = build_journal_suggestions(&top_fixups);
    let share_commands = build_share_commands(out_dir, &run_id);
    let actions = intake_report_actions(
        &repair_sources,
        &top_fixups,
        &share_commands,
        &next_commands,
    );
    let source_freshness = build_source_freshness_report(
        &result.configured.successes,
        &result.configured.failures,
        explanations,
    );

    Ok(IntakeReport {
        schema_version: 1,
        run_id: run_id.clone(),
        readiness: readiness.to_string(),
        config_path: config_path.display().to_string(),
        out_dir: out_dir.display().to_string(),
        run_dir: result.outputs.out_dir.display().to_string(),
        packet_path: result.outputs.packet_md.display().to_string(),
        period: result.window.period.clone(),
        window: IntakeReportWindow {
            since: result.window.since.to_string(),
            until: result.window.until.to_string(),
            label: result.window.window_label(),
        },
        reports: IntakeReportFiles {
            markdown: report_markdown_path(result).display().to_string(),
            json: report_json_path(result).display().to_string(),
        },
        included_sources: build_included_sources(result),
        skipped_sources: build_skipped_sources(result),
        source_decisions: intake_source_decision_reports(explanations),
        source_freshness,
        repair_sources,
        curation_notes,
        good,
        needs_attention: attention,
        evidence_debt,
        top_fixups,
        journal_suggestions,
        share_commands,
        next_commands,
        actions,
        artifacts,
    })
}

fn build_completion_signals(result: &ConfiguredRunResult) -> Vec<String> {
    let mut good = result
        .configured
        .successes
        .iter()
        .map(|(name, ingest)| {
            format!(
                "{} collected {}",
                display_source_label(name),
                event_count_phrase(ingest.events.len())
            )
        })
        .collect::<Vec<_>>();
    good.extend([
        "Packet rendered".to_string(),
        "Evidence ledger written".to_string(),
        "Coverage manifest written".to_string(),
        "Review inspection completed".to_string(),
    ]);
    good
}

fn build_attention_items(
    result: &ConfiguredRunResult,
    coverage: &CoverageManifest,
    events: &[EventEnvelope],
    validation_errors: &[String],
    signals: &WorkstreamQualitySignals<'_>,
) -> Vec<String> {
    let mut attention = source_failure_attention(result);
    attention.extend(coverage_attention(coverage));
    attention.extend(evidence_attention(events));
    attention.extend(workstream_attention(validation_errors, signals));
    attention
}

fn source_failure_attention(result: &ConfiguredRunResult) -> Vec<String> {
    result
        .configured
        .failures
        .iter()
        .map(|failure| {
            format!(
                "{} skipped: {}",
                display_source_label(&failure.name),
                failure.error
            )
        })
        .collect()
}

fn coverage_attention(coverage: &CoverageManifest) -> Vec<String> {
    let mut attention = Vec::new();
    if coverage.completeness != shiplog::schema::coverage::Completeness::Complete {
        attention.push(format!(
            "Coverage is {}; skipped or incomplete sources are recorded.",
            coverage.completeness
        ));
    }
    let gap_count = coverage_gap_count(coverage);
    if gap_count > 0 {
        attention.push(format!("{gap_count} coverage gap(s) should be reviewed."));
    }
    attention
}

fn evidence_attention(events: &[EventEnvelope]) -> Vec<String> {
    if events.is_empty() {
        vec!["No events collected; add manual evidence or enable a source.".to_string()]
    } else {
        Vec::new()
    }
}

fn workstream_attention(
    validation_errors: &[String],
    signals: &WorkstreamQualitySignals<'_>,
) -> Vec<String> {
    let mut attention = Vec::new();
    if !validation_errors.is_empty() {
        attention.push(format!(
            "{} workstream validation issue(s) need repair.",
            validation_errors.len()
        ));
    }
    if !signals.no_receipt_workstreams.is_empty() {
        attention.push(format!(
            "{} workstream(s) have no selected receipts.",
            signals.no_receipt_workstreams.len()
        ));
    }
    if !signals.broad_workstreams.is_empty() {
        attention.push(format!(
            "{} broad workstream(s) may need splitting.",
            signals.broad_workstreams.len()
        ));
    }
    if !signals.manual_context_workstreams.is_empty() {
        attention.push(format!(
            "{} broad workstream(s) need outcome context.",
            signals.manual_context_workstreams.len()
        ));
    }
    attention
}

fn intake_readiness(
    validation_errors: &[String],
    events: &[EventEnvelope],
    attention: &[String],
) -> &'static str {
    if !validation_errors.is_empty() {
        "Needs repair"
    } else if events.is_empty() {
        "Needs evidence"
    } else if attention.is_empty() {
        "Ready for review"
    } else {
        "Needs curation"
    }
}

fn build_evidence_debt(
    run_id: &str,
    coverage: &CoverageManifest,
    events: &[EventEnvelope],
    skipped_sources: &[ConfiguredSourceSkip],
    workstreams: &WorkstreamsFile,
    validation_errors: &[String],
    signals: &WorkstreamQualitySignals<'_>,
) -> Vec<IntakeReportEvidenceDebt> {
    detect_evidence_debt(EvidenceDebtInput {
        run_id,
        coverage,
        events,
        skipped_sources,
        workstreams,
        validation_errors,
        signals,
    })
    .iter()
    .map(|item| IntakeReportEvidenceDebt {
        severity: item.severity.label().to_string(),
        kind: item.kind.label().to_string(),
        summary: item.summary.clone(),
        detail: item.detail.clone(),
        next_step: item.next_step.clone(),
    })
    .collect()
}

fn map_top_fixups(fixups: Vec<ReviewFixup>) -> Vec<IntakeReportFixup> {
    fixups
        .iter()
        .take(5)
        .map(|fixup| IntakeReportFixup {
            id: fixup.id.clone(),
            kind: fixup.kind.label().to_string(),
            title: fixup.title.clone(),
            detail: fixup.detail.clone(),
            command: fixup.command.clone(),
        })
        .collect()
}

fn build_next_commands(
    result: &ConfiguredRunResult,
    out_dir: &Path,
    config_path: &Path,
    run_id: &str,
    signals: &WorkstreamQualitySignals<'_>,
) -> Vec<String> {
    intake_readiness_next_steps(
        run_id,
        out_dir,
        config_path,
        &result.configured.failures,
        signals
            .no_receipt_workstreams
            .first()
            .map(|workstream| workstream.title.as_str()),
        signals
            .broad_workstreams
            .first()
            .map(|workstream| workstream.title.as_str()),
        signals
            .manual_context_workstreams
            .first()
            .map(|workstream| workstream.title.as_str()),
    )
}

fn build_artifacts(result: &ConfiguredRunResult) -> Vec<IntakeReportArtifact> {
    let mut artifacts = vec![
        IntakeReportArtifact {
            label: "packet".to_string(),
            path: result.outputs.packet_md.display().to_string(),
        },
        IntakeReportArtifact {
            label: "ledger".to_string(),
            path: result.outputs.ledger_events_jsonl.display().to_string(),
        },
        IntakeReportArtifact {
            label: "coverage".to_string(),
            path: result.outputs.coverage_manifest_json.display().to_string(),
        },
        IntakeReportArtifact {
            label: format!("workstreams ({})", result.ws_source),
            path: result.outputs.workstreams_yaml.display().to_string(),
        },
        IntakeReportArtifact {
            label: "bundle manifest".to_string(),
            path: result.outputs.bundle_manifest_json.display().to_string(),
        },
        IntakeReportArtifact {
            label: "intake report markdown".to_string(),
            path: report_markdown_path(result).display().to_string(),
        },
        IntakeReportArtifact {
            label: "intake report json".to_string(),
            path: report_json_path(result).display().to_string(),
        },
    ];
    if let Some(zip_path) = &result.outputs.zip_path {
        artifacts.push(IntakeReportArtifact {
            label: "zip bundle".to_string(),
            path: zip_path.display().to_string(),
        });
    }
    let source_failures_path = result.outputs.out_dir.join(SOURCE_FAILURES_FILENAME);
    if source_failures_path.exists() {
        artifacts.push(IntakeReportArtifact {
            label: "source failures".to_string(),
            path: source_failures_path.display().to_string(),
        });
    }
    artifacts
}

fn report_markdown_path(result: &ConfiguredRunResult) -> PathBuf {
    result.outputs.out_dir.join("intake.report.md")
}

fn report_json_path(result: &ConfiguredRunResult) -> PathBuf {
    result.outputs.out_dir.join("intake.report.json")
}

fn build_included_sources(result: &ConfiguredRunResult) -> Vec<IntakeReportIncludedSource> {
    result
        .configured
        .successes
        .iter()
        .map(|(name, ingest)| {
            let identity = intake_report_source_identity(name);
            IntakeReportIncludedSource {
                source: identity.source,
                source_key: identity.source_key,
                source_label: identity.source_label.clone(),
                event_count: ingest.events.len(),
                summary: format!(
                    "{} collected {}",
                    identity.source_label,
                    event_count_phrase(ingest.events.len())
                ),
            }
        })
        .collect()
}

fn build_skipped_sources(result: &ConfiguredRunResult) -> Vec<IntakeReportSkippedSource> {
    result
        .configured
        .failures
        .iter()
        .map(|failure| {
            let identity = intake_report_source_identity(&failure.name);
            IntakeReportSkippedSource {
                source: identity.source,
                source_key: identity.source_key,
                source_label: identity.source_label,
                reason: failure.error.clone(),
            }
        })
        .collect()
}

fn build_journal_suggestions(top_fixups: &[IntakeReportFixup]) -> Vec<String> {
    top_fixups
        .iter()
        .map(|fixup| fixup.command.as_str())
        .filter(|command| command.starts_with("shiplog journal add "))
        .map(str::to_string)
        .collect()
}

fn build_share_commands(out_dir: &Path, run_id: &str) -> Vec<String> {
    let out_arg = quote_cli_value(&out_dir.display().to_string());
    vec![
        format!("shiplog share manager --out {out_arg} --run {run_id}"),
        format!("shiplog share public --out {out_arg} --run {run_id}"),
    ]
}
