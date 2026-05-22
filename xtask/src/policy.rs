//! Shared policy-ledger loading and types.
//!
//! Loads every `policy/*.toml` file, validates the common header
//! (`schema_version`, `policy`, `owner`, `status`), and exposes the raw
//! TOML for per-ledger checkers.
//!
//! See [`docs/POLICY_ALLOWLISTS.md`](../../docs/POLICY_ALLOWLISTS.md) for
//! the schema contract.

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

/// Common header every policy ledger must declare.
#[derive(Debug, Clone, Deserialize)]
pub struct PolicyHeader {
    pub schema_version: u32,
    pub policy: String,
    pub owner: String,
    pub status: String,
}

/// One loaded policy ledger.
#[derive(Debug)]
pub struct LoadedPolicy {
    pub(crate) path: PathBuf,
    pub(crate) header: PolicyHeader,
    pub(crate) raw: toml::Table,
}

/// Detect the workspace root by walking up from `CARGO_MANIFEST_DIR` looking
/// for the first directory whose `Cargo.toml` declares `[workspace]`.
pub fn detect_workspace_root() -> Result<PathBuf> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .context("CARGO_MANIFEST_DIR not set; run via `cargo` or pass --workspace-root")?;
    let start = PathBuf::from(manifest_dir);
    let mut cursor: Option<&Path> = Some(start.as_path());
    while let Some(dir) = cursor {
        let candidate = dir.join("Cargo.toml");
        if candidate.is_file() {
            let text = fs::read_to_string(&candidate)
                .with_context(|| format!("read {}", candidate.display()))?;
            if text.contains("[workspace]") {
                return Ok(dir.to_path_buf());
            }
        }
        cursor = dir.parent();
    }
    Err(anyhow!(
        "could not detect workspace root from manifest dir {}",
        start.display()
    ))
}

/// Path to the `policy/` directory under a given workspace root.
pub fn policy_dir(workspace_root: &Path) -> PathBuf {
    workspace_root.join("policy")
}

/// Load every `*.toml` file under `policy_dir`, validating the common header.
pub fn load_all(policy_dir: &Path) -> Result<Vec<LoadedPolicy>> {
    let mut out = Vec::new();
    let entries = fs::read_dir(policy_dir)
        .with_context(|| format!("read policy dir: {}", policy_dir.display()))?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let raw: toml::Table =
            toml::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
        let header: PolicyHeader = toml::from_str(&text)
            .with_context(|| format!("invalid common header in {}", path.display()))?;
        out.push(LoadedPolicy { path, header, raw });
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_policy_dir(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempdir().expect("tempdir");
        let policy = dir.path().join("policy");
        fs::create_dir_all(&policy).expect("create policy dir");
        for (name, content) in files {
            fs::write(policy.join(name), content).expect("write fixture");
        }
        dir
    }

    #[test]
    fn loads_well_formed_policy_files() {
        let dir = make_policy_dir(&[
            (
                "ci-budget.toml",
                "schema_version = 1\npolicy = \"ci-budget\"\nowner = \"x\"\nstatus = \"advisory\"\n",
            ),
            (
                "ci-lanes.toml",
                "schema_version = 1\npolicy = \"ci-lanes\"\nowner = \"x\"\nstatus = \"advisory\"\n",
            ),
        ]);
        let loaded = load_all(&dir.path().join("policy")).expect("load_all");
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].header.policy, "ci-budget");
        assert_eq!(loaded[1].header.policy, "ci-lanes");
        assert!(loaded.iter().all(|p| p.header.schema_version == 1));
    }

    #[test]
    fn rejects_missing_common_header_field() {
        let dir = make_policy_dir(&[(
            "broken.toml",
            "schema_version = 1\npolicy = \"broken\"\nstatus = \"advisory\"\n",
        )]);
        let err = load_all(&dir.path().join("policy")).expect_err("missing owner should fail");
        assert!(format!("{err:#}").contains("invalid common header"));
    }

    #[test]
    fn ignores_non_toml_files() {
        let dir = make_policy_dir(&[
            ("README.md", "# orientation\n"),
            (
                "ci-budget.toml",
                "schema_version = 1\npolicy = \"ci-budget\"\nowner = \"x\"\nstatus = \"advisory\"\n",
            ),
        ]);
        let loaded = load_all(&dir.path().join("policy")).expect("load_all");
        assert_eq!(loaded.len(), 1);
    }

    #[test]
    fn fails_on_unparseable_toml() {
        let dir = make_policy_dir(&[("garbage.toml", "this is = not = toml\n")]);
        let err = load_all(&dir.path().join("policy")).expect_err("garbage should fail to parse");
        assert!(format!("{err:#}").contains("parse"));
    }
}
