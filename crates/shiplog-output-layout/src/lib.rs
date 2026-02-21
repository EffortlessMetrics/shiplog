//! Canonical output layout contracts for shiplog run artifacts.

use std::path::{Path, PathBuf};

/// Canonical artifact filenames emitted by the shiplog pipeline.
pub const FILE_PACKET_MD: &str = "packet.md";
pub const FILE_LEDGER_EVENTS_JSONL: &str = "ledger.events.jsonl";
pub const FILE_COVERAGE_MANIFEST_JSON: &str = "coverage.manifest.json";
pub const FILE_BUNDLE_MANIFEST_JSON: &str = "bundle.manifest.json";
pub const FILE_REDACTION_ALIASES_JSON: &str = "redaction.aliases.json";

/// Canonical directory names used by profile-based outputs.
pub const DIR_PROFILES: &str = "profiles";
pub const PROFILE_INTERNAL: &str = "internal";
pub const PROFILE_MANAGER: &str = "manager";
pub const PROFILE_PUBLIC: &str = "public";

/// Paths for a complete shiplog run output directory.
#[derive(Debug, Clone)]
pub struct RunArtifactPaths {
    pub out_dir: PathBuf,
}

impl RunArtifactPaths {
    /// Construct a path helper for a given run output directory.
    pub fn new(out_dir: impl Into<PathBuf>) -> Self {
        Self {
            out_dir: out_dir.into(),
        }
    }

    /// `packet.md`
    pub fn packet_md(&self) -> PathBuf {
        self.out_dir.join(FILE_PACKET_MD)
    }

    /// `ledger.events.jsonl`
    pub fn ledger_events(&self) -> PathBuf {
        self.out_dir.join(FILE_LEDGER_EVENTS_JSONL)
    }

    /// `coverage.manifest.json`
    pub fn coverage_manifest(&self) -> PathBuf {
        self.out_dir.join(FILE_COVERAGE_MANIFEST_JSON)
    }

    /// `bundle.manifest.json`
    pub fn bundle_manifest(&self) -> PathBuf {
        self.out_dir.join(FILE_BUNDLE_MANIFEST_JSON)
    }

    /// `profiles/<profile>/packet.md`
    pub fn profile_packet(&self, profile: impl AsRef<str>) -> PathBuf {
        self.out_dir
            .join(DIR_PROFILES)
            .join(profile.as_ref())
            .join(FILE_PACKET_MD)
    }
}

/// Compute the zip file path for a run profile.
/// - `"internal"` -> `<run_dir>.zip`
/// - any other value -> `<run_dir>.<profile>.zip`
pub fn zip_path_for_profile(out_dir: &Path, profile: &str) -> PathBuf {
    if profile == PROFILE_INTERNAL {
        return out_dir.with_extension("zip");
    }

    let stem = out_dir.file_name().unwrap_or_default().to_string_lossy();
    out_dir.with_file_name(format!("{}.{}.zip", stem, profile))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_paths_are_stable() {
        let paths = RunArtifactPaths::new("/tmp/run_01");
        assert_eq!(paths.packet_md(), PathBuf::from("/tmp/run_01/packet.md"));
        assert_eq!(
            paths.ledger_events(),
            PathBuf::from("/tmp/run_01/ledger.events.jsonl")
        );
        assert_eq!(
            paths.coverage_manifest(),
            PathBuf::from("/tmp/run_01/coverage.manifest.json")
        );
        assert_eq!(
            paths.bundle_manifest(),
            PathBuf::from("/tmp/run_01/bundle.manifest.json")
        );
        assert_eq!(
            paths.profile_packet(PROFILE_MANAGER),
            PathBuf::from("/tmp/run_01/profiles/manager/packet.md")
        );
    }

    #[test]
    fn artifact_zip_path_depends_on_profile() {
        let internal = zip_path_for_profile(Path::new("/tmp/run_01"), PROFILE_INTERNAL);
        let manager = zip_path_for_profile(Path::new("/tmp/run_01"), PROFILE_MANAGER);
        assert_eq!(internal, Path::new("/tmp/run_01.zip"));
        assert_eq!(manager, Path::new("/tmp/run_01.manager.zip"));
    }
}
