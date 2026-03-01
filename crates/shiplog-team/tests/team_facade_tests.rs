//! Integration tests for shiplog-team facade crate.
//!
//! Verifies that public re-exports from shiplog-team-aggregate are accessible
//! through the facade, and that the facade wires correctly end-to-end.

use chrono::{NaiveDate, TimeZone, Utc};
use shiplog_ids::EventId;
use shiplog_schema::coverage::{Completeness, CoverageManifest, TimeWindow};
use shiplog_schema::event::{
    Actor, EventEnvelope, EventKind, EventPayload, PullRequestEvent, PullRequestState, RepoRef,
    RepoVisibility, SourceRef, SourceSystem,
};
use std::io::Write;
use std::path::Path;

// Import everything through the facade
use shiplog_team::{
    TeamAggregateResult, TeamAggregator, TeamConfig, TeamMemberSummary, TeamOutputFiles,
    parse_alias_list, parse_csv_list, resolve_team_config, write_team_outputs,
};

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn make_event(id: &str, repo: &str, when: chrono::DateTime<Utc>) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["team", id]),
        kind: EventKind::PullRequest,
        occurred_at: when,
        actor: Actor {
            login: "member".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: repo.into(),
            html_url: None,
            visibility: RepoVisibility::Public,
        },
        payload: EventPayload::PullRequest(PullRequestEvent {
            number: 1,
            title: format!("PR {id}"),
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
        run_id: shiplog_ids::RunId("facade-test".into()),
        generated_at: Utc::now(),
        user: "member".into(),
        window: TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        },
        mode: "test".into(),
        sources: vec!["github".into()],
        slices: vec![],
        warnings: vec![],
        completeness: Completeness::Complete,
    }
}

fn write_member(dir: &Path, member: &str) {
    let run_dir = dir.join(member);
    std::fs::create_dir_all(&run_dir).unwrap();

    let when = Utc.with_ymd_and_hms(2025, 1, 15, 12, 0, 0).unwrap();
    let events = vec![make_event(&format!("{member}-1"), "org/repo", when)];

    let mut f = std::fs::File::create(run_dir.join("ledger.events.jsonl")).unwrap();
    for event in &events {
        writeln!(f, "{}", serde_json::to_string(event).unwrap()).unwrap();
    }
    std::fs::write(
        run_dir.join("coverage.manifest.json"),
        serde_json::to_string_pretty(&make_coverage()).unwrap(),
    )
    .unwrap();
}

// ---------------------------------------------------------------------------
// Re-export accessibility
// ---------------------------------------------------------------------------

#[test]
fn facade_re_exports_parse_csv_list() {
    let result = parse_csv_list("a,b,c");
    assert_eq!(result, vec!["a", "b", "c"]);
}

#[test]
fn facade_re_exports_parse_alias_list() {
    let aliases = parse_alias_list(&["a=A Name".to_string()]).unwrap();
    assert_eq!(aliases.get("a").unwrap(), "A Name");
}

#[test]
fn facade_re_exports_resolve_team_config() {
    let cfg = resolve_team_config(
        None,
        Some("alice,bob".into()),
        None,
        None,
        None,
        None,
        None,
        vec![],
    )
    .unwrap();
    assert_eq!(cfg.members, vec!["alice", "bob"]);
}

// ---------------------------------------------------------------------------
// End-to-end: aggregate + render + write via facade
// ---------------------------------------------------------------------------

#[test]
fn facade_end_to_end() {
    let dir = tempfile::tempdir().unwrap();
    write_member(dir.path(), "alice");

    let cfg = TeamConfig {
        members: vec!["alice".into()],
        ..TeamConfig::default()
    };
    let agg = TeamAggregator::new(cfg);
    let result: TeamAggregateResult = agg.aggregate(dir.path()).unwrap();
    assert_eq!(result.included_members, vec!["alice"]);

    let packet = agg.render_packet_markdown(&result).unwrap();
    assert!(packet.contains("# Team Shipping Packet"));

    let out_dir = dir.path().join("output");
    let files: TeamOutputFiles = write_team_outputs(&out_dir, &packet, &result).unwrap();
    assert!(files.packet.exists());
    assert!(files.events.exists());
    assert!(files.coverage.exists());
}

#[test]
fn facade_type_team_member_summary_accessible() {
    let _summary = TeamMemberSummary {
        member: "test".into(),
        display_name: "Test".into(),
        included: true,
        ledger_dir: std::path::PathBuf::from("test"),
        event_count: 0,
        warnings: vec![],
    };
}
