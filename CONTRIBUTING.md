# Contributing to Shiplog

Thank you for your interest in contributing to shiplog! This document provides guidelines for contributing to the project.

## Code of Conduct

Be respectful, inclusive, and constructive. We're building a tool that helps people tell their professional stories with receipts and coverage.

## Getting Started

### Prerequisites

- Rust 1.92+ (see `rust-version` in `Cargo.toml`)
- SQLite (bundled via rusqlite)
- Git

### Setup

```bash
git clone https://github.com/EffortlessMetrics/shiplog.git
cd shiplog
cargo build --release
```

### Running Tests

```bash
# Run all tests
cargo test --workspace

# Run with all features
cargo test --workspace --all-features

# Run clippy (zero warnings policy)
cargo clippy --workspace --all-targets -- -D warnings

# Check formatting
cargo fmt --all -- --check
```

## Development Workflow

1. **Fork and branch** from `main`
2. **Make your changes** with clear commit messages
3. **Add tests** for new functionality
4. **Update documentation** (README, CHANGELOG, etc.)
5. **Run the full test suite**
6. **Submit a PR** with a clear description

### Commit Message Format

```
type(scope): description

[optional body]

[optional footer]
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style (formatting, missing semicolons, etc.)
- `refactor`: Code refactoring
- `perf`: Performance improvements
- `test`: Adding or updating tests
- `chore`: Build process, dependencies, etc.

Examples:
```
feat(cache): add TTL-based cache expiration

fix(ingest): handle GHES host URLs correctly

docs: update README with new CLI options
```

## Project Structure

```
shiplog/
├── apps/shiplog/           # Main CLI application
├── crates/
│   ├── shiplog-bundle/     # Archive format
│   ├── shiplog-cache/      # SQLite caching
│   ├── shiplog-coverage/   # Coverage tracking
│   ├── shiplog-engine/     # Core orchestration
│   ├── shiplog-ids/        # ID generation
│   ├── shiplog-ingest-github/  # GitHub adapter
│   ├── shiplog-ingest-json/    # JSON adapter
│   ├── shiplog-ingest-manual/  # Manual events
│   ├── shiplog-ports/      # Port traits
│   ├── shiplog-redact/     # Redaction system
│   ├── shiplog-render-md/  # Markdown renderer
│   ├── shiplog-render-json/# JSON renderer
│   ├── shiplog-schema/     # Data models
│   ├── shiplog-testkit/    # Test utilities
│   └── shiplog-workstreams/# Workstream clustering
├── examples/               # Example fixtures
└── fuzz/                   # Fuzzing targets
```

## Architecture Guidelines

### Ports and Adapters

We use a ports-and-adapters (hexagonal) architecture:

- **Ports** (`shiplog-ports`): Define interfaces (traits)
- **Adapters**: Implement ports for specific technologies
  - Primary adapters: CLI, future GUI
  - Secondary adapters: GitHub API, SQLite, filesystem

### Crate Boundaries

- Each crate has a single responsibility
- Crates depend only on what they need
- Prefer dependency injection over global state
- Public API should be minimal and well-documented

### Testing Strategy

- **Unit tests**: In each crate's `src/` directory
- **Property-based tests**: For redaction, ID generation
- **Integration tests**: For end-to-end workflows
- **Snapshot tests**: For packet output stability

## Redaction Testing

When modifying redaction code, ensure you add property-based tests:

```rust
#[test]
fn my_redaction_no_leak() {
    let r = DeterministicRedactor::new(b"test-key");
    let sensitive = "secret data";
    
    let ev = create_event_with(sensitive);
    let out = r.redact_events(&[ev], "public").unwrap();
    let json = serde_json::to_string(&out).unwrap();
    
    assert!(!json.contains(sensitive), "Sensitive data leaked in output");
}
```

## Performance Guidelines

- Cache expensive operations (API calls, calculations)
- Use streaming for large files
- Avoid unnecessary allocations in hot paths
- Profile before optimizing (`cargo flamegraph`)

## Security Considerations

- Never log tokens or sensitive data
- Redaction must be deterministic (same input → same alias)
- All redaction happens locally; no data leaves the machine
- Cache files should have restrictive permissions

## Documentation

- Add doc comments to all public APIs
- Update README.md for user-facing changes
- Update ARCHITECTURE.md for structural changes
- Update CHANGELOG.md for each release

## Release Process

1. Update version in `Cargo.toml` (workspace)
2. Update `CHANGELOG.md`
3. Create release branch: `git checkout -b release/vX.Y.Z`
4. Run full test suite
5. Create PR for review
6. After merge, tag release: `git tag vX.Y.Z`
7. Push tags: `git push --tags`
8. GitHub Actions builds release artifacts

## Questions?

- Open an issue for bugs or feature requests
- Start a discussion for architectural questions
- Join our community chat (link TBD)

## License

By contributing, you agree that your contributions will be licensed under the MIT OR Apache-2.0 license.
