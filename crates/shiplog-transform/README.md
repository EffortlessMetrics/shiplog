# shiplog-transform

Event transformation and enrichment pipeline for shiplog.

## Usage

```rust
use shiplog_transform::{TransformPipeline, TransformRule, TransformRuleType};

let mut pipeline = TransformPipeline::new();
pipeline.add_rule(TransformRule::new("add-team", TransformRuleType::AddField {
    field: "team".into(),
    value: "platform".into(),
}));
let enriched = pipeline.apply(events)?;
```

## Features

- `RawEvent` / `EnrichedEvent` — pre- and post-transform event types
- `TransformRule` — named transformation step
- `TransformRuleType` — Rename, AddField, RemoveField, MapValues, Custom
- `TransformPipeline` — ordered chain of transformation rules
- `add_tag()`, `add_metadata()`, `enrich_with_workstream()` — helper functions

## Part of the shiplog workspace

See the [workspace README](../../README.md) for overall architecture.

## License

Licensed under either of [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
or [MIT license](http://opensource.org/licenses/MIT) at your option.
