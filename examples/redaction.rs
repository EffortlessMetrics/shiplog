//! Demonstrate the redaction pipeline: internal → manager → public profiles.
//!
//! Shiplog uses deterministic HMAC-SHA256 redaction so the same key always
//! produces the same aliases, making redacted packets diffable across runs.
//!
//! Run from the workspace root:
//!   cargo run -p shiplog --example redaction

use anyhow::{Context, Result};
use shiplog_ports::Redactor;
use shiplog_redact::DeterministicRedactor;
use shiplog_schema::event::{EventEnvelope, EventPayload};

fn main() -> Result<()> {
    // Load fixture events (2 PRs + 1 review across two repos).
    let events = load_fixture_events()?;

    // Create a deterministic redactor with a stable key.
    // The same key always produces the same aliases.
    let redactor = DeterministicRedactor::new(b"example-redaction-key");

    // --- Internal profile (original, unredacted) ---
    println!("=== Internal Profile (original) ===\n");
    print_events(&events);

    // --- Manager profile ---
    // Preserves structure but aliases private repository names and URLs.
    println!("=== Manager Profile ===\n");
    let manager_events = redactor
        .redact_events(&events, "manager")
        .context("redact for manager profile")?;
    print_events(&manager_events);

    // --- Public profile ---
    // Strips titles, links, and other sensitive details.
    println!("=== Public Profile ===\n");
    let public_events = redactor
        .redact_events(&events, "public")
        .context("redact for public profile")?;
    print_events(&public_events);

    // --- Determinism check ---
    // A second redactor with the same key must produce identical aliases.
    println!("=== Determinism Check ===\n");
    let redactor2 = DeterministicRedactor::new(b"example-redaction-key");
    let manager2 = redactor2
        .redact_events(&events, "manager")
        .context("second redaction pass")?;

    for (a, b) in manager_events.iter().zip(manager2.iter()) {
        assert_eq!(
            a.repo.full_name, b.repo.full_name,
            "aliases must be stable across redactor instances"
        );
        println!("  ✓ stable alias: {}", a.repo.full_name);
    }

    // --- Different key produces different aliases ---
    println!("\n=== Different Key ===\n");
    let other_redactor = DeterministicRedactor::new(b"a-totally-different-key");
    let other_manager = other_redactor
        .redact_events(&events, "manager")
        .context("redact with different key")?;

    for (same_key, diff_key) in manager_events.iter().zip(other_manager.iter()) {
        println!(
            "  key A → {}  |  key B → {}",
            same_key.repo.full_name, diff_key.repo.full_name
        );
    }

    Ok(())
}

/// Pretty-print a slice of events showing the fields most affected by redaction.
fn print_events(events: &[EventEnvelope]) {
    for ev in events {
        let title = match &ev.payload {
            EventPayload::PullRequest(pr) => pr.title.clone(),
            EventPayload::Review(r) => r.pull_title.clone(),
            EventPayload::Manual(m) => m.title.clone(),
        };
        println!("  Repo:  {}", ev.repo.full_name);
        println!("  Title: {title}");
        println!("  Actor: {}", ev.actor.login);
        if let Some(link) = ev.links.first() {
            println!("  Link:  {}", link.url);
        } else {
            println!("  Link:  (none)");
        }
        println!();
    }
}

fn load_fixture_events() -> Result<Vec<EventEnvelope>> {
    let text = std::fs::read_to_string("examples/fixture/ledger.events.jsonl")
        .context("read fixture events")?;
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).context("parse event line"))
        .collect()
}
