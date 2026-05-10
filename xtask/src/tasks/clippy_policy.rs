//! Clippy ledger checkers (PR #150 of the v0.5.0 ladder).
//!
//! Two `cargo xtask check-*` commands:
//!
//! - `check-lint-policy` — verify `policy/clippy-lints.toml` matches
//!   `clippy.toml` and `[workspace.lints]` in `Cargo.toml`; verify each
//!   broad workspace allow is receipted in `policy/clippy-debt.toml`.
//! - `check-clippy-exceptions` — scan Rust source for bare
//!   `#[allow(clippy::...)]` (rejected) and `#[expect(clippy::...,
//!   reason = "...")]` (must cite a `policy/clippy-exceptions.toml`
//!   entry via `reason = "policy:<id>"`).
//!
//! Both checkers respect the shared
//! [`Mode`] — `advisory` (print, exit 0) or `blocking-allowlist` (exit
//! non-zero on findings).
//!
//! See [`docs/CLIPPY_POLICY.md`](../../docs/CLIPPY_POLICY.md).

use anyhow::{Context, Result, bail};
use regex::Regex;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

use crate::policy;
use crate::tasks::file_policy::Mode;

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
// Policy structs (only the fields we consume)
// ────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ClippyLintsPolicy {
    msrv: String,
    #[serde(default, rename = "active")]
    active: Vec<LintEntry>,
    #[serde(default, rename = "planned")]
    planned: Vec<PlannedLintEntry>,
}

#[derive(Debug, Deserialize)]
struct LintEntry {
    name: String,
    level: String,
    #[serde(default)]
    debt_ref: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PlannedLintEntry {
    name: String,
    level: String,
    activate_when_msrv: String,
}

#[derive(Debug, Deserialize)]
struct ClippyDebt {
    #[serde(default, rename = "debt")]
    debt: Vec<DebtEntry>,
}

#[derive(Debug, Deserialize)]
struct DebtEntry {
    id: String,
    lint: String,
    // `lint_level` exists in the TOML for review context but isn't consumed by
    // the cross-checker (we look up the level from workspace.lints itself).
    #[allow(dead_code)]
    lint_level: String,
}

#[derive(Debug, Deserialize)]
struct ClippyExceptions {
    #[serde(default, rename = "exception")]
    exception: Vec<ExceptionEntry>,
}

#[derive(Debug, Deserialize)]
struct ExceptionEntry {
    id: String,
    // `lint` exists in the TOML so reviewers can see which lint each exception
    // applies to, but the source-side cite uses only the id. Reserved for
    // future use (e.g. cross-check that source `#[expect(clippy::foo, ...)]`
    // matches the exception's declared lint).
    #[allow(dead_code)]
    #[serde(default)]
    lint: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WorkspaceCargo {
    workspace: WorkspaceBlock,
}

#[derive(Debug, Deserialize)]
struct WorkspaceBlock {
    package: PackageBlock,
    #[serde(default)]
    lints: Option<LintsBlock>,
}

#[derive(Debug, Deserialize)]
struct PackageBlock {
    #[serde(rename = "rust-version")]
    rust_version: String,
}

#[derive(Debug, Deserialize)]
struct LintsBlock {
    #[serde(default)]
    rust: BTreeMap<String, toml::Value>,
    #[serde(default)]
    clippy: BTreeMap<String, toml::Value>,
}

#[derive(Debug, Deserialize)]
struct ClippyToml {
    msrv: String,
}

// ────────────────────────────────────────────────────────────────────────
// check-lint-policy
// ────────────────────────────────────────────────────────────────────────

pub fn check_lint_policy(workspace_root: &Path, mode: Mode) -> Result<()> {
    let policy_dir = policy::policy_dir(workspace_root);
    let lints_text = fs::read_to_string(policy_dir.join("clippy-lints.toml"))
        .context("read policy/clippy-lints.toml")?;
    let lints: ClippyLintsPolicy =
        toml::from_str(&lints_text).context("parse policy/clippy-lints.toml")?;
    let debt_text = fs::read_to_string(policy_dir.join("clippy-debt.toml"))
        .context("read policy/clippy-debt.toml")?;
    let debt: ClippyDebt = toml::from_str(&debt_text).context("parse policy/clippy-debt.toml")?;

    let cargo_text = fs::read_to_string(workspace_root.join("Cargo.toml"))
        .context("read workspace Cargo.toml")?;
    let cargo: WorkspaceCargo =
        toml::from_str(&cargo_text).context("parse workspace Cargo.toml")?;

    let clippy_toml_path = workspace_root.join("clippy.toml");
    let clippy_toml: ClippyToml = if clippy_toml_path.is_file() {
        let text = fs::read_to_string(&clippy_toml_path).context("read clippy.toml")?;
        toml::from_str(&text).context("parse clippy.toml")?
    } else {
        ClippyToml {
            msrv: lints.msrv.clone(),
        }
    };

    let mut findings = Vec::new();

    // 1. workspace rust-version == policy msrv
    if cargo.workspace.package.rust_version != lints.msrv {
        findings.push(Finding {
            kind: "msrv-mismatch".to_string(),
            detail: format!(
                "workspace.package.rust-version = {:?}, policy/clippy-lints.toml msrv = {:?}",
                cargo.workspace.package.rust_version, lints.msrv
            ),
        });
    }

    // 2. clippy.toml msrv == policy msrv
    if clippy_toml.msrv != lints.msrv {
        findings.push(Finding {
            kind: "msrv-mismatch".to_string(),
            detail: format!(
                "clippy.toml msrv = {:?}, policy/clippy-lints.toml msrv = {:?}",
                clippy_toml.msrv, lints.msrv
            ),
        });
    }

    // Build maps of workspace lints (by `category::name` form).
    let mut workspace_lints: BTreeMap<String, String> = BTreeMap::new();
    if let Some(lints_block) = &cargo.workspace.lints {
        for (name, value) in &lints_block.rust {
            workspace_lints.insert(format!("rust::{name}"), level_of(value));
        }
        for (name, value) in &lints_block.clippy {
            workspace_lints.insert(format!("clippy::{name}"), level_of(value));
        }
    }

    // 3. Each [[active]] lint exists in workspace.lints at the declared level.
    for active in &lints.active {
        match workspace_lints.get(&active.name) {
            Some(actual_level) => {
                if actual_level != &active.level {
                    findings.push(Finding {
                        kind: "active-level-mismatch".to_string(),
                        detail: format!(
                            "lint {:?} declared level {:?} in policy but workspace.lints has {:?}",
                            active.name, active.level, actual_level
                        ),
                    });
                }
            }
            None => {
                findings.push(Finding {
                    kind: "active-missing".to_string(),
                    detail: format!(
                        "policy [[active]] lint {:?} is not present in [workspace.lints] in Cargo.toml",
                        active.name
                    ),
                });
            }
        }
    }

    // 4. Broad allows in workspace.lints must be receipted: either via an
    //    [[active]] entry whose `debt_ref` points to a debt entry, OR via a
    //    debt entry listing the lint by name directly.
    let debt_lints: BTreeSet<String> = debt.debt.iter().map(|d| d.lint.clone()).collect();
    for (name, level) in &workspace_lints {
        if level == "allow" {
            let in_active_with_debt = lints.active.iter().any(|a| {
                &a.name == name
                    && a.level == "allow"
                    && a.debt_ref
                        .as_ref()
                        .is_some_and(|id| debt.debt.iter().any(|d| &d.id == id))
            });
            let in_debt_directly = debt_lints.contains(name);
            if !in_active_with_debt && !in_debt_directly {
                findings.push(Finding {
                    kind: "allow-without-debt".to_string(),
                    detail: format!(
                        "workspace.lints allow on {name:?} has no matching entry in policy/clippy-debt.toml or [[active]] with debt_ref"
                    ),
                });
            }
        }
    }

    // 5. Planned lints with activate_when_msrv <= policy msrv: warning that they're
    //    eligible to activate (informational; not a hard finding unless lint
    //    actually fires).
    for planned in &lints.planned {
        if msrv_at_least(&lints.msrv, &planned.activate_when_msrv) {
            // Eligible — informational only (not a finding); the lint may still be
            // intentionally deferred to PR #152.
            let _ = (planned.name.as_str(), planned.level.as_str());
        }
    }

    report("check-lint-policy", &findings, mode)
}

fn level_of(value: &toml::Value) -> String {
    match value {
        toml::Value::String(s) => s.clone(),
        toml::Value::Table(t) => t
            .get("level")
            .and_then(|v| v.as_str())
            .unwrap_or("?")
            .to_string(),
        _ => "?".to_string(),
    }
}

fn msrv_at_least(have: &str, want: &str) -> bool {
    parse_msrv(have)
        .zip(parse_msrv(want))
        .is_some_and(|(a, b)| a >= b)
}

fn parse_msrv(s: &str) -> Option<(u32, u32, u32)> {
    let mut parts = s.split('.').map(|p| p.parse::<u32>().ok());
    let a = parts.next()??;
    let b = parts.next()??;
    let c = parts.next().flatten().unwrap_or(0);
    Some((a, b, c))
}

// ────────────────────────────────────────────────────────────────────────
// check-clippy-exceptions
// ────────────────────────────────────────────────────────────────────────

pub fn check_clippy_exceptions(workspace_root: &Path, mode: Mode) -> Result<()> {
    let policy_dir = policy::policy_dir(workspace_root);
    let exc_text = fs::read_to_string(policy_dir.join("clippy-exceptions.toml"))
        .context("read policy/clippy-exceptions.toml")?;
    let exceptions: ClippyExceptions =
        toml::from_str(&exc_text).context("parse policy/clippy-exceptions.toml")?;
    let exception_ids: BTreeSet<String> =
        exceptions.exception.iter().map(|e| e.id.clone()).collect();

    let rust_files = git_ls_rs_files(workspace_root)?;
    let mut findings = Vec::new();
    let mut cited_ids: BTreeSet<String> = BTreeSet::new();

    let allow_re = bare_allow_regex();
    let expect_re = expect_with_reason_regex();

    for file in &rust_files {
        let path = workspace_root.join(file);
        let text = match fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => continue,
        };
        // Track multi-line raw string depth (`r#"..."#` style). A line that
        // opens a raw string without closing it puts the scanner in
        // raw-string mode; subsequent lines are skipped until we see the
        // closing `"#`.
        let mut in_raw_string = false;
        for (lineno, line) in text.lines().enumerate() {
            if in_raw_string {
                if line.contains("\"#") {
                    in_raw_string = false;
                }
                continue;
            }
            // Skip lines inside string literals or doc-comment examples by being
            // crude: a leading `//` (line comment) means this is documentation
            // text, not actual code.
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") {
                continue;
            }
            // Detect raw-string opener (r#" or r##" etc.) without matching
            // close on the same line; switch to raw-string mode for the next
            // iteration.
            if line.contains("r#\"") && !line.contains("\"#") {
                in_raw_string = true;
                continue;
            }
            if let Some(cap) = allow_re.captures(line)
                && let Some(mat) = cap.get(0)
                && !is_inside_string_literal(line, mat.start())
            {
                let lint = cap.get(1).map_or("?", |m| m.as_str()).to_string();
                findings.push(Finding {
                    kind: "bare-allow".to_string(),
                    detail: format!(
                        "{}:{}: bare #[allow(clippy::{lint}, ...)] — use #[expect(clippy::{lint}, reason = \"policy:<id>\")] instead",
                        file,
                        lineno + 1
                    ),
                });
            }
            if let Some(cap) = expect_re.captures(line) {
                let lint = cap.get(1).map_or("?", |m| m.as_str()).to_string();
                let reason = cap.get(2).map_or("", |m| m.as_str());
                let policy_id = reason.strip_prefix("policy:");
                match policy_id {
                    None => {
                        findings.push(Finding {
                            kind: "expect-without-policy-citation".to_string(),
                            detail: format!(
                                "{}:{}: #[expect(clippy::{lint}, ...)] reason {reason:?} should start with `policy:<id>`",
                                file,
                                lineno + 1
                            ),
                        });
                    }
                    Some(id) => {
                        if !exception_ids.contains(id) {
                            findings.push(Finding {
                                kind: "expect-cites-unknown-id".to_string(),
                                detail: format!(
                                    "{}:{}: #[expect(clippy::{lint}, reason = \"policy:{id}\")] cites unknown exception id; add to policy/clippy-exceptions.toml",
                                    file,
                                    lineno + 1
                                ),
                            });
                        } else {
                            cited_ids.insert(id.to_string());
                        }
                    }
                }
            }
        }
    }

    // Find unused exception entries (declared but never cited).
    for exc in &exceptions.exception {
        if !cited_ids.contains(&exc.id) {
            findings.push(Finding {
                kind: "exception-unused".to_string(),
                detail: format!(
                    "policy/clippy-exceptions.toml entry id={:?} is not cited from any source location",
                    exc.id
                ),
            });
        }
    }

    report("check-clippy-exceptions", &findings, mode)
}

/// Detect whether `position` falls inside a `"..."` string literal on the
/// given line. Counts unescaped quotes before `position`; odd → inside.
/// Doesn't handle multi-line raw strings or escaped quotes inside raw
/// strings — sufficient for line-by-line lint scanning.
fn is_inside_string_literal(line: &str, position: usize) -> bool {
    let mut count = 0;
    let mut iter = line[..position].chars().peekable();
    while let Some(c) = iter.next() {
        match c {
            '\\' => {
                iter.next();
            }
            '"' => count += 1,
            _ => {}
        }
    }
    count % 2 == 1
}

fn git_ls_rs_files(workspace_root: &Path) -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["ls-files", "*.rs"])
        .current_dir(workspace_root)
        .output()
        .context("invoke git ls-files *.rs")?;
    if !output.status.success() {
        bail!(
            "git ls-files *.rs failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8(output.stdout)
        .context("git output not valid UTF-8")?
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect())
}

fn bare_allow_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"#\[\s*allow\s*\(\s*clippy::([A-Za-z0-9_]+)").expect("static regex compiles")
    })
}

fn expect_with_reason_regex() -> &'static Regex {
    // Match `#[expect(clippy::FOO, reason = "BAR")]` (single-line form).
    // Captures: (1) lint name, (2) reason string.
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"#\[\s*expect\s*\(\s*clippy::([A-Za-z0-9_]+)\s*,\s*reason\s*=\s*"([^"]*)""#)
            .expect("static regex compiles")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
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
        // Initialize a git repo so `git ls-files` works.
        let _ = Command::new("git")
            .args(["init", "-q"])
            .current_dir(dir.path())
            .output();
        let _ = Command::new("git")
            .args(["add", "-A"])
            .current_dir(dir.path())
            .output();
        dir
    }

    fn cargo_toml(rust_version: &str, lints: &str) -> String {
        format!(
            r#"
[workspace]
members = []

[workspace.package]
version = "0.4.0"
edition = "2024"
rust-version = "{rust_version}"

{lints}
"#
        )
    }

    #[test]
    fn lint_policy_passes_for_aligned_workspace() {
        let dir = fixture(&[
            (
                "Cargo.toml",
                &cargo_toml(
                    "1.95",
                    r#"
[workspace.lints.clippy]
enum_glob_use = "warn"
"#,
                ),
            ),
            ("clippy.toml", "msrv = \"1.95\"\n"),
            (
                "policy/clippy-lints.toml",
                r#"
schema_version = 1
policy = "clippy-lints"
owner = "x"
status = "advisory"
msrv = "1.95"

[[active]]
name = "clippy::enum_glob_use"
level = "warn"
class = "style"
reason = "x"
"#,
            ),
            (
                "policy/clippy-debt.toml",
                r#"
schema_version = 1
policy = "clippy-debt"
owner = "x"
status = "advisory"
"#,
            ),
        ]);
        check_lint_policy(dir.path(), Mode::BlockingAllowlist).expect("aligned workspace");
    }

    #[test]
    fn lint_policy_fails_on_msrv_mismatch() {
        let dir = fixture(&[
            (
                "Cargo.toml",
                &cargo_toml("1.92", "[workspace.lints.clippy]\n"),
            ),
            ("clippy.toml", "msrv = \"1.95\"\n"),
            (
                "policy/clippy-lints.toml",
                r#"
schema_version = 1
policy = "clippy-lints"
owner = "x"
status = "advisory"
msrv = "1.95"
"#,
            ),
            (
                "policy/clippy-debt.toml",
                r#"
schema_version = 1
policy = "clippy-debt"
owner = "x"
status = "advisory"
"#,
            ),
        ]);
        let err = check_lint_policy(dir.path(), Mode::BlockingAllowlist)
            .expect_err("should fail on MSRV drift");
        assert!(format!("{err:#}").contains("check-lint-policy"));
    }

    #[test]
    fn lint_policy_fails_when_active_lint_missing_from_workspace() {
        let dir = fixture(&[
            (
                "Cargo.toml",
                &cargo_toml("1.95", "[workspace.lints.clippy]\n"),
            ),
            ("clippy.toml", "msrv = \"1.95\"\n"),
            (
                "policy/clippy-lints.toml",
                r#"
schema_version = 1
policy = "clippy-lints"
owner = "x"
status = "advisory"
msrv = "1.95"

[[active]]
name = "clippy::enum_glob_use"
level = "warn"
class = "style"
reason = "x"
"#,
            ),
            (
                "policy/clippy-debt.toml",
                r#"
schema_version = 1
policy = "clippy-debt"
owner = "x"
status = "advisory"
"#,
            ),
        ]);
        let err = check_lint_policy(dir.path(), Mode::BlockingAllowlist)
            .expect_err("active lint not in workspace");
        assert!(format!("{err:#}").contains("check-lint-policy"));
    }

    #[test]
    fn lint_policy_fails_on_undebt_broad_allow() {
        let dir = fixture(&[
            (
                "Cargo.toml",
                &cargo_toml(
                    "1.95",
                    r#"
[workspace.lints.clippy]
needless_pass_by_value = "allow"
"#,
                ),
            ),
            ("clippy.toml", "msrv = \"1.95\"\n"),
            (
                "policy/clippy-lints.toml",
                r#"
schema_version = 1
policy = "clippy-lints"
owner = "x"
status = "advisory"
msrv = "1.95"

[[active]]
name = "clippy::needless_pass_by_value"
level = "allow"
class = "style"
reason = "x"
"#,
            ),
            (
                "policy/clippy-debt.toml",
                r#"
schema_version = 1
policy = "clippy-debt"
owner = "x"
status = "advisory"
"#,
            ),
        ]);
        let err = check_lint_policy(dir.path(), Mode::BlockingAllowlist)
            .expect_err("broad allow not receipted");
        assert!(format!("{err:#}").contains("check-lint-policy"));
    }

    #[test]
    fn bare_allow_regex_matches_simple_form() {
        let re = bare_allow_regex();
        assert!(re.is_match("    #[allow(clippy::foo)]"));
        assert!(re.is_match("#[allow(clippy::needless_pass_by_value)]"));
        assert!(re.is_match("#[allow(clippy::foo, dead_code)]"));
        assert!(!re.is_match("#[expect(clippy::foo, reason = \"...\")]"));
        // `dead_code` alone (rust lint, not clippy) — not flagged
        assert!(!re.is_match("#[allow(dead_code)]"));
    }

    #[test]
    fn expect_with_reason_regex_extracts_reason() {
        let re = expect_with_reason_regex();
        let caps = re
            .captures("#[expect(clippy::foo, reason = \"policy:clippy-0001\")]")
            .unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "foo");
        assert_eq!(caps.get(2).unwrap().as_str(), "policy:clippy-0001");
    }

    #[test]
    fn clippy_exceptions_finds_bare_allow_in_source() {
        let dir = fixture(&[
            (
                "policy/clippy-exceptions.toml",
                r#"
schema_version = 1
policy = "clippy-exceptions"
owner = "x"
status = "advisory"
"#,
            ),
            (
                "src/lib.rs",
                r#"
#[allow(clippy::needless_pass_by_value)]
fn foo(x: String) {}
"#,
            ),
        ]);
        let err = check_clippy_exceptions(dir.path(), Mode::BlockingAllowlist)
            .expect_err("bare allow should fail");
        assert!(format!("{err:#}").contains("check-clippy-exceptions"));
    }

    #[test]
    fn clippy_exceptions_passes_for_expect_with_known_policy_id() {
        let dir = fixture(&[
            (
                "policy/clippy-exceptions.toml",
                r#"
schema_version = 1
policy = "clippy-exceptions"
owner = "x"
status = "advisory"

[[exception]]
id = "clippy-0001"
lint = "clippy::indexing_slicing"
path = "src/lib.rs"
classification = "x"
owner = "cli"
reason = "x"
created = "2026-05-09"
"#,
            ),
            (
                "src/lib.rs",
                r#"
#[expect(clippy::indexing_slicing, reason = "policy:clippy-0001")]
fn foo() { let v = vec![1]; let _ = v[0]; }
"#,
            ),
        ]);
        check_clippy_exceptions(dir.path(), Mode::BlockingAllowlist).expect("aligned");
    }

    #[test]
    fn clippy_exceptions_fails_when_expect_cites_unknown_id() {
        let dir = fixture(&[
            (
                "policy/clippy-exceptions.toml",
                r#"
schema_version = 1
policy = "clippy-exceptions"
owner = "x"
status = "advisory"
"#,
            ),
            (
                "src/lib.rs",
                "#[expect(clippy::foo, reason = \"policy:bogus-id\")]\nfn x() {}\n",
            ),
        ]);
        let err =
            check_clippy_exceptions(dir.path(), Mode::BlockingAllowlist).expect_err("unknown id");
        assert!(format!("{err:#}").contains("check-clippy-exceptions"));
    }

    #[test]
    fn clippy_exceptions_fails_when_expect_reason_lacks_policy_prefix() {
        let dir = fixture(&[
            (
                "policy/clippy-exceptions.toml",
                r#"
schema_version = 1
policy = "clippy-exceptions"
owner = "x"
status = "advisory"
"#,
            ),
            (
                "src/lib.rs",
                "#[expect(clippy::foo, reason = \"just because\")]\nfn x() {}\n",
            ),
        ]);
        let err = check_clippy_exceptions(dir.path(), Mode::BlockingAllowlist)
            .expect_err("missing policy: prefix");
        assert!(format!("{err:#}").contains("check-clippy-exceptions"));
    }

    #[test]
    fn clippy_exceptions_skips_lines_in_comments() {
        // A doc comment showing an example shouldn't trigger the checker.
        let dir = fixture(&[
            (
                "policy/clippy-exceptions.toml",
                r#"
schema_version = 1
policy = "clippy-exceptions"
owner = "x"
status = "advisory"
"#,
            ),
            (
                "src/lib.rs",
                r#"
// example: #[allow(clippy::foo)] — would be flagged if this were code
fn x() {}
"#,
            ),
        ]);
        check_clippy_exceptions(dir.path(), Mode::BlockingAllowlist)
            .expect("comment-line allow should be skipped");
    }

    #[test]
    fn msrv_at_least_compares_semver_parts() {
        assert!(msrv_at_least("1.95", "1.94"));
        assert!(msrv_at_least("1.95", "1.95"));
        assert!(!msrv_at_least("1.94", "1.95"));
        assert!(msrv_at_least("1.95.0", "1.95"));
    }
}
