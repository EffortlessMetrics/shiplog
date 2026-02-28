//! Integration tests for shiplog-team-aggregate: aggregation, I/O, edge cases.

use chrono::{NaiveDate, TimeZone, Utc};
use shiplog_ids::EventId;
use shiplog_schema::coverage::{Completeness, CoverageManifest, TimeWindow};
use shiplog_schema::event::{
    Actor, EventEnvelope, EventKind, EventPayload, PullRequestEvent, PullRequestState, RepoRef,
    RepoVisibility, SourceRef, SourceSystem,
};
use shiplog_team_aggregate::{TeamAggregator, TeamConfig, write_team_outputs};
use std::io::Write;
use std::path::Path;

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

fn make_coverage(run_id: &str) -> CoverageManifest {
    CoverageManifest {
        run_id: shiplog_ids::RunId(run_id.to_string()),
        generated_at: Utc::now(),
        user: "alice".to_string(),
        window: TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        },
        mode: "test".to_string(),
        sources: vec!["github".to_string()],
        slices: vec![],
        warnings: vec![],
        completeness: Completeness::Complete,
    }
}

fn write_member(dir: &Path, member: &str, events: &[EventEnvelope], cov: &CoverageManifest) {
    let run_dir = dir.join(member);
    std::fs::create_dir_all(&run_dir).unwrap();

    let mut f = std::fs::File::create(run_dir.join("ledger.events.jsonl")).unwrap();
    for event in events {
        writeln!(f, "{}", serde_json::to_string(event).unwrap()).unwrap();
    }
    std::fs::write(
        run_dir.join("coverage.manifest.json"),
        serde_json::to_string_pretty(cov).unwrap(),
    )
    .unwrap();
}

fn write_member_default(dir: &Path, member: &str) {
    let when = Utc.with_ymd_and_hms(2025, 1, 15, 12, 0, 0).unwrap();
    let events = vec![
        make_event(&format!("{member}-1"), "org/repo-a", when),
        make_event(&format!("{member}-2"), "org/repo-b", when),
    ];
    write_member(
        dir,
        member,
        &events,
        &make_coverage(&format!("{member}-run")),
    );
}

// ---------------------------------------------------------------------------
// TeamAggregator::aggregate
// ---------------------------------------------------------------------------

#[test]
fn aggregate_no_members_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let agg = TeamAggregator::new(TeamConfig::default());
    let result = agg.aggregate(dir.path());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("No team members"));
}

#[test]
fn aggregate_single_member() {
    let dir = tempfile::tempdir().unwrap();
    write_member_default(dir.path(), "alice");

    let agg = TeamAggregator::new(TeamConfig {
        members: vec!["alice".into()],
        ..TeamConfig::default()
    });
    let result = agg.aggregate(dir.path()).unwrap();
    assert_eq!(result.included_members, vec!["alice"]);
    assert_eq!(result.events.len(), 2);
    assert!(result.missing_members.is_empty());
    assert!(result.warnings.is_empty());
}

#[test]
fn aggregate_multiple_members() {
    let dir = tempfile::tempdir().unwrap();
    write_member_default(dir.path(), "alice");
    write_member_default(dir.path(), "bob");

    let agg = TeamAggregator::new(TeamConfig {
        members: vec!["alice".into(), "bob".into()],
        ..TeamConfig::default()
    });
    let result = agg.aggregate(dir.path()).unwrap();
    assert_eq!(result.included_members.len(), 2);
    assert_eq!(result.events.len(), 4);
    assert!(result.warnings.is_empty());
}

#[test]
fn aggregate_deduplicates_member_list() {
    let dir = tempfile::tempdir().unwrap();
    write_member_default(dir.path(), "alice");

    let agg = TeamAggregator::new(TeamConfig {
        members: vec!["alice".into(), "alice".into()],
        ..TeamConfig::default()
    });
    let result = agg.aggregate(dir.path()).unwrap();
    assert_eq!(result.included_members, vec!["alice"]);
    assert_eq!(result.requested_members, vec!["alice"]);
}

#[test]
fn aggregate_missing_member_produces_warning() {
    let dir = tempfile::tempdir().unwrap();
    write_member_default(dir.path(), "alice");

    let agg = TeamAggregator::new(TeamConfig {
        members: vec!["alice".into(), "ghost".into()],
        ..TeamConfig::default()
    });
    let result = agg.aggregate(dir.path()).unwrap();
    assert_eq!(result.included_members, vec!["alice"]);
    assert_eq!(result.missing_members, vec!["ghost"]);
    assert!(result.warnings.iter().any(|w| w.contains("Missing ledger")));
}

#[test]
fn aggregate_all_missing_returns_empty_events() {
    let dir = tempfile::tempdir().unwrap();

    let agg = TeamAggregator::new(TeamConfig {
        members: vec!["ghost1".into(), "ghost2".into()],
        ..TeamConfig::default()
    });
    let result = agg.aggregate(dir.path()).unwrap();
    assert!(result.events.is_empty());
    assert!(result.included_members.is_empty());
    assert_eq!(result.missing_members.len(), 2);
}

#[test]
fn aggregate_uses_alias_for_display_name() {
    let dir = tempfile::tempdir().unwrap();
    write_member_default(dir.path(), "alice");

    let agg = TeamAggregator::new(TeamConfig {
        members: vec!["alice".into()],
        aliases: std::collections::HashMap::from([("alice".into(), "Alice S.".into())]),
        ..TeamConfig::default()
    });
    let result = agg.aggregate(dir.path()).unwrap();
    assert_eq!(result.members[0].display_name, "Alice S.");
}

#[test]
fn aggregate_date_filter_since() {
    let dir = tempfile::tempdir().unwrap();
    let early = Utc.with_ymd_and_hms(2025, 1, 5, 0, 0, 0).unwrap();
    let late = Utc.with_ymd_and_hms(2025, 1, 20, 0, 0, 0).unwrap();
    let events = vec![
        make_event("early", "org/repo", early),
        make_event("late", "org/repo", late),
    ];
    write_member(dir.path(), "alice", &events, &make_coverage("run"));

    let agg = TeamAggregator::new(TeamConfig {
        members: vec!["alice".into()],
        since: Some(NaiveDate::from_ymd_opt(2025, 1, 15).unwrap()),
        ..TeamConfig::default()
    });
    let result = agg.aggregate(dir.path()).unwrap();
    assert_eq!(result.events.len(), 1);
}

#[test]
fn aggregate_date_filter_until() {
    let dir = tempfile::tempdir().unwrap();
    let early = Utc.with_ymd_and_hms(2025, 1, 5, 0, 0, 0).unwrap();
    let late = Utc.with_ymd_and_hms(2025, 1, 20, 0, 0, 0).unwrap();
    let events = vec![
        make_event("early", "org/repo", early),
        make_event("late", "org/repo", late),
    ];
    write_member(dir.path(), "alice", &events, &make_coverage("run"));

    let agg = TeamAggregator::new(TeamConfig {
        members: vec!["alice".into()],
        until: Some(NaiveDate::from_ymd_opt(2025, 1, 10).unwrap()),
        ..TeamConfig::default()
    });
    let result = agg.aggregate(dir.path()).unwrap();
    assert_eq!(result.events.len(), 1);
}

#[test]
fn aggregate_date_filter_both() {
    let dir = tempfile::tempdir().unwrap();
    let jan5 = Utc.with_ymd_and_hms(2025, 1, 5, 0, 0, 0).unwrap();
    let jan15 = Utc.with_ymd_and_hms(2025, 1, 15, 0, 0, 0).unwrap();
    let jan25 = Utc.with_ymd_and_hms(2025, 1, 25, 0, 0, 0).unwrap();
    let events = vec![
        make_event("early", "org/repo", jan5),
        make_event("mid", "org/repo", jan15),
        make_event("late", "org/repo", jan25),
    ];
    write_member(dir.path(), "alice", &events, &make_coverage("run"));

    let agg = TeamAggregator::new(TeamConfig {
        members: vec!["alice".into()],
        since: Some(NaiveDate::from_ymd_opt(2025, 1, 10).unwrap()),
        until: Some(NaiveDate::from_ymd_opt(2025, 1, 20).unwrap()),
        ..TeamConfig::default()
    });
    let result = agg.aggregate(dir.path()).unwrap();
    assert_eq!(result.events.len(), 1);
}

#[test]
fn aggregate_member_with_empty_events_after_filter_produces_warning() {
    let dir = tempfile::tempdir().unwrap();
    let jan5 = Utc.with_ymd_and_hms(2025, 1, 5, 0, 0, 0).unwrap();
    let events = vec![make_event("old", "org/repo", jan5)];
    write_member(dir.path(), "alice", &events, &make_coverage("run"));

    let agg = TeamAggregator::new(TeamConfig {
        members: vec!["alice".into()],
        since: Some(NaiveDate::from_ymd_opt(2025, 6, 1).unwrap()),
        until: Some(NaiveDate::from_ymd_opt(2025, 7, 1).unwrap()),
        ..TeamConfig::default()
    });
    let result = agg.aggregate(dir.path()).unwrap();
    assert!(
        result
            .warnings
            .iter()
            .any(|w| w.contains("zero events after filtering"))
    );
}

#[test]
fn aggregate_schema_version_compatible() {
    let dir = tempfile::tempdir().unwrap();
    let mut cov = make_coverage("run");
    cov.completeness = Completeness::Complete;
    let when = Utc.with_ymd_and_hms(2025, 1, 15, 0, 0, 0).unwrap();
    write_member(
        dir.path(),
        "alice",
        &[make_event("e1", "org/repo", when)],
        &cov,
    );

    let agg = TeamAggregator::new(TeamConfig {
        members: vec!["alice".into()],
        ..TeamConfig::default()
    });
    let result = agg.aggregate(dir.path()).unwrap();
    assert_eq!(result.included_members, vec!["alice"]);
}

// ---------------------------------------------------------------------------
// TeamAggregator::render_packet_markdown
// ---------------------------------------------------------------------------

#[test]
fn render_packet_from_aggregate() {
    let dir = tempfile::tempdir().unwrap();
    write_member_default(dir.path(), "alice");

    let agg = TeamAggregator::new(TeamConfig {
        members: vec!["alice".into()],
        ..TeamConfig::default()
    });
    let result = agg.aggregate(dir.path()).unwrap();
    let packet = agg.render_packet_markdown(&result).unwrap();
    assert!(packet.contains("# Team Shipping Packet"));
    assert!(packet.contains("Total events: 2"));
}

// ---------------------------------------------------------------------------
// write_team_outputs
// ---------------------------------------------------------------------------

#[test]
fn write_team_outputs_creates_files() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_member_default(root, "alice");

    let agg = TeamAggregator::new(TeamConfig {
        members: vec!["alice".into()],
        ..TeamConfig::default()
    });
    let result = agg.aggregate(root).unwrap();
    let packet = agg.render_packet_markdown(&result).unwrap();

    let out_dir = root.join("output");
    let files = write_team_outputs(&out_dir, &packet, &result).unwrap();

    assert!(files.packet.exists());
    assert!(files.events.exists());
    assert!(files.coverage.exists());

    let packet_text = std::fs::read_to_string(&files.packet).unwrap();
    assert!(packet_text.contains("# Team Shipping Packet"));

    let events_text = std::fs::read_to_string(&files.events).unwrap();
    let lines: Vec<&str> = events_text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();
    assert_eq!(lines.len(), 2);

    let coverage_text = std::fs::read_to_string(&files.coverage).unwrap();
    let cov: serde_json::Value = serde_json::from_str(&coverage_text).unwrap();
    assert!(cov.get("run_id").is_some());
}

#[test]
fn write_team_outputs_creates_directory_if_needed() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_member_default(root, "alice");

    let agg = TeamAggregator::new(TeamConfig {
        members: vec!["alice".into()],
        ..TeamConfig::default()
    });
    let result = agg.aggregate(root).unwrap();
    let packet = agg.render_packet_markdown(&result).unwrap();

    let deep_dir = root.join("a").join("b").join("c");
    let files = write_team_outputs(&deep_dir, &packet, &result).unwrap();
    assert!(files.packet.exists());
}

#[test]
fn write_team_outputs_empty_events() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    let agg = TeamAggregator::new(TeamConfig {
        members: vec!["ghost".into()],
        ..TeamConfig::default()
    });
    let result = agg.aggregate(root).unwrap();
    let packet = agg.render_packet_markdown(&result).unwrap();

    let out_dir = root.join("output");
    let files = write_team_outputs(&out_dir, &packet, &result).unwrap();

    let events_text = std::fs::read_to_string(&files.events).unwrap();
    assert!(events_text.trim().is_empty());
}

// ---------------------------------------------------------------------------
// snapshot: aggregate result coverage JSON
// ---------------------------------------------------------------------------

#[test]
fn snapshot_coverage_output() {
    let dir = tempfile::tempdir().unwrap();
    write_member_default(dir.path(), "alice");

    let agg = TeamAggregator::new(TeamConfig {
        members: vec!["alice".into()],
        ..TeamConfig::default()
    });
    let result = agg.aggregate(dir.path()).unwrap();

    // Replace non-deterministic fields for snapshot stability
    let mut cov_json: serde_json::Value = serde_json::to_value(&result.coverage).unwrap();
    cov_json["run_id"] = serde_json::Value::String("REDACTED".into());
    cov_json["generated_at"] = serde_json::Value::String("REDACTED".into());

    insta::assert_snapshot!(
        "aggregate_coverage",
        serde_json::to_string_pretty(&cov_json).unwrap()
    );
}

// ---------------------------------------------------------------------------
// proptest: aggregation invariants
// ---------------------------------------------------------------------------

mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn included_plus_missing_plus_incompatible_equals_requested(
            n_present in 0usize..4,
            n_missing in 0usize..4,
        ) {
            let dir = tempfile::tempdir().unwrap();
            let mut members = Vec::new();

            for i in 0..n_present {
                let name = format!("present{i}");
                write_member_default(dir.path(), &name);
                members.push(name);
            }
            for i in 0..n_missing {
                members.push(format!("missing{i}"));
            }

            if members.is_empty() {
                return Ok(());
            }

            let agg = TeamAggregator::new(TeamConfig {
                members,
                ..TeamConfig::default()
            });
            let result = agg.aggregate(dir.path()).unwrap();

            let total = result.included_members.len()
                + result.missing_members.len()
                + result.incompatible_members.len();
            prop_assert_eq!(
                total,
                result.requested_members.len(),
                "member counts don't sum"
            );
        }
    }
}
