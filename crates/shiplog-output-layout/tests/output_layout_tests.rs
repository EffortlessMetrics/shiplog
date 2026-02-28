use shiplog_output_layout::*;
use std::path::{Path, PathBuf};

#[test]
fn constants_are_stable() {
    assert_eq!(FILE_PACKET_MD, "packet.md");
    assert_eq!(FILE_LEDGER_EVENTS_JSONL, "ledger.events.jsonl");
    assert_eq!(FILE_COVERAGE_MANIFEST_JSON, "coverage.manifest.json");
    assert_eq!(FILE_BUNDLE_MANIFEST_JSON, "bundle.manifest.json");
    assert_eq!(FILE_REDACTION_ALIASES_JSON, "redaction.aliases.json");
    assert_eq!(DIR_PROFILES, "profiles");
    assert_eq!(PROFILE_INTERNAL, "internal");
    assert_eq!(PROFILE_MANAGER, "manager");
    assert_eq!(PROFILE_PUBLIC, "public");
}

#[test]
fn run_artifact_paths_from_string() {
    let paths = RunArtifactPaths::new("out/run_01");
    assert!(paths.packet_md().ends_with("packet.md"));
    assert!(paths.ledger_events().ends_with("ledger.events.jsonl"));
    assert!(
        paths
            .coverage_manifest()
            .ends_with("coverage.manifest.json")
    );
    assert!(paths.bundle_manifest().ends_with("bundle.manifest.json"));
}

#[test]
fn profile_packet_includes_profile_name() {
    let paths = RunArtifactPaths::new("out/run_01");
    let p = paths.profile_packet("manager");
    let s = p.to_string_lossy();
    assert!(s.contains("profiles"));
    assert!(s.contains("manager"));
    assert!(s.contains("packet.md"));
}

#[test]
fn zip_path_internal_has_no_profile_suffix() {
    let p = zip_path_for_profile(Path::new("out/run_01"), PROFILE_INTERNAL);
    assert_eq!(p, PathBuf::from("out/run_01.zip"));
}

#[test]
fn zip_path_non_internal_includes_profile() {
    let p = zip_path_for_profile(Path::new("out/run_01"), PROFILE_MANAGER);
    assert_eq!(p, PathBuf::from("out/run_01.manager.zip"));

    let p = zip_path_for_profile(Path::new("out/run_01"), PROFILE_PUBLIC);
    assert_eq!(p, PathBuf::from("out/run_01.public.zip"));
}

#[test]
fn all_zip_paths_end_with_zip() {
    for profile in &[PROFILE_INTERNAL, PROFILE_MANAGER, PROFILE_PUBLIC, "custom"] {
        let p = zip_path_for_profile(Path::new("dir"), profile);
        assert!(p.to_string_lossy().ends_with(".zip"));
    }
}

#[test]
fn clone_produces_equal_paths() {
    let paths = RunArtifactPaths::new("run");
    let cloned = paths.clone();
    assert_eq!(paths.packet_md(), cloned.packet_md());
    assert_eq!(paths.ledger_events(), cloned.ledger_events());
}
