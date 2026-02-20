# shiplog-hooks

Pre/post processing hooks for shiplog pipeline.

## Overview

Provides a hook system for extending the shiplog pipeline with custom pre-processing and post-processing logic. Hooks can be registered for various pipeline stages to modify data, log events, or trigger external actions.

## Features

- Hook trait for implementing custom processing logic
- Multiple pipeline stage hooks: pre/post ingest, transform, render, bundle, export
- Hook registry for managing multiple hooks
- Closure-based hook creation for simple use cases

## Usage

```rust
use shiplog_hooks::{HookRegistry, HookContext, HookResult, PipelineStage, HookMetadata, ClosureHook};

// Create a registry
let mut registry = HookRegistry::new();

// Register a closure hook
registry.register(
    PipelineStage::PreIngest,
    ClosureHook::new("my-hook", |context: &HookContext| {
        println!("Processing at stage: {}", context.pipeline_stage);
        HookResult::success()
    }),
);

// Execute hooks at a stage
let context = HookContext {
    pipeline_stage: PipelineStage::PreIngest,
    data: serde_json::json!({"events": []}),
    metadata: HookMetadata::new("test"),
};

let result = registry.execute(PipelineStage::PreIngest, context);
```

## License

MIT OR Apache-2.0
