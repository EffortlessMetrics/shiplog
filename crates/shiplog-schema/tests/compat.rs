//! Backward-compatibility and forward-compatibility tests for shiplog-schema.
//!
//! Each test embeds an inline JSON/YAML string representing an "older" or
//! "future" wire format and verifies the current code still deserialises it.

use shiplog_schema::coverage::{Completeness, CoverageManifest, CoverageSlice};
use shiplog_schema::event::*;
use shiplog_schema::workstream::{WorkstreamStats, WorkstreamsFile};

// ── v0.1-era EventEnvelope: object-form SourceSystem, PascalCase enums ──

/// Simulates a v0.1 PR event where SourceSystem was serialised as an
/// object (`{"Github": null}`) and enum variants were PascalCase.
#[test]
fn v01_event_envelope_pr_object_source() {
    let json = r#"{
        "id": "sha256:abc123",
        "kind": "PullRequest",
        "occurred_at": "2025-01-15T10:00:00Z",
        "actor": { "login": "octocat", "id": 1 },
        "repo": {
            "full_name": "acme/widgets",
            "html_url": "https://github.com/acme/widgets",
            "visibility": "Public"
        },
        "payload": {
            "type": "PullRequest",
            "data": {
                "number": 42,
                "title": "Add feature",
                "state": "Merged",
                "created_at": "2025-01-10T09:00:00Z",
                "merged_at": "2025-01-15T10:00:00Z",
                "additions": 100,
                "deletions": 20,
                "changed_files": 5,
                "touched_paths_hint": ["src/lib.rs"],
                "window": null
            }
        },
        "tags": ["feature"],
        "links": [],
        "source": {
            "system": {"Github": null},
            "url": null,
            "opaque_id": "PR_abc"
        }
    }"#;

    let ev: EventEnvelope = serde_json::from_str(json).unwrap();
    assert_eq!(ev.source.system, SourceSystem::Github);
    assert_eq!(ev.kind, EventKind::PullRequest);
    if let EventPayload::PullRequest(pr) = &ev.payload {
        assert_eq!(pr.number, 42);
        assert_eq!(pr.state, PullRequestState::Merged);
    } else {
        panic!("expected PR payload");
    }
}

/// v0.1 review event with object-form SourceSystem `{"Other": "gitlab"}`.
#[test]
fn v01_event_envelope_review_other_source() {
    let json = r#"{
        "id": "sha256:def456",
        "kind": "Review",
        "occurred_at": "2025-02-01T14:30:00Z",
        "actor": { "login": "reviewer", "id": null },
        "repo": {
            "full_name": "acme/widgets",
            "html_url": null,
            "visibility": "Private"
        },
        "payload": {
            "type": "Review",
            "data": {
                "pull_number": 42,
                "pull_title": "Add feature",
                "submitted_at": "2025-02-01T14:30:00Z",
                "state": "approved",
                "window": null
            }
        },
        "tags": [],
        "links": [],
        "source": {
            "system": {"Other": "gitlab"},
            "url": null,
            "opaque_id": null
        }
    }"#;

    let ev: EventEnvelope = serde_json::from_str(json).unwrap();
    assert_eq!(ev.source.system, SourceSystem::Other("gitlab".into()));
    assert_eq!(ev.kind, EventKind::Review);
}

// ── v0.2 format: lowercase string SourceSystem ──────────────────────────

#[test]
fn v02_event_envelope_pr_string_source() {
    let json = r#"{
        "id": "sha256:ghi789",
        "kind": "PullRequest",
        "occurred_at": "2025-03-01T08:00:00Z",
        "actor": { "login": "dev", "id": 99 },
        "repo": {
            "full_name": "acme/api",
            "html_url": "https://github.com/acme/api",
            "visibility": "Public"
        },
        "payload": {
            "type": "PullRequest",
            "data": {
                "number": 7,
                "title": "Fix bug",
                "state": "Merged",
                "created_at": "2025-02-28T12:00:00Z",
                "merged_at": "2025-03-01T08:00:00Z",
                "additions": 5,
                "deletions": 2,
                "changed_files": 1,
                "touched_paths_hint": [],
                "window": null
            }
        },
        "tags": [],
        "links": [],
        "source": {
            "system": "github",
            "url": null,
            "opaque_id": null
        }
    }"#;

    let ev: EventEnvelope = serde_json::from_str(json).unwrap();
    assert_eq!(ev.source.system, SourceSystem::Github);
}

// ── SourceSystem backward compat: PascalCase and lowercase both parse ───

#[test]
fn source_system_pascal_and_lowercase_all_variants() {
    let cases = [
        (r#""github""#, SourceSystem::Github),
        (r#""Github""#, SourceSystem::Github),
        (r#""GITHUB""#, SourceSystem::Github),
        (r#""json_import""#, SourceSystem::JsonImport),
        (r#""JsonImport""#, SourceSystem::JsonImport),
        (r#""local_git""#, SourceSystem::LocalGit),
        (r#""LocalGit""#, SourceSystem::LocalGit),
        (r#""manual""#, SourceSystem::Manual),
        (r#""Manual""#, SourceSystem::Manual),
        (r#""unknown""#, SourceSystem::Unknown),
        (r#""Unknown""#, SourceSystem::Unknown),
    ];
    for (json, expected) in cases {
        let got: SourceSystem = serde_json::from_str(json).unwrap();
        assert_eq!(got, expected, "parsing {json}");
    }
}

#[test]
fn source_system_object_forms_all_variants() {
    let cases = [
        (r#"{"Github": null}"#, SourceSystem::Github),
        (r#"{"JsonImport": null}"#, SourceSystem::JsonImport),
        (r#"{"LocalGit": null}"#, SourceSystem::LocalGit),
        (r#"{"Manual": null}"#, SourceSystem::Manual),
        (r#"{"Unknown": null}"#, SourceSystem::Unknown),
        (r#"{"Other": "jira"}"#, SourceSystem::Other("jira".into())),
        // Object-form Other normalises known names
        (r#"{"Other": "github"}"#, SourceSystem::Github),
    ];
    for (json, expected) in cases {
        let got: SourceSystem = serde_json::from_str(json).unwrap();
        assert_eq!(got, expected, "parsing {json}");
    }
}

// ── ManualDate backward compat ──────────────────────────────────────────

#[test]
fn manual_date_single_string() {
    let json = r#""2025-06-01""#;
    let d: ManualDate = serde_json::from_str(json).unwrap();
    assert_eq!(
        d,
        ManualDate::Single(chrono::NaiveDate::from_ymd_opt(2025, 6, 1).unwrap())
    );
}

#[test]
fn manual_date_range_object() {
    let json = r#"{"start": "2025-01-01", "end": "2025-03-31"}"#;
    let d: ManualDate = serde_json::from_str(json).unwrap();
    assert_eq!(
        d,
        ManualDate::Range {
            start: chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            end: chrono::NaiveDate::from_ymd_opt(2025, 3, 31).unwrap(),
        }
    );
}

// ── Forward compat: unknown fields are ignored ──────────────────────────

#[test]
fn event_envelope_ignores_unknown_top_level_fields() {
    let json = r#"{
        "id": "sha256:fwd1",
        "kind": "PullRequest",
        "occurred_at": "2025-06-01T00:00:00Z",
        "actor": { "login": "a", "id": null },
        "repo": { "full_name": "x/y", "html_url": null, "visibility": "Public" },
        "payload": {
            "type": "PullRequest",
            "data": {
                "number": 1,
                "title": "t",
                "state": "Open",
                "created_at": "2025-06-01T00:00:00Z",
                "merged_at": null,
                "additions": null,
                "deletions": null,
                "changed_files": null,
                "touched_paths_hint": [],
                "window": null
            }
        },
        "tags": [],
        "links": [],
        "source": { "system": "github", "url": null, "opaque_id": null },
        "future_field": "should be ignored",
        "another_future": 42
    }"#;

    let ev: EventEnvelope = serde_json::from_str(json).unwrap();
    assert_eq!(ev.id.0, "sha256:fwd1");
}

#[test]
fn actor_ignores_unknown_fields() {
    let json = r#"{"login": "bot", "id": 5, "avatar_url": "https://img", "extra": true}"#;
    let actor: Actor = serde_json::from_str(json).unwrap();
    assert_eq!(actor.login, "bot");
    assert_eq!(actor.id, Some(5));
}

#[test]
fn repo_ref_ignores_unknown_fields() {
    let json = r#"{
        "full_name": "org/repo",
        "html_url": null,
        "visibility": "Public",
        "stars": 999,
        "archived": false
    }"#;
    let r: RepoRef = serde_json::from_str(json).unwrap();
    assert_eq!(r.full_name, "org/repo");
}

#[test]
fn pr_event_ignores_unknown_fields() {
    let json = r#"{
        "number": 1,
        "title": "t",
        "state": "Open",
        "created_at": "2025-06-01T00:00:00Z",
        "merged_at": null,
        "additions": null,
        "deletions": null,
        "changed_files": null,
        "touched_paths_hint": [],
        "window": null,
        "draft": true,
        "review_decision": "APPROVED"
    }"#;
    let pr: PullRequestEvent = serde_json::from_str(json).unwrap();
    assert_eq!(pr.number, 1);
}

#[test]
fn review_event_ignores_unknown_fields() {
    let json = r#"{
        "pull_number": 42,
        "pull_title": "feat",
        "submitted_at": "2025-06-01T00:00:00Z",
        "state": "approved",
        "window": null,
        "body": "LGTM",
        "is_dismissed": false
    }"#;
    let r: ReviewEvent = serde_json::from_str(json).unwrap();
    assert_eq!(r.pull_number, 42);
}

#[test]
fn manual_event_ignores_unknown_fields() {
    let json = r#"{
        "event_type": "Note",
        "title": "thing",
        "description": null,
        "started_at": null,
        "ended_at": null,
        "impact": null,
        "priority": "high",
        "confidence": 0.9
    }"#;
    let m: ManualEvent = serde_json::from_str(json).unwrap();
    assert_eq!(m.title, "thing");
}

#[test]
fn coverage_manifest_ignores_unknown_fields() {
    let json = r#"{
        "run_id": "r1",
        "generated_at": "2025-06-01T00:00:00Z",
        "user": "u",
        "window": { "since": "2025-01-01", "until": "2025-04-01" },
        "mode": "merged",
        "sources": [],
        "slices": [],
        "warnings": [],
        "completeness": "Complete",
        "new_metric": 123
    }"#;
    let cm: CoverageManifest = serde_json::from_str(json).unwrap();
    assert_eq!(cm.completeness, Completeness::Complete);
}

#[test]
fn coverage_slice_ignores_unknown_fields() {
    let json = r#"{
        "window": { "since": "2025-01-01", "until": "2025-02-01" },
        "query": "q",
        "total_count": 10,
        "fetched": 10,
        "incomplete_results": false,
        "notes": [],
        "api_version": "2025-01-01"
    }"#;
    let cs: CoverageSlice = serde_json::from_str(json).unwrap();
    assert_eq!(cs.total_count, 10);
}

// ── serde skip_serializing_if behavior ──────────────────────────────────
// The schema does NOT use skip_serializing_if, so None fields serialize as
// `null`. Verify this contract is preserved (consumers rely on field presence).

#[test]
fn none_fields_serialize_as_null_not_omitted() {
    let actor = Actor {
        login: "a".into(),
        id: None,
    };
    let json = serde_json::to_string(&actor).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(
        v.get("id").is_some(),
        "None field 'id' must be present as null, got: {json}"
    );
    assert!(v["id"].is_null());
}

#[test]
fn none_optional_fields_present_in_pr_event() {
    let pr = PullRequestEvent {
        number: 1,
        title: "t".into(),
        state: PullRequestState::Open,
        created_at: chrono::Utc::now(),
        merged_at: None,
        additions: None,
        deletions: None,
        changed_files: None,
        touched_paths_hint: vec![],
        window: None,
    };
    let json = serde_json::to_string(&pr).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    for key in ["merged_at", "additions", "deletions", "changed_files", "window"] {
        assert!(
            v.get(key).is_some(),
            "Optional field '{key}' must be present as null"
        );
    }
}

#[test]
fn none_optional_fields_present_in_source_ref() {
    let sr = SourceRef {
        system: SourceSystem::Github,
        url: None,
        opaque_id: None,
    };
    let json = serde_json::to_string(&sr).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    for key in ["url", "opaque_id"] {
        assert!(
            v.get(key).is_some(),
            "Optional field '{key}' must be present as null"
        );
    }
}

// ── WorkstreamsFile backward compat ─────────────────────────────────────

#[test]
fn workstreams_file_v01_minimal() {
    // Older format with no summary, no tags, bare-bones stats
    let yaml = r#"
version: 1
generated_at: "2025-01-01T00:00:00Z"
workstreams:
  - id: "sha256:ws1"
    title: "Auth"
    summary: null
    tags: []
    stats:
      pull_requests: 3
      reviews: 1
      manual_events: 0
    events: ["sha256:e1"]
    receipts: []
"#;
    let wf: WorkstreamsFile = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(wf.workstreams.len(), 1);
    assert_eq!(wf.workstreams[0].title, "Auth");
    assert!(wf.workstreams[0].summary.is_none());
}

#[test]
fn workstreams_file_ignores_unknown_fields_yaml() {
    let yaml = r#"
version: 1
generated_at: "2025-06-01T00:00:00Z"
schema_version: "0.3.0"
workstreams:
  - id: "sha256:ws1"
    title: "Infra"
    summary: "K8s migration"
    tags: ["infra"]
    stats:
      pull_requests: 2
      reviews: 0
      manual_events: 1
    events: ["sha256:e1"]
    receipts: ["sha256:e1"]
    priority: "high"
    confidence: 0.95
"#;
    let wf: WorkstreamsFile = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(wf.workstreams[0].title, "Infra");
}

#[test]
fn workstream_stats_ignores_unknown_fields() {
    let json = r#"{"pull_requests": 1, "reviews": 2, "manual_events": 3, "deployments": 5}"#;
    let stats: WorkstreamStats = serde_json::from_str(json).unwrap();
    assert_eq!(stats.pull_requests, 1);
    assert_eq!(stats.reviews, 2);
    assert_eq!(stats.manual_events, 3);
}
