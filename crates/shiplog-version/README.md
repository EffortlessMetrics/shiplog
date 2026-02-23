# shiplog-version

Version parsing and comparison utilities for shiplog.

## Usage

```rust
use shiplog_version::Version;

let version = Version::parse("1.2.3").unwrap();
println!("Version: {}", version);
```
