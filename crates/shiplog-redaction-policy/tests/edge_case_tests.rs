//! Edge case tests for shiplog-redaction-policy.

use chrono::{NaiveDate, Utc};
use shiplog_ids::{EventId, WorkstreamId};
use shiplog_redaction_policy::{
    RedactionProfile, redact_event_with_aliases, redact_events_with_aliases,
    redact_workstream_with_aliases, redact_workstreams_with_aliases,
};
use shiplog_schema::event::*;
use shiplog_schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};

fn noop_alias(_kind: &str, value: &str) -> String {
    format!("ALIAS({value})")
}

fn make_pr_event(title: &str, repo: &str, paths: Vec<String>) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["edge", "1"]),
        kind: EventKind::PullRequest,
        occurred_at: Utc::now(),
        actor: Actor {
            login: "dev".into(),
            id: None,
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
        tags: vec![],
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

fn make_review_event(pull_title: &str, repo: &str) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["edge", "review", "1"]),
        kind: EventKind::Review,
        occurred_at: Utc::now(),
        actor: Actor {
            login: "reviewer".into(),
            id: None,
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

fn make_manual_event(
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
        tags: vec![],
        links: vec![],
        source: SourceRef {
            system: SourceSystem::Manual,
            url: None,
            opaque_id: None,
        },
    }
}

fn make_workstream(title: &str, summary: Option<&str>, tags: Vec<&str>) -> Workstream {
    Workstream {
        id: WorkstreamId::from_parts(["ws", "edge"]),
        title: title.into(),
        summary: summary.map(String::from),
        tags: tags.into_iter().map(String::from).collect(),
        stats: WorkstreamStats::zero(),
        events: vec![],
        receipts: vec![],
    }
}

// ============================================================================
// Empty string fields
// ============================================================================

#[test]
fn empty_title_pr_event_public_still_redacts() {
    let event = make_pr_event("", "org/repo", vec![]);
    let out = redact_event_with_aliases(event, RedactionProfile::Public, &noop_alias);
    match &out.payload {
        EventPayload::PullRequest(pr) => {
            assert_eq!(pr.title, "[redacted]");
        }
        _ => panic!("expected PR payload"),
    }
}

#[test]
fn empty_repo_name_public_still_aliases() {
    let event = make_pr_event("title", "", vec![]);
    let out = redact_event_with_aliases(event, RedactionProfile::Public, &noop_alias);
    assert_eq!(out.repo.full_name, "ALIAS()");
}

#[test]
fn empty_manual_fields_manager_clears_nones() {
    let event = make_manual_event("title", None, None);
    let out = redact_event_with_aliases(event, RedactionProfile::Manager, &noop_alias);
    match &out.payload {
        EventPayload::Manual(m) => {
            assert!(m.description.is_none());
            assert!(m.impact.is_none());
        }
        _ => panic!("expected Manual payload"),
    }
}

#[test]
fn empty_manual_fields_public_redacts_title_clears_optionals() {
    let event = make_manual_event("", Some("desc"), Some("impact"));
    let out = redact_event_with_aliases(event, RedactionProfile::Public, &noop_alias);
    match &out.payload {
        EventPayload::Manual(m) => {
            assert_eq!(m.title, "[redacted]");
            assert!(m.description.is_none());
            assert!(m.impact.is_none());
        }
        _ => panic!("expected Manual payload"),
    }
}

// ============================================================================
// Unicode in all fields
// ============================================================================

#[test]
fn unicode_title_public_redacts() {
    let event = make_pr_event(
        "功能: 添加中文支持 🎉",
        "org/项目",
        vec!["src/中文.rs".into()],
    );
    let out = redact_event_with_aliases(event, RedactionProfile::Public, &noop_alias);
    match &out.payload {
        EventPayload::PullRequest(pr) => {
            assert_eq!(pr.title, "[redacted]");
            assert!(pr.touched_paths_hint.is_empty());
        }
        _ => panic!("expected PR payload"),
    }
    assert_eq!(
        out.repo.full_name,
        "ALIAS(org/項目)"
            .replace("項目", "项目")
            .replace("ALIAS(org/项目)", &noop_alias("repo", "org/项目"))
    );
    assert!(out.links.is_empty());
}

#[test]
fn unicode_workstream_title_public_aliases() {
    let ws = make_workstream(
        "プロジェクト: 認証改善",
        Some("内部詳細"),
        vec!["repo", "security"],
    );
    let out = redact_workstream_with_aliases(ws, RedactionProfile::Public, &noop_alias);
    assert_eq!(out.title, noop_alias("ws", "プロジェクト: 認証改善"));
    assert!(out.summary.is_none());
    assert!(!out.tags.contains(&"repo".to_string()));
}

#[test]
fn unicode_review_title_public_redacts() {
    let event = make_review_event("검토: 보안 패치 🔒", "org/프로젝트");
    let out = redact_event_with_aliases(event, RedactionProfile::Public, &noop_alias);
    match &out.payload {
        EventPayload::Review(r) => {
            assert_eq!(r.pull_title, "[redacted]");
        }
        _ => panic!("expected Review payload"),
    }
}

#[test]
fn unicode_fields_manager_preserves_title() {
    let event = make_pr_event("功能: 数据迁移 🚀", "org/项目", vec!["src/数据.rs".into()]);
    let out = redact_event_with_aliases(event, RedactionProfile::Manager, &noop_alias);
    match &out.payload {
        EventPayload::PullRequest(pr) => {
            assert_eq!(pr.title, "功能: 数据迁移 🚀");
            assert!(pr.touched_paths_hint.is_empty());
        }
        _ => panic!("expected PR payload"),
    }
}

#[test]
fn unicode_fields_internal_preserves_all() {
    let event = make_pr_event("功能: 数据迁移 🚀", "org/项目", vec!["src/数据.rs".into()]);
    let out = redact_event_with_aliases(event.clone(), RedactionProfile::Internal, &noop_alias);
    assert_eq!(out, event);
}

// ============================================================================
// Very long strings
// ============================================================================

#[test]
fn very_long_title_redacts_correctly() {
    let long_title = "A".repeat(100_000);
    let event = make_pr_event(&long_title, "org/repo", vec![]);
    let out = redact_event_with_aliases(event, RedactionProfile::Public, &noop_alias);
    match &out.payload {
        EventPayload::PullRequest(pr) => {
            assert_eq!(pr.title, "[redacted]");
        }
        _ => panic!("expected PR payload"),
    }
}

#[test]
fn very_long_repo_name_aliases_correctly() {
    let long_repo = format!("org/{}", "x".repeat(100_000));
    let event = make_pr_event("title", &long_repo, vec![]);
    let out = redact_event_with_aliases(event, RedactionProfile::Public, &noop_alias);
    assert_eq!(out.repo.full_name, noop_alias("repo", &long_repo));
}

#[test]
fn many_touched_paths_cleared_in_manager() {
    let paths: Vec<String> = (0..10_000).map(|i| format!("src/file_{i}.rs")).collect();
    let event = make_pr_event("title", "org/repo", paths);
    let out = redact_event_with_aliases(event, RedactionProfile::Manager, &noop_alias);
    match &out.payload {
        EventPayload::PullRequest(pr) => {
            assert!(pr.touched_paths_hint.is_empty());
        }
        _ => panic!("expected PR payload"),
    }
}

#[test]
fn many_links_cleared_in_manager_and_public() {
    let mut event = make_pr_event("title", "org/repo", vec![]);
    event.links = (0..1_000)
        .map(|i| Link {
            label: format!("link-{i}"),
            url: format!("https://example.com/{i}"),
        })
        .collect();

    let mgr = redact_event_with_aliases(event.clone(), RedactionProfile::Manager, &noop_alias);
    assert!(mgr.links.is_empty());

    let pub_out = redact_event_with_aliases(event, RedactionProfile::Public, &noop_alias);
    assert!(pub_out.links.is_empty());
}

// ============================================================================
// All three profiles produce different outputs
// ============================================================================

#[test]
fn three_profiles_produce_different_event_outputs() {
    let event = make_pr_event(
        "Secret Feature",
        "acme/private",
        vec!["src/secret.rs".into()],
    );

    let internal =
        redact_event_with_aliases(event.clone(), RedactionProfile::Internal, &noop_alias);
    let manager = redact_event_with_aliases(event.clone(), RedactionProfile::Manager, &noop_alias);
    let public = redact_event_with_aliases(event, RedactionProfile::Public, &noop_alias);

    let json_i = serde_json::to_string(&internal).unwrap();
    let json_m = serde_json::to_string(&manager).unwrap();
    let json_p = serde_json::to_string(&public).unwrap();

    assert_ne!(json_i, json_m, "internal and manager should differ");
    assert_ne!(json_i, json_p, "internal and public should differ");
    assert_ne!(json_m, json_p, "manager and public should differ");
}

#[test]
fn three_profiles_produce_different_workstream_outputs() {
    let ws = make_workstream(
        "Secret Migration",
        Some("Internal details"),
        vec!["infra", "repo"],
    );

    let internal =
        redact_workstream_with_aliases(ws.clone(), RedactionProfile::Internal, &noop_alias);
    let manager =
        redact_workstream_with_aliases(ws.clone(), RedactionProfile::Manager, &noop_alias);
    let public = redact_workstream_with_aliases(ws, RedactionProfile::Public, &noop_alias);

    let json_i = serde_json::to_string(&internal).unwrap();
    let json_m = serde_json::to_string(&manager).unwrap();
    let json_p = serde_json::to_string(&public).unwrap();

    assert_ne!(json_i, json_m, "internal and manager should differ");
    assert_ne!(json_i, json_p, "internal and public should differ");
    assert_ne!(json_m, json_p, "manager and public should differ");
}

// ============================================================================
// Policy application to all event types
// ============================================================================

#[test]
fn review_event_manager_preserves_all_fields() {
    let event = make_review_event("Review of Secret PR", "org/repo");
    let out = redact_event_with_aliases(event.clone(), RedactionProfile::Manager, &noop_alias);
    match (&event.payload, &out.payload) {
        (EventPayload::Review(orig), EventPayload::Review(red)) => {
            assert_eq!(orig.pull_title, red.pull_title);
            assert_eq!(orig.state, red.state);
            assert_eq!(orig.pull_number, red.pull_number);
        }
        _ => panic!("expected Review payload"),
    }
    assert!(out.links.is_empty());
}

#[test]
fn review_event_public_redacts_pull_title_only() {
    let event = make_review_event("Review of Secret PR", "org/repo");
    let out = redact_event_with_aliases(event.clone(), RedactionProfile::Public, &noop_alias);
    match &out.payload {
        EventPayload::Review(r) => {
            assert_eq!(r.pull_title, "[redacted]");
            assert_eq!(r.state, "approved");
            assert_eq!(r.pull_number, 1);
        }
        _ => panic!("expected Review payload"),
    }
}

#[test]
fn all_manual_event_types_redact_consistently() {
    let types = [
        ManualEventType::Note,
        ManualEventType::Incident,
        ManualEventType::Design,
        ManualEventType::Mentoring,
        ManualEventType::Launch,
        ManualEventType::Migration,
        ManualEventType::Review,
        ManualEventType::Other,
    ];
    for event_type in types {
        let mut event = make_manual_event("Secret title", Some("desc"), Some("impact"));
        if let EventPayload::Manual(ref mut m) = event.payload {
            m.event_type = event_type.clone();
        }
        let out = redact_event_with_aliases(event, RedactionProfile::Public, &noop_alias);
        match &out.payload {
            EventPayload::Manual(m) => {
                assert_eq!(m.title, "[redacted]");
                assert!(m.description.is_none());
                assert!(m.impact.is_none());
                assert_eq!(m.event_type, event_type);
            }
            _ => panic!("expected Manual payload"),
        }
    }
}

// ============================================================================
// Empty event vectors
// ============================================================================

#[test]
fn empty_events_vector_returns_empty_for_all_profiles() {
    let events: Vec<EventEnvelope> = vec![];
    for profile in [
        RedactionProfile::Internal,
        RedactionProfile::Manager,
        RedactionProfile::Public,
    ] {
        let out = redact_events_with_aliases(&events, profile, &noop_alias);
        assert!(out.is_empty());
    }
}

#[test]
fn empty_workstreams_returns_empty_for_all_profiles() {
    let ws_file = WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![],
    };
    for profile in [
        RedactionProfile::Internal,
        RedactionProfile::Manager,
        RedactionProfile::Public,
    ] {
        let out = redact_workstreams_with_aliases(&ws_file, profile, &noop_alias);
        assert!(out.workstreams.is_empty());
    }
}

// ============================================================================
// Workstream edge cases: tags with "repo" substring
// ============================================================================

#[test]
fn only_exact_repo_tag_is_filtered_in_public() {
    let ws = make_workstream(
        "Workstream",
        None,
        vec!["repo", "repository", "mono-repo", "repo-tools", "REPO"],
    );
    let out = redact_workstream_with_aliases(ws, RedactionProfile::Public, &noop_alias);
    assert!(!out.tags.contains(&"repo".to_string()));
    assert!(out.tags.contains(&"repository".to_string()));
    assert!(out.tags.contains(&"mono-repo".to_string()));
    assert!(out.tags.contains(&"repo-tools".to_string()));
    assert!(out.tags.contains(&"REPO".to_string()));
}

#[test]
fn workstream_with_no_summary_public_keeps_none() {
    let ws = make_workstream("Title", None, vec!["tag1"]);
    let out = redact_workstream_with_aliases(ws, RedactionProfile::Public, &noop_alias);
    assert!(out.summary.is_none());
}

#[test]
fn workstream_with_empty_tags_public_no_panic() {
    let ws = make_workstream("Title", Some("Summary"), vec![]);
    let out = redact_workstream_with_aliases(ws, RedactionProfile::Public, &noop_alias);
    assert!(out.tags.is_empty());
    assert!(out.summary.is_none());
}

// ============================================================================
// Redaction consistency: same alias resolver produces same results
// ============================================================================

#[test]
fn redacting_same_event_twice_produces_identical_output() {
    let event = make_pr_event("Consistent", "org/repo", vec!["path.rs".into()]);
    let out1 = redact_event_with_aliases(event.clone(), RedactionProfile::Public, &noop_alias);
    let out2 = redact_event_with_aliases(event, RedactionProfile::Public, &noop_alias);
    assert_eq!(out1, out2);
}

#[test]
fn redacting_same_workstream_twice_produces_identical_output() {
    let ws = make_workstream("Consistent WS", Some("summary"), vec!["repo", "tag"]);
    let out1 = redact_workstream_with_aliases(ws.clone(), RedactionProfile::Public, &noop_alias);
    let out2 = redact_workstream_with_aliases(ws, RedactionProfile::Public, &noop_alias);
    assert_eq!(out1, out2);
}

// ============================================================================
// Event fields preserved across all profiles
// ============================================================================

#[test]
fn event_id_preserved_across_all_profiles() {
    let event = make_pr_event("title", "org/repo", vec![]);
    for profile in [
        RedactionProfile::Internal,
        RedactionProfile::Manager,
        RedactionProfile::Public,
    ] {
        let out = redact_event_with_aliases(event.clone(), profile, &noop_alias);
        assert_eq!(out.id, event.id, "id changed for profile {profile:?}");
        assert_eq!(out.kind, event.kind, "kind changed for profile {profile:?}");
        assert_eq!(
            out.occurred_at, event.occurred_at,
            "occurred_at changed for profile {profile:?}"
        );
        assert_eq!(
            out.actor, event.actor,
            "actor changed for profile {profile:?}"
        );
    }
}

// ============================================================================
// Workstream fields preserved across profiles
// ============================================================================

#[test]
fn workstream_id_and_stats_preserved_across_all_profiles() {
    let ws = Workstream {
        id: WorkstreamId::from_parts(["ws", "preserve"]),
        title: "Title".into(),
        summary: Some("Summary".into()),
        tags: vec!["repo".into(), "tag".into()],
        stats: WorkstreamStats {
            pull_requests: 5,
            reviews: 3,
            manual_events: 1,
        },
        events: vec![EventId::from_parts(["e", "1"])],
        receipts: vec![EventId::from_parts(["e", "1"])],
    };

    for profile in [
        RedactionProfile::Internal,
        RedactionProfile::Manager,
        RedactionProfile::Public,
    ] {
        let out = redact_workstream_with_aliases(ws.clone(), profile, &noop_alias);
        assert_eq!(out.id, ws.id, "id changed for profile {profile:?}");
        assert_eq!(out.stats, ws.stats, "stats changed for profile {profile:?}");
        assert_eq!(
            out.events, ws.events,
            "events changed for profile {profile:?}"
        );
        assert_eq!(
            out.receipts, ws.receipts,
            "receipts changed for profile {profile:?}"
        );
    }
}

// ============================================================================
// Mixed event types in a single batch
// ============================================================================

#[test]
fn mixed_event_types_batch_public_redacts_all_correctly() {
    let events = vec![
        make_pr_event("PR Title", "org/repo", vec!["path.rs".into()]),
        make_review_event("Review Title", "org/repo"),
        make_manual_event("Manual Title", Some("desc"), Some("impact")),
    ];

    let out = redact_events_with_aliases(&events, RedactionProfile::Public, &noop_alias);

    assert_eq!(out.len(), 3);

    match &out[0].payload {
        EventPayload::PullRequest(pr) => assert_eq!(pr.title, "[redacted]"),
        _ => panic!("expected PR"),
    }
    match &out[1].payload {
        EventPayload::Review(r) => assert_eq!(r.pull_title, "[redacted]"),
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
}

#[test]
fn mixed_event_types_batch_manager_preserves_titles() {
    let events = vec![
        make_pr_event("PR Title", "org/repo", vec!["path.rs".into()]),
        make_review_event("Review Title", "org/repo"),
        make_manual_event("Manual Title", Some("desc"), Some("impact")),
    ];

    let out = redact_events_with_aliases(&events, RedactionProfile::Manager, &noop_alias);

    match &out[0].payload {
        EventPayload::PullRequest(pr) => {
            assert_eq!(pr.title, "PR Title");
            assert!(pr.touched_paths_hint.is_empty());
        }
        _ => panic!("expected PR"),
    }
    match &out[1].payload {
        EventPayload::Review(r) => assert_eq!(r.pull_title, "Review Title"),
        _ => panic!("expected Review"),
    }
    match &out[2].payload {
        EventPayload::Manual(m) => {
            assert_eq!(m.title, "Manual Title");
            assert!(m.description.is_none());
            assert!(m.impact.is_none());
        }
        _ => panic!("expected Manual"),
    }
}

// ============================================================================
// Public profile: source URL and html_url handling
// ============================================================================

#[test]
fn public_clears_source_url_and_html_url() {
    let event = make_pr_event("title", "org/repo", vec![]);
    let out = redact_event_with_aliases(event, RedactionProfile::Public, &noop_alias);
    assert!(out.source.url.is_none());
    assert!(out.repo.html_url.is_none());
}

#[test]
fn manager_preserves_source_url() {
    let event = make_pr_event("title", "org/repo", vec![]);
    let out = redact_event_with_aliases(event.clone(), RedactionProfile::Manager, &noop_alias);
    assert_eq!(out.source.url, event.source.url);
}

// ============================================================================
// Workstreams file metadata preserved
// ============================================================================

#[test]
fn workstreams_file_version_and_generated_at_preserved() {
    let ws_file = WorkstreamsFile {
        version: 42,
        generated_at: Utc::now(),
        workstreams: vec![make_workstream("Title", Some("Sum"), vec!["repo"])],
    };

    for profile in [
        RedactionProfile::Internal,
        RedactionProfile::Manager,
        RedactionProfile::Public,
    ] {
        let out = redact_workstreams_with_aliases(&ws_file, profile, &noop_alias);
        assert_eq!(out.version, 42, "version changed for profile {profile:?}");
        assert_eq!(
            out.generated_at, ws_file.generated_at,
            "generated_at changed for profile {profile:?}"
        );
    }
}

// ============================================================================
// Multiple workstreams with duplicate "repo" tags
// ============================================================================

#[test]
fn multiple_workstreams_all_have_repo_tag_filtered_in_public() {
    let ws_file = WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![
            make_workstream("WS1", None, vec!["repo", "tag1"]),
            make_workstream("WS2", Some("sum"), vec!["repo", "tag2"]),
            make_workstream("WS3", None, vec!["tag3"]),
        ],
    };

    let out = redact_workstreams_with_aliases(&ws_file, RedactionProfile::Public, &noop_alias);
    for ws in &out.workstreams {
        assert!(!ws.tags.contains(&"repo".to_string()));
        assert!(ws.summary.is_none());
    }
}
