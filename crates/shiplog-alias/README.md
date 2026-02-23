# shiplog-alias

Deterministic alias generation and alias cache persistence for shiplog.

This crate isolates keyed aliasing from higher-level redaction policy so other
crates can share a stable alias contract.

## API

- `DeterministicAliasStore`: keyed alias generation with in-memory cache
- `load_cache` / `save_cache`: persist aliases across runs
- `CACHE_FILENAME`: canonical alias cache filename
