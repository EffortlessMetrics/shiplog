//! Transition receipts: bounded divergence authority for the swarm cutover.
//!
//! During the cutover, some changes landed directly on the source repository
//! instead of flowing through a swarm promotion. A transition receipt records
//! one such source PR and, per path, what the swarm side did about it. That
//! evidence is what lets `promote` accept a difference between the two trees
//! that `policy/source-only-paths.toml` does not permanently allow.
//!
//! Three properties keep this from becoming a standing bypass:
//!
//! * **Per-path disposition.** A source PR is rarely uniform. The port that
//!   landed as source #666 / swarm #274 left seven paths byte-identical and six
//!   intentionally different, so one status over one path list would have been
//!   false for half of it.
//! * **Bounded lifetime.** An entry grants nothing once `consumed_by` names the
//!   promotion that reconciled it. Entries are kept as history, not authority.
//! * **Narrow authority.** Only a resolved receipt (`equivalent`,
//!   `dependency_equivalent`, `tree_equivalent`, `superseded_in_swarm`) grants anything, and only
//!   permission for the overlay
//!   to keep swarm content on a path both sides changed, because swarm
//!   demonstrably already carries or supersedes the source change.
//!   `missing_in_swarm` grants nothing and blocks: swarm does not carry the
//!   change, so promoting would revert it. `conflicting` blocks outright.
//!   An exceptional `resolution = "discard_source"` may explicitly choose the
//!   swarm tree for a missing-in-swarm path, but only with exact tree entries,
//!   a decision receipt, and a reason; it is bounded by `consumed_by`.
//!
//! Evidence is checked rather than assumed: the recorded merge SHAs must belong
//! to merged PRs and be reachable from the relevant branch, `equivalent` must
//! agree under `git patch-id --stable` for that path, `dependency_equivalent`
//! must agree on the exact Cargo.lock package-version transitions,
//! `tree_equivalent` must resolve to the same blob at that path on both promoted refs, and
//! `superseded_in_swarm` must present a contiguous version chain starting where
//! the source change started.
//!
//! `equivalent` and `tree_equivalent` are separate dispositions on purpose.
//! Patch identity is the stronger claim and stays exactly as strict as it was;
//! `tree_equivalent` exists for the real case where two repositories started
//! from different content and converged on identical content by different
//! patches, which patch identity cannot express and must not be made to.

use anyhow::{Context, Result, bail, ensure};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::promotion_state::{
    PROMOTION_STATE_PATH, SourceAuthorityDecision, Transition, TransitionDisposition,
    TransitionPath, TransitionResolution, TreeEntry,
};

/// Divergence the transition receipts justify, split by the shape of the
/// difference they actually prove.
#[derive(Debug, Default)]
pub struct TransitionAuthority {
    /// Paths both repositories changed, where a receipt proves the two changes
    /// are reconciled, so the overlay may keep the swarm content. Carries the
    /// evidence that earned it, so a decision can state its basis rather than
    /// just its effect.
    pub two_sided: BTreeMap<String, ResolvedReceipt>,
    /// Paths a receipt records as changed on source with no swarm counterpart,
    /// mapped to the receipt that records it.
    ///
    /// These grant nothing. The overlay keeps swarm content for any path the
    /// policy does not mark source-authoritative, so promoting would revert the
    /// source change. They are carried only so the refusal can name the receipt
    /// and say what to do about it.
    pub awaiting_swarm: BTreeMap<String, String>,
    /// Explicit human-reviewed decisions to discard a source-side change and
    /// take the exact swarm tree entry for this bounded promotion.
    pub discard_source: BTreeMap<String, DiscardSourceDecision>,
    /// Explicit human-reviewed decisions to retain the exact source tree entry
    /// for a source-authoritative path that swarm changed during this bounded
    /// promotion.
    pub source_authority: BTreeMap<String, SourceAuthorityDecision>,
    /// Source merge commits an active receipt accounts for. A source commit that
    /// followed the last promotion merge is otherwise unapproved divergence, so
    /// without this the path-level authority above could never be reached.
    pub source_commits: BTreeSet<String>,
}

/// The evidence behind a resolved path grant.
#[derive(Clone, Debug)]
pub struct ResolvedReceipt {
    pub source_pr: String,
    pub swarm_chain: Vec<String>,
    pub disposition: TransitionDisposition,
}

/// The reviewable decision behind an exceptional discard-source resolution.
#[derive(Clone, Debug)]
pub struct DiscardSourceDecision {
    pub source_pr: String,
    pub decision_receipt: String,
    pub decision_merge_sha: String,
    pub reason: String,
}

/// Repository access the evidence checks need.
pub trait TransitionPort {
    fn git_output(&self, workspace_root: &Path, args: &[&str]) -> Result<String>;
    fn gh_output(&self, args: &[&str]) -> Result<Vec<u8>>;
    /// `git patch-id --stable` over `patch`, returning the patch id alone.
    fn git_patch_id(&self, patch: &str) -> Result<String>;
}

/// Where a receipt's recorded merge commit must be reachable from.
pub struct TransitionRefs<'a> {
    pub source_repo: &'a str,
    pub swarm_repo: &'a str,
    /// Exact source commit selected for this promotion.
    pub source_target: &'a str,
    /// Exact swarm commit selected for this promotion.
    pub swarm_target: &'a str,
}

/// Check every active transition receipt and return the divergence they earn.
pub fn derive_authority(
    port: &impl TransitionPort,
    workspace_root: &Path,
    refs: &TransitionRefs<'_>,
    transitions: &[Transition],
) -> Result<TransitionAuthority> {
    let mut authority = TransitionAuthority::default();
    for entry in transitions {
        // A consumed receipt is retained as history and skipped. Re-validating it
        // would make a migration record permanently load-bearing, which is the
        // failure `consumed_by` exists to prevent.
        if entry.consumed_by().is_some() {
            continue;
        }
        ensure_evidence_targets(
            port,
            workspace_root,
            &format!("active transition {}", entry.source_pr),
            &entry.source_target,
            &entry.swarm_target,
            refs,
        )?;
        check_merged_at(
            port,
            workspace_root,
            refs.source_repo,
            &entry.source_pr,
            &entry.source_merge_sha,
            refs.source_target,
        )
        .with_context(|| format!("transition {}: source merge evidence", entry.source_pr))?;

        // Recorded only after the merge evidence above passed, so an unverified
        // receipt cannot let a source commit through the ancestry walk.
        authority
            .source_commits
            .insert(entry.source_merge_sha.clone());

        let source_patch = pull_request_patch(port, refs.source_repo, &entry.source_pr)?;
        for path in &entry.path {
            check_path(
                port,
                workspace_root,
                refs,
                entry,
                path,
                &source_patch,
                &mut authority,
            )
            .with_context(|| {
                format!(
                    "transition {} path {}: {}",
                    entry.source_pr, path.path, path.disposition
                )
            })?;
        }
    }
    Ok(authority)
}

/// Verify explicit source-authority decisions and return the paths they allow
/// the overlay to keep from source. A source-only policy entry by itself is not
/// enough when swarm touched the path: this receipt is the bounded decision
/// that makes the discarded swarm change visible and reviewable.
pub fn derive_source_authority(
    port: &impl TransitionPort,
    workspace_root: &Path,
    refs: &TransitionRefs<'_>,
    source_only_paths: &[String],
    decisions: &[SourceAuthorityDecision],
) -> Result<BTreeMap<String, SourceAuthorityDecision>> {
    let mut authority = BTreeMap::new();
    for decision in decisions {
        if decision.consumed_by().is_some() {
            continue;
        }
        if authority.contains_key(&decision.path) {
            bail!(
                "source_authority path {} appears more than once",
                decision.path
            );
        }
        if !source_only_paths.iter().any(|path| path == &decision.path) {
            bail!(
                "source_authority path {} is not listed in policy/source-only-paths.toml",
                decision.path
            );
        }
        ensure_evidence_targets(
            port,
            workspace_root,
            &format!("source_authority {}", decision.path),
            &decision.source_target,
            &decision.swarm_target,
            refs,
        )?;
        let evidence_source_entry = tree_entry(
            port,
            workspace_root,
            &decision.source_target,
            &decision.path,
        )?;
        let evidence_swarm_entry =
            tree_entry(port, workspace_root, &decision.swarm_target, &decision.path)?;
        verify_tree_entries(
            decision.source_tree_entry.as_ref(),
            decision.swarm_tree_entry.as_ref(),
            evidence_source_entry.as_ref(),
            evidence_swarm_entry.as_ref(),
        )?;
        let current_source_entry =
            tree_entry(port, workspace_root, refs.source_target, &decision.path)?;
        let current_swarm_entry =
            tree_entry(port, workspace_root, refs.swarm_target, &decision.path)?;
        if current_source_entry == current_swarm_entry {
            bail!(
                "source_authority path {} requires differing source and swarm tree entries at the current promotion targets, but both are {current_source_entry:?}",
                decision.path,
            );
        }
        verify_tree_entries(
            decision.source_tree_entry.as_ref(),
            decision.swarm_tree_entry.as_ref(),
            current_source_entry.as_ref(),
            current_swarm_entry.as_ref(),
        )?;
        check_merged_at(
            port,
            workspace_root,
            refs.swarm_repo,
            &decision.decision_receipt,
            &decision.decision_merge_sha,
            refs.swarm_target,
        )
        .with_context(|| {
            format!(
                "source_authority {}: decision receipt evidence",
                decision.path
            )
        })?;
        authority.insert(decision.path.clone(), decision.clone());
    }
    Ok(authority)
}

fn check_path(
    port: &impl TransitionPort,
    workspace_root: &Path,
    refs: &TransitionRefs<'_>,
    entry: &Transition,
    path: &TransitionPath,
    source_patch: &str,
    authority: &mut TransitionAuthority,
) -> Result<()> {
    if path.disposition == TransitionDisposition::Conflicting {
        bail!("marked conflicting; resolve it before promoting");
    }
    // Established before any disposition grants anything. A receipt may only
    // speak for paths its own source PR touched, otherwise one merged PR would
    // authorize divergence on an unrelated file it never changed.
    let source_section = patch_for_path(source_patch, &path.path).with_context(|| {
        format!(
            "source PR {} does not touch this path",
            entry.source_pr.as_str()
        )
    })?;

    if path.disposition == TransitionDisposition::MissingInSwarm {
        if !path.swarm_chain.is_empty() {
            bail!("missing_in_swarm must not name swarm PRs");
        }
        if path.resolution == Some(TransitionResolution::DiscardSource) {
            let evidence_source_entry =
                tree_entry(port, workspace_root, &entry.source_target, &path.path)?;
            let evidence_swarm_entry =
                tree_entry(port, workspace_root, &entry.swarm_target, &path.path)?;
            verify_tree_entries(
                path.source_tree_entry.as_ref(),
                path.swarm_tree_entry.as_ref(),
                evidence_source_entry.as_ref(),
                evidence_swarm_entry.as_ref(),
            )?;
            let current_source_entry =
                tree_entry(port, workspace_root, refs.source_target, &path.path)?;
            let current_swarm_entry =
                tree_entry(port, workspace_root, refs.swarm_target, &path.path)?;
            if current_source_entry == current_swarm_entry {
                bail!(
                    "discard_source requires differing source and swarm tree entries at the current promotion targets, but both are {current_source_entry:?}"
                );
            }
            verify_current_tree_entries(
                path,
                path.source_tree_entry.as_ref(),
                path.swarm_tree_entry.as_ref(),
                current_source_entry.as_ref(),
                current_swarm_entry.as_ref(),
                evidence_source_entry.as_ref(),
                evidence_swarm_entry.as_ref(),
            )?;
            check_merged_at(
                port,
                workspace_root,
                refs.swarm_repo,
                &path.decision_receipt,
                &path.decision_merge_sha,
                refs.swarm_target,
            )
            .with_context(|| {
                format!(
                    "discard_source decision {}: decision receipt evidence",
                    path.decision_receipt
                )
            })?;
            authority.two_sided.remove(&path.path);
            authority.awaiting_swarm.remove(&path.path);
            authority.discard_source.insert(
                path.path.clone(),
                DiscardSourceDecision {
                    source_pr: entry.source_pr.clone(),
                    decision_receipt: path.decision_receipt.clone(),
                    decision_merge_sha: path.decision_merge_sha.clone(),
                    reason: path.reason.clone(),
                },
            );
            return Ok(());
        }
        // Deliberately not an authority grant. Swarm does not carry this change,
        // and the overlay keeps swarm content for paths outside
        // `policy/source-only-paths.toml`, so promoting would revert it. Record
        // it so the refusal can explain itself.
        // A newer missing-in-swarm receipt supersedes any older resolved receipt
        // for this path. Keeping both would let the planner select stale
        // two-sided authority before it sees the newer blocking evidence.
        authority.two_sided.remove(&path.path);
        authority.discard_source.remove(&path.path);
        authority
            .awaiting_swarm
            .insert(path.path.clone(), entry.source_pr.clone());
        return Ok(());
    }

    if path.disposition == TransitionDisposition::ConvergedAtTarget {
        let evidence_source_entry =
            tree_entry(port, workspace_root, &entry.source_target, &path.path)?;
        let evidence_swarm_entry =
            tree_entry(port, workspace_root, &entry.swarm_target, &path.path)?;
        verify_tree_entries(
            path.source_tree_entry.as_ref(),
            path.swarm_tree_entry.as_ref(),
            evidence_source_entry.as_ref(),
            evidence_swarm_entry.as_ref(),
        )?;
        ensure!(
            evidence_source_entry == evidence_swarm_entry,
            "converged_at_target requires identical source and swarm evidence entries"
        );
        let current_source_entry =
            tree_entry(port, workspace_root, refs.source_target, &path.path)?;
        let current_swarm_entry = tree_entry(port, workspace_root, refs.swarm_target, &path.path)?;
        verify_current_tree_entries(
            path,
            path.source_tree_entry.as_ref(),
            path.swarm_tree_entry.as_ref(),
            current_source_entry.as_ref(),
            current_swarm_entry.as_ref(),
            evidence_source_entry.as_ref(),
            evidence_swarm_entry.as_ref(),
        )?;
        check_merged_at(
            port,
            workspace_root,
            refs.swarm_repo,
            &path.decision_receipt,
            &path.decision_merge_sha,
            refs.swarm_target,
        )
        .with_context(|| {
            format!(
                "converged_at_target decision {}: decision receipt evidence",
                path.decision_receipt
            )
        })?;
        authority.discard_source.insert(
            path.path.clone(),
            DiscardSourceDecision {
                source_pr: entry.source_pr.clone(),
                decision_receipt: path.decision_receipt.clone(),
                decision_merge_sha: path.decision_merge_sha.clone(),
                reason: path.reason.clone(),
            },
        );
        authority.two_sided.remove(&path.path);
        authority.awaiting_swarm.remove(&path.path);
        return Ok(());
    }

    if path.swarm_chain.is_empty() {
        bail!("{} requires at least one swarm PR", path.disposition);
    }
    let mut swarm_patches = Vec::new();
    for swarm_pr in &path.swarm_chain {
        let merge_sha = entry.swarm_merge_sha(swarm_pr).with_context(|| {
            format!("{swarm_pr} is named in a chain but has no recorded swarm_merge_sha")
        })?;
        check_merged_at(
            port,
            workspace_root,
            refs.swarm_repo,
            swarm_pr,
            merge_sha,
            refs.swarm_target,
        )?;
        swarm_patches.push(pull_request_patch(port, refs.swarm_repo, swarm_pr)?);
    }

    let mut current_tree_entries = None;

    match path.disposition {
        TransitionDisposition::Equivalent => {
            if path.swarm_chain.len() != 1 {
                bail!("equivalent must name exactly one swarm PR, not a chain");
            }
            let swarm_section =
                patch_for_path(&swarm_patches[0], &path.path).with_context(|| {
                    format!("swarm PR {} does not touch this path", path.swarm_chain[0])
                })?;
            let source_id = port.git_patch_id(&source_section)?;
            let swarm_id = port.git_patch_id(&swarm_section)?;
            if source_id != swarm_id {
                bail!(
                    "claimed equivalent but patch ids differ: source {source_id}, swarm {swarm_id}"
                );
            }
        }
        TransitionDisposition::DependencyEquivalent => {
            if path.path != "Cargo.lock" {
                bail!(
                    "dependency_equivalent is only valid for Cargo.lock, not {}",
                    path.path
                );
            }
            if path.swarm_chain.len() != 1 {
                bail!("dependency_equivalent must name exactly one swarm PR, not a chain");
            }
            let swarm_section =
                patch_for_path(&swarm_patches[0], &path.path).with_context(|| {
                    format!("swarm PR {} does not touch the path", path.swarm_chain[0])
                })?;
            let source_transitions = lock_transitions(&source_section);
            let swarm_transitions = lock_transitions(&swarm_section);
            if source_transitions.is_empty() || source_transitions != swarm_transitions {
                bail!(
                    "claimed dependency_equivalent but Cargo.lock package-version transitions differ: source {source_transitions:?}, swarm {swarm_transitions:?}"
                );
            }
        }
        TransitionDisposition::TreeEquivalent => {
            if path.swarm_chain.len() != 1 {
                bail!("tree_equivalent must name exactly one swarm PR, not a chain");
            }
            // The swarm PR must have touched the path too, exactly as
            // `equivalent` requires. Otherwise a path the two trees happen to
            // agree on could be claimed by a swarm PR that never went near it.
            patch_for_path(&swarm_patches[0], &path.path).with_context(|| {
                format!("swarm PR {} does not touch this path", path.swarm_chain[0])
            })?;
            // The evidence is the outcome, not the edit: the resulting blob at
            // this path must be byte-identical on both promoted refs. Blob ids
            // are content hashes, so equal ids mean equal bytes.
            let source_entry = tree_entry(port, workspace_root, refs.source_target, &path.path)?;
            let swarm_entry = tree_entry(port, workspace_root, refs.swarm_target, &path.path)?;
            if source_entry != swarm_entry {
                bail!(
                    "claimed tree_equivalent but the resulting tree entries differ: source {source_entry:?}, swarm {swarm_entry:?}"
                );
            }
            current_tree_entries = Some((source_entry, swarm_entry));
        }
        TransitionDisposition::SupersededInSwarm => {
            // The chain must begin by reproducing the source change, not merely
            // by starting at the same version. Without this, a swarm step from
            // the same old version to an unrelated new one satisfied the walk
            // while never incorporating what source landed.
            let first = patch_for_path(&swarm_patches[0], &path.path).with_context(|| {
                format!(
                    "chained swarm PR {} does not touch this path",
                    path.swarm_chain[0]
                )
            })?;
            let reproduces = port.git_patch_id(&source_section)? == port.git_patch_id(&first)?
                || reaches_source_result(&source_section, &first);
            if !reproduces {
                bail!(
                    "chain does not start by reproducing the source change: {} must be equivalent to {} for this path, or land the same resulting versions",
                    path.swarm_chain[0],
                    entry.source_pr
                );
            }
            check_supersession_chain(&source_section, &swarm_patches, &path.path)?;
        }
        _ => unreachable!("handled above"),
    }
    if current_tree_entries.is_none() {
        current_tree_entries = Some((
            tree_entry(port, workspace_root, refs.source_target, &path.path)?,
            tree_entry(port, workspace_root, refs.swarm_target, &path.path)?,
        ));
    }
    let Some((source_entry, swarm_entry)) = current_tree_entries.as_ref() else {
        bail!("resolved transition path has no current tree entries");
    };
    let evidence_source_entry = tree_entry(port, workspace_root, &entry.source_target, &path.path)?;
    let evidence_swarm_entry = tree_entry(port, workspace_root, &entry.swarm_target, &path.path)?;
    verify_tree_entries(
        path.source_tree_entry.as_ref(),
        path.swarm_tree_entry.as_ref(),
        evidence_source_entry.as_ref(),
        evidence_swarm_entry.as_ref(),
    )?;
    verify_current_tree_entries(
        path,
        path.source_tree_entry.as_ref(),
        path.swarm_tree_entry.as_ref(),
        source_entry.as_ref(),
        swarm_entry.as_ref(),
        evidence_source_entry.as_ref(),
        evidence_swarm_entry.as_ref(),
    )?;
    // A resolved receipt reconciles a path both sides touched. It deliberately
    // does not grant one-sided source authority: only
    // `policy/source-only-paths.toml` may select source content, so a settled
    // migration can never manufacture temporary source authority.
    authority.two_sided.insert(
        path.path.clone(),
        ResolvedReceipt {
            source_pr: entry.source_pr.clone(),
            swarm_chain: path.swarm_chain.clone(),
            disposition: path.disposition,
        },
    );
    authority.discard_source.remove(&path.path);
    Ok(())
}

/// Evidence targets are immutable observations. They must remain ancestors of
/// the current promotion targets, while the recorded tree entries are checked
/// again at both target pairs below. This permits unrelated tip advancement
/// without allowing a later path change or historical convergence to reuse an
/// old decision.
fn ensure_evidence_targets(
    port: &impl TransitionPort,
    workspace_root: &Path,
    label: &str,
    source_target: &str,
    swarm_target: &str,
    refs: &TransitionRefs<'_>,
) -> Result<()> {
    if source_target.is_empty() || swarm_target.is_empty() {
        bail!("{label} has no source_target/swarm_target evidence binding");
    }
    for (side, evidence_target, current_target) in [
        ("source", source_target, refs.source_target),
        ("swarm", swarm_target, refs.swarm_target),
    ] {
        port.git_output(
            workspace_root,
            &["merge-base", "--is-ancestor", evidence_target, current_target],
        )
        .with_context(|| {
            format!(
                "{label}: {side} evidence target {evidence_target} is not an ancestor of current promotion target {current_target}"
            )
        })?;
    }
    Ok(())
}

fn verify_tree_entries(
    recorded_source: Option<&TreeEntry>,
    recorded_swarm: Option<&TreeEntry>,
    source_entry: Option<&TreeEntry>,
    swarm_entry: Option<&TreeEntry>,
) -> Result<()> {
    if recorded_source != source_entry {
        bail!(
            "recorded source tree entry does not match the exact promotion target: recorded {:?}, current {:?}",
            recorded_source,
            source_entry
        );
    }
    if recorded_swarm != swarm_entry {
        bail!(
            "recorded swarm tree entry does not match the exact promotion target: recorded {:?}, current {:?}",
            recorded_swarm,
            swarm_entry
        );
    }
    Ok(())
}

fn verify_current_tree_entries(
    path: &TransitionPath,
    recorded_source: Option<&TreeEntry>,
    recorded_swarm: Option<&TreeEntry>,
    current_source: Option<&TreeEntry>,
    current_swarm: Option<&TreeEntry>,
    evidence_source: Option<&TreeEntry>,
    evidence_swarm: Option<&TreeEntry>,
) -> Result<()> {
    if !path.self_referential && path.disposition != TransitionDisposition::ConvergedAtTarget {
        return verify_tree_entries(
            recorded_source,
            recorded_swarm,
            current_source,
            current_swarm,
        );
    }

    if path.self_referential {
        ensure!(
            path.path == PROMOTION_STATE_PATH,
            "self-referential transition path {} is not the canonical promotion manifest",
            path.path
        );
    } else {
        ensure!(
            path.disposition == TransitionDisposition::ConvergedAtTarget,
            "unexpected current-tree exception for transition path {}",
            path.path
        );
        ensure!(
            recorded_source == recorded_swarm,
            "converged_at_target requires identical recorded source and swarm entries"
        );
    }
    ensure!(
        recorded_source == current_source,
        "self-referential promotion manifest source entry changed: recorded {:?}, current {:?}",
        recorded_source,
        current_source
    );
    ensure!(
        current_swarm.is_some(),
        "special transition path must exist at the current swarm target"
    );
    ensure!(
        evidence_swarm != current_swarm,
        "special transition path must materialize a new current swarm entry"
    );
    // Historical entries remain exact evidence. The current swarm entry is
    // intentionally different either because this receipt is stored in the
    // manifest it describes, or because swarm made a later product change
    // after both repositories had converged at the evidence target.
    verify_tree_entries(
        recorded_source,
        recorded_swarm,
        evidence_source,
        evidence_swarm,
    )
}

/// Confirm the PR is merged, the recorded SHA is that merge, and the merge is
/// reachable from the branch that repository promotes from.
///
/// Recording a SHA is not evidence on its own: a well-formed but wrong SHA is
/// indistinguishable from a correct one without asking the forge which commit
/// the PR actually produced.
fn check_merged_at(
    port: &impl TransitionPort,
    workspace_root: &Path,
    repo: &str,
    receipt: &str,
    merge_sha: &str,
    reachable_from: &str,
) -> Result<()> {
    let number = receipt_number(receipt, repo)?;
    let raw = port.gh_output(&[
        "pr",
        "view",
        &number,
        "--repo",
        repo,
        "--json",
        "state,mergeCommit",
    ])?;
    let view: serde_json::Value =
        serde_json::from_slice(&raw).with_context(|| format!("parse {receipt} PR view"))?;
    let state = view.get("state").and_then(|value| value.as_str());
    if state != Some("MERGED") {
        bail!("{receipt} is {}, not MERGED", state.unwrap_or("unknown"));
    }
    let merged = view
        .get("mergeCommit")
        .and_then(|commit| commit.get("oid"))
        .and_then(|oid| oid.as_str())
        .with_context(|| format!("{receipt} reports no merge commit"))?;
    // Compared in full rather than by prefix. This is a fail-closed authority
    // boundary, so an abbreviation that merely happens to share a prefix must not
    // pass as evidence.
    if merged != merge_sha {
        bail!("{receipt} merged as {merged}, but the receipt records {merge_sha}");
    }
    // Reachability is checked locally: the forge can say what a PR merged as,
    // but only ancestry proves that commit is still part of the promoted line
    // rather than something later rewritten away.
    port.git_output(
        workspace_root,
        &["merge-base", "--is-ancestor", merge_sha, reachable_from],
    )
    .with_context(|| {
        format!("{receipt} merge {merge_sha} is not reachable from {reachable_from}")
    })?;
    Ok(())
}

/// The complete tree entry for `path` as it stands on an exact commit.
///
/// Absence is meaningful: two missing paths are equivalent, while a missing
/// path and a present path are not. Mode and object type are part of the
/// result, so executable files, symlinks, and gitlinks cannot be confused with
/// regular files that happen to share an object id.
fn tree_entry(
    port: &impl TransitionPort,
    workspace_root: &Path,
    git_target: &str,
    path: &str,
) -> Result<Option<TreeEntry>> {
    let literal_path = format!(":(literal){path}");
    let output = port
        .git_output(
            workspace_root,
            &[
                "ls-tree",
                "-z",
                "--full-tree",
                git_target,
                "--",
                &literal_path,
            ],
        )
        .with_context(|| format!("inspect {git_target}:{path}"))?;
    let Some(record) = output.split('\0').find(|record| !record.is_empty()) else {
        return Ok(None);
    };
    let (metadata, recorded_path) = record
        .split_once('\t')
        .with_context(|| format!("parse tree entry for {git_target}:{path}"))?;
    if recorded_path != path {
        bail!("tree lookup for {git_target}:{path} returned unexpected path {recorded_path}");
    }
    let mut fields = metadata.split_whitespace();
    let mode = fields
        .next()
        .context("tree entry is missing its mode")?
        .to_string();
    let object_type = fields
        .next()
        .context("tree entry is missing its object type")?
        .to_string();
    let oid = fields
        .next()
        .context("tree entry is missing its object id")?
        .to_string();
    if fields.next().is_some() {
        bail!("tree entry for {git_target}:{path} has unexpected fields");
    }
    Ok(Some(TreeEntry {
        mode,
        object_type,
        oid,
    }))
}

/// Require the swarm steps to form an unbroken version chain that starts where
/// the source change started.
///
/// Proving the versions merely differ is not supersession: an unrelated
/// downgrade also differs. The chain has to begin at the source change's own
/// starting version so the swarm side demonstrably continues that history.
fn check_supersession_chain(
    source_section: &str,
    swarm_patches: &[String],
    path: &str,
) -> Result<()> {
    let source = lock_transitions(source_section);
    if source.is_empty() {
        bail!("no Cargo.lock package versions found in the source change for {path}");
    }
    let mut steps = Vec::new();
    for patch in swarm_patches {
        let section = patch_for_path(patch, path)
            .with_context(|| format!("a chained swarm PR does not touch {path}"))?;
        steps.push(lock_transitions(&section));
    }
    let mut proven = false;
    for (package, (source_from, source_to)) in &source {
        let mut cursor = match source_from {
            Some(from) => from.clone(),
            None => continue,
        };
        let mut advanced = false;
        for step in &steps {
            let Some((step_from, step_to)) = step.get(package) else {
                continue;
            };
            let (Some(step_from), Some(step_to)) = (step_from, step_to) else {
                continue;
            };
            if step_from != &cursor {
                bail!(
                    "chain for {package} is not contiguous: expected a step starting at {cursor}, found one starting at {step_from}"
                );
            }
            cursor = step_to.clone();
            advanced = true;
        }
        if !advanced {
            continue;
        }
        if let Some(source_to) = source_to
            && &cursor == source_to
        {
            bail!(
                "chain for {package} ends at {cursor}, the same version source landed; that is equivalent, not superseded"
            );
        }
        proven = true;
    }
    if !proven {
        bail!("no chained swarm change continues the source version history for {path}");
    }
    Ok(())
}

/// `name`/`version` pairs a Cargo.lock diff moves, as `package -> (from, to)`.
fn lock_transitions(section: &str) -> BTreeMap<String, (Option<String>, Option<String>)> {
    let mut transitions: BTreeMap<String, (Option<String>, Option<String>)> = BTreeMap::new();
    let mut package: Option<String> = None;
    for line in section.lines() {
        if line.starts_with("+++") || line.starts_with("---") || line.starts_with("diff --git") {
            continue;
        }
        let Some(content) = line.strip_prefix('+').or_else(|| line.strip_prefix('-')) else {
            if let Some(rest) = line.strip_prefix(' ')
                && let Some(name) = keyed_value(rest, "name")
            {
                package = Some(name);
            }
            continue;
        };
        if let Some(name) = keyed_value(content, "name") {
            package = Some(name);
            continue;
        }
        if let Some(version) = keyed_value(content, "version")
            && let Some(name) = package.clone()
        {
            let entry = transitions.entry(name).or_insert((None, None));
            if line.starts_with('-') {
                entry.0 = Some(version);
            } else {
                entry.1 = Some(version);
            }
        }
    }
    transitions
}

/// Parse a `key = "value"` line through the TOML parser, so escapes and
/// malformed values are rejected rather than mis-sliced.
fn keyed_value(line: &str, key: &str) -> Option<String> {
    let line = line.trim();
    if !line.strip_prefix(key)?.trim_start().starts_with('=') {
        return None;
    }
    let table = line.parse::<toml::Table>().ok()?;
    let value = table.get(key)?.as_str()?;
    (!value.is_empty()).then(|| value.to_string())
}

/// The `diff --git` section for exactly `path`.
///
/// Identity is computed per section so it carries the file it belongs to. A
/// whole-patch digest over added and removed lines would treat the same edit
/// applied to a different file as the same change.
fn patch_for_path(patch: &str, path: &str) -> Option<String> {
    let mut section: Option<Vec<&str>> = None;
    for line in patch.lines() {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            let matches = rest
                .split_whitespace()
                .nth(1)
                .and_then(|value| value.strip_prefix("b/"))
                .is_some_and(|value| value == path);
            if matches {
                section = Some(vec![line]);
                continue;
            }
            if section.is_some() {
                break;
            }
            continue;
        }
        if let Some(lines) = section.as_mut() {
            lines.push(line);
        }
    }
    let lines = section?;
    let mut text = lines.join("\n");
    text.push('\n');
    Some(text)
}

/// Stable patch identity via git itself, rather than a hand-rolled digest.
///
/// `--stable` ignores line numbers but keeps file identity, so the same edit
/// applied to a different file is a different id.
pub fn system_patch_id(patch: &str) -> Result<String> {
    let stdout = run_git_with_stdin(&["patch-id", "--stable"], patch)?;
    // Output is "<patch-id> <commit-id>"; an empty patch yields nothing at all,
    // which must not silently compare equal to another empty result.
    stdout
        .split_whitespace()
        .next()
        .map(str::to_string)
        .context("transition: git patch-id produced no id for the supplied patch")
}

/// `git hash-object` over arbitrary content.
///
/// Gives a stable content id using git's own hashing rather than taking on a
/// digest dependency for one identifier.
pub fn git_hash_object(content: &str) -> Result<String> {
    run_git_with_stdin(&["hash-object", "--stdin"], content)?
        .split_whitespace()
        .next()
        .map(str::to_string)
        .context("transition: git hash-object produced no id")
}

pub(super) fn decode_git_stdout(args: &[&str], stdout: &[u8]) -> Result<String> {
    String::from_utf8(stdout.to_vec()).with_context(|| {
        format!(
            "git {} produced non-UTF-8 stdout; refusing to interpret path identity",
            args.join(" ")
        )
    })
}

fn run_git_with_stdin(args: &[&str], stdin: &str) -> Result<String> {
    use std::io::Write as _;
    use std::process::{Command, Stdio};

    let mut child = Command::new("git")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("transition: run git {}", args.join(" ")))?;
    child
        .stdin
        .take()
        .context("transition: git stdin unavailable")?
        .write_all(stdin.as_bytes())
        .context("transition: write to git stdin")?;
    let output = child
        .wait_with_output()
        .context("transition: collect git output")?;
    if !output.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    decode_git_stdout(args, &output.stdout)
}

fn pull_request_patch(port: &impl TransitionPort, repo: &str, receipt: &str) -> Result<String> {
    let number = receipt_number(receipt, repo)?;
    let raw = port
        .gh_output(&["pr", "diff", &number, "--repo", repo, "--patch"])
        .with_context(|| format!("fetch patch for {receipt}"))?;
    String::from_utf8(raw).with_context(|| format!("decode patch for {receipt} as UTF-8"))
}

/// PR number from `owner/repo#number`, rejecting a receipt that names a
/// different repository than the one about to be queried.
///
/// Discarding the repository component would let a manifest record
/// `OtherOrg/OtherRepo#657` while the verifier answered from the expected
/// repository's #657 instead.
fn receipt_number(receipt: &str, expected_repo: &str) -> Result<String> {
    let (repo, number) = receipt
        .split_once('#')
        .with_context(|| format!("receipt {receipt:?} must be owner/repo#number"))?;
    if repo != expected_repo {
        bail!("receipt {receipt:?} names repository {repo:?}, expected {expected_repo:?}");
    }
    if number.is_empty() || !number.chars().all(|c| c.is_ascii_digit()) {
        bail!("receipt {receipt:?} must end with a numeric PR id");
    }
    Ok(number.to_string())
}

/// Whether a swarm path patch lands the same resulting versions as the source
/// patch, which is equivalence in effect even when the patch text differs
/// because the two repositories started from different content.
fn reaches_source_result(source_section: &str, swarm_section: &str) -> bool {
    let source = lock_transitions(source_section);
    let swarm = lock_transitions(swarm_section);
    if source.is_empty() {
        return false;
    }
    source.iter().all(|(package, (_, source_to))| {
        swarm
            .get(package)
            .is_some_and(|(_, swarm_to)| swarm_to == source_to)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    const CARGO_LOCK: &str = "Cargo.lock";
    const SOURCE: &str = "EffortlessMetrics/shiplog";
    const SWARM: &str = "EffortlessMetrics/shiplog-swarm";

    #[test]
    fn git_stdout_rejects_non_utf8_path_identity() -> Result<()> {
        let error = decode_git_stdout(&["ls-tree", "-rz"], &[b'1', 0xff])
            .expect_err("invalid Git bytes must not be lossily decoded");
        ensure!(error.to_string().contains("non-UTF-8 stdout"));
        ensure!(error.to_string().contains("path identity"));
        Ok(())
    }

    struct StubPort {
        /// `repo#n` to the PR view JSON the forge would return.
        views: BTreeMap<String, String>,
        /// `repo#n` to that PR's patch.
        patches: BTreeMap<String, String>,
        /// SHAs that are reachable from the ref they are checked against.
        reachable: BTreeSet<String>,
        /// `<commit>:<path>` to the complete tree entry, or explicit absence.
        tree_entries: BTreeMap<String, Option<TreeEntry>>,
        ancestry_checks: RefCell<Vec<String>>,
    }

    impl StubPort {
        fn new() -> Self {
            Self {
                views: BTreeMap::new(),
                patches: BTreeMap::new(),
                reachable: BTreeSet::new(),
                tree_entries: BTreeMap::new(),
                ancestry_checks: RefCell::new(Vec::new()),
            }
        }

        /// Record a regular-file tree entry for `<git_target>:<path>`.
        fn blob(mut self, git_ref: &str, path: &str, blob: &str) -> Self {
            self.tree_entries.insert(
                format!("{git_ref}:{path}"),
                Some(TreeEntry {
                    mode: "100644".to_string(),
                    object_type: "blob".to_string(),
                    oid: blob.to_string(),
                }),
            );
            self
        }

        fn tree(
            mut self,
            git_target: &str,
            path: &str,
            mode: &str,
            object_type: &str,
            oid: &str,
        ) -> Self {
            self.tree_entries.insert(
                format!("{git_target}:{path}"),
                Some(TreeEntry {
                    mode: mode.to_string(),
                    object_type: object_type.to_string(),
                    oid: oid.to_string(),
                }),
            );
            self
        }

        fn absent(mut self, git_target: &str, path: &str) -> Self {
            self.tree_entries
                .insert(format!("{git_target}:{path}"), None);
            self
        }

        fn merged(mut self, receipt: &str, merge_sha: &str, patch: &str) -> Self {
            let key = key_for(receipt);
            self.views.insert(
                key.clone(),
                format!("{{\"state\":\"MERGED\",\"mergeCommit\":{{\"oid\":\"{merge_sha}\"}}}}"),
            );
            self.patches.insert(key, patch.to_string());
            self.reachable.insert(merge_sha.to_string());
            self
        }

        fn open(mut self, receipt: &str, patch: &str) -> Self {
            let key = key_for(receipt);
            self.views.insert(
                key.clone(),
                "{\"state\":\"OPEN\",\"mergeCommit\":null}".to_string(),
            );
            self.patches.insert(key, patch.to_string());
            self
        }

        fn unreachable(mut self, merge_sha: &str) -> Self {
            self.reachable.remove(merge_sha);
            self
        }

        fn ancestor(mut self, target: &str) -> Self {
            self.reachable.insert(target.to_string());
            self
        }
    }

    fn key_for(receipt: &str) -> String {
        receipt.to_string()
    }

    impl TransitionPort for StubPort {
        fn git_output(&self, _workspace_root: &Path, args: &[&str]) -> Result<String> {
            if args.first() == Some(&"merge-base") {
                let sha = args
                    .get(2)
                    .context("stub: merge-base without a sha")?
                    .to_string();
                let current = args
                    .get(3)
                    .context("stub: merge-base without a current target")?;
                self.ancestry_checks.borrow_mut().push(sha.clone());
                if sha != *current && !self.reachable.contains(&sha) {
                    bail!("stub: {sha} is not an ancestor");
                }
                return Ok(String::new());
            }
            if args.first() == Some(&"rev-parse") {
                bail!("stub: unexpected legacy rev-parse lookup {args:?}");
            }
            if args.first() == Some(&"ls-tree") {
                let git_target = args.get(3).context("stub: ls-tree without a target")?;
                let pathspec = args.last().context("stub: ls-tree without a path")?;
                let path = pathspec
                    .strip_prefix(":(literal)")
                    .context("stub: ls-tree path was not literal")?;
                let key = format!("{git_target}:{path}");
                return match self.tree_entries.get(&key) {
                    Some(Some(entry)) => Ok(format!(
                        "{} {} {}\t{}\0",
                        entry.mode, entry.object_type, entry.oid, path
                    )),
                    Some(None) => Ok(String::new()),
                    None => bail!("stub: no tree entry recorded for {key}"),
                };
            }
            bail!("stub: unexpected git {args:?}")
        }

        fn gh_output(&self, args: &[&str]) -> Result<Vec<u8>> {
            let number = args.get(2).context("stub: gh call without a number")?;
            let repo = args
                .iter()
                .position(|arg| *arg == "--repo")
                .and_then(|index| args.get(index + 1))
                .context("stub: gh call without a repo")?;
            let key = format!("{repo}#{number}");
            match args.get(1) {
                Some(&"view") => {
                    let view = self
                        .views
                        .get(&key)
                        .with_context(|| format!("stub: no view for {key}"))?;
                    Ok(view.clone().into_bytes())
                }
                Some(&"diff") => {
                    let patch = self
                        .patches
                        .get(&key)
                        .with_context(|| format!("stub: no patch for {key}"))?;
                    Ok(patch.clone().into_bytes())
                }
                other => bail!("stub: unexpected gh pr {other:?}"),
            }
        }

        fn git_patch_id(&self, patch: &str) -> Result<String> {
            system_patch_id(patch)
        }
    }

    fn refs() -> TransitionRefs<'static> {
        TransitionRefs {
            source_repo: SOURCE,
            swarm_repo: SWARM,
            source_target: "origin/main",
            swarm_target: "swarm/main",
        }
    }

    fn entry(disposition: TransitionDisposition, chain: &[(&str, &str)]) -> Transition {
        let mut swarm_merge_sha = BTreeMap::new();
        for (receipt, sha) in chain {
            swarm_merge_sha.insert((*receipt).to_string(), (*sha).to_string());
        }
        Transition {
            source_pr: format!("{SOURCE}#657"),
            source_merge_sha: "e6a72ad11ab22d03b1cf434b237bf0fff9c145bf".to_string(),
            source_target: "origin/main".to_string(),
            swarm_target: "swarm/main".to_string(),
            consumed_by: String::new(),
            swarm_merge_sha,
            path: vec![TransitionPath {
                path: CARGO_LOCK.to_string(),
                disposition,
                resolution: None,
                decision_receipt: String::new(),
                decision_merge_sha: String::new(),
                reason: String::new(),
                self_referential: false,
                swarm_chain: chain
                    .iter()
                    .map(|(receipt, _)| (*receipt).to_string())
                    .collect(),
                source_tree_entry: None,
                swarm_tree_entry: None,
            }],
        }
    }

    fn bind_entries(
        mut transition: Transition,
        source: Option<TreeEntry>,
        swarm: Option<TreeEntry>,
    ) -> Transition {
        transition.path[0].source_tree_entry = source;
        transition.path[0].swarm_tree_entry = swarm;
        transition
    }

    fn blob_entry(oid: &str) -> TreeEntry {
        TreeEntry {
            mode: "100644".to_string(),
            object_type: "blob".to_string(),
            oid: oid.to_string(),
        }
    }

    /// A resolved receipt reconciles a path both sides changed. It must not also
    /// grant one-sided source authority, or a settled migration would approve
    /// unrelated future source-only drift on the same path.
    #[test]
    fn resolved_receipt_grants_two_sided_only() -> Result<()> {
        let patch = section(CARGO_LOCK, "1.52.3", "1.53.0", "tokio");
        let swarm = format!("{SWARM}#269");
        let port = StubPort::new()
            .merged(
                &format!("{SOURCE}#657"),
                "e6a72ad11ab22d03b1cf434b237bf0fff9c145bf",
                &patch,
            )
            .merged(&swarm, "ba4aeaf78cb17f980f1da05d24d9638033b95f68", &patch)
            .blob(
                "origin/main",
                CARGO_LOCK,
                "9f2c1b6a0d4e5f708192a3b4c5d6e7f809a1b2c3",
            )
            .blob(
                "swarm/main",
                CARGO_LOCK,
                "9f2c1b6a0d4e5f708192a3b4c5d6e7f809a1b2c3",
            );
        let entries = vec![bind_entries(
            entry(
                TransitionDisposition::Equivalent,
                &[(swarm.as_str(), "ba4aeaf78cb17f980f1da05d24d9638033b95f68")],
            ),
            Some(blob_entry("9f2c1b6a0d4e5f708192a3b4c5d6e7f809a1b2c3")),
            Some(blob_entry("9f2c1b6a0d4e5f708192a3b4c5d6e7f809a1b2c3")),
        )];

        let authority = derive_authority(&port, Path::new("."), &refs(), &entries)?;

        assert!(authority.two_sided.contains_key(CARGO_LOCK));
        assert!(
            authority.awaiting_swarm.is_empty(),
            "a resolved receipt has nothing awaiting swarm: {:?}",
            authority.awaiting_swarm
        );
        Ok(())
    }

    /// The source merge a receipt accounts for must be reported, so the ancestry
    /// walk can step over it. Without this the path-level authority is
    /// unreachable: a source commit following the promotion merge is rejected as
    /// unapproved divergence before alignment is ever consulted.
    #[test]
    fn active_receipt_accounts_for_its_source_merge_commit() -> Result<()> {
        let patch = section(CARGO_LOCK, "1.52.3", "1.53.0", "tokio");
        let sha = "e6a72ad11ab22d03b1cf434b237bf0fff9c145bf";
        let port = StubPort::new().merged(&format!("{SOURCE}#657"), sha, &patch);
        let mut entries = vec![entry(TransitionDisposition::MissingInSwarm, &[])];

        let active = derive_authority(&port, Path::new("."), &refs(), &entries)?;
        assert!(
            active.source_commits.contains(sha),
            "an active receipt must account for its source merge"
        );

        // A consumed receipt stops accounting for it, so the commit becomes
        // unapproved divergence again rather than staying permanently waved through.
        entries[0].consumed_by = format!("{SOURCE}#655");
        let consumed = derive_authority(&port, Path::new("."), &refs(), &entries)?;
        assert!(consumed.source_commits.is_empty());
        Ok(())
    }

    /// Evidence must gate the commit-level allowance too, not just the paths.
    #[test]
    fn unverified_receipt_accounts_for_nothing() {
        let patch = section(CARGO_LOCK, "1.52.3", "1.53.0", "tokio");
        let port = StubPort::new().open(&format!("{SOURCE}#657"), &patch);
        let entries = vec![entry(TransitionDisposition::MissingInSwarm, &[])];
        assert!(
            derive_authority(&port, Path::new("."), &refs(), &entries).is_err(),
            "an unmerged receipt must not account for its source commit"
        );
    }

    /// `missing_in_swarm` grants nothing. It records that swarm lacks the change
    /// so the refusal can name the receipt, and stops recording it once consumed.
    #[test]
    fn missing_in_swarm_records_but_never_grants() -> Result<()> {
        let patch = section(CARGO_LOCK, "1.52.3", "1.53.0", "tokio");
        let port = StubPort::new().merged(
            &format!("{SOURCE}#657"),
            "e6a72ad11ab22d03b1cf434b237bf0fff9c145bf",
            &patch,
        );
        let mut entries = vec![entry(TransitionDisposition::MissingInSwarm, &[])];

        let active = derive_authority(&port, Path::new("."), &refs(), &entries)?;
        assert_eq!(
            active.awaiting_swarm.get(CARGO_LOCK).map(String::as_str),
            Some(format!("{SOURCE}#657").as_str())
        );
        assert!(
            active.two_sided.is_empty(),
            "missing_in_swarm must not grant reconciliation authority"
        );

        // Once consumed the receipt is history and grants nothing, so the
        // migration mechanism cannot become a permanent bypass.
        entries[0].consumed_by = format!("{SOURCE}#655");
        let consumed = derive_authority(&port, Path::new("."), &refs(), &entries)?;
        assert!(
            consumed.awaiting_swarm.is_empty() && consumed.two_sided.is_empty(),
            "a consumed receipt still granted authority"
        );
        Ok(())
    }

    #[test]
    fn discard_source_grants_only_the_explicit_bounded_swarm_resolution() -> Result<()> {
        let patch = section(CARGO_LOCK, "1.52.3", "1.53.0", "tokio");
        let source_oid = "9f2c1b6a0d4e5f708192a3b4c5d6e7f809a1b2c3";
        let swarm_oid = "8e1b2c3d4f5061728394a5b6c7d8e9f0a1b2c3d4";
        let decision_sha = "7d0a1b2c3d4e5f60718293a4b5c6d7e8f9a0b1c2";
        let source = format!("{SOURCE}#657");
        let port = StubPort::new()
            .merged(&source, "e6a72ad11ab22d03b1cf434b237bf0fff9c145bf", &patch)
            .merged(&format!("{SWARM}#242"), decision_sha, "")
            .blob("origin/main", CARGO_LOCK, source_oid)
            .blob("swarm/main", CARGO_LOCK, swarm_oid);
        let mut transition = entry(TransitionDisposition::MissingInSwarm, &[]);
        let path = &mut transition.path[0];
        path.resolution = Some(TransitionResolution::DiscardSource);
        path.decision_receipt = format!("{SWARM}#242");
        path.decision_merge_sha = decision_sha.to_string();
        path.reason = "Reviewed swarm state supersedes the source transition copy.".to_string();
        path.source_tree_entry = Some(blob_entry(source_oid));
        path.swarm_tree_entry = Some(blob_entry(swarm_oid));

        let authority = derive_authority(&port, Path::new("."), &refs(), &[transition.clone()])?;
        let decision = authority
            .discard_source
            .get(CARGO_LOCK)
            .context("discard_source must record its bounded decision")?;
        assert_eq!(decision.source_pr, source);
        assert_eq!(decision.decision_receipt, format!("{SWARM}#242"));
        assert_eq!(decision.decision_merge_sha, decision_sha);
        assert!(authority.awaiting_swarm.is_empty());
        assert!(authority.two_sided.is_empty());

        let unreachable_port = StubPort::new()
            .merged(&source, "e6a72ad11ab22d03b1cf434b237bf0fff9c145bf", &patch)
            .merged(&format!("{SWARM}#242"), decision_sha, "")
            .unreachable(decision_sha)
            .blob("origin/main", CARGO_LOCK, source_oid)
            .blob("swarm/main", CARGO_LOCK, swarm_oid);
        let error = derive_authority(
            &unreachable_port,
            Path::new("."),
            &refs(),
            &[transition.clone()],
        )
        .expect_err("an unreachable decision receipt must not grant authority");
        assert!(format!("{error:#}").contains("decision receipt evidence"));
        assert!(format!("{error:#}").contains("not reachable from swarm/main"));

        let mut absent_source_transition = transition.clone();
        absent_source_transition.path[0].source_tree_entry = None;
        absent_source_transition.path[0].swarm_tree_entry = Some(blob_entry(swarm_oid));
        let absent_source_port = StubPort::new()
            .merged(&source, "e6a72ad11ab22d03b1cf434b237bf0fff9c145bf", &patch)
            .merged(&format!("{SWARM}#242"), decision_sha, "")
            .absent("origin/main", CARGO_LOCK)
            .blob("swarm/main", CARGO_LOCK, swarm_oid);
        let authority = derive_authority(
            &absent_source_port,
            Path::new("."),
            &refs(),
            &[absent_source_transition],
        )?;
        assert!(authority.discard_source.contains_key(CARGO_LOCK));
        Ok(())
    }

    #[test]
    fn self_referential_manifest_may_change_only_its_current_swarm_entry() -> Result<()> {
        let path = PROMOTION_STATE_PATH;
        let patch = section(path, "old", "new", "promotion-state");
        let source_oid = "9f2c1b6a0d4e5f708192a3b4c5d6e7f809a1b2c3";
        let evidence_swarm_oid = "8e1b2c3d4f5061728394a5b6c7d8e9f0a1b2c3d4";
        let current_swarm_oid = "7d0a1b2c3d4e5f60718293a4b5c6d7e8f9a0b1c2";
        let decision_sha = "6c9d0e1f2a3b4c5d6e7f8091a2b3c4d5e6f70819";
        let source = format!("{SOURCE}#657");
        let decision = format!("{SWARM}#242");
        let mut transition = entry(TransitionDisposition::MissingInSwarm, &[]);
        transition.source_target = "origin/evidence".to_string();
        transition.swarm_target = "swarm/evidence".to_string();
        let transition_path = &mut transition.path[0];
        transition_path.path = path.to_string();
        transition_path.resolution = Some(TransitionResolution::DiscardSource);
        transition_path.self_referential = true;
        transition_path.decision_receipt = decision.clone();
        transition_path.decision_merge_sha = decision_sha.to_string();
        transition_path.reason = "The active receipt is stored in this manifest.".to_string();
        transition_path.source_tree_entry = Some(blob_entry(source_oid));
        transition_path.swarm_tree_entry = Some(blob_entry(evidence_swarm_oid));

        let port = StubPort::new()
            .ancestor("origin/evidence")
            .ancestor("swarm/evidence")
            .merged(&source, &transition.source_merge_sha, &patch)
            .merged(&decision, decision_sha, "")
            .blob("origin/evidence", path, source_oid)
            .blob("swarm/evidence", path, evidence_swarm_oid)
            .blob("origin/main", path, source_oid)
            .blob("swarm/main", path, current_swarm_oid);
        let current_refs = TransitionRefs {
            source_repo: SOURCE,
            swarm_repo: SWARM,
            source_target: "origin/main",
            swarm_target: "swarm/main",
        };

        let authority = derive_authority(&port, Path::new("."), &current_refs, &[transition])?;
        assert!(authority.discard_source.contains_key(path));
        Ok(())
    }

    #[test]
    fn converged_at_target_allows_a_later_swarm_product_change() -> Result<()> {
        let path = "xtask/src/tasks/promote.rs";
        let patch = section(path, "old", "new", "promotion");
        let converged_oid = "9f2c1b6a0d4e5f708192a3b4c5d6e7f809a1b2c3";
        let current_swarm_oid = "8e1b2c3d4f5061728394a5b6c7d8e9f0a1b2c3d4";
        let decision_sha = "7d0a1b2c3d4e5f60718293a4b5c6d7e8f9a0b1c2";
        let source = format!("{SOURCE}#657");
        let decision = format!("{SWARM}#242");
        let mut transition = entry(TransitionDisposition::ConvergedAtTarget, &[]);
        transition.source_target = "origin/evidence".to_string();
        transition.swarm_target = "swarm/evidence".to_string();
        let transition_path = &mut transition.path[0];
        transition_path.path = path.to_string();
        transition_path.resolution = Some(TransitionResolution::DiscardSource);
        transition_path.decision_receipt = decision.clone();
        transition_path.decision_merge_sha = decision_sha.to_string();
        transition_path.reason =
            "A later swarm product change supersedes the converged source copy.".to_string();
        transition_path.source_tree_entry = Some(blob_entry(converged_oid));
        transition_path.swarm_tree_entry = Some(blob_entry(converged_oid));

        let port = StubPort::new()
            .ancestor("origin/evidence")
            .ancestor("swarm/evidence")
            .merged(&source, &transition.source_merge_sha, &patch)
            .merged(&decision, decision_sha, "")
            .blob("origin/evidence", path, converged_oid)
            .blob("swarm/evidence", path, converged_oid)
            .blob("origin/main", path, converged_oid)
            .blob("swarm/main", path, current_swarm_oid);
        let current_refs = TransitionRefs {
            source_repo: SOURCE,
            swarm_repo: SWARM,
            source_target: "origin/main",
            swarm_target: "swarm/main",
        };

        let authority = derive_authority(&port, Path::new("."), &current_refs, &[transition])?;
        assert!(authority.discard_source.contains_key(path));
        Ok(())
    }

    fn source_authority_decision(
        path: &str,
        source_oid: &str,
        swarm_oid: &str,
        decision_sha: &str,
    ) -> SourceAuthorityDecision {
        SourceAuthorityDecision {
            path: path.to_string(),
            source_target: "origin/main".to_string(),
            swarm_target: "swarm/main".to_string(),
            decision_receipt: format!("{SWARM}#319"),
            decision_merge_sha: decision_sha.to_string(),
            reason: "Reviewed source release workflow remains authoritative.".to_string(),
            consumed_by: String::new(),
            source_tree_entry: Some(blob_entry(source_oid)),
            swarm_tree_entry: Some(blob_entry(swarm_oid)),
        }
    }

    #[test]
    fn source_authority_requires_exact_policy_targets_entries_and_receipt() -> Result<()> {
        let path = "governance.yml";
        let source_oid = "9f2c1b6a0d4e5f708192a3b4c5d6e7f809a1b2c3";
        let swarm_oid = "8e1b2c3d4f5061728394a5b6c7d8e9f0a1b2c3d4";
        let decision_sha = "7d0a1b2c3d4e5f60718293a4b5c6d7e8f9a0b1c2";
        let decision = source_authority_decision(path, source_oid, swarm_oid, decision_sha);
        let port = StubPort::new()
            .merged(&decision.decision_receipt, decision_sha, "")
            .blob("origin/main", path, source_oid)
            .blob("swarm/main", path, swarm_oid);

        let authority = derive_source_authority(
            &port,
            Path::new("."),
            &refs(),
            &[path.to_string()],
            std::slice::from_ref(&decision),
        )?;
        assert_eq!(
            authority.get(path).map(|value| value.reason.as_str()),
            Some(decision.reason.as_str())
        );
        Ok(())
    }

    #[test]
    fn consumed_source_authority_grants_nothing() -> Result<()> {
        let path = "governance.yml";
        let mut decision = source_authority_decision(
            path,
            "9f2c1b6a0d4e5f708192a3b4c5d6e7f809a1b2c3",
            "8e1b2c3d4f5061728394a5b6c7d8e9f0a1b2c3d4",
            "7d0a1b2c3d4e5f60718293a4b5c6d7e8f9a0b1c2",
        );
        decision.consumed_by = format!("{SOURCE}#655");

        let authority = derive_source_authority(
            &StubPort::new(),
            Path::new("."),
            &refs(),
            &[path.to_string()],
            &[decision],
        )?;
        assert!(authority.is_empty());
        Ok(())
    }

    #[test]
    fn source_authority_rejects_an_unlisted_path() {
        let decision = source_authority_decision(
            "release.yml",
            "9f2c1b6a0d4e5f708192a3b4c5d6e7f809a1b2c3",
            "8e1b2c3d4f5061728394a5b6c7d8e9f0a1b2c3d4",
            "7d0a1b2c3d4e5f60718293a4b5c6d7e8f9a0b1c2",
        );
        let error = derive_source_authority(
            &StubPort::new(),
            Path::new("."),
            &refs(),
            &["governance.yml".to_string()],
            &[decision],
        )
        .expect_err("source authority must not expand the policy allowlist");
        assert!(
            error
                .to_string()
                .contains("not listed in policy/source-only-paths.toml")
        );
    }

    #[test]
    fn source_authority_rejects_equal_or_stale_tree_entries() {
        let path = "governance.yml";
        let source_oid = "9f2c1b6a0d4e5f708192a3b4c5d6e7f809a1b2c3";
        let swarm_oid = "8e1b2c3d4f5061728394a5b6c7d8e9f0a1b2c3d4";
        let decision_sha = "7d0a1b2c3d4e5f60718293a4b5c6d7e8f9a0b1c2";
        let mut decision = source_authority_decision(path, source_oid, swarm_oid, decision_sha);
        decision.source_target = "source-evidence".to_string();
        decision.swarm_target = "swarm-evidence".to_string();

        let equal_port = StubPort::new()
            .merged(&decision.decision_receipt, decision_sha, "")
            .ancestor("source-evidence")
            .ancestor("swarm-evidence")
            .blob("source-evidence", path, source_oid)
            .blob("swarm-evidence", path, swarm_oid)
            .blob("origin/main", path, source_oid)
            .blob("swarm/main", path, source_oid);
        let error = derive_source_authority(
            &equal_port,
            Path::new("."),
            &refs(),
            &[path.to_string()],
            std::slice::from_ref(&decision),
        )
        .expect_err("equal trees must not create a source-authority decision");
        assert!(
            error
                .to_string()
                .contains("requires differing source and swarm")
        );

        let stale_port = StubPort::new()
            .merged(&decision.decision_receipt, decision_sha, "")
            .ancestor("source-evidence")
            .ancestor("swarm-evidence")
            .blob("source-evidence", path, source_oid)
            .blob("swarm-evidence", path, swarm_oid)
            .blob(
                "origin/main",
                path,
                "1111111111111111111111111111111111111111",
            )
            .blob("swarm/main", path, swarm_oid);
        let error = derive_source_authority(
            &stale_port,
            Path::new("."),
            &refs(),
            &[path.to_string()],
            &[decision],
        )
        .expect_err("stale tree bindings must not grant source authority");
        assert!(
            error
                .to_string()
                .contains("recorded source tree entry does not match")
        );
    }

    #[test]
    fn source_authority_rejects_an_unreachable_decision_receipt() {
        let path = "governance.yml";
        let source_oid = "9f2c1b6a0d4e5f708192a3b4c5d6e7f809a1b2c3";
        let swarm_oid = "8e1b2c3d4f5061728394a5b6c7d8e9f0a1b2c3d4";
        let decision_sha = "7d0a1b2c3d4e5f60718293a4b5c6d7e8f9a0b1c2";
        let decision = source_authority_decision(path, source_oid, swarm_oid, decision_sha);
        let port = StubPort::new()
            .merged(&decision.decision_receipt, decision_sha, "")
            .unreachable(decision_sha)
            .blob("origin/main", path, source_oid)
            .blob("swarm/main", path, swarm_oid);
        let error = derive_source_authority(
            &port,
            Path::new("."),
            &refs(),
            &[path.to_string()],
            &[decision],
        )
        .expect_err("an unreachable decision receipt must not grant authority");
        assert!(format!("{error:#}").contains("decision receipt evidence"));
        assert!(format!("{error:#}").contains("not reachable from swarm/main"));
    }

    /// A newer missing-in-swarm receipt must supersede an older resolved
    /// receipt for the same path rather than leaving stale two-sided authority.
    #[test]
    fn missing_in_swarm_supersedes_stale_resolved_receipt() -> Result<()> {
        let patch = section(CARGO_LOCK, "1.52.3", "1.53.0", "tokio");
        let source = format!("{SOURCE}#657");
        let swarm = format!("{SWARM}#269");
        let port = StubPort::new()
            .merged(&source, "e6a72ad11ab22d03b1cf434b237bf0fff9c145bf", &patch)
            .merged(&swarm, "ba4aeaf78cb17f980f1da05d24d9638033b95f68", &patch)
            .blob(
                "origin/main",
                CARGO_LOCK,
                "9f2c1b6a0d4e5f708192a3b4c5d6e7f809a1b2c3",
            )
            .blob(
                "swarm/main",
                CARGO_LOCK,
                "9f2c1b6a0d4e5f708192a3b4c5d6e7f809a1b2c3",
            );
        let entries = vec![
            bind_entries(
                entry(
                    TransitionDisposition::Equivalent,
                    &[(swarm.as_str(), "ba4aeaf78cb17f980f1da05d24d9638033b95f68")],
                ),
                Some(blob_entry("9f2c1b6a0d4e5f708192a3b4c5d6e7f809a1b2c3")),
                Some(blob_entry("9f2c1b6a0d4e5f708192a3b4c5d6e7f809a1b2c3")),
            ),
            entry(TransitionDisposition::MissingInSwarm, &[]),
        ];

        let authority = derive_authority(&port, Path::new("."), &refs(), &entries)?;

        assert!(authority.two_sided.is_empty());
        assert_eq!(
            authority.awaiting_swarm.get(CARGO_LOCK).map(String::as_str),
            Some(source.as_str())
        );
        Ok(())
    }

    /// Recording a SHA is not evidence. An unmerged PR must be rejected even
    /// though the receipt is well formed.
    #[test]
    fn unmerged_source_pr_is_rejected() {
        let patch = section(CARGO_LOCK, "1.52.3", "1.53.0", "tokio");
        let port = StubPort::new().open(&format!("{SOURCE}#657"), &patch);
        let entries = vec![entry(TransitionDisposition::MissingInSwarm, &[])];
        let error = derive_authority(&port, Path::new("."), &refs(), &entries)
            .expect_err("an unmerged PR must not supply evidence");
        assert!(error.to_string().contains("source merge evidence"));
        assert!(
            format!("{error:#}").contains("not MERGED"),
            "unexpected error: {error:#}"
        );
    }

    /// A well-formed SHA that is not the PR's merge commit must be rejected.
    #[test]
    fn wrong_recorded_merge_sha_is_rejected() {
        let patch = section(CARGO_LOCK, "1.52.3", "1.53.0", "tokio");
        let port = StubPort::new().merged(
            &format!("{SOURCE}#657"),
            "1111111111111111111111111111111111111111",
            &patch,
        );
        let entries = vec![entry(TransitionDisposition::MissingInSwarm, &[])];
        let error = derive_authority(&port, Path::new("."), &refs(), &entries)
            .expect_err("a mismatched merge sha must be rejected");
        assert!(
            format!("{error:#}").contains("but the receipt records"),
            "unexpected error: {error:#}"
        );
    }

    /// The merge must still be part of the promoted line, not something later
    /// rewritten away, which only local ancestry can establish.
    #[test]
    fn unreachable_merge_sha_is_rejected() {
        let patch = section(CARGO_LOCK, "1.52.3", "1.53.0", "tokio");
        let sha = "e6a72ad11ab22d03b1cf434b237bf0fff9c145bf";
        let port = StubPort::new()
            .merged(&format!("{SOURCE}#657"), sha, &patch)
            .unreachable(sha);
        let entries = vec![entry(TransitionDisposition::MissingInSwarm, &[])];
        let error = derive_authority(&port, Path::new("."), &refs(), &entries)
            .expect_err("an unreachable merge must be rejected");
        assert!(
            format!("{error:#}").contains("not reachable from origin/main"),
            "unexpected error: {error:#}"
        );
    }

    /// Equivalence is decided by `git patch-id --stable` on the path's own
    /// section, so the same edit to a different file is a different identity.
    #[test]
    fn equivalent_requires_matching_patch_id_for_that_path() -> Result<()> {
        let source_patch = section(CARGO_LOCK, "1.52.3", "1.53.0", "tokio");
        let divergent = section(CARGO_LOCK, "1.52.3", "9.9.9", "tokio");
        let swarm = format!("{SWARM}#269");
        let swarm_sha = "ba4aeaf78cb17f980f1da05d24d9638033b95f68";
        let port = StubPort::new()
            .merged(
                &format!("{SOURCE}#657"),
                "e6a72ad11ab22d03b1cf434b237bf0fff9c145bf",
                &source_patch,
            )
            .merged(&swarm, swarm_sha, &divergent);
        let entries = vec![entry(
            TransitionDisposition::Equivalent,
            &[(swarm.as_str(), swarm_sha)],
        )];
        let error = derive_authority(&port, Path::new("."), &refs(), &entries)
            .expect_err("differing patches must not be accepted as equivalent");
        assert!(
            format!("{error:#}").contains("patch ids differ"),
            "unexpected error: {error:#}"
        );

        // The same content under a different file name is a different change.
        let elsewhere = section("other/Cargo.lock", "1.52.3", "1.53.0", "tokio");
        assert_ne!(
            system_patch_id(&source_patch)?,
            system_patch_id(&elsewhere)?,
            "patch identity must include the file it applies to"
        );
        Ok(())
    }

    /// Dependency bumps can have different raw patch identities when the two
    /// lockfiles carry different surrounding package resolutions. The exact
    /// package-version transition remains the narrower evidence claim.
    #[test]
    fn dependency_equivalent_accepts_matching_lock_version_transitions() -> Result<()> {
        let source_patch = section(CARGO_LOCK, "1.52.3", "1.53.0", "tokio");
        let swarm_patch = source_patch.replace("[[package]]", "[[package]]\n# swarm context");
        let swarm = format!("{SWARM}#269");
        let swarm_sha = "ba4aeaf78cb17f980f1da05d24d9638033b95f68";
        let port = StubPort::new()
            .merged(
                &format!("{SOURCE}#657"),
                "e6a72ad11ab22d03b1cf434b237bf0fff9c145bf",
                &source_patch,
            )
            .merged(&swarm, swarm_sha, &swarm_patch)
            .blob(
                "origin/main",
                CARGO_LOCK,
                "9f2c1b6a0d4e5f708192a3b4c5d6e7f809a1b2c3",
            )
            .blob(
                "swarm/main",
                CARGO_LOCK,
                "9f2c1b6a0d4e5f708192a3b4c5d6e7f809a1b2c3",
            );
        let entries = vec![bind_entries(
            entry(
                TransitionDisposition::DependencyEquivalent,
                &[(swarm.as_str(), swarm_sha)],
            ),
            Some(blob_entry("9f2c1b6a0d4e5f708192a3b4c5d6e7f809a1b2c3")),
            Some(blob_entry("9f2c1b6a0d4e5f708192a3b4c5d6e7f809a1b2c3")),
        )];

        let authority = derive_authority(&port, Path::new("."), &refs(), &entries)?;
        assert_eq!(
            authority
                .two_sided
                .get(CARGO_LOCK)
                .context("dependency equivalence must grant two-sided authority")?
                .disposition,
            TransitionDisposition::DependencyEquivalent
        );
        Ok(())
    }

    /// The real converged case: the two repositories started from different
    /// content and landed identical content by different patches. Patch identity
    /// says no, and it is right to; the resulting blobs are what agree.
    #[test]
    fn tree_equivalent_accepts_identical_blobs_from_differing_patches() -> Result<()> {
        let source_patch = section(CARGO_LOCK, "1.52.3", "1.53.0", "tokio");
        // Same destination, different starting point, so a different patch id.
        let swarm_patch = section(CARGO_LOCK, "1.50.0", "1.53.0", "tokio");
        assert_ne!(
            system_patch_id(&source_patch)?,
            system_patch_id(&swarm_patch)?,
            "the fixture must not accidentally share a patch id"
        );
        let converged = "9f2c1b6a0d4e5f708192a3b4c5d6e7f809a1b2c3";
        let swarm = format!("{SWARM}#269");
        let swarm_sha = "ba4aeaf78cb17f980f1da05d24d9638033b95f68";
        let port = StubPort::new()
            .merged(
                &format!("{SOURCE}#657"),
                "e6a72ad11ab22d03b1cf434b237bf0fff9c145bf",
                &source_patch,
            )
            .merged(&swarm, swarm_sha, &swarm_patch)
            .blob("origin/main", CARGO_LOCK, converged)
            .blob("swarm/main", CARGO_LOCK, converged);
        let entries = vec![bind_entries(
            entry(
                TransitionDisposition::TreeEquivalent,
                &[(swarm.as_str(), swarm_sha)],
            ),
            Some(blob_entry(converged)),
            Some(blob_entry(converged)),
        )];

        let authority = derive_authority(&port, Path::new("."), &refs(), &entries)?;

        let receipt = authority
            .two_sided
            .get(CARGO_LOCK)
            .context("tree_equivalent must grant two-sided authority")?;
        assert_eq!(
            receipt.disposition,
            TransitionDisposition::TreeEquivalent,
            "the grant must record the disposition that earned it"
        );
        assert!(authority.awaiting_swarm.is_empty());
        Ok(())
    }

    #[test]
    fn active_receipt_rejects_stale_recorded_tree_entry() {
        let patch = section(CARGO_LOCK, "1.52.3", "1.53.0", "tokio");
        let swarm = format!("{SWARM}#269");
        let swarm_sha = "ba4aeaf78cb17f980f1da05d24d9638033b95f68";
        let current = "9f2c1b6a0d4e5f708192a3b4c5d6e7f809a1b2c3";
        let port = StubPort::new()
            .merged(
                &format!("{SOURCE}#657"),
                "e6a72ad11ab22d03b1cf434b237bf0fff9c145bf",
                &patch,
            )
            .merged(&swarm, swarm_sha, &patch)
            .blob("origin/main", CARGO_LOCK, current)
            .blob("swarm/main", CARGO_LOCK, current);
        let entries = vec![bind_entries(
            entry(
                TransitionDisposition::Equivalent,
                &[(swarm.as_str(), swarm_sha)],
            ),
            Some(TreeEntry {
                mode: "100755".to_string(),
                object_type: "blob".to_string(),
                oid: current.to_string(),
            }),
            Some(blob_entry(current)),
        )];

        let error = derive_authority(&port, Path::new("."), &refs(), &entries)
            .expect_err("an active receipt must not reuse stale path metadata");
        assert!(
            format!("{error:#}").contains("recorded source tree entry does not match"),
            "unexpected error: {error:#}"
        );
    }

    #[test]
    fn active_receipt_cannot_be_reused_for_different_promotion_targets() {
        let mut entries = vec![entry(TransitionDisposition::MissingInSwarm, &[])];
        entries[0].source_target = "source-old-target".to_string();
        let error = derive_authority(&StubPort::new(), Path::new("."), &refs(), &entries)
            .expect_err("a receipt bound to another target must fail closed");
        assert!(
            format!("{error:#}").contains("not an ancestor of current promotion target"),
            "unexpected error: {error:#}"
        );
    }

    #[test]
    fn active_receipt_allows_unrelated_tip_when_recorded_entries_are_unchanged() -> Result<()> {
        let source_patch = section(CARGO_LOCK, "1.52.3", "1.53.0", "tokio");
        let swarm = format!("{SWARM}#269");
        let swarm_sha = "ba4aeaf78cb17f980f1da05d24d9638033b95f68";
        let converged = "9f2c1b6a0d4e5f708192a3b4c5d6e7f809a1b2c3";
        let source = format!("{SOURCE}#657");
        let mut transition = bind_entries(
            entry(
                TransitionDisposition::TreeEquivalent,
                &[(swarm.as_str(), swarm_sha)],
            ),
            Some(blob_entry(converged)),
            Some(blob_entry(converged)),
        );
        transition.source_target = "source-evidence".to_string();
        transition.swarm_target = "swarm-evidence".to_string();

        let port = StubPort::new()
            .merged(
                &source,
                "e6a72ad11ab22d03b1cf434b237bf0fff9c145bf",
                &source_patch,
            )
            .merged(&swarm, swarm_sha, &source_patch)
            .ancestor("source-evidence")
            .ancestor("swarm-evidence")
            .blob("source-evidence", CARGO_LOCK, converged)
            .blob("swarm-evidence", CARGO_LOCK, converged)
            // The current tips advanced for unrelated paths, but this path's
            // complete tree entries remain exactly the recorded evidence.
            .blob("origin/main", CARGO_LOCK, converged)
            .blob("swarm/main", CARGO_LOCK, converged);

        let authority = derive_authority(&port, Path::new("."), &refs(), &[transition])?;
        assert!(authority.two_sided.contains_key(CARGO_LOCK));
        Ok(())
    }

    #[test]
    fn active_receipt_rejects_governed_path_change_after_evidence() {
        let source_patch = section(CARGO_LOCK, "1.52.3", "1.53.0", "tokio");
        let swarm = format!("{SWARM}#269");
        let swarm_sha = "ba4aeaf78cb17f980f1da05d24d9638033b95f68";
        let recorded = "9f2c1b6a0d4e5f708192a3b4c5d6e7f809a1b2c3";
        let changed = "1111111111111111111111111111111111111111";
        let source = format!("{SOURCE}#657");
        let mut transition = bind_entries(
            entry(
                TransitionDisposition::Equivalent,
                &[(swarm.as_str(), swarm_sha)],
            ),
            Some(blob_entry(recorded)),
            Some(blob_entry(recorded)),
        );
        transition.source_target = "source-evidence".to_string();
        transition.swarm_target = "swarm-evidence".to_string();

        let port = StubPort::new()
            .merged(
                &source,
                "e6a72ad11ab22d03b1cf434b237bf0fff9c145bf",
                &source_patch,
            )
            .merged(&swarm, swarm_sha, &source_patch)
            .ancestor("source-evidence")
            .ancestor("swarm-evidence")
            .blob("source-evidence", CARGO_LOCK, recorded)
            .blob("swarm-evidence", CARGO_LOCK, recorded)
            .blob("origin/main", CARGO_LOCK, changed)
            .blob("swarm/main", CARGO_LOCK, recorded);

        let error = derive_authority(&port, Path::new("."), &refs(), &[transition])
            .expect_err("a later governed-path change must invalidate old evidence");
        assert!(
            format!("{error:#}").contains("recorded source tree entry does not match"),
            "unexpected error: {error:#}"
        );
    }

    /// `tree_equivalent` is a claim about the outcome, so differing outcomes
    /// must be refused even though both sides touched the path.
    #[test]
    fn tree_equivalent_rejects_differing_blobs() {
        let source_patch = section(CARGO_LOCK, "1.52.3", "1.53.0", "tokio");
        let swarm_patch = section(CARGO_LOCK, "1.52.3", "9.9.9", "tokio");
        let swarm = format!("{SWARM}#269");
        let swarm_sha = "ba4aeaf78cb17f980f1da05d24d9638033b95f68";
        let port = StubPort::new()
            .merged(
                &format!("{SOURCE}#657"),
                "e6a72ad11ab22d03b1cf434b237bf0fff9c145bf",
                &source_patch,
            )
            .merged(&swarm, swarm_sha, &swarm_patch)
            .blob(
                "origin/main",
                CARGO_LOCK,
                "1111111111111111111111111111111111111111",
            )
            .blob(
                "swarm/main",
                CARGO_LOCK,
                "2222222222222222222222222222222222222222",
            );
        let entries = vec![entry(
            TransitionDisposition::TreeEquivalent,
            &[(swarm.as_str(), swarm_sha)],
        )];
        let error = derive_authority(&port, Path::new("."), &refs(), &entries)
            .expect_err("differing resulting blobs must not be accepted as tree_equivalent");
        assert!(
            format!("{error:#}").contains("the resulting tree entries differ"),
            "unexpected error: {error:#}"
        );
    }

    #[test]
    fn tree_equivalent_rejects_matching_oid_with_different_mode_or_type() -> Result<()> {
        let source_patch = section(CARGO_LOCK, "1.52.3", "1.53.0", "tokio");
        let swarm = format!("{SWARM}#269");
        let swarm_sha = "ba4aeaf78cb17f980f1da05d24d9638033b95f68";
        let oid = "9f2c1b6a0d4e5f708192a3b4c5d6e7f809a1b2c3";
        let source = format!("{SOURCE}#657");

        for (source_mode, source_type, swarm_mode, swarm_type) in [
            ("100644", "blob", "100755", "blob"),
            ("100644", "blob", "160000", "commit"),
        ] {
            let port = StubPort::new()
                .merged(
                    &source,
                    "e6a72ad11ab22d03b1cf434b237bf0fff9c145bf",
                    &source_patch,
                )
                .merged(&swarm, swarm_sha, &source_patch)
                .tree("origin/main", CARGO_LOCK, source_mode, source_type, oid)
                .tree("swarm/main", CARGO_LOCK, swarm_mode, swarm_type, oid);
            let entries = vec![entry(
                TransitionDisposition::TreeEquivalent,
                &[(swarm.as_str(), swarm_sha)],
            )];
            let error = derive_authority(&port, Path::new("."), &refs(), &entries)
                .expect_err("tree entry metadata differences must not be accepted");
            assert!(
                format!("{error:#}").contains("the resulting tree entries differ"),
                "unexpected error: {error:#}"
            );
        }
        Ok(())
    }

    #[test]
    fn tree_equivalent_treats_matching_absence_as_equal() -> Result<()> {
        let source_patch = section(CARGO_LOCK, "1.52.3", "1.53.0", "tokio");
        let swarm = format!("{SWARM}#269");
        let swarm_sha = "ba4aeaf78cb17f980f1da05d24d9638033b95f68";
        let source = format!("{SOURCE}#657");
        let entries = vec![entry(
            TransitionDisposition::TreeEquivalent,
            &[(swarm.as_str(), swarm_sha)],
        )];
        let both_absent = StubPort::new()
            .merged(
                &source,
                "e6a72ad11ab22d03b1cf434b237bf0fff9c145bf",
                &source_patch,
            )
            .merged(&swarm, swarm_sha, &source_patch)
            .absent("origin/main", CARGO_LOCK)
            .absent("swarm/main", CARGO_LOCK);
        derive_authority(&both_absent, Path::new("."), &refs(), &entries)?;

        let one_present = StubPort::new()
            .merged(
                &source,
                "e6a72ad11ab22d03b1cf434b237bf0fff9c145bf",
                &source_patch,
            )
            .merged(&swarm, swarm_sha, &source_patch)
            .absent("origin/main", CARGO_LOCK)
            .blob(
                "swarm/main",
                CARGO_LOCK,
                "9f2c1b6a0d4e5f708192a3b4c5d6e7f809a1b2c3",
            );
        let error = derive_authority(&one_present, Path::new("."), &refs(), &entries)
            .expect_err("one absent and one present path must not be equivalent");
        assert!(
            format!("{error:#}").contains("the resulting tree entries differ"),
            "unexpected error: {error:#}"
        );
        Ok(())
    }

    #[test]
    fn tree_equivalent_binds_evidence_to_historical_targets() -> Result<()> {
        let source_patch = section(CARGO_LOCK, "1.52.3", "1.53.0", "tokio");
        let swarm = format!("{SWARM}#269");
        let swarm_sha = "ba4aeaf78cb17f980f1da05d24d9638033b95f68";
        let source = format!("{SOURCE}#657");
        let old_refs = TransitionRefs {
            source_repo: SOURCE,
            swarm_repo: SWARM,
            source_target: "source-old-target",
            swarm_target: "swarm-old-target",
        };
        let port = StubPort::new()
            .merged(
                &source,
                "e6a72ad11ab22d03b1cf434b237bf0fff9c145bf",
                &source_patch,
            )
            .merged(&swarm, swarm_sha, &source_patch)
            // The branch tips later converged, but the proposed historical
            // promotion targets still disagree and must be rejected.
            .blob(
                "source-old-target",
                CARGO_LOCK,
                "1111111111111111111111111111111111111111",
            )
            .blob(
                "swarm-old-target",
                CARGO_LOCK,
                "2222222222222222222222222222222222222222",
            )
            .blob(
                "source/main",
                CARGO_LOCK,
                "9f2c1b6a0d4e5f708192a3b4c5d6e7f809a1b2c3",
            )
            .blob(
                "swarm/main",
                CARGO_LOCK,
                "9f2c1b6a0d4e5f708192a3b4c5d6e7f809a1b2c3",
            );
        let entries = vec![entry(
            TransitionDisposition::TreeEquivalent,
            &[(swarm.as_str(), swarm_sha)],
        )];
        let mut entries = entries;
        entries[0].source_target = old_refs.source_target.to_string();
        entries[0].swarm_target = old_refs.swarm_target.to_string();

        let error = derive_authority(&port, Path::new("."), &old_refs, &entries)
            .expect_err("historical target disagreement must not use later branch convergence");
        assert!(
            format!("{error:#}").contains("the resulting tree entries differ"),
            "unexpected error: {error:#}"
        );
        Ok(())
    }

    /// Adding `tree_equivalent` must not soften `equivalent`. `equivalent` still
    /// decides on patch identity alone: identical resulting blobs do not rescue
    /// differing patches, and it never consults blob evidence at all.
    #[test]
    fn equivalent_behaviour_is_unchanged_by_tree_equivalent() -> Result<()> {
        let source_patch = section(CARGO_LOCK, "1.52.3", "1.53.0", "tokio");
        let swarm_patch = section(CARGO_LOCK, "1.50.0", "1.53.0", "tokio");
        let converged = "9f2c1b6a0d4e5f708192a3b4c5d6e7f809a1b2c3";
        let swarm = format!("{SWARM}#269");
        let swarm_sha = "ba4aeaf78cb17f980f1da05d24d9638033b95f68";

        // Identical resulting blobs, different patches: still rejected.
        let port = StubPort::new()
            .merged(
                &format!("{SOURCE}#657"),
                "e6a72ad11ab22d03b1cf434b237bf0fff9c145bf",
                &source_patch,
            )
            .merged(&swarm, swarm_sha, &swarm_patch)
            .blob("origin/main", CARGO_LOCK, converged)
            .blob("swarm/main", CARGO_LOCK, converged);
        let entries = vec![bind_entries(
            entry(
                TransitionDisposition::Equivalent,
                &[(swarm.as_str(), swarm_sha)],
            ),
            Some(blob_entry(converged)),
            Some(blob_entry(converged)),
        )];
        let error = derive_authority(&port, Path::new("."), &refs(), &entries)
            .expect_err("equivalent must still require matching patch ids");
        assert!(
            format!("{error:#}").contains("patch ids differ"),
            "unexpected error: {error:#}"
        );

        // Matching patches: still accepted. The disposition-specific
        // equivalence check ignores blob evidence, while the universal
        // recorded-tree check now requires matching current and recorded
        // entries for every active resolved path.
        let matching = StubPort::new()
            .merged(
                &format!("{SOURCE}#657"),
                "e6a72ad11ab22d03b1cf434b237bf0fff9c145bf",
                &source_patch,
            )
            .merged(&swarm, swarm_sha, &source_patch)
            .blob("origin/main", CARGO_LOCK, converged)
            .blob("swarm/main", CARGO_LOCK, converged);
        let authority = derive_authority(&matching, Path::new("."), &refs(), &entries)?;
        let receipt = authority
            .two_sided
            .get(CARGO_LOCK)
            .context("equivalent must still grant two-sided authority")?;
        assert_eq!(receipt.disposition, TransitionDisposition::Equivalent);
        Ok(())
    }

    /// A receipt may only speak for paths its own source PR touched. Granting
    /// `missing_in_swarm` before checking that let one merged PR authorize
    /// divergence on an unrelated file it never changed.
    #[test]
    fn missing_in_swarm_requires_the_source_pr_to_touch_the_path() {
        let patch = section(CARGO_LOCK, "1.52.3", "1.53.0", "tokio");
        let port = StubPort::new().merged(
            &format!("{SOURCE}#657"),
            "e6a72ad11ab22d03b1cf434b237bf0fff9c145bf",
            &patch,
        );
        let mut entries = vec![entry(TransitionDisposition::MissingInSwarm, &[])];
        entries[0].path[0].path = "some-unrelated-source-file".to_string();
        let error = derive_authority(&port, Path::new("."), &refs(), &entries)
            .expect_err("a path the source PR never touched must not be authorized");
        assert!(
            format!("{error:#}").contains("does not touch this path"),
            "unexpected error: {error:#}"
        );
    }

    /// A receipt naming a different repository must not be answered from the
    /// expected repository's PR of the same number.
    #[test]
    fn receipt_repository_identity_is_enforced() {
        assert!(receipt_number(&format!("{SOURCE}#657"), SOURCE).is_ok());
        let error = receipt_number("OtherOrg/OtherRepo#657", SOURCE)
            .expect_err("a foreign repository receipt must be rejected");
        assert!(
            error.to_string().contains("names repository"),
            "unexpected error: {error}"
        );
    }

    /// Starting at the right version is not enough. A swarm step from the same
    /// old version to an unrelated new one advanced the walk while never
    /// incorporating what source landed.
    #[test]
    fn supersession_rejects_a_divergent_step_from_the_source_start() {
        let source_patch = section(CARGO_LOCK, "1.52.3", "1.53.0", "tokio");
        // Same starting version, unrelated destination: contiguous, but not a
        // continuation of the source change.
        let divergent = section(CARGO_LOCK, "1.52.3", "9.9.9", "tokio");
        let swarm = format!("{SWARM}#269");
        let swarm_sha = "ba4aeaf78cb17f980f1da05d24d9638033b95f68";
        let port = StubPort::new()
            .merged(
                &format!("{SOURCE}#657"),
                "e6a72ad11ab22d03b1cf434b237bf0fff9c145bf",
                &source_patch,
            )
            .merged(&swarm, swarm_sha, &divergent);
        let entries = vec![entry(
            TransitionDisposition::SupersededInSwarm,
            &[(swarm.as_str(), swarm_sha)],
        )];
        let error = derive_authority(&port, Path::new("."), &refs(), &entries)
            .expect_err("a divergent first step must not prove supersession");
        assert!(
            format!("{error:#}").contains("does not start by reproducing the source change"),
            "unexpected error: {error:#}"
        );
    }

    /// The Tokio chain the manifest actually needs: swarm reproduces the source
    /// bump, then continues past it.
    #[test]
    fn supersession_accepts_a_chain_that_reproduces_then_advances() -> Result<()> {
        let source_patch = section(CARGO_LOCK, "1.52.3", "1.53.0", "tokio");
        let reproduce = section(CARGO_LOCK, "1.52.3", "1.53.0", "tokio");
        let advance = section(CARGO_LOCK, "1.53.0", "1.53.1", "tokio");
        let first = format!("{SWARM}#269");
        let second = format!("{SWARM}#265");
        let first_sha = "ba4aeaf78cb17f980f1da05d24d9638033b95f68";
        let second_sha = "97239104cb2923e389749ac733755a955e4c6cc5";
        let port = StubPort::new()
            .merged(
                &format!("{SOURCE}#657"),
                "e6a72ad11ab22d03b1cf434b237bf0fff9c145bf",
                &source_patch,
            )
            .merged(&first, first_sha, &reproduce)
            .merged(&second, second_sha, &advance)
            .blob(
                "origin/main",
                CARGO_LOCK,
                "9f2c1b6a0d4e5f708192a3b4c5d6e7f809a1b2c3",
            )
            .blob(
                "swarm/main",
                CARGO_LOCK,
                "9f2c1b6a0d4e5f708192a3b4c5d6e7f809a1b2c3",
            );
        let entries = vec![bind_entries(
            entry(
                TransitionDisposition::SupersededInSwarm,
                &[(first.as_str(), first_sha), (second.as_str(), second_sha)],
            ),
            Some(blob_entry("9f2c1b6a0d4e5f708192a3b4c5d6e7f809a1b2c3")),
            Some(blob_entry("9f2c1b6a0d4e5f708192a3b4c5d6e7f809a1b2c3")),
        )];

        let authority = derive_authority(&port, Path::new("."), &refs(), &entries)?;
        assert!(authority.two_sided.contains_key(CARGO_LOCK));
        assert!(authority.awaiting_swarm.is_empty());
        Ok(())
    }

    /// `conflicting` blocks rather than granting anything.
    #[test]
    fn conflicting_disposition_blocks() {
        let patch = section(CARGO_LOCK, "1.52.3", "1.53.0", "tokio");
        let port = StubPort::new().merged(
            &format!("{SOURCE}#657"),
            "e6a72ad11ab22d03b1cf434b237bf0fff9c145bf",
            &patch,
        );
        let entries = vec![entry(TransitionDisposition::Conflicting, &[])];
        let error = derive_authority(&port, Path::new("."), &refs(), &entries)
            .expect_err("conflicting must block");
        assert!(
            format!("{error:#}").contains("resolve it before promoting"),
            "unexpected error: {error:#}"
        );
    }

    fn section(path: &str, from: &str, to: &str, package: &str) -> String {
        format!(
            "diff --git a/{path} b/{path}\nindex 111..222 100644\n--- a/{path}\n+++ b/{path}\n@@ -1,4 +1,4 @@\n [[package]]\n-name = \"{package}\"\n-version = \"{from}\"\n+name = \"{package}\"\n+version = \"{to}\"\n"
        )
    }

    #[test]
    fn patch_for_path_selects_only_the_requested_file() {
        let patch = format!(
            "{}{}",
            section("Cargo.lock", "1.0.0", "1.1.0", "tokio"),
            section("other/Cargo.lock", "9.0.0", "9.1.0", "tokio")
        );
        let selected = patch_for_path(&patch, CARGO_LOCK).expect("section");
        assert!(selected.contains("diff --git a/Cargo.lock b/Cargo.lock"));
        assert!(!selected.contains("other/Cargo.lock"));
        assert!(selected.contains("1.1.0"));
        assert!(!selected.contains("9.1.0"));
        assert!(patch_for_path(&patch, "missing.txt").is_none());
    }

    #[test]
    fn lock_transitions_reads_from_and_to_versions() {
        let parsed = lock_transitions(&section(CARGO_LOCK, "1.52.3", "1.53.0", "tokio"));
        let (from, to) = parsed.get("tokio").expect("tokio");
        assert_eq!(from.as_deref(), Some("1.52.3"));
        assert_eq!(to.as_deref(), Some("1.53.0"));
    }

    #[test]
    fn keyed_value_rejects_malformed_input_without_panicking() {
        for malformed in ["name = \"", "name = \"\"", "name =", "name", "name = 1"] {
            assert_eq!(keyed_value(malformed, "name"), None, "{malformed:?}");
        }
        assert_eq!(
            keyed_value("name = \"tokio\"", "name").as_deref(),
            Some("tokio")
        );
    }

    /// The chain must start where the source change started. Mapping a source
    /// bump straight onto a later swarm bump skips the step that connects them.
    #[test]
    fn supersession_requires_a_contiguous_chain_from_the_source_start() {
        let source = section(CARGO_LOCK, "1.52.3", "1.53.0", "tokio");
        let later_only = vec![section(CARGO_LOCK, "1.53.0", "1.53.1", "tokio")];
        let error = check_supersession_chain(&source, &later_only, CARGO_LOCK)
            .expect_err("a chain skipping the connecting step must be rejected");
        assert!(
            error.to_string().contains("not contiguous"),
            "unexpected error: {error}"
        );

        let full_chain = vec![
            section(CARGO_LOCK, "1.52.3", "1.53.0", "tokio"),
            section(CARGO_LOCK, "1.53.0", "1.53.1", "tokio"),
        ];
        check_supersession_chain(&source, &full_chain, CARGO_LOCK)
            .expect("the full chain should prove supersession");
    }

    /// An unrelated move that merely differs from source is not supersession.
    #[test]
    fn supersession_rejects_a_divergent_step() {
        let source = section(CARGO_LOCK, "1.52.3", "1.53.0", "tokio");
        let downgrade = vec![section(CARGO_LOCK, "1.40.0", "1.39.0", "tokio")];
        let error = check_supersession_chain(&source, &downgrade, CARGO_LOCK)
            .expect_err("a divergent step must be rejected");
        assert!(
            error.to_string().contains("not contiguous"),
            "unexpected error: {error}"
        );
    }

    /// Landing on the same version source landed is equivalence, and claiming
    /// supersession would earn authority the evidence does not support.
    #[test]
    fn supersession_rejects_a_chain_that_only_reaches_the_source_version() {
        let source = section(CARGO_LOCK, "1.52.3", "1.53.0", "tokio");
        let same = vec![section(CARGO_LOCK, "1.52.3", "1.53.0", "tokio")];
        let error = check_supersession_chain(&source, &same, CARGO_LOCK)
            .expect_err("reaching only the source version is not supersession");
        assert!(
            error.to_string().contains("equivalent, not superseded"),
            "unexpected error: {error}"
        );
    }
}
