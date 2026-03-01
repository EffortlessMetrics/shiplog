# shiplog-workflow

Workflow and orchestration utilities for shiplog pipeline execution.

## Usage

```rust
use shiplog_workflow::{Workflow, Task, WorkflowEngine};

let mut workflow = Workflow::new("nightly-collect");
workflow.add_task(Task::new("fetch", fetch_fn));
workflow.add_task(Task::new("render", render_fn));

let engine = WorkflowEngine::new();
engine.run(&workflow)?;
```

## Features

- `Workflow` — named sequence of tasks
- `Task` — single unit of work within a workflow
- `WorkflowState` — Pending, Running, Completed, Failed, Cancelled
- `WorkflowEngine` — executes workflows with state tracking

## Part of the shiplog workspace

See the [workspace README](../../README.md) for overall architecture.

## License

Licensed under either of [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
or [MIT license](http://opensource.org/licenses/MIT) at your option.
