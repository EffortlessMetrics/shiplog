//! `cargo xtask check-policy-schemas`
//!
//! Validates every `policy/*.toml` file:
//!
//! - parses as TOML,
//! - has the well-formed common header (`schema_version`, `policy`, `owner`,
//!   `status`),
//! - `policy` field equals the file stem,
//! - `schema_version` is `1`,
//! - `status` is `"advisory"` or `"blocking"`.

use anyhow::{Result, bail};
use std::path::Path;

use crate::policy;

pub fn run(workspace_root: &Path) -> Result<()> {
    let dir = policy::policy_dir(workspace_root);
    let loaded = policy::load_all(&dir)?;

    println!(
        "Loaded {} policy ledger(s) from {}",
        loaded.len(),
        dir.display()
    );

    let mut findings: Vec<String> = Vec::new();
    for entry in &loaded {
        let stem = entry
            .path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();

        if entry.header.policy != stem {
            findings.push(format!(
                "{}: policy = {:?} does not match filename stem {:?}",
                entry.path.display(),
                entry.header.policy,
                stem
            ));
        }
        if entry.header.schema_version != 1 {
            findings.push(format!(
                "{}: schema_version = {} (only v1 supported)",
                entry.path.display(),
                entry.header.schema_version
            ));
        }
        if entry.header.status != "advisory" && entry.header.status != "blocking" {
            findings.push(format!(
                "{}: status = {:?} (expected \"advisory\" or \"blocking\")",
                entry.path.display(),
                entry.header.status
            ));
        }
        if entry.header.owner.trim().is_empty() {
            findings.push(format!("{}: owner is empty", entry.path.display()));
        }
    }

    if findings.is_empty() {
        println!("All policy ledgers have well-formed common headers.");
        Ok(())
    } else {
        for f in &findings {
            eprintln!("- {f}");
        }
        bail!("{} policy schema finding(s)", findings.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn fixture(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempdir().expect("tempdir");
        let policy = dir.path().join("policy");
        fs::create_dir_all(&policy).expect("create policy dir");
        for (name, content) in files {
            fs::write(policy.join(name), content).expect("write fixture");
        }
        dir
    }

    #[test]
    fn passes_for_well_formed_ledger() {
        let dir = fixture(&[(
            "ci-budget.toml",
            "schema_version = 1\npolicy = \"ci-budget\"\nowner = \"x\"\nstatus = \"advisory\"\n",
        )]);
        run(dir.path()).expect("should pass");
    }

    #[test]
    fn fails_when_policy_field_mismatches_filename() {
        let dir = fixture(&[(
            "ci-budget.toml",
            "schema_version = 1\npolicy = \"ci-not-budget\"\nowner = \"x\"\nstatus = \"advisory\"\n",
        )]);
        let err = run(dir.path()).expect_err("policy/filename mismatch should fail");
        assert!(format!("{err:#}").contains("policy schema finding"));
    }

    #[test]
    fn fails_when_status_is_invalid() {
        let dir = fixture(&[(
            "ci-budget.toml",
            "schema_version = 1\npolicy = \"ci-budget\"\nowner = \"x\"\nstatus = \"skipped\"\n",
        )]);
        let err = run(dir.path()).expect_err("invalid status should fail");
        assert!(format!("{err:#}").contains("policy schema finding"));
    }

    #[test]
    fn fails_when_schema_version_is_not_one() {
        let dir = fixture(&[(
            "ci-budget.toml",
            "schema_version = 2\npolicy = \"ci-budget\"\nowner = \"x\"\nstatus = \"advisory\"\n",
        )]);
        let err = run(dir.path()).expect_err("non-v1 schema should fail");
        assert!(format!("{err:#}").contains("policy schema finding"));
    }

    #[test]
    fn passes_for_empty_policy_dir() {
        let dir = tempdir().expect("tempdir");
        fs::create_dir_all(dir.path().join("policy")).expect("create policy dir");
        run(dir.path()).expect("empty policy dir is well-formed");
    }
}
