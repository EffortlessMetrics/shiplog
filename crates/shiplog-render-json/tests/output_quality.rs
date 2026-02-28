//! Output-quality tests for the JSON renderer.
//!
//! Validates JSON schema compliance (required fields), null/missing field handling,
//! array ordering consistency, and pretty-print vs compact output.

use chrono::{NaiveDate, TimeZone, Utc};
use shiplog_ids::{EventId, RunId};
use shiplog_render_json::{write_coverage_manifest, write_events_jsonl};
use shiplog_schema::coverage::{Completeness, CoverageManifest, CoverageSlice, TimeWindow};
use shiplog_schema::event::*;
use shiplog_testkit::pr_event;

// ── Helpers ──────────────────────────────────────────────────────────────

fn deterministic_coverage(completeness: Completeness) -> CoverageManifest {
    CoverageManifest {
        run_id: RunId("quality_test_run_2025".into()),
        generated_at: Utc.timestamp_opt(1_704_067_200, 0).unwrap(), // 2024-01-01T00:00:00Z
        user: "testuser".into(),
        window: TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
        },
        mode: "merged".into(),
        sources: vec!["github".into()],
        slices: vec![],
        warnings: vec![],
        completeness,
    }
}

// ── JSON schema compliance (required fields present) ────────────────────

#[test]
fn event_json_has_all_required_fields() {
    let events = vec![pr_event("acme/repo", 1, "Test PR")];
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.jsonl");
    write_events_jsonl(&path, &events).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    let json: serde_json::Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
    let obj = json.as_object().unwrap();

    for field in &[
        "id",
        "kind",
        "occurred_at",
        "actor",
        "repo",
        "payload",
        "tags",
        "links",
        "source",
    ] {
        assert!(
            obj.contains_key(*field),
            "Event JSON missing required field: {field}"
        );
    }
}

#[test]
fn event_actor_has_required_fields() {
    let events = vec![pr_event("acme/repo", 1, "Test PR")];
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.jsonl");
    write_events_jsonl(&path, &events).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    let json: serde_json::Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
    let actor = json.get("actor").unwrap().as_object().unwrap();

    assert!(actor.contains_key("login"), "Actor missing 'login'");
    assert!(actor.contains_key("id"), "Actor missing 'id'");
}

#[test]
fn event_repo_has_required_fields() {
    let events = vec![pr_event("acme/repo", 1, "Test PR")];
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.jsonl");
    write_events_jsonl(&path, &events).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    let json: serde_json::Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
    let repo = json.get("repo").unwrap().as_object().unwrap();

    assert!(repo.contains_key("full_name"), "Repo missing 'full_name'");
    assert!(repo.contains_key("html_url"), "Repo missing 'html_url'");
    assert!(repo.contains_key("visibility"), "Repo missing 'visibility'");
}

#[test]
fn event_source_has_required_fields() {
    let events = vec![pr_event("acme/repo", 1, "Test PR")];
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.jsonl");
    write_events_jsonl(&path, &events).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    let json: serde_json::Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
    let source = json.get("source").unwrap().as_object().unwrap();

    assert!(source.contains_key("system"), "Source missing 'system'");
    assert!(source.contains_key("url"), "Source missing 'url'");
    assert!(
        source.contains_key("opaque_id"),
        "Source missing 'opaque_id'"
    );
}

#[test]
fn coverage_manifest_has_all_required_fields() {
    let cov = deterministic_coverage(Completeness::Complete);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("coverage.manifest.json");
    write_coverage_manifest(&path, &cov).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    let obj = json.as_object().unwrap();

    for field in &[
        "run_id",
        "generated_at",
        "user",
        "window",
        "mode",
        "sources",
        "slices",
        "warnings",
        "completeness",
    ] {
        assert!(
            obj.contains_key(*field),
            "Coverage manifest missing required field: {field}"
        );
    }
}

// ── Null / missing field handling ───────────────────────────────────────

#[test]
fn null_optional_fields_serialized_correctly() {
    // Event with all optionals set to None
    let event = EventEnvelope {
        id: EventId::from_parts(["test", "nulls"]),
        kind: EventKind::PullRequest,
        occurred_at: Utc.timestamp_opt(0, 0).unwrap(),
        actor: Actor {
            login: "user".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: "acme/repo".into(),
            html_url: None,
            visibility: RepoVisibility::Unknown,
        },
        payload: EventPayload::PullRequest(PullRequestEvent {
            number: 1,
            title: "Null test".into(),
            state: PullRequestState::Open,
            created_at: Utc.timestamp_opt(0, 0).unwrap(),
            merged_at: None,
            additions: None,
            deletions: None,
            changed_files: None,
            touched_paths_hint: vec![],
            window: None,
        }),
        tags: vec![],
        links: vec![],
        source: SourceRef {
            system: SourceSystem::Github,
            url: None,
            opaque_id: None,
        },
    };

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.jsonl");
    write_events_jsonl(&path, &[event]).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    let json: serde_json::Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();

    // Null fields should still be present in JSON as null
    assert!(json["actor"]["id"].is_null(), "actor.id should be null");
    assert!(
        json["repo"]["html_url"].is_null(),
        "repo.html_url should be null"
    );
    assert!(json["source"]["url"].is_null(), "source.url should be null");
    assert!(
        json["source"]["opaque_id"].is_null(),
        "source.opaque_id should be null"
    );

    insta::assert_snapshot!("null_optional_fields_jsonl", text);
}

#[test]
fn empty_arrays_serialized_as_empty_json_arrays() {
    let events = vec![pr_event("acme/repo", 1, "Test")];
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.jsonl");
    write_events_jsonl(&path, &events).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    let json: serde_json::Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();

    assert!(json["tags"].is_array(), "tags should be an array");
    assert_eq!(json["tags"].as_array().unwrap().len(), 0);

    let payload = &json["payload"]["data"];
    assert!(
        payload["touched_paths_hint"].is_array(),
        "touched_paths_hint should be an array"
    );
}

// ── Array ordering consistency ──────────────────────────────────────────

#[test]
fn event_ordering_preserved_across_writes() {
    let events: Vec<_> = (1..=10)
        .map(|i| pr_event("acme/repo", i, &format!("PR #{i}")))
        .collect();

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.jsonl");
    write_events_jsonl(&path, &events).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    let parsed: Vec<EventEnvelope> = text
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();

    assert_eq!(parsed.len(), events.len());
    for (i, (orig, loaded)) in events.iter().zip(parsed.iter()).enumerate() {
        assert_eq!(orig.id, loaded.id, "Event order mismatch at index {i}");
    }
}

#[test]
fn event_ordering_deterministic_across_repeated_writes() {
    let events: Vec<_> = (1..=5)
        .map(|i| pr_event("acme/repo", i, &format!("PR #{i}")))
        .collect();

    let dir = tempfile::tempdir().unwrap();
    let path1 = dir.path().join("events1.jsonl");
    let path2 = dir.path().join("events2.jsonl");

    write_events_jsonl(&path1, &events).unwrap();
    write_events_jsonl(&path2, &events).unwrap();

    let text1 = std::fs::read_to_string(&path1).unwrap();
    let text2 = std::fs::read_to_string(&path2).unwrap();

    assert_eq!(
        text1, text2,
        "Repeated writes of same events should produce identical output"
    );
}

#[test]
fn coverage_sources_order_preserved() {
    let mut cov = deterministic_coverage(Completeness::Complete);
    cov.sources = vec!["github".into(), "manual".into(), "json_import".into()];

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("coverage.manifest.json");
    write_coverage_manifest(&path, &cov).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    let loaded: CoverageManifest = serde_json::from_str(&text).unwrap();

    assert_eq!(
        loaded.sources, cov.sources,
        "Source order should be preserved"
    );
}

// ── Pretty-print vs compact output ─────────────────────────────────────

#[test]
fn jsonl_events_are_compact_one_per_line() {
    let events = vec![
        pr_event("acme/repo", 1, "First"),
        pr_event("acme/repo", 2, "Second"),
    ];

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.jsonl");
    write_events_jsonl(&path, &events).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    let lines: Vec<_> = text.lines().collect();

    assert_eq!(lines.len(), 2, "Should have exactly 2 lines");

    // Each line should be valid JSON and not contain newlines (compact)
    for (i, line) in lines.iter().enumerate() {
        assert!(
            !line.contains('\n'),
            "Line {i} should not contain embedded newlines"
        );
        let parsed: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("Line {i} is not valid JSON: {e}"));
        assert!(parsed.is_object(), "Line {i} should be a JSON object");
    }
}

#[test]
fn jsonl_lines_end_with_newline() {
    let events = vec![pr_event("acme/repo", 1, "Test")];

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.jsonl");
    write_events_jsonl(&path, &events).unwrap();

    let bytes = std::fs::read(&path).unwrap();
    assert!(
        bytes.ends_with(b"\n"),
        "JSONL file should end with a newline"
    );
}

#[test]
fn coverage_manifest_is_pretty_printed() {
    let cov = deterministic_coverage(Completeness::Complete);

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("coverage.manifest.json");
    write_coverage_manifest(&path, &cov).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();

    // Pretty-printed JSON has newlines and indentation
    assert!(text.contains('\n'), "Manifest should be multi-line");
    assert!(text.contains("  "), "Manifest should contain indentation");

    // But still valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(parsed.is_object());
}

#[test]
fn jsonl_compact_is_shorter_than_pretty() {
    let events = vec![pr_event("acme/repo", 1, "Test")];

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.jsonl");
    write_events_jsonl(&path, &events).unwrap();

    let compact_text = std::fs::read_to_string(&path).unwrap();
    let compact_line = compact_text.lines().next().unwrap();

    // Re-serialize as pretty to compare
    let json: serde_json::Value = serde_json::from_str(compact_line).unwrap();
    let pretty = serde_json::to_string_pretty(&json).unwrap();

    assert!(
        compact_line.len() < pretty.len(),
        "Compact JSONL ({} bytes) should be shorter than pretty ({} bytes)",
        compact_line.len(),
        pretty.len()
    );
}

// ── Snapshot: coverage manifest with slices and warnings ────────────────

#[test]
fn snapshot_coverage_with_multiple_slices_and_warnings() {
    let mut cov = deterministic_coverage(Completeness::Partial);
    cov.slices = vec![
        CoverageSlice {
            window: TimeWindow {
                since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
            },
            query: "author:testuser is:merged".into(),
            total_count: 200,
            fetched: 100,
            incomplete_results: Some(true),
            notes: vec!["Rate limited".into()],
        },
        CoverageSlice {
            window: TimeWindow {
                since: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
                until: NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
            },
            query: "author:testuser is:merged".into(),
            total_count: 50,
            fetched: 50,
            incomplete_results: Some(false),
            notes: vec![],
        },
    ];
    cov.warnings = vec![
        "Some data may be missing due to API limits".into(),
        "Review data was not available for 3 repos".into(),
    ];

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("coverage.manifest.json");
    write_coverage_manifest(&path, &cov).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    insta::assert_snapshot!("coverage_manifest_slices_and_warnings", text);
}

// ── Snapshot: event with all optional fields populated ──────────────────

#[test]
fn snapshot_event_with_all_fields_populated() {
    let event = EventEnvelope {
        id: EventId::from_parts(["github", "pr", "acme/repo", "99"]),
        kind: EventKind::PullRequest,
        occurred_at: Utc.timestamp_opt(1_704_067_200, 0).unwrap(),
        actor: Actor {
            login: "octocat".into(),
            id: Some(12345),
        },
        repo: RepoRef {
            full_name: "acme/repo".into(),
            html_url: Some("https://github.com/acme/repo".into()),
            visibility: RepoVisibility::Public,
        },
        payload: EventPayload::PullRequest(PullRequestEvent {
            number: 99,
            title: "Fully populated event".into(),
            state: PullRequestState::Merged,
            created_at: Utc.timestamp_opt(1_704_067_200, 0).unwrap(),
            merged_at: Some(Utc.timestamp_opt(1_704_153_600, 0).unwrap()),
            additions: Some(150),
            deletions: Some(30),
            changed_files: Some(8),
            touched_paths_hint: vec!["src/lib.rs".into(), "tests/main.rs".into()],
            window: Some(TimeWindow {
                since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
            }),
        }),
        tags: vec!["feature".into(), "high-priority".into()],
        links: vec![
            Link {
                label: "pr".into(),
                url: "https://github.com/acme/repo/pull/99".into(),
            },
            Link {
                label: "issue".into(),
                url: "https://github.com/acme/repo/issues/50".into(),
            },
        ],
        source: SourceRef {
            system: SourceSystem::Github,
            url: Some("https://api.github.com/repos/acme/repo/pulls/99".into()),
            opaque_id: Some("PR_abc123".into()),
        },
    };

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.jsonl");
    write_events_jsonl(&path, &[event]).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    insta::assert_snapshot!("event_all_fields_populated", text);
}

// ── Roundtrip fidelity for review and manual events ─────────────────────

#[test]
fn review_event_roundtrip_preserves_state() {
    let event = EventEnvelope {
        id: EventId::from_parts(["review", "roundtrip"]),
        kind: EventKind::Review,
        occurred_at: Utc.timestamp_opt(0, 0).unwrap(),
        actor: Actor {
            login: "reviewer".into(),
            id: Some(999),
        },
        repo: RepoRef {
            full_name: "acme/repo".into(),
            html_url: None,
            visibility: RepoVisibility::Private,
        },
        payload: EventPayload::Review(ReviewEvent {
            pull_number: 42,
            pull_title: "Important fix".into(),
            submitted_at: Utc.timestamp_opt(0, 0).unwrap(),
            state: "changes_requested".into(),
            window: None,
        }),
        tags: vec!["review".into()],
        links: vec![],
        source: SourceRef {
            system: SourceSystem::Github,
            url: None,
            opaque_id: None,
        },
    };

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.jsonl");
    write_events_jsonl(&path, std::slice::from_ref(&event)).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    let loaded: EventEnvelope = serde_json::from_str(text.lines().next().unwrap()).unwrap();

    assert_eq!(loaded.id, event.id);
    assert_eq!(loaded.kind, EventKind::Review);
    if let EventPayload::Review(r) = &loaded.payload {
        assert_eq!(r.state, "changes_requested");
        assert_eq!(r.pull_number, 42);
    } else {
        panic!("Expected Review payload");
    }
}

#[test]
fn manual_event_roundtrip_preserves_type() {
    let event = EventEnvelope {
        id: EventId::from_parts(["manual", "roundtrip"]),
        kind: EventKind::Manual,
        occurred_at: Utc.timestamp_opt(0, 0).unwrap(),
        actor: Actor {
            login: "user".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: "acme/repo".into(),
            html_url: None,
            visibility: RepoVisibility::Unknown,
        },
        payload: EventPayload::Manual(ManualEvent {
            event_type: ManualEventType::Design,
            title: "Architecture review session".into(),
            description: Some("Reviewed microservice boundaries".into()),
            started_at: Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
            ended_at: Some(NaiveDate::from_ymd_opt(2025, 1, 3).unwrap()),
            impact: Some("Improved service isolation".into()),
        }),
        tags: vec!["design".into(), "architecture".into()],
        links: vec![Link {
            label: "doc".into(),
            url: "https://docs.example.com/arch-review".into(),
        }],
        source: SourceRef {
            system: SourceSystem::Manual,
            url: None,
            opaque_id: Some("manual-1".into()),
        },
    };

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.jsonl");
    write_events_jsonl(&path, std::slice::from_ref(&event)).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    let loaded: EventEnvelope = serde_json::from_str(text.lines().next().unwrap()).unwrap();

    assert_eq!(loaded, event);
}
