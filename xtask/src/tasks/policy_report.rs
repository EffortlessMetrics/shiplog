//! `cargo xtask policy-report`
//!
//! Human summary of every policy ledger: file path, status, top-level
//! entry count.

use anyhow::Result;
use std::path::Path;

use crate::policy;

pub fn run(workspace_root: &Path) -> Result<()> {
    let dir = policy::policy_dir(workspace_root);
    let loaded = policy::load_all(&dir)?;

    println!(
        "Policy report: {} ledger(s) under {}",
        loaded.len(),
        dir.display()
    );
    println!();

    println!(
        "{:<40} {:<10} {:<22} {:<8}",
        "ledger", "status", "owner", "entries"
    );
    println!("{}", "-".repeat(82));

    for entry in &loaded {
        let entry_count = count_top_level_array_entries(&entry.raw);
        let name = entry
            .path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        println!(
            "{:<40} {:<10} {:<22} {:<8}",
            name, entry.header.status, entry.header.owner, entry_count
        );
    }

    println!();
    println!("(entry count is the sum of top-level array-of-table sizes; comments not counted)");
    Ok(())
}

fn count_top_level_array_entries(table: &toml::Table) -> usize {
    table
        .values()
        .filter_map(|v| v.as_array())
        .map(|a| a.len())
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn counts_array_entries() {
        let dir = tempdir().expect("tempdir");
        let policy = dir.path().join("policy");
        fs::create_dir_all(&policy).expect("create policy dir");
        fs::write(
            policy.join("clippy-debt.toml"),
            "schema_version = 1\npolicy = \"clippy-debt\"\nowner = \"x\"\nstatus = \"advisory\"\n\n[[debt]]\nid = \"a\"\n\n[[debt]]\nid = \"b\"\n",
        )
        .expect("write fixture");
        run(dir.path()).expect("should produce a report");
    }

    #[test]
    fn empty_policy_dir_is_ok() {
        let dir = tempdir().expect("tempdir");
        fs::create_dir_all(dir.path().join("policy")).expect("create policy dir");
        run(dir.path()).expect("empty report is fine");
    }
}
