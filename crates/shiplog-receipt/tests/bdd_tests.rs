//! BDD tests for shiplog-receipt.

use chrono::{NaiveDate, TimeZone, Utc};
use shiplog_ids::EventId;
use shiplog_receipt::{format_receipt_markdown, manual_type_emoji};
use shiplog_schema::event::*;
use shiplog_testkit::bdd::assertions::{assert_contains, assert_eq, assert_present, assert_true};
use shiplog_testkit::bdd::{Scenario, ScenarioContext};

fn given_events(ctx: &mut ScenarioContext) {
    let pr = EventEnvelope {
        id: EventId::from_parts(["pr", "bdd", "1"]),
        kind: EventKind::PullRequest,
        occurred_at: Utc.with_ymd_and_hms(2025, 1, 5, 12, 0, 0).unwrap(),
        actor: Actor {
            login: "dev".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: "acme/service".into(),
            html_url: None,
            visibility: RepoVisibility::Private,
        },
        payload: EventPayload::PullRequest(PullRequestEvent {
            number: 10,
            title: "Improve packet receipts".into(),
            state: PullRequestState::Merged,
            created_at: Utc.with_ymd_and_hms(2025, 1, 5, 10, 0, 0).unwrap(),
            merged_at: Some(Utc.with_ymd_and_hms(2025, 1, 5, 11, 0, 0).unwrap()),
            additions: Some(20),
            deletions: Some(5),
            changed_files: Some(2),
            touched_paths_hint: vec![],
            window: None,
        }),
        tags: vec![],
        links: vec![Link {
            label: "pr".into(),
            url: "https://github.com/acme/service/pull/10".into(),
        }],
        source: SourceRef {
            system: SourceSystem::Github,
            url: None,
            opaque_id: None,
        },
    };

    let manual = EventEnvelope {
        id: EventId::from_parts(["manual", "bdd", "1"]),
        kind: EventKind::Manual,
        occurred_at: Utc.with_ymd_and_hms(2025, 1, 5, 15, 0, 0).unwrap(),
        actor: Actor {
            login: "dev".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: "acme/service".into(),
            html_url: None,
            visibility: RepoVisibility::Private,
        },
        payload: EventPayload::Manual(ManualEvent {
            event_type: ManualEventType::Incident,
            title: "Mitigate outage".into(),
            description: None,
            started_at: Some(NaiveDate::from_ymd_opt(2025, 1, 5).unwrap()),
            ended_at: Some(NaiveDate::from_ymd_opt(2025, 1, 5).unwrap()),
            impact: None,
        }),
        tags: vec![],
        links: vec![Link {
            label: "postmortem".into(),
            url: "https://example.com/postmortem".into(),
        }],
        source: SourceRef {
            system: SourceSystem::Manual,
            url: None,
            opaque_id: None,
        },
    };

    ctx.strings
        .insert("pr_line".to_string(), format_receipt_markdown(&pr));
    ctx.strings
        .insert("manual_line".to_string(), format_receipt_markdown(&manual));
    ctx.strings.insert(
        "incident_emoji".to_string(),
        manual_type_emoji(&ManualEventType::Incident).to_string(),
    );
}

fn then_receipt_contract_is_stable(ctx: &ScenarioContext) -> Result<(), String> {
    let pr_line = assert_present(ctx.string("pr_line"), "pr_line")?;
    let manual_line = assert_present(ctx.string("manual_line"), "manual_line")?;
    let incident_emoji = assert_present(ctx.string("incident_emoji"), "incident_emoji")?;

    assert_true(pr_line.starts_with("- [PR]"), "pr prefix")?;
    assert_contains(pr_line, "2025-01-05", "pr date")?;
    assert_contains(pr_line, "acme/service", "pr repo")?;
    assert_contains(
        pr_line,
        "https://github.com/acme/service/pull/10",
        "pr link",
    )?;

    assert_true(manual_line.starts_with("- [🚨]"), "manual emoji prefix")?;
    assert_contains(manual_line, "Mitigate outage", "manual title")?;
    assert_contains(
        manual_line,
        "[postmortem](https://example.com/postmortem)",
        "manual link",
    )?;

    assert_eq(incident_emoji, "🚨", "incident emoji constant")
}

#[test]
fn bdd_receipt_markdown_contract_is_stable() {
    let scenario = Scenario::new("Receipt formatter keeps canonical markdown contract")
        .given("canonical PR and manual events", given_events)
        .then(
            "formatted receipt lines preserve prefix/date/link conventions",
            then_receipt_contract_is_stable,
        );

    scenario.run().expect("BDD scenario should pass");
}
