# shiplog-eventbus

Event bus for internal messaging within shiplog.

## Usage

```rust
use shiplog_eventbus::{EventBus, Event, EventHandler};

let bus = EventBus::new();
bus.subscribe("topic", handler);
bus.publish(Event::new("topic", payload));
```

## Features

- `Event` — typed event with topic and payload
- `EventBus` — thread-safe publish/subscribe bus with history
- `EventHandler` — trait for event subscribers

## Part of the shiplog workspace

See the [workspace README](../../README.md) for overall architecture.

## License

Licensed under either of [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
or [MIT license](http://opensource.org/licenses/MIT) at your option.
