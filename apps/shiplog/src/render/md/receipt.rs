//! Markdown receipt formatter for canonical shiplog events.
//!
//! This crate has a single responsibility:
//! - converting one `EventEnvelope` into one display-ready receipt line

use shiplog_schema::event::{EventEnvelope, EventKind, EventPayload, ManualEventType};

/// Format one event into a markdown receipt line.
#[must_use]
pub fn format_receipt_markdown(event: &EventEnvelope) -> String {
    match (&event.kind, &event.payload) {
        (EventKind::PullRequest, EventPayload::PullRequest(pr)) => {
            let repo = &event.repo.full_name;
            let url = event
                .links
                .iter()
                .find(|link| link.label == "pr")
                .map(|link| link.url.as_str())
                .unwrap_or("");
            let date = event.occurred_at.format("%Y-%m-%d");

            if url.is_empty() {
                format!("- [PR] {} ({date}) — {repo}", pr.title)
            } else {
                format!("- [PR] {} ({date}) — [{repo}]({url})", pr.title)
            }
        }
        (EventKind::Review, EventPayload::Review(review)) => {
            let repo = &event.repo.full_name;
            let url = event
                .links
                .iter()
                .find(|link| link.label == "pr")
                .map(|link| link.url.as_str())
                .unwrap_or("");
            let date = event.occurred_at.format("%Y-%m-%d");

            if url.is_empty() {
                format!("- [Review] {} ({date}) — {repo}", review.state)
            } else {
                format!("- [Review] {} ({date}) — [{repo}]({url})", review.state)
            }
        }
        (EventKind::Manual, EventPayload::Manual(manual)) => {
            let emoji = manual_type_emoji(&manual.event_type);
            let links: Vec<String> = event
                .links
                .iter()
                .map(|link| format!("[{}]({})", link.label, link.url))
                .collect();
            let links_suffix = if links.is_empty() {
                String::new()
            } else {
                format!(" — {}", links.join(", "))
            };
            let date = event.occurred_at.format("%Y-%m-%d");
            format!("- [{emoji}] {} ({date}){links_suffix}", manual.title)
        }
        _ => format!("- event {}", event.id),
    }
}

/// Emoji marker for manual event kinds.
#[must_use]
pub fn manual_type_emoji(event_type: &ManualEventType) -> &'static str {
    match event_type {
        ManualEventType::Note => "📝",
        ManualEventType::Incident => "🚨",
        ManualEventType::Design => "🏗️",
        ManualEventType::Mentoring => "👨‍🏫",
        ManualEventType::Launch => "🚀",
        ManualEventType::Migration => "🔄",
        ManualEventType::Review => "👀",
        ManualEventType::Other => "📌",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, TimeZone, Utc};
    use shiplog_ids::EventId;
    use shiplog_schema::event::*;

    fn sample_pr_event() -> EventEnvelope {
        EventEnvelope {
            id: EventId::from_parts(["pr", "1"]),
            kind: EventKind::PullRequest,
            occurred_at: Utc.with_ymd_and_hms(2025, 1, 2, 8, 0, 0).unwrap(),
            actor: Actor {
                login: "octocat".into(),
                id: None,
            },
            repo: RepoRef {
                full_name: "acme/service".into(),
                html_url: None,
                visibility: RepoVisibility::Private,
            },
            payload: EventPayload::PullRequest(PullRequestEvent {
                number: 42,
                title: "Ship receipts".into(),
                state: PullRequestState::Merged,
                created_at: Utc.with_ymd_and_hms(2025, 1, 2, 7, 0, 0).unwrap(),
                merged_at: Some(Utc.with_ymd_and_hms(2025, 1, 2, 8, 0, 0).unwrap()),
                additions: Some(10),
                deletions: Some(2),
                changed_files: Some(3),
                touched_paths_hint: vec![],
                window: None,
            }),
            tags: vec![],
            links: vec![Link {
                label: "pr".into(),
                url: "https://github.com/acme/service/pull/42".into(),
            }],
            source: SourceRef {
                system: SourceSystem::Github,
                url: None,
                opaque_id: None,
            },
        }
    }

    #[test]
    fn formats_pull_request_with_link() {
        let line = format_receipt_markdown(&sample_pr_event());
        assert!(line.starts_with("- [PR]"));
        assert!(line.contains("Ship receipts"));
        assert!(line.contains("2025-01-02"));
        assert!(line.contains("[acme/service](https://github.com/acme/service/pull/42)"));
    }

    #[test]
    fn formats_review_without_link() {
        let mut event = sample_pr_event();
        event.kind = EventKind::Review;
        event.payload = EventPayload::Review(ReviewEvent {
            pull_number: 42,
            pull_title: "Ship receipts".into(),
            submitted_at: Utc.with_ymd_and_hms(2025, 1, 2, 9, 0, 0).unwrap(),
            state: "approved".into(),
            window: None,
        });
        event.links.clear();

        let line = format_receipt_markdown(&event);
        assert_eq!(line, "- [Review] approved (2025-01-02) — acme/service");
    }

    #[test]
    fn formats_manual_event_with_type_emoji() {
        let mut event = sample_pr_event();
        event.id = EventId::from_parts(["manual", "1"]);
        event.kind = EventKind::Manual;
        event.payload = EventPayload::Manual(ManualEvent {
            event_type: ManualEventType::Incident,
            title: "Resolve outage".into(),
            description: None,
            started_at: Some(NaiveDate::from_ymd_opt(2025, 1, 2).unwrap()),
            ended_at: Some(NaiveDate::from_ymd_opt(2025, 1, 2).unwrap()),
            impact: None,
        });
        event.links = vec![Link {
            label: "runbook".into(),
            url: "https://internal/runbook/42".into(),
        }];

        let line = format_receipt_markdown(&event);
        assert_eq!(
            line,
            "- [🚨] Resolve outage (2025-01-02) — [runbook](https://internal/runbook/42)"
        );
    }

    #[test]
    fn manual_type_emoji_stays_stable() {
        assert_eq!(manual_type_emoji(&ManualEventType::Note), "📝");
        assert_eq!(manual_type_emoji(&ManualEventType::Incident), "🚨");
        assert_eq!(manual_type_emoji(&ManualEventType::Design), "🏗️");
        assert_eq!(manual_type_emoji(&ManualEventType::Mentoring), "👨‍🏫");
        assert_eq!(manual_type_emoji(&ManualEventType::Launch), "🚀");
        assert_eq!(manual_type_emoji(&ManualEventType::Migration), "🔄");
        assert_eq!(manual_type_emoji(&ManualEventType::Review), "👀");
        assert_eq!(manual_type_emoji(&ManualEventType::Other), "📌");
    }

    #[test]
    fn mismatched_kind_and_payload_uses_fallback() {
        let mut event = sample_pr_event();
        event.kind = EventKind::Manual;
        let line = format_receipt_markdown(&event);
        assert_eq!(line, format!("- event {}", event.id));
    }
}
