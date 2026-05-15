use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::Command;

const PR_DIR: &str = "target/ripr/pr";
const REVIEW_DIR: &str = "target/ripr/review";

pub fn pr(workspace_root: &Path, check: bool, base: &str) -> Result<()> {
    let json = workspace_root.join(PR_DIR).join("repo-exposure.json");
    let markdown = workspace_root.join(PR_DIR).join("repo-exposure.md");

    if check {
        check_json(&json)?;
        check_nonempty(&markdown)?;
        println!("ripr-pr: output contract is intact");
        return Ok(());
    }

    std::fs::create_dir_all(workspace_root.join(PR_DIR)).context("create ripr PR output dir")?;
    let ripr_bin = ripr_bin();
    run_ripr_check(workspace_root, &ripr_bin, base, "repo-exposure-json", &json)?;
    run_ripr_check(
        workspace_root,
        &ripr_bin,
        base,
        "repo-exposure-md",
        &markdown,
    )?;
    println!(
        "ripr-pr: wrote {} and {}",
        json.display(),
        markdown.display()
    );
    Ok(())
}

pub fn review_comments(workspace_root: &Path, check: bool, base: &str, head: &str) -> Result<()> {
    let json = workspace_root.join(REVIEW_DIR).join("comments.json");
    let markdown = workspace_root.join(REVIEW_DIR).join("comments.md");

    if check {
        check_json(&json)?;
        check_nonempty(&markdown)?;
        println!("ripr-review-comments: output contract is intact");
        return Ok(());
    }

    std::fs::create_dir_all(workspace_root.join(REVIEW_DIR))
        .context("create ripr review output dir")?;
    let ripr_bin = ripr_bin();
    let output = Command::new(&ripr_bin)
        .arg("review-comments")
        .arg("--root")
        .arg(workspace_root)
        .arg("--base")
        .arg(base)
        .arg("--head")
        .arg(head)
        .arg("--out")
        .arg(&json)
        .current_dir(workspace_root)
        .output()
        .with_context(|| format!("run {ripr_bin} review-comments"))?;

    if !output.status.success() {
        bail!(
            "{ripr_bin} review-comments failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    println!(
        "ripr-review-comments: wrote {} and {}",
        json.display(),
        markdown.display()
    );
    Ok(())
}

fn run_ripr_check(
    workspace_root: &Path,
    ripr_bin: &str,
    base: &str,
    format: &str,
    output_path: &Path,
) -> Result<()> {
    let output = Command::new(ripr_bin)
        .arg("check")
        .arg("--root")
        .arg(workspace_root)
        .arg("--base")
        .arg(base)
        .arg("--format")
        .arg(format)
        .current_dir(workspace_root)
        .output()
        .with_context(|| format!("run {ripr_bin} check --format {format}"))?;

    if !output.status.success() {
        bail!(
            "{ripr_bin} check --format {format} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    std::fs::write(output_path, output.stdout)
        .with_context(|| format!("write {}", output_path.display()))
}

fn ripr_bin() -> String {
    std::env::var("RIPR_BIN").unwrap_or_else(|_| "ripr".to_string())
}

fn check_json(path: &Path) -> Result<()> {
    let text = check_nonempty(path)?;
    let value: serde_json::Value =
        serde_json::from_str(&text).with_context(|| format!("parse {} as JSON", path.display()))?;
    if !value.is_object() {
        bail!("{} must contain a JSON object", path.display());
    }
    Ok(())
}

fn check_nonempty(path: &Path) -> Result<String> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    if text.trim().is_empty() {
        bail!("{} is empty", path.display());
    }
    Ok(text)
}
