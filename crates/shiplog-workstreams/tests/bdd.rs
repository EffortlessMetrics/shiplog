//! BDD-style integration tests for workstream clustering and YAML override.
//!
//! Exercises repo-based clustering and the WorkstreamManager file precedence.

use chrono::Utc;
use shiplog_ids::EventId;
use shiplog_ports::WorkstreamClusterer;
use shiplog_schema::event::*;
use shiplog_schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};
use shiplog_testkit::bdd::Scenario;
use shiplog_testkit::bdd::assertions::*;
use shiplog_workstreams::{RepoClusterer, WorkstreamManager, write_workstreams};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_event(repo: &str, id_suffix: &str, number: u64, kind: EventKind) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["bdd", id_suffix]),
        kind: kind.clone(),
        occurred_at: Utc::now(),
        actor: Actor {
            login: "dev".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: repo.into(),
            html_url: Some(format!("https://github.com/{repo}")),
            visibility: RepoVisibility::Unknown,
        },
        payload: match kind {
            EventKind::PullRequest => EventPayload::PullRequest(PullRequestEvent {
                number,
                title: format!("PR #{number}"),
                state: PullRequestState::Merged,
                created_at: Utc::now(),
                merged_at: Some(Utc::now()),
                additions: Some(5),
                deletions: Some(1),
                changed_files: Some(2),
                touched_paths_hint: vec![],
                window: None,
            }),
            EventKind::Review => EventPayload::Review(ReviewEvent {
                pull_number: number,
                pull_title: format!("Review #{number}"),
                submitted_at: Utc::now(),
                state: "approved".into(),
                window: None,
            }),
            EventKind::Manual => EventPayload::Manual(ManualEvent {
                event_type: ManualEventType::Note,
                title: format!("Manual #{number}"),
                description: None,
                started_at: None,
                ended_at: None,
                impact: None,
            }),
        },
        tags: vec![],
        links: vec![],
        source: SourceRef {
            system: SourceSystem::Unknown,
            url: None,
            opaque_id: None,
        },
    }
}

fn make_workstreams(title: &str) -> WorkstreamsFile {
    WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![Workstream {
            id: shiplog_ids::WorkstreamId::from_parts(["bdd", "ws", title]),
            title: title.to_string(),
            summary: Some("test workstream".into()),
            tags: vec!["repo".into()],
            stats: WorkstreamStats::zero(),
            events: vec![],
            receipts: vec![],
        }],
    }
}

// ===========================================================================
// Scenario 1: RepoClusterer groups events by repository
// ===========================================================================

#[test]
fn scenario_repo_clusterer_groups_by_repo() {
    Scenario::new("RepoClusterer groups events into per-repo workstreams")
        .given("events from three different repositories", |ctx| {
            ctx.numbers.insert("repo_count".into(), 3);
            ctx.numbers.insert("total_events".into(), 5);
        })
        .when("the RepoClusterer clusters the events", |ctx| {
            let events = vec![
                make_event("org/api", "pr1", 1, EventKind::PullRequest),
                make_event("org/api", "pr2", 2, EventKind::PullRequest),
                make_event("org/web", "pr3", 3, EventKind::PullRequest),
                make_event("org/infra", "pr4", 4, EventKind::PullRequest),
                make_event("org/infra", "pr5", 5, EventKind::PullRequest),
            ];

            let ws = RepoClusterer.cluster(&events).map_err(|e| e.to_string())?;
            ctx.numbers
                .insert("workstream_count".into(), ws.workstreams.len() as u64);

            let total_events: u64 = ws.workstreams.iter().map(|w| w.events.len() as u64).sum();
            ctx.numbers.insert("clustered_events".into(), total_events);

            let titles: Vec<String> = ws.workstreams.iter().map(|w| w.title.clone()).collect();
            ctx.flags
                .insert("has_api".into(), titles.contains(&"org/api".to_string()));
            ctx.flags
                .insert("has_web".into(), titles.contains(&"org/web".to_string()));
            ctx.flags.insert(
                "has_infra".into(),
                titles.contains(&"org/infra".to_string()),
            );
            ctx.flags.insert(
                "all_tagged_repo".into(),
                ws.workstreams
                    .iter()
                    .all(|w| w.tags.contains(&"repo".to_string())),
            );
            Ok(())
        })
        .then("three workstreams are created", |ctx| {
            let count = assert_present(ctx.number("workstream_count"), "workstream count")?;
            assert_eq(count, 3, "workstream count")
        })
        .then("all five events are assigned", |ctx| {
            let total = assert_present(ctx.number("clustered_events"), "clustered events")?;
            assert_eq(total, 5, "total clustered events")
        })
        .then("each repo has a workstream", |ctx| {
            assert_true(ctx.flag("has_api").unwrap_or(false), "org/api workstream")?;
            assert_true(ctx.flag("has_web").unwrap_or(false), "org/web workstream")?;
            assert_true(
                ctx.flag("has_infra").unwrap_or(false),
                "org/infra workstream",
            )
        })
        .then("all workstreams are tagged with 'repo'", |ctx| {
            assert_true(
                ctx.flag("all_tagged_repo").unwrap_or(false),
                "all tagged repo",
            )
        })
        .run()
        .expect("repo clustering scenario should pass");
}

// ===========================================================================
// Scenario 2: Mixed event kinds produce correct stats
// ===========================================================================

#[test]
fn scenario_mixed_event_kinds_stats() {
    Scenario::new("Mixed event kinds produce correct workstream stats")
        .given(
            "events with PRs, reviews, and manual entries in one repo",
            |ctx| {
                ctx.strings.insert("repo".into(), "acme/mixed".to_string());
            },
        )
        .when("the RepoClusterer clusters the events", |ctx| {
            let events = vec![
                make_event("acme/mixed", "pr1", 1, EventKind::PullRequest),
                make_event("acme/mixed", "pr2", 2, EventKind::PullRequest),
                make_event("acme/mixed", "rev1", 3, EventKind::Review),
                make_event("acme/mixed", "man1", 4, EventKind::Manual),
                make_event("acme/mixed", "man2", 5, EventKind::Manual),
                make_event("acme/mixed", "man3", 6, EventKind::Manual),
            ];

            let ws = RepoClusterer.cluster(&events).map_err(|e| e.to_string())?;
            let stats = &ws.workstreams[0].stats;
            ctx.numbers
                .insert("pr_count".into(), stats.pull_requests as u64);
            ctx.numbers
                .insert("review_count".into(), stats.reviews as u64);
            ctx.numbers
                .insert("manual_count".into(), stats.manual_events as u64);
            ctx.numbers
                .insert("total_events".into(), ws.workstreams[0].events.len() as u64);
            Ok(())
        })
        .then("2 pull requests in stats", |ctx| {
            let count = assert_present(ctx.number("pr_count"), "PR count")?;
            assert_eq(count, 2, "PR count in stats")
        })
        .then("1 review in stats", |ctx| {
            let count = assert_present(ctx.number("review_count"), "review count")?;
            assert_eq(count, 1, "review count in stats")
        })
        .then("3 manual events in stats", |ctx| {
            let count = assert_present(ctx.number("manual_count"), "manual count")?;
            assert_eq(count, 3, "manual event count in stats")
        })
        .then("6 total events assigned", |ctx| {
            let total = assert_present(ctx.number("total_events"), "total events")?;
            assert_eq(total, 6, "total events in workstream")
        })
        .run()
        .expect("mixed event stats scenario should pass");
}

// ===========================================================================
// Scenario 3: Curated workstreams override suggested
// ===========================================================================

#[test]
fn scenario_curated_overrides_suggested() {
    Scenario::new("Curated workstreams.yaml takes precedence over suggested")
        .given("both curated and suggested workstream files exist", |ctx| {
            let dir = tempfile::tempdir().unwrap();
            let curated = make_workstreams("Curated Auth Overhaul");
            let suggested = make_workstreams("Suggested Auth Work");

            write_workstreams(&WorkstreamManager::curated_path(dir.path()), &curated).unwrap();
            write_workstreams(&WorkstreamManager::suggested_path(dir.path()), &suggested).unwrap();

            ctx.paths.insert("out_dir".into(), dir.path().to_path_buf());
            ctx.strings
                .insert("tmp_root".into(), dir.keep().to_string_lossy().into_owned());
        })
        .when("the WorkstreamManager loads effective workstreams", |ctx| {
            let out_dir = ctx.path("out_dir").unwrap().to_path_buf();
            let ws = WorkstreamManager::load_effective(&out_dir, &RepoClusterer, &[])
                .map_err(|e| e.to_string())?;
            ctx.strings
                .insert("loaded_title".into(), ws.workstreams[0].title.clone());
            Ok(())
        })
        .then("the curated workstream title is loaded", |ctx| {
            let title = assert_present(ctx.string("loaded_title"), "loaded title")?;
            assert_eq(title, "Curated Auth Overhaul", "workstream title")
        })
        .run()
        .expect("curated override scenario should pass");
}

// ===========================================================================
// Scenario 4: Missing files fall back to clustering
// ===========================================================================

#[test]
fn scenario_missing_files_fall_back_to_clustering() {
    Scenario::new("Missing workstream files trigger clustering from events")
        .given("a directory with no workstream files", |ctx| {
            let dir = tempfile::tempdir().unwrap();
            ctx.paths.insert("out_dir".into(), dir.path().to_path_buf());
            ctx.strings
                .insert("tmp_root".into(), dir.keep().to_string_lossy().into_owned());
        })
        .when("WorkstreamManager loads with events", |ctx| {
            let out_dir = ctx.path("out_dir").unwrap().to_path_buf();
            let events = vec![
                make_event("acme/fallback", "fb1", 1, EventKind::PullRequest),
                make_event("acme/fallback", "fb2", 2, EventKind::PullRequest),
            ];

            let ws = WorkstreamManager::load_effective(&out_dir, &RepoClusterer, &events)
                .map_err(|e| e.to_string())?;
            ctx.numbers
                .insert("ws_count".into(), ws.workstreams.len() as u64);
            ctx.strings
                .insert("ws_title".into(), ws.workstreams[0].title.clone());
            ctx.flags.insert(
                "suggested_written".into(),
                WorkstreamManager::suggested_path(&out_dir).exists(),
            );
            Ok(())
        })
        .then("one workstream is generated for the repo", |ctx| {
            let count = assert_present(ctx.number("ws_count"), "workstream count")?;
            assert_eq(count, 1, "generated workstream count")
        })
        .then("the title matches the repo name", |ctx| {
            let title = assert_present(ctx.string("ws_title"), "workstream title")?;
            assert_eq(title, "acme/fallback", "generated workstream title")
        })
        .then("a suggested file is written to disk", |ctx| {
            assert_true(
                ctx.flag("suggested_written").unwrap_or(false),
                "suggested file written",
            )
        })
        .run()
        .expect("fallback clustering scenario should pass");
}

// ===========================================================================
// Scenario 5: Empty events produce no workstreams
// ===========================================================================

#[test]
fn scenario_empty_events_no_workstreams() {
    Scenario::new("Empty event list produces an empty workstreams file")
        .given("no events to cluster", |ctx| {
            ctx.numbers.insert("event_count".into(), 0);
        })
        .when("the RepoClusterer clusters the empty list", |ctx| {
            let ws = RepoClusterer.cluster(&[]).map_err(|e| e.to_string())?;
            ctx.numbers
                .insert("ws_count".into(), ws.workstreams.len() as u64);
            ctx.numbers.insert("version".into(), ws.version as u64);
            Ok(())
        })
        .then("zero workstreams are produced", |ctx| {
            let count = assert_present(ctx.number("ws_count"), "workstream count")?;
            assert_eq(count, 0, "workstream count for empty events")
        })
        .then("version is 1", |ctx| {
            let ver = assert_present(ctx.number("version"), "version")?;
            assert_eq(ver, 1, "workstreams file version")
        })
        .run()
        .expect("empty events scenario should pass");
}

// ===========================================================================
// Scenario 6: Workstreams are sorted by repo name
// ===========================================================================

#[test]
fn scenario_workstreams_sorted_alphabetically() {
    Scenario::new("Workstreams are sorted alphabetically by repo name")
        .given("events from repos in reverse alphabetical order", |ctx| {
            ctx.strings
                .insert("expected_order".into(), "alpha,bravo,zulu".into());
        })
        .when("the RepoClusterer clusters the events", |ctx| {
            let events = vec![
                make_event("zulu", "z1", 1, EventKind::PullRequest),
                make_event("alpha", "a1", 2, EventKind::PullRequest),
                make_event("bravo", "b1", 3, EventKind::PullRequest),
            ];

            let ws = RepoClusterer.cluster(&events).map_err(|e| e.to_string())?;
            let titles: Vec<String> = ws.workstreams.iter().map(|w| w.title.clone()).collect();
            ctx.strings.insert("actual_order".into(), titles.join(","));
            Ok(())
        })
        .then("workstreams appear in alphabetical order", |ctx| {
            let expected = assert_present(ctx.string("expected_order"), "expected order")?;
            let actual = assert_present(ctx.string("actual_order"), "actual order")?;
            assert_eq(actual, expected, "workstream sort order")
        })
        .run()
        .expect("sorting scenario should pass");
}

// ===========================================================================
// Scenario 7: Deterministic IDs for same repo produce same workstream ID
// ===========================================================================

#[test]
fn scenario_deterministic_workstream_ids() {
    Scenario::new("Same repo always produces the same workstream ID")
        .given("events from the same repo", |ctx| {
            ctx.strings
                .insert("repo".into(), "acme/deterministic".into());
        })
        .when("the clusterer runs twice", |ctx| {
            let events = vec![
                make_event("acme/deterministic", "d1", 1, EventKind::PullRequest),
                make_event("acme/deterministic", "d2", 2, EventKind::PullRequest),
            ];

            let ws1 = RepoClusterer.cluster(&events).map_err(|e| e.to_string())?;
            let ws2 = RepoClusterer.cluster(&events).map_err(|e| e.to_string())?;

            ctx.strings
                .insert("id_1".into(), ws1.workstreams[0].id.to_string());
            ctx.strings
                .insert("id_2".into(), ws2.workstreams[0].id.to_string());
            Ok(())
        })
        .then("both runs produce the same workstream ID", |ctx| {
            let id1 = assert_present(ctx.string("id_1"), "first ID")?;
            let id2 = assert_present(ctx.string("id_2"), "second ID")?;
            assert_eq(id1, id2, "workstream IDs should be deterministic")
        })
        .run()
        .expect("deterministic ID scenario should pass");
}
