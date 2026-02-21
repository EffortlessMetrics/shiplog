//! BDD scenarios that lock in microcrate public contracts for external reuse.

#[cfg(any(
    feature = "microcrate_export",
    feature = "microcrate_validate",
    feature = "microcrate_storage",
    feature = "microcrate_notify"
))]
use crate::bdd::assertions::assert_true;
#[cfg(feature = "microcrate_export")]
use shiplog_export::{
    ExportData, ExportEvent, ExportFormat, ExportOptions,
    FILE_BUNDLE_MANIFEST_JSON as EXPORT_FILE_BUNDLE_MANIFEST_JSON,
    FILE_COVERAGE_MANIFEST_JSON as EXPORT_FILE_COVERAGE_MANIFEST_JSON,
    FILE_LEDGER_EVENTS_JSONL as EXPORT_FILE_LEDGER_EVENTS_JSONL,
    FILE_PACKET_MD as EXPORT_FILE_PACKET_MD, RunArtifactPaths as ExportRunArtifactPaths,
    export_data, zip_path_for_profile as export_zip_path_for_profile,
};
#[cfg(feature = "microcrate_output_layout")]
use shiplog_output_layout::{
    DIR_PROFILES, FILE_BUNDLE_MANIFEST_JSON, FILE_COVERAGE_MANIFEST_JSON, FILE_LEDGER_EVENTS_JSONL,
    FILE_PACKET_MD, FILE_REDACTION_ALIASES_JSON, PROFILE_INTERNAL, PROFILE_MANAGER, PROFILE_PUBLIC,
    RunArtifactPaths, zip_path_for_profile,
};
#[cfg(feature = "microcrate_export")]
use std::path::Path;

#[cfg(feature = "microcrate_notify")]
use shiplog_notify::{Notification, NotificationService};

#[cfg(feature = "microcrate_storage")]
use shiplog_storage::{InMemoryStorage, Storage, StorageKey};

#[cfg(feature = "microcrate_validate")]
use shiplog_validate::{EventValidator, Packet, PacketValidator};

#[cfg(any(
    feature = "microcrate_export",
    feature = "microcrate_validate",
    feature = "microcrate_storage",
    feature = "microcrate_notify"
))]
use crate::bdd::Scenario;

#[cfg(feature = "microcrate_export")]
/// Scenario: canonical filenames and zip naming are stable.
pub fn microcrate_export_contract() -> Scenario {
    Scenario::new("Export crate keeps canonical artifact contract stable")
        .when(
            "artifact paths and contract exports are resolved from the microcrate",
            |ctx| {
                let run_dir = Path::new("/tmp/shiplog-contracts");
                let paths = ExportRunArtifactPaths::new(run_dir);
                ctx.paths.insert("packet_md".to_string(), paths.packet_md());
                ctx.paths
                    .insert("ledger_events".to_string(), paths.ledger_events());
                ctx.paths
                    .insert("coverage_manifest".to_string(), paths.coverage_manifest());
                ctx.paths
                    .insert("bundle_manifest".to_string(), paths.bundle_manifest());
                ctx.paths.insert(
                    "manager_profile_packet".to_string(),
                    paths.profile_packet("manager"),
                );

                ctx.strings.insert(
                    "internal_zip".to_string(),
                    export_zip_path_for_profile(run_dir, "internal")
                        .to_string_lossy()
                        .to_string(),
                );
                ctx.strings.insert(
                    "manager_zip".to_string(),
                    export_zip_path_for_profile(run_dir, "manager")
                        .to_string_lossy()
                        .to_string(),
                );

                let data = {
                    let mut data = ExportData::new("contract-run".to_string());
                    data.add_event(ExportEvent {
                        id: "abc".to_string(),
                        source: "github".to_string(),
                        title: "Contract event".to_string(),
                        timestamp: "2025-01-01T00:00:00Z".to_string(),
                    });
                    data
                };
                let payload = export_data(
                    &data,
                    &ExportOptions {
                        format: ExportFormat::Json,
                        pretty: true,
                        include_metadata: true,
                    },
                )
                .map_err(|err| format!("export_data should succeed: {err}"))?;
                ctx.strings.insert("json_output".to_string(), payload);
                Ok(())
            },
        )
        .then("all canonical filenames should be stable", |ctx| {
            let packet_md = crate::bdd::assertions::assert_present(
                ctx.path("packet_md").map(|p| p.to_path_buf()),
                "packet_md",
            )?;
            let ledger = crate::bdd::assertions::assert_present(
                ctx.path("ledger_events").map(|p| p.to_path_buf()),
                "ledger_events",
            )?;
            let coverage = crate::bdd::assertions::assert_present(
                ctx.path("coverage_manifest").map(|p| p.to_path_buf()),
                "coverage_manifest",
            )?;
            let bundle = crate::bdd::assertions::assert_present(
                ctx.path("bundle_manifest").map(|p| p.to_path_buf()),
                "bundle_manifest",
            )?;

            if packet_md.file_name().and_then(|v| v.to_str()) != Some(EXPORT_FILE_PACKET_MD) {
                return Err("packet filename must stay packet.md".to_string());
            }
            if ledger.file_name().and_then(|v| v.to_str()) != Some(EXPORT_FILE_LEDGER_EVENTS_JSONL)
            {
                return Err("ledger filename must stay ledger.events.jsonl".to_string());
            }
            if coverage.file_name().and_then(|v| v.to_str())
                != Some(EXPORT_FILE_COVERAGE_MANIFEST_JSON)
            {
                return Err("coverage filename must stay coverage.manifest.json".to_string());
            }
            if bundle.file_name().and_then(|v| v.to_str()) != Some(EXPORT_FILE_BUNDLE_MANIFEST_JSON)
            {
                return Err("bundle filename must stay bundle.manifest.json".to_string());
            }
            Ok(())
        })
        .then(
            "the profile packet path should be under profiles/manager",
            |ctx| {
                let manager_profile_packet = crate::bdd::assertions::assert_present(
                    ctx.path("manager_profile_packet").map(|p| p.to_path_buf()),
                    "manager_profile_packet",
                )?;
                if manager_profile_packet.file_name().and_then(|v| v.to_str())
                    != Some(EXPORT_FILE_PACKET_MD)
                {
                    return Err("manager profile packet filename must be packet.md".to_string());
                }
                let manager_dir = manager_profile_packet.parent().and_then(|p| p.file_name());
                if manager_dir != Some(std::ffi::OsStr::new("manager")) {
                    return Err(
                        "manager profile packet path should be profiles/manager/packet.md"
                            .to_string(),
                    );
                }
                Ok(())
            },
        )
        .then("zip naming should include profile suffixes", |ctx| {
            let internal =
                crate::bdd::assertions::assert_present(ctx.string("internal_zip"), "internal_zip")?;
            let manager =
                crate::bdd::assertions::assert_present(ctx.string("manager_zip"), "manager_zip")?;
            assert_true(
                internal.ends_with(".zip") && !internal.contains("manager"),
                "internal zip naming",
            )?;
            assert_true(manager.ends_with(".zip"), "manager zip naming")?;
            Ok(())
        })
        .then("JSON export should contain canonical event id", |ctx| {
            let output =
                crate::bdd::assertions::assert_present(ctx.string("json_output"), "json_output")?;
            crate::bdd::assertions::assert_contains(output, "abc", "json output includes event id")
        })
}

#[cfg(feature = "microcrate_output_layout")]
/// Scenario: artifact layout paths are canonical.
pub fn microcrate_output_layout_contract() -> Scenario {
    Scenario::new("Output layout contract keeps canonical paths and zip naming")
        .when("layout paths are derived from a stable API", |ctx| {
            let run_dir = std::path::Path::new("/tmp/shiplog-layout-contract");
            let paths = RunArtifactPaths::new(run_dir);

            ctx.paths.insert("packet_md".to_string(), paths.packet_md());
            ctx.paths
                .insert("ledger_events".to_string(), paths.ledger_events());
            ctx.paths
                .insert("coverage_manifest".to_string(), paths.coverage_manifest());
            ctx.paths
                .insert("bundle_manifest".to_string(), paths.bundle_manifest());
            ctx.paths.insert(
                "manager_packet".to_string(),
                paths.profile_packet(PROFILE_MANAGER),
            );
            ctx.paths.insert(
                "public_packet".to_string(),
                paths.profile_packet(PROFILE_PUBLIC),
            );
            ctx.strings.insert(
                "internal_zip".to_string(),
                zip_path_for_profile(run_dir, PROFILE_INTERNAL)
                    .to_string_lossy()
                    .to_string(),
            );
            ctx.strings.insert(
                "public_zip".to_string(),
                zip_path_for_profile(run_dir, PROFILE_PUBLIC)
                    .to_string_lossy()
                    .to_string(),
            );
            ctx.strings.insert(
                "redaction_aliases_filename".to_string(),
                FILE_REDACTION_ALIASES_JSON.to_string(),
            );
            ctx.strings
                .insert("profiles_dir".to_string(), DIR_PROFILES.to_string());
            ctx.flags.insert(
                "has_redaction_aliases_filename".to_string(),
                !FILE_REDACTION_ALIASES_JSON.is_empty(),
            );
            Ok(())
        })
        .then("all canonical filenames should be stable", |ctx| {
            let packet_md = crate::bdd::assertions::assert_present(
                ctx.path("packet_md").map(|p| p.to_path_buf()),
                "packet_md",
            )?;
            let ledger = crate::bdd::assertions::assert_present(
                ctx.path("ledger_events").map(|p| p.to_path_buf()),
                "ledger_events",
            )?;
            let coverage = crate::bdd::assertions::assert_present(
                ctx.path("coverage_manifest").map(|p| p.to_path_buf()),
                "coverage_manifest",
            )?;
            let bundle = crate::bdd::assertions::assert_present(
                ctx.path("bundle_manifest").map(|p| p.to_path_buf()),
                "bundle_manifest",
            )?;

            if packet_md.file_name().and_then(|v| v.to_str()) != Some(FILE_PACKET_MD) {
                return Err("packet filename must stay packet.md".to_string());
            }
            if ledger.file_name().and_then(|v| v.to_str()) != Some(FILE_LEDGER_EVENTS_JSONL) {
                return Err("ledger filename must stay ledger.events.jsonl".to_string());
            }
            if coverage.file_name().and_then(|v| v.to_str()) != Some(FILE_COVERAGE_MANIFEST_JSON) {
                return Err("coverage filename must stay coverage.manifest.json".to_string());
            }
            if bundle.file_name().and_then(|v| v.to_str()) != Some(FILE_BUNDLE_MANIFEST_JSON) {
                return Err("bundle filename must stay bundle.manifest.json".to_string());
            }
            let expected_redaction_aliases = crate::bdd::assertions::assert_present(
                ctx.string("redaction_aliases_filename").map(String::from),
                "redaction_aliases_filename",
            )?;
            if expected_redaction_aliases != FILE_REDACTION_ALIASES_JSON {
                return Err("redaction alias filename must stay redaction.aliases.json".to_string());
            }

            let manager_packet = crate::bdd::assertions::assert_present(
                ctx.path("manager_packet").map(|p| p.to_path_buf()),
                "manager_packet",
            )?;
            let public_packet = crate::bdd::assertions::assert_present(
                ctx.path("public_packet").map(|p| p.to_path_buf()),
                "public_packet",
            )?;
            if manager_packet.parent().and_then(|p| p.file_name())
                != Some(std::ffi::OsStr::new(PROFILE_MANAGER))
            {
                return Err("manager profile should be under profiles/manager".to_string());
            }
            if public_packet.parent().and_then(|p| p.file_name())
                != Some(std::ffi::OsStr::new(PROFILE_PUBLIC))
            {
                return Err("public profile should be under profiles/public".to_string());
            }
            let has_profiles_root = ctx.string("profiles_dir").unwrap_or_default();
            if has_profiles_root != DIR_PROFILES {
                return Err("profiles dir constant should remain profiles".to_string());
            }
            assert_true(
                ctx.flag("has_redaction_aliases_filename").unwrap_or(false),
                "has redaction alias constant",
            )?;
            Ok(())
        })
        .then("zip naming should include profile suffixes", |ctx| {
            let internal =
                crate::bdd::assertions::assert_present(ctx.string("internal_zip"), "internal_zip")?;
            let public =
                crate::bdd::assertions::assert_present(ctx.string("public_zip"), "public_zip")?;
            assert_true(internal.ends_with(".zip"), "internal zip naming")?;
            assert_true(
                !internal.contains(PROFILE_PUBLIC),
                "internal zip no profile suffix",
            )?;
            assert_true(public.ends_with(".zip"), "public zip naming")?;
            assert_true(
                public.contains(PROFILE_PUBLIC),
                "public zip includes profile suffix",
            )?;
            Ok(())
        })
}

#[cfg(feature = "microcrate_validate")]
/// Scenario: validation contract for ids, sources, and packet shells.
pub fn microcrate_validate_contract() -> Scenario {
    Scenario::new("Validate crate preserves event and packet validation contracts")
        .given("an event id and team source payload", |ctx| {
            ctx.strings
                .insert("good_event_id".to_string(), "a".repeat(64));
            ctx.strings
                .insert("bad_event_id".to_string(), "bad".to_string());
            ctx.strings
                .insert("good_source".to_string(), "github".to_string());
        })
        .when(
            "validation routines run against known-good and known-bad values",
            |ctx| {
                let good_id = ctx.string("good_event_id").unwrap_or("");
                let bad_id = ctx.string("bad_event_id").unwrap_or("");
                let good_source = ctx.string("good_source").unwrap_or("");

                let valid_id = EventValidator::validate_event_id(good_id);
                let invalid_id = EventValidator::validate_event_id(bad_id);
                let valid_source = EventValidator::validate_source(good_source);
                let invalid_source = EventValidator::validate_source("bad-source");

                ctx.flags
                    .insert("valid_event_id".to_string(), valid_id.is_ok());
                ctx.flags
                    .insert("invalid_event_id".to_string(), invalid_id.is_err());
                ctx.flags
                    .insert("valid_source".to_string(), valid_source.is_ok());
                ctx.flags
                    .insert("invalid_source".to_string(), invalid_source.is_err());

                let packet = Packet {
                    id: "contract-packet".to_string(),
                    events: vec!["evt".to_string()],
                };
                let packet_empty = Packet {
                    id: "empty".to_string(),
                    events: vec![],
                };
                ctx.flags.insert(
                    "valid_packet".to_string(),
                    PacketValidator::validate_packet(&packet).is_ok(),
                );
                ctx.flags.insert(
                    "invalid_packet".to_string(),
                    PacketValidator::validate_packet(&packet_empty).is_err(),
                );
                Ok(())
            },
        )
        .then(
            "good values should validate and bad values should fail",
            |ctx| {
                assert_true(
                    ctx.flag("valid_event_id").unwrap_or(false),
                    "good event id validated",
                )?;
                assert_true(
                    ctx.flag("invalid_event_id").unwrap_or(false),
                    "bad event id rejected",
                )?;
                assert_true(
                    ctx.flag("valid_source").unwrap_or(false),
                    "good source validated",
                )?;
                assert_true(
                    ctx.flag("invalid_source").unwrap_or(false),
                    "bad source rejected",
                )?;
                assert_true(
                    ctx.flag("valid_packet").unwrap_or(false),
                    "packet with events valid",
                )?;
                assert_true(
                    ctx.flag("invalid_packet").unwrap_or(false),
                    "packet without events rejected",
                )
            },
        )
}

#[cfg(feature = "microcrate_storage")]
/// Scenario: storage crate keeps a deterministic in-memory contract.
pub fn microcrate_storage_contract() -> Scenario {
    Scenario::new("Storage crate supports deterministic in-memory operations")
        .given("an empty memory-backed store", |ctx| {
            ctx.flags.insert("empty".to_string(), true);
        })
        .when("values are written, read, listed, then deleted", |ctx| {
            let mut store = InMemoryStorage::new();
            let key = StorageKey::from_path("contract/run_001");
            store
                .set(&key, b"payload".to_vec())
                .map_err(|err| format!("set should succeed: {err}"))?;
            let exists = store
                .exists(&key)
                .map_err(|err| format!("exists should succeed: {err}"))?;
            let value = store
                .get(&key)
                .map_err(|err| format!("get should succeed: {err}"))?;
            let listed = store
                .list(&StorageKey::from_path("contract"))
                .map_err(|err| format!("list should succeed: {err}"))?;
            let has_prefix = listed.iter().any(|k| k.0 == key.0);
            store
                .delete(&key)
                .map_err(|err| format!("delete should succeed: {err}"))?;
            let exists_after = store
                .exists(&key)
                .map_err(|err| format!("exists should succeed after delete: {err}"))?;

            ctx.flags
                .insert("write_read_roundtrip".to_string(), value.is_some());
            ctx.flags.insert("exists_before_delete".to_string(), exists);
            ctx.flags.insert("has_prefix_key".to_string(), has_prefix);
            ctx.flags
                .insert("missing_after_delete".to_string(), !exists_after);
            Ok(())
        })
        .then(
            "write/read/list/delete should behave deterministically",
            |ctx| {
                assert_true(
                    ctx.flag("write_read_roundtrip").unwrap_or(false),
                    "write/read roundtrip",
                )?;
                assert_true(
                    ctx.flag("exists_before_delete").unwrap_or(false),
                    "exists before delete",
                )?;
                assert_true(ctx.flag("has_prefix_key").unwrap_or(false), "key listed")?;
                assert_true(
                    ctx.flag("missing_after_delete").unwrap_or(false),
                    "key removed",
                )
            },
        )
}

#[cfg(feature = "microcrate_notify")]
/// Scenario: notification crate exposes constructor and send contracts.
pub fn microcrate_notify_contract() -> Scenario {
    Scenario::new("Notify crate provides a stable notification contract")
        .given("a default notification service", |ctx| {
            ctx.flags.insert("service_ready".to_string(), true);
        })
        .when("a high-priority packet notification is sent", |ctx| {
            let service = NotificationService::default();
            let note = Notification::alert("Contract Packet", "Stable contract validation");
            service
                .send(&note)
                .map_err(|err| format!("send should succeed: {err}"))?;
            ctx.strings
                .insert("priority".to_string(), format!("{:?}", note.priority));
            ctx.flags.insert("notification_sent".to_string(), true);
            Ok(())
        })
        .then(
            "the notification should be produced with a stable alert priority",
            |ctx| {
                assert_true(
                    ctx.flag("notification_sent").unwrap_or(false),
                    "notification sent",
                )?;
                assert_true(
                    ctx.string("priority").unwrap_or("").contains("High"),
                    "notification priority",
                )
            },
        )
}
