//! Generated public Shields endpoint badges.
//!
//! Public badge endpoints are repo-scoped. Diff-scoped evidence belongs in
//! PR summaries and artifacts under `target/`, never in committed badges.

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const BADGE_ENDPOINT_DIR: &str = "badges";
const BADGE_ENDPOINT_TARGET_DIR: &str = "target/xtask/badges";

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
struct ShieldsEndpointBadge {
    #[serde(rename = "schemaVersion")]
    schema_version: u8,
    label: String,
    message: String,
    color: String,
}

pub fn run(workspace_root: &Path, check: bool) -> Result<()> {
    let target_dir = workspace_root.join(BADGE_ENDPOINT_TARGET_DIR);
    fs::create_dir_all(&target_dir)
        .with_context(|| format!("create badge target dir {}", target_dir.display()))?;

    let ripr_plus = ripr_plus_badge(workspace_root)?;
    validate_shields_badge(&ripr_plus, Some("ripr+"))?;
    write_json_pretty(&target_dir.join("ripr-plus.json"), &ripr_plus)?;

    if check {
        let committed_dir = workspace_root.join(BADGE_ENDPOINT_DIR);
        compare_files(
            &committed_dir.join("ripr-plus.json"),
            &target_dir.join("ripr-plus.json"),
        )?;
        println!("badges: committed endpoints are current");
        return Ok(());
    }

    let committed_dir = workspace_root.join(BADGE_ENDPOINT_DIR);
    fs::create_dir_all(&committed_dir)
        .with_context(|| format!("create badge endpoint dir {}", committed_dir.display()))?;
    fs::copy(
        target_dir.join("ripr-plus.json"),
        committed_dir.join("ripr-plus.json"),
    )
    .context("copy generated ripr+ badge into badges/")?;

    println!("badges: refreshed public endpoint JSON under badges/");
    Ok(())
}

fn ripr_plus_badge(workspace_root: &Path) -> Result<ShieldsEndpointBadge> {
    ensure_test_efficiency_report(workspace_root)?;
    let ripr_bin = std::env::var("RIPR_BIN").unwrap_or_else(|_| "ripr".to_string());

    let output = Command::new(&ripr_bin)
        .arg("check")
        .arg("--root")
        .arg(workspace_root)
        .arg("--format")
        .arg("repo-badge-plus-shields")
        .current_dir(workspace_root)
        .output()
        .with_context(|| format!("invoke {ripr_bin} for repo-scoped ripr+ badge"))?;

    if !output.status.success() {
        bail!(
            "{ripr_bin} repo-badge-plus-shields failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    serde_json::from_slice(&output.stdout)
        .with_context(|| format!("{ripr_bin} emitted invalid Shields endpoint JSON"))
}

fn ensure_test_efficiency_report(workspace_root: &Path) -> Result<()> {
    let reports_dir = workspace_root.join("target/ripr/reports");
    fs::create_dir_all(&reports_dir)
        .with_context(|| format!("create RIPR reports dir {}", reports_dir.display()))?;
    let report = reports_dir.join("test-efficiency.json");
    if report.exists() {
        return Ok(());
    }

    // `ripr` badge-plus formats join repo exposure with a test-efficiency
    // report. shiplog does not yet have a richer test-efficiency xtask, so the
    // badge generator writes the minimal zero-finding report under target/.
    // Public committed badge JSON remains a Shields projection only.
    fs::write(
        &report,
        "{\n  \"schema_version\": \"0.1\",\n  \"tests\": [],\n  \"metrics\": {\n    \"tests_scanned\": 0\n  }\n}\n",
    )
    .with_context(|| format!("write minimal test-efficiency report {}", report.display()))
}

fn validate_shields_badge(
    badge: &ShieldsEndpointBadge,
    expected_label: Option<&str>,
) -> Result<()> {
    if badge.schema_version != 1 {
        bail!("badge `{}` has unsupported schemaVersion", badge.label);
    }

    if let Some(expected_label) = expected_label
        && badge.label != expected_label
    {
        bail!(
            "badge label drifted: got `{}`, expected `{expected_label}`",
            badge.label
        );
    }

    if badge.message.trim().is_empty() {
        bail!("badge `{}` has empty message", badge.label);
    }

    if badge.color.trim().is_empty() {
        bail!("badge `{}` has empty color", badge.label);
    }

    Ok(())
}

fn write_json_pretty(path: &Path, badge: &ShieldsEndpointBadge) -> Result<()> {
    let json = serde_json::to_string_pretty(badge).context("serialize Shields endpoint JSON")?;
    fs::write(path, format!("{json}\n"))
        .with_context(|| format!("write Shields endpoint JSON to {}", path.display()))
}

fn compare_files(committed: &Path, generated: &Path) -> Result<()> {
    let committed_bytes = fs::read(committed)
        .with_context(|| format!("read committed badge {}", committed.display()))?;
    let generated_bytes = fs::read(generated)
        .with_context(|| format!("read generated badge {}", generated.display()))?;
    if committed_bytes != generated_bytes {
        bail!(
            "badge endpoint drift: {} differs from generated {} (run `cargo xtask badges`)",
            committed.display(),
            generated.display()
        );
    }
    Ok(())
}

#[allow(dead_code)]
fn workspace_relative(path: &Path, workspace_root: &Path) -> PathBuf {
    path.strip_prefix(workspace_root)
        .unwrap_or(path)
        .to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ripr_plus_badge_shape_is_stable() {
        let badge = ShieldsEndpointBadge {
            schema_version: 1,
            label: "ripr+".to_string(),
            message: "0".to_string(),
            color: "brightgreen".to_string(),
        };

        validate_shields_badge(&badge, Some("ripr+")).unwrap();
    }

    #[test]
    fn rejects_empty_badge_message() {
        let badge = ShieldsEndpointBadge {
            schema_version: 1,
            label: "ripr+".to_string(),
            message: " ".to_string(),
            color: "brightgreen".to_string(),
        };

        assert!(validate_shields_badge(&badge, Some("ripr+")).is_err());
    }
}
