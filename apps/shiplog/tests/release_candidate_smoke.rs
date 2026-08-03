//! Ordinary-PR proof for the staged release-candidate selection contract.
//!
//! These tests synthesize one complete four-platform candidate bundle and drive
//! the real platform smoke script. They prove selection, manifest, checksum,
//! command, and receipt behavior without creating a tag or public release. They
//! do not replace the explicit-tag four-runner proof required before #245 closes.

use anyhow::{Context, Result, bail, ensure};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

const SOURCE_SHA: &str = "1111111111111111111111111111111111111111";
const RELEASE_ASSETS: [&str; 4] = [
    "shiplog-x86_64-unknown-linux-gnu/shiplog-x86_64-unknown-linux-gnu",
    "shiplog-x86_64-apple-darwin/shiplog-x86_64-apple-darwin",
    "shiplog-aarch64-apple-darwin/shiplog-aarch64-apple-darwin",
    "shiplog-x86_64-pc-windows-msvc/shiplog-x86_64-pc-windows-msvc.exe",
];

struct CandidateFixture {
    _root: TempDir,
    candidate_dir: PathBuf,
    smoke_dir: PathBuf,
    asset_path: PathBuf,
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn current_release_asset() -> Result<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Ok(RELEASE_ASSETS[0]),
        ("macos", "x86_64") => Ok(RELEASE_ASSETS[1]),
        ("macos", "aarch64") => Ok(RELEASE_ASSETS[2]),
        ("windows", "x86_64") => Ok(RELEASE_ASSETS[3]),
        (os, arch) => bail!("unsupported release candidate test platform: {os}/{arch}"),
    }
}

fn sha256(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

fn write_candidate_metadata(fixture: &CandidateFixture) -> Result<()> {
    let sums_path = fixture.candidate_dir.join("SHA256SUMS.txt");
    let mut sums = String::new();
    for relative in RELEASE_ASSETS {
        let path = fixture.candidate_dir.join(relative);
        sums.push_str(&format!("{}  {relative}\n", sha256(&path)?));
    }
    fs::write(&sums_path, sums).with_context(|| format!("write {}", sums_path.display()))?;

    let manifest_path = fixture.candidate_dir.join("RELEASE_CANDIDATE.txt");
    fs::write(
        &manifest_path,
        format!(
            concat!(
                "schema_version=1\n",
                "release_tag=v{}\n",
                "source_sha={}\n",
                "repository=EffortlessMetrics/shiplog-swarm\n",
                "workflow_run_id=1\n",
                "workflow_run_attempt=1\n",
                "asset_count=4\n",
                "checksum_manifest_sha256={}\n"
            ),
            env!("CARGO_PKG_VERSION"),
            SOURCE_SHA,
            sha256(&sums_path)?
        ),
    )
    .with_context(|| format!("write {}", manifest_path.display()))?;
    Ok(())
}

fn candidate_fixture() -> Result<CandidateFixture> {
    let root = TempDir::new().context("create release candidate fixture")?;
    let candidate_dir = root.path().join("candidate");
    let smoke_dir = root.path().join("smoke");
    let asset_relative = current_release_asset()?;
    let asset_path = candidate_dir.join(asset_relative);

    for relative in RELEASE_ASSETS {
        let path = candidate_dir.join(relative);
        let parent = path
            .parent()
            .context("candidate asset should have a parent")?;
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        if relative == asset_relative {
            fs::copy(env!("CARGO_BIN_EXE_shiplog"), &path).with_context(|| {
                format!(
                    "copy test binary {} to {}",
                    env!("CARGO_BIN_EXE_shiplog"),
                    path.display()
                )
            })?;
        } else {
            fs::write(&path, format!("fixture placeholder for {relative}\n"))
                .with_context(|| format!("write {}", path.display()))?;
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&asset_path)
            .with_context(|| format!("read metadata for {}", asset_path.display()))?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&asset_path, permissions)
            .with_context(|| format!("set executable permissions for {}", asset_path.display()))?;
    }

    let fixture = CandidateFixture {
        _root: root,
        candidate_dir,
        smoke_dir,
        asset_path,
    };
    write_candidate_metadata(&fixture)?;
    Ok(fixture)
}

fn run_candidate_smoke(fixture: &CandidateFixture) -> Result<Output> {
    let root = repo_root();
    let version = env!("CARGO_PKG_VERSION");

    #[cfg(windows)]
    let mut command = {
        let script_path = root.join("scripts/release-install-smoke.ps1");
        let script = script_path
            .to_str()
            .context("PowerShell smoke path is not UTF-8")?;
        let mut command = Command::new("pwsh");
        command.args(["-NoProfile", "-File", script, version]);
        command
    };

    #[cfg(not(windows))]
    let mut command = {
        let script_path = root.join("scripts/release-install-smoke.sh");
        let script = script_path
            .to_str()
            .context("Bash smoke path is not UTF-8")?;
        let mut command = Command::new("bash");
        command.args([script, version]);
        command
    };

    command
        .current_dir(&root)
        .env("SHIPLOG_RELEASE_CANDIDATE_DIR", &fixture.candidate_dir)
        .env("SHIPLOG_RELEASE_SOURCE_SHA", SOURCE_SHA)
        .env("SHIPLOG_RELEASE_SMOKE_DIR", &fixture.smoke_dir)
        .env_remove("GITHUB_TOKEN")
        .env_remove("GH_TOKEN")
        .env_remove("GITLAB_TOKEN")
        .env_remove("JIRA_TOKEN")
        .env_remove("LINEAR_API_KEY")
        .env_remove("SHIPLOG_REDACT_KEY")
        .output()
        .context("run staged release candidate smoke")
}

fn combined_output(output: &Output) -> String {
    format!(
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn manifest_path(fixture: &CandidateFixture) -> PathBuf {
    fixture.candidate_dir.join("RELEASE_CANDIDATE.txt")
}

fn rewrite_host_checksum_entry_as_flat_path(fixture: &CandidateFixture) -> Result<()> {
    let asset = current_release_asset()?;
    let asset_name = Path::new(asset)
        .file_name()
        .and_then(|name| name.to_str())
        .context("host candidate asset name is not UTF-8")?;
    let sums_path = fixture.candidate_dir.join("SHA256SUMS.txt");
    let sums = fs::read_to_string(&sums_path)
        .with_context(|| format!("read checksum manifest {}", sums_path.display()))?;
    let host_digest = sha256(&fixture.asset_path)?;
    let rewritten = sums
        .lines()
        .map(|line| {
            if line.ends_with(asset) {
                format!("{host_digest}  {asset_name}")
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    fs::write(&sums_path, rewritten)
        .with_context(|| format!("write flat checksum manifest {}", sums_path.display()))?;
    let sums_digest = sha256(&sums_path)?;
    let manifest = manifest_path(fixture);
    let text = fs::read_to_string(&manifest)
        .with_context(|| format!("read candidate manifest {}", manifest.display()))?;
    let updated = text
        .lines()
        .map(|line| {
            if line.starts_with("checksum_manifest_sha256=") {
                format!("checksum_manifest_sha256={sums_digest}")
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    fs::write(&manifest, updated).with_context(|| {
        format!(
            "write flat-checksum candidate manifest {}",
            manifest.display()
        )
    })?;
    Ok(())
}

#[test]
fn staged_candidate_smoke_uses_local_bundle_and_emits_receipts() -> Result<()> {
    let fixture = candidate_fixture()?;
    let output = run_candidate_smoke(&fixture)?;
    ensure!(output.status.success(), "{}", combined_output(&output));

    let text = combined_output(&output);
    ensure!(
        text.contains("release install smoke passed"),
        "all successful modes should emit the general release smoke line: {text}"
    );
    ensure!(
        text.contains("staged release candidate smoke passed"),
        "candidate mode should report its exact proof boundary: {text}"
    );
    ensure!(
        fixture
            .smoke_dir
            .join("cold-start/status.latest.json")
            .is_file(),
        "candidate smoke should retain the status receipt"
    );
    ensure!(
        fixture
            .smoke_dir
            .join("demo-review-rescue.stdout")
            .is_file(),
        "candidate smoke should retain the no-network rescue transcript"
    );
    Ok(())
}

#[test]
fn staged_candidate_smoke_rejects_incomplete_four_platform_bundle() -> Result<()> {
    let fixture = candidate_fixture()?;
    let current = current_release_asset()?;
    let missing_relative = RELEASE_ASSETS
        .iter()
        .copied()
        .find(|relative| *relative != current)
        .context("fixture should have a non-host candidate asset")?;
    let missing_name = Path::new(missing_relative)
        .file_name()
        .and_then(|name| name.to_str())
        .context("missing candidate asset name is not UTF-8")?;
    let missing_path = fixture.candidate_dir.join(missing_relative);
    fs::remove_file(&missing_path)
        .with_context(|| format!("remove missing fixture asset {}", missing_path.display()))?;

    let output = run_candidate_smoke(&fixture)?;
    ensure!(
        !output.status.success(),
        "incomplete candidate set unexpectedly passed"
    );
    ensure!(
        combined_output(&output).contains(&format!(
            "candidate bundle must contain exactly one {missing_name}; found 0"
        )),
        "candidate mode must prove all four platform binaries exist: {}",
        combined_output(&output)
    );
    Ok(())
}

#[test]
fn staged_candidate_smoke_rejects_duplicate_manifest_fields() -> Result<()> {
    let fixture = candidate_fixture()?;
    let path = manifest_path(&fixture);
    let mut manifest = fs::read_to_string(&path)
        .with_context(|| format!("read candidate manifest {}", path.display()))?;
    manifest.push_str("release_tag=v0.0.0\n");
    fs::write(&path, manifest)
        .with_context(|| format!("write duplicate-field manifest {}", path.display()))?;

    let output = run_candidate_smoke(&fixture)?;
    ensure!(
        !output.status.success(),
        "duplicate manifest unexpectedly passed"
    );
    ensure!(
        combined_output(&output).contains("duplicate candidate manifest field: release_tag"),
        "duplicate keys must fail before candidate execution: {}",
        combined_output(&output)
    );
    Ok(())
}

#[test]
fn staged_candidate_smoke_rejects_unknown_manifest_fields() -> Result<()> {
    let fixture = candidate_fixture()?;
    let path = manifest_path(&fixture);
    let mut manifest = fs::read_to_string(&path)
        .with_context(|| format!("read candidate manifest {}", path.display()))?;
    manifest.push_str("unexpected=value\n");
    fs::write(&path, manifest)
        .with_context(|| format!("write unknown-field manifest {}", path.display()))?;

    let output = run_candidate_smoke(&fixture)?;
    ensure!(
        !output.status.success(),
        "unknown manifest field unexpectedly passed"
    );
    ensure!(
        combined_output(&output).contains("unknown candidate manifest field: unexpected"),
        "unknown keys must fail before candidate execution: {}",
        combined_output(&output)
    );
    Ok(())
}

#[test]
fn staged_candidate_smoke_rejects_invalid_workflow_identity() -> Result<()> {
    let fixture = candidate_fixture()?;
    let path = manifest_path(&fixture);
    let manifest = fs::read_to_string(&path)
        .with_context(|| format!("read candidate manifest {}", path.display()))?
        .replace("workflow_run_id=1", "workflow_run_id=not-a-run");
    fs::write(&path, manifest)
        .with_context(|| format!("write invalid-identity manifest {}", path.display()))?;

    let output = run_candidate_smoke(&fixture)?;
    ensure!(
        !output.status.success(),
        "unsafe workflow identity unexpectedly passed"
    );
    ensure!(
        combined_output(&output).contains("invalid workflow run identity"),
        "manifest identities must stay closed and machine-shaped: {}",
        combined_output(&output)
    );
    Ok(())
}

#[test]
fn staged_candidate_smoke_rejects_checksum_mismatch_before_execution() -> Result<()> {
    let fixture = candidate_fixture()?;
    let mut bytes = fs::read(&fixture.asset_path)
        .with_context(|| format!("read candidate asset {}", fixture.asset_path.display()))?;
    bytes.extend_from_slice(b"corrupt");
    fs::write(&fixture.asset_path, bytes).with_context(|| {
        format!(
            "write corrupted candidate asset {}",
            fixture.asset_path.display()
        )
    })?;

    let output = run_candidate_smoke(&fixture)?;
    ensure!(
        !output.status.success(),
        "corrupted candidate unexpectedly passed"
    );
    ensure!(
        combined_output(&output).contains("checksum mismatch"),
        "corruption should fail at the checksum boundary: {}",
        combined_output(&output)
    );
    Ok(())
}

#[test]
fn staged_candidate_smoke_accepts_flat_checksum_entry() -> Result<()> {
    let fixture = candidate_fixture()?;
    rewrite_host_checksum_entry_as_flat_path(&fixture)?;
    let output = run_candidate_smoke(&fixture)?;
    ensure!(output.status.success(), "{}", combined_output(&output));
    Ok(())
}

#[test]
fn staged_candidate_smoke_rejects_broken_command_with_matching_checksum() -> Result<()> {
    let fixture = candidate_fixture()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::write(&fixture.asset_path, "#!/usr/bin/env bash\nexit 42\n").with_context(|| {
            format!(
                "write broken Unix candidate {}",
                fixture.asset_path.display()
            )
        })?;
        let mut permissions = fs::metadata(&fixture.asset_path)
            .with_context(|| format!("read metadata for {}", fixture.asset_path.display()))?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&fixture.asset_path, permissions).with_context(|| {
            format!(
                "set executable permissions for {}",
                fixture.asset_path.display()
            )
        })?;
    }
    #[cfg(windows)]
    fs::write(&fixture.asset_path, b"not a Windows executable").with_context(|| {
        format!(
            "write broken Windows candidate {}",
            fixture.asset_path.display()
        )
    })?;
    write_candidate_metadata(&fixture)?;

    let output = run_candidate_smoke(&fixture)?;
    ensure!(
        !output.status.success(),
        "broken candidate unexpectedly passed"
    );
    ensure!(
        combined_output(&output).contains("candidate binary failed --version"),
        "matching checksums must not rescue a broken executable: {}",
        combined_output(&output)
    );
    Ok(())
}

#[test]
fn release_workflow_binds_tag_push_identity_and_staged_contract() -> Result<()> {
    let path = repo_root().join(".github/workflows/release.yml");
    let workflow = fs::read_to_string(&path)
        .with_context(|| format!("read release workflow {}", path.display()))?;
    ensure!(
        workflow.contains("WEBHOOK_SHA: ${{ github.sha }}"),
        "preflight must receive the push webhook SHA through the step environment"
    );
    ensure!(
        workflow.contains("EVENT_NAME\" == \"push\" && \"$release_sha\" != \"$WEBHOOK_SHA\""),
        "tag-push preflight must reject a tag that moved after the webhook"
    );
    ensure!(
        workflow.contains(
            "Swarm verifies release inputs but cannot create a GitHub release."
        ) && workflow.contains(
            "Promote this exact proven tree to EffortlessMetrics/shiplog for authorized release execution."
        ),
        "the source handoff must preserve both durable summary statements"
    );
    ensure!(
        workflow.contains("Confirm exact negative-control checkout")
            && workflow.contains("run: test \"$(git rev-parse HEAD)\" = \"$RELEASE_SHA\""),
        "negative controls must verify the checked-out release SHA"
    );
    ensure!(
        !workflow.contains("actions/download-artifact@v8"),
        "all download-artifact actions must be immutable"
    );
    ensure!(
        workflow.contains("asset_count=$asset_count")
            && workflow.contains("asset_count=\"${#expected[@]}\""),
        "candidate manifest asset_count must derive from its canonical expected set"
    );
    Ok(())
}
