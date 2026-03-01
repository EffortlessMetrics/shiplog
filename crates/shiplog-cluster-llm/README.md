# shiplog-cluster-llm

LLM-assisted workstream clustering adapters.

## Main types

- `LlmClusterer`: clusters events through a configurable LLM backend.
- `LlmWithFallback`: falls back to repo clustering when LLM clustering fails.
- `LlmConfig`: request/model/token budget configuration.
- `OpenAiCompatibleBackend`: OpenAI chat-completions protocol backend.
- `parse_llm_response` is delegated to `shiplog-cluster-llm-parse` as a dedicated parser microcrate.

## Notes

- Prompts expect JSON object output describing workstreams and receipt indices.
- Large event sets are chunked by estimated token budget.
- Parsing handles invalid indices conservatively and creates an `Uncategorized` workstream for orphans.
