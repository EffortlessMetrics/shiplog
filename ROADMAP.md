# Shiplog Roadmap

This document outlines the planned evolution of shiplog from v0.1.0 toward a stable v1.0 release.

## v0.1.0 (Current) - Foundation

**Status:** In PR review

**Theme:** "It works on my machine" → trustworthy artifact generation

**Deliverables:**
- [x] SQLite cache for GitHub API responses
- [x] Deterministic redaction (internal/manager/public profiles)
- [x] Workstream curation workflow
- [x] Coverage manifest with slicing metadata
- [x] Markdown packet generation

**Known Limitations:**
- Manual events require hand-editing YAML
- No incremental updates (full re-fetch on refresh)
- Bundle format is basic zip

## v0.2.0 - The Feedback Loop

**Theme:** Make it usable for real self-reviews

**Deliverables:**
- [ ] Interactive TUI for manual event entry (`shiplog add`)
- [ ] Diff view between runs (what changed since last packet?)
- [ ] Packet validation (check for completeness before submission)
- [ ] Better coverage analysis (identify gaps in GitHub data)
- [ ] Export to PDF (via pandoc or native)
- [ ] Plugin system for custom ingesters

**CLI Additions:**
```bash
shiplog add                    # Interactive manual event creation
shiplog diff <run1> <run2>     # Compare two runs
shiplog validate <run>         # Check packet completeness
shiplog export --format pdf    # Generate PDF output
```

## v0.3.0 - Collaboration

**Theme:** Share packets safely

**Deliverables:**
- [ ] Signed bundles (cryptographic verification)
- [ ] Packet viewer web app (read-only, privacy-safe)
- [ ] Manager packet aggregation (combine multiple team member packets)
- [ ] Comment/review workflow on packets
- [ ] Integration with common HR systems (Workday, etc.)

**Security:**
- [ ] Key derivation from passphrase
- [ ] Hardware token support (YubiKey) for signing

## v0.4.0 - Intelligence

**Theme:** Help users tell better stories

**Deliverables:**
- [ ] Auto-suggest workstreams based on repo clustering + commit message analysis
- [ ] Impact estimation (lines changed → rough effort sizing)
- [ ] Narrative scaffolding ("This workstream shows progression from X to Y")
- [ ] Peer comparison (anonymized: "you shipped more reviews than 70% of IC3s")
- [ ] Gap detection ("No work in Q2 - is this accurate?")

**ML/AI (local-only, privacy-preserving):**
- [ ] Local LLM for summarizing PR descriptions into workstream bullets
- [ ] Pattern recognition for recurring work types

## v1.0.0 - Production

**Theme:** Enterprise-ready, stable API

**Deliverables:**
- [ ] Stable schema (backward-compatible changes only)
- [ ] Migration tool for schema updates
- [ ] Comprehensive test suite (>90% coverage)
- [ ] Performance benchmarks
- [ ] Security audit
- [ ] Official documentation site
- [ ] Package manager distribution (brew, cargo, apt)

**Stability Guarantees:**
- Schema version negotiation
- 2-year deprecation window for features
- LTS releases for enterprise customers

## Future Possibilities (Post-v1.0)

**Integrations:**
- Jira/Linear issue linking
- Slack/Teams notification bots
- Calendar integration ("block time for self-review")
- GitLab/Bitbucket support

**Advanced Features:**
- Real-time sync (webhook-based event ingestion)
- Team dashboard (aggregate stats without individual detail)
- Historical trend analysis ("your review velocity increased 20% YoY")
- Custom redaction profiles

## Technical Debt & Infrastructure

**CI/CD:**
- [ ] Automated release pipeline
- [ ] Cross-platform builds (Windows, macOS, Linux)
- [ ] Homebrew formula
- [ ] Docker image

**Quality:**
- [ ] Property-based tests for redaction
- [ ] Fuzz testing for ingestors
- [ ] Benchmark suite for cache performance
- [ ] Documentation coverage

**Ecosystem:**
- [ ] VS Code extension for packet preview
- [ ] GitHub Action for CI packet generation
- [ ] Pre-commit hooks for validation

## Release Cadence

- **Patch releases (0.x.y):** As needed for bug fixes
- **Minor releases (0.x.0):** Every 6-8 weeks
- **Major releases (x.0.0):** When breaking changes required (v1.0 TBD)

## How to Influence This Roadmap

1. Open an issue describing your use case
2. Comment on existing issues with +1 or detailed feedback
3. Submit PRs (see CONTRIBUTING.md)
4. Join discussions in GitHub Discussions

---

*Last updated: 2026-02-11*
*Roadmap version: 0.1.0*
