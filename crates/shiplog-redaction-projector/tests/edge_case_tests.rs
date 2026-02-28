//! Edge case tests for shiplog-redaction-projector.

use chrono::{NaiveDate, Utc};
use shiplog_ids::{EventId, WorkstreamId};
use shiplog_redaction_projector::{
    RedactionProfile, parse_profile, project_events_with_aliases, project_workstreams_with_aliases,
};
use shiplog_schema::event::*;
use shiplog_schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};

fn stable_alias(kind: &str, value: &str) -> String {
    format!("{kind}::{value}")
}

fn make_pr_event(title: &str, repo: &str) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["proj", "edge", "1"]),
        kind: EventKind::PullRequest,
        occurred_at: Utc::now(),
        actor: Actor {
            login: "dev".into(),
            id: Some(1),
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
            touched_paths_hint: vec!["src/main.rs".into()],
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

fn make_review_event(pull_title: &str, repo: &str) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["proj", "edge", "review"]),
        kind: EventKind::Review,
        occurred_at: Utc::now(),
        actor: Actor {
            login: "reviewer".into(),
            id: Some(2),
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

fn make_manual_event(title: &str) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["proj", "edge", "manual"]),
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
            event_type: ManualEventType::Design,
            title: title.into(),
            description: Some("Sensitive design notes".into()),
            started_at: Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
            ended_at: None,
            impact: Some("Major architecture change".into()),
        }),
        tags: vec!["design".into()],
        links: vec![],
        source: SourceRef {
            system: SourceSystem::Manual,
            url: None,
            opaque_id: None,
        },
    }
}

fn make_workstreams(titles: &[&str]) -> WorkstreamsFile {
    WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: titles
            .iter()
            .enumerate()
            .map(|(i, title)| Workstream {
                id: WorkstreamId::from_parts(["ws", &i.to_string()]),
                title: (*title).into(),
                summary: Some(format!("Summary for {title}")),
                tags: vec!["repo".into(), "infra".into()],
                stats: WorkstreamStats {
                    pull_requests: 2,
                    reviews: 1,
                    manual_events: 0,
                },
                events: vec![],
                receipts: vec![],
            })
            .collect(),
    }
}

// ============================================================================
// Profile parsing edge cases
// ============================================================================

#[test]
fn parse_profile_unicode_defaults_to_public() {
    assert_eq!(parse_profile("интернал"), RedactionProfile::Public);
    assert_eq!(parse_profile("マネージャー"), RedactionProfile::Public);
    assert_eq!(parse_profile("公開"), RedactionProfile::Public);
    assert_eq!(parse_profile("🔒"), RedactionProfile::Public);
}

#[test]
fn parse_profile_whitespace_defaults_to_public() {
    assert_eq!(parse_profile(""), RedactionProfile::Public);
    assert_eq!(parse_profile(" "), RedactionProfile::Public);
    assert_eq!(parse_profile("\t"), RedactionProfile::Public);
    assert_eq!(parse_profile("\n"), RedactionProfile::Public);
}

#[test]
fn parse_profile_case_sensitive() {
    assert_eq!(parse_profile("Internal"), RedactionProfile::Public);
    assert_eq!(parse_profile("INTERNAL"), RedactionProfile::Public);
    assert_eq!(parse_profile("Manager"), RedactionProfile::Public);
    assert_eq!(parse_profile("MANAGER"), RedactionProfile::Public);
    assert_eq!(parse_profile("Public"), RedactionProfile::Public);
    assert_eq!(parse_profile("PUBLIC"), RedactionProfile::Public);
}

#[test]
fn parse_profile_with_extra_chars_defaults_to_public() {
    assert_eq!(parse_profile("internal "), RedactionProfile::Public);
    assert_eq!(parse_profile(" internal"), RedactionProfile::Public);
    assert_eq!(parse_profile("internal\0"), RedactionProfile::Public);
}

// ============================================================================
// Complex events with mixed types
// ============================================================================

#[test]
fn project_mixed_event_types_public() {
    let events = vec![
        make_pr_event("PR Secret", "org/private"),
        make_review_event("Review Secret", "org/private"),
        make_manual_event("Manual Secret"),
    ];

    let out = project_events_with_aliases(&events, "public", &stable_alias);

    assert_eq!(out.len(), 3);

    // All titles should be [redacted]
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

    // All links should be cleared
    for event in &out {
        assert!(event.links.is_empty());
        assert!(event.source.url.is_none());
    }
}

#[test]
fn project_mixed_event_types_manager() {
    let events = vec![
        make_pr_event("PR Title", "org/private"),
        make_review_event("Review Title", "org/private"),
        make_manual_event("Manual Title"),
    ];

    let out = project_events_with_aliases(&events, "manager", &stable_alias);

    // PR title preserved, paths cleared
    match &out[0].payload {
        EventPayload::PullRequest(pr) => {
            assert_eq!(pr.title, "PR Title");
            assert!(pr.touched_paths_hint.is_empty());
        }
        _ => panic!("expected PR"),
    }

    // Review event unchanged in manager profile
    match &out[1].payload {
        EventPayload::Review(r) => assert_eq!(r.pull_title, "Review Title"),
        _ => panic!("expected Review"),
    }

    // Manual title preserved, desc/impact removed
    match &out[2].payload {
        EventPayload::Manual(m) => {
            assert_eq!(m.title, "Manual Title");
            assert!(m.description.is_none());
            assert!(m.impact.is_none());
        }
        _ => panic!("expected Manual"),
    }

    // Links cleared for all in manager
    for event in &out {
        assert!(event.links.is_empty());
    }
}

#[test]
fn project_mixed_event_types_internal() {
    let events = vec![
        make_pr_event("PR Title", "org/private"),
        make_review_event("Review Title", "org/private"),
        make_manual_event("Manual Title"),
    ];

    let out = project_events_with_aliases(&events, "internal", &stable_alias);
    assert_eq!(out, events);
}

// ============================================================================
// Field preservation vs stripping per profile
// ============================================================================

#[test]
fn public_strips_repo_html_url_and_source_url() {
    let event = make_pr_event("title", "org/repo");
    let out = project_events_with_aliases(&[event], "public", &stable_alias);
    assert!(out[0].repo.html_url.is_none());
    assert!(out[0].source.url.is_none());
}

#[test]
fn manager_preserves_repo_and_source_url() {
    let event = make_pr_event("title", "org/repo");
    let out = project_events_with_aliases(std::slice::from_ref(&event), "manager", &stable_alias);
    assert_eq!(out[0].repo.full_name, event.repo.full_name);
    assert_eq!(out[0].source.url, event.source.url);
}

#[test]
fn public_aliases_repo_name() {
    let event = make_pr_event("title", "org/repo");
    let out = project_events_with_aliases(&[event], "public", &stable_alias);
    assert_eq!(out[0].repo.full_name, stable_alias("repo", "org/repo"));
}

// ============================================================================
// Profile-specific workstream projections
// ============================================================================

#[test]
fn project_workstreams_public_aliases_all_titles() {
    let ws = make_workstreams(&["WS Alpha", "WS Beta", "WS Gamma"]);
    let out = project_workstreams_with_aliases(&ws, "public", &stable_alias);

    assert_eq!(out.workstreams.len(), 3);
    for (orig, proj) in ws.workstreams.iter().zip(out.workstreams.iter()) {
        assert_eq!(proj.title, stable_alias("ws", &orig.title));
        assert!(proj.summary.is_none());
        assert!(!proj.tags.contains(&"repo".to_string()));
        assert!(proj.tags.contains(&"infra".to_string()));
    }
}

#[test]
fn project_workstreams_manager_keeps_titles_strips_summaries() {
    let ws = make_workstreams(&["WS Alpha", "WS Beta"]);
    let out = project_workstreams_with_aliases(&ws, "manager", &stable_alias);

    for (orig, proj) in ws.workstreams.iter().zip(out.workstreams.iter()) {
        assert_eq!(proj.title, orig.title);
        assert!(proj.summary.is_none());
        assert!(proj.tags.contains(&"repo".to_string()));
    }
}

#[test]
fn project_workstreams_internal_is_identity() {
    let ws = make_workstreams(&["WS Alpha"]);
    let out = project_workstreams_with_aliases(&ws, "internal", &stable_alias);
    assert_eq!(out, ws);
}

// ============================================================================
// Unknown profiles behave like public
// ============================================================================

#[test]
fn various_unknown_profiles_all_behave_like_public() {
    let events = vec![make_pr_event("Secret", "org/repo")];
    let public_out = project_events_with_aliases(&events, "public", &stable_alias);

    for unknown in ["unknown", "INTERNAL", "admin", "root", "🔒", "", " "] {
        let out = project_events_with_aliases(&events, unknown, &stable_alias);
        assert_eq!(
            out, public_out,
            "profile {unknown:?} should behave like public"
        );
    }
}

#[test]
fn unknown_profiles_for_workstreams_behave_like_public() {
    let ws = make_workstreams(&["WS"]);
    let public_out = project_workstreams_with_aliases(&ws, "public", &stable_alias);

    for unknown in ["unknown", "MANAGER", "", "🔓"] {
        let out = project_workstreams_with_aliases(&ws, unknown, &stable_alias);
        assert_eq!(
            out, public_out,
            "profile {unknown:?} should behave like public for workstreams"
        );
    }
}

// ============================================================================
// Empty inputs
// ============================================================================

#[test]
fn empty_events_all_profiles() {
    let events: Vec<EventEnvelope> = vec![];
    for profile in ["internal", "manager", "public"] {
        let out = project_events_with_aliases(&events, profile, &stable_alias);
        assert!(out.is_empty());
    }
}

#[test]
fn empty_workstreams_all_profiles() {
    let ws = WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![],
    };
    for profile in ["internal", "manager", "public"] {
        let out = project_workstreams_with_aliases(&ws, profile, &stable_alias);
        assert!(out.workstreams.is_empty());
    }
}

// ============================================================================
// Projection consistency (idempotency for profile string)
// ============================================================================

#[test]
fn same_profile_string_produces_identical_results() {
    let events = vec![make_pr_event("title", "org/repo")];
    let out1 = project_events_with_aliases(&events, "public", &stable_alias);
    let out2 = project_events_with_aliases(&events, "public", &stable_alias);
    assert_eq!(out1, out2);
}

#[test]
fn canonical_and_parsed_profile_produce_same_result() {
    let events = vec![make_pr_event("title", "org/repo")];
    for profile_str in ["internal", "manager", "public"] {
        let parsed = parse_profile(profile_str);
        let canonical = parsed.as_str();
        let out_raw = project_events_with_aliases(&events, profile_str, &stable_alias);
        let out_canonical = project_events_with_aliases(&events, canonical, &stable_alias);
        assert_eq!(out_raw, out_canonical, "profile {profile_str}");
    }
}

// ============================================================================
// Tags preserved across all profiles
// ============================================================================

#[test]
fn event_tags_preserved_across_all_profiles() {
    let event = make_pr_event("title", "org/repo");
    for profile in ["internal", "manager", "public"] {
        let out = project_events_with_aliases(std::slice::from_ref(&event), profile, &stable_alias);
        assert_eq!(
            out[0].tags, event.tags,
            "tags changed for profile {profile}"
        );
    }
}

// ============================================================================
// Workstream stats and events preserved
// ============================================================================

#[test]
fn workstream_stats_preserved_across_all_profiles() {
    let ws = make_workstreams(&["WS"]);
    for profile in ["internal", "manager", "public"] {
        let out = project_workstreams_with_aliases(&ws, profile, &stable_alias);
        assert_eq!(
            out.workstreams[0].stats, ws.workstreams[0].stats,
            "stats changed for profile {profile}"
        );
    }
}
