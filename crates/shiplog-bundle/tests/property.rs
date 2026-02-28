use proptest::prelude::*;
use shiplog_bundle::{write_bundle_manifest, write_zip};
use shiplog_ids::RunId;
use shiplog_output_layout::*;
use shiplog_schema::bundle::BundleProfile;
use std::fs;

fn make_test_dir(dir: &std::path::Path) {
    fs::write(dir.join(FILE_PACKET_MD), "# Packet").unwrap();
    fs::write(dir.join(FILE_LEDGER_EVENTS_JSONL), "").unwrap();
    fs::write(dir.join(FILE_COVERAGE_MANIFEST_JSON), "{}").unwrap();
    fs::write(
        dir.join(FILE_REDACTION_ALIASES_JSON),
        r#"{"version":1,"entries":{}}"#,
    )
    .unwrap();

    let mgr = dir.join(DIR_PROFILES).join(PROFILE_MANAGER);
    fs::create_dir_all(&mgr).unwrap();
    fs::write(mgr.join(FILE_PACKET_MD), "# Manager").unwrap();

    let pub_dir = dir.join(DIR_PROFILES).join(PROFILE_PUBLIC);
    fs::create_dir_all(&pub_dir).unwrap();
    fs::write(pub_dir.join(FILE_PACKET_MD), "# Public").unwrap();
}

proptest! {
    #[test]
    fn checksums_are_deterministic(content in "[a-zA-Z0-9 \n]{0,500}") {
        let dir1 = tempfile::tempdir().unwrap();
        let dir2 = tempfile::tempdir().unwrap();

        fs::write(dir1.path().join(FILE_PACKET_MD), &content).unwrap();
        fs::write(dir2.path().join(FILE_PACKET_MD), &content).unwrap();

        let run_id = RunId("determinism-test".into());
        let m1 = write_bundle_manifest(dir1.path(), &run_id, &BundleProfile::Internal).unwrap();
        let m2 = write_bundle_manifest(dir2.path(), &run_id, &BundleProfile::Internal).unwrap();

        prop_assert_eq!(m1.files.len(), m2.files.len());
        for (f1, f2) in m1.files.iter().zip(m2.files.iter()) {
            prop_assert_eq!(&f1.sha256, &f2.sha256);
            prop_assert_eq!(f1.bytes, f2.bytes);
            prop_assert_eq!(&f1.path, &f2.path);
        }
    }

    #[test]
    fn manifest_never_includes_forbidden_files(content in "[a-zA-Z0-9]{1,100}") {
        let dir = tempfile::tempdir().unwrap();
        make_test_dir(dir.path());
        fs::write(dir.path().join(FILE_PACKET_MD), &content).unwrap();

        let run_id = RunId("forbidden-test".into());

        for profile in [BundleProfile::Internal, BundleProfile::Manager, BundleProfile::Public] {
            let manifest = write_bundle_manifest(dir.path(), &run_id, &profile).unwrap();
            for f in &manifest.files {
                prop_assert!(
                    !f.path.contains(FILE_REDACTION_ALIASES_JSON),
                    "redaction.aliases.json leaked in {:?}", profile
                );
                prop_assert!(
                    !f.path.contains(FILE_BUNDLE_MANIFEST_JSON),
                    "bundle.manifest.json leaked in {:?}", profile
                );
            }
        }
    }

    #[test]
    fn profile_scoping_filters_correctly(content in "[a-zA-Z0-9]{1,50}") {
        let dir = tempfile::tempdir().unwrap();
        make_test_dir(dir.path());
        fs::write(dir.path().join(FILE_PACKET_MD), &content).unwrap();

        let run_id = RunId("scope-test".into());

        let internal = write_bundle_manifest(dir.path(), &run_id, &BundleProfile::Internal).unwrap();
        let manager = write_bundle_manifest(dir.path(), &run_id, &BundleProfile::Manager).unwrap();
        let public = write_bundle_manifest(dir.path(), &run_id, &BundleProfile::Public).unwrap();

        // Internal includes everything except forbidden files
        prop_assert!(internal.files.len() >= manager.files.len());
        prop_assert_eq!(manager.files.len(), 2usize);
        prop_assert_eq!(public.files.len(), 2usize);

        // Manager bundle contains manager packet + coverage
        let manager_paths: Vec<&str> = manager.files.iter().map(|f| f.path.as_str()).collect();
        prop_assert!(manager_paths.contains(&FILE_COVERAGE_MANIFEST_JSON));
        prop_assert!(manager_paths.iter().any(|p| p.contains(PROFILE_MANAGER)));

        // Public bundle contains public packet + coverage
        let public_paths: Vec<&str> = public.files.iter().map(|f| f.path.as_str()).collect();
        prop_assert!(public_paths.contains(&FILE_COVERAGE_MANIFEST_JSON));
        prop_assert!(public_paths.iter().any(|p| p.contains(PROFILE_PUBLIC)));

        // Manager should not contain public files and vice versa
        prop_assert!(!manager_paths.iter().any(|p| p.contains(PROFILE_PUBLIC)));
        prop_assert!(!public_paths.iter().any(|p| p.contains(PROFILE_MANAGER)));
    }

    #[test]
    fn zip_file_count_matches_manifest(content in "[a-zA-Z0-9]{1,50}") {
        let dir = tempfile::tempdir().unwrap();
        let zip_dir = tempfile::tempdir().unwrap();
        make_test_dir(dir.path());
        fs::write(dir.path().join(FILE_PACKET_MD), &content).unwrap();

        let run_id = RunId("zip-count-test".into());

        for profile in [BundleProfile::Internal, BundleProfile::Manager, BundleProfile::Public] {
            let manifest = write_bundle_manifest(dir.path(), &run_id, &profile).unwrap();

            let zip_path = zip_dir.path().join(format!("{:?}.zip", profile));
            write_zip(dir.path(), &zip_path, &profile).unwrap();

            let file = fs::File::open(&zip_path).unwrap();
            let archive = zip::ZipArchive::new(file).unwrap();

            prop_assert_eq!(
                archive.len(), manifest.files.len(),
                "zip entry count should match manifest file count for {:?}", profile
            );
        }
    }

    #[test]
    fn checksum_bytes_match_file_size(content in "[a-zA-Z0-9 ]{0,200}") {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(FILE_PACKET_MD), &content).unwrap();

        let run_id = RunId("bytes-test".into());
        let manifest = write_bundle_manifest(dir.path(), &run_id, &BundleProfile::Internal).unwrap();

        for f in &manifest.files {
            let full_path = dir.path().join(&f.path);
            let actual_len = fs::metadata(&full_path).unwrap().len();
            prop_assert_eq!(f.bytes, actual_len, "bytes mismatch for {}", f.path);
        }
    }
}
