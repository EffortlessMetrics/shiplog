# shiplog-watcher

File system watcher for real-time shiplog sync.

## Usage

```rust
use shiplog_watcher::{FileWatcher, WatcherConfig};

let config = WatcherConfig { recursive: true, ..Default::default() };
let watcher = FileWatcher::new("./out", config)?;
for event in watcher.events() {
    println!("{:?}: {}", event.kind, event.path.display());
}
```

## Features

- `WatcherConfig` — recursive monitoring and polling configuration
- `FileWatcher` — watches directories for file system events
- `FileEvent` — describes a detected change
- `FileEventKind` — created, modified, removed event types

## Part of the shiplog workspace

See the [workspace README](../../README.md) for overall architecture.

## License

Licensed under either of [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
or [MIT license](http://opensource.org/licenses/MIT) at your option.
