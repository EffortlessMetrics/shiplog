use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

const BADGE_ENDPOINT_DIR: &str = "badges";
const BADGE_ENDPOINT_TARGET_DIR: &str = "target/xtask/badges";
const TEST_EFFICIENCY_REPORT: &str = "target/ripr/reports/test-efficiency.json";

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ShieldsEndpointBadge {
    #[serde(rename = "schemaVersion")]
    schema_version: u8,
    label: String,
    message: String,
    color: String,
}

pub fn run(workspace_root: &Path, check: bool) -> Result<()> {
    let target_dir = workspace_root.join(BADGE_ENDPOINT_TARGET_DIR);
    std::fs::create_dir_all(&target_dir).context("create badge target directory")?;

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
    std::fs::create_dir_all(&committed_dir).context("create committed badge directory")?;
    std::fs::copy(
        target_dir.join("ripr-plus.json"),
        committed_dir.join("ripr-plus.json"),
    )
    .context("refresh committed ripr+ badge endpoint")?;

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
        .with_context(|| format!("run {ripr_bin} for repo-scoped ripr+ badge"))?;

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
    let report = workspace_root.join(TEST_EFFICIENCY_REPORT);
    if report.exists() {
        return Ok(());
    }

    let parent = report
        .parent()
        .ok_or_else(|| anyhow::anyhow!("test-efficiency report path has no parent"))?;
    std::fs::create_dir_all(parent).context("create ripr reports directory")?;
    std::fs::write(
        &report,
        concat!(
            "{\n",
            "  \"schema_version\": \"0.1\",\n",
            "  \"tests\": [],\n",
            "  \"metrics\": {\n",
            "    \"tests_scanned\": 0,\n",
            "    \"reason_counts\": {}\n",
            "  }\n",
            "}\n"
        ),
    )
    .context("write empty ripr test-efficiency report placeholder")?;
    Ok(())
}

pub fn validate_shields_badge(
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
    std::fs::write(path, format!("{json}\n")).with_context(|| format!("write {}", path.display()))
}

fn compare_files(committed: &Path, generated: &Path) -> Result<()> {
    let committed_text = read_file(committed)?;
    let generated_text = read_file(generated)?;
    if committed_text != generated_text {
        bail!(
            "badge endpoint drift: {} differs from {}; run `cargo xtask badges`",
            committed.display(),
            generated.display()
        );
    }
    Ok(())
}

fn read_file(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))
}

#[allow(dead_code)]
fn endpoint_path(workspace_root: &Path, name: &str) -> PathBuf {
    workspace_root.join(BADGE_ENDPOINT_DIR).join(name)
}

#[cfg(test)]
mod tests {
    use super::{ShieldsEndpointBadge, validate_shields_badge};

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
    fn scanner_safe_badge_shape_is_stable() {
        let badge = ShieldsEndpointBadge {
            schema_version: 1,
            label: "fixtures".to_string(),
            message: "scanner-safe".to_string(),
            color: "brightgreen".to_string(),
        };

        validate_shields_badge(&badge, Some("fixtures")).unwrap();
    }
}
