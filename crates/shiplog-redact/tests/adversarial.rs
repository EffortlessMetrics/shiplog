//! Adversarial and negative-input tests for shiplog-redact.

use chrono::Utc;
use shiplog_ids::EventId;
use shiplog_ports::Redactor;
use shiplog_redact::{DeterministicRedactor, RedactionProfile};
use shiplog_schema::event::*;

fn make_event(title: &str, repo: &str) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["test", title]),
        kind: EventKind::PullRequest,
        occurred_at: Utc::now(),
        actor: Actor {
            login: "testuser".into(),
            id: None,
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
            deletions: Some(5),
            changed_files: Some(3),
            touched_paths_hint: vec!["src/secret.rs".into()],
            window: None,
        }),
        tags: vec!["sensitive-tag".into()],
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

#[test]
fn input_containing_redaction_key_does_not_leak_title() {
    let key = "my-secret-redaction-key";
    let r = DeterministicRedactor::new(key.as_bytes());
    let ev = make_event(&format!("Fix bug in {key} module"), "org/repo");
    let out = r.redact_events(&[ev], "public").unwrap();
    if let EventPayload::PullRequest(pr) = &out[0].payload {
        assert_eq!(pr.title, "[redacted]");
    } else {
        panic!("expected PR payload");
    }
}

#[test]
fn empty_key_produces_deterministic_output() {
    let r1 = DeterministicRedactor::new(b"");
    let r2 = DeterministicRedactor::new(b"");
    let ev = make_event("test title", "org/repo");
    let out1 = r1.redact_events(&[ev.clone()], "public").unwrap();
    let out2 = r2.redact_events(&[ev], "public").unwrap();
    let json1 = serde_json::to_string(&out1).unwrap();
    let json2 = serde_json::to_string(&out2).unwrap();
    assert_eq!(json1, json2, "empty key should still be deterministic");
}

#[test]
fn very_long_key_does_not_panic() {
    let long_key = vec![0xABu8; 100_000];
    let r = DeterministicRedactor::new(&long_key);
    let ev = make_event("test", "org/repo");
    let result = r.redact_events(&[ev], "public");
    assert!(result.is_ok());
}

#[test]
fn double_redaction_re_aliases_repo_names() {
    let r = DeterministicRedactor::new(b"key");
    let ev = make_event("original title", "org/repo");
    let first = r.redact_events(&[ev], "public").unwrap();
    let second = r.redact_events(&first, "public").unwrap();
    assert_ne!(
        first[0].repo.full_name, second[0].repo.full_name,
        "double-redaction re-aliases repo names"
    );
    if let EventPayload::PullRequest(pr) = &second[0].payload {
        assert_eq!(pr.title, "[redacted]");
    }
}

#[test]
fn unicode_key_and_data() {
    let r = DeterministicRedactor::new("\u{65E5}\u{672C}\u{8A9E}\u{1F510}".as_bytes());
    let ev = make_event(
        "\u{30BF}\u{30A4}\u{30C8}\u{30EB}",
        "\u{7D44}\u{7E54}/\u{30EA}\u{30DD}",
    );
    let result = r.redact_events(&[ev], "public");
    assert!(result.is_ok());
}

#[test]
fn unknown_profile_defaults_to_public() {
    let p = RedactionProfile::from_profile_str("nonexistent");
    assert_eq!(p, RedactionProfile::Public);
}

#[test]
fn empty_profile_string_defaults_to_public() {
    let p = RedactionProfile::from_profile_str("");
    assert_eq!(p, RedactionProfile::Public);
}

#[test]
fn internal_profile_preserves_titles() {
    let r = DeterministicRedactor::new(b"key");
    let ev = make_event("secret title", "org/repo");
    let out = r.redact_events(&[ev], "internal").unwrap();
    if let EventPayload::PullRequest(pr) = &out[0].payload {
        assert_eq!(pr.title, "secret title");
    }
}

#[test]
fn redact_empty_events() {
    let r = DeterministicRedactor::new(b"key");
    let out = r.redact_events(&[], "public").unwrap();
    assert!(out.is_empty());
}

#[test]
fn different_keys_produce_different_aliases() {
    let r1 = DeterministicRedactor::new(b"key-alpha");
    let r2 = DeterministicRedactor::new(b"key-beta");
    let ev = make_event("same title", "same/repo");
    let out1 = r1.redact_events(&[ev.clone()], "public").unwrap();
    let out2 = r2.redact_events(&[ev], "public").unwrap();
    assert_ne!(out1[0].repo.full_name, out2[0].repo.full_name);
}

#[test]
fn manager_profile_keeps_title_and_repo() {
    let r = DeterministicRedactor::new(b"key");
    let ev = make_event("visible title", "org/repo");
    let out = r.redact_events(&[ev], "manager").unwrap();
    if let EventPayload::PullRequest(pr) = &out[0].payload {
        assert_eq!(pr.title, "visible title");
    }
    assert_eq!(out[0].repo.full_name, "org/repo");
}

#[test]
fn review_event_title_stripped_in_public() {
    let r = DeterministicRedactor::new(b"key");
    let ev = EventEnvelope {
        id: EventId::from_parts(["review", "1"]),
        kind: EventKind::Review,
        occurred_at: Utc::now(),
        actor: Actor {
            login: "reviewer".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: "org/repo".into(),
            html_url: None,
            visibility: RepoVisibility::Private,
        },
        payload: EventPayload::Review(ReviewEvent {
            pull_number: 42,
            pull_title: "secret review title".into(),
            submitted_at: Utc::now(),
            state: "approved".into(),
            window: None,
        }),
        tags: vec![],
        links: vec![],
        source: SourceRef {
            system: SourceSystem::Github,
            url: None,
            opaque_id: None,
        },
    };
    let out = r.redact_events(&[ev], "public").unwrap();
    let json = serde_json::to_string(&out).unwrap();
    assert!(!json.contains("secret review title"));
}
