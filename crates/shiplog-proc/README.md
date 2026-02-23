# shiplog-proc

Process utilities for shiplog.

## Usage

```rust
use shiplog_proc::{pid, num_cpus, process_name};

println!("PID: {}", pid());
println!("CPUs: {}", num_cpus());
println!("Process: {}", process_name());
```
