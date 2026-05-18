#![allow(dead_code)]

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ReviewLoopStatus {
    pub(crate) overall_status: ReviewLoopOverallStatus,
    pub(crate) setup_summary: SetupStatusSummary,
    pub(crate) latest_run: Option<LatestRunSummary>,
    pub(crate) packet_readiness: PacketReadinessSummary,
    pub(crate) source_summary: SourceStatusSummary,
    pub(crate) repair_summary: RepairStatusSummary,
    pub(crate) diff_summary: DiffStatusSummary,
    pub(crate) share_summary: ShareStatusSummary,
    pub(crate) blocking_reasons: Vec<StatusBlockingReason>,
    pub(crate) next_actions: Vec<StatusNextAction>,
    pub(crate) receipt_refs: Vec<StatusReceiptRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReviewLoopStatusInputs {
    pub(crate) setup_summary: SetupStatusSummary,
    pub(crate) latest_run: Option<LatestRunSummary>,
    pub(crate) packet_readiness: PacketReadinessSummary,
    pub(crate) source_summary: SourceStatusSummary,
    pub(crate) repair_summary: RepairStatusSummary,
    pub(crate) diff_summary: DiffStatusSummary,
    pub(crate) share_summary: ShareStatusSummary,
    pub(crate) receipt_refs: Vec<StatusReceiptRef>,
}

impl Default for ReviewLoopStatusInputs {
    fn default() -> Self {
        Self {
            setup_summary: SetupStatusSummary::unknown(),
            latest_run: None,
            packet_readiness: PacketReadinessSummary::unknown(),
            source_summary: SourceStatusSummary::default(),
            repair_summary: RepairStatusSummary::default(),
            diff_summary: DiffStatusSummary::unknown(),
            share_summary: ShareStatusSummary::unknown(),
            receipt_refs: Vec::new(),
        }
    }
}

impl ReviewLoopStatus {
    pub(crate) fn from_inputs(mut inputs: ReviewLoopStatusInputs) -> Self {
        inputs.source_summary.normalize();
        inputs.repair_summary.normalize();
        inputs.share_summary.normalize();
        inputs.receipt_refs.sort();
        inputs.receipt_refs.dedup();

        let mut blocking_reasons = Vec::new();
        let mut next_actions = Vec::new();

        let overall_status = if inputs.setup_summary.blocks_review_loop() {
            blocking_reasons.push(StatusBlockingReason::from_setup(&inputs.setup_summary));
            if inputs.setup_summary.next_actions.is_empty() {
                next_actions.push(StatusNextAction::doctor_setup(
                    "setup must be inspected before evidence collection",
                ));
                next_actions.push(StatusNextAction::sources_status(
                    "source readiness should be inspected before repair",
                ));
            } else {
                next_actions.extend(inputs.setup_summary.next_actions.iter().cloned());
            }

            if inputs.setup_summary.status == SetupSummaryStatus::Blocked {
                ReviewLoopOverallStatus::Blocked
            } else {
                ReviewLoopOverallStatus::NeedsSetup
            }
        } else if inputs.setup_summary.status == SetupSummaryStatus::Unknown {
            next_actions.push(StatusNextAction::doctor_setup(
                "setup readiness receipt is missing",
            ));
            ReviewLoopOverallStatus::Unknown
        } else if inputs.latest_run.is_none() {
            next_actions.push(StatusNextAction::intake(
                "setup is usable and no latest run is available",
            ));
            ReviewLoopOverallStatus::ReadyToCollect
        } else if inputs.repair_summary.applied_not_rerun {
            blocking_reasons.push(StatusBlockingReason::repair_in_progress(
                "repair was applied, but intake has not been rerun",
                inputs.repair_summary.receipt_refs.clone(),
            ));
            next_actions.push(StatusNextAction::intake(
                "rerun intake after applying evidence repair",
            ));
            ReviewLoopOverallStatus::RepairInProgress
        } else if inputs.repair_summary.open_items > 0 {
            blocking_reasons.push(StatusBlockingReason::repair_needed(
                inputs.repair_summary.open_items,
                inputs.repair_summary.receipt_refs.clone(),
            ));
            next_actions.push(StatusNextAction::repair_plan(
                "inspect repair items before running write-producing repair",
            ));
            if inputs.repair_summary.safe_write_count > 0 {
                next_actions.push(StatusNextAction::journal_add_from_repair(
                    "local evidence repair is available",
                ));
            }
            ReviewLoopOverallStatus::NeedsRepair
        } else if inputs.packet_readiness.status == PacketReadinessStatus::NeedsEvidence {
            blocking_reasons.push(StatusBlockingReason::packet_readiness(
                "packet still needs evidence",
                inputs.packet_readiness.receipt_refs.clone(),
            ));
            next_actions.push(StatusNextAction::repair_plan(
                "inspect evidence gaps before sharing",
            ));
            ReviewLoopOverallStatus::NeedsEvidence
        } else if inputs.share_summary.has_blocked_profile() {
            blocking_reasons.extend(inputs.share_summary.blocking_reasons());
            next_actions.push(StatusNextAction::share_explain(
                "inspect share posture before rendering",
            ));
            ReviewLoopOverallStatus::ShareBlocked
        } else if inputs.share_summary.all_renderable() {
            next_actions.push(StatusNextAction::share_explain(
                "confirm share posture before rendering",
            ));
            ReviewLoopOverallStatus::ReadyToShare
        } else if inputs.packet_readiness.status == PacketReadinessStatus::ReadyWithCaveats
            || inputs.setup_summary.status == SetupSummaryStatus::ReadyWithCaveats
        {
            next_actions.push(StatusNextAction::share_explain(
                "review caveats before share verification",
            ));
            ReviewLoopOverallStatus::ReadyWithCaveats
        } else if inputs.packet_readiness.status == PacketReadinessStatus::Ready {
            next_actions.push(StatusNextAction::share_explain(
                "explain share posture before verification or rendering",
            ));
            ReviewLoopOverallStatus::ReadyToExplainShare
        } else {
            next_actions.push(StatusNextAction::doctor_setup(
                "status is missing enough receipts to choose a later step",
            ));
            ReviewLoopOverallStatus::Unknown
        };

        normalize_next_actions(&mut next_actions);
        blocking_reasons.sort();
        blocking_reasons.dedup();

        Self {
            overall_status,
            setup_summary: inputs.setup_summary,
            latest_run: inputs.latest_run,
            packet_readiness: inputs.packet_readiness,
            source_summary: inputs.source_summary,
            repair_summary: inputs.repair_summary,
            diff_summary: inputs.diff_summary,
            share_summary: inputs.share_summary,
            blocking_reasons,
            next_actions,
            receipt_refs: inputs.receipt_refs,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReviewLoopOverallStatus {
    Unknown,
    NeedsSetup,
    ReadyToCollect,
    NeedsEvidence,
    NeedsRepair,
    RepairInProgress,
    ReadyWithCaveats,
    ReadyToExplainShare,
    ShareBlocked,
    ReadyToShare,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct SetupStatusSummary {
    pub(crate) status: SetupSummaryStatus,
    pub(crate) reason: String,
    pub(crate) next_actions: Vec<StatusNextAction>,
    pub(crate) receipt_refs: Vec<StatusReceiptRef>,
}

impl SetupStatusSummary {
    pub(crate) fn ready(reason: impl Into<String>) -> Self {
        Self {
            status: SetupSummaryStatus::Ready,
            reason: reason.into(),
            next_actions: Vec::new(),
            receipt_refs: Vec::new(),
        }
    }

    pub(crate) fn ready_with_caveats(reason: impl Into<String>) -> Self {
        Self {
            status: SetupSummaryStatus::ReadyWithCaveats,
            reason: reason.into(),
            next_actions: Vec::new(),
            receipt_refs: Vec::new(),
        }
    }

    pub(crate) fn needs_setup(
        reason: impl Into<String>,
        next_actions: Vec<StatusNextAction>,
    ) -> Self {
        Self {
            status: SetupSummaryStatus::NeedsSetup,
            reason: reason.into(),
            next_actions,
            receipt_refs: Vec::new(),
        }
    }

    pub(crate) fn blocked(reason: impl Into<String>, next_actions: Vec<StatusNextAction>) -> Self {
        Self {
            status: SetupSummaryStatus::Blocked,
            reason: reason.into(),
            next_actions,
            receipt_refs: Vec::new(),
        }
    }

    pub(crate) fn unknown() -> Self {
        Self {
            status: SetupSummaryStatus::Unknown,
            reason: "setup readiness receipt is missing".to_string(),
            next_actions: Vec::new(),
            receipt_refs: Vec::new(),
        }
    }

    fn blocks_review_loop(&self) -> bool {
        matches!(
            self.status,
            SetupSummaryStatus::NeedsSetup | SetupSummaryStatus::Blocked
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SetupSummaryStatus {
    Ready,
    ReadyWithCaveats,
    NeedsSetup,
    Blocked,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct LatestRunSummary {
    pub(crate) run_id: String,
    pub(crate) report_path: String,
    pub(crate) receipt_refs: Vec<StatusReceiptRef>,
}

impl LatestRunSummary {
    pub(crate) fn new(run_id: impl Into<String>, report_path: impl Into<String>) -> Self {
        let report_path = report_path.into();
        Self {
            run_id: run_id.into(),
            receipt_refs: vec![StatusReceiptRef::path(
                "latest_run.report_path",
                "intake_report",
                report_path.clone(),
            )],
            report_path,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct PacketReadinessSummary {
    pub(crate) status: PacketReadinessStatus,
    pub(crate) reason: String,
    pub(crate) receipt_refs: Vec<StatusReceiptRef>,
}

impl PacketReadinessSummary {
    pub(crate) fn ready(reason: impl Into<String>) -> Self {
        Self {
            status: PacketReadinessStatus::Ready,
            reason: reason.into(),
            receipt_refs: vec![StatusReceiptRef::field(
                "packet_readiness.status",
                "intake_report",
            )],
        }
    }

    pub(crate) fn ready_with_caveats(reason: impl Into<String>) -> Self {
        Self {
            status: PacketReadinessStatus::ReadyWithCaveats,
            reason: reason.into(),
            receipt_refs: vec![StatusReceiptRef::field(
                "packet_readiness.status",
                "intake_report",
            )],
        }
    }

    pub(crate) fn needs_evidence(reason: impl Into<String>) -> Self {
        Self {
            status: PacketReadinessStatus::NeedsEvidence,
            reason: reason.into(),
            receipt_refs: vec![StatusReceiptRef::field(
                "packet_readiness.status",
                "intake_report",
            )],
        }
    }

    pub(crate) fn unknown() -> Self {
        Self {
            status: PacketReadinessStatus::Unknown,
            reason: "packet readiness receipt is missing".to_string(),
            receipt_refs: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PacketReadinessStatus {
    Ready,
    ReadyWithCaveats,
    NeedsEvidence,
    NeedsRepair,
    Unknown,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub(crate) struct SourceStatusSummary {
    pub(crate) included: Vec<SourceCountSummary>,
    pub(crate) unavailable: Vec<SourceIssueSummary>,
    pub(crate) disabled: Vec<SourceIssueSummary>,
    pub(crate) receipt_refs: Vec<StatusReceiptRef>,
}

impl SourceStatusSummary {
    fn normalize(&mut self) {
        self.included.sort();
        self.included.dedup();
        self.unavailable.sort();
        self.unavailable.dedup();
        self.disabled.sort();
        self.disabled.dedup();
        self.receipt_refs.sort();
        self.receipt_refs.dedup();
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub(crate) struct SourceCountSummary {
    pub(crate) source_key: String,
    pub(crate) source_label: String,
    pub(crate) event_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub(crate) struct SourceIssueSummary {
    pub(crate) source_key: String,
    pub(crate) source_label: String,
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub(crate) struct RepairStatusSummary {
    pub(crate) open_items: usize,
    pub(crate) safe_write_count: usize,
    pub(crate) setup_blocked_write_count: usize,
    pub(crate) applied_not_rerun: bool,
    pub(crate) receipt_refs: Vec<StatusReceiptRef>,
}

impl RepairStatusSummary {
    fn normalize(&mut self) {
        self.receipt_refs.sort();
        self.receipt_refs.dedup();
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct DiffStatusSummary {
    pub(crate) status: DiffSummaryStatus,
    pub(crate) reason: String,
    pub(crate) receipt_refs: Vec<StatusReceiptRef>,
}

impl DiffStatusSummary {
    pub(crate) fn unknown() -> Self {
        Self {
            status: DiffSummaryStatus::Unknown,
            reason: "diff receipt is missing".to_string(),
            receipt_refs: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DiffSummaryStatus {
    Available,
    NoPriorComparableRun,
    NotGenerated,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ShareStatusSummary {
    pub(crate) profiles: Vec<ShareProfileSummary>,
    pub(crate) receipt_refs: Vec<StatusReceiptRef>,
}

impl ShareStatusSummary {
    pub(crate) fn unknown() -> Self {
        Self {
            profiles: Vec::new(),
            receipt_refs: Vec::new(),
        }
    }

    fn normalize(&mut self) {
        self.profiles.sort();
        self.profiles.dedup();
        self.receipt_refs.sort();
        self.receipt_refs.dedup();
    }

    fn has_blocked_profile(&self) -> bool {
        self.profiles
            .iter()
            .any(|profile| profile.status == ShareProfileStatus::Blocked)
    }

    fn all_renderable(&self) -> bool {
        !self.profiles.is_empty()
            && self
                .profiles
                .iter()
                .all(|profile| profile.status == ShareProfileStatus::Ready)
    }

    fn blocking_reasons(&self) -> Vec<StatusBlockingReason> {
        self.profiles
            .iter()
            .filter(|profile| profile.status == ShareProfileStatus::Blocked)
            .map(|profile| StatusBlockingReason {
                key: format!("share_profile:{}", profile.profile_key),
                label: format!("{} share blocked", profile.profile_label),
                status: "blocked".to_string(),
                reason: profile.reason.clone(),
                scope: "share".to_string(),
                receipt_refs: profile.receipt_refs.clone(),
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub(crate) struct ShareProfileSummary {
    pub(crate) profile_key: String,
    pub(crate) profile_label: String,
    pub(crate) status: ShareProfileStatus,
    pub(crate) reason: String,
    pub(crate) receipt_refs: Vec<StatusReceiptRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ShareProfileStatus {
    Ready,
    ReadyWithCaveats,
    Blocked,
    NotGenerated,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub(crate) struct StatusBlockingReason {
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) status: String,
    pub(crate) reason: String,
    pub(crate) scope: String,
    pub(crate) receipt_refs: Vec<StatusReceiptRef>,
}

impl StatusBlockingReason {
    fn from_setup(setup: &SetupStatusSummary) -> Self {
        Self {
            key: "setup".to_string(),
            label: "Setup".to_string(),
            status: setup_status_key(setup.status).to_string(),
            reason: setup.reason.clone(),
            scope: "setup".to_string(),
            receipt_refs: setup.receipt_refs.clone(),
        }
    }

    fn repair_in_progress(reason: impl Into<String>, receipt_refs: Vec<StatusReceiptRef>) -> Self {
        Self {
            key: "repair_in_progress".to_string(),
            label: "Repair in progress".to_string(),
            status: "repair_in_progress".to_string(),
            reason: reason.into(),
            scope: "repair".to_string(),
            receipt_refs,
        }
    }

    fn repair_needed(open_items: usize, receipt_refs: Vec<StatusReceiptRef>) -> Self {
        Self {
            key: "repair_items_open".to_string(),
            label: "Repair items open".to_string(),
            status: "needs_repair".to_string(),
            reason: format!("{open_items} repair item(s) remain open"),
            scope: "repair".to_string(),
            receipt_refs,
        }
    }

    fn packet_readiness(reason: impl Into<String>, receipt_refs: Vec<StatusReceiptRef>) -> Self {
        Self {
            key: "packet_readiness".to_string(),
            label: "Packet readiness".to_string(),
            status: "needs_evidence".to_string(),
            reason: reason.into(),
            scope: "packet".to_string(),
            receipt_refs,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub(crate) struct StatusNextAction {
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) command: String,
    pub(crate) writes: bool,
    pub(crate) reason: String,
    pub(crate) preconditions: Vec<String>,
    pub(crate) priority: u8,
    pub(crate) receipt_refs: Vec<StatusReceiptRef>,
}

impl StatusNextAction {
    pub(crate) fn init_guided(reason: impl Into<String>) -> Self {
        Self::new(
            "init_guided",
            "Create guided setup",
            "shiplog init --guided",
            true,
            reason,
            1,
        )
    }

    pub(crate) fn doctor_setup(reason: impl Into<String>) -> Self {
        Self::new(
            "doctor_setup",
            "Inspect setup",
            "shiplog doctor --setup",
            false,
            reason,
            1,
        )
    }

    pub(crate) fn sources_status(reason: impl Into<String>) -> Self {
        Self::new(
            "sources_status",
            "Inspect source setup",
            "shiplog sources status",
            false,
            reason,
            2,
        )
    }

    pub(crate) fn intake(reason: impl Into<String>) -> Self {
        Self::new(
            "intake",
            "Collect evidence",
            "shiplog intake --last-6-months --explain",
            true,
            reason,
            1,
        )
    }

    pub(crate) fn repair_plan(reason: impl Into<String>) -> Self {
        Self::new(
            "repair_plan",
            "Inspect repair plan",
            "shiplog repair plan --latest",
            false,
            reason,
            1,
        )
    }

    pub(crate) fn journal_add_from_repair(reason: impl Into<String>) -> Self {
        Self::new(
            "journal_add_from_repair",
            "Add local evidence from repair",
            "shiplog journal add --from-repair <repair_id>",
            true,
            reason,
            2,
        )
        .with_preconditions(["setup ready", "repair item is local-journal safe"])
    }

    pub(crate) fn share_explain(reason: impl Into<String>) -> Self {
        Self::new(
            "share_explain_manager",
            "Explain manager share posture",
            "shiplog share explain manager --latest",
            false,
            reason,
            1,
        )
    }

    fn new(
        key: impl Into<String>,
        label: impl Into<String>,
        command: impl Into<String>,
        writes: bool,
        reason: impl Into<String>,
        priority: u8,
    ) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            command: command.into(),
            writes,
            reason: reason.into(),
            preconditions: Vec::new(),
            priority,
            receipt_refs: Vec::new(),
        }
    }

    fn with_preconditions<const N: usize>(mut self, preconditions: [&str; N]) -> Self {
        self.preconditions = preconditions
            .into_iter()
            .map(std::string::ToString::to_string)
            .collect();
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub(crate) struct StatusReceiptRef {
    pub(crate) field: String,
    pub(crate) kind: String,
    pub(crate) path: Option<String>,
    pub(crate) key: Option<String>,
}

impl StatusReceiptRef {
    pub(crate) fn field(field: impl Into<String>, kind: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            kind: kind.into(),
            path: None,
            key: None,
        }
    }

    pub(crate) fn path(
        field: impl Into<String>,
        kind: impl Into<String>,
        path: impl Into<String>,
    ) -> Self {
        Self {
            field: field.into(),
            kind: kind.into(),
            path: Some(path.into()),
            key: None,
        }
    }

    pub(crate) fn keyed(
        field: impl Into<String>,
        kind: impl Into<String>,
        key: impl Into<String>,
    ) -> Self {
        Self {
            field: field.into(),
            kind: kind.into(),
            path: None,
            key: Some(key.into()),
        }
    }
}

fn normalize_next_actions(next_actions: &mut Vec<StatusNextAction>) {
    for action in next_actions.iter_mut() {
        action.preconditions.sort();
        action.preconditions.dedup();
        action.receipt_refs.sort();
        action.receipt_refs.dedup();
    }
    next_actions.sort_by(|left, right| {
        left.priority
            .cmp(&right.priority)
            .then_with(|| left.key.cmp(&right.key))
            .then_with(|| left.command.cmp(&right.command))
    });
    next_actions.dedup();
}

fn setup_status_key(status: SetupSummaryStatus) -> &'static str {
    match status {
        SetupSummaryStatus::Ready => "ready",
        SetupSummaryStatus::ReadyWithCaveats => "ready_with_caveats",
        SetupSummaryStatus::NeedsSetup => "needs_setup",
        SetupSummaryStatus::Blocked => "blocked",
        SetupSummaryStatus::Unknown => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_config_status_can_represent_setup_writer() {
        let status = ReviewLoopStatus::from_inputs(ReviewLoopStatusInputs {
            setup_summary: SetupStatusSummary::needs_setup(
                "shiplog.toml is missing",
                vec![
                    StatusNextAction::sources_status("inspect source setup after init"),
                    StatusNextAction::init_guided("create local setup files"),
                    StatusNextAction::doctor_setup("inspect setup after init"),
                ],
            ),
            ..ReviewLoopStatusInputs::default()
        });

        assert_eq!(status.overall_status, ReviewLoopOverallStatus::NeedsSetup);
        assert_eq!(status.blocking_reasons[0].scope, "setup");
        assert_eq!(status.next_actions[0].key, "doctor_setup");
        assert!(!status.next_actions[0].writes);
        assert_eq!(status.next_actions[1].key, "init_guided");
        assert!(status.next_actions[1].writes);
        assert_eq!(status.next_actions[2].key, "sources_status");
    }

    #[test]
    fn missing_setup_receipt_is_unknown_not_ready_to_collect() {
        let status = ReviewLoopStatus::from_inputs(ReviewLoopStatusInputs::default());

        assert_eq!(status.overall_status, ReviewLoopOverallStatus::Unknown);
        assert_eq!(status.next_actions.len(), 1);
        assert_eq!(status.next_actions[0].key, "doctor_setup");
        assert!(!status.next_actions[0].writes);
    }

    #[test]
    fn ready_setup_without_run_routes_to_intake() {
        let status = ReviewLoopStatus::from_inputs(ReviewLoopStatusInputs {
            setup_summary: SetupStatusSummary::ready("local setup ready"),
            ..ReviewLoopStatusInputs::default()
        });

        assert_eq!(
            status.overall_status,
            ReviewLoopOverallStatus::ReadyToCollect
        );
        assert_eq!(status.next_actions.len(), 1);
        assert_eq!(status.next_actions[0].key, "intake");
        assert!(status.next_actions[0].writes);
    }

    #[test]
    fn setup_blocked_suppresses_evidence_repair_writes() {
        let status = ReviewLoopStatus::from_inputs(ReviewLoopStatusInputs {
            setup_summary: SetupStatusSummary::blocked(
                "manual_events.yaml is malformed",
                vec![StatusNextAction::doctor_setup(
                    "repair setup before evidence repair",
                )],
            ),
            latest_run: Some(LatestRunSummary::new(
                "run-1",
                "out/run-1/intake.report.json",
            )),
            repair_summary: RepairStatusSummary {
                open_items: 2,
                safe_write_count: 1,
                setup_blocked_write_count: 1,
                applied_not_rerun: false,
                receipt_refs: vec![StatusReceiptRef::field("repair_items", "intake_report")],
            },
            ..ReviewLoopStatusInputs::default()
        });

        assert_eq!(status.overall_status, ReviewLoopOverallStatus::Blocked);
        assert_eq!(status.next_actions.len(), 1);
        assert_eq!(status.next_actions[0].key, "doctor_setup");
        assert!(
            status
                .next_actions
                .iter()
                .all(|action| action.key != "journal_add_from_repair")
        );
    }

    #[test]
    fn report_present_with_repair_items_routes_read_first_then_safe_write() {
        let status = ReviewLoopStatus::from_inputs(ReviewLoopStatusInputs {
            setup_summary: SetupStatusSummary::ready("local setup ready"),
            latest_run: Some(LatestRunSummary::new(
                "run-1",
                "out/run-1/intake.report.json",
            )),
            packet_readiness: PacketReadinessSummary::needs_evidence("manual evidence is missing"),
            repair_summary: RepairStatusSummary {
                open_items: 2,
                safe_write_count: 1,
                setup_blocked_write_count: 0,
                applied_not_rerun: false,
                receipt_refs: vec![StatusReceiptRef::field("repair_items", "intake_report")],
            },
            ..ReviewLoopStatusInputs::default()
        });

        assert_eq!(status.overall_status, ReviewLoopOverallStatus::NeedsRepair);
        assert_eq!(status.blocking_reasons[0].key, "repair_items_open");
        assert_eq!(status.next_actions[0].key, "repair_plan");
        assert!(!status.next_actions[0].writes);
        assert_eq!(status.next_actions[1].key, "journal_add_from_repair");
        assert!(status.next_actions[1].writes);
    }

    #[test]
    fn repair_applied_but_not_rerun_prefers_intake() {
        let status = ReviewLoopStatus::from_inputs(ReviewLoopStatusInputs {
            setup_summary: SetupStatusSummary::ready("local setup ready"),
            latest_run: Some(LatestRunSummary::new(
                "run-1",
                "out/run-1/intake.report.json",
            )),
            repair_summary: RepairStatusSummary {
                applied_not_rerun: true,
                receipt_refs: vec![StatusReceiptRef::field("journal_repair", "manual_journal")],
                ..RepairStatusSummary::default()
            },
            ..ReviewLoopStatusInputs::default()
        });

        assert_eq!(
            status.overall_status,
            ReviewLoopOverallStatus::RepairInProgress
        );
        assert_eq!(status.next_actions[0].key, "intake");
        assert!(status.next_actions[0].writes);
    }

    #[test]
    fn share_blocked_status_never_offers_render() {
        let status = ReviewLoopStatus::from_inputs(ReviewLoopStatusInputs {
            setup_summary: SetupStatusSummary::ready("local setup ready"),
            latest_run: Some(LatestRunSummary::new(
                "run-1",
                "out/run-1/intake.report.json",
            )),
            packet_readiness: PacketReadinessSummary::ready("packet ready"),
            share_summary: ShareStatusSummary {
                profiles: vec![ShareProfileSummary {
                    profile_key: "manager".to_string(),
                    profile_label: "Manager".to_string(),
                    status: ShareProfileStatus::Blocked,
                    reason: "SHIPLOG_REDACT_KEY missing".to_string(),
                    receipt_refs: vec![StatusReceiptRef::keyed(
                        "share_profiles",
                        "share_readiness",
                        "manager",
                    )],
                }],
                receipt_refs: Vec::new(),
            },
            ..ReviewLoopStatusInputs::default()
        });

        assert_eq!(status.overall_status, ReviewLoopOverallStatus::ShareBlocked);
        assert_eq!(status.next_actions.len(), 1);
        assert_eq!(status.next_actions[0].key, "share_explain_manager");
        assert!(!status.next_actions[0].writes);
        assert!(
            status
                .next_actions
                .iter()
                .all(|action| !action.command.starts_with("shiplog share manager"))
        );
    }

    #[test]
    fn next_actions_are_deterministic() {
        let status = ReviewLoopStatus::from_inputs(ReviewLoopStatusInputs {
            setup_summary: SetupStatusSummary::needs_setup(
                "setup incomplete",
                vec![
                    StatusNextAction::sources_status("inspect sources"),
                    StatusNextAction::doctor_setup("inspect setup"),
                    StatusNextAction::init_guided("create setup"),
                    StatusNextAction::doctor_setup("inspect setup"),
                ],
            ),
            ..ReviewLoopStatusInputs::default()
        });

        let keys: Vec<&str> = status
            .next_actions
            .iter()
            .map(|action| action.key.as_str())
            .collect();

        assert_eq!(keys, ["doctor_setup", "init_guided", "sources_status"]);
    }
}
