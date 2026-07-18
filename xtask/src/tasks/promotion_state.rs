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
use std::fs;
use std::path::Path;

pub const MANIFEST_REL: &str = "plans/shiplog-swarm/promotion-state.toml";
pub const GENERATED_REL: &str = "plans/shiplog-swarm/current-promotion.md";

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
}
