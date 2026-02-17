# shiplog-cluster-llm

LLM-assisted workstream clustering adapter.

## Key types

- `LlmClusterer`: clusters events using an LLM backend.
- `LlmWithFallback`: falls back to repo clustering on failure.
- `LlmConfig`: model/request constraints.
- `OpenAiCompatibleBackend`: backend for OpenAI-compatible chat-completions APIs.

The crate parses structured LLM output into canonical `WorkstreamsFile` data.
