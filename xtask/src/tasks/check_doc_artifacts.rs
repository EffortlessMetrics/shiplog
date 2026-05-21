//! `cargo xtask check-doc-artifacts`
//!
//! Validates `policy/doc-artifacts.toml`, the source-of-truth ledger for
//! proposal/spec/ADR/plan artifacts.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path};

#[derive(Debug, Deserialize)]
struct DocArtifactsPolicy {
    #[serde(default)]
    artifact: Vec<Artifact>,
}

#[derive(Debug, Deserialize)]
struct Artifact {
    id: String,
    kind: String,
    path: String,
    status: String,
    owner: String,
    #[serde(default)]
    linked_proposal: Option<String>,
    #[serde(default)]
    linked_spec: Option<String>,
    #[serde(default)]
    linked_specs: Vec<String>,
    #[serde(default)]
    linked_adr: Option<String>,
    #[serde(default)]
    linked_adrs: Vec<String>,
    #[serde(default)]
    linked_plan: Option<String>,
    #[serde(default)]
    superseded_by: Option<String>,
    #[serde(default)]
    standalone_reason: Option<String>,
}

pub fn run(workspace_root: &Path) -> Result<()> {
    let path = workspace_root.join("policy").join("doc-artifacts.toml");
    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let policy: DocArtifactsPolicy =
        toml::from_str(&text).with_context(|| format!("parse {}", path.display()))?;

    let mut findings = Vec::new();
    let mut seen = BTreeSet::new();
    let mut by_id: BTreeMap<&str, &Artifact> = BTreeMap::new();

    for artifact in &policy.artifact {
        if !seen.insert(artifact.id.as_str()) {
            findings.push(format!(
                "[doc-artifact-duplicate-id] duplicate artifact id {:?}",
                artifact.id
            ));
            continue;
        }
        by_id.insert(artifact.id.as_str(), artifact);
    }

    for artifact in &policy.artifact {
        validate_artifact_shape(artifact, workspace_root, &by_id, &mut findings);
    }

    if findings.is_empty() {
        println!(
            "check-doc-artifacts: {} artifact(s) linked and valid.",
            policy.artifact.len()
        );
        return Ok(());
    }

    for finding in &findings {
        eprintln!("  {finding}");
    }
    Err(anyhow::anyhow!(
        "check-doc-artifacts: {} finding(s)",
        findings.len()
    ))
}

fn validate_artifact_shape(
    artifact: &Artifact,
    workspace_root: &Path,
    by_id: &BTreeMap<&str, &Artifact>,
    findings: &mut Vec<String>,
) {
    if artifact.id.trim().is_empty() {
        findings.push("[doc-artifact-empty-id] artifact id is empty".to_string());
    }
    if artifact.owner.trim().is_empty() {
        findings.push(format!(
            "[doc-artifact-empty-owner] {} owner is empty",
            artifact.id
        ));
    }
    if !valid_kind(&artifact.kind) {
        findings.push(format!(
            "[doc-artifact-invalid-kind] {} kind {:?} is not proposal/spec/adr/plan",
            artifact.id, artifact.kind
        ));
    }
    if !valid_status(&artifact.status) {
        findings.push(format!(
            "[doc-artifact-invalid-status] {} status {:?} is not proposed/accepted/active/done/superseded/archived",
            artifact.id, artifact.status
        ));
    }
    if !id_matches_kind(&artifact.id, &artifact.kind) {
        findings.push(format!(
            "[doc-artifact-id-kind-mismatch] {} does not match kind {:?}",
            artifact.id, artifact.kind
        ));
    }
    if artifact.path.trim().is_empty() {
        findings.push(format!(
            "[doc-artifact-empty-path] {} path is empty",
            artifact.id
        ));
        return;
    }
    if !relative_repo_path(&artifact.path) {
        findings.push(format!(
            "[doc-artifact-unsafe-path] {} path {:?} must be a relative repo path",
            artifact.id, artifact.path
        ));
        return;
    }
    if !path_matches_kind(&artifact.path, &artifact.kind) {
        findings.push(format!(
            "[doc-artifact-path-kind-mismatch] {} kind {:?} does not match path {:?}",
            artifact.id, artifact.kind, artifact.path
        ));
    }

    let artifact_path = workspace_root.join(&artifact.path);
    match fs::read_to_string(&artifact_path) {
        Ok(contents) => {
            if !contents.contains(&artifact.id) {
                findings.push(format!(
                    "[doc-artifact-id-not-in-file] {} is not mentioned in {}",
                    artifact.id, artifact.path
                ));
            }
        }
        Err(err) => findings.push(format!(
            "[doc-artifact-missing-file] {} path {:?} is not readable: {err}",
            artifact.id, artifact.path
        )),
    }

    validate_link(
        &artifact.id,
        "linked_proposal",
        artifact.linked_proposal.as_deref(),
        "proposal",
        by_id,
        findings,
    );
    validate_link(
        &artifact.id,
        "linked_spec",
        artifact.linked_spec.as_deref(),
        "spec",
        by_id,
        findings,
    );
    validate_link(
        &artifact.id,
        "linked_adr",
        artifact.linked_adr.as_deref(),
        "adr",
        by_id,
        findings,
    );
    validate_link(
        &artifact.id,
        "linked_plan",
        artifact.linked_plan.as_deref(),
        "plan",
        by_id,
        findings,
    );
    for linked in &artifact.linked_specs {
        validate_link(
            &artifact.id,
            "linked_specs",
            Some(linked.as_str()),
            "spec",
            by_id,
            findings,
        );
    }
    for linked in &artifact.linked_adrs {
        validate_link(
            &artifact.id,
            "linked_adrs",
            Some(linked.as_str()),
            "adr",
            by_id,
            findings,
        );
    }

    if artifact.status == "accepted"
        && artifact.kind == "spec"
        && artifact.linked_proposal.is_none()
        && artifact
            .standalone_reason
            .as_deref()
            .is_none_or(|reason| reason.trim().is_empty())
    {
        findings.push(format!(
            "[doc-artifact-accepted-spec-unlinked] {} is accepted but has no linked_proposal or standalone_reason",
            artifact.id
        ));
    }

    if artifact.status == "active"
        && artifact.kind == "plan"
        && artifact.linked_proposal.is_none()
        && artifact.linked_spec.is_none()
    {
        findings.push(format!(
            "[doc-artifact-active-plan-unlinked] {} is active but has no linked_proposal or linked_spec",
            artifact.id
        ));
    }

    if artifact.status == "superseded" && artifact.superseded_by.is_none() {
        findings.push(format!(
            "[doc-artifact-superseded-without-replacement] {} is superseded but has no superseded_by",
            artifact.id
        ));
    }
    validate_link(
        &artifact.id,
        "superseded_by",
        artifact.superseded_by.as_deref(),
        "any",
        by_id,
        findings,
    );
}

fn validate_link(
    source_id: &str,
    field: &str,
    linked_id: Option<&str>,
    expected_kind: &str,
    by_id: &BTreeMap<&str, &Artifact>,
    findings: &mut Vec<String>,
) {
    let Some(linked_id) = linked_id else {
        return;
    };
    match by_id.get(linked_id) {
        Some(target) if expected_kind == "any" || target.kind == expected_kind => {}
        Some(target) => findings.push(format!(
            "[doc-artifact-link-kind-mismatch] {source_id} {field} {linked_id:?} points to kind {:?}, expected {expected_kind:?}",
            target.kind
        )),
        None => findings.push(format!(
            "[doc-artifact-unresolved-link] {source_id} {field} references missing artifact {linked_id:?}"
        )),
    }
}

fn valid_kind(kind: &str) -> bool {
    matches!(kind, "proposal" | "spec" | "adr" | "plan")
}

fn valid_status(status: &str) -> bool {
    matches!(
        status,
        "proposed" | "accepted" | "active" | "done" | "superseded" | "archived"
    )
}

fn id_matches_kind(id: &str, kind: &str) -> bool {
    match kind {
        "proposal" => id.contains("-PROP-"),
        "spec" => id.contains("-SPEC-"),
        "adr" => id.contains("-ADR-"),
        "plan" => id.contains("-PLAN-"),
        _ => true,
    }
}

fn path_matches_kind(path: &str, kind: &str) -> bool {
    let path = path.replace('\\', "/");
    match kind {
        "proposal" => path.starts_with("docs/proposals/"),
        "spec" => path.starts_with("docs/specs/"),
        "adr" => path.starts_with("docs/adr/"),
        "plan" => path.starts_with("plans/"),
        _ => true,
    }
}

fn relative_repo_path(path: &str) -> bool {
    let path = Path::new(path);
    !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    fn fixture(policy_body: &str, files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        write(
            &dir.path().join("policy").join("doc-artifacts.toml"),
            policy_body,
        );
        for (path, body) in files {
            write(&dir.path().join(path), body);
        }
        dir
    }

    const HEADER: &str = r#"
schema_version = 1
policy = "doc-artifacts"
owner = "repo-infra"
status = "advisory"
"#;

    #[test]
    fn linked_artifacts_pass() {
        let policy = format!(
            r#"{HEADER}
[[artifact]]
id = "SHIPLOG-PROP-0001"
kind = "proposal"
path = "docs/proposals/SHIPLOG-PROP-0001-example.md"
status = "proposed"
owner = "repo-infra"

[[artifact]]
id = "SHIPLOG-SPEC-0001"
kind = "spec"
path = "docs/specs/SHIPLOG-SPEC-0001-example.md"
status = "accepted"
owner = "repo-infra"
linked_proposal = "SHIPLOG-PROP-0001"

[[artifact]]
id = "SHIPLOG-ADR-0001"
kind = "adr"
path = "docs/adr/SHIPLOG-ADR-0001-example.md"
status = "accepted"
owner = "repo-infra"
linked_proposal = "SHIPLOG-PROP-0001"
linked_specs = ["SHIPLOG-SPEC-0001"]

[[artifact]]
id = "SHIPLOG-PLAN-0001"
kind = "plan"
path = "plans/example/implementation-plan.md"
status = "active"
owner = "repo-infra"
linked_proposal = "SHIPLOG-PROP-0001"
linked_spec = "SHIPLOG-SPEC-0001"
linked_adr = "SHIPLOG-ADR-0001"
"#
        );
        let dir = fixture(
            &policy,
            &[
                (
                    "docs/proposals/SHIPLOG-PROP-0001-example.md",
                    "# SHIPLOG-PROP-0001\n",
                ),
                (
                    "docs/specs/SHIPLOG-SPEC-0001-example.md",
                    "# SHIPLOG-SPEC-0001\n",
                ),
                (
                    "docs/adr/SHIPLOG-ADR-0001-example.md",
                    "# SHIPLOG-ADR-0001\n",
                ),
                (
                    "plans/example/implementation-plan.md",
                    "Plan artifact: SHIPLOG-PLAN-0001\n",
                ),
            ],
        );
        run(dir.path()).expect("valid linked artifacts should pass");
    }

    #[test]
    fn missing_artifact_file_is_finding() {
        let policy = format!(
            r#"{HEADER}
[[artifact]]
id = "SHIPLOG-PROP-0001"
kind = "proposal"
path = "docs/proposals/SHIPLOG-PROP-0001-example.md"
status = "proposed"
owner = "repo-infra"
"#
        );
        let dir = fixture(&policy, &[]);
        let err = run(dir.path()).unwrap_err();
        assert!(err.to_string().contains("1 finding"));
    }

    #[test]
    fn duplicate_artifact_id_is_finding() {
        let policy = format!(
            r#"{HEADER}
[[artifact]]
id = "SHIPLOG-PROP-0001"
kind = "proposal"
path = "docs/proposals/SHIPLOG-PROP-0001-a.md"
status = "proposed"
owner = "repo-infra"

[[artifact]]
id = "SHIPLOG-PROP-0001"
kind = "proposal"
path = "docs/proposals/SHIPLOG-PROP-0001-b.md"
status = "proposed"
owner = "repo-infra"
"#
        );
        let dir = fixture(
            &policy,
            &[
                (
                    "docs/proposals/SHIPLOG-PROP-0001-a.md",
                    "# SHIPLOG-PROP-0001\n",
                ),
                (
                    "docs/proposals/SHIPLOG-PROP-0001-b.md",
                    "# SHIPLOG-PROP-0001\n",
                ),
            ],
        );
        let err = run(dir.path()).unwrap_err();
        assert!(err.to_string().contains("1 finding"));
    }

    #[test]
    fn unresolved_link_is_finding() {
        let policy = format!(
            r#"{HEADER}
[[artifact]]
id = "SHIPLOG-SPEC-0001"
kind = "spec"
path = "docs/specs/SHIPLOG-SPEC-0001-example.md"
status = "proposed"
owner = "repo-infra"
linked_proposal = "SHIPLOG-PROP-9999"
"#
        );
        let dir = fixture(
            &policy,
            &[(
                "docs/specs/SHIPLOG-SPEC-0001-example.md",
                "# SHIPLOG-SPEC-0001\n",
            )],
        );
        let err = run(dir.path()).unwrap_err();
        assert!(err.to_string().contains("1 finding"));
    }

    #[test]
    fn accepted_spec_without_proposal_or_standalone_reason_is_finding() {
        let policy = format!(
            r#"{HEADER}
[[artifact]]
id = "SHIPLOG-SPEC-0001"
kind = "spec"
path = "docs/specs/SHIPLOG-SPEC-0001-example.md"
status = "accepted"
owner = "repo-infra"
"#
        );
        let dir = fixture(
            &policy,
            &[(
                "docs/specs/SHIPLOG-SPEC-0001-example.md",
                "# SHIPLOG-SPEC-0001\n",
            )],
        );
        let err = run(dir.path()).unwrap_err();
        assert!(err.to_string().contains("1 finding"));
    }

    #[test]
    fn active_plan_without_proposal_or_spec_is_finding() {
        let policy = format!(
            r#"{HEADER}
[[artifact]]
id = "SHIPLOG-PLAN-0001"
kind = "plan"
path = "plans/example/implementation-plan.md"
status = "active"
owner = "repo-infra"
"#
        );
        let dir = fixture(
            &policy,
            &[(
                "plans/example/implementation-plan.md",
                "Plan artifact: SHIPLOG-PLAN-0001\n",
            )],
        );
        let err = run(dir.path()).unwrap_err();
        assert!(err.to_string().contains("1 finding"));
    }
}
