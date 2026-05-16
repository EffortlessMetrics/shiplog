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
    let manual_events_path = configured_manual_events_path(config_path);

    let good = build_completion_signals(result);
    let attention = build_attention_items(result, &coverage, &events, &validation_errors, &signals);
    let readiness = intake_readiness(&validation_errors, &events, &attention);
    let evidence_debt = build_evidence_debt(EvidenceDebtInput {
        out_dir,
        run_id: &run_id,
        manual_events_path: manual_events_path.as_deref(),
        coverage: &coverage,
        events: &events,
        skipped_sources: &skipped_sources,
        workstreams: &workstreams,
        validation_errors: &validation_errors,
        signals: &signals,
    });
    let top_fixups = map_top_fixups(review_fixups(
        &run_id,
        out_dir,
        manual_events_path.as_deref(),
        &skipped_sources,
        &validation_errors,
        &signals,
    ));
    let curation_notes = intake_curation_notes(result);
    let next_commands = build_next_commands(
        result,
        out_dir,
        config_path,
        manual_events_path.as_deref(),
        &run_id,
        &signals,
    );
    let artifacts = build_artifacts(result);
    let repair_sources = intake_repair_source_reports(explanations, &result.configured.failures);
    let journal_suggestions = build_journal_suggestions(&top_fixups);
    let share_commands = build_share_commands(out_dir, &run_id);
    let source_freshness = build_source_freshness_report(
        &result.configured.successes,
        &result.configured.failures,
        explanations,
    );
    let included_sources = build_included_sources(result);
    let skipped_sources_report = build_skipped_sources(result, explanations);
    let actions = intake_report_actions(
        &repair_sources,
        &top_fixups,
        &share_commands,
        &next_commands,
    );
    let repair_items = build_repair_items(RepairItemInputs {
        repair_sources: &repair_sources,
        source_freshness: &source_freshness,
        out_dir,
        config_path,
        needs_attention: &attention,
        evidence_debt: &evidence_debt,
        top_fixups: &top_fixups,
        journal_suggestions: &journal_suggestions,
        actions: &actions,
        next_commands: &next_commands,
        artifacts: &artifacts,
    });
    let packet_quality = build_packet_quality(PacketQualityInputs {
        readiness,
        events: &events,
        workstreams: &workstreams,
        included_sources: &included_sources,
        source_freshness: &source_freshness,
        repair_items: &repair_items,
        needs_attention: &attention,
        evidence_debt: &evidence_debt,
        artifacts: &artifacts,
        next_commands: &next_commands,
    });

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
        included_sources,
        skipped_sources: skipped_sources_report,
        source_decisions: intake_source_decision_reports(explanations, &result.configured.failures),
        source_freshness,
        repair_sources,
        repair_items,
        packet_quality,
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

fn build_evidence_debt(input: EvidenceDebtInput<'_>) -> Vec<IntakeReportEvidenceDebt> {
    detect_evidence_debt(input)
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
    manual_events_path: Option<&Path>,
    run_id: &str,
    signals: &WorkstreamQualitySignals<'_>,
) -> Vec<String> {
    intake_readiness_next_steps(
        run_id,
        out_dir,
        config_path,
        manual_events_path,
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

fn build_skipped_sources(
    result: &ConfiguredRunResult,
    explanations: &[IntakeSourceExplanation],
) -> Vec<IntakeReportSkippedSource> {
    let mut seen = BTreeSet::new();
    let mut skipped_sources = Vec::new();

    for explanation in explanations {
        if !matches!(explanation.decision, IntakeSourceDecision::Skipped) {
            continue;
        }
        let identity = intake_report_source_identity(&explanation.name);
        if seen.insert(identity.source_key.clone()) {
            skipped_sources.push(IntakeReportSkippedSource {
                source: identity.source,
                source_key: identity.source_key,
                source_label: identity.source_label,
                reason: explanation.reason.clone(),
            });
        }
    }

    for failure in &result.configured.failures {
        let identity = intake_report_source_identity(&failure.name);
        if seen.insert(identity.source_key.clone()) {
            skipped_sources.push(IntakeReportSkippedSource {
                source: identity.source,
                source_key: identity.source_key,
                source_label: identity.source_label,
                reason: failure.error.clone(),
            });
        }
    }

    skipped_sources
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

#[derive(Debug)]
struct RepairItemDraft {
    repair_key: String,
    source_key: Option<String>,
    source_label: Option<String>,
    kind: String,
    reason: String,
    action_kind: String,
    action_command: Option<String>,
    clears_when: String,
    receipt_refs: Vec<IntakeReportRepairReceiptRef>,
}

struct RepairItemInputs<'a> {
    repair_sources: &'a [IntakeReportRepairSource],
    source_freshness: &'a [IntakeReportSourceFreshness],
    out_dir: &'a Path,
    config_path: &'a Path,
    needs_attention: &'a [String],
    evidence_debt: &'a [IntakeReportEvidenceDebt],
    top_fixups: &'a [IntakeReportFixup],
    journal_suggestions: &'a [String],
    actions: &'a [IntakeReportAction],
    next_commands: &'a [String],
    artifacts: &'a [IntakeReportArtifact],
}

struct PacketQualityInputs<'a> {
    readiness: &'a str,
    events: &'a [EventEnvelope],
    workstreams: &'a WorkstreamsFile,
    included_sources: &'a [IntakeReportIncludedSource],
    source_freshness: &'a [IntakeReportSourceFreshness],
    repair_items: &'a [IntakeReportRepairItem],
    needs_attention: &'a [String],
    evidence_debt: &'a [IntakeReportEvidenceDebt],
    artifacts: &'a [IntakeReportArtifact],
    next_commands: &'a [String],
}

fn build_packet_quality(inputs: PacketQualityInputs<'_>) -> IntakeReportPacketQuality {
    let evidence_strength = build_evidence_strength(&inputs);
    let packet_readiness = build_packet_readiness(&inputs, &evidence_strength);
    let claim_candidates = build_claim_candidates(&inputs, &evidence_strength);

    IntakeReportPacketQuality {
        packet_readiness,
        evidence_strength,
        claim_candidates,
        share_posture: Vec::new(),
    }
}

fn build_packet_readiness(
    inputs: &PacketQualityInputs<'_>,
    evidence_strength: &[IntakeReportEvidenceStrength],
) -> IntakeReportPacketReadiness {
    let packet_strength_status = evidence_strength
        .iter()
        .find(|item| item.scope == "packet")
        .map(|item| item.status.as_str())
        .unwrap_or("needs_context");
    let (mut status, mut summary) = match inputs.readiness {
        "Ready for review" => ("ready", "Ready for review."),
        "Needs curation" => ("ready_with_caveats", "Ready with caveats."),
        "Needs evidence" => ("needs_evidence", "Needs evidence."),
        "Needs repair" => ("blocked", "Blocked until repair."),
        _ => ("needs_context", "Needs context."),
    };
    if status == "ready" && packet_strength_status != "strong" {
        status = "ready_with_caveats";
        summary = "Ready with caveats.";
    }
    let mut reasons = Vec::new();

    if inputs
        .needs_attention
        .iter()
        .any(|item| item.contains("No events collected"))
    {
        reasons.push(IntakeReportPacketReadinessReason {
            kind: "missing_evidence".to_string(),
            summary: "No evidence events were collected.".to_string(),
            receipt_refs: repair_aware_receipt_refs(
                "needs_attention",
                inputs.repair_items,
                "manual:manual_evidence_missing:no_events",
            ),
        });
    }
    if !inputs.repair_items.is_empty() {
        reasons.push(IntakeReportPacketReadinessReason {
            kind: "open_repair_items".to_string(),
            summary: format!("{} repair item(s) remain open.", inputs.repair_items.len()),
            receipt_refs: vec![quality_receipt_ref("repair_items")],
        });
    }
    if !inputs.evidence_debt.is_empty() {
        reasons.push(IntakeReportPacketReadinessReason {
            kind: "evidence_debt".to_string(),
            summary: format!(
                "{} evidence debt item(s) need review.",
                inputs.evidence_debt.len()
            ),
            receipt_refs: vec![quality_receipt_ref("evidence_debt")],
        });
    }
    if reasons.is_empty() {
        reasons.push(IntakeReportPacketReadinessReason {
            kind: "evidence_strength".to_string(),
            summary: format!("Packet evidence strength is {packet_strength_status}."),
            receipt_refs: vec![quality_receipt_ref("included_sources")],
        });
    }

    IntakeReportPacketReadiness {
        status: status.to_string(),
        summary: summary.to_string(),
        reasons,
        next_actions: inputs.next_commands.iter().take(3).cloned().collect(),
    }
}

fn build_evidence_strength(inputs: &PacketQualityInputs<'_>) -> Vec<IntakeReportEvidenceStrength> {
    let mut strengths = Vec::new();
    let total_events = inputs
        .included_sources
        .iter()
        .map(|source| source.event_count)
        .sum::<usize>();
    let manual_events = inputs
        .included_sources
        .iter()
        .filter(|source| source.source_key == "manual")
        .map(|source| source.event_count)
        .sum::<usize>();
    let source_backed_events = total_events.saturating_sub(manual_events);
    let source_gap_count = source_quality_gap_count(inputs.source_freshness);

    let (packet_status, packet_reason) = if total_events == 0 {
        (
            "needs_context",
            "No evidence events were collected; add manual evidence or configure a source."
                .to_string(),
        )
    } else if source_backed_events == 0 {
        if source_gap_count > 0 {
            (
                "manual_only",
                format!(
                    "Manual evidence is present, but {source_gap_count} source(s) are skipped, stale, cached, or unavailable."
                ),
            )
        } else {
            (
                "manual_only",
                "Evidence is present, but it only comes from manual evidence.".to_string(),
            )
        }
    } else if source_gap_count > 0
        || !inputs.repair_items.is_empty()
        || !inputs.evidence_debt.is_empty()
    {
        (
            "partial",
            format!(
                "Source-backed evidence is present, with {source_gap_count} source gap(s), {} open repair item(s), and {} evidence debt item(s).",
                inputs.repair_items.len(),
                inputs.evidence_debt.len()
            ),
        )
    } else {
        (
            "strong",
            "Source-backed receipts are present and no open repair item directly undermines the packet."
                .to_string(),
        )
    };
    strengths.push(IntakeReportEvidenceStrength {
        scope: "packet".to_string(),
        status: packet_status.to_string(),
        reason: packet_reason,
        receipt_refs: packet_evidence_receipts(inputs, total_events, source_gap_count),
    });

    for source in inputs.included_sources {
        let (status, reason) = if source.event_count == 0 {
            (
                "needs_context",
                format!(
                    "{} ran but collected no evidence events.",
                    source.source_label
                ),
            )
        } else if source.source_key == "manual" {
            (
                "manual_only",
                format!(
                    "{} contributed {}.",
                    source.source_label,
                    event_count_phrase(source.event_count)
                ),
            )
        } else {
            (
                "strong",
                format!(
                    "{} contributed source-backed {}.",
                    source.source_label,
                    event_count_phrase(source.event_count)
                ),
            )
        };
        strengths.push(IntakeReportEvidenceStrength {
            scope: format!("source:{}", source.source_key),
            status: status.to_string(),
            reason,
            receipt_refs: vec![quality_source_receipt_ref(
                "included_sources",
                &source.source_key,
            )],
        });
    }

    for freshness in inputs.source_freshness {
        let Some(status) = evidence_strength_for_freshness(&freshness.status) else {
            continue;
        };
        let reason = freshness.reason.clone().unwrap_or_else(|| {
            format!(
                "{} source freshness is {}.",
                freshness.source_label, freshness.status
            )
        });
        strengths.push(IntakeReportEvidenceStrength {
            scope: format!("source:{}", freshness.source_key),
            status: status.to_string(),
            reason,
            receipt_refs: vec![quality_source_receipt_ref(
                "source_freshness",
                &freshness.source_key,
            )],
        });
    }

    if !inputs.artifacts.is_empty() {
        strengths.push(IntakeReportEvidenceStrength {
            scope: "artifacts".to_string(),
            status: "strong".to_string(),
            reason: "Run artifacts are recorded for packet, ledger, coverage, and report review."
                .to_string(),
            receipt_refs: vec![quality_receipt_ref("artifacts")],
        });
    }

    strengths
}

fn build_claim_candidates(
    inputs: &PacketQualityInputs<'_>,
    evidence_strength: &[IntakeReportEvidenceStrength],
) -> Vec<IntakeReportClaimCandidate> {
    if inputs.events.is_empty() {
        return Vec::new();
    }

    let packet_strength = evidence_strength
        .iter()
        .find(|item| item.scope == "packet")
        .map(|item| item.status.as_str())
        .unwrap_or("needs_context");
    let events_by_id = inputs
        .events
        .iter()
        .map(|event| (event.id.0.as_str(), event))
        .collect::<HashMap<_, _>>();
    let mut candidates = Vec::new();
    let mut seen_ids = BTreeSet::new();

    for workstream in inputs.workstreams.workstreams.iter().take(5) {
        let supporting_events = workstream
            .events
            .iter()
            .filter_map(|id| events_by_id.get(id.0.as_str()).copied())
            .collect::<Vec<_>>();
        if supporting_events.is_empty() {
            continue;
        }

        let title = claim_candidate_title(workstream, &supporting_events);
        let claim_id = unique_claim_id(&title, &mut seen_ids);
        let supporting_sources = claim_supporting_sources(&supporting_events);
        let evidence_strength = claim_evidence_strength(&supporting_sources, packet_strength);
        let mut caveats = Vec::new();
        if evidence_strength == "manual_only" {
            caveats.push("Only manual evidence currently supports this candidate.".to_string());
        } else if evidence_strength == "partial" {
            caveats.push(format!("Packet evidence strength is {packet_strength}."));
        }

        candidates.push(IntakeReportClaimCandidate {
            claim_id,
            title,
            supporting_repair_keys: Vec::new(),
            supporting_sources,
            evidence_strength: evidence_strength.to_string(),
            supporting_receipt_refs: claim_receipt_refs(inputs, &supporting_events),
            missing_context_prompts: claim_missing_context_prompts(evidence_strength),
            safe_profiles: vec!["manager".to_string()],
            caveats,
        });
    }

    candidates
}

fn claim_candidate_title(workstream: &Workstream, events: &[&EventEnvelope]) -> String {
    if is_generic_workstream_title(&workstream.title)
        && let Some(title) = events.first().map(|event| event_claim_title(event))
    {
        return title;
    }
    workstream.title.clone()
}

fn is_generic_workstream_title(title: &str) -> bool {
    let normalized = title.trim().to_ascii_lowercase();
    normalized == "misc"
        || normalized == "manual/general"
        || normalized == "general"
        || normalized.ends_with("/general")
}

fn event_claim_title(event: &EventEnvelope) -> String {
    match &event.payload {
        EventPayload::PullRequest(pr) => pr.title.clone(),
        EventPayload::Review(review) => format!("Review support for {}", review.pull_title),
        EventPayload::Manual(manual) => manual.title.clone(),
    }
}

fn unique_claim_id(title: &str, seen_ids: &mut BTreeSet<String>) -> String {
    let base = {
        let token = action_token(title);
        if token.is_empty() {
            "claim_workstream".to_string()
        } else {
            format!("claim_{token}")
        }
    };
    if seen_ids.insert(base.clone()) {
        return base;
    }

    let mut suffix = 2_u64;
    loop {
        let candidate = format!("{base}_{suffix}");
        if seen_ids.insert(candidate.clone()) {
            return candidate;
        }
        suffix += 1;
    }
}

fn claim_supporting_sources(events: &[&EventEnvelope]) -> Vec<String> {
    let mut sources = events
        .iter()
        .map(|event| claim_source_key(event).to_string())
        .collect::<Vec<_>>();
    sources.sort();
    sources.dedup();
    sources
}

fn claim_source_key(event: &EventEnvelope) -> &'static str {
    match &event.source.system {
        shiplog::schema::event::SourceSystem::Github => "github",
        shiplog::schema::event::SourceSystem::JsonImport => "json",
        shiplog::schema::event::SourceSystem::LocalGit => "git",
        shiplog::schema::event::SourceSystem::Manual => "manual",
        shiplog::schema::event::SourceSystem::Unknown => "unknown",
        shiplog::schema::event::SourceSystem::Other(_) => "unknown",
        _ => "unknown",
    }
}

fn claim_evidence_strength(sources: &[String], packet_strength: &str) -> &'static str {
    if sources.is_empty() {
        "needs_context"
    } else if sources.iter().all(|source| source == "manual") {
        "manual_only"
    } else if packet_strength == "strong" {
        "strong"
    } else {
        "partial"
    }
}

fn claim_receipt_refs(
    inputs: &PacketQualityInputs<'_>,
    supporting_events: &[&EventEnvelope],
) -> Vec<IntakeReportQualityReceiptRef> {
    let mut refs = Vec::new();
    for source in claim_supporting_sources(supporting_events) {
        refs.push(quality_source_receipt_ref("included_sources", &source));
    }
    refs.push(quality_receipt_ref("artifacts"));
    if !inputs.evidence_debt.is_empty() {
        refs.push(quality_receipt_ref("evidence_debt"));
    }
    refs
}

fn claim_missing_context_prompts(evidence_strength: &str) -> Vec<String> {
    let mut prompts = vec![
        "What changed after this work?".to_string(),
        "Who benefited?".to_string(),
        "What evidence would strengthen this claim?".to_string(),
    ];
    if evidence_strength == "manual_only" {
        prompts.push("Which source-backed receipt could confirm this?".to_string());
    }
    prompts
}

fn source_quality_gap_count(source_freshness: &[IntakeReportSourceFreshness]) -> usize {
    source_freshness
        .iter()
        .filter(|freshness| evidence_strength_for_freshness(&freshness.status).is_some())
        .count()
}

fn evidence_strength_for_freshness(status: &str) -> Option<&'static str> {
    match status {
        "skipped" | "unavailable" => Some("source_skipped"),
        "stale" | "cached" => Some("partial"),
        _ => None,
    }
}

fn packet_evidence_receipts(
    inputs: &PacketQualityInputs<'_>,
    total_events: usize,
    source_gap_count: usize,
) -> Vec<IntakeReportQualityReceiptRef> {
    let mut refs = Vec::new();
    if total_events == 0 {
        refs.push(quality_receipt_ref("needs_attention"));
        refs.extend(repair_key_receipt_refs(
            inputs.repair_items,
            "manual:manual_evidence_missing:no_events",
        ));
    } else {
        refs.push(quality_receipt_ref("included_sources"));
    }
    if source_gap_count > 0 {
        refs.push(quality_receipt_ref("source_freshness"));
    }
    if !inputs.repair_items.is_empty() {
        refs.push(quality_receipt_ref("repair_items"));
    }
    if !inputs.evidence_debt.is_empty() {
        refs.push(quality_receipt_ref("evidence_debt"));
    }
    refs
}

fn repair_aware_receipt_refs(
    field: &str,
    repair_items: &[IntakeReportRepairItem],
    repair_key: &str,
) -> Vec<IntakeReportQualityReceiptRef> {
    let mut refs = vec![quality_receipt_ref(field)];
    refs.extend(repair_key_receipt_refs(repair_items, repair_key));
    refs
}

fn repair_key_receipt_refs(
    repair_items: &[IntakeReportRepairItem],
    repair_key: &str,
) -> Vec<IntakeReportQualityReceiptRef> {
    repair_items
        .iter()
        .filter(|item| item.repair_key == repair_key)
        .map(|item| quality_repair_receipt_ref(&item.repair_key))
        .collect()
}

fn quality_receipt_ref(field: &str) -> IntakeReportQualityReceiptRef {
    IntakeReportQualityReceiptRef {
        field: field.to_string(),
        source_key: None,
        repair_key: None,
    }
}

fn quality_source_receipt_ref(field: &str, source_key: &str) -> IntakeReportQualityReceiptRef {
    IntakeReportQualityReceiptRef {
        field: field.to_string(),
        source_key: Some(source_key.to_string()),
        repair_key: None,
    }
}

fn quality_repair_receipt_ref(repair_key: &str) -> IntakeReportQualityReceiptRef {
    IntakeReportQualityReceiptRef {
        field: "repair_items".to_string(),
        source_key: None,
        repair_key: Some(repair_key.to_string()),
    }
}

fn build_repair_items(inputs: RepairItemInputs<'_>) -> Vec<IntakeReportRepairItem> {
    let RepairItemInputs {
        repair_sources,
        source_freshness,
        out_dir,
        config_path,
        needs_attention,
        evidence_debt,
        top_fixups,
        journal_suggestions,
        actions,
        next_commands,
        artifacts,
    } = inputs;
    let mut drafts = Vec::new();
    let mut seen = BTreeSet::new();

    for attention in needs_attention {
        if !attention.contains("No events collected") {
            continue;
        }
        push_repair_item_draft(
            &mut drafts,
            &mut seen,
            RepairItemDraft {
                repair_key: "manual:manual_evidence_missing:no_events".to_string(),
                source_key: Some("manual".to_string()),
                source_label: Some("Manual".to_string()),
                kind: "manual_evidence_missing".to_string(),
                reason: attention.clone(),
                action_kind: "journal_add".to_string(),
                action_command: Some("shiplog journal add".to_string()),
                clears_when: "manual source contributes at least one evidence event".to_string(),
                receipt_refs: vec![IntakeReportRepairReceiptRef {
                    field: "needs_attention".to_string(),
                    source_key: Some("manual".to_string()),
                }],
            },
        );
    }

    for repair in repair_sources {
        let kind = if repair.kind == "cache_replay" {
            "source_cached_only"
        } else {
            "source_skipped_configuration"
        };
        let action_kind = if kind == "source_cached_only" {
            "rerun_intake"
        } else {
            "configure_source"
        };
        push_repair_item_draft(
            &mut drafts,
            &mut seen,
            RepairItemDraft {
                repair_key: format!("source:{}:{kind}", repair.source_key),
                source_key: Some(repair.source_key.clone()),
                source_label: Some(repair.source_label.clone()),
                kind: kind.to_string(),
                reason: format!("{} needs repair: {}", repair.source_label, repair.reason),
                action_kind: action_kind.to_string(),
                action_command: repair.commands.first().cloned(),
                clears_when: format!(
                    "{} source contributes evidence on a rerun",
                    repair.source_label
                ),
                receipt_refs: vec![IntakeReportRepairReceiptRef {
                    field: "repair_sources".to_string(),
                    source_key: Some(repair.source_key.clone()),
                }],
            },
        );
    }

    for freshness in source_freshness {
        let Some(kind) = source_freshness_repair_kind(&freshness.status) else {
            continue;
        };
        push_repair_item_draft(
            &mut drafts,
            &mut seen,
            RepairItemDraft {
                repair_key: format!("source:{}:{kind}", freshness.source_key),
                source_key: Some(freshness.source_key.clone()),
                source_label: Some(freshness.source_label.clone()),
                kind: kind.to_string(),
                reason: freshness.reason.clone().unwrap_or_else(|| {
                    format!(
                        "{} evidence is {}",
                        freshness.source_label, freshness.status
                    )
                }),
                action_kind: "rerun_intake".to_string(),
                action_command: intake_rerun_command(next_commands, config_path, out_dir),
                clears_when: format!(
                    "{} source contributes fresh evidence on a rerun",
                    freshness.source_label
                ),
                receipt_refs: vec![IntakeReportRepairReceiptRef {
                    field: "source_freshness".to_string(),
                    source_key: Some(freshness.source_key.clone()),
                }],
            },
        );
    }

    for fixup in top_fixups {
        if !journal_suggestions
            .iter()
            .any(|suggestion| suggestion == &fixup.command)
        {
            continue;
        }
        push_repair_item_draft(
            &mut drafts,
            &mut seen,
            RepairItemDraft {
                repair_key: format!("manual:manual_evidence_missing:{}", fixup.id),
                source_key: Some("manual".to_string()),
                source_label: Some("Manual".to_string()),
                kind: "manual_evidence_missing".to_string(),
                reason: fixup.title.clone(),
                action_kind: "journal_add".to_string(),
                action_command: Some(fixup.command.clone()),
                clears_when: "manual source contributes at least one evidence event".to_string(),
                receipt_refs: vec![
                    IntakeReportRepairReceiptRef {
                        field: "top_fixups".to_string(),
                        source_key: None,
                    },
                    IntakeReportRepairReceiptRef {
                        field: "journal_suggestions".to_string(),
                        source_key: Some("manual".to_string()),
                    },
                ],
            },
        );
    }

    for debt in evidence_debt {
        push_repair_item_draft(
            &mut drafts,
            &mut seen,
            RepairItemDraft {
                repair_key: format!("evidence_debt:{}", action_token(&debt.kind)),
                source_key: None,
                source_label: None,
                kind: "evidence_debt_open".to_string(),
                reason: debt.summary.clone(),
                action_kind: "no_safe_action".to_string(),
                action_command: None,
                clears_when: "the evidence debt item is absent from a later report".to_string(),
                receipt_refs: vec![IntakeReportRepairReceiptRef {
                    field: "evidence_debt".to_string(),
                    source_key: None,
                }],
            },
        );
    }

    let has_share_actions = actions
        .iter()
        .any(|action| action.kind.starts_with("share_"));
    if !drafts.is_empty() && has_share_actions {
        push_repair_item_draft(
            &mut drafts,
            &mut seen,
            RepairItemDraft {
                repair_key: "share:share_redaction_required".to_string(),
                source_key: None,
                source_label: None,
                kind: "share_redaction_required".to_string(),
                reason:
                    "Manager and public share commands require a redaction key before rendering."
                        .to_string(),
                action_kind: "no_safe_action".to_string(),
                action_command: None,
                clears_when: "manager or public share output is rendered with --redact-key or SHIPLOG_REDACT_KEY"
                    .to_string(),
                receipt_refs: vec![IntakeReportRepairReceiptRef {
                    field: "actions".to_string(),
                    source_key: None,
                }],
            },
        );
    }

    for artifact in artifacts {
        if artifact.label != "source failures" {
            continue;
        }
        push_repair_item_draft(
            &mut drafts,
            &mut seen,
            RepairItemDraft {
                repair_key: "artifact:source_failures".to_string(),
                source_key: None,
                source_label: None,
                kind: "artifact_missing_or_unopened".to_string(),
                reason: "Source failure receipts were written for inspection.".to_string(),
                action_kind: "open_artifact".to_string(),
                action_command: None,
                clears_when:
                    "the failing source contributes evidence or the source failure artifact is absent"
                        .to_string(),
                receipt_refs: vec![IntakeReportRepairReceiptRef {
                    field: "artifacts".to_string(),
                    source_key: None,
                }],
            },
        );
    }

    drafts
        .into_iter()
        .enumerate()
        .map(|(idx, draft)| {
            let repair_id = format!(
                "repair_{:03}_{}",
                idx + 1,
                repair_id_token(&draft.repair_key)
            );
            let action_command = if draft.action_kind == "journal_add" {
                Some(journal_add_repair_command(&repair_id, out_dir))
            } else {
                draft.action_command
            };
            IntakeReportRepairItem {
                repair_id,
                repair_key: draft.repair_key,
                source_key: draft.source_key,
                source_label: draft.source_label,
                kind: draft.kind,
                reason: draft.reason,
                action: IntakeReportRepairAction {
                    kind: draft.action_kind,
                    command: action_command,
                },
                clears_when: draft.clears_when,
                receipt_refs: draft.receipt_refs,
            }
        })
        .collect()
}

fn push_repair_item_draft(
    drafts: &mut Vec<RepairItemDraft>,
    seen: &mut BTreeSet<String>,
    draft: RepairItemDraft,
) {
    if seen.insert(draft.repair_key.clone()) {
        drafts.push(draft);
    }
}

fn source_freshness_repair_kind(status: &str) -> Option<&'static str> {
    match status {
        "stale" => Some("source_freshness_stale"),
        "cached" => Some("source_cached_only"),
        _ => None,
    }
}

fn intake_rerun_command(
    next_commands: &[String],
    config_path: &Path,
    out_dir: &Path,
) -> Option<String> {
    next_commands
        .iter()
        .find(|command| command.contains("shiplog intake "))
        .cloned()
        .or_else(|| {
            Some(format!(
                "shiplog intake --config {} --out {} --last-6-months --explain",
                quote_cli_value(&config_path.display().to_string()),
                quote_cli_value(&out_dir.display().to_string())
            ))
        })
}

fn repair_id_token(repair_key: &str) -> String {
    let token = action_token(repair_key);
    if token.len() > 48 {
        token[..48].trim_end_matches('_').to_string()
    } else {
        token
    }
}

fn journal_add_repair_command(repair_id: &str, out_dir: &Path) -> String {
    format!(
        "shiplog journal add --from-repair {repair_id} --out {} --latest",
        quote_cli_value(&out_dir.display().to_string())
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intake_rerun_command_preserves_report_context_in_fallback() {
        let command = intake_rerun_command(
            &[],
            Path::new("configs/shiplog.toml"),
            Path::new("out/review packets"),
        );
        assert!(
            command.is_some(),
            "fallback rerun command should be present"
        );
        let Some(command) = command else {
            return;
        };

        assert!(command.contains("shiplog intake"));
        assert!(command.contains("--config \"configs/shiplog.toml\""));
        assert!(command.contains("--out \"out/review packets\""));
        assert!(command.contains("--last-6-months --explain"));
    }

    #[test]
    fn intake_rerun_command_prefers_existing_intake_guidance() {
        let command = intake_rerun_command(
            &[
                "shiplog doctor --config shiplog.toml".to_string(),
                "shiplog intake --config custom.toml --out out/custom --last-6-months --explain"
                    .to_string(),
            ],
            Path::new("shiplog.toml"),
            Path::new("out"),
        );
        assert!(command.is_some(), "existing rerun command should be reused");
        let Some(command) = command else {
            return;
        };

        assert_eq!(
            command,
            "shiplog intake --config custom.toml --out out/custom --last-6-months --explain"
        );
    }
}
