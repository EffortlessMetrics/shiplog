# Roadmap

This roadmap is organized by horizon rather than dates. Items move between horizons as priorities shift.

## Now (v0.2.x)

Active development. These items are in progress or planned for the next minor release.

- **Binary releases via GitHub Actions** -- Pre-built binaries for Linux, macOS, and Windows so users do not need a Rust toolchain to install shiplog.
- **Local git ingest adapter** (`shiplog-ingest-git`) -- Ingest commit history directly from a local git repository without going through the GitHub API.
- **Improved packet formatting** -- Better Markdown structure, configurable section ordering, and cleaner receipt presentation.
- **Cache improvements** -- TTL configuration, cache size limits, and cache inspection commands.

## Next (v0.3.x)

Planned after current work stabilizes. Design may still evolve.

- **GitLab ingest adapter** -- Merge request and review ingestion from GitLab, including self-hosted instances.
- **Jira/Linear ingest adapter** -- Pull issue and ticket data as evidence alongside code activity.
- **Multi-source merging** -- Combine events from multiple sources (e.g., GitHub + manual + Jira) into a single coherent packet with unified coverage tracking.
- **Configurable packet templates** -- User-defined Markdown templates for packet rendering, replacing the built-in layout.
- **LLM clustering as default feature** -- Move LLM-assisted clustering out of feature-gate once the interface stabilizes.

## Later (exploratory)

Ideas under consideration. No commitment to timeline or implementation.

- **Team aggregation mode** -- Generate packets across multiple users for team-level shipping summaries.
- **Continuous/cron mode** -- Run shiplog on a schedule, appending to an existing ledger incrementally.
- **TUI workstream editor** -- Interactive terminal UI for curating workstreams instead of hand-editing YAML.
- **Web viewer** -- Browser-based viewer for rendered packets, with navigation and search.
- **Plugin system** -- Loadable adapter plugins so third-party sources do not require forking the repository.

## Non-goals

These are intentionally out of scope for shiplog:

- **Analytics dashboard.** shiplog produces static packets, not live dashboards or metrics visualizations.
- **AI-generated narrative.** Packets provide evidence scaffolds and optional LLM clustering. The narrative is written by the human, not generated.
- **Telemetry or tracking.** shiplog does not phone home, collect usage data, or require an account.
- **Manager-facing tooling.** shiplog is for the IC preparing their own review. Manager workflows are out of scope.

## How to influence the roadmap

- Open a [GitHub issue](https://github.com/EffortlessMetrics/shiplog/issues) with the `enhancement` label to propose new features.
- Pull requests are welcome for items in the Now and Next horizons. See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.
- For Later items, start with an issue to discuss the approach before writing code.
