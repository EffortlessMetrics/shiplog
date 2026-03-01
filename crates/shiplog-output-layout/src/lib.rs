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

    // --- Constant value tests ---

    #[test]
    fn file_constants_have_expected_values() {
        assert_eq!(FILE_PACKET_MD, "packet.md");
        assert_eq!(FILE_LEDGER_EVENTS_JSONL, "ledger.events.jsonl");
        assert_eq!(FILE_COVERAGE_MANIFEST_JSON, "coverage.manifest.json");
        assert_eq!(FILE_BUNDLE_MANIFEST_JSON, "bundle.manifest.json");
        assert_eq!(FILE_REDACTION_ALIASES_JSON, "redaction.aliases.json");
    }

    #[test]
    fn profile_constants_have_expected_values() {
        assert_eq!(DIR_PROFILES, "profiles");
        assert_eq!(PROFILE_INTERNAL, "internal");
        assert_eq!(PROFILE_MANAGER, "manager");
        assert_eq!(PROFILE_PUBLIC, "public");
    }

    // --- RunArtifactPaths edge cases ---

    #[test]
    fn paths_from_string() {
        let paths = RunArtifactPaths::new(String::from("/data/output"));
        assert_eq!(paths.out_dir, PathBuf::from("/data/output"));
    }

    #[test]
    fn paths_from_pathbuf() {
        let pb = PathBuf::from("/data/output");
        let paths = RunArtifactPaths::new(pb.clone());
        assert_eq!(paths.out_dir, pb);
    }

    #[test]
    fn paths_relative_dir() {
        let paths = RunArtifactPaths::new("out/run_01");
        assert_eq!(paths.packet_md(), PathBuf::from("out/run_01/packet.md"));
        assert_eq!(
            paths.ledger_events(),
            PathBuf::from("out/run_01/ledger.events.jsonl")
        );
    }

    #[test]
    fn paths_current_dir() {
        let paths = RunArtifactPaths::new(".");
        assert_eq!(paths.packet_md(), PathBuf::from("./packet.md"));
    }

    #[test]
    fn profile_packet_internal() {
        let paths = RunArtifactPaths::new("/run");
        assert_eq!(
            paths.profile_packet(PROFILE_INTERNAL),
            PathBuf::from("/run/profiles/internal/packet.md")
        );
    }

    #[test]
    fn profile_packet_public() {
        let paths = RunArtifactPaths::new("/run");
        assert_eq!(
            paths.profile_packet(PROFILE_PUBLIC),
            PathBuf::from("/run/profiles/public/packet.md")
        );
    }

    #[test]
    fn profile_packet_custom_profile() {
        let paths = RunArtifactPaths::new("/run");
        assert_eq!(
            paths.profile_packet("custom"),
            PathBuf::from("/run/profiles/custom/packet.md")
        );
    }

    #[test]
    fn all_paths_share_out_dir_prefix() {
        let paths = RunArtifactPaths::new("/base");
        let base = PathBuf::from("/base");
        assert!(paths.packet_md().starts_with(&base));
        assert!(paths.ledger_events().starts_with(&base));
        assert!(paths.coverage_manifest().starts_with(&base));
        assert!(paths.bundle_manifest().starts_with(&base));
        assert!(paths.profile_packet("any").starts_with(&base));
    }

    #[test]
    fn clone_produces_equal_paths() {
        let paths = RunArtifactPaths::new("/run");
        let cloned = paths.clone();
        assert_eq!(paths.packet_md(), cloned.packet_md());
        assert_eq!(paths.ledger_events(), cloned.ledger_events());
    }

    #[test]
    fn debug_impl_not_empty() {
        let paths = RunArtifactPaths::new("/run");
        let debug = format!("{:?}", paths);
        assert!(!debug.is_empty());
        assert!(debug.contains("RunArtifactPaths"));
    }

    // --- zip_path_for_profile edge cases ---

    #[test]
    fn zip_path_public_profile() {
        let p = zip_path_for_profile(Path::new("/tmp/run_01"), PROFILE_PUBLIC);
        assert_eq!(p, Path::new("/tmp/run_01.public.zip"));
    }

    #[test]
    fn zip_path_custom_profile() {
        let p = zip_path_for_profile(Path::new("/out/my_run"), "custom");
        assert_eq!(p, Path::new("/out/my_run.custom.zip"));
    }

    #[test]
    fn zip_path_internal_always_just_zip() {
        let p = zip_path_for_profile(Path::new("/a/b/c"), PROFILE_INTERNAL);
        assert_eq!(p, Path::new("/a/b/c.zip"));
    }

    #[test]
    fn zip_path_relative_dir() {
        let p = zip_path_for_profile(Path::new("out/run"), PROFILE_MANAGER);
        assert_eq!(p, Path::new("out/run.manager.zip"));
    }

    #[test]
    fn zip_path_relative_internal() {
        let p = zip_path_for_profile(Path::new("out/run"), PROFILE_INTERNAL);
        assert_eq!(p, Path::new("out/run.zip"));
    }

    #[test]
    fn zip_path_profiles_are_distinct() {
        let base = Path::new("/run");
        let internal = zip_path_for_profile(base, PROFILE_INTERNAL);
        let manager = zip_path_for_profile(base, PROFILE_MANAGER);
        let public = zip_path_for_profile(base, PROFILE_PUBLIC);
        // All three are distinct
        assert_ne!(internal, manager);
        assert_ne!(internal, public);
        assert_ne!(manager, public);
    }

    #[test]
    fn zip_path_all_end_with_zip() {
        let base = Path::new("/run");
        for profile in &[PROFILE_INTERNAL, PROFILE_MANAGER, PROFILE_PUBLIC, "custom"] {
            let p = zip_path_for_profile(base, profile);
            assert!(
                p.to_string_lossy().ends_with(".zip"),
                "expected .zip suffix for profile '{}', got {:?}",
                profile,
                p
            );
        }
    }

    // --- Property tests ---

    mod prop {
        use super::*;
        use proptest::prelude::*;

        fn arb_dir() -> impl Strategy<Value = String> {
            // Generate well-formed directory paths avoiding edge cases
            // like bare "/" or "//" which produce UNC paths on Windows
            proptest::string::string_regex("[a-z]{1,5}(/[a-z]{1,5}){0,3}").unwrap()
        }

        proptest! {
            #[test]
            fn packet_md_always_ends_with_filename(dir in arb_dir()) {
                let paths = RunArtifactPaths::new(&dir);
                let p = paths.packet_md();
                prop_assert!(p.ends_with(FILE_PACKET_MD));
            }

            #[test]
            fn ledger_events_always_ends_with_filename(dir in arb_dir()) {
                let paths = RunArtifactPaths::new(&dir);
                let p = paths.ledger_events();
                prop_assert!(p.ends_with(FILE_LEDGER_EVENTS_JSONL));
            }

            #[test]
            fn coverage_manifest_always_ends_with_filename(dir in arb_dir()) {
                let paths = RunArtifactPaths::new(&dir);
                let p = paths.coverage_manifest();
                prop_assert!(p.ends_with(FILE_COVERAGE_MANIFEST_JSON));
            }

            #[test]
            fn bundle_manifest_always_ends_with_filename(dir in arb_dir()) {
                let paths = RunArtifactPaths::new(&dir);
                let p = paths.bundle_manifest();
                prop_assert!(p.ends_with(FILE_BUNDLE_MANIFEST_JSON));
            }

            #[test]
            fn profile_packet_always_contains_profiles_dir(
                dir in arb_dir(),
                profile in "[a-z]{1,10}",
            ) {
                let paths = RunArtifactPaths::new(&dir);
                let p = paths.profile_packet(&profile);
                let p_str = p.to_string_lossy();
                prop_assert!(p_str.contains(DIR_PROFILES));
                prop_assert!(p_str.contains(&profile));
                prop_assert!(p.ends_with(FILE_PACKET_MD));
            }

            #[test]
            fn zip_path_always_ends_with_zip(
                dir in arb_dir(),
                profile in "[a-z]{1,10}",
            ) {
                let p = zip_path_for_profile(Path::new(&dir), &profile);
                prop_assert!(
                    p.to_string_lossy().ends_with(".zip"),
                    "expected .zip suffix, got {:?}", p
                );
            }

            #[test]
            fn zip_path_internal_never_contains_profile_name(dir in arb_dir()) {
                let p = zip_path_for_profile(Path::new(&dir), PROFILE_INTERNAL);
                let name = p.file_name().unwrap().to_string_lossy();
                // For internal, the filename should NOT contain "internal"
                prop_assert!(!name.contains("internal"));
            }

            #[test]
            fn zip_path_non_internal_contains_profile_name(
                dir in arb_dir(),
                profile in "[a-z]{1,10}",
            ) {
                prop_assume!(profile != PROFILE_INTERNAL);
                let p = zip_path_for_profile(Path::new(&dir), &profile);
                let name = p.file_name().unwrap().to_string_lossy();
                prop_assert!(
                    name.contains(&profile),
                    "expected profile '{}' in filename '{}'", profile, name
                );
            }

            #[test]
            fn all_artifact_paths_start_with_out_dir(dir in arb_dir()) {
                let paths = RunArtifactPaths::new(&dir);
                let base = PathBuf::from(&dir);
                prop_assert!(paths.packet_md().starts_with(&base));
                prop_assert!(paths.ledger_events().starts_with(&base));
                prop_assert!(paths.coverage_manifest().starts_with(&base));
                prop_assert!(paths.bundle_manifest().starts_with(&base));
            }
        }
    }
}
