//! Integration tests for shiplog-team-render: packet rendering, templates, edge cases.

use chrono::{NaiveDate, TimeZone, Utc};
use shiplog_ids::{EventId, RunId};
use shiplog_schema::coverage::{Completeness, CoverageManifest, TimeWindow};
use shiplog_schema::event::{
    Actor, EventEnvelope, EventKind, EventPayload, PullRequestEvent, PullRequestState, RepoRef,
    RepoVisibility, SourceRef, SourceSystem,
};
use shiplog_team_core::TeamConfig;
use shiplog_team_render::{TeamAggregateResult, TeamMemberSummary, render_packet_markdown};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn make_event(id: &str, repo: &str, when: chrono::DateTime<Utc>) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["team", id]),
        kind: EventKind::PullRequest,
        occurred_at: when,
        actor: Actor {
            login: "alice".to_string(),
            id: None,
        },
        repo: RepoRef {
            full_name: repo.to_string(),
            html_url: None,
            visibility: RepoVisibility::Public,
        },
        payload: EventPayload::PullRequest(PullRequestEvent {
            number: 1,
            title: format!("Event {id}"),
            state: PullRequestState::Merged,
            created_at: when,
            merged_at: Some(when),
            additions: Some(1),
            deletions: Some(0),
            changed_files: Some(1),
            touched_paths_hint: vec![],
            window: Some(TimeWindow {
                since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
            }),
        }),
        tags: vec![],
        links: vec![],
        source: SourceRef {
            system: SourceSystem::Github,
            url: None,
            opaque_id: None,
        },
    }
}

fn make_coverage() -> CoverageManifest {
    CoverageManifest {
        run_id: RunId("test-run".to_string()),
        generated_at: Utc.with_ymd_and_hms(2025, 1, 15, 0, 0, 0).unwrap(),
        user: "team".to_string(),
        window: TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        },
        mode: "merged".to_string(),
        sources: vec!["alice".to_string(), "bob".to_string()],
        slices: vec![],
        warnings: vec![],
        completeness: Completeness::Complete,
    }
}

fn make_result(events: Vec<EventEnvelope>, warnings: Vec<String>) -> TeamAggregateResult {
    TeamAggregateResult {
        requested_members: vec!["alice".into(), "bob".into()],
        included_members: vec!["alice".into()],
        missing_members: vec!["bob".into()],
        incompatible_members: vec![],
        members: vec![
            TeamMemberSummary {
                member: "alice".into(),
                display_name: "Alice S.".into(),
                included: true,
                ledger_dir: PathBuf::from("alice"),
                event_count: events.len(),
                warnings: vec![],
            },
            TeamMemberSummary {
                member: "bob".into(),
                display_name: "Bob".into(),
                included: false,
                ledger_dir: PathBuf::from("bob"),
                event_count: 0,
                warnings: vec!["Missing ledger".to_string()],
            },
        ],
        coverage: make_coverage(),
        events,
        warnings,
    }
}

fn default_result() -> TeamAggregateResult {
    let when = Utc.with_ymd_and_hms(2025, 1, 10, 0, 0, 0).unwrap();
    make_result(
        vec![
            make_event("e1", "org/repo-a", when),
            make_event("e2", "org/repo-a", when),
            make_event("e3", "org/repo-b", when),
        ],
        vec!["Missing ledger".to_string()],
    )
}

// ---------------------------------------------------------------------------
// render_packet_markdown – default template
// ---------------------------------------------------------------------------

#[test]
fn default_packet_has_all_sections() {
    let cfg = TeamConfig::default();
    let packet = render_packet_markdown(&cfg, &default_result()).unwrap();

    assert!(packet.contains("# Team Shipping Packet"));
    assert!(packet.contains("## Summary"));
    assert!(packet.contains("## Coverage"));
    assert!(packet.contains("## Workstreams"));
    assert!(packet.contains("## Receipts"));
    assert!(packet.contains("## Warnings"));
}

#[test]
fn default_packet_member_counts() {
    let cfg = TeamConfig::default();
    let packet = render_packet_markdown(&cfg, &default_result()).unwrap();

    assert!(packet.contains("Members configured: 2"));
    assert!(packet.contains("Members included: 1"));
    assert!(packet.contains("Members missing ledgers: bob"));
}

#[test]
fn default_packet_event_count() {
    let cfg = TeamConfig::default();
    let packet = render_packet_markdown(&cfg, &default_result()).unwrap();
    assert!(packet.contains("Total events: 3"));
}

#[test]
fn default_packet_receipts_section() {
    let cfg = TeamConfig::default();
    let packet = render_packet_markdown(&cfg, &default_result()).unwrap();
    assert!(packet.contains("### Alice S."));
    assert!(packet.contains("Included events: 3"));
    assert!(packet.contains("### Bob"));
    assert!(packet.contains("Not included"));
}

#[test]
fn default_packet_coverage_section() {
    let cfg = TeamConfig::default();
    let packet = render_packet_markdown(&cfg, &default_result()).unwrap();
    assert!(packet.contains("Window: 2025-01-01..2025-02-01"));
}

#[test]
fn default_packet_workstreams_groups_by_repo() {
    let cfg = TeamConfig::default();
    let packet = render_packet_markdown(&cfg, &default_result()).unwrap();
    // repo-a has 2 events, repo-b has 1 — sorted by count desc
    assert!(packet.contains("## org/repo-a\n- events: 2"));
    assert!(packet.contains("## org/repo-b\n- events: 1"));
}

// ---------------------------------------------------------------------------
// section filtering
// ---------------------------------------------------------------------------

#[test]
fn custom_sections_only_renders_requested() {
    let cfg = TeamConfig {
        sections: vec!["summary".into()],
        ..TeamConfig::default()
    };
    let packet = render_packet_markdown(&cfg, &default_result()).unwrap();

    assert!(packet.contains("## Summary"));
    assert!(!packet.contains("## Workstreams"));
    assert!(!packet.contains("## Receipts"));
    assert!(!packet.contains("## Coverage\n"));
}

#[test]
fn no_warnings_section_when_no_warnings() {
    let when = Utc.with_ymd_and_hms(2025, 1, 10, 0, 0, 0).unwrap();
    let result = TeamAggregateResult {
        requested_members: vec!["alice".into()],
        included_members: vec!["alice".into()],
        missing_members: vec![],
        incompatible_members: vec![],
        members: vec![TeamMemberSummary {
            member: "alice".into(),
            display_name: "Alice".into(),
            included: true,
            ledger_dir: PathBuf::from("alice"),
            event_count: 1,
            warnings: vec![],
        }],
        coverage: make_coverage(),
        events: vec![make_event("e1", "org/repo", when)],
        warnings: vec![],
    };

    let cfg = TeamConfig::default();
    let packet = render_packet_markdown(&cfg, &result).unwrap();
    assert!(!packet.contains("## Warnings"));
}

// ---------------------------------------------------------------------------
// empty events edge case
// ---------------------------------------------------------------------------

#[test]
fn empty_events_shows_no_workstream_activity() {
    let result = make_result(vec![], vec![]);
    let cfg = TeamConfig::default();
    let packet = render_packet_markdown(&cfg, &result).unwrap();
    assert!(packet.contains("No workstream activity."));
    assert!(packet.contains("Total events: 0"));
}

// ---------------------------------------------------------------------------
// incompatible members
// ---------------------------------------------------------------------------

#[test]
fn incompatible_members_shown_in_header() {
    let result = TeamAggregateResult {
        requested_members: vec!["alice".into(), "carol".into()],
        included_members: vec!["alice".into()],
        missing_members: vec![],
        incompatible_members: vec!["carol".into()],
        members: vec![],
        coverage: make_coverage(),
        events: vec![],
        warnings: vec![],
    };
    let cfg = TeamConfig::default();
    let packet = render_packet_markdown(&cfg, &result).unwrap();
    assert!(packet.contains("Members skipped for incompatible schema: carol"));
}

// ---------------------------------------------------------------------------
// custom template
// ---------------------------------------------------------------------------

#[test]
fn custom_template_renders_variables() {
    let dir = tempfile::tempdir().unwrap();
    let template_path = dir.path().join("custom.md");
    std::fs::write(
        &template_path,
        "# Custom\nTotal={{ events }}\nIncluded={{ included_member_count }}\nSources={{ coverage_sources }}",
    )
    .unwrap();

    let cfg = TeamConfig {
        template: Some(template_path),
        ..TeamConfig::default()
    };
    let packet = render_packet_markdown(&cfg, &default_result()).unwrap();
    assert!(packet.contains("Total=3"));
    assert!(packet.contains("Included=1"));
    assert!(packet.contains("Sources=alice, bob"));
}

#[test]
fn custom_template_nonexistent_file_returns_error() {
    let cfg = TeamConfig {
        template: Some(PathBuf::from("nonexistent.md")),
        ..TeamConfig::default()
    };
    let result = render_packet_markdown(&cfg, &default_result());
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// snapshot: default packet
// ---------------------------------------------------------------------------

#[test]
fn snapshot_default_packet() {
    let cfg = TeamConfig::default();
    let packet = render_packet_markdown(&cfg, &default_result()).unwrap();
    insta::assert_snapshot!("default_team_packet", packet);
}

#[test]
fn snapshot_summary_only_packet() {
    let cfg = TeamConfig {
        sections: vec!["summary".into()],
        ..TeamConfig::default()
    };
    let packet = render_packet_markdown(&cfg, &default_result()).unwrap();
    insta::assert_snapshot!("summary_only_packet", packet);
}

// ---------------------------------------------------------------------------
// TeamAggregateResult / TeamMemberSummary serialization round-trip
// ---------------------------------------------------------------------------

#[test]
fn member_summary_json_round_trip() {
    let summary = TeamMemberSummary {
        member: "alice".into(),
        display_name: "Alice".into(),
        included: true,
        ledger_dir: PathBuf::from("alice"),
        event_count: 5,
        warnings: vec!["something".into()],
    };
    let json = serde_json::to_string(&summary).unwrap();
    let back: TeamMemberSummary = serde_json::from_str(&json).unwrap();
    assert_eq!(back.member, "alice");
    assert_eq!(back.event_count, 5);
    assert_eq!(back.warnings, vec!["something"]);
}

// ---------------------------------------------------------------------------
// proptest: rendering properties
// ---------------------------------------------------------------------------

mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn default_packet_always_starts_with_heading(
            n_events in 0usize..5,
        ) {
            let when = Utc.with_ymd_and_hms(2025, 1, 10, 0, 0, 0).unwrap();
            let events: Vec<_> = (0..n_events)
                .map(|i| make_event(&format!("e{i}"), "org/repo", when))
                .collect();
            let result = make_result(events, vec![]);
            let cfg = TeamConfig::default();
            let packet = render_packet_markdown(&cfg, &result).unwrap();
            prop_assert!(packet.starts_with("# Team Shipping Packet\n"));
        }

        #[test]
        fn event_count_matches_in_packet(
            n_events in 0usize..10,
        ) {
            let when = Utc.with_ymd_and_hms(2025, 1, 10, 0, 0, 0).unwrap();
            let events: Vec<_> = (0..n_events)
                .map(|i| make_event(&format!("e{i}"), "org/repo", when))
                .collect();
            let result = make_result(events, vec![]);
            let cfg = TeamConfig::default();
            let packet = render_packet_markdown(&cfg, &result).unwrap();
            let expected = format!("Total events: {n_events}");
            prop_assert!(packet.contains(&expected), "missing '{expected}' in packet");
        }
    }
}
