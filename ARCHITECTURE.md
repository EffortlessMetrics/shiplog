# Shiplog Architecture

This document explains the high-level architecture of shiplog and how the pieces fit together.

## Overview

Shiplog is a self-review packet generator that:
1. **Ingests** work evidence from various sources (GitHub, manual events, JSON files)
2. **Clusters** events into workstreams (thematic groups)
3. **Curates** workstreams with user edits
4. **Redacts** sensitive information for different audiences
5. **Renders** packets in various formats (Markdown, JSON)
6. **Bundles** everything into a distributable artifact

## Architecture Pattern: Ports and Adapters

We use a ports-and-adapters (hexagonal) architecture to keep the core domain independent of external concerns.

```
┌─────────────────────────────────────────────────────────────┐
│                    PRIMARY ADAPTERS                         │
│                  (CLI, future GUI/TUI)                      │
└───────────────────────┬─────────────────────────────────────┘
                        │ Uses
                        ▼
┌─────────────────────────────────────────────────────────────┐
│                         PORTS                               │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │ Ingestor │  │Workstream│  │ Redactor │  │ Renderer │   │
│  │  Trait   │  │Clusterer │  │  Trait   │  │  Trait   │   │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘   │
└───────────────────────┬─────────────────────────────────────┘
                        │ Implements
                        ▼
┌─────────────────────────────────────────────────────────────┐
│                   SECONDARY ADAPTERS                        │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │  GitHub  │  │   Repo   │  │Determinis│  │ Markdown │   │
│  │   API    │  │Clusterer │  │ticRedactor│  │ Renderer │   │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘   │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐                  │
│  │   JSON   │  │  SQLite  │  │   JSON   │                  │
│  │  Files   │  │  Cache   │  │ Renderer │                  │
│  └──────────┘  └──────────┘  └──────────┘                  │
└─────────────────────────────────────────────────────────────┘
```

## Core Domain

### Events (`shiplog-schema`)

The canonical data model:

```rust
EventEnvelope {
    id: EventId,           // Unique identifier
    kind: EventKind,       // PR, Review, Manual
    occurred_at: DateTime,
    actor: Actor,
    repo: RepoRef,
    payload: EventPayload, // Type-specific data
    tags: Vec<String>,
    links: Vec<Link>,      // URLs to evidence
    source: SourceRef,     // Where it came from
}
```

### Coverage (`shiplog-coverage`)

Every packet includes a coverage manifest that documents:
- **Completeness**: Complete, Partial, or Incomplete
- **Slicing**: How date ranges were subdivided to handle API limits
- **Sources**: Which ingesters contributed
- **Warnings**: Any caveats about the data

This creates transparency about what the packet represents.

### Workstreams (`shiplog-workstreams`)

Workstreams group related events into narratives:

```rust
Workstream {
    id: WorkstreamId,
    title: String,              // User-curated
    summary: Option<String>,    // User-written narrative
    tags: Vec<String>,          // e.g., "backend", "performance"
    stats: WorkstreamStats,     // PR count, review count, etc.
    events: Vec<EventId>,       // All events in this workstream
    receipts: Vec<EventId>,     // Key events to highlight
}
```

The workflow:
1. **Generate**: Auto-cluster by repo → `workstreams.suggested.yaml`
2. **Curate**: User edits, renames, adds summaries → `workstreams.yaml`
3. **Preserve**: Curated file is never overwritten

## Data Flow

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   Collect   │───▶│    Run      │───▶│   Render    │
│   Command   │    │   Engine    │    │   Command   │
└─────────────┘    └─────────────┘    └─────────────┘
        │                  │                  │
        ▼                  ▼                  ▼
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│  GitHub API │    │ Workstream  │    │   Redactor  │
│  (cached)   │    │   Manager   │    │  (profiles) │
└─────────────┘    └─────────────┘    └─────────────┘
        │                  │                  │
        ▼                  ▼                  ▼
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│ledger.events│    │workstreams. │    │  packet.md  │
│   .jsonl    │    │    yaml     │    │             │
└─────────────┘    └─────────────┘    └─────────────┘
```

## Key Design Decisions

### 1. Immutable Events

Once written to `ledger.events.jsonl`, events are never modified. This creates an audit trail. If data needs correction, a new run is created.

### 2. Curated Workstreams

The tool suggests workstreams but the user has final say. The curated `workstreams.yaml` is preserved across refreshes, allowing the narrative to evolve as the user reviews their work.

### 3. Deterministic Redaction

Given the same redaction key and input, the output is always identical. This enables:
- Stable aliases across runs
- Testability
- No randomness that could leak information

### 4. Local-First

All processing happens locally:
- No data sent to external services
- SQLite cache for API responses
- Redaction happens before any sharing

### 5. Schema Versioning

The bundle format includes schema version information, enabling future migration tools.

## Crate Responsibilities

| Crate | Responsibility | Key Types |
|-------|---------------|-----------|
| `shiplog-ports` | Interface definitions | `Ingestor`, `Renderer`, `Redactor`, `WorkstreamClusterer` |
| `shiplog-schema` | Data models | `EventEnvelope`, `Workstream`, `CoverageManifest` |
| `shiplog-engine` | Orchestration | `Engine::run()`, `Engine::refresh()` |
| `shiplog-ingest-github` | GitHub API adapter | `GithubIngestor` |
| `shiplog-cache` | SQLite caching | `ApiCache`, `CacheKey` |
| `shiplog-workstreams` | Clustering logic | `RepoClusterer`, `WorkstreamManager` |
| `shiplog-redact` | Redaction profiles | `DeterministicRedactor`, `RedactionProfile` |
| `shiplog-render-md` | Markdown output | `MarkdownRenderer` |
| `shiplog-bundle` | Archive creation | `Bundle::create()` |
| `shiplog-ids` | ID generation | `EventId`, `RunId`, `WorkstreamId` |

## Security Model

### Trust Boundaries

1. **Local machine**: Full trust, all data available
2. **Internal sharing**: Manager profile, some context preserved
3. **Public sharing**: Public profile, maximum redaction

### Redaction Profiles

| Profile | Titles | Repos | URLs | Paths |
|---------|--------|-------|------|-------|
| Internal | ✓ | ✓ | ✓ | ✓ |
| Manager | ✓ | ✓ | ✗ | ✗ |
| Public | Aliased | Aliased | ✗ | ✗ |

## Extension Points

### Custom Ingestors

Implement the `Ingestor` trait:

```rust
impl Ingestor for MyIngestor {
    fn ingest(&self) -> Result<IngestOutput> {
        // Fetch from custom source
        // Return events + coverage
    }
}
```

### Custom Renderers

Implement the `Renderer` trait for new output formats (PDF, HTML, etc.).

### Custom Clusterers

Implement `WorkstreamClusterer` for different clustering strategies (ML-based, semantic, etc.).

## Future Architecture

See [ROADMAP.md](./ROADMAP.md) for planned architectural evolution.

Key upcoming changes:
- Plugin system for custom ingestors
- Signed bundles for verification
- Incremental updates (delta refresh)
- Real-time sync via webhooks

---

*For questions about architecture, open a discussion in GitHub Discussions.*
