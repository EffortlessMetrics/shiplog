//! BDD scenarios for shiplog-team-render: packet rendering, section filtering.

use chrono::{NaiveDate, TimeZone, Utc};
use shiplog_ids::{EventId, RunId};
use shiplog_schema::coverage::{Completeness, CoverageManifest, TimeWindow};
use shiplog_schema::event::{
    Actor, EventEnvelope, EventKind, EventPayload, PullRequestEvent, PullRequestState, RepoRef,
    RepoVisibility, SourceRef, SourceSystem,
};
use shiplog_team_core::TeamConfig;
use shiplog_team_render::{TeamAggregateResult, TeamMemberSummary, render_packet_markdown};
use shiplog_testkit::bdd::{Scenario, assertions};
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

fn sample_result() -> TeamAggregateResult {
    let when = Utc.with_ymd_and_hms(2025, 1, 10, 0, 0, 0).unwrap();
    TeamAggregateResult {
        requested_members: vec!["alice".into(), "bob".into()],
        included_members: vec!["alice".into(), "bob".into()],
        missing_members: vec![],
        incompatible_members: vec![],
        members: vec![
            TeamMemberSummary {
                member: "alice".into(),
                display_name: "Alice S.".into(),
                included: true,
                ledger_dir: PathBuf::from("alice"),
                event_count: 2,
                warnings: vec![],
            },
            TeamMemberSummary {
                member: "bob".into(),
                display_name: "Bob J.".into(),
                included: true,
                ledger_dir: PathBuf::from("bob"),
                event_count: 1,
                warnings: vec![],
            },
        ],
        coverage: make_coverage(),
        events: vec![
            make_event("e1", "org/repo-a", when),
            make_event("e2", "org/repo-a", when),
            make_event("e3", "org/repo-b", when),
        ],
        warnings: vec![],
    }
}

// ---------------------------------------------------------------------------
// Scenario: Given aggregated team data, when rendering, then all sections present
// ---------------------------------------------------------------------------

#[test]
fn bdd_render_all_sections_present() {
    Scenario::new("Rendering aggregated team data includes all default sections")
        .given(
            "an aggregated team result with two members and three events",
            |ctx| {
                let result = sample_result();
                ctx.numbers
                    .insert("event_count".into(), result.events.len() as u64);
                ctx.numbers
                    .insert("member_count".into(), result.requested_members.len() as u64);
                // Serialize the result so we can access it in the when step
                let json = serde_json::to_string(&result).expect("serialize result");
                ctx.data.insert("result_json".into(), json.into_bytes());
            },
        )
        .when("rendering with default config", |ctx| {
            let json = std::str::from_utf8(ctx.data.get("result_json").unwrap()).unwrap();
            let result: TeamAggregateResult =
                serde_json::from_str(json).expect("deserialize result");
            let cfg = TeamConfig::default();
            let packet =
                render_packet_markdown(&cfg, &result).map_err(|e| format!("render: {e}"))?;
            ctx.strings.insert("packet".into(), packet);
            Ok(())
        })
        .then("the packet contains the heading", |ctx| {
            let packet = assertions::assert_present(ctx.string("packet"), "packet")?;
            assertions::assert_contains(packet, "# Team Shipping Packet", "heading")
        })
        .then("the Summary section is present", |ctx| {
            let packet = assertions::assert_present(ctx.string("packet"), "packet")?;
            assertions::assert_contains(packet, "## Summary", "summary section")
        })
        .then("the Coverage section is present", |ctx| {
            let packet = assertions::assert_present(ctx.string("packet"), "packet")?;
            assertions::assert_contains(packet, "## Coverage", "coverage section")
        })
        .then("the Workstreams section is present", |ctx| {
            let packet = assertions::assert_present(ctx.string("packet"), "packet")?;
            assertions::assert_contains(packet, "## Workstreams", "workstreams section")
        })
        .then("the Receipts section is present", |ctx| {
            let packet = assertions::assert_present(ctx.string("packet"), "packet")?;
            assertions::assert_contains(packet, "## Receipts", "receipts section")
        })
        .then("member display names appear in receipts", |ctx| {
            let packet = assertions::assert_present(ctx.string("packet"), "packet")?;
            assertions::assert_contains(packet, "### Alice S.", "alice receipt")?;
            assertions::assert_contains(packet, "### Bob J.", "bob receipt")
        })
        .then("event count matches", |ctx| {
            let packet = assertions::assert_present(ctx.string("packet"), "packet")?;
            assertions::assert_contains(packet, "Total events: 3", "event count in packet")
        })
        .run()
        .expect("scenario should pass");
}

// ---------------------------------------------------------------------------
// Scenario: Given custom sections, when rendering, then only specified sections shown
// ---------------------------------------------------------------------------

#[test]
fn bdd_render_custom_sections_only() {
    Scenario::new("Rendering with custom sections shows only specified ones")
        .given("an aggregated team result", |ctx| {
            let result = sample_result();
            let json = serde_json::to_string(&result).expect("serialize result");
            ctx.data.insert("result_json".into(), json.into_bytes());
        })
        .when("rendering with only summary and coverage sections", |ctx| {
            let json = std::str::from_utf8(ctx.data.get("result_json").unwrap()).unwrap();
            let result: TeamAggregateResult =
                serde_json::from_str(json).expect("deserialize result");
            let cfg = TeamConfig {
                sections: vec!["summary".into(), "coverage".into()],
                ..TeamConfig::default()
            };
            let packet =
                render_packet_markdown(&cfg, &result).map_err(|e| format!("render: {e}"))?;
            ctx.strings.insert("packet".into(), packet);
            Ok(())
        })
        .then("the Summary section is present", |ctx| {
            let packet = assertions::assert_present(ctx.string("packet"), "packet")?;
            assertions::assert_contains(packet, "## Summary", "summary section")
        })
        .then("the Coverage section is present", |ctx| {
            let packet = assertions::assert_present(ctx.string("packet"), "packet")?;
            assertions::assert_contains(packet, "## Coverage", "coverage section")
        })
        .then("the Workstreams section is NOT present", |ctx| {
            let packet = assertions::assert_present(ctx.string("packet"), "packet")?;
            assertions::assert_not_contains(packet, "## Workstreams", "workstreams excluded")
        })
        .then("the Receipts section is NOT present", |ctx| {
            let packet = assertions::assert_present(ctx.string("packet"), "packet")?;
            assertions::assert_not_contains(packet, "## Receipts", "receipts excluded")
        })
        .run()
        .expect("scenario should pass");
}
