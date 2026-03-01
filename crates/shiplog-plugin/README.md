# shiplog-plugin

Plugin system for loadable third-party ingest adapters.

## Usage

```rust
use shiplog_plugin::{PluginManager, PluginManifest};

let manager = PluginManager::new();
manager.install(&manifest)?;
let plugins = manager.list();
```

## Features

- `PluginManifest` — metadata describing a plugin (name, version, entry point)
- `PluginStatus` — tracks plugin lifecycle state
- `PluginManager` — install, list, and manage plugins

## Part of the shiplog workspace

See the [workspace README](../../README.md) for overall architecture.

## License

Licensed under either of [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
or [MIT license](http://opensource.org/licenses/MIT) at your option.
