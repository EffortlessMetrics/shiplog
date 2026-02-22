# shiplog-env

Environment variable utilities for shiplog.

## Usage

```rust
use shiplog_env::{get_var, is_set, is_truthy};

let value = get_var("MY_VAR");
if is_truthy("DEBUG") {
    println!("Debug mode enabled");
}
```
