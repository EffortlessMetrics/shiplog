# Fuzzing Infrastructure for shiplog

This directory contains fuzzing harnesses for `cargo fuzz` (libFuzzer) to test the robustness of shiplog's input parsers.

## Overview

Fuzzing targets the parsing and processing of external data formats that can cause panics, crashes, or unexpected behavior. The strategy balances finding real bugs with maintaining a sustainable fuzzing program integrated into the development workflow.

## Why fuzz shiplog?

Two primary surfaces are worth punishing:

1. **JSONL ingestion** (`ledger.events.jsonl`) - Primary user-facing data format
2. **YAML workstream edits** (`workstreams.yaml`) - User-curated input that directly affects output

Additional targets include:
- GitHub API responses (external API, but responses are controlled by GitHub)
- Configuration files (`shiplog.yaml`) - User configuration
- Manual events YAML (`manual_events.yaml`) - User-curated input

## Setup

```bash
# Install cargo-fuzz
cargo install cargo-fuzz

# Initialize fuzzing (if not already done)
cargo fuzz init

# Run a specific fuzz target
cargo fuzz run parse_jsonl

# Run with corpus from examples
cargo fuzz run parse_jsonl -- -dict=fuzz/dictionaries/json.dict
```

## Fuzz Targets

| Target | Description | Priority |
|--------|-------------|----------|
| `parse_jsonl` | JSONL ingestion (ledger.events.jsonl) | HIGH |
| `parse_workstreams` | YAML workstream edits (workstreams.yaml) | HIGH |
| `parse_github_api` | GitHub API responses | MEDIUM |
| `parse_config` | Configuration files (shiplog.yaml) | MEDIUM |
| `parse_manual_events` | Manual events YAML (manual_events.yaml) | HIGH |

## Dictionaries

Structured format dictionaries are provided to guide the fuzzer toward valid inputs:

- `dictionaries/json.dict` - Common JSON keys and values
- `dictionaries/yaml.dict` - Common YAML keys and values

Use dictionaries when running fuzzers:

```bash
cargo fuzz run parse_jsonl -- -dict=fuzz/dictionaries/json.dict
cargo fuzz run parse_workstreams -- -dict=fuzz/dictionaries/yaml.dict
```

## Corpus Management

The fuzzer maintains a corpus of interesting inputs in `fuzz/corpus/<target>/`. Seed the corpus with valid inputs:

```bash
# Copy fixture files to corpus
mkdir -p fuzz/corpus/parse_jsonl
cp examples/fixture/ledger.events.jsonl fuzz/corpus/parse_jsonl/

mkdir -p fuzz/corpus/parse_workstreams
# Create a sample workstreams.yaml
```

## CI/CD Integration

For CI, run short fuzzing runs to catch obvious issues:

```bash
# Run for 30 seconds per target
cargo fuzz run parse_jsonl -- -max_total_time=30
cargo fuzz run parse_workstreams -- -max_total_time=30
```

## Running All Fuzzers

A convenience script to run all fuzzers for a short duration:

```bash
#!/bin/bash
# fuzz-all.sh

targets=("parse_jsonl" "parse_workstreams" "parse_github_api" "parse_config" "parse_manual_events")

for target in "${targets[@]}"; do
    echo "Fuzzing $target..."
    cargo fuzz run "$target" -- -max_total_time=30
done
```

## Debugging Crashes

When a crash is found, the fuzzer will create a crash input file. To reproduce:

```bash
cargo fuzz run parse_jsonl fuzz/artifacts/parse_jsonl/crash-<hash>
```

## Minimizing Crash Inputs

To minimize a crash input:

```bash
cargo fuzz cmin parse_jsonl fuzz/artifacts/parse_jsonl/crash-<hash>
```

## Coverage Analysis

To check coverage:

```bash
cargo fuzz coverage parse_jsonl
```

## Reference

For more details, see:
- [Fuzzing Strategy](../plans/fuzzing-strategy.md)
- [cargo-fuzz documentation](https://github.com/rust-fuzz/cargo-fuzz)
- [libFuzzer documentation](https://llvm.org/docs/LibFuzzer.html)
