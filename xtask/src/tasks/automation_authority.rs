use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_yaml::Value as Yaml;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RepositoryRole {
    Swarm,
    Source,
}

impl RepositoryRole {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "swarm" => Ok(Self::Swarm),
            "source" => Ok(Self::Source),
            other => bail!("repository role must be 'swarm' or 'source', got {other:?}"),
        }
    }
}

#[derive(Debug, Deserialize)]
struct Policy {
    schema_version: u32,
    policy: String,
    status: String,
    repository_role: RepositoryRole,
    rule: Vec<Rule>,
}

#[derive(Debug, Deserialize)]
struct Rule {
    automation: Automation,
    swarm: Effect,
    source: Effect,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "kebab-case")]
enum Automation {
    DependencyUpdates,
    SecurityRemediation,
    ScheduledSecurity,
    ReviewBots,
    Promotion,
    ReleaseExecution,
    EmergencyHotfix,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum Effect {
    ProductPr,
    VerificationOnly,
    CheckArtifactOrHandoff,
    ReviewComment,
    PrepareSourcePr,
    MergeCheckpoint,
    Forbidden,
    ExplicitlyAuthorized,
    AuthorizedOnlyThenBackport,
}

pub fn run(workspace_root: &Path, role: RepositoryRole) -> Result<()> {
    let role = ci_bound_role(
        role,
        std::env::var("GITHUB_ACTIONS").ok().as_deref(),
        std::env::var("GITHUB_REPOSITORY").ok().as_deref(),
    )?;
    let findings = inspect(workspace_root, role)?;
    if findings.is_empty() {
        println!("check-automation-authority ({role:?}): no findings");
        return Ok(());
    }
    for finding in &findings {
        eprintln!("automation-authority: {finding}");
    }
    bail!(
        "check-automation-authority found {} issue(s)",
        findings.len()
    )
}

pub fn run_pinned(workspace_root: &Path) -> Result<()> {
    let path = workspace_root.join("policy/automation-authority.toml");
    let policy: Policy = toml::from_str(
        &fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?,
    )
    .with_context(|| format!("parse {}", path.display()))?;
    run(workspace_root, policy.repository_role)
}

fn ci_bound_role(
    requested: RepositoryRole,
    github_actions: Option<&str>,
    repository: Option<&str>,
) -> Result<RepositoryRole> {
    if github_actions != Some("true") {
        return Ok(requested);
    }
    let repository = repository.context("GITHUB_REPOSITORY is required in GitHub Actions")?;
    let expected = match repository {
        "EffortlessMetrics/shiplog-swarm" => RepositoryRole::Swarm,
        "EffortlessMetrics/shiplog" => RepositoryRole::Source,
        other => bail!("untrusted GitHub Actions repository identity {other:?}"),
    };
    if requested != expected {
        bail!(
            "requested/policy role {requested:?} does not match immutable GitHub repository identity {repository:?} ({expected:?})"
        );
    }
    Ok(expected)
}

fn inspect(workspace_root: &Path, role: RepositoryRole) -> Result<Vec<String>> {
    let policy_path = workspace_root.join("policy/automation-authority.toml");
    let policy: Policy = toml::from_str(
        &fs::read_to_string(&policy_path)
            .with_context(|| format!("read {}", policy_path.display()))?,
    )
    .with_context(|| format!("parse {}", policy_path.display()))?;
    let mut findings = validate_policy(&policy);
    if policy.repository_role != role {
        findings.push(format!(
            "requested role {role:?} does not match trusted policy role {:?}",
            policy.repository_role
        ));
    }

    let dependabot_path = workspace_root.join(".github/dependabot.yml");
    let dependabot_text = fs::read_to_string(&dependabot_path)
        .with_context(|| format!("read {}", dependabot_path.display()))?;
    let dependabot: Yaml = serde_yaml::from_str(&dependabot_text)
        .with_context(|| format!("parse YAML {}", dependabot_path.display()))?;
    let updates = yaml_get(&dependabot, "updates").and_then(Yaml::as_sequence);
    match role {
        RepositoryRole::Swarm if updates.is_none_or(Vec::is_empty) => findings
            .push("swarm Dependabot must retain authoritative product update entries".to_string()),
        RepositoryRole::Source if updates.is_none_or(|entries| !entries.is_empty()) => {
            findings.push("source Dependabot must use an empty updates list".to_string())
        }
        _ => {}
    }

    let workflows = workspace_root.join(".github/workflows");
    for entry in
        fs::read_dir(&workflows).with_context(|| format!("read {}", workflows.display()))?
    {
        let path = entry
            .with_context(|| format!("read directory entry in {}", workflows.display()))?
            .path();
        if matches!(
            path.extension().and_then(|value| value.to_str()),
            Some("yml" | "yaml")
        ) {
            inspect_workflow(&path, role, &mut findings)?;
        }
    }
    Ok(findings)
}

fn validate_policy(policy: &Policy) -> Vec<String> {
    let mut findings = Vec::new();
    if policy.schema_version != 1 || policy.policy != "automation-authority" {
        findings.push("automation authority policy header is invalid".to_string());
    }
    if policy.status != "blocking" {
        findings.push("automation authority policy must be blocking".to_string());
    }
    let expected = [
        (
            Automation::DependencyUpdates,
            Effect::ProductPr,
            Effect::VerificationOnly,
        ),
        (
            Automation::SecurityRemediation,
            Effect::ProductPr,
            Effect::VerificationOnly,
        ),
        (
            Automation::ScheduledSecurity,
            Effect::ProductPr,
            Effect::CheckArtifactOrHandoff,
        ),
        (
            Automation::ReviewBots,
            Effect::ReviewComment,
            Effect::ReviewComment,
        ),
        (
            Automation::Promotion,
            Effect::PrepareSourcePr,
            Effect::MergeCheckpoint,
        ),
        (
            Automation::ReleaseExecution,
            Effect::Forbidden,
            Effect::ExplicitlyAuthorized,
        ),
        (
            Automation::EmergencyHotfix,
            Effect::ProductPr,
            Effect::AuthorizedOnlyThenBackport,
        ),
    ];
    let mut seen = BTreeSet::new();
    for rule in &policy.rule {
        if !seen.insert(rule.automation) {
            findings.push(format!("duplicate automation rule {:?}", rule.automation));
        }
    }
    for (automation, swarm, source) in expected {
        let matching: Vec<_> = policy
            .rule
            .iter()
            .filter(|rule| rule.automation == automation)
            .collect();
        if matching.is_empty() {
            findings.push(format!("missing automation rule {automation:?}"));
        } else if let Some(rule) = matching
            .iter()
            .find(|rule| rule.swarm != swarm || rule.source != source)
        {
            findings.push(format!(
                "automation rule {automation:?} contradicts required effects: swarm={:?}, source={:?}",
                rule.swarm, rule.source
            ));
        }
    }
    findings
}

fn inspect_workflow(path: &Path, role: RepositoryRole, findings: &mut Vec<String>) -> Result<()> {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("<unknown>");
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let yaml: Yaml = serde_yaml::from_str(&text)
        .with_context(|| format!("parse workflow YAML {}", path.display()))?;
    let top_permissions = yaml_get(&yaml, "permissions");
    let jobs = yaml_get(&yaml, "jobs")
        .and_then(Yaml::as_mapping)
        .with_context(|| format!("workflow jobs must be a mapping in {}", path.display()))?;
    for (job_name, job) in jobs {
        let job_name = job_name.as_str().unwrap_or("<unknown>");
        let effective = yaml_get(job, "permissions").or(top_permissions);
        let contents = permission(effective, "contents");
        let source_writer =
            name == "release.yml" && matches!(job_name, "create-release" | "upload-assets");
        match role {
            RepositoryRole::Swarm if name == "release.yml" && contents != Some("read") => {
                findings.push(format!(
                    "swarm release job {job_name:?} must have effective contents: read"
                ));
            }
            RepositoryRole::Source if contents.is_none() => findings.push(format!(
                "source workflow {name:?} job {job_name:?} omits effective contents permission"
            )),
            RepositoryRole::Source if contents == Some("write") && !source_writer => {
                findings.push(format!(
                    "source routine workflow {name:?} job {job_name:?} enables contents writes"
                ));
            }
            RepositoryRole::Source if source_writer && contents != Some("write") => {
                findings.push(format!(
                    "source release authority job {job_name:?} must declare contents: write"
                ));
            }
            _ => {}
        }
        if role == RepositoryRole::Source {
            for scope in write_scopes(effective) {
                if !(source_writer && scope == "contents") {
                    findings.push(format!(
                        "source workflow {name:?} job {job_name:?} enables forbidden {scope}: write"
                    ));
                }
            }
        }
        let mut strings = Vec::new();
        collect_strings(job, &mut strings);
        for value in strings {
            let mutation = mutation_kind(value);
            let allowed = match mutation {
                None => true,
                Some(MutationKind::ReleaseOperation) => {
                    role == RepositoryRole::Source && source_writer
                }
                Some(MutationKind::AlternateCredentialOrMutation) => {
                    role == RepositoryRole::Swarm && name != "release.yml"
                }
            };
            if !allowed {
                findings.push(format!(
                    "workflow {name:?} job {job_name:?} contains forbidden mutation path {value:?}"
                ));
            }
        }
    }
    Ok(())
}

fn yaml_get<'a>(value: &'a Yaml, key: &str) -> Option<&'a Yaml> {
    value.as_mapping()?.get(Yaml::String(key.to_string()))
}

fn permission<'a>(permissions: Option<&'a Yaml>, key: &str) -> Option<&'a str> {
    let permissions = permissions?;
    match permissions {
        Yaml::Mapping(_) => yaml_get(permissions, key)?.as_str(),
        Yaml::String(value) if value == "read-all" => Some("read"),
        Yaml::String(value) if value == "write-all" => Some("write"),
        _ => None,
    }
}

fn write_scopes(permissions: Option<&Yaml>) -> Vec<String> {
    match permissions {
        Some(Yaml::String(value)) if value == "write-all" => vec!["write-all".to_string()],
        Some(Yaml::Mapping(values)) => values
            .iter()
            .filter(|(_, value)| value.as_str() == Some("write"))
            .map(|(key, _)| key.as_str().unwrap_or("<non-string>").to_string())
            .collect(),
        _ => Vec::new(),
    }
}

fn collect_strings<'a>(value: &'a Yaml, output: &mut Vec<&'a str>) {
    match value {
        Yaml::String(value) => output.push(value),
        Yaml::Sequence(values) => values
            .iter()
            .for_each(|value| collect_strings(value, output)),
        Yaml::Mapping(values) => values.iter().for_each(|(key, value)| {
            collect_strings(key, output);
            collect_strings(value, output);
        }),
        _ => {}
    }
}

#[derive(Clone, Copy)]
enum MutationKind {
    ReleaseOperation,
    AlternateCredentialOrMutation,
}

fn mutation_kind(value: &str) -> Option<MutationKind> {
    let lower = value.to_ascii_lowercase();
    if ["softprops/action-gh-release", "gh release create"]
        .iter()
        .any(|marker| lower.contains(marker))
    {
        return Some(MutationKind::ReleaseOperation);
    }
    [
        "git push",
        "gh pr create",
        "create-pull-request",
        "create-github-app-token",
        "cargo publish",
        "personal_access_token",
        "app_token",
        "pat_token",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
    .then_some(MutationKind::AlternateCredentialOrMutation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::{Result, ensure};
    use tempfile::tempdir;

    fn fixture(role: RepositoryRole, source_mutation: bool) -> Result<tempfile::TempDir> {
        let dir = tempdir()?;
        fs::create_dir_all(dir.path().join("policy"))?;
        fs::create_dir_all(dir.path().join(".github/workflows"))?;
        let role_name = if role == RepositoryRole::Source {
            "source"
        } else {
            "swarm"
        };
        let policy = include_str!("../../../policy/automation-authority.toml").replace(
            "repository_role = \"swarm\"",
            &format!("repository_role = \"{role_name}\""),
        );
        fs::write(dir.path().join("policy/automation-authority.toml"), policy)?;
        let updates = if role == RepositoryRole::Source {
            "updates: []\n"
        } else {
            "updates:\n  - package-ecosystem: cargo\n"
        };
        fs::write(dir.path().join(".github/dependabot.yml"), updates)?;
        let permission = if source_mutation { "write" } else { "read" };
        for name in ["droid-security-scan.yml", "security.yml"] {
            fs::write(
                dir.path().join(".github/workflows").join(name),
                format!(
                    "on:\n  schedule:\n    - cron: '0 0 * * 0'\npermissions:\n  contents: {permission}\njobs:\n  verify:\n    runs-on: ubuntu-latest\n    steps:\n      - run: echo verify\n"
                ),
            )?;
        }
        for name in [
            "bdd-smoke.yml",
            "bdd-testing.yml",
            "ci.yml",
            "fuzz-smoke.yml",
            "fuzzing.yml",
            "mutation-testing.yml",
            "property-smoke.yml",
            "property-testing.yml",
        ] {
            fs::write(
                dir.path().join(".github/workflows").join(name),
                "on: workflow_dispatch\npermissions:\n  contents: read\njobs:\n  verify:\n    runs-on: ubuntu-latest\n    steps:\n      - run: echo verify\n",
            )?;
        }
        let release_permission = if role == RepositoryRole::Source {
            "write"
        } else {
            "read"
        };
        fs::write(
            dir.path().join(".github/workflows/release.yml"),
            format!(
                "on:\n  workflow_dispatch:\npermissions:\n  contents: read\njobs:\n  create-release:\n    permissions:\n      contents: {release_permission}\n  upload-assets:\n    permissions:\n      contents: {release_permission}\n"
            ),
        )?;
        Ok(dir)
    }

    #[test]
    fn accepts_swarm_authority() -> Result<()> {
        let dir = fixture(RepositoryRole::Swarm, true)?;
        ensure!(inspect(dir.path(), RepositoryRole::Swarm)?.is_empty());
        Ok(())
    }

    #[test]
    fn accepts_read_only_source_verification() -> Result<()> {
        let dir = fixture(RepositoryRole::Source, false)?;
        ensure!(inspect(dir.path(), RepositoryRole::Source)?.is_empty());
        Ok(())
    }

    #[test]
    fn rejects_source_dependabot_and_scheduled_writes() -> Result<()> {
        let dir = fixture(RepositoryRole::Source, true)?;
        fs::write(
            dir.path().join(".github/dependabot.yml"),
            "updates:\n  - package-ecosystem: cargo\n",
        )?;
        let findings = inspect(dir.path(), RepositoryRole::Source)?;
        ensure!(
            findings
                .iter()
                .any(|finding| finding.contains("Dependabot"))
        );
        ensure!(
            findings
                .iter()
                .any(|finding| finding.contains("contents writes"))
        );
        Ok(())
    }

    #[test]
    fn rejects_source_routine_contents_write_and_extra_release_grant() -> Result<()> {
        let dir = fixture(RepositoryRole::Source, false)?;
        fs::write(
            dir.path().join(".github/workflows/droid-review.yml"),
            "on:\n  pull_request:\npermissions:\n  contents: write\njobs:\n  review:\n    runs-on: ubuntu-latest\n    steps:\n      - run: echo review\n",
        )?;
        fs::write(
            dir.path().join(".github/workflows/release.yml"),
            "permissions:\n  contents: read\njobs:\n  create-release:\n    permissions:\n      contents: write\n  upload-assets:\n    permissions:\n      contents: write\n  extra-writer:\n    permissions:\n      contents: write\n",
        )?;

        let findings = inspect(dir.path(), RepositoryRole::Source)?;

        ensure!(
            findings
                .iter()
                .any(|finding| finding.contains("routine workflow"))
        );
        ensure!(
            findings
                .iter()
                .any(|finding| finding.contains("extra-writer"))
        );
        Ok(())
    }

    #[test]
    fn rejects_wrong_requested_role() -> Result<()> {
        let dir = fixture(RepositoryRole::Swarm, false)?;
        let findings = inspect(dir.path(), RepositoryRole::Source)?;
        ensure!(
            findings
                .iter()
                .any(|finding| finding.contains("trusted policy role"))
        );
        Ok(())
    }

    #[test]
    fn rejects_duplicate_missing_and_contradictory_matrix_rows() -> Result<()> {
        let mut policy: Policy =
            toml::from_str(include_str!("../../../policy/automation-authority.toml"))?;
        policy
            .rule
            .retain(|rule| rule.automation != Automation::ReviewBots);
        policy.rule.push(Rule {
            automation: Automation::ReleaseExecution,
            swarm: Effect::ProductPr,
            source: Effect::ProductPr,
        });
        let findings = validate_policy(&policy);
        ensure!(findings.iter().any(|finding| finding.contains("duplicate")));
        ensure!(
            findings
                .iter()
                .any(|finding| finding.contains("ReviewBots"))
        );
        ensure!(
            findings
                .iter()
                .any(|finding| finding.contains("contradicts"))
        );
        Ok(())
    }

    #[test]
    fn rejects_unknown_matrix_effect() -> Result<()> {
        let text = include_str!("../../../policy/automation-authority.toml")
            .replace("swarm = \"product-pr\"", "swarm = \"surprise-writer\"");
        ensure!(toml::from_str::<Policy>(&text).is_err());
        Ok(())
    }

    #[test]
    fn rejects_source_mutation_commands_and_credentials() -> Result<()> {
        let dir = fixture(RepositoryRole::Source, false)?;
        fs::write(
            dir.path().join(".github/workflows/agent.yml"),
            "on:\n  schedule:\n    - cron: '0 0 * * 0'\npermissions:\n  contents: read\njobs:\n  mutate:\n    runs-on: ubuntu-latest\n    steps:\n      - run: git push origin HEAD:fix\n        env:\n          PAT_TOKEN: secret\n",
        )?;
        let findings = inspect(dir.path(), RepositoryRole::Source)?;
        ensure!(
            findings
                .iter()
                .filter(|finding| finding.contains("forbidden mutation path"))
                .count()
                >= 2
        );
        Ok(())
    }

    #[test]
    fn ci_repository_identity_cannot_be_overridden() -> Result<()> {
        ensure!(
            ci_bound_role(
                RepositoryRole::Swarm,
                Some("true"),
                Some("EffortlessMetrics/shiplog-swarm")
            )? == RepositoryRole::Swarm
        );
        ensure!(
            ci_bound_role(
                RepositoryRole::Source,
                Some("true"),
                Some("EffortlessMetrics/shiplog-swarm")
            )
            .is_err()
        );
        ensure!(
            ci_bound_role(
                RepositoryRole::Swarm,
                Some("true"),
                Some("fork/shiplog-swarm")
            )
            .is_err()
        );
        Ok(())
    }

    #[test]
    fn source_rejects_every_non_contents_write_scope() -> Result<()> {
        let dir = fixture(RepositoryRole::Source, false)?;
        fs::write(
            dir.path().join(".github/workflows/agent.yml"),
            "on: workflow_dispatch\npermissions:\n  contents: read\n  issues: write\n  id-token: write\njobs:\n  mutate:\n    runs-on: ubuntu-latest\n    steps:\n      - run: echo no\n",
        )?;
        let findings = inspect(dir.path(), RepositoryRole::Source)?;
        ensure!(findings.iter().any(|finding| finding.contains("issues")));
        ensure!(findings.iter().any(|finding| finding.contains("id-token")));
        Ok(())
    }

    #[test]
    fn source_release_writer_rejects_alternate_mutation_paths() -> Result<()> {
        let dir = fixture(RepositoryRole::Source, false)?;
        fs::write(
            dir.path().join(".github/workflows/release.yml"),
            "on: workflow_dispatch\npermissions:\n  contents: read\njobs:\n  create-release:\n    permissions:\n      contents: write\n    steps:\n      - uses: softprops/action-gh-release@pinned\n      - run: git push origin HEAD:main\n        env:\n          PAT_TOKEN: secret\n  upload-assets:\n    permissions:\n      contents: write\n",
        )?;
        let findings = inspect(dir.path(), RepositoryRole::Source)?;
        ensure!(
            findings
                .iter()
                .filter(|finding| finding.contains("forbidden mutation path"))
                .count()
                >= 2
        );
        ensure!(
            !findings
                .iter()
                .any(|finding| finding.contains("softprops/action-gh-release"))
        );
        Ok(())
    }
}
