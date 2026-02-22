# shiplog-semver

Semantic version utilities for shiplog.

## Usage

```rust
use shiplog_semver::SemVer;

let version = SemVer::parse("1.2.3-alpha.1+build.123").unwrap();
println!("Version: {}", version);
```
