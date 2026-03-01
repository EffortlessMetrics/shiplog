# shiplog-pubsub

Pub/sub messaging utilities for shiplog event distribution.

## Usage

```rust
use shiplog_pubsub::{PubSub, Subscriber};

let bus: PubSub<String> = PubSub::new();
let sub = bus.subscribe("topic");
bus.publish("topic", "hello".into());
let msg = sub.recv()?;
```

## Features

- `Message<T>` — typed message with topic and payload
- `PubSub<T>` — generic publish/subscribe broker
- `Subscriber<T>` — topic-based subscriber with receive

## Part of the shiplog workspace

See the [workspace README](../../README.md) for overall architecture.

## License

Licensed under either of [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
or [MIT license](http://opensource.org/licenses/MIT) at your option.
