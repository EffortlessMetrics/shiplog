//! Edge case tests for shiplog-redact.
//!
//! Covers: very long strings, unicode in all fields, empty strings,
//! profile differentiation, and redaction consistency across calls.

use chrono::{NaiveDate, Utc};
use shiplog_ids::{EventId, WorkstreamId};
use shiplog_ports::Redactor;
use shiplog_redact::DeterministicRedactor;
use shiplog_schema::event::*;
use shiplog_schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn pr_event_with(title: &str, repo: &str, paths: Vec<String>) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["edge", "pr", "1"]),
        kind: EventKind::PullRequest,
        occurred_at: Utc::now(),
        actor: Actor {
            login: "dev".into(),
            id: Some(42),
        },
        repo: RepoRef {
            full_name: repo.into(),
            html_url: Some(format!("https://github.com/{repo}")),
            visibility: RepoVisibility::Private,
        },
        payload: EventPayload::PullRequest(PullRequestEvent {
            number: 1,
            title: title.into(),
            state: PullRequestState::Merged,
            created_at: Utc::now(),
            merged_at: Some(Utc::now()),
            additions: Some(10),
            deletions: Some(3),
            changed_files: Some(2),
            touched_paths_hint: paths,
            window: None,
        }),
        tags: vec!["feature".into()],
        links: vec![Link {
            label: "pr".into(),
            url: format!("https://github.com/{repo}/pull/1"),
        }],
        source: SourceRef {
            system: SourceSystem::Github,
            url: Some(format!("https://api.github.com/repos/{repo}/pulls/1")),
            opaque_id: None,
        },
    }
}

fn review_event_with(pull_title: &str, repo: &str) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["edge", "review", "1"]),
        kind: EventKind::Review,
        occurred_at: Utc::now(),
        actor: Actor {
            login: "reviewer".into(),
            id: Some(7),
        },
        repo: RepoRef {
            full_name: repo.into(),
            html_url: Some(format!("https://github.com/{repo}")),
            visibility: RepoVisibility::Private,
        },
        payload: EventPayload::Review(ReviewEvent {
            pull_number: 1,
            pull_title: pull_title.into(),
            submitted_at: Utc::now(),
            state: "approved".into(),
            window: None,
        }),
        tags: vec![],
        links: vec![Link {
            label: "review".into(),
            url: format!("https://github.com/{repo}/pull/1#review"),
        }],
        source: SourceRef {
            system: SourceSystem::Github,
            url: Some(format!(
                "https://api.github.com/repos/{repo}/pulls/1/reviews/1"
            )),
            opaque_id: None,
        },
    }
}

fn manual_event_with(
    title: &str,
    description: Option<&str>,
    impact: Option<&str>,
) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["edge", "manual", "1"]),
        kind: EventKind::Manual,
        occurred_at: Utc::now(),
        actor: Actor {
            login: "dev".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: "org/repo".into(),
            html_url: None,
            visibility: RepoVisibility::Private,
        },
        payload: EventPayload::Manual(ManualEvent {
            event_type: ManualEventType::Incident,
            title: title.into(),
            description: description.map(String::from),
            started_at: Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
            ended_at: Some(NaiveDate::from_ymd_opt(2025, 1, 2).unwrap()),
            impact: impact.map(String::from),
        }),
        tags: vec!["incident".into()],
        links: vec![Link {
            label: "postmortem".into(),
            url: "https://internal.wiki/incident/42".into(),
        }],
        source: SourceRef {
            system: SourceSystem::Manual,
            url: Some("https://internal.api/incidents/42".into()),
            opaque_id: None,
        },
    }
}

fn make_workstreams_file(titles: &[(&str, Option<&str>, Vec<&str>)]) -> WorkstreamsFile {
    WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: titles
            .iter()
            .enumerate()
            .map(|(i, (title, summary, tags))| Workstream {
                id: WorkstreamId::from_parts(["ws", &i.to_string()]),
                title: (*title).into(),
                summary: summary.map(|s| s.to_string()),
                tags: tags.iter().map(|t| t.to_string()).collect(),
                stats: WorkstreamStats {
                    pull_requests: 3,
                    reviews: 2,
                    manual_events: 1,
                },
                events: vec![],
                receipts: vec![],
            })
            .collect(),
    }
}

// ===========================================================================
// Very long strings
// ===========================================================================

#[test]
fn very_long_title_does_not_degrade_redaction() {
    let r = DeterministicRedactor::new(b"long-key");
    let long_title = "A".repeat(100_000);
    let event = pr_event_with(&long_title, "org/repo", vec![]);
    let out = r.redact_events(&[event], "public").unwrap();

    match &out[0].payload {
        EventPayload::PullRequest(pr) => assert_eq!(pr.title, "[redacted]"),
        _ => panic!("expected PR"),
    }
    // Original title should not appear in JSON
    let json = serde_json::to_string(&out).unwrap();
    assert!(!json.contains(&long_title));
}

#[test]
fn very_long_repo_name_aliases_deterministically() {
    let r = DeterministicRedactor::new(b"long-key");
    let long_repo = format!("org/{}", "x".repeat(50_000));
    let event = pr_event_with("title", &long_repo, vec![]);

    let out1 = r
        .redact_events(std::slice::from_ref(&event), "public")
        .unwrap();
    let out2 = r.redact_events(&[event], "public").unwrap();

    assert_eq!(out1[0].repo.full_name, out2[0].repo.full_name);
    assert_ne!(out1[0].repo.full_name, long_repo);
}

#[test]
fn many_events_redact_without_panic() {
    let r = DeterministicRedactor::new(b"batch-key");
    let events: Vec<_> = (0..1_000)
        .map(|i| pr_event_with(&format!("PR {i}"), &format!("org/repo-{i}"), vec![]))
        .collect();

    let out = r.redact_events(&events, "public").unwrap();
    assert_eq!(out.len(), 1_000);
}

#[test]
fn many_paths_cleared_efficiently() {
    let r = DeterministicRedactor::new(b"paths-key");
    let paths: Vec<String> = (0..10_000).map(|i| format!("src/file_{i}.rs")).collect();
    let event = pr_event_with("title", "org/repo", paths);

    let out = r.redact_events(&[event], "manager").unwrap();
    match &out[0].payload {
        EventPayload::PullRequest(pr) => assert!(pr.touched_paths_hint.is_empty()),
        _ => panic!("expected PR"),
    }
}

// ===========================================================================
// Unicode characters in all fields
// ===========================================================================

#[test]
fn unicode_pr_title_redacted_in_public() {
    let r = DeterministicRedactor::new(b"unicode-key");
    let event = pr_event_with(
        "功能: 添加中文支持 🎉",
        "org/项目",
        vec!["src/中文.rs".into()],
    );

    let out = r.redact_events(&[event], "public").unwrap();
    match &out[0].payload {
        EventPayload::PullRequest(pr) => {
            assert_eq!(pr.title, "[redacted]");
            assert!(pr.touched_paths_hint.is_empty());
        }
        _ => panic!("expected PR"),
    }

    let json = serde_json::to_string(&out).unwrap();
    assert!(!json.contains("功能"));
    assert!(!json.contains("中文"));
    assert!(!json.contains("🎉"));
}

#[test]
fn unicode_review_title_redacted_in_public() {
    let r = DeterministicRedactor::new(b"unicode-key");
    let event = review_event_with("검토: 보안 패치 🔒", "org/프로젝트");

    let out = r.redact_events(&[event], "public").unwrap();
    match &out[0].payload {
        EventPayload::Review(rv) => assert_eq!(rv.pull_title, "[redacted]"),
        _ => panic!("expected Review"),
    }
    let json = serde_json::to_string(&out).unwrap();
    assert!(!json.contains("검토"));
    assert!(!json.contains("보안"));
}

#[test]
fn unicode_manual_event_redacted_in_public() {
    let r = DeterministicRedactor::new(b"unicode-key");
    let event = manual_event_with(
        "インシデント: データ漏洩",
        Some("顧客データが公開ログに露出"),
        Some("1万件のレコードに影響"),
    );

    let out = r.redact_events(&[event], "public").unwrap();
    let json = serde_json::to_string(&out).unwrap();
    assert!(!json.contains("インシデント"));
    assert!(!json.contains("顧客データ"));
    assert!(!json.contains("レコード"));
}

#[test]
fn unicode_workstream_title_aliased_in_public() {
    let r = DeterministicRedactor::new(b"unicode-key");
    let ws = make_workstreams_file(&[(
        "プロジェクト: 認証改善 🔐",
        Some("内部詳細情報"),
        vec!["security", "repo"],
    )]);

    let out = r.redact_workstreams(&ws, "public").unwrap();
    let json = serde_json::to_string(&out).unwrap();
    assert!(!json.contains("プロジェクト"));
    assert!(!json.contains("認証改善"));
    assert!(!json.contains("内部詳細"));
}

#[test]
fn unicode_preserved_in_internal() {
    let r = DeterministicRedactor::new(b"unicode-key");
    let event = pr_event_with("功能: 中文支持", "org/项目", vec!["src/中文.rs".into()]);

    let out = r.redact_events(&[event], "internal").unwrap();
    let json = serde_json::to_string(&out).unwrap();
    assert!(json.contains("功能: 中文支持"));
    assert!(json.contains("中文.rs"));
}

#[test]
fn unicode_title_preserved_in_manager() {
    let r = DeterministicRedactor::new(b"unicode-key");
    let event = pr_event_with("功能: 中文支持", "org/项目", vec!["src/中文.rs".into()]);

    let out = r.redact_events(&[event], "manager").unwrap();
    match &out[0].payload {
        EventPayload::PullRequest(pr) => {
            assert_eq!(pr.title, "功能: 中文支持");
            assert!(pr.touched_paths_hint.is_empty());
        }
        _ => panic!("expected PR"),
    }
}

// ===========================================================================
// Empty/null strings
// ===========================================================================

#[test]
fn empty_title_still_redacted() {
    let r = DeterministicRedactor::new(b"empty-key");
    let event = pr_event_with("", "org/repo", vec![]);

    let out = r.redact_events(&[event], "public").unwrap();
    match &out[0].payload {
        EventPayload::PullRequest(pr) => assert_eq!(pr.title, "[redacted]"),
        _ => panic!("expected PR"),
    }
}

#[test]
fn empty_repo_name_aliases_deterministically() {
    let r = DeterministicRedactor::new(b"empty-key");
    let event = pr_event_with("title", "", vec![]);

    let out = r.redact_events(&[event], "public").unwrap();
    assert!(!out[0].repo.full_name.is_empty());
}

#[test]
fn empty_events_vector() {
    let r = DeterministicRedactor::new(b"empty-key");
    let events: Vec<EventEnvelope> = vec![];

    for profile in ["internal", "manager", "public"] {
        let out = r.redact_events(&events, profile).unwrap();
        assert!(out.is_empty());
    }
}

#[test]
fn empty_workstreams_file() {
    let r = DeterministicRedactor::new(b"empty-key");
    let ws = WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![],
    };

    for profile in ["internal", "manager", "public"] {
        let out = r.redact_workstreams(&ws, profile).unwrap();
        assert!(out.workstreams.is_empty());
    }
}

#[test]
fn none_description_and_impact_stay_none_across_profiles() {
    let r = DeterministicRedactor::new(b"none-key");
    let event = manual_event_with("title", None, None);

    for profile in ["internal", "manager", "public"] {
        let out = r
            .redact_events(std::slice::from_ref(&event), profile)
            .unwrap();
        match &out[0].payload {
            EventPayload::Manual(m) => {
                assert!(m.description.is_none(), "desc should be None for {profile}");
                assert!(m.impact.is_none(), "impact should be None for {profile}");
            }
            _ => panic!("expected Manual"),
        }
    }
}

// ===========================================================================
// All three profiles produce different outputs
// ===========================================================================

#[test]
fn three_profiles_produce_distinct_event_outputs() {
    let r = DeterministicRedactor::new(b"distinct-key");
    let event = pr_event_with(
        "Secret Feature",
        "acme/private",
        vec!["src/secret.rs".into()],
    );

    let internal = r
        .redact_events(std::slice::from_ref(&event), "internal")
        .unwrap();
    let manager = r
        .redact_events(std::slice::from_ref(&event), "manager")
        .unwrap();
    let public = r.redact_events(&[event], "public").unwrap();

    let json_i = serde_json::to_string(&internal).unwrap();
    let json_m = serde_json::to_string(&manager).unwrap();
    let json_p = serde_json::to_string(&public).unwrap();

    assert_ne!(json_i, json_m, "internal vs manager should differ");
    assert_ne!(json_i, json_p, "internal vs public should differ");
    assert_ne!(json_m, json_p, "manager vs public should differ");
}

#[test]
fn three_profiles_produce_distinct_workstream_outputs() {
    let r = DeterministicRedactor::new(b"distinct-key");
    let ws = make_workstreams_file(&[(
        "Secret Migration",
        Some("Internal architecture details"),
        vec!["infra", "repo"],
    )]);

    let internal = r.redact_workstreams(&ws, "internal").unwrap();
    let manager = r.redact_workstreams(&ws, "manager").unwrap();
    let public = r.redact_workstreams(&ws, "public").unwrap();

    let json_i = serde_json::to_string(&internal).unwrap();
    let json_m = serde_json::to_string(&manager).unwrap();
    let json_p = serde_json::to_string(&public).unwrap();

    assert_ne!(json_i, json_m, "internal vs manager should differ");
    assert_ne!(json_i, json_p, "internal vs public should differ");
    assert_ne!(json_m, json_p, "manager vs public should differ");
}

#[test]
fn profile_strictness_ordering_events() {
    let r = DeterministicRedactor::new(b"order-key");
    let event = pr_event_with("Secret", "acme/private", vec!["src/path.rs".into()]);

    let internal = r
        .redact_events(std::slice::from_ref(&event), "internal")
        .unwrap();
    let manager = r
        .redact_events(std::slice::from_ref(&event), "manager")
        .unwrap();
    let public = r.redact_events(&[event], "public").unwrap();

    // Internal preserves everything
    assert!(!internal[0].links.is_empty());
    assert!(internal[0].source.url.is_some());
    match &internal[0].payload {
        EventPayload::PullRequest(pr) => assert!(!pr.touched_paths_hint.is_empty()),
        _ => panic!("expected PR"),
    }

    // Manager strips links and paths but keeps title, repo, source URL
    assert!(manager[0].links.is_empty());
    assert!(manager[0].source.url.is_some());
    assert_eq!(manager[0].repo.full_name, "acme/private");
    match &manager[0].payload {
        EventPayload::PullRequest(pr) => {
            assert_eq!(pr.title, "Secret");
            assert!(pr.touched_paths_hint.is_empty());
        }
        _ => panic!("expected PR"),
    }

    // Public strips everything
    assert!(public[0].links.is_empty());
    assert!(public[0].source.url.is_none());
    assert_ne!(public[0].repo.full_name, "acme/private");
    match &public[0].payload {
        EventPayload::PullRequest(pr) => assert_eq!(pr.title, "[redacted]"),
        _ => panic!("expected PR"),
    }
}

// ===========================================================================
// Redaction consistency across separate calls
// ===========================================================================

#[test]
fn same_key_same_event_produces_identical_results() {
    let event = pr_event_with("Consistent PR", "org/repo", vec!["path.rs".into()]);

    let r1 = DeterministicRedactor::new(b"consistent-key");
    let r2 = DeterministicRedactor::new(b"consistent-key");

    let out1 = r1
        .redact_events(std::slice::from_ref(&event), "public")
        .unwrap();
    let out2 = r2.redact_events(&[event], "public").unwrap();

    assert_eq!(out1, out2);
}

#[test]
fn same_key_separate_calls_produce_identical_workstreams() {
    let ws = make_workstreams_file(&[("WS Title", Some("Summary"), vec!["repo", "tag"])]);

    let r1 = DeterministicRedactor::new(b"ws-key");
    let r2 = DeterministicRedactor::new(b"ws-key");

    let out1 = r1.redact_workstreams(&ws, "public").unwrap();
    let out2 = r2.redact_workstreams(&ws, "public").unwrap();

    assert_eq!(out1, out2);
}

#[test]
fn same_redactor_multiple_calls_consistent() {
    let r = DeterministicRedactor::new(b"multi-call-key");
    let event = pr_event_with("PR Title", "org/repo", vec![]);

    let out1 = r
        .redact_events(std::slice::from_ref(&event), "public")
        .unwrap();
    let out2 = r
        .redact_events(std::slice::from_ref(&event), "public")
        .unwrap();
    let out3 = r.redact_events(&[event], "public").unwrap();

    assert_eq!(out1, out2);
    assert_eq!(out2, out3);
}

#[test]
fn different_keys_produce_different_aliases() {
    let event = pr_event_with("PR Title", "org/repo", vec![]);

    let r1 = DeterministicRedactor::new(b"key-alpha");
    let r2 = DeterministicRedactor::new(b"key-bravo");

    let out1 = r1
        .redact_events(std::slice::from_ref(&event), "public")
        .unwrap();
    let out2 = r2.redact_events(&[event], "public").unwrap();

    assert_ne!(
        out1[0].repo.full_name, out2[0].repo.full_name,
        "different keys should produce different repo aliases"
    );
}

// ===========================================================================
// Mixed event types in a batch
// ===========================================================================

#[test]
fn mixed_event_types_all_redacted_correctly_in_public() {
    let r = DeterministicRedactor::new(b"mixed-key");
    let events = vec![
        pr_event_with("PR Secret", "org/private", vec!["src/secret.rs".into()]),
        review_event_with("Review Secret", "org/private"),
        manual_event_with(
            "Manual Secret",
            Some("Sensitive desc"),
            Some("Sensitive impact"),
        ),
    ];

    let out = r.redact_events(&events, "public").unwrap();
    assert_eq!(out.len(), 3);

    match &out[0].payload {
        EventPayload::PullRequest(pr) => {
            assert_eq!(pr.title, "[redacted]");
            assert!(pr.touched_paths_hint.is_empty());
        }
        _ => panic!("expected PR"),
    }
    match &out[1].payload {
        EventPayload::Review(rv) => assert_eq!(rv.pull_title, "[redacted]"),
        _ => panic!("expected Review"),
    }
    match &out[2].payload {
        EventPayload::Manual(m) => {
            assert_eq!(m.title, "[redacted]");
            assert!(m.description.is_none());
            assert!(m.impact.is_none());
        }
        _ => panic!("expected Manual"),
    }

    for event in &out {
        assert!(event.links.is_empty());
        assert!(event.source.url.is_none());
    }
}

// ===========================================================================
// Preserved fields across all profiles
// ===========================================================================

#[test]
fn event_id_and_kind_preserved_across_all_profiles() {
    let r = DeterministicRedactor::new(b"preserve-key");
    let event = pr_event_with("title", "org/repo", vec![]);

    for profile in ["internal", "manager", "public"] {
        let out = r
            .redact_events(std::slice::from_ref(&event), profile)
            .unwrap();
        assert_eq!(out[0].id, event.id, "id changed for {profile}");
        assert_eq!(out[0].kind, event.kind, "kind changed for {profile}");
        assert_eq!(
            out[0].occurred_at, event.occurred_at,
            "occurred_at changed for {profile}"
        );
        assert_eq!(out[0].actor, event.actor, "actor changed for {profile}");
        assert_eq!(out[0].tags, event.tags, "tags changed for {profile}");
    }
}

#[test]
fn workstream_id_and_stats_preserved_across_all_profiles() {
    let r = DeterministicRedactor::new(b"preserve-key");
    let ws = make_workstreams_file(&[("Title", Some("Summary"), vec!["repo", "tag"])]);

    for profile in ["internal", "manager", "public"] {
        let out = r.redact_workstreams(&ws, profile).unwrap();
        assert_eq!(
            out.workstreams[0].id, ws.workstreams[0].id,
            "id changed for {profile}"
        );
        assert_eq!(
            out.workstreams[0].stats, ws.workstreams[0].stats,
            "stats changed for {profile}"
        );
    }
}

// ===========================================================================
// Cache round-trip with unicode data
// ===========================================================================

#[test]
fn cache_round_trip_with_unicode_aliases() {
    let dir = tempfile::tempdir().unwrap();
    let cache_path = dir.path().join("redaction.aliases.json");

    let r1 = DeterministicRedactor::new(b"unicode-cache-key");
    let event = pr_event_with("功能: 中文", "org/项目", vec![]);

    let out1 = r1
        .redact_events(std::slice::from_ref(&event), "public")
        .unwrap();
    r1.save_cache(&cache_path).unwrap();

    let r2 = DeterministicRedactor::new(b"unicode-cache-key");
    r2.load_cache(&cache_path).unwrap();
    let out2 = r2.redact_events(&[event], "public").unwrap();

    assert_eq!(out1, out2);
}

// ===========================================================================
// Unknown profile strings treated as public
// ===========================================================================

#[test]
fn unknown_profile_strings_treated_as_public() {
    let r = DeterministicRedactor::new(b"unknown-key");
    let event = pr_event_with("Secret", "org/repo", vec![]);

    let public_out = r
        .redact_events(std::slice::from_ref(&event), "public")
        .unwrap();

    for unknown in ["unknown", "INTERNAL", "Manager", "admin", "", " ", "🔒"] {
        let out = r
            .redact_events(std::slice::from_ref(&event), unknown)
            .unwrap();
        assert_eq!(
            out, public_out,
            "profile {unknown:?} should behave like public"
        );
    }
}
