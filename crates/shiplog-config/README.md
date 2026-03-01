# shiplog-config

Configuration management and loading for shiplog.

## Usage

```rust
use shiplog_config::{ShiplogConfig, load_config, save_config, ConfigFormat};

let config = load_config("shiplog.yaml")?;
save_config(&config, "shiplog.json", ConfigFormat::Json)?;
```

## Features

- `ShiplogConfig` — top-level configuration struct
- `WorkstreamConfig` — workstream-specific settings
- `ConfigFormat` — YAML and JSON format support
- `load_config()` / `save_config()` — file-based config I/O

## Part of the shiplog workspace

See the [workspace README](../../README.md) for overall architecture.

## License

Licensed under either of [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
or [MIT license](http://opensource.org/licenses/MIT) at your option.
