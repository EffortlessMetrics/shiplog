//! `cargo xtask promotion-body`
//!
//! Generates a source promotion pull request body from the current source/swarm
//! refs. This is a derived artifact only; it does not create or edit PRs.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const SWARM_REPO: &str = "EffortlessMetrics/shiplog-swarm";

pub struct PromotionBodyInputs {
    pub workspace_root: PathBuf,
    pub source_ref: String,
    pub swarm_ref: String,
    pub swarm_head: Option<String>,
    pub included_swarm_prs: Vec<String>,
    pub swarm_pr_run: Option<String>,
    pub swarm_main_run: Option<String>,
    pub source_pr_run: Option<String>,
    pub source_post_merge_run: Option<String>,
    pub output: PathBuf,
}

#[derive(Debug)]
struct PromotionBodyData {
    source_ref: String,
    swarm_ref: String,
    swarm_head: String,
    included_swarm_prs: Vec<String>,
    swarm_pr_run: Option<String>,
    swarm_main_run: Option<String>,
    source_pr_run: Option<String>,
    source_post_merge_run: Option<String>,
}

pub fn run(inputs: PromotionBodyInputs) -> Result<()> {
    let body = render(&inputs)?;
    write_rendered(&inputs.workspace_root, &inputs.output, &body)?;
    println!(
        "promotion-body: wrote {}",
        display_path(
            &inputs.workspace_root,
            &resolve_output_path(&inputs.workspace_root, &inputs.output)
        )
    );
    Ok(())
}

pub(crate) fn render(inputs: &PromotionBodyInputs) -> Result<String> {
    let swarm_head = match inputs.swarm_head.as_deref() {
        Some(value) if !value.trim().is_empty() => value.trim().to_string(),
        _ => git_output(&inputs.workspace_root, &["rev-parse", &inputs.swarm_ref])
            .with_context(|| format!("promotion-body: resolve {}", inputs.swarm_ref))?,
    };
    let included_swarm_prs = if inputs.included_swarm_prs.is_empty() {
        infer_included_swarm_prs(
            &inputs.workspace_root,
            &inputs.source_ref,
            &inputs.swarm_ref,
            &swarm_head,
        )?
    } else {
        normalize_included_swarm_prs(&inputs.included_swarm_prs)
    };

    let data = PromotionBodyData {
        source_ref: inputs.source_ref.clone(),
        swarm_ref: inputs.swarm_ref.clone(),
        swarm_head,
        included_swarm_prs,
        swarm_pr_run: inputs.swarm_pr_run.clone(),
        swarm_main_run: inputs.swarm_main_run.clone(),
        source_pr_run: inputs.source_pr_run.clone(),
        source_post_merge_run: inputs.source_post_merge_run.clone(),
    };
    Ok(render_promotion_body(&data))
}

pub(crate) fn write_rendered(workspace_root: &Path, output: &Path, body: &str) -> Result<PathBuf> {
    let output_path = resolve_output_path(workspace_root, output);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    fs::write(&output_path, body).with_context(|| format!("write {}", output_path.display()))?;
    Ok(output_path)
}

fn git_output(workspace_root: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(workspace_root)
        .output()
        .with_context(|| format!("run git {}", args.join(" ")))?;
    if !output.status.success() {
        anyhow::bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn git_output_lines(workspace_root: &Path, args: &[&str]) -> Result<Vec<String>> {
    Ok(git_output(workspace_root, args)?
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

fn infer_included_swarm_prs(
    workspace_root: &Path,
    source_ref: &str,
    swarm_ref: &str,
    swarm_head: &str,
) -> Result<Vec<String>> {
    let range = format!("{source_ref}..{swarm_ref}");
    let subjects = git_output_lines(workspace_root, &["log", "--pretty=%s", &range])
        .with_context(|| format!("promotion-body: list commits in {range}"))?;
    let prs = included_swarm_prs_from_subjects(&subjects);
    if !prs.is_empty() {
        return Ok(prs);
    }

    let parent_line = git_output(workspace_root, &["show", "-s", "--format=%P", source_ref])
        .with_context(|| format!("promotion-body: inspect parents for {source_ref}"))?;
    let Some(parent_range) = promotion_merge_parent_range(&parent_line, swarm_head) else {
        return Ok(prs);
    };
    let subjects = git_output_lines(workspace_root, &["log", "--pretty=%s", &parent_range])
        .with_context(|| format!("promotion-body: list merged swarm commits in {parent_range}"))?;
    Ok(included_swarm_prs_from_subjects(&subjects))
}

fn included_swarm_prs_from_subjects(subjects: &[String]) -> Vec<String> {
    let mut prs = Vec::new();
    for subject in subjects.iter().rev() {
        if let Some(number) = extract_parenthesized_pull_request_number(subject) {
            let receipt = format!("{SWARM_REPO}#{number}");
            if !prs.contains(&receipt) {
                prs.push(receipt);
            }
        }
    }
    prs
}

fn normalize_included_swarm_prs(values: &[String]) -> Vec<String> {
    let mut prs = Vec::new();
    for value in values {
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        let receipt = match value.strip_prefix('#') {
            Some(number) => format!("{SWARM_REPO}#{number}"),
            None if value.chars().all(|ch| ch.is_ascii_digit()) => {
                format!("{SWARM_REPO}#{value}")
            }
            None => value.to_string(),
        };
        if !prs.contains(&receipt) {
            prs.push(receipt);
        }
    }
    prs
}

fn promotion_merge_parent_range(parent_line: &str, swarm_head: &str) -> Option<String> {
    let parents = parent_line.split_whitespace().collect::<Vec<_>>();
    let [source_parent, swarm_parent] = parents.as_slice() else {
        return None;
    };
    if same_commit_id(swarm_parent, swarm_head) {
        Some(format!("{source_parent}..{swarm_parent}"))
    } else {
        None
    }
}

fn same_commit_id(left: &str, right: &str) -> bool {
    let left = left.trim();
    let right = right.trim();
    if left.is_empty() || right.is_empty() {
        return false;
    }
    if left == right {
        return true;
    }
    left.len().min(right.len()) >= 7 && (left.starts_with(right) || right.starts_with(left))
}

fn extract_parenthesized_pull_request_number(subject: &str) -> Option<u64> {
    let start = subject.rfind("(#")?;
    let rest = &subject[start + 2..];
    let end = rest.find(')')?;
    rest[..end].parse().ok()
}

fn render_promotion_body(data: &PromotionBodyData) -> String {
    let mut out = String::new();
    out.push_str("## Summary\n\n");
    out.push_str(&format!(
        "Promotes shiplog-swarm/main through {} into the release/source repo.\n\n",
        data.swarm_head
    ));
    out.push_str("Merge with a regular merge commit; do not squash.\n\n");

    out.push_str("## Swarm head\n\n");
    out.push_str(&data.swarm_head);
    out.push_str("\n\n");

    out.push_str("## Included swarm PRs\n\n");
    if data.included_swarm_prs.is_empty() {
        out.push_str("- None inferred from ");
        out.push_str(&format!(
            "`{}..{}`; verify refs before promotion.\n\n",
            data.source_ref, data.swarm_ref
        ));
    } else {
        for pr in &data.included_swarm_prs {
            out.push_str("- ");
            out.push_str(pr);
            out.push('\n');
        }
        out.push('\n');
    }

    out.push_str("## Swarm proof\n\n");
    out.push_str(&proof_line(
        "Swarm PR routed run",
        data.swarm_pr_run.as_deref(),
        "pending",
    ));
    out.push_str(&proof_line(
        "Swarm main post-merge routed run",
        data.swarm_main_run.as_deref(),
        "pending",
    ));
    out.push('\n');

    out.push_str("## Source proof\n\n");
    out.push_str(&proof_line(
        "Source PR routed run",
        data.source_pr_run.as_deref(),
        "pending until source PR checks complete",
    ));
    out.push_str(&proof_line(
        "Source post-merge routed run",
        data.source_post_merge_run.as_deref(),
        "pending until merge",
    ));
    out.push('\n');

    out.push_str("## Claim boundary\n\n");
    out.push_str(
        "Promotion only. No release, tag, publish, branch-protection, runner-policy, or release-authority changes.\n",
    );
    out.push_str("\n## Rollback\n\n");
    out.push_str(
        "If the promotion must be reversed, revert the regular merge commit in the source repository and pause further promotions. Investigate and reconcile the source/swarm divergence; resume promotion only after alignment and source truth are restored. This tool does not perform rollback.\n",
    );
    out
}

fn proof_line(label: &str, run: Option<&str>, pending: &str) -> String {
    match run {
        Some(run) if !run.trim().is_empty() => {
            format!("- {label}: {run}, Shiplog Rust Small Result passed.\n")
        }
        _ => format!("- {label}: {pending}.\n"),
    }
}

fn resolve_output_path(workspace_root: &Path, output: &Path) -> PathBuf {
    if output.is_absolute() {
        output.to_path_buf()
    } else {
        workspace_root.join(output)
    }
}

fn display_path(workspace_root: &Path, path: &Path) -> String {
    path.strip_prefix(workspace_root)
        .unwrap_or(path)
        .display()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn included_swarm_prs_are_in_commit_order() {
        let subjects = vec![
            "xtask: recommend next repo-control slice (#150)".to_string(),
            "docs: add swarm next-improvement discovery (#149)".to_string(),
        ];

        assert_eq!(
            included_swarm_prs_from_subjects(&subjects),
            vec![
                "EffortlessMetrics/shiplog-swarm#149",
                "EffortlessMetrics/shiplog-swarm#150"
            ]
        );
    }

    #[test]
    fn normalizes_explicit_included_swarm_prs() {
        let values = vec![
            "151".to_string(),
            "#152".to_string(),
            "EffortlessMetrics/shiplog-swarm#153".to_string(),
            "151".to_string(),
            " ".to_string(),
        ];

        assert_eq!(
            normalize_included_swarm_prs(&values),
            vec![
                "EffortlessMetrics/shiplog-swarm#151",
                "EffortlessMetrics/shiplog-swarm#152",
                "EffortlessMetrics/shiplog-swarm#153"
            ]
        );
    }

    #[test]
    fn promotion_merge_parent_range_matches_swarm_second_parent() {
        let range = promotion_merge_parent_range(
            "1111111111111111111111111111111111111111 2222222222222222222222222222222222222222",
            "2222222222222222222222222222222222222222",
        );

        assert_eq!(
            range.as_deref(),
            Some(
                "1111111111111111111111111111111111111111..2222222222222222222222222222222222222222"
            )
        );
    }

    #[test]
    fn promotion_merge_parent_range_allows_abbreviated_swarm_head() {
        let range = promotion_merge_parent_range(
            "1111111111111111111111111111111111111111 2222222222222222222222222222222222222222",
            "2222222",
        );

        assert!(range.is_some());
    }

    #[test]
    fn promotion_merge_parent_range_rejects_too_short_swarm_head() {
        let range = promotion_merge_parent_range(
            "1111111111111111111111111111111111111111 2222222222222222222222222222222222222222",
            "222",
        );

        assert_eq!(range, None);
    }

    #[test]
    fn promotion_merge_parent_range_rejects_non_matching_second_parent() {
        let range = promotion_merge_parent_range(
            "1111111111111111111111111111111111111111 2222222222222222222222222222222222222222",
            "3333333333333333333333333333333333333333",
        );

        assert_eq!(range, None);
    }

    #[test]
    fn promotion_merge_subjects_infer_swarm_prs_after_source_merge() {
        let subjects = vec![
            "xtask: include control-plane closeout sections (#161)".to_string(),
            "docs: expand closeout control-plane template (#160)".to_string(),
        ];

        assert_eq!(
            included_swarm_prs_from_subjects(&subjects),
            vec![
                "EffortlessMetrics/shiplog-swarm#160",
                "EffortlessMetrics/shiplog-swarm#161"
            ]
        );
    }

    #[test]
    fn renders_merge_commit_promotion_body_with_pending_source_proof() {
        let data = PromotionBodyData {
            source_ref: "origin/main".to_string(),
            swarm_ref: "swarm/main".to_string(),
            swarm_head: "e303d696bd063d8362ec30c2c0d72b2a68cf9498".to_string(),
            included_swarm_prs: vec!["EffortlessMetrics/shiplog-swarm#150".to_string()],
            swarm_pr_run: Some("26803480265".to_string()),
            swarm_main_run: Some("26803857830".to_string()),
            source_pr_run: None,
            source_post_merge_run: None,
        };

        let body = render_promotion_body(&data);

        assert!(body.contains("regular merge commit; do not squash"));
        assert!(body.contains("EffortlessMetrics/shiplog-swarm#150"));
        assert!(
            body.contains("Swarm PR routed run: 26803480265, Shiplog Rust Small Result passed")
        );
        assert!(body.contains("Source PR routed run: pending until source PR checks complete"));
        assert!(body.contains("No release, tag, publish"));
        assert!(body.contains("## Rollback"));
        assert!(body.contains("revert the regular merge commit in the source repository"));
        assert!(body.contains("pause further promotions"));
        assert!(body.contains("Investigate and reconcile the source/swarm divergence"));
        assert!(body.contains("This tool does not perform rollback"));
    }
}
