//! BDD scenarios for the explicit first-use setup command.

use std::path::Path;
use std::process::Command;

use shiplog_testkit::bdd::assertions::*;
use shiplog_testkit::bdd::{Scenario, ScenarioContext};

fn shiplog_command(root: &Path, args: &[&str]) -> Result<std::process::Output, String> {
    Command::new(env!("CARGO_BIN_EXE_shiplog"))
        .current_dir(root)
        .args(args)
        .output()
        .map_err(|err| format!("run shiplog {:?}: {err}", args))
}

fn workspace(ctx: &ScenarioContext) -> Result<&Path, String> {
    ctx.path("workspace")
        .ok_or_else(|| "workspace path was not initialized".to_string())
}

fn given_empty_workspace(ctx: &mut ScenarioContext) {
    match tempfile::tempdir() {
        Ok(dir) => {
            let path = dir.keep();
            ctx.register_cleanup_path(path.clone());
            ctx.paths.insert("workspace".into(), path);
        }
        Err(err) => {
            ctx.strings.insert("setup_error".into(), err.to_string());
        }
    }
}

fn fail_if_setup_error(ctx: &ScenarioContext) -> Result<(), String> {
    if let Some(error) = ctx.string("setup_error") {
        return Err(format!("create temporary workspace: {error}"));
    }
    Ok(())
}

fn run_command(ctx: &mut ScenarioContext, args: &[&str]) -> Result<(), String> {
    fail_if_setup_error(ctx)?;
    let root = workspace(ctx)?;
    let output = shiplog_command(root, args)?;
    ctx.flags
        .insert("command_succeeded".into(), output.status.success());
    ctx.data.insert("stdout".into(), output.stdout);
    ctx.data.insert("stderr".into(), output.stderr);
    Ok(())
}

fn stdout(ctx: &ScenarioContext) -> Result<&str, String> {
    std::str::from_utf8(
        ctx.data
            .get("stdout")
            .ok_or_else(|| "command stdout was not captured".to_string())?,
    )
    .map_err(|err| format!("decode command stdout: {err}"))
}

fn stderr(ctx: &ScenarioContext) -> Result<&str, String> {
    std::str::from_utf8(
        ctx.data
            .get("stderr")
            .ok_or_else(|| "command stderr was not captured".to_string())?,
    )
    .map_err(|err| format!("decode command stderr: {err}"))
}

fn file_tree(root: &Path) -> Result<Vec<String>, String> {
    fn visit(root: &Path, dir: &Path, paths: &mut Vec<String>) -> Result<(), String> {
        let mut children = std::fs::read_dir(dir)
            .map_err(|err| format!("read {}: {err}", dir.display()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| format!("read entry under {}: {err}", dir.display()))?;
        children.sort_by_key(|entry| entry.path());
        for child in children {
            let path = child.path();
            let metadata = child
                .metadata()
                .map_err(|err| format!("metadata {}: {err}", path.display()))?;
            if metadata.is_dir() {
                visit(root, &path, paths)?;
            } else if metadata.is_file() {
                let relative = path
                    .strip_prefix(root)
                    .map_err(|err| format!("strip {}: {err}", path.display()))?
                    .to_string_lossy()
                    .replace('\\', "/");
                paths.push(relative);
            }
        }
        Ok(())
    }

    let mut paths = Vec::new();
    visit(root, root, &mut paths)?;
    Ok(paths)
}

fn capture_tree(ctx: &mut ScenarioContext, key: &str) -> Result<(), String> {
    let root = workspace(ctx)?;
    let paths = file_tree(root)?;
    ctx.data.insert(
        key.into(),
        serde_json::to_vec(&paths).map_err(|err| format!("serialize file tree: {err}"))?,
    );
    Ok(())
}

fn stored_tree(ctx: &ScenarioContext, key: &str) -> Result<Vec<String>, String> {
    serde_json::from_slice(
        ctx.data
            .get(key)
            .ok_or_else(|| format!("file tree {key:?} was not captured"))?,
    )
    .map_err(|err| format!("decode file tree {key:?}: {err}"))
}

#[test]
fn start_help_describes_confirmation_and_preview() -> Result<(), String> {
    Scenario::new("Start help exposes confirmation and preview controls")
        .given("an empty workspace", given_empty_workspace)
        .when("the user requests start help", |ctx| {
            run_command(ctx, &["start", "--help"])
        })
        .then("help succeeds", |ctx| {
            assert_true(
                ctx.flag("command_succeeded").unwrap_or(false),
                "help succeeds",
            )
        })
        .then("help names confirmation and preview", |ctx| {
            let output = stdout(ctx)?;
            assert_contains(output, "--yes", "start help confirmation")?;
            assert_contains(output, "--dry-run", "start help preview")?;
            assert_contains(
                output,
                "without collecting evidence",
                "start help safety posture",
            )
        })
        .run()
}

#[test]
fn start_without_confirmation_writes_nothing() -> Result<(), String> {
    Scenario::new("Start refuses unconfirmed setup writes")
        .given("an empty workspace", given_empty_workspace)
        .when("the user runs start without confirmation", |ctx| {
            capture_tree(ctx, "before")?;
            run_command(ctx, &["start"])
        })
        .then("start fails before writing", |ctx| {
            assert_false(
                ctx.flag("command_succeeded").unwrap_or(true),
                "unconfirmed start fails",
            )?;
            assert_contains(
                stderr(ctx)?,
                "shiplog start requires --yes",
                "confirmation error",
            )?;
            assert_contains(stderr(ctx)?, "--dry-run", "preview guidance")
        })
        .then("the workspace is unchanged", |ctx| {
            let before = stored_tree(ctx, "before")?;
            let after = file_tree(workspace(ctx)?)?;
            assert_eq(after, before, "unconfirmed start file tree")
        })
        .run()
}

#[test]
fn start_dry_run_previews_without_writing() -> Result<(), String> {
    Scenario::new("Start dry-run previews the guided scaffold")
        .given("an empty workspace", given_empty_workspace)
        .when("the user previews start", |ctx| {
            capture_tree(ctx, "before")?;
            run_command(ctx, &["start", "--dry-run"])
        })
        .then("the preview succeeds", |ctx| {
            assert_true(
                ctx.flag("command_succeeded").unwrap_or(false),
                "dry-run succeeds",
            )?;
            let output = stdout(ctx)?;
            assert_contains(output, "Would write guided shiplog.toml", "guided preview")?;
            assert_contains(output, "[sources.manual]", "manual source preview")?;
            assert_contains(output, "enabled = true", "enabled manual preview")
        })
        .then("the preview writes no files", |ctx| {
            let before = stored_tree(ctx, "before")?;
            let after = file_tree(workspace(ctx)?)?;
            assert_eq(after, before, "dry-run file tree")
        })
        .run()
}

#[test]
fn start_yes_creates_only_local_guided_setup() -> Result<(), String> {
    Scenario::new("Confirmed start creates local setup without collection")
        .given("an empty workspace", given_empty_workspace)
        .when("the user initializes a local repository and confirms start", |ctx| {
            fail_if_setup_error(ctx)?;
            let root = workspace(ctx)?;
            git2::Repository::init(root).map_err(|err| format!("initialize git fixture: {err}"))?;
            capture_tree(ctx, "before")?;
            run_command(ctx, &["start", "--yes"])
        })
        .then("confirmed start succeeds with local sources", |ctx| {
            assert_true(
                ctx.flag("command_succeeded").unwrap_or(false),
                "confirmed start succeeds",
            )?;
            assert_contains(
                stdout(ctx)?,
                "Initialized guided shiplog setup",
                "setup success",
            )?;
            assert_not_contains(
                stdout(ctx)?,
                "export GITHUB_TOKEN",
                "token-free setup guidance",
            )?;
            let root = workspace(ctx)?;
            let config = std::fs::read_to_string(root.join("shiplog.toml"))
                .map_err(|err| format!("read guided config: {err}"))?;
            assert_contains(&config, "[sources.git]\nenabled = true", "local git source")?;
            assert_contains(
                &config,
                "[sources.manual]\nenabled = true",
                "manual source",
            )?;
            assert_contains(
                &config,
                "[sources.github]\n# GitHub auth uses environment credentials or an authenticated gh CLI session.\n# Use either user or me = true.\nenabled = false",
                "disabled token provider",
            )?;
            assert_contains(
                stdout(ctx)?,
                "shiplog doctor --setup",
                "next setup guidance",
            )
        })
        .then("confirmed start creates no evidence artifacts", |ctx| {
            let before = stored_tree(ctx, "before")?;
            let after = file_tree(workspace(ctx)?)?;
            let added: Vec<_> = after
                .iter()
                .filter(|path| !before.contains(path))
                .cloned()
                .collect();
            assert_eq(
                added,
                vec!["manual_events.yaml".to_string(), "shiplog.toml".to_string()],
                "confirmed start added files",
            )
        })
        .run()
}

#[test]
fn start_yes_preserves_both_existing_setup_files() -> Result<(), String> {
    Scenario::new("Confirmed start preserves existing setup files")
        .given("a workspace with both setup files", |ctx| {
            given_empty_workspace(ctx);
            if ctx.string("setup_error").is_some() {
                return;
            }
            let Some(root) = ctx.paths.get("workspace") else {
                return;
            };
            if let Err(err) = std::fs::write(root.join("shiplog.toml"), "existing setup") {
                ctx.strings.insert("setup_error".into(), err.to_string());
            }
            if let Err(err) =
                std::fs::write(root.join("manual_events.yaml"), "existing manual events")
            {
                ctx.strings.insert("setup_error".into(), err.to_string());
            }
        })
        .when("the user confirms start", |ctx| {
            run_command(ctx, &["start", "--yes"])
        })
        .then("start refuses to overwrite either file", |ctx| {
            assert_false(
                ctx.flag("command_succeeded").unwrap_or(true),
                "existing setup refusal",
            )?;
            assert_contains(stderr(ctx)?, "already exists", "existing setup error")
        })
        .then("both setup files retain their contents", |ctx| {
            let root = workspace(ctx)?;
            let config = std::fs::read_to_string(root.join("shiplog.toml"))
                .map_err(|err| format!("read existing config: {err}"))?;
            let manual = std::fs::read_to_string(root.join("manual_events.yaml"))
                .map_err(|err| format!("read existing manual events: {err}"))?;
            assert_eq(config, "existing setup".to_string(), "existing config")?;
            assert_eq(
                manual,
                "existing manual events".to_string(),
                "existing manual events",
            )
        })
        .run()
}
