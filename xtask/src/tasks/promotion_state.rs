//! `cargo xtask promotion-state`
//!
//! The single bounded current-promotion manifest is
//! `plans/shiplog-swarm/promotion-state.toml`. It records only the latest
//! completed promotion slice and the pending swarm range; historical
//! promotions stay in `plans/shiplog-swarm/implementation-plan.md` and Git
//! history.
//!
//! This task validates that manifest (failing closed on malformed state) and
//! generates the human-readable `plans/shiplog-swarm/current-promotion.md`
//! from it. `--check` verifies the manifest and that the checked-in generated
//! Markdown matches what the manifest would produce. The same invariant is
//! enforced inside the required `cargo test` gate by the
//! `checked_in_current_promotion_md_matches_manifest` test, so a second source
//! of truth cannot silently drift back in.

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

pub const MANIFEST_REL: &str = "plans/shiplog-swarm/promotion-state.toml";
pub const GENERATED_REL: &str = "plans/shiplog-swarm/current-promotion.md";
pub const PROMOTION_STATE_PATH: &str = MANIFEST_REL;

const GENERATED_BANNER: &str = "<!-- GENERATED FROM plans/shiplog-swarm/promotion-state.toml BY `cargo xtask promotion-state`. DO NOT EDIT BY HAND. -->";
const VALID_STATUSES: &[&str] = &["completed", "pending"];
const VALID_DISPOSITIONS: &[&str] = &["completed", "completed-with-governance", "pending"];

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromotionState {
    pub schema_version: u32,
    pub latest_promotion: LatestPromotion,
    #[serde(default)]
    pub pending: Pending,
    /// Source changes that landed outside a promotion during the cutover. These
    /// sit at the top level rather than under `latest_promotion` because their
    /// lifetime is their own: an entry is retired by `consumed_by`, not by the
    /// next promotion replacing the block.
    #[serde(default)]
    pub transition: Vec<Transition>,
    /// Bounded decisions that retain source content for explicitly
    /// source-authoritative paths that swarm changed. These are separate from
    /// source-side transition evidence because no source PR is being claimed
    /// as the historical cause of the swarm-only change.
    #[serde(default)]
    pub source_authority: Vec<SourceAuthorityDecision>,
}

/// One source PR that landed directly on source during the cutover.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transition {
    pub source_pr: String,
    pub source_merge_sha: String,
    /// Exact source and swarm commits whose trees this receipt describes.
    /// Empty values are accepted only for consumed legacy receipts; active
    /// receipts fail closed during structural validation.
    #[serde(default)]
    pub source_target: String,
    #[serde(default)]
    pub swarm_target: String,
    /// The promotion that reconciled this receipt. While empty the receipt is
    /// active; once set it is history and grants nothing.
    #[serde(default)]
    pub consumed_by: String,
    /// Merge commit for each swarm PR named by a path's chain, as
    /// `owner/repo#number = "<sha>"`.
    #[serde(default)]
    pub swarm_merge_sha: BTreeMap<String, String>,
    /// Per-path disposition. A source PR is rarely uniform, so one status over
    /// one path list would misdescribe part of it.
    #[serde(default)]
    pub path: Vec<TransitionPath>,
}

impl Transition {
    /// The promotion that consumed this receipt, or `None` while it is active.
    pub fn consumed_by(&self) -> Option<&str> {
        (!self.consumed_by.is_empty()).then_some(self.consumed_by.as_str())
    }

    /// Recorded merge commit for a swarm PR named in a chain.
    pub fn swarm_merge_sha(&self, receipt: &str) -> Option<&str> {
        self.swarm_merge_sha.get(receipt).map(String::as_str)
    }
}

/// A reviewed, consumptive decision to keep source content for one exact
/// source-authoritative path during one promotion.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceAuthorityDecision {
    pub path: String,
    /// Immutable evidence commits whose tree entries this decision binds. They
    /// must remain ancestors of the current promotion targets; the entries are
    /// revalidated at both the evidence and current target pairs.
    #[serde(default)]
    pub source_target: String,
    #[serde(default)]
    pub swarm_target: String,
    /// The merged swarm PR that records the human-reviewed decision.
    pub decision_receipt: String,
    /// Merge SHA of `decision_receipt`, reachable from the current swarm target.
    pub decision_merge_sha: String,
    /// Human-readable reason for retaining source content.
    pub reason: String,
    /// The promotion that consumed this decision. Consumed decisions are
    /// retained as history and grant no authority.
    #[serde(default)]
    pub consumed_by: String,
    /// Complete source-side tree entry at the evidence `source_target`; absence
    /// is valid.
    #[serde(default)]
    pub source_tree_entry: Option<TreeEntry>,
    /// Complete swarm-side tree entry at the evidence `swarm_target`; absence
    /// is valid.
    #[serde(default)]
    pub swarm_tree_entry: Option<TreeEntry>,
}

impl SourceAuthorityDecision {
    /// The promotion that consumed this decision, or `None` while active.
    pub fn consumed_by(&self) -> Option<&str> {
        (!self.consumed_by.is_empty()).then_some(self.consumed_by.as_str())
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransitionPath {
    pub path: String,
    pub disposition: TransitionDisposition,
    /// The deliberate outcome selected for an exceptional source-side
    /// divergence. Historical evidence and promotion resolution are kept
    /// separate so discarding a source change cannot be inferred from a
    /// generic evidence disposition.
    #[serde(default)]
    pub resolution: Option<TransitionResolution>,
    /// Exact issue or PR receipt that records the human-reviewed decision.
    #[serde(default)]
    pub decision_receipt: String,
    /// Merge SHA of the decision receipt, which must be reachable from the
    /// current swarm promotion target before it can grant authority.
    #[serde(default)]
    pub decision_merge_sha: String,
    /// Human-readable reason for the exceptional resolution.
    #[serde(default)]
    pub reason: String,
    /// The path is the manifest that stores this receipt itself. Its
    /// historical swarm tree entry remains exact evidence, but the current
    /// swarm entry must change when the receipt is materialized. This escape
    /// hatch is restricted to the canonical promotion manifest and an
    /// explicit discard-source resolution by structural validation below.
    #[serde(default)]
    pub self_referential: bool,
    /// Ordered swarm PRs that carried this path. `equivalent`,
    /// `dependency_equivalent`, and `tree_equivalent` each name exactly one;
    /// `superseded_in_swarm` names the steps that continue the source history.
    #[serde(default)]
    pub swarm_chain: Vec<String>,
    /// Complete source-side tree entry at the evidence `source_target`.
    /// `None` records that the path was absent at that evidence target.
    #[serde(default)]
    pub source_tree_entry: Option<TreeEntry>,
    /// Complete swarm-side tree entry at the evidence `swarm_target`.
    /// `None` records that the path was absent at that evidence target.
    #[serde(default)]
    pub swarm_tree_entry: Option<TreeEntry>,
}

/// The complete Git tree entry persisted by a transition receipt.
///
/// Mode and object type are part of the identity: a symlink, executable file,
/// gitlink, and regular file must not be conflated merely because they share
/// an object id. Absence is represented by the surrounding `Option`.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TreeEntry {
    pub mode: String,
    pub object_type: String,
    pub oid: String,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransitionDisposition {
    /// Both sides made the same change to this path.
    Equivalent,
    /// Both sides made the same Cargo.lock package-version transition, even
    /// when lockfile context makes the raw patch identities differ.
    DependencyEquivalent,
    /// Both sides arrived at the same resulting content for this path by
    /// different patches. Distinct from `equivalent`, which asserts the two
    /// changes are the same change; here only the outcome agrees.
    TreeEquivalent,
    /// The swarm side continued past what source landed.
    SupersededInSwarm,
    /// Source changed it and swarm has not caught up.
    MissingInSwarm,
    /// Both repositories had the same exact tree entry at the evidence target;
    /// a later swarm-only product change may now replace the source copy.
    ConvergedAtTarget,
    /// The two sides disagree; promotion must not proceed.
    Conflicting,
}

/// An explicit, bounded resolution for historical transition evidence.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransitionResolution {
    /// Discard the source-side change for this exact path and take the swarm
    /// tree entry during this bounded promotion.
    DiscardSource,
}

impl std::fmt::Display for TransitionDisposition {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Equivalent => "equivalent",
            Self::DependencyEquivalent => "dependency_equivalent",
            Self::TreeEquivalent => "tree_equivalent",
            Self::SupersededInSwarm => "superseded_in_swarm",
            Self::MissingInSwarm => "missing_in_swarm",
            Self::ConvergedAtTarget => "converged_at_target",
            Self::Conflicting => "conflicting",
        };
        formatter.write_str(name)
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LatestPromotion {
    pub status: String,
    #[serde(default)]
    pub disposition: Option<String>,
    pub source_promotion_pr: String,
    #[serde(default)]
    pub source_merge_sha: String,
    pub promoted_swarm_head: String,
    #[serde(default)]
    pub source_governance: Vec<String>,
    #[serde(default)]
    pub source_post_merge_proof: String,
    #[serde(default)]
    pub included_swarm_prs: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pending {
    #[serde(default)]
    pub swarm_pr_range: Vec<String>,
    #[serde(default)]
    pub deferred_receipt_carry: Vec<String>,
}

impl PromotionState {
    /// Every receipt this manifest records as belonging to the latest
    /// completed promotion slice (source promotion PR, governance receipts,
    /// and included swarm PRs).
    pub fn recorded_receipts(&self) -> Vec<String> {
        let mut receipts = vec![self.latest_promotion.source_promotion_pr.clone()];
        receipts.extend(self.latest_promotion.source_governance.iter().cloned());
        receipts.extend(self.latest_promotion.included_swarm_prs.iter().cloned());
        receipts
    }

    /// True when `receipt` is already carried by this manifest (either as part
    /// of the latest promotion slice or explicitly deferred).
    pub fn carries_receipt(&self, receipt: &str) -> bool {
        self.recorded_receipts()
            .iter()
            .any(|value| value == receipt)
            || self
                .pending
                .deferred_receipt_carry
                .iter()
                .any(|value| value == receipt)
    }

    /// True when `receipt` is explicitly deferred to a later substantive
    /// carry-forward rather than treated as stale.
    pub fn is_deferred(&self, receipt: &str) -> bool {
        self.pending
            .deferred_receipt_carry
            .iter()
            .any(|value| value == receipt)
    }
}

/// Load and validate the bounded promotion-state manifest.
pub fn load(workspace_root: &Path) -> Result<PromotionState> {
    let path = workspace_root.join(MANIFEST_REL);
    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let state: PromotionState =
        toml::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    validate(&state).with_context(|| format!("validate {}", path.display()))?;
    Ok(state)
}

/// Load the manifest if it is present, returning `None` when it does not exist.
///
/// A malformed manifest still fails closed; only a missing file is tolerated so
/// callers that inspect promotion state (e.g. `repo-contract-report`) degrade
/// gracefully on a checkout that predates the manifest.
pub fn load_optional(workspace_root: &Path) -> Result<Option<PromotionState>> {
    let path = workspace_root.join(MANIFEST_REL);
    if !path.exists() {
        return Ok(None);
    }
    load(workspace_root).map(Some)
}

fn validate(state: &PromotionState) -> Result<()> {
    if state.schema_version != 1 {
        bail!(
            "unsupported schema_version {}; expected 1",
            state.schema_version
        );
    }
    let promotion = &state.latest_promotion;
    if !VALID_STATUSES.contains(&promotion.status.as_str()) {
        bail!(
            "latest_promotion.status {:?} is not one of {VALID_STATUSES:?}",
            promotion.status
        );
    }
    if let Some(disposition) = promotion.disposition.as_deref()
        && !VALID_DISPOSITIONS.contains(&disposition)
    {
        bail!("latest_promotion.disposition {disposition:?} is not one of {VALID_DISPOSITIONS:?}");
    }
    validate_receipt(
        "latest_promotion.source_promotion_pr",
        &promotion.source_promotion_pr,
    )?;
    if !promotion.source_merge_sha.is_empty() {
        validate_sha(
            "latest_promotion.source_merge_sha",
            &promotion.source_merge_sha,
        )?;
    }
    validate_sha(
        "latest_promotion.promoted_swarm_head",
        &promotion.promoted_swarm_head,
    )?;
    for receipt in &promotion.source_governance {
        validate_receipt("latest_promotion.source_governance", receipt)?;
    }
    for receipt in &promotion.included_swarm_prs {
        validate_receipt("latest_promotion.included_swarm_prs", receipt)?;
    }
    for receipt in &state.pending.swarm_pr_range {
        validate_receipt("pending.swarm_pr_range", receipt)?;
    }
    for receipt in &state.pending.deferred_receipt_carry {
        validate_receipt("pending.deferred_receipt_carry", receipt)?;
    }
    validate_transitions(&state.transition)?;
    validate_source_authority(&state.source_authority)?;
    Ok(())
}

fn validate_source_authority(decisions: &[SourceAuthorityDecision]) -> Result<()> {
    let mut seen_paths = BTreeSet::new();
    for decision in decisions {
        if decision.path.trim().is_empty() {
            bail!("source_authority has an empty path");
        }
        if !seen_paths.insert(decision.path.as_str()) {
            bail!(
                "source_authority path {} appears more than once; use one bounded decision",
                decision.path
            );
        }
        validate_receipt(
            "source_authority.decision_receipt",
            &decision.decision_receipt,
        )?;
        validate_full_sha(
            "source_authority.decision_merge_sha",
            &decision.decision_merge_sha,
        )?;
        if decision.reason.trim().is_empty() {
            bail!(
                "source_authority path {} requires a human-readable reason",
                decision.path
            );
        }
        if !decision.consumed_by.is_empty() {
            validate_receipt("source_authority.consumed_by", &decision.consumed_by)?;
        } else {
            if decision.source_target.is_empty() || decision.swarm_target.is_empty() {
                bail!(
                    "active source_authority path {} must record exact source_target and swarm_target",
                    decision.path
                );
            }
            validate_full_sha("source_authority.source_target", &decision.source_target)?;
            validate_full_sha("source_authority.swarm_target", &decision.swarm_target)?;
        }
    }
    Ok(())
}

/// Structural checks on transition receipts. Evidence checks (that the recorded
/// merges exist, are reachable, and agree) need repository access and live in
/// [`super::transition`].
fn validate_transitions(transitions: &[Transition]) -> Result<()> {
    let mut active_source_merges = BTreeMap::new();
    let mut active_paths = BTreeMap::new();
    for entry in transitions {
        validate_receipt("transition.source_pr", &entry.source_pr)?;
        validate_full_sha("transition.source_merge_sha", &entry.source_merge_sha)?;
        if entry.consumed_by.is_empty() {
            if let Some(previous_merge) = active_source_merges.get(&entry.source_pr) {
                if previous_merge != &entry.source_merge_sha {
                    bail!(
                        "active transition source_pr {} has mismatched source merge SHAs: {} and {}",
                        entry.source_pr,
                        previous_merge,
                        entry.source_merge_sha
                    );
                }
            } else {
                active_source_merges
                    .insert(entry.source_pr.clone(), entry.source_merge_sha.clone());
            }
        }
        if !entry.consumed_by.is_empty() {
            validate_receipt("transition.consumed_by", &entry.consumed_by)?;
        }
        for (receipt, sha) in &entry.swarm_merge_sha {
            validate_receipt("transition.swarm_merge_sha key", receipt)?;
            validate_full_sha("transition.swarm_merge_sha", sha)?;
        }
        if entry.path.is_empty() {
            bail!(
                "transition {} records no paths; an entry that grants nothing should be removed",
                entry.source_pr
            );
        }
        // Disjoint paths within an entry: two dispositions for one path would
        // make the authority it grants ambiguous.
        let mut seen_paths = BTreeSet::new();
        for path in &entry.path {
            if path.path.trim().is_empty() {
                bail!("transition {} has an empty path", entry.source_pr);
            }
            if !seen_paths.insert(path.path.as_str()) {
                bail!(
                    "transition {} lists path {} more than once",
                    entry.source_pr,
                    path.path
                );
            }
            if entry.consumed_by.is_empty() {
                if let Some(previous_source_pr) = active_paths.get(&path.path) {
                    bail!(
                        "active transition path {} appears in both {} and {}",
                        path.path,
                        previous_source_pr,
                        entry.source_pr
                    );
                }
                active_paths.insert(path.path.clone(), entry.source_pr.clone());
            }
            if path.self_referential && path.path != PROMOTION_STATE_PATH {
                bail!(
                    "transition {} path {} may mark self_referential only for {}",
                    entry.source_pr,
                    path.path,
                    PROMOTION_STATE_PATH
                );
            }
            if path.disposition == TransitionDisposition::ConvergedAtTarget
                && path.resolution != Some(TransitionResolution::DiscardSource)
            {
                bail!(
                    "transition {} path {} converged_at_target requires discard_source resolution",
                    entry.source_pr,
                    path.path
                );
            }
            match path.resolution {
                None => {
                    if path.self_referential {
                        bail!(
                            "transition {} path {} self_referential requires discard_source resolution",
                            entry.source_pr,
                            path.path
                        );
                    }
                    if !path.decision_receipt.is_empty()
                        || !path.decision_merge_sha.is_empty()
                        || !path.reason.is_empty()
                    {
                        bail!(
                            "transition {} path {} has decision metadata without an explicit resolution",
                            entry.source_pr,
                            path.path
                        );
                    }
                }
                Some(TransitionResolution::DiscardSource) => {
                    if !matches!(
                        path.disposition,
                        TransitionDisposition::MissingInSwarm
                            | TransitionDisposition::ConvergedAtTarget
                    ) {
                        bail!(
                            "transition {} path {} discard_source requires missing_in_swarm or converged_at_target evidence",
                            entry.source_pr,
                            path.path
                        );
                    }
                    if path.decision_receipt.trim().is_empty() {
                        bail!(
                            "transition {} path {} discard_source requires decision_receipt",
                            entry.source_pr,
                            path.path
                        );
                    }
                    validate_receipt("transition.path.decision_receipt", &path.decision_receipt)?;
                    validate_full_sha(
                        "transition.path.decision_merge_sha",
                        &path.decision_merge_sha,
                    )?;
                    if path.reason.trim().is_empty() {
                        bail!(
                            "transition {} path {} discard_source requires reason",
                            entry.source_pr,
                            path.path
                        );
                    }
                    // `None` is a meaningful exact binding for an absent path.
                    // The repository-aware transition check below compares both
                    // options to the selected targets and rejects omitted or
                    // stale bindings before granting authority.
                }
            }
            match path.disposition {
                TransitionDisposition::MissingInSwarm | TransitionDisposition::Conflicting => {
                    if !path.swarm_chain.is_empty() {
                        bail!(
                            "transition {} path {} is {} and must not name swarm PRs",
                            entry.source_pr,
                            path.path,
                            path.disposition
                        );
                    }
                }
                TransitionDisposition::Equivalent => {
                    if path.swarm_chain.len() != 1 {
                        bail!(
                            "transition {} path {} is equivalent and must name exactly one swarm PR",
                            entry.source_pr,
                            path.path
                        );
                    }
                }
                TransitionDisposition::DependencyEquivalent => {
                    if path.swarm_chain.len() != 1 {
                        bail!(
                            "transition {} path {} is dependency_equivalent and must name exactly one swarm PR",
                            entry.source_pr,
                            path.path
                        );
                    }
                }
                TransitionDisposition::TreeEquivalent => {
                    if path.swarm_chain.len() != 1 {
                        bail!(
                            "transition {} path {} is tree_equivalent and must name exactly one swarm PR",
                            entry.source_pr,
                            path.path
                        );
                    }
                }
                TransitionDisposition::SupersededInSwarm => {
                    if path.swarm_chain.is_empty() {
                        bail!(
                            "transition {} path {} is superseded_in_swarm and must name its chain",
                            entry.source_pr,
                            path.path
                        );
                    }
                }
                TransitionDisposition::ConvergedAtTarget => {
                    if !path.swarm_chain.is_empty() {
                        bail!(
                            "transition {} path {} is converged_at_target and must not name swarm PRs",
                            entry.source_pr,
                            path.path
                        );
                    }
                }
            }
            for receipt in &path.swarm_chain {
                validate_receipt("transition.path.swarm_chain", receipt)?;
                if !entry.swarm_merge_sha.contains_key(receipt) {
                    bail!(
                        "transition {} names {receipt} in a chain without a recorded swarm_merge_sha",
                        entry.source_pr
                    );
                }
            }
        }
        if entry.consumed_by.is_empty() {
            if entry.source_target.is_empty() || entry.swarm_target.is_empty() {
                bail!(
                    "active transition {} must record exact source_target and swarm_target",
                    entry.source_pr
                );
            }
            validate_full_sha("transition.source_target", &entry.source_target)?;
            validate_full_sha("transition.swarm_target", &entry.swarm_target)?;
        }
    }
    Ok(())
}

/// A receipt is `owner/repo#number`, e.g. `EffortlessMetrics/shiplog#655`.
fn validate_receipt(field: &str, value: &str) -> Result<()> {
    let Some((repo, number)) = value.split_once('#') else {
        bail!("{field} receipt {value:?} must be `owner/repo#number`");
    };
    if !repo.contains('/') || repo.starts_with('/') || repo.ends_with('/') {
        bail!("{field} receipt {value:?} must have an `owner/repo` prefix");
    }
    if number.is_empty() || !number.chars().all(|c| c.is_ascii_digit()) {
        bail!("{field} receipt {value:?} must end with a numeric issue/PR id");
    }
    Ok(())
}

/// Transition receipts name merge commits that gate divergence authority, so
/// they must be unambiguous. An abbreviation could share a prefix with another
/// commit, and the evidence check compares in full.
fn validate_full_sha(field: &str, value: &str) -> Result<()> {
    if value.len() != 40 || !value.chars().all(|c| c.is_ascii_hexdigit()) {
        bail!("{field} {value:?} must be a full 40-character hex commit SHA");
    }
    Ok(())
}

fn validate_sha(field: &str, value: &str) -> Result<()> {
    if value.len() < 7 || value.len() > 40 || !value.chars().all(|c| c.is_ascii_hexdigit()) {
        bail!("{field} {value:?} must be a 7-40 character hex commit SHA");
    }
    Ok(())
}

/// Run the task: validate the manifest and generate (or, with `check`, verify)
/// `current-promotion.md`.
pub fn run(workspace_root: &Path, check: bool) -> Result<()> {
    let state = load(workspace_root)?;
    let expected = render_markdown(&state);
    let generated_path = workspace_root.join(GENERATED_REL);

    if check {
        let actual = fs::read_to_string(&generated_path)
            .with_context(|| format!("read {}", generated_path.display()))?;
        if actual != expected {
            bail!(
                "{} is out of sync with {}; run `cargo xtask promotion-state` to regenerate it",
                GENERATED_REL,
                MANIFEST_REL
            );
        }
        println!("promotion-state: manifest valid and {GENERATED_REL} in sync");
    } else {
        fs::write(&generated_path, &expected)
            .with_context(|| format!("write {}", generated_path.display()))?;
        println!("promotion-state: wrote {GENERATED_REL} from {MANIFEST_REL}");
    }

    Ok(())
}

fn render_markdown(state: &PromotionState) -> String {
    let promotion = &state.latest_promotion;
    let mut out = String::new();
    out.push_str(GENERATED_BANNER);
    out.push('\n');
    out.push_str("# Current shiplog-swarm Promotion\n\n");

    let status_line = match promotion.disposition.as_deref() {
        Some("completed-with-governance") => {
            "completed; approved source governance follows the promotion".to_string()
        }
        Some(other) => other.to_string(),
        None => promotion.status.clone(),
    };
    out.push_str(&format!("**Status:** {status_line}\n"));
    out.push_str(&format!(
        "**Promoted swarm head:** `{}`\n",
        promotion.promoted_swarm_head
    ));
    out.push_str(&format!(
        "**Source promotion:** `{}`\n",
        promotion.source_promotion_pr
    ));
    if !promotion.source_merge_sha.is_empty() {
        out.push_str(&format!(
            "**Source merge commit:** `{}`\n",
            promotion.source_merge_sha
        ));
    }
    for receipt in &promotion.source_governance {
        out.push_str(&format!("**Source governance:** `{receipt}`\n"));
    }
    if !promotion.source_post_merge_proof.is_empty() {
        out.push_str(&format!(
            "**Source post-merge proof:** `{}`\n",
            promotion.source_post_merge_proof
        ));
    }

    out.push_str("\n## Included work\n\n");
    if promotion.included_swarm_prs.is_empty() {
        out.push_str("- (none recorded)\n");
    } else {
        for receipt in &promotion.included_swarm_prs {
            out.push_str(&format!("- `{receipt}`\n"));
        }
    }

    out.push_str("\n## Pending swarm work\n\n");
    if state.pending.swarm_pr_range.is_empty() {
        out.push_str("- (none; source is current through the promoted swarm head)\n");
    } else {
        for receipt in &state.pending.swarm_pr_range {
            out.push_str(&format!("- `{receipt}`\n"));
        }
    }
    if !state.pending.deferred_receipt_carry.is_empty() {
        out.push_str("\n### Deferred receipt carry-forward\n\n");
        for receipt in &state.pending.deferred_receipt_carry {
            out.push_str(&format!("- `{receipt}`\n"));
        }
    }

    out.push_str("\n## Source-authority decisions\n\n");
    if state.source_authority.is_empty() {
        out.push_str("- (none recorded)\n");
    } else {
        for decision in &state.source_authority {
            out.push_str(&format!("- path: `{}`\n", decision.path));
            out.push_str(&format!("  - reason: {}\n", decision.reason));
            out.push_str(&format!(
                "  - source target: `{}`\n",
                decision.source_target
            ));
            out.push_str(&format!("  - swarm target: `{}`\n", decision.swarm_target));
            out.push_str(&format!(
                "  - decision receipt: `{}` at `{}`\n",
                decision.decision_receipt, decision.decision_merge_sha
            ));
            out.push_str(&format!(
                "  - consumed by: `{}`\n",
                if decision.consumed_by.is_empty() {
                    "(active)"
                } else {
                    decision.consumed_by.as_str()
                }
            ));
        }
    }

    out.push_str("\n## Truth hierarchy\n\n");
    out.push_str(
        "1. Git refs and ancestry\n\
         2. GitHub PR / check state\n\
         3. `plans/shiplog-swarm/promotion-state.toml` (this promotion's source of truth)\n\
         4. Generated reports (`target/source-of-truth/*`, this file)\n\
         5. Historical archive (`plans/shiplog-swarm/implementation-plan.md`)\n",
    );

    out.push_str("\n## Topology boundary\n\n");
    out.push_str(
        "- Product development remains authoritative in `EffortlessMetrics/shiplog-swarm`.\n\
         - Source promotion uses a regular merge commit; do not squash.\n\
         - Release authority, tags, publishing, signing, and release workflows remain in `EffortlessMetrics/shiplog`.\n",
    );

    out.push_str("\n## Next action\n\n");
    if state.pending.swarm_pr_range.is_empty() {
        out.push_str(
            "Source is current through the promoted swarm head. Continue with the next \
             substantive swarm PR; carry these receipts rather than opening a receipt-only PR.\n",
        );
    } else {
        out.push_str(
            "Prepare the next source promotion for the pending swarm range with \
             `cargo xtask promote --swarm-sha $(git rev-parse swarm/main)`. Carry these \
             receipts in the next substantive swarm PR; do not open a receipt-only refresh PR.\n",
        );
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn completed_manifest() -> PromotionState {
        let text = r#"
schema_version = 1
[latest_promotion]
status = "completed"
disposition = "completed-with-governance"
source_promotion_pr = "EffortlessMetrics/shiplog#655"
promoted_swarm_head = "c4fdba223d1c5c5b99a95b159ab8123d83d4b842"
source_governance = ["EffortlessMetrics/shiplog#656"]
included_swarm_prs = ["EffortlessMetrics/shiplog-swarm#238"]
[pending]
swarm_pr_range = ["EffortlessMetrics/shiplog-swarm#248"]
deferred_receipt_carry = []
"#;
        let state: PromotionState = toml::from_str(text).expect("parse");
        validate(&state).expect("valid");
        state
    }

    fn manifest_with_transition(transition: &str) -> Result<PromotionState> {
        let text = format!(
            r#"
schema_version = 1
[latest_promotion]
status = "completed"
disposition = "completed-with-governance"
source_promotion_pr = "EffortlessMetrics/shiplog#655"
promoted_swarm_head = "c4fdba223d1c5c5b99a95b159ab8123d83d4b842"
[pending]
{transition}
"#
        );
        let state: PromotionState = toml::from_str(&text)?;
        validate(&state)?;
        Ok(state)
    }

    /// A path both sides changed needs a named swarm counterpart. Accepting
    /// `equivalent` without one would grant reconciliation authority from a
    /// receipt that points at nothing.
    #[test]
    fn transition_equivalent_requires_exactly_one_swarm_pr() {
        let error = manifest_with_transition(
            r#"
[[transition]]
source_pr = "EffortlessMetrics/shiplog#666"
source_merge_sha = "d88d59a1a5af338537e35ff98b8ddda14d4673cf"
[[transition.path]]
path = "AGENTS.md"
disposition = "equivalent"
"#,
        )
        .expect_err("equivalent without a swarm PR must be rejected");
        assert!(
            format!("{error:#}").contains("must name exactly one swarm PR"),
            "unexpected error: {error:#}"
        );
    }

    /// `tree_equivalent` is its own recorded name, so an existing `equivalent`
    /// receipt keeps meaning exactly what it meant when it was written.
    #[test]
    fn transition_tree_equivalent_is_a_distinct_recorded_disposition() -> Result<()> {
        let state = manifest_with_transition(
            r#"
[[transition]]
source_pr = "EffortlessMetrics/shiplog#666"
source_merge_sha = "d88d59a1a5af338537e35ff98b8ddda14d4673cf"
source_target = "1111111111111111111111111111111111111111"
swarm_target = "2222222222222222222222222222222222222222"
swarm_merge_sha = { "EffortlessMetrics/shiplog-swarm#274" = "1ca35e97ba506062376e6f78b6633003e25db963" }
[[transition.path]]
path = "xtask/tests/cli.rs"
disposition = "tree_equivalent"
swarm_chain = ["EffortlessMetrics/shiplog-swarm#274"]
"#,
        )?;
        let path = &state.transition[0].path[0];
        assert_eq!(path.disposition, TransitionDisposition::TreeEquivalent);
        assert_eq!(path.disposition.to_string(), "tree_equivalent");
        assert_ne!(path.disposition, TransitionDisposition::Equivalent);

        let error = manifest_with_transition(
            r#"
[[transition]]
source_pr = "EffortlessMetrics/shiplog#666"
source_merge_sha = "d88d59a1a5af338537e35ff98b8ddda14d4673cf"
[[transition.path]]
path = "xtask/tests/cli.rs"
disposition = "tree_equivalent"
"#,
        )
        .expect_err("tree_equivalent without a swarm PR must be rejected");
        assert!(
            format!("{error:#}").contains("tree_equivalent and must name exactly one swarm PR"),
            "unexpected error: {error:#}"
        );
        Ok(())
    }

    #[test]
    fn discard_source_requires_bounded_decision_metadata_and_exact_tree_entries() -> Result<()> {
        let state = manifest_with_transition(
            r#"
[[transition]]
source_pr = "EffortlessMetrics/shiplog#674"
source_merge_sha = "b31d5f6d9700698b463d8f2b71b9d48a191f433c"
source_target = "1111111111111111111111111111111111111111"
swarm_target = "2222222222222222222222222222222222222222"
[[transition.path]]
path = "docs/xtask.md"
disposition = "missing_in_swarm"
resolution = "discard_source"
decision_receipt = "EffortlessMetrics/shiplog-swarm#242"
decision_merge_sha = "7d0a1b2c3d4e5f60718293a4b5c6d7e8f9a0b1c2"
reason = "The reviewed swarm control-plane copy supersedes the source transition copy for this bounded promotion."
source_tree_entry = { mode = "100644", object_type = "blob", oid = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" }
swarm_tree_entry = { mode = "100644", object_type = "blob", oid = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb" }
"#,
        )?;
        let path = &state.transition[0].path[0];
        assert_eq!(path.resolution, Some(TransitionResolution::DiscardSource));
        assert_eq!(path.decision_receipt, "EffortlessMetrics/shiplog-swarm#242");
        Ok(())
    }

    #[test]
    fn self_referential_transition_is_restricted_to_the_promotion_manifest() -> Result<()> {
        let state = manifest_with_transition(
            r#"
[[transition]]
source_pr = "EffortlessMetrics/shiplog#674"
source_merge_sha = "b31d5f6d9700698b463d8f2b71b9d48a191f433c"
source_target = "1111111111111111111111111111111111111111"
swarm_target = "2222222222222222222222222222222222222222"
[[transition.path]]
path = "plans/shiplog-swarm/promotion-state.toml"
disposition = "missing_in_swarm"
resolution = "discard_source"
self_referential = true
decision_receipt = "EffortlessMetrics/shiplog-swarm#242"
decision_merge_sha = "7d0a1b2c3d4e5f60718293a4b5c6d7e8f9a0b1c2"
reason = "The active decision is stored in the promotion manifest."
source_tree_entry = { mode = "100644", object_type = "blob", oid = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" }
swarm_tree_entry = { mode = "100644", object_type = "blob", oid = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb" }
"#,
        )?;
        assert!(state.transition[0].path[0].self_referential);

        let error = manifest_with_transition(
            r#"
[[transition]]
source_pr = "EffortlessMetrics/shiplog#674"
source_merge_sha = "b31d5f6d9700698b463d8f2b71b9d48a191f433c"
source_target = "1111111111111111111111111111111111111111"
swarm_target = "2222222222222222222222222222222222222222"
[[transition.path]]
path = "docs/xtask.md"
disposition = "missing_in_swarm"
resolution = "discard_source"
self_referential = true
decision_receipt = "EffortlessMetrics/shiplog-swarm#242"
decision_merge_sha = "7d0a1b2c3d4e5f60718293a4b5c6d7e8f9a0b1c2"
reason = "This path is not the promotion manifest."
source_tree_entry = { mode = "100644", object_type = "blob", oid = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" }
swarm_tree_entry = { mode = "100644", object_type = "blob", oid = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb" }
"#,
        )
        .expect_err("self_referential must be restricted to the promotion manifest");
        assert!(format!("{error:#}").contains("may mark self_referential only"));
        Ok(())
    }

    #[test]
    fn discard_source_rejects_missing_decision_receipt() {
        let error = manifest_with_transition(
            r#"
[[transition]]
source_pr = "EffortlessMetrics/shiplog#674"
source_merge_sha = "b31d5f6d9700698b463d8f2b71b9d48a191f433c"
source_target = "1111111111111111111111111111111111111111"
swarm_target = "2222222222222222222222222222222222222222"
[[transition.path]]
path = "docs/xtask.md"
disposition = "missing_in_swarm"
resolution = "discard_source"
reason = "A reason without a decision receipt is not bounded authority."
source_tree_entry = { mode = "100644", object_type = "blob", oid = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" }
swarm_tree_entry = { mode = "100644", object_type = "blob", oid = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb" }
"#,
        )
        .expect_err("discard_source without a decision receipt must fail closed");
        assert!(
            format!("{error:#}").contains("requires decision_receipt"),
            "unexpected error: {error:#}"
        );
    }

    /// Naming a chain step without recording its merge commit would leave the
    /// evidence check nothing to verify against.
    #[test]
    fn transition_chain_requires_a_recorded_merge_sha() {
        let error = manifest_with_transition(
            r#"
[[transition]]
source_pr = "EffortlessMetrics/shiplog#657"
source_merge_sha = "e6a72ad11ab22d03b1cf434b237bf0fff9c145bf"
[[transition.path]]
path = "Cargo.lock"
disposition = "superseded_in_swarm"
swarm_chain = ["EffortlessMetrics/shiplog-swarm#265"]
"#,
        )
        .expect_err("a chain step without a merge sha must be rejected");
        assert!(
            format!("{error:#}").contains("without a recorded swarm_merge_sha"),
            "unexpected error: {error:#}"
        );
    }

    /// An abbreviated SHA could share a prefix with another commit, which is not
    /// good enough to gate divergence authority.
    #[test]
    fn transition_requires_full_length_merge_shas() {
        let error = manifest_with_transition(
            r#"
[[transition]]
source_pr = "EffortlessMetrics/shiplog#666"
source_merge_sha = "d88d59a"
[[transition.path]]
path = "AGENTS.md"
disposition = "missing_in_swarm"
"#,
        )
        .expect_err("an abbreviated merge sha must be rejected");
        assert!(
            format!("{error:#}").contains("full 40-character hex commit SHA"),
            "unexpected error: {error:#}"
        );
    }

    #[test]
    fn active_transition_requires_exact_target_bindings() {
        let error = manifest_with_transition(
            r#"
[[transition]]
source_pr = "EffortlessMetrics/shiplog#666"
source_merge_sha = "d88d59a1a5af338537e35ff98b8ddda14d4673cf"
[[transition.path]]
path = "AGENTS.md"
disposition = "missing_in_swarm"
"#,
        )
        .expect_err("an active receipt without exact targets must fail closed");
        assert!(
            format!("{error:#}").contains("must record exact source_target and swarm_target"),
            "unexpected error: {error:#}"
        );
    }

    #[test]
    fn consumed_legacy_transition_remains_historical() -> Result<()> {
        let state = manifest_with_transition(
            r#"
[[transition]]
source_pr = "EffortlessMetrics/shiplog#666"
source_merge_sha = "d88d59a1a5af338537e35ff98b8ddda14d4673cf"
consumed_by = "EffortlessMetrics/shiplog#655"
[[transition.path]]
path = "AGENTS.md"
disposition = "missing_in_swarm"
"#,
        )?;
        assert_eq!(
            state.transition[0].consumed_by(),
            Some("EffortlessMetrics/shiplog#655")
        );
        assert!(state.transition[0].source_target.is_empty());
        Ok(())
    }

    /// Two dispositions for one path would make the authority it grants
    /// ambiguous.
    #[test]
    fn transition_rejects_a_duplicated_path() {
        let error = manifest_with_transition(
            r#"
[[transition]]
source_pr = "EffortlessMetrics/shiplog#666"
source_merge_sha = "d88d59a1a5af338537e35ff98b8ddda14d4673cf"
[[transition.path]]
path = "AGENTS.md"
disposition = "missing_in_swarm"
[[transition.path]]
path = "AGENTS.md"
disposition = "conflicting"
"#,
        )
        .expect_err("a duplicated path must be rejected");
        assert!(
            format!("{error:#}").contains("more than once"),
            "unexpected error: {error:#}"
        );
    }

    /// A later exact-target binding may revisit one source PR, but only for a
    /// disjoint active path set. Consumed historical entries are inert and may
    /// overlap the later binding.
    #[test]
    fn transition_accepts_disjoint_active_rebind_for_same_source_pr() -> Result<()> {
        let state = manifest_with_transition(
            r#"
[[transition]]
source_pr = "EffortlessMetrics/shiplog#666"
source_merge_sha = "d88d59a1a5af338537e35ff98b8ddda14d4673cf"
consumed_by = "EffortlessMetrics/shiplog#682"
[[transition.path]]
path = "docs/xtask.md"
disposition = "missing_in_swarm"
[[transition]]
source_pr = "EffortlessMetrics/shiplog#666"
source_merge_sha = "d88d59a1a5af338537e35ff98b8ddda14d4673cf"
source_target = "1111111111111111111111111111111111111111"
swarm_target = "2222222222222222222222222222222222222222"
[[transition.path]]
path = "plans/shiplog-swarm/promotion-runbook.md"
disposition = "missing_in_swarm"
"#,
        )?;
        assert_eq!(state.transition.len(), 2);
        Ok(())
    }

    #[test]
    fn transition_rejects_overlapping_active_rebind_path() {
        let error = manifest_with_transition(
            r#"
[[transition]]
source_pr = "EffortlessMetrics/shiplog#666"
source_merge_sha = "d88d59a1a5af338537e35ff98b8ddda14d4673cf"
source_target = "1111111111111111111111111111111111111111"
swarm_target = "2222222222222222222222222222222222222222"
[[transition.path]]
path = "docs/xtask.md"
disposition = "missing_in_swarm"
[[transition]]
source_pr = "EffortlessMetrics/shiplog#666"
source_merge_sha = "d88d59a1a5af338537e35ff98b8ddda14d4673cf"
source_target = "3333333333333333333333333333333333333333"
swarm_target = "4444444444444444444444444444444444444444"
[[transition.path]]
path = "docs/xtask.md"
disposition = "missing_in_swarm"
"#,
        )
        .expect_err("an active path must not have two bindings");
        assert!(
            format!("{error:#}").contains("appears in both"),
            "unexpected error: {error:#}"
        );
    }

    #[test]
    fn transition_rejects_overlapping_active_path_across_source_prs() {
        let error = manifest_with_transition(
            r#"
[[transition]]
source_pr = "EffortlessMetrics/shiplog#666"
source_merge_sha = "d88d59a1a5af338537e35ff98b8ddda14d4673cf"
source_target = "1111111111111111111111111111111111111111"
swarm_target = "2222222222222222222222222222222222222222"
[[transition.path]]
path = "docs/xtask.md"
disposition = "missing_in_swarm"
[[transition]]
source_pr = "EffortlessMetrics/shiplog#667"
source_merge_sha = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
source_target = "3333333333333333333333333333333333333333"
swarm_target = "4444444444444444444444444444444444444444"
[[transition.path]]
path = "docs/xtask.md"
disposition = "missing_in_swarm"
"#,
        )
        .expect_err("an active path must not have two source-PR bindings");
        assert!(
            format!("{error:#}").contains("appears in both"),
            "unexpected error: {error:#}"
        );
    }

    #[test]
    fn transition_rejects_mismatched_source_merge_on_active_rebind() {
        let error = manifest_with_transition(
            r#"
[[transition]]
source_pr = "EffortlessMetrics/shiplog#666"
source_merge_sha = "d88d59a1a5af338537e35ff98b8ddda14d4673cf"
source_target = "1111111111111111111111111111111111111111"
swarm_target = "2222222222222222222222222222222222222222"
[[transition.path]]
path = "docs/xtask.md"
disposition = "missing_in_swarm"
[[transition]]
source_pr = "EffortlessMetrics/shiplog#666"
source_merge_sha = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
source_target = "3333333333333333333333333333333333333333"
swarm_target = "4444444444444444444444444444444444444444"
[[transition.path]]
path = "plans/shiplog-swarm/promotion-runbook.md"
disposition = "missing_in_swarm"
"#,
        )
        .expect_err("an active rebind must retain the source merge identity");
        assert!(
            format!("{error:#}").contains("mismatched source merge SHAs"),
            "unexpected error: {error:#}"
        );
    }

    /// A per-path manifest is the point: one source PR carries different
    /// dispositions for different paths.
    #[test]
    fn transition_accepts_mixed_dispositions_across_paths() -> Result<()> {
        let state = manifest_with_transition(
            r#"
[[transition]]
source_pr = "EffortlessMetrics/shiplog#666"
source_merge_sha = "d88d59a1a5af338537e35ff98b8ddda14d4673cf"
source_target = "1111111111111111111111111111111111111111"
swarm_target = "2222222222222222222222222222222222222222"
swarm_merge_sha = { "EffortlessMetrics/shiplog-swarm#274" = "1ca35e97ba506062376e6f78b6633003e25db963" }
[[transition.path]]
path = "xtask/src/tasks/check_goals.rs"
disposition = "equivalent"
swarm_chain = ["EffortlessMetrics/shiplog-swarm#274"]
[[transition.path]]
path = ".codex/goals/active.toml"
disposition = "missing_in_swarm"
"#,
        )?;
        let entry = &state.transition[0];
        assert_eq!(entry.path.len(), 2);
        assert_eq!(entry.consumed_by(), None);
        assert_eq!(
            entry.swarm_merge_sha("EffortlessMetrics/shiplog-swarm#274"),
            Some("1ca35e97ba506062376e6f78b6633003e25db963")
        );
        Ok(())
    }

    #[test]
    fn accepts_a_first_completed_promotion() {
        let state = completed_manifest();
        assert_eq!(state.schema_version, 1);
        assert_eq!(state.latest_promotion.status, "completed");
    }

    #[test]
    fn accepts_no_pending_work() {
        let text = r#"
schema_version = 1
[latest_promotion]
status = "completed"
source_promotion_pr = "EffortlessMetrics/shiplog#655"
promoted_swarm_head = "c4fdba22"
"#;
        let state: PromotionState = toml::from_str(text).expect("parse");
        validate(&state).expect("valid");
        assert!(state.pending.swarm_pr_range.is_empty());
    }

    #[test]
    fn accepts_pending_swarm_work() {
        let state = completed_manifest();
        assert_eq!(
            state.pending.swarm_pr_range,
            vec!["EffortlessMetrics/shiplog-swarm#248".to_string()]
        );
    }

    #[test]
    fn accepts_approved_source_governance_after_promotion() {
        let state = completed_manifest();
        assert_eq!(
            state.latest_promotion.source_governance,
            vec!["EffortlessMetrics/shiplog#656".to_string()]
        );
        assert_eq!(
            state.latest_promotion.disposition.as_deref(),
            Some("completed-with-governance")
        );
    }

    #[test]
    fn rejects_wrong_schema_version() {
        let text = r#"
schema_version = 2
[latest_promotion]
status = "completed"
source_promotion_pr = "EffortlessMetrics/shiplog#655"
promoted_swarm_head = "c4fdba22"
"#;
        let state: PromotionState = toml::from_str(text).expect("parse");
        let error = validate(&state).expect_err("should reject");
        assert!(error.to_string().contains("schema_version"), "{error}");
    }

    #[test]
    fn rejects_unknown_status() {
        let text = r#"
schema_version = 1
[latest_promotion]
status = "bogus"
source_promotion_pr = "EffortlessMetrics/shiplog#655"
promoted_swarm_head = "c4fdba22"
"#;
        let state: PromotionState = toml::from_str(text).expect("parse");
        assert!(validate(&state).is_err());
    }

    #[test]
    fn rejects_malformed_receipt() {
        let text = r#"
schema_version = 1
[latest_promotion]
status = "completed"
source_promotion_pr = "not-a-receipt"
promoted_swarm_head = "c4fdba22"
"#;
        let state: PromotionState = toml::from_str(text).expect("parse");
        let error = validate(&state).expect_err("should reject");
        assert!(error.to_string().contains("receipt"), "{error}");
    }

    #[test]
    fn rejects_non_hex_swarm_head() {
        let text = r#"
schema_version = 1
[latest_promotion]
status = "completed"
source_promotion_pr = "EffortlessMetrics/shiplog#655"
promoted_swarm_head = "zzz"
"#;
        let state: PromotionState = toml::from_str(text).expect("parse");
        assert!(validate(&state).is_err());
    }

    #[test]
    fn rejects_unknown_fields() {
        let text = r#"
schema_version = 1
surprise = true
[latest_promotion]
status = "completed"
source_promotion_pr = "EffortlessMetrics/shiplog#655"
promoted_swarm_head = "c4fdba22"
"#;
        assert!(toml::from_str::<PromotionState>(text).is_err());
    }

    #[test]
    fn accepts_a_bounded_source_authority_decision() -> Result<()> {
        let text = r#"
schema_version = 1
[latest_promotion]
status = "completed"
source_promotion_pr = "EffortlessMetrics/shiplog#655"
promoted_swarm_head = "c4fdba223d1c5c5b99a95b159ab8123d83d4b842"

[[source_authority]]
path = ".github/workflows/release.yml"
source_target = "1111111111111111111111111111111111111111"
swarm_target = "2222222222222222222222222222222222222222"
decision_receipt = "EffortlessMetrics/shiplog-swarm#319"
decision_merge_sha = "7d0a1b2c3d4e5f60718293a4b5c6d7e8f9a0b1c2"
reason = "The canonical source repository retains release-writer authority."
source_tree_entry = { mode = "100644", object_type = "blob", oid = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" }
swarm_tree_entry = { mode = "100644", object_type = "blob", oid = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb" }
"#;
        let state: PromotionState = toml::from_str(text)?;
        validate(&state)?;
        assert_eq!(state.source_authority.len(), 1);
        assert_eq!(
            state.source_authority[0].path,
            ".github/workflows/release.yml"
        );
        Ok(())
    }

    #[test]
    fn rejects_unbounded_or_duplicate_source_authority_decisions() {
        let missing_targets = r#"
schema_version = 1
[latest_promotion]
status = "completed"
source_promotion_pr = "EffortlessMetrics/shiplog#655"
promoted_swarm_head = "c4fdba223d1c5c5b99a95b159ab8123d83d4b842"
[[source_authority]]
path = "release.yml"
decision_receipt = "EffortlessMetrics/shiplog-swarm#319"
decision_merge_sha = "7d0a1b2c3d4e5f60718293a4b5c6d7e8f9a0b1c2"
reason = "reviewed"
"#;
        let state: PromotionState = toml::from_str(missing_targets).expect("parse");
        let error = validate(&state).expect_err("active decisions need exact targets");
        assert!(
            error
                .to_string()
                .contains("exact source_target and swarm_target")
        );

        let duplicate = r#"
schema_version = 1
[latest_promotion]
status = "completed"
source_promotion_pr = "EffortlessMetrics/shiplog#655"
promoted_swarm_head = "c4fdba223d1c5c5b99a95b159ab8123d83d4b842"
[[source_authority]]
path = "release.yml"
source_target = "1111111111111111111111111111111111111111"
swarm_target = "2222222222222222222222222222222222222222"
decision_receipt = "EffortlessMetrics/shiplog-swarm#319"
decision_merge_sha = "7d0a1b2c3d4e5f60718293a4b5c6d7e8f9a0b1c2"
reason = "reviewed"
[[source_authority]]
path = "release.yml"
source_target = "1111111111111111111111111111111111111111"
swarm_target = "2222222222222222222222222222222222222222"
decision_receipt = "EffortlessMetrics/shiplog-swarm#319"
decision_merge_sha = "7d0a1b2c3d4e5f60718293a4b5c6d7e8f9a0b1c2"
reason = "reviewed"
"#;
        let state: PromotionState = toml::from_str(duplicate).expect("parse");
        let error = validate(&state).expect_err("one path may have only one decision");
        assert!(error.to_string().contains("appears more than once"));
    }

    #[test]
    fn recorded_receipts_cover_source_governance_and_included_prs() {
        let state = completed_manifest();
        let receipts = state.recorded_receipts();
        assert!(receipts.contains(&"EffortlessMetrics/shiplog#655".to_string()));
        assert!(receipts.contains(&"EffortlessMetrics/shiplog#656".to_string()));
        assert!(receipts.contains(&"EffortlessMetrics/shiplog-swarm#238".to_string()));
        // Pending work is not a recorded (carried) receipt.
        assert!(!receipts.contains(&"EffortlessMetrics/shiplog-swarm#248".to_string()));
    }

    #[test]
    fn deferred_receipts_are_carried_but_marked_deferred() {
        let text = r#"
schema_version = 1
[latest_promotion]
status = "completed"
source_promotion_pr = "EffortlessMetrics/shiplog#655"
promoted_swarm_head = "c4fdba22"
[pending]
deferred_receipt_carry = ["EffortlessMetrics/shiplog-swarm#240"]
"#;
        let state: PromotionState = toml::from_str(text).expect("parse");
        validate(&state).expect("valid");
        assert!(state.carries_receipt("EffortlessMetrics/shiplog-swarm#240"));
        assert!(state.is_deferred("EffortlessMetrics/shiplog-swarm#240"));
        assert!(!state.is_deferred("EffortlessMetrics/shiplog#655"));
    }

    #[test]
    fn checked_in_current_promotion_md_matches_manifest() {
        // The workspace root is the parent of the xtask crate directory.
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask crate has a parent workspace directory");
        let state = load(workspace_root).expect("load checked-in promotion-state manifest");
        let expected = render_markdown(&state);
        let actual = fs::read_to_string(workspace_root.join(GENERATED_REL))
            .expect("read checked-in current-promotion.md")
            .replace("\r\n", "\n");
        assert_eq!(
            actual, expected,
            "plans/shiplog-swarm/current-promotion.md is out of sync with promotion-state.toml; \
             regenerate it with `cargo xtask promotion-state`"
        );
    }

    #[test]
    fn generated_markdown_has_banner_and_is_deterministic() {
        let state = completed_manifest();
        let first = render_markdown(&state);
        let second = render_markdown(&state);
        assert_eq!(first, second);
        assert!(first.starts_with(GENERATED_BANNER));
        assert!(first.contains("c4fdba223d1c5c5b99a95b159ab8123d83d4b842"));
        assert!(first.contains("EffortlessMetrics/shiplog-swarm#248"));
        assert!(first.contains("## Truth hierarchy"));
    }

    #[test]
    fn generated_markdown_includes_source_authority_metadata() {
        let mut state = completed_manifest();
        state.source_authority.push(SourceAuthorityDecision {
            path: ".github/workflows/release.yml".to_string(),
            source_target: "1111111111111111111111111111111111111111".to_string(),
            swarm_target: "2222222222222222222222222222222222222222".to_string(),
            decision_receipt: "EffortlessMetrics/shiplog-swarm#319".to_string(),
            decision_merge_sha: "7d0a1b2c3d4e5f60718293a4b5c6d7e8f9a0b1c2".to_string(),
            reason: "Source retains release-writer authority.".to_string(),
            consumed_by: String::new(),
            source_tree_entry: None,
            swarm_tree_entry: None,
        });

        let rendered = render_markdown(&state);
        assert!(rendered.contains("## Source-authority decisions"));
        assert!(rendered.contains(".github/workflows/release.yml"));
        assert!(rendered.contains("Source retains release-writer authority."));
        assert!(rendered.contains("source target: `1111111111111111111111111111111111111111`"));
        assert!(rendered.contains("swarm target: `2222222222222222222222222222222222222222`"));
        assert!(rendered.contains("consumed by: `(active)`"));
    }
}
