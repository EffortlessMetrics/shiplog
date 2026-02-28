//! BDD scenarios for shiplog-team-aggregate: aggregation, missing members, date filtering.

use chrono::{NaiveDate, TimeZone, Utc};
use shiplog_ids::EventId;
use shiplog_schema::coverage::{Completeness, CoverageManifest, TimeWindow};
use shiplog_schema::event::{
    Actor, EventEnvelope, EventKind, EventPayload, PullRequestEvent, PullRequestState, RepoRef,
    RepoVisibility, SourceRef, SourceSystem,
};
use shiplog_team_aggregate::{TeamAggregator, TeamConfig};
use shiplog_testkit::bdd::{Scenario, assertions};
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
            login: "user".to_string(),
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
        user: "user".to_string(),
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

// ---------------------------------------------------------------------------
// Scenario: Given multiple team members, when aggregating, then all events collected
// ---------------------------------------------------------------------------

#[test]
fn bdd_aggregate_multiple_members_collects_all_events() {
    Scenario::new("Aggregating multiple team members collects all events")
        .given("two team members with ledgers on disk", |ctx| {
            let dir = tempfile::tempdir().expect("tempdir").keep();
            let when = Utc.with_ymd_and_hms(2025, 1, 15, 12, 0, 0).unwrap();
            let alice_events = vec![
                make_event("alice-1", "org/repo-a", when),
                make_event("alice-2", "org/repo-b", when),
            ];
            let bob_events = vec![
                make_event("bob-1", "org/repo-a", when),
                make_event("bob-2", "org/repo-c", when),
                make_event("bob-3", "org/repo-c", when),
            ];
            write_member(&dir, "alice", &alice_events, &make_coverage("alice-run"));
            write_member(&dir, "bob", &bob_events, &make_coverage("bob-run"));

            ctx.strings
                .insert("root_dir".into(), dir.to_string_lossy().into_owned());
            ctx.numbers.insert("alice_events".into(), 2);
            ctx.numbers.insert("bob_events".into(), 3);
        })
        .when("aggregating both members", |ctx| {
            let root = ctx.string("root_dir").unwrap().to_string();
            let agg = TeamAggregator::new(TeamConfig {
                members: vec!["alice".into(), "bob".into()],
                ..TeamConfig::default()
            });
            let result = agg
                .aggregate(Path::new(&root))
                .map_err(|e| format!("aggregate failed: {e}"))?;

            ctx.numbers
                .insert("total_events".into(), result.events.len() as u64);
            ctx.numbers.insert(
                "included_count".into(),
                result.included_members.len() as u64,
            );
            ctx.numbers
                .insert("missing_count".into(), result.missing_members.len() as u64);
            ctx.numbers
                .insert("warning_count".into(), result.warnings.len() as u64);
            Ok(())
        })
        .then("all events from both members are collected", |ctx| {
            let total = assertions::assert_present(ctx.number("total_events"), "total_events")?;
            let expected = ctx.number("alice_events").unwrap() + ctx.number("bob_events").unwrap();
            assertions::assert_eq(total, expected, "total event count")
        })
        .then("both members are marked as included", |ctx| {
            let included =
                assertions::assert_present(ctx.number("included_count"), "included_count")?;
            assertions::assert_eq(included, 2, "included members")
        })
        .then("no members are missing", |ctx| {
            let missing = assertions::assert_present(ctx.number("missing_count"), "missing_count")?;
            assertions::assert_eq(missing, 0, "missing members")
        })
        .then("no warnings are produced", |ctx| {
            let warnings =
                assertions::assert_present(ctx.number("warning_count"), "warning_count")?;
            assertions::assert_eq(warnings, 0, "warning count")
        })
        .run()
        .expect("scenario should pass");
}

// ---------------------------------------------------------------------------
// Scenario: Given member with no events, when aggregating, then member listed as missing
// ---------------------------------------------------------------------------

#[test]
fn bdd_missing_member_listed_in_result() {
    Scenario::new("Missing member is reported in aggregation result")
        .given("one member with a ledger and one without", |ctx| {
            let dir = tempfile::tempdir().expect("tempdir").keep();
            let when = Utc.with_ymd_and_hms(2025, 1, 15, 12, 0, 0).unwrap();
            let events = vec![make_event("alice-1", "org/repo", when)];
            write_member(&dir, "alice", &events, &make_coverage("alice-run"));
            // "ghost" has no ledger directory at all

            ctx.strings
                .insert("root_dir".into(), dir.to_string_lossy().into_owned());
        })
        .when("aggregating alice and ghost", |ctx| {
            let root = ctx.string("root_dir").unwrap().to_string();
            let agg = TeamAggregator::new(TeamConfig {
                members: vec!["alice".into(), "ghost".into()],
                ..TeamConfig::default()
            });
            let result = agg
                .aggregate(Path::new(&root))
                .map_err(|e| format!("aggregate failed: {e}"))?;

            ctx.numbers.insert(
                "included_count".into(),
                result.included_members.len() as u64,
            );
            ctx.numbers
                .insert("missing_count".into(), result.missing_members.len() as u64);
            ctx.flags.insert(
                "has_missing_warning".into(),
                result.warnings.iter().any(|w| w.contains("Missing ledger")),
            );
            ctx.strings
                .insert("missing_list".into(), result.missing_members.join(","));
            // Check that member summaries capture the missing member
            ctx.numbers
                .insert("summary_count".into(), result.members.len() as u64);
            Ok(())
        })
        .then("alice is included", |ctx| {
            let count = assertions::assert_present(ctx.number("included_count"), "included_count")?;
            assertions::assert_eq(count, 1, "included count")
        })
        .then("ghost is listed as missing", |ctx| {
            let count = assertions::assert_present(ctx.number("missing_count"), "missing_count")?;
            assertions::assert_eq(count, 1, "missing count")?;
            let list = assertions::assert_present(ctx.string("missing_list"), "missing_list")?;
            assertions::assert_contains(list, "ghost", "missing list")
        })
        .then("a warning is produced for the missing member", |ctx| {
            let has_warning =
                assertions::assert_present(ctx.flag("has_missing_warning"), "has_missing_warning")?;
            assertions::assert_true(has_warning, "missing ledger warning present")
        })
        .then("both members appear in summaries", |ctx| {
            let count = assertions::assert_present(ctx.number("summary_count"), "summary_count")?;
            assertions::assert_eq(count, 2, "summary count covers all requested members")
        })
        .run()
        .expect("scenario should pass");
}

// ---------------------------------------------------------------------------
// Scenario: Given overlapping date ranges, when filtering, then correct events included
// ---------------------------------------------------------------------------

#[test]
fn bdd_date_range_filtering_includes_correct_events() {
    Scenario::new("Date range filtering includes only events in window")
        .given("a member with events across January 2025", |ctx| {
            let dir = tempfile::tempdir().expect("tempdir").keep();
            let jan5 = Utc.with_ymd_and_hms(2025, 1, 5, 0, 0, 0).unwrap();
            let jan15 = Utc.with_ymd_and_hms(2025, 1, 15, 0, 0, 0).unwrap();
            let jan25 = Utc.with_ymd_and_hms(2025, 1, 25, 0, 0, 0).unwrap();
            let events = vec![
                make_event("early", "org/repo", jan5),
                make_event("mid", "org/repo", jan15),
                make_event("late", "org/repo", jan25),
            ];
            write_member(&dir, "alice", &events, &make_coverage("run"));

            ctx.strings
                .insert("root_dir".into(), dir.to_string_lossy().into_owned());
            ctx.numbers.insert("total_on_disk".into(), 3);
        })
        .when("aggregating with since=Jan 10 and until=Jan 20", |ctx| {
            let root = ctx.string("root_dir").unwrap().to_string();
            let agg = TeamAggregator::new(TeamConfig {
                members: vec!["alice".into()],
                since: Some(NaiveDate::from_ymd_opt(2025, 1, 10).unwrap()),
                until: Some(NaiveDate::from_ymd_opt(2025, 1, 20).unwrap()),
                ..TeamConfig::default()
            });
            let result = agg
                .aggregate(Path::new(&root))
                .map_err(|e| format!("aggregate failed: {e}"))?;

            ctx.numbers
                .insert("filtered_events".into(), result.events.len() as u64);
            ctx.numbers.insert(
                "included_count".into(),
                result.included_members.len() as u64,
            );
            Ok(())
        })
        .then("only the mid-January event is included", |ctx| {
            let count =
                assertions::assert_present(ctx.number("filtered_events"), "filtered_events")?;
            assertions::assert_eq(count, 1, "filtered event count")
        })
        .then("the member is still included", |ctx| {
            let count = assertions::assert_present(ctx.number("included_count"), "included_count")?;
            assertions::assert_eq(count, 1, "included member count")
        })
        .run()
        .expect("scenario should pass");
}
