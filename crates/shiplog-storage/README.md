# shiplog-storage

Storage abstractions for shiplog outputs and cache-like data.

- Common storage trait and key/value error model
- In-memory implementation used for tests and lightweight callers
- Minimal surface area for plugging filesystem, cloud, or queue-backed backends

## Compatibility

The contract is intentionally conservative to make custom storage adapters easy to
implement and easy to swap.
