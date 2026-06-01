//! File-policy checkers (PR #149 of the v0.5.0 ladder).
//!
//! Seven `cargo xtask check-*` commands consume the seven file-policy
//! ledgers landed in PR #141:
//!
//! - `check-file-policy` ↔ `policy/non-rust-allowlist.toml`
//! - `check-generated` ↔ `policy/generated-allowlist.toml`
//! - `check-executable-files` ↔ `policy/executable-allowlist.toml`
//! - `check-workflows` ↔ `policy/workflow-allowlist.toml`
//! - `check-dependency-surfaces` ↔ `policy/dependency-surface-allowlist.toml`
//! - `check-process-policy` ↔ `policy/process-allowlist.toml`
//! - `check-network-policy` ↔ `policy/network-allowlist.toml`
//!
//! All checkers share a common `Mode`:
//!
//! - `Advisory` — print findings, exit 0
//! - `BlockingAllowlist` — exit non-zero on any finding
//!
//! See [`docs/FILE_POLICY.md`](../../docs/FILE_POLICY.md) and
//! [`docs/POLICY_ALLOWLISTS.md`](../../docs/POLICY_ALLOWLISTS.md).

use anyhow::{Context, Result, bail};
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::policy;

#[derive(Debug, Clone, Copy)]
pub enum Mode {
    Advisory,
    BlockingAllowlist,
}

#[derive(Debug)]
struct Finding {
    kind: String,
    detail: String,
}

fn report(label: &str, findings: &[Finding], mode: Mode) -> Result<()> {
    if findings.is_empty() {
        println!("{label}: no findings.");
        return Ok(());
    }
    println!("{label}: {} finding(s).", findings.len());
    for f in findings {
        eprintln!("  [{}] {}", f.kind, f.detail);
    }
    match mode {
        Mode::Advisory => {
            println!("(advisory mode: not failing)");
            Ok(())
        }
        Mode::BlockingAllowlist => {
            bail!(
                "{label}: {} finding(s) (blocking-allowlist mode)",
                findings.len()
            )
        }
    }
}

// ────────────────────────────────────────────────────────────────────────
// Common helpers
// ────────────────────────────────────────────────────────────────────────

fn git_ls_files(workspace_root: &Path) -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["ls-files"])
        .current_dir(workspace_root)
        .output()
        .context("invoke git ls-files")?;
    if !output.status.success() {
        bail!(
            "git ls-files failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8(output.stdout)
        .context("git ls-files output not valid UTF-8")?
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect())
}

fn git_ls_files_stage(workspace_root: &Path) -> Result<Vec<(String, String)>> {
    // Output format: `<mode> <hash> <stage>\t<path>` per line. Mode is octal
    // (e.g. 100644 = regular, 100755 = executable, 120000 = symlink).
    let output = Command::new("git")
        .args(["ls-files", "--stage"])
        .current_dir(workspace_root)
        .output()
        .context("invoke git ls-files --stage")?;
    if !output.status.success() {
        bail!(
            "git ls-files --stage failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let text = String::from_utf8(output.stdout).context("git output not valid UTF-8")?;
    let mut out = Vec::new();
    for line in text.lines() {
        let mut parts = line.splitn(2, '\t');
        let meta = parts.next().unwrap_or("");
        let path = parts.next().unwrap_or("");
        let mode = meta.split_whitespace().next().unwrap_or("");
        if !mode.is_empty() && !path.is_empty() {
            out.push((mode.to_string(), path.to_string()));
        }
    }
    Ok(out)
}

fn build_globset(globs: &[String], context_label: &str) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for g in globs {
        let glob = Glob::new(g).with_context(|| format!("bad glob in {context_label}: {g}"))?;
        builder.add(glob);
    }
    builder
        .build()
        .with_context(|| format!("build glob set for {context_label}"))
}

fn load_policy_table(policy_dir: &Path, file: &str) -> Result<toml::Table> {
    let path = policy_dir.join(file);
    let text =
        fs::read_to_string(&path).with_context(|| format!("read policy {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("parse policy {}", path.display()))
}

fn validate_entry_fields(entries: &[toml::Value], required: &[&str], label: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (idx, entry) in entries.iter().enumerate() {
        let table = match entry.as_table() {
            Some(t) => t,
            None => {
                findings.push(Finding {
                    kind: format!("{label}-bad-entry"),
                    detail: format!("entry [{idx}] is not a table"),
                });
                continue;
            }
        };
        for required_field in required {
            if !table.contains_key(*required_field) {
                let id = table
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("(no id)");
                findings.push(Finding {
                    kind: format!("{label}-missing-field"),
                    detail: format!("entry id={id:?} missing required field {required_field:?}"),
                });
            }
        }
    }
    findings
}

// ────────────────────────────────────────────────────────────────────────
// check-file-policy (non-rust-allowlist)
// ────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct AllowGlob {
    #[serde(default)]
    glob: Option<String>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default, rename = "broad_glob_reason")]
    broad_glob_reason: Option<String>,
}

pub fn check_file_policy(workspace_root: &Path, mode: Mode) -> Result<()> {
    let policy_dir = policy::policy_dir(workspace_root);
    let table = load_policy_table(&policy_dir, "non-rust-allowlist.toml")?;
    let allows = table
        .get("allow")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let parsed: Vec<AllowGlob> = allows
        .iter()
        .filter_map(|v| v.clone().try_into().ok())
        .collect();
    let globs: Vec<String> = parsed
        .iter()
        .filter_map(|a| a.glob.clone().or_else(|| a.path.clone()))
        .collect();
    let matcher = build_globset(&globs, "non-rust-allowlist")?;

    let files = git_ls_files(workspace_root)?;
    let mut findings = Vec::new();
    for file in &files {
        let path = file.replace('\\', "/");
        if path.ends_with(".rs") {
            continue;
        }
        if !matcher.is_match(&path) {
            findings.push(Finding {
                kind: "unallowlisted-non-rust".to_string(),
                detail: path,
            });
        }
    }

    findings.extend(check_broad_glob_justifications(&parsed));

    report("check-file-policy", &findings, mode)
}

/// Returns true if a glob pattern is considered "broad" — it contains a
/// wildcard component (`*` or `**`) and therefore matches more than a
/// single named path. Narrow globs (literal paths via the `path` field,
/// or `glob` entries with no wildcards) don't require a
/// `broad_glob_reason`.
fn is_broad_glob(g: &str) -> bool {
    g.contains("**") || g.contains('*')
}

/// Validate that every allow entry whose `glob` is broad declares a
/// non-empty, non-whitespace `broad_glob_reason`. The original check
/// only required the field to be present, which let `""` and `"  "`
/// pass silently — degrading the auditor receipt from "deliberate
/// justification" to "field exists."
fn check_broad_glob_justifications(entries: &[AllowGlob]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for entry in entries {
        let Some(g) = entry.glob.as_deref() else {
            continue;
        };
        if !is_broad_glob(g) {
            continue;
        }
        match entry.broad_glob_reason.as_deref() {
            None => findings.push(Finding {
                kind: "broad-glob-without-reason".to_string(),
                detail: format!("allow glob {g:?} should set `broad_glob_reason`"),
            }),
            Some(reason) if reason.trim().is_empty() => findings.push(Finding {
                kind: "broad-glob-empty-reason".to_string(),
                detail: format!(
                    "allow glob {g:?} has an empty / whitespace-only `broad_glob_reason`; explain why a narrower set of paths is not viable"
                ),
            }),
            Some(_) => {}
        }
    }
    findings
}

// ────────────────────────────────────────────────────────────────────────
// check-generated
// ────────────────────────────────────────────────────────────────────────

pub fn check_generated(workspace_root: &Path, mode: Mode) -> Result<()> {
    let policy_dir = policy::policy_dir(workspace_root);
    let table = load_policy_table(&policy_dir, "generated-allowlist.toml")?;
    let allows = table
        .get("allow")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let allow_paths: BTreeSet<String> = allows
        .iter()
        .filter_map(|v| v.as_table())
        .filter_map(|t| t.get("path").and_then(|p| p.as_str()).map(String::from))
        .collect();

    // Parse .gitattributes for `linguist-generated=true` entries.
    let gitattr = workspace_root.join(".gitattributes");
    let generated_paths = if gitattr.is_file() {
        let text = fs::read_to_string(&gitattr).context("read .gitattributes")?;
        let mut out = BTreeSet::new();
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if trimmed.contains("linguist-generated=true")
                && let Some(path) = trimmed.split_whitespace().next()
            {
                out.insert(path.to_string());
            }
        }
        out
    } else {
        BTreeSet::new()
    };

    let mut findings = Vec::new();
    for path in &generated_paths {
        if !allow_paths.contains(path) {
            findings.push(Finding {
                kind: "gitattributes-missing-allow".to_string(),
                detail: format!(
                    ".gitattributes marks {path:?} as linguist-generated but no allow entry exists"
                ),
            });
        }
    }
    for path in &allow_paths {
        if !generated_paths.contains(path) {
            findings.push(Finding {
                kind: "allow-missing-gitattributes".to_string(),
                detail: format!("allow lists {path:?} but .gitattributes does not mark it"),
            });
        }
    }

    report("check-generated", &findings, mode)
}

// ────────────────────────────────────────────────────────────────────────
// check-executable-files
// ────────────────────────────────────────────────────────────────────────

pub fn check_executable_files(workspace_root: &Path, mode: Mode) -> Result<()> {
    let policy_dir = policy::policy_dir(workspace_root);
    let table = load_policy_table(&policy_dir, "executable-allowlist.toml")?;
    let allows = table
        .get("allow")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let allow_paths: BTreeSet<String> = allows
        .iter()
        .filter_map(|v| v.as_table())
        .filter_map(|t| t.get("path").and_then(|p| p.as_str()).map(String::from))
        .collect();

    let staged = git_ls_files_stage(workspace_root)?;
    let executable_paths: BTreeSet<String> = staged
        .iter()
        .filter(|(mode_str, _)| mode_str == "100755")
        .map(|(_, path)| path.replace('\\', "/"))
        .collect();

    let mut findings = Vec::new();
    for path in &executable_paths {
        if !allow_paths.contains(path) {
            findings.push(Finding {
                kind: "unallowlisted-executable".to_string(),
                detail: format!("file {path:?} has executable bit but no allow entry"),
            });
        }
    }
    for path in &allow_paths {
        if !executable_paths.contains(path) {
            findings.push(Finding {
                kind: "allow-not-executable".to_string(),
                detail: format!(
                    "allow lists {path:?} but git tree mode is not 100755 (file may not be executable)"
                ),
            });
        }
    }

    report("check-executable-files", &findings, mode)
}

// ────────────────────────────────────────────────────────────────────────
// check-workflows
// ────────────────────────────────────────────────────────────────────────

pub fn check_workflows(workspace_root: &Path, mode: Mode) -> Result<()> {
    let policy_dir = policy::policy_dir(workspace_root);
    let table = load_policy_table(&policy_dir, "workflow-allowlist.toml")?;
    let entries = table
        .get("entry")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    // Map allowlist entries by path.
    let mut allow_by_path: std::collections::BTreeMap<String, &toml::Table> =
        std::collections::BTreeMap::new();
    for entry in &entries {
        if let Some(t) = entry.as_table()
            && let Some(path) = t.get("path").and_then(|p| p.as_str())
        {
            allow_by_path.insert(path.to_string(), t);
        }
    }

    // Inventory actual workflow files.
    let workflows_dir = workspace_root.join(".github").join("workflows");
    let mut workflow_paths: BTreeSet<String> = BTreeSet::new();
    if workflows_dir.is_dir() {
        for entry in fs::read_dir(&workflows_dir).context("read .github/workflows")? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("yml")
                || path.extension().and_then(|e| e.to_str()) == Some("yaml")
            {
                let rel = path
                    .strip_prefix(workspace_root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                workflow_paths.insert(rel);
            }
        }
    }

    let mut findings = Vec::new();

    // Workflow files without allowlist entries.
    for path in &workflow_paths {
        if !allow_by_path.contains_key(path) {
            findings.push(Finding {
                kind: "workflow-missing-allow".to_string(),
                detail: format!("workflow {path:?} has no entry in workflow-allowlist.toml"),
            });
        }
    }
    // Allowlist entries pointing at missing files.
    for path in allow_by_path.keys() {
        if !workflow_paths.contains(path) {
            findings.push(Finding {
                kind: "allow-missing-workflow".to_string(),
                detail: format!(
                    "workflow-allowlist.toml lists {path:?} but the file does not exist"
                ),
            });
        }
    }

    // Per-workflow: every `uses: X@Y` line should be in the entry's external_actions.
    for (path, table) in &allow_by_path {
        let workflow_path = workspace_root.join(path.replace('/', std::path::MAIN_SEPARATOR_STR));
        if !workflow_path.is_file() {
            continue;
        }
        let text = fs::read_to_string(&workflow_path)
            .with_context(|| format!("read {}", workflow_path.display()))?;
        findings.extend(workflow_self_hosted_trust_findings(path, &text));
        let actual_uses: BTreeSet<String> = text.lines().filter_map(extract_uses).collect();
        let declared: BTreeSet<String> = table
            .get("external_actions")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();
        for u in &actual_uses {
            if !declared.contains(u) {
                findings.push(Finding {
                    kind: "uses-undeclared".to_string(),
                    detail: format!(
                        "workflow {path:?} uses {u:?} but it is not in external_actions"
                    ),
                });
            }
        }
        for d in &declared {
            if !actual_uses.contains(d) {
                findings.push(Finding {
                    kind: "uses-stale".to_string(),
                    detail: format!(
                        "workflow {path:?} declares external_action {d:?} but no `uses:` matches"
                    ),
                });
            }
        }
    }

    report("check-workflows", &findings, mode)
}

fn workflow_self_hosted_trust_findings(path: &str, text: &str) -> Vec<Finding> {
    const SWARM_REPO_SELECTOR: &str = "github.repository == 'EffortlessMetrics/shiplog-swarm'";
    const SAME_REPO_PULL_REQUEST: &str =
        "github.event.pull_request.head.repo.full_name == github.repository";
    const NON_PULL_REQUEST: &str = "github.event_name != 'pull_request'";

    text.lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let selects_swarm_repo = line.contains(SWARM_REPO_SELECTOR);
            let selects_self_hosted =
                line.contains("'self-hosted'") || line.contains("matrix.os");
            let has_trust_boundary =
                line.contains(NON_PULL_REQUEST) && line.contains(SAME_REPO_PULL_REQUEST);
            (selects_swarm_repo && selects_self_hosted && !has_trust_boundary).then(|| Finding {
                kind: "workflow-self-hosted-without-trust-boundary".to_string(),
                detail: format!(
                    "workflow {path:?} line {} selects self-hosted for shiplog-swarm without a same-repo pull_request guard",
                    index + 1
                ),
            })
        })
        .collect()
}

fn extract_uses(line: &str) -> Option<String> {
    // Allow YAML list-marker prefix `- uses: ...` as well as bare `uses: ...`.
    let trimmed = line.trim().trim_start_matches('-').trim_start();
    let stripped = trimmed.strip_prefix("uses:")?;
    let value = stripped.trim();
    if value.is_empty() {
        return None;
    }
    // Strip an optional inline comment.
    let no_comment = value.split('#').next().unwrap_or(value).trim();
    if no_comment.is_empty() {
        None
    } else {
        Some(no_comment.to_string())
    }
}

// ────────────────────────────────────────────────────────────────────────
// check-dependency-surfaces
// ────────────────────────────────────────────────────────────────────────

pub fn check_dependency_surfaces(workspace_root: &Path, mode: Mode) -> Result<()> {
    let policy_dir = policy::policy_dir(workspace_root);
    let table = load_policy_table(&policy_dir, "dependency-surface-allowlist.toml")?;
    let allows = table
        .get("allow")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let parsed: Vec<AllowGlob> = allows
        .iter()
        .filter_map(|v| v.clone().try_into().ok())
        .collect();

    let tracked = git_ls_files(workspace_root)?;
    let tracked: Vec<String> = tracked.into_iter().map(|p| p.replace('\\', "/")).collect();

    let mut findings = Vec::new();
    for entry in &parsed {
        let pattern = entry.path.clone().or_else(|| entry.glob.clone());
        let Some(pattern) = pattern else {
            continue;
        };
        let glob = match Glob::new(&pattern) {
            Ok(g) => g.compile_matcher(),
            Err(e) => {
                findings.push(Finding {
                    kind: "bad-pattern".to_string(),
                    detail: format!("dependency-surface entry pattern {pattern:?} invalid: {e}"),
                });
                continue;
            }
        };
        if !tracked.iter().any(|p| glob.is_match(p)) {
            findings.push(Finding {
                kind: "surface-missing".to_string(),
                detail: format!(
                    "dependency-surface entry {pattern:?} matches no tracked file in the workspace"
                ),
            });
        }
    }

    report("check-dependency-surfaces", &findings, mode)
}

// ────────────────────────────────────────────────────────────────────────
// check-process-policy / check-network-policy (entry validation only in v1)
// ────────────────────────────────────────────────────────────────────────

pub fn check_process_policy(workspace_root: &Path, mode: Mode) -> Result<()> {
    let policy_dir = policy::policy_dir(workspace_root);
    let table = load_policy_table(&policy_dir, "process-allowlist.toml")?;
    let allows = table
        .get("allow")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let findings = validate_entry_fields(
        &allows,
        &[
            "id",
            "binary",
            "argv_shape",
            "network_reach",
            "owner",
            "reason",
            "created",
        ],
        "process-policy",
    );
    report("check-process-policy", &findings, mode)
}

pub fn check_network_policy(workspace_root: &Path, mode: Mode) -> Result<()> {
    let policy_dir = policy::policy_dir(workspace_root);
    let table = load_policy_table(&policy_dir, "network-allowlist.toml")?;
    let allows = table
        .get("allow")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let findings = validate_entry_fields(
        &allows,
        &[
            "id",
            "destination",
            "auth_required",
            "lane",
            "owner",
            "reason",
            "created",
        ],
        "network-policy",
    );
    report("check-network-policy", &findings, mode)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn fixture(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        for (name, content) in files {
            let path = dir.path().join(name);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, content).unwrap();
        }
        dir
    }

    #[test]
    fn process_policy_validates_required_fields() {
        let dir = fixture(&[(
            "policy/process-allowlist.toml",
            r#"
schema_version = 1
policy = "process-allowlist"
owner = "x"
status = "advisory"

[[allow]]
id = "proc-a"
binary = "cargo"
argv_shape = ["install"]
network_reach = true
owner = "release"
reason = "x"
created = "2026-05-09"
"#,
        )]);
        check_process_policy(dir.path(), Mode::BlockingAllowlist).expect("well-formed entry");
    }

    #[test]
    fn process_policy_finds_missing_required_fields() {
        let dir = fixture(&[(
            "policy/process-allowlist.toml",
            r#"
schema_version = 1
policy = "process-allowlist"
owner = "x"
status = "advisory"

[[allow]]
id = "proc-a"
binary = "cargo"
"#,
        )]);
        let err =
            check_process_policy(dir.path(), Mode::BlockingAllowlist).expect_err("missing fields");
        assert!(format!("{err:#}").contains("process-policy"));
    }

    #[test]
    fn network_policy_validates_required_fields() {
        let dir = fixture(&[(
            "policy/network-allowlist.toml",
            r#"
schema_version = 1
policy = "network-allowlist"
owner = "x"
status = "advisory"

[[allow]]
id = "net-a"
destination = "crates.io"
auth_required = false
lane = "build"
owner = "release"
reason = "x"
created = "2026-05-09"
"#,
        )]);
        check_network_policy(dir.path(), Mode::BlockingAllowlist).expect("well-formed entry");
    }

    #[test]
    fn generated_finds_drift_between_gitattributes_and_allow() {
        let dir = fixture(&[
            (
                ".gitattributes",
                "policy/no-panic-baseline.toml linguist-generated=true\n",
            ),
            (
                "policy/generated-allowlist.toml",
                r#"
schema_version = 1
policy = "generated-allowlist"
owner = "x"
status = "advisory"
"#,
            ),
        ]);
        let err = check_generated(dir.path(), Mode::BlockingAllowlist)
            .expect_err("gitattributes entry not in allow");
        assert!(format!("{err:#}").contains("check-generated"));
    }

    #[test]
    fn generated_passes_when_aligned() {
        let dir = fixture(&[
            (
                ".gitattributes",
                "policy/no-panic-baseline.toml linguist-generated=true\n",
            ),
            (
                "policy/generated-allowlist.toml",
                r#"
schema_version = 1
policy = "generated-allowlist"
owner = "x"
status = "advisory"

[[allow]]
id = "g1"
path = "policy/no-panic-baseline.toml"
generator = "xtask"
regenerate_command = "cargo xtask"
owner = "policy"
reason = "x"
created = "2026-05-09"
"#,
            ),
        ]);
        check_generated(dir.path(), Mode::BlockingAllowlist).expect("aligned");
    }

    #[test]
    fn extract_uses_strips_inline_comments() {
        assert_eq!(
            extract_uses("      - uses: actions/checkout@v6.0.2  # pinned"),
            Some("actions/checkout@v6.0.2".to_string())
        );
        assert_eq!(extract_uses("name: foo"), None);
        assert_eq!(extract_uses("uses:"), None);
    }

    #[test]
    fn workflow_self_hosted_trust_boundary_accepts_same_repo_guard() {
        let text = "runs-on: ${{ github.repository == 'EffortlessMetrics/shiplog-swarm' && (github.event_name != 'pull_request' || github.event.pull_request.head.repo.full_name == github.repository) && 'self-hosted' || 'ubuntu-latest' }}";

        let findings = workflow_self_hosted_trust_findings(".github/workflows/test.yml", text);

        assert!(findings.is_empty());
    }

    #[test]
    fn workflow_self_hosted_trust_boundary_rejects_repo_only_selector() {
        let text = "runs-on: ${{ github.repository == 'EffortlessMetrics/shiplog-swarm' && 'self-hosted' || 'ubuntu-latest' }}";

        let findings = workflow_self_hosted_trust_findings(".github/workflows/test.yml", text);

        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].kind,
            "workflow-self-hosted-without-trust-boundary"
        );
    }

    #[test]
    fn workflow_self_hosted_trust_boundary_rejects_matrix_selector_without_guard() {
        let text = "runs-on: ${{ github.repository == 'EffortlessMetrics/shiplog-swarm' && matrix.os || 'ubuntu-latest' }}";

        let findings = workflow_self_hosted_trust_findings(".github/workflows/test.yml", text);

        assert_eq!(findings.len(), 1);
    }

    fn allow(glob: Option<&str>, reason: Option<&str>) -> AllowGlob {
        AllowGlob {
            glob: glob.map(String::from),
            path: None,
            broad_glob_reason: reason.map(String::from),
        }
    }

    #[test]
    fn broad_glob_check_accepts_narrow_glob_without_reason() {
        let entries = [allow(Some("Cargo.toml"), None)];
        assert!(check_broad_glob_justifications(&entries).is_empty());
    }

    #[test]
    fn broad_glob_check_accepts_path_field_without_reason() {
        // `path = "..."` (not `glob`) is implicitly narrow; the check
        // should never fire on it.
        let entries = [AllowGlob {
            glob: None,
            path: Some(String::from("LICENSE")),
            broad_glob_reason: None,
        }];
        assert!(check_broad_glob_justifications(&entries).is_empty());
    }

    #[test]
    fn broad_glob_check_accepts_broad_glob_with_meaningful_reason() {
        let entries = [allow(
            Some("docs/**"),
            Some("Docs tree; per-file enumeration adds no review value."),
        )];
        assert!(check_broad_glob_justifications(&entries).is_empty());
    }

    #[test]
    fn broad_glob_check_rejects_broad_glob_without_reason() {
        let entries = [allow(Some("docs/**"), None)];
        let findings = check_broad_glob_justifications(&entries);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, "broad-glob-without-reason");
    }

    #[test]
    fn broad_glob_check_rejects_empty_reason() {
        let entries = [allow(Some("docs/**"), Some(""))];
        let findings = check_broad_glob_justifications(&entries);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, "broad-glob-empty-reason");
    }

    #[test]
    fn broad_glob_check_rejects_whitespace_only_reason() {
        let entries = [allow(Some("docs/**"), Some("   \n\t "))];
        let findings = check_broad_glob_justifications(&entries);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, "broad-glob-empty-reason");
    }

    #[test]
    fn broad_glob_check_distinguishes_single_star_from_double() {
        // Both single-`*` and `**` count as broad. A glob like
        // `scripts/*.sh` is broad even without `**`.
        let entries = [allow(Some("scripts/*.sh"), None)];
        let findings = check_broad_glob_justifications(&entries);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, "broad-glob-without-reason");
    }

    #[test]
    fn broad_glob_check_accumulates_multiple_findings() {
        let entries = [
            allow(Some("docs/**"), None),                // missing
            allow(Some("LICENSE-*"), Some("   ")),       // empty
            allow(Some("Cargo.toml"), None),             // narrow, OK
            allow(Some("**/*.rs"), Some("Rust files.")), // OK
        ];
        let findings = check_broad_glob_justifications(&entries);
        assert_eq!(findings.len(), 2);
        let kinds: Vec<&str> = findings.iter().map(|f| f.kind.as_str()).collect();
        assert!(kinds.contains(&"broad-glob-without-reason"));
        assert!(kinds.contains(&"broad-glob-empty-reason"));
    }
}
