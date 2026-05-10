//! `cargo xtask package-version`
//!
//! Verifies workspace package version alignment. Currently delegates to
//! `scripts/package-version-audit.sh` until Rust parity is proven.

use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::{Command, Stdio};

pub fn run(workspace_root: &Path) -> Result<()> {
    let script = workspace_root
        .join("scripts")
        .join("package-version-audit.sh");
    if !script.is_file() {
        bail!(
            "missing release script: {}; expected the package-version audit shell script",
            script.display()
        );
    }
    if cfg!(windows) {
        bail!(
            "cargo xtask package-version requires bash + python3; not available on Windows. \
             Run `bash {}` from a bash-capable shell (WSL or Git Bash) instead.",
            script.display()
        );
    }
    let status = Command::new("bash")
        .arg(&script)
        .current_dir(workspace_root)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("invoke {}", script.display()))?;
    if status.success() {
        Ok(())
    } else {
        bail!(
            "{} exited with {}",
            script.display(),
            status
                .code()
                .map_or_else(|| "signal".to_string(), |c| c.to_string())
        )
    }
}
