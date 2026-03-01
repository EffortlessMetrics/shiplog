# shiplog-cluster-llm-prompt

Prompt formatting helpers for LLM workstream clustering.

## Usage

```rust
use shiplog_cluster_llm_prompt::{system_prompt, summarize_event, chunk_events};

let prompt = system_prompt();
let summary = summarize_event(&event);
let chunks = chunk_events(&events, token_budget);
```

## Features

- `system_prompt()` — generates the LLM system prompt for clustering
- `summarize_event()` — produces an LLM-friendly event summary
- `format_event_list()` — formats a batch of events for prompt inclusion
- `chunk_events()` — splits large event sets by estimated token budget

## Part of the shiplog workspace

See the [workspace README](../../README.md) for overall architecture.

## License

Licensed under either of [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
or [MIT license](http://opensource.org/licenses/MIT) at your option.
