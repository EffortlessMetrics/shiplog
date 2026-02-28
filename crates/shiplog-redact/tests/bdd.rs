//! BDD-style integration tests for shiplog-redact.
//!
//! Each test follows the Given / When / Then pattern using the
//! `shiplog_testkit::bdd::Scenario` runner.

use shiplog_testkit::bdd::Scenario;

use chrono::Utc;
use shiplog_ids::{EventId, WorkstreamId};
use shiplog_ports::Redactor;
use shiplog_redact::DeterministicRedactor;
use shiplog_schema::event::*;
use shiplog_schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};

// ---------------------------------------------------------------------------
// Helpers – build reusable fixtures without cluttering scenarios
// ---------------------------------------------------------------------------

fn sample_pr_event(repo: &str, title: &str, number: u64) -> EventEnvelope {
    let now = Utc::now();
    EventEnvelope {
        id: EventId::from_parts(["bdd", "pr", repo, &number.to_string()]),
        kind: EventKind::PullRequest,
        occurred_at: now,
        actor: Actor {
            login: "dev-user".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: repo.into(),
            html_url: Some(format!("https://github.com/{repo}")),
            visibility: RepoVisibility::Private,
        },
        payload: EventPayload::PullRequest(PullRequestEvent {
            number,
            title: title.into(),
            state: PullRequestState::Merged,
            created_at: now,
            merged_at: Some(now),
            additions: Some(42),
            deletions: Some(7),
            changed_files: Some(3),
            touched_paths_hint: vec!["src/secret.rs".into()],
            window: None,
        }),
        tags: vec![],
        links: vec![Link {
            label: "pr".into(),
            url: format!("https://github.com/{repo}/pull/{number}"),
        }],
        source: SourceRef {
            system: SourceSystem::Github,
            url: Some(format!(
                "https://api.github.com/repos/{repo}/pulls/{number}"
            )),
            opaque_id: None,
        },
    }
}

fn sample_workstreams_file() -> WorkstreamsFile {
    WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![Workstream {
            id: WorkstreamId::from_parts(["bdd", "ws", "1"]),
            title: "Secret Auth Overhaul".into(),
            summary: Some("Re-architect the internal auth layer".into()),
            tags: vec!["security".into(), "backend".into(), "repo".into()],
            stats: WorkstreamStats::zero(),
            events: vec![],
            receipts: vec![],
        }],
    }
}

// ===========================================================================
// Scenario 1: Internal profile preserves all data
// ===========================================================================

#[test]
fn scenario_internal_profile_preserves_all_data() {
    Scenario::new("Internal profile preserves all data")
        .given("an event with sensitive fields", |ctx| {
            let ev = sample_pr_event("acme/secret-repo", "Confidential Feature", 1);
            let json = serde_json::to_string(&[&ev]).unwrap();
            ctx.data.insert("events".into(), json.into_bytes());
            ctx.strings.insert("key".into(), "test-key".into());
        })
        .when("the event is redacted with the internal profile", |ctx| {
            let key = ctx.string("key").unwrap();
            let redactor = DeterministicRedactor::new(key.as_bytes());
            let events: Vec<EventEnvelope> = serde_json::from_slice(&ctx.data["events"]).unwrap();
            let redacted = redactor
                .redact_events(&events, "internal")
                .map_err(|e| e.to_string())?;
            let json = serde_json::to_string(&redacted).map_err(|e| e.to_string())?;
            ctx.data.insert("redacted".into(), json.into_bytes());
            Ok(())
        })
        .then("all original fields are preserved", |ctx| {
            let json = std::str::from_utf8(&ctx.data["redacted"]).unwrap();
            if !json.contains("Confidential Feature") {
                return Err("title was redacted but should be preserved".into());
            }
            if !json.contains("acme/secret-repo") {
                return Err("repo name was redacted but should be preserved".into());
            }
            if !json.contains("src/secret.rs") {
                return Err("touched paths were stripped but should be preserved".into());
            }
            if !json.contains("https://github.com/acme/secret-repo/pull/1") {
                return Err("link URL was stripped but should be preserved".into());
            }
            Ok(())
        })
        .run()
        .expect("scenario should pass");
}

// ===========================================================================
// Scenario 2: Manager profile redacts specific fields
// ===========================================================================

#[test]
fn scenario_manager_profile_redacts_details_but_keeps_context() {
    Scenario::new("Manager profile redacts details but keeps context")
        .given("a PR event with paths, links, and a title", |ctx| {
            let ev = sample_pr_event("org/repo", "Add OAuth2 support", 10);
            let json = serde_json::to_string(&[&ev]).unwrap();
            ctx.data.insert("events".into(), json.into_bytes());
            ctx.strings.insert("key".into(), "mgr-key".into());
        })
        .when("the event is redacted with the manager profile", |ctx| {
            let key = ctx.string("key").unwrap();
            let redactor = DeterministicRedactor::new(key.as_bytes());
            let events: Vec<EventEnvelope> = serde_json::from_slice(&ctx.data["events"]).unwrap();
            let redacted = redactor
                .redact_events(&events, "manager")
                .map_err(|e| e.to_string())?;
            let json = serde_json::to_string(&redacted).map_err(|e| e.to_string())?;
            ctx.data.insert("redacted_json".into(), json.into_bytes());
            let serialized = serde_json::to_vec(&redacted).map_err(|e| e.to_string())?;
            ctx.data.insert("redacted_events".into(), serialized);
            Ok(())
        })
        .then("the title is preserved", |ctx| {
            let json = std::str::from_utf8(&ctx.data["redacted_json"]).unwrap();
            if !json.contains("Add OAuth2 support") {
                return Err("manager profile should preserve PR title".into());
            }
            Ok(())
        })
        .then("touched paths are cleared", |ctx| {
            let events: Vec<EventEnvelope> =
                serde_json::from_slice(&ctx.data["redacted_events"]).unwrap();
            match &events[0].payload {
                EventPayload::PullRequest(pr) => {
                    if !pr.touched_paths_hint.is_empty() {
                        return Err("manager profile should clear touched_paths_hint".into());
                    }
                }
                _ => return Err("expected PR payload".into()),
            }
            Ok(())
        })
        .then("links are stripped", |ctx| {
            let events: Vec<EventEnvelope> =
                serde_json::from_slice(&ctx.data["redacted_events"]).unwrap();
            if !events[0].links.is_empty() {
                return Err("manager profile should strip links".into());
            }
            Ok(())
        })
        .run()
        .expect("scenario should pass");
}

// ===========================================================================
// Scenario 3: Public profile strips titles and links
// ===========================================================================

#[test]
fn scenario_public_profile_strips_titles_and_links() {
    Scenario::new("Public profile strips titles, links, and aliases repos")
        .given("a PR event with sensitive content", |ctx| {
            let ev = sample_pr_event("acme/top-secret", "Internal Auth Bypass Fix", 99);
            let json = serde_json::to_string(&[&ev]).unwrap();
            ctx.data.insert("events".into(), json.into_bytes());
            ctx.strings.insert("key".into(), "pub-key".into());
        })
        .when("the event is redacted with the public profile", |ctx| {
            let key = ctx.string("key").unwrap();
            let redactor = DeterministicRedactor::new(key.as_bytes());
            let events: Vec<EventEnvelope> = serde_json::from_slice(&ctx.data["events"]).unwrap();
            let redacted = redactor
                .redact_events(&events, "public")
                .map_err(|e| e.to_string())?;
            let serialized = serde_json::to_vec(&redacted).map_err(|e| e.to_string())?;
            ctx.data.insert("redacted_events".into(), serialized);
            let json = serde_json::to_string(&redacted).map_err(|e| e.to_string())?;
            ctx.data.insert("redacted_json".into(), json.into_bytes());
            Ok(())
        })
        .then("the PR title is replaced with [redacted]", |ctx| {
            let events: Vec<EventEnvelope> =
                serde_json::from_slice(&ctx.data["redacted_events"]).unwrap();
            match &events[0].payload {
                EventPayload::PullRequest(pr) => {
                    if pr.title != "[redacted]" {
                        return Err(format!("expected title '[redacted]', got '{}'", pr.title));
                    }
                }
                _ => return Err("expected PR payload".into()),
            }
            Ok(())
        })
        .then("all links are removed", |ctx| {
            let events: Vec<EventEnvelope> =
                serde_json::from_slice(&ctx.data["redacted_events"]).unwrap();
            if !events[0].links.is_empty() {
                return Err("public profile should remove all links".into());
            }
            Ok(())
        })
        .then("the repo name is aliased", |ctx| {
            let json = std::str::from_utf8(&ctx.data["redacted_json"]).unwrap();
            if json.contains("acme/top-secret") {
                return Err("public profile should alias repo name".into());
            }
            Ok(())
        })
        .then("source URL is stripped", |ctx| {
            let events: Vec<EventEnvelope> =
                serde_json::from_slice(&ctx.data["redacted_events"]).unwrap();
            if events[0].source.url.is_some() {
                return Err("public profile should strip source URL".into());
            }
            Ok(())
        })
        .then("touched paths are cleared", |ctx| {
            let events: Vec<EventEnvelope> =
                serde_json::from_slice(&ctx.data["redacted_events"]).unwrap();
            match &events[0].payload {
                EventPayload::PullRequest(pr) => {
                    if !pr.touched_paths_hint.is_empty() {
                        return Err("public profile should clear touched paths".into());
                    }
                }
                _ => return Err("expected PR payload".into()),
            }
            Ok(())
        })
        .run()
        .expect("scenario should pass");
}

// ===========================================================================
// Scenario 4: Deterministic redaction produces same hash for same input
// ===========================================================================

#[test]
fn scenario_deterministic_redaction_same_key_same_output() {
    Scenario::new("Deterministic redaction produces same hash for same input")
        .given("two redactors initialised with the same key", |ctx| {
            ctx.strings.insert("key".into(), "stable-key".into());
            let ev = sample_pr_event("org/repo", "Feature X", 5);
            let json = serde_json::to_string(&[&ev]).unwrap();
            ctx.data.insert("events".into(), json.into_bytes());
        })
        .when(
            "both redact the same event with the public profile",
            |ctx| {
                let key = ctx.string("key").unwrap();
                let events: Vec<EventEnvelope> =
                    serde_json::from_slice(&ctx.data["events"]).unwrap();

                let r1 = DeterministicRedactor::new(key.as_bytes());
                let r2 = DeterministicRedactor::new(key.as_bytes());

                let out1 = r1
                    .redact_events(&events, "public")
                    .map_err(|e| e.to_string())?;
                let out2 = r2
                    .redact_events(&events, "public")
                    .map_err(|e| e.to_string())?;

                let json1 = serde_json::to_string(&out1).map_err(|e| e.to_string())?;
                let json2 = serde_json::to_string(&out2).map_err(|e| e.to_string())?;
                ctx.data.insert("json1".into(), json1.into_bytes());
                ctx.data.insert("json2".into(), json2.into_bytes());
                Ok(())
            },
        )
        .then("both outputs are byte-identical", |ctx| {
            if ctx.data["json1"] != ctx.data["json2"] {
                return Err("redaction output should be identical for same key".into());
            }
            Ok(())
        })
        .run()
        .expect("scenario should pass");
}

// ===========================================================================
// Scenario 5: Different keys produce different hashes
// ===========================================================================

#[test]
fn scenario_different_keys_produce_different_aliases() {
    Scenario::new("Different keys produce different hashes")
        .given("two redactors with different keys", |ctx| {
            ctx.strings.insert("key_a".into(), "key-alpha".into());
            ctx.strings.insert("key_b".into(), "key-bravo".into());
            let ev = sample_pr_event("org/repo", "Same PR", 1);
            let json = serde_json::to_string(&[&ev]).unwrap();
            ctx.data.insert("events".into(), json.into_bytes());
        })
        .when(
            "each redactor processes the same event with the public profile",
            |ctx| {
                let events: Vec<EventEnvelope> =
                    serde_json::from_slice(&ctx.data["events"]).unwrap();

                let r_a = DeterministicRedactor::new(ctx.string("key_a").unwrap().as_bytes());
                let r_b = DeterministicRedactor::new(ctx.string("key_b").unwrap().as_bytes());

                let out_a = r_a
                    .redact_events(&events, "public")
                    .map_err(|e| e.to_string())?;
                let out_b = r_b
                    .redact_events(&events, "public")
                    .map_err(|e| e.to_string())?;

                // Store aliased repo names for comparison
                ctx.strings
                    .insert("repo_a".into(), out_a[0].repo.full_name.clone());
                ctx.strings
                    .insert("repo_b".into(), out_b[0].repo.full_name.clone());
                Ok(())
            },
        )
        .then("the aliased repo names differ", |ctx| {
            let repo_a = ctx.string("repo_a").unwrap();
            let repo_b = ctx.string("repo_b").unwrap();
            if repo_a == repo_b {
                return Err(format!(
                    "different keys should produce different aliases, both got '{repo_a}'"
                ));
            }
            Ok(())
        })
        .run()
        .expect("scenario should pass");
}

// ===========================================================================
// Scenario 6: Alias caching returns consistent results
// ===========================================================================

#[test]
fn scenario_alias_cache_round_trip_consistency() {
    Scenario::new("Alias caching returns consistent results across save/load")
        .given(
            "a redactor that has generated aliases and saved its cache",
            |ctx| {
                let dir = tempfile::tempdir().unwrap();
                let cache_path = dir.path().join("redaction.aliases.json");

                let redactor = DeterministicRedactor::new(b"cache-key");
                let events = vec![sample_pr_event("cached/repo", "Cached PR", 1)];
                let redacted = redactor.redact_events(&events, "public").unwrap();
                redactor.save_cache(&cache_path).unwrap();

                let json = serde_json::to_string(&redacted).unwrap();
                ctx.data.insert("first_output".into(), json.into_bytes());
                ctx.strings.insert(
                    "cache_path".into(),
                    cache_path.to_string_lossy().into_owned(),
                );
                // Keep tempdir alive by leaking into data (path string is enough)
                let events_json = serde_json::to_string(&events).unwrap();
                ctx.data.insert("events".into(), events_json.into_bytes());
                // Store dir handle bytes to keep it alive
                let dir_path = dir.keep();
                ctx.strings
                    .insert("tmp_dir".into(), dir_path.to_string_lossy().into_owned());
            },
        )
        .when(
            "a new redactor loads the cache and re-redacts the same events",
            |ctx| {
                let cache_path_str = ctx.string("cache_path").unwrap().to_string();
                let cache_path = std::path::Path::new(&cache_path_str);
                let events: Vec<EventEnvelope> =
                    serde_json::from_slice(&ctx.data["events"]).unwrap();

                let redactor = DeterministicRedactor::new(b"cache-key");
                redactor.load_cache(cache_path).map_err(|e| e.to_string())?;
                let redacted = redactor
                    .redact_events(&events, "public")
                    .map_err(|e| e.to_string())?;

                let json = serde_json::to_string(&redacted).map_err(|e| e.to_string())?;
                ctx.data.insert("second_output".into(), json.into_bytes());
                Ok(())
            },
        )
        .then("both outputs are identical", |ctx| {
            if ctx.data["first_output"] != ctx.data["second_output"] {
                return Err("alias cache should produce identical redaction output".into());
            }
            Ok(())
        })
        .run()
        .expect("scenario should pass");

    // Cleanup: tempdir is already dropped via into_path; OS will clean up
}

// ===========================================================================
// Scenario 7 (bonus): Public workstream redaction
// ===========================================================================

#[test]
fn scenario_public_profile_redacts_workstreams() {
    Scenario::new("Public profile aliases workstream titles, strips summaries, removes repo tag")
        .given("a workstreams file with sensitive content", |ctx| {
            let ws = sample_workstreams_file();
            let json = serde_json::to_string(&ws).unwrap();
            ctx.data.insert("workstreams".into(), json.into_bytes());
            ctx.strings.insert("key".into(), "ws-key".into());
        })
        .when(
            "the workstreams are redacted with the public profile",
            |ctx| {
                let key = ctx.string("key").unwrap();
                let ws: WorkstreamsFile = serde_json::from_slice(&ctx.data["workstreams"]).unwrap();
                let redactor = DeterministicRedactor::new(key.as_bytes());
                let redacted = redactor
                    .redact_workstreams(&ws, "public")
                    .map_err(|e| e.to_string())?;
                let json = serde_json::to_string(&redacted).map_err(|e| e.to_string())?;
                ctx.data
                    .insert("redacted_ws_json".into(), json.into_bytes());
                let serialized = serde_json::to_vec(&redacted).map_err(|e| e.to_string())?;
                ctx.data.insert("redacted_ws".into(), serialized);
                Ok(())
            },
        )
        .then("the original title is replaced with an alias", |ctx| {
            let json = std::str::from_utf8(&ctx.data["redacted_ws_json"]).unwrap();
            if json.contains("Secret Auth Overhaul") {
                return Err("public profile should alias workstream title".into());
            }
            Ok(())
        })
        .then("the summary is removed", |ctx| {
            let ws: WorkstreamsFile = serde_json::from_slice(&ctx.data["redacted_ws"]).unwrap();
            if ws.workstreams[0].summary.is_some() {
                return Err("public profile should strip workstream summary".into());
            }
            Ok(())
        })
        .then("the 'repo' tag is filtered out", |ctx| {
            let ws: WorkstreamsFile = serde_json::from_slice(&ctx.data["redacted_ws"]).unwrap();
            if ws.workstreams[0].tags.contains(&"repo".into()) {
                return Err("public profile should filter out 'repo' tag".into());
            }
            Ok(())
        })
        .then("other tags are preserved", |ctx| {
            let ws: WorkstreamsFile = serde_json::from_slice(&ctx.data["redacted_ws"]).unwrap();
            let tags = &ws.workstreams[0].tags;
            if !tags.contains(&"security".into()) || !tags.contains(&"backend".into()) {
                return Err("non-repo tags should be preserved".into());
            }
            Ok(())
        })
        .run()
        .expect("scenario should pass");
}
