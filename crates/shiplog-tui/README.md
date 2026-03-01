# shiplog-tui

Interactive terminal UI for curating workstreams.

## Usage

```rust
use shiplog_tui::{TuiEditor, TuiConfig, TuiMode};

let config = TuiConfig { mode: TuiMode::Interactive };
let editor = TuiEditor::new(config);
editor.run()?;
```

## Features

- `TuiConfig` — configuration for the terminal UI
- `TuiMode` — interactive or batch mode
- `TuiEditor` — workstream curation editor

## Part of the shiplog workspace

See the [workspace README](../../README.md) for overall architecture.

## License

Licensed under either of [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
or [MIT license](http://opensource.org/licenses/MIT) at your option.
