//! Integration tests for shiplog-receipt.

use chrono::{NaiveDate, TimeZone, Utc};
use shiplog_ids::EventId;
use shiplog_receipt::format_receipt_markdown;
use shiplog_schema::event::*;
use shiplog_testkit::pr_event;

#[test]
fn formats_testkit_pr_fixture_without_losing_repo_or_url() {
    let event = pr_event("acme/service", 42, "Refactor receipt rendering");
    let line = format_receipt_markdown(&event);

    assert!(line.starts_with("- [PR] Refactor receipt rendering"));
    assert!(line.contains("acme/service"));
    assert!(line.contains("https://github.com/acme/service/pull/42"));
}

#[test]
fn formats_manual_fixture_with_links_and_emoji() {
    let event = EventEnvelope {
        id: EventId::from_parts(["manual", "integration", "1"]),
        kind: EventKind::Manual,
        occurred_at: Utc.with_ymd_and_hms(2025, 2, 1, 12, 0, 0).unwrap(),
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
            event_type: ManualEventType::Launch,
            title: "Ship v0.2.1".into(),
            description: None,
            started_at: Some(NaiveDate::from_ymd_opt(2025, 2, 1).unwrap()),
            ended_at: Some(NaiveDate::from_ymd_opt(2025, 2, 1).unwrap()),
            impact: None,
        }),
        tags: vec![],
        links: vec![Link {
            label: "release".into(),
            url: "https://example.com/releases/v0.2.1".into(),
        }],
        source: SourceRef {
            system: SourceSystem::Manual,
            url: None,
            opaque_id: None,
        },
    };

    let line = format_receipt_markdown(&event);
    assert_eq!(
        line,
        "- [🚀] Ship v0.2.1 (2025-02-01) — [release](https://example.com/releases/v0.2.1)"
    );
}
