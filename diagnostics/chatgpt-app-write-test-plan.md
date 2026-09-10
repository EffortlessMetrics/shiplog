# ChatGPT connected-app write A/B test plan

Diagnostic only. This file and the probe branches are intentionally isolated from `main`.

## Question

Does GPT-6 Pro / GPT-6 Astra fail connected-app writes more often than GPT-5.6 Sol under matched conditions, and if so does the failure follow payload size, concurrent app-write load, automatic review, or provider-specific behavior?

## Rules

- Use a fresh chat for each measured run.
- Select the requested model before sending the test prompt.
- Use the connected app only.
- Perform exactly one measured write attempt.
- Never retry a failed or ambiguous measured write.
- Immediately read authoritative provider state after the attempt, even when the write reports an error.
- Record the exact surfaced error/status text verbatim.
- Do not switch models inside a measured run.
- Keep measured GitHub runs on separate branches so branch-head/SHA races cannot masquerade as model failures.
- Treat `quiet` and `storm` as observed load conditions. A storm means normal concurrent ChatGPT/Codex GitHub write activity is already high; do not manufacture a same-branch stress test.

## GitHub cells, repetition 01

| Model | Load | Payload | Branch |
| --- | --- | --- | --- |
| Astra | quiet | small | `diag/chatgpt-write-astra-q-small-01` |
| Sol | quiet | small | `diag/chatgpt-write-sol-q-small-01` |
| Astra | quiet | large | `diag/chatgpt-write-astra-q-large-01` |
| Sol | quiet | large | `diag/chatgpt-write-sol-q-large-01` |
| Astra | storm | small | `diag/chatgpt-write-astra-storm-small-01` |
| Sol | storm | small | `diag/chatgpt-write-sol-storm-small-01` |
| Astra | storm | large | `diag/chatgpt-write-astra-storm-large-01` |
| Sol | storm | large | `diag/chatgpt-write-sol-storm-large-01` |

Small runs update only `diagnostics/chatgpt-app-write-probe.md` from `SMALL_MARKER=seed-5.6` to the run ID.

Large runs create `diagnostics/chatgpt-app-write-large-probe.txt` in their isolated branch. The intended file is:

- first line: `RUN_ID=<run-id>`
- second line: exactly 32,768 characters generated as `"0123456789abcdef"` repeated 2,048 times
- trailing newline

The generated payload keeps the user prompt small while making the nested app write action large.

## Private Google Drive cells

Eight native Google Docs with the matching Astra/Sol × quiet/storm × small/large names are pre-seeded in the user's `ChatGPT` Drive folder. Each starts with `SMALL_MARKER=seed-5.6` and `LARGE_MARKER=seed-5.6`.

Small runs replace only the small marker with the run ID. Large runs replace only the large marker with `LARGE_MARKER=<run-id>:` followed by the same exact 32,768-character generated payload. Each run then reads the document text back immediately.

## Run order

Do not always put Astra first. For repetition 01 use paired AB/BA ordering where possible:

1. GitHub Astra quiet small, then Sol quiet small control if needed.
2. Drive Sol quiet small control, then Astra quiet small.
3. GitHub Sol quiet large, then Astra quiet large.
4. Drive Astra quiet large, then Sol quiet large.
5. Repeat the same payload pairs during a naturally occurring high-concurrency period, reversing first-model order from the quiet pair.

For repetitions 02 and 03, create fresh isolated branches/docs and reverse or randomize model order. Do not reuse a mutated cell.

## Observation record

For every measured attempt record:

| Field | Value |
| --- | --- |
| Timestamp with timezone | |
| Product surface | Chat / Work / Codex |
| Client/app version if visible | |
| Model selected | |
| Load condition | quiet / storm |
| Approx. other active app-writing threads | |
| Provider | GitHub / Google Drive |
| Payload | small / 32 KiB |
| Exact app action/tool used | |
| Write action emitted? | yes / no |
| First write result | success / error / timeout / denial / ambiguous |
| Exact surfaced text | |
| Elapsed time to result | |
| Authoritative read-back | |
| Intended mutation exists remotely | yes / no |
| Tool/app still usable after failure | yes / no |

## Interpretation

- Astra small succeeds, Astra large fails, Sol succeeds on both: strongly supports a model-specific review/payload path.
- Astra fails on GitHub and Drive under matched quiet conditions: weakens GitHub-specific throttling and strengthens a model/harness/review explanation.
- Both models fail mainly during storms: load/backpressure dominates.
- Failure near the automatic-review timeout window, or explicit automatic-review/Guardian text: strongly implicates the Astra review path.
- Error reported but read-back contains the intended state: acknowledgement/reconciliation failure, not a failed provider mutation.
- No write action is emitted at all: model/tool-discovery or model-policy behavior, not provider write execution.
- 429 or explicit secondary-rate-limit text: GitHub throttling.
- 503, connector dependency/rate-limit text, transport closure, or MCP timeout: OpenAI connector/runtime path.
- 409/422 on GitHub: check for branch/SHA race before attributing to model.

## Existing baseline

On 2026-09-10, GPT-5.6 Sol completed the quiet-small GitHub probe on its first write and immediate read-back confirmed `SMALL_MARKER=sol-q-small-01`. GPT-5.6 Sol also completed the quiet-small Google Drive marker replacement on its first measured write and immediate read-back confirmed `SMALL_MARKER=sol-q-small-01`.
