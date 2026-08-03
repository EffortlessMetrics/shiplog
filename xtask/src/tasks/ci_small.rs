//! `cargo xtask ci-small` - the canonical local and hosted small gate.

use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;
use std::process::Command;

const STEPS: &[&str] = &["format", "clippy", "tests", "no-panic", "authority", "diff"];

/// Run the exact required small-gate sequence, or one named step for CI log
/// granularity.
pub fn run(workspace_root: &Path, selected_step: Option<&str>) -> Result<()> {
    match selected_step {
        None => {
            for step in STEPS {
                run_step(workspace_root, step)?;
            }
        }
        Some(step) if STEPS.contains(&step) => run_step(workspace_root, step)?,
        Some(step) => bail!(
            "unknown ci-small step {step:?}; expected one of {}",
            STEPS.join(", ")
        ),
    }

    Ok(())
}

fn run_step(workspace_root: &Path, step: &str) -> Result<()> {
    let (program, args): (&str, &[&str]) = match step {
        "format" => ("cargo", &["fmt", "--all", "--", "--check"]),
        "clippy" => (
            "cargo",
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--all-features",
                "--locked",
                "--",
                "-D",
                "warnings",
            ],
        ),
        "tests" => (
            "cargo",
            &[
                "test",
                "--workspace",
                "--all-features",
                "--locked",
                "--",
                "--test-threads=4",
            ],
        ),
        "no-panic" => (
            "cargo",
            &[
                "xtask",
                "check-no-panic-family",
                "--mode",
                "blocking-allowlist",
            ],
        ),
        "authority" => {
            println!("==> check pinned automation authority");
            return crate::tasks::automation_authority::run_pinned(workspace_root);
        }
        "diff" => ("git", &["diff", "--check"]),
        _ => bail!("unknown ci-small step {step:?}"),
    };

    println!("==> {program} {}", args.join(" "));
    let isolated_target = if cfg!(windows) && step == "tests" {
        let path =
            std::env::temp_dir().join(format!("shiplog-ci-small-target-{}", std::process::id()));
        fs::create_dir_all(&path)
            .with_context(|| format!("create Windows test target directory {}", path.display()))?;
        Some(path)
    } else {
        None
    };

    let mut command = Command::new(program);
    command.args(args).current_dir(workspace_root);
    if let Some(path) = &isolated_target {
        // `cargo xtask` holds target/debug/xtask.exe open on Windows. Keep
        // the full workspace test command exact while compiling its test
        // harnesses in an isolated target directory.
        command.env("CARGO_TARGET_DIR", path);
    }
    let status = command
        .status()
        .with_context(|| format!("run {program} for ci-small step {step}"))?;
    if let Some(path) = isolated_target {
        let _ = fs::remove_dir_all(path);
    }
    if !status.success() {
        bail!("ci-small step {step} failed with {status}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{STEPS, run};
    use anyhow::{Result, bail};
    use std::path::Path;

    #[test]
    fn exposes_the_required_sequence_in_order() -> Result<()> {
        if STEPS != ["format", "clippy", "tests", "no-panic", "authority", "diff"] {
            bail!("ci-small steps changed unexpectedly: {STEPS:?}");
        }
        Ok(())
    }

    #[test]
    fn rejects_unknown_steps_before_spawning_a_process() -> Result<()> {
        let error = match run(Path::new("."), Some("unknown")) {
            Ok(()) => bail!("unknown step unexpectedly succeeded"),
            Err(error) => error,
        };
        if !error.to_string().contains("unknown ci-small step") {
            bail!("unexpected error: {error}");
        }
        Ok(())
    }
}
