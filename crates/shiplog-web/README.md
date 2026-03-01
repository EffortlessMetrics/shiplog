# shiplog-web

Browser-based viewer for rendered packets.

## Usage

```rust
use shiplog_web::{WebViewer, WebConfig};

let config = WebConfig { host: "127.0.0.1".into(), port: 8080 };
let viewer = WebViewer::new(config);
viewer.serve("./out/run_id")?;
```

## Features

- `WebConfig` — host and port configuration
- `WebViewer` — serves rendered packets in a local browser UI

## Part of the shiplog workspace

See the [workspace README](../../README.md) for overall architecture.

## License

Licensed under either of [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
or [MIT license](http://opensource.org/licenses/MIT) at your option.
