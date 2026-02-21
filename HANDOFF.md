# shiplog - Handoff

*Last updated: 2026-02-17*

## 0) One sentence

**shiplog compiles activity into a defensible self-review packet - with receipts, explicit coverage limits, and safe share variants.**

It is intentionally **not** an engineering analytics dashboard and **not** an "AI writes your review" app.

---

## 1) Product vs substrate

### The product: the CLI (`shiplog`)

This is what users "buy" (even when it is OSS):

- one workflow: **collect -> curate -> render**
- one artifact set: **packet + ledger + coverage + bundles**
- one mental model: **narrative is human; receipts are machine**

### The substrate: a public crate ecosystem

Publishing the crates is useful for integrations, but you must be explicit about contracts:

#### Stable integration surface (treat as public API)

- **`shiplog-schema`** - canonical types + JSONL/JSON contracts
- **`shiplog-ports`** - extension traits (ingest/cluster/render/redact)
- **`shiplog-ids`** - deterministic IDs

These are what third parties should build against.

#### Supported modules (published, but not a promise)

Everything else can be published for composability/transparency, but you reserve the right to reshape internals between minor versions:

- `shiplog-engine`, `shiplog-bundle`, `shiplog-redact`, `shiplog-workstreams`, `shiplog-cache`
- `shiplog-ingest-*`, `shiplog-render-*`, `shiplog-cluster-llm`

#### Dev-only

- `shiplog-testkit` (recommended: **unpublished** or explicitly unstable)

**Rule:** Do not let publishable crates depend on unpublished crates (even dev-deps) unless you have proven packaging/publish works.

---

## 2) What shiplog produces

Run outputs live under `out/<run_id>/`:

```text
out/<run_id>/
  packet.md                     # internal packet (primary)
  workstreams.yaml              # curated by user; never overwritten
  workstreams.suggested.yaml    # generated suggestions; safe to regen
  ledger.events.jsonl           # canonical event ledger (append-only)
  coverage.manifest.json        # what was queried; completeness + gaps
  bundle.manifest.json          # checksums of included files
  profiles/
    manager/packet.md           # redacted for managers (context kept)
    public/packet.md            # redacted for sharing (aliases/strips)
  <run_id>[.<profile>].zip      # optional bundle zip (profile-scoped)
```

### Trust surfaces

- **Ledger** is the evidence appendix (machine output).
- **Packet** is narrative scaffolding (human-edited).
- **Coverage manifest** is the "honesty stamp" (what might be missing).
- **Bundles** are the share artifact (integrity + redaction posture).

---

## 3) User workflows (what "done" feels like)

### 3.1 Collect (GitHub)

```bash
shiplog collect github \
  --user <login> \
  --since YYYY-MM-DD \
  --until YYYY-MM-DD \
  --mode merged \
  --out ./out
```

Notes:

- `--mode merged|created` changes the "lens" for PR collection.
- `--include-reviews` adds review events (best-effort coverage).
- `--throttle-ms <N>` helps avoid rate limits.
- `--api-base <URL>` supports GHES.

### 3.2 Curate workstreams (human step)

- copy `workstreams.suggested.yaml` -> `workstreams.yaml`
- merge/split/rename workstreams until they match how you talk about your year
- pick a handful of receipts per workstream; the long tail stays in the ledger

### 3.3 Render (no re-fetch)

```bash
shiplog render --run-dir out/<run_id>
```

Add safe sharing variants:

```bash
shiplog render --run-dir out/<run_id> --redact-key <KEY>
```

### 3.4 Refresh (re-fetch receipts; keep curation)

Refresh is "update evidence, do not touch my narrative."

```bash
shiplog refresh github \
  --user <login> \
  --since YYYY-MM-DD \
  --until YYYY-MM-DD \
  --run-dir out/<run_id> \
  --out ./out
```

### 3.5 Import (org mode / upstream ledger)

Import consumes an existing directory containing at least:

- `ledger.events.jsonl`
- `coverage.manifest.json`

```bash
shiplog import --dir <existing_run_dir> --out ./out
```

`--regen` should force re-clustering (and must not preserve stale workstreams from the target output dir).

### 3.6 Bundles and profile-scoped sharing

- `--zip` writes a zip bundle
- `--bundle-profile internal|manager|public` scopes included files

Recommended default for sharing outside your machine:

- **manager** bundle for internal reviewers
- **public** bundle for recruiting / external sharing

---

## 4) Core contracts (schemas that matter)

### Events

`EventEnvelope` is the canonical unit:

- stable `EventId`
- `EventKind` (PR/Review/Manual/etc.)
- `RepoRef` with visibility
- payload per event type
- tags + links
- `SourceRef` (`SourceSystem` + URL/opaque id)

**Design rule:** Never "invent impact." Impact fields exist as prompts; users fill them.

### Coverage

Coverage is not an afterthought:

- completeness is **first-class**
- slicing/rate/cap failures must be visible
- "authoritative-looking lies" are the category-killer; coverage prevents that.

### Workstreams

Workstreams are the primary UI/output unit:

- suggestions are disposable (`workstreams.suggested.yaml`)
- curated file is state (`workstreams.yaml`) and treated like source code

### Redaction

Three render profiles:

- internal: full
- manager: strips sensitive details while keeping context
- public: aliases names + strips links/titles/etc.

**Important:** alias cache (`redaction.aliases.json`) must be **excluded** from share bundles.

---

## 5) Architecture (ports-and-adapters)

### Dependency rules

- **Foundation** crates (`ids`, `schema`, `ports`) have no adapter deps.
- Adapters depend on foundation and ports.
- Engine wires adapters through ports.
- CLI wires the concrete graph; it is the "composition root."

### Key crates

- `shiplog-schema`: canonical types + on-disk contracts
- `shiplog-ports`: traits (`Ingestor`, `WorkstreamClusterer`, `Renderer`, `Redactor`)
- `shiplog-ids`: stable deterministic IDs
- `shiplog-engine`: orchestration (collect/render/refresh/import)
- `shiplog-ingest-github`: GitHub adapter (window slicing, caching, GHES)
- `shiplog-cache`: SQLite TTL cache for API responses
- `shiplog-workstreams`: clustering + curated/suggested semantics
- `shiplog-redact`: deterministic redaction profiles + alias cache persistence
- `shiplog-bundle`: manifests + zip bundles (profile-scoped)
- `shiplog-render-md`: Markdown packet renderer (snapshot-tested)
- `shiplog-render-json`: JSON/JSONL render outputs
- `shiplog-cluster-llm`: optional semantic clustering via OpenAI-compatible endpoint
- `shiplog-testkit`: scenario helpers (BDD) - dev-only by default

---

## 6) Quality gates and testing posture

### Baseline gates (always)

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

### Snapshot tests (visual regression)

- Use `insta::assert_snapshot!` for Markdown strings.
- Prefer stable timestamps in fixtures to reduce churn.
- Review diffs like you review code - snapshots are part of the contract.

### Property tests (invariants)

Use `proptest` for things that must never regress:

- ID determinism
- redaction leak guarantees (especially public profile)
- bundle inclusion/exclusion rules
- coverage slicing invariants

### Fuzzing

Fuzz the parsers and file formats first:

- JSONL event ledger
- YAML workstreams / manual events
- LLM response JSON parsing (robustness against junk wrappers)

### Mutation testing

Use mutation testing on "trust surfaces":

- redaction and bundle filters
- schema serialization/deserialization
- coverage slicing logic

---

## 7) Privacy & security posture

### "Safe by default" principles

- LLM clustering is **opt-in** (build feature, not runtime default).
- Public profile stripping must be provable (tests).
- Redaction alias cache is excluded from bundles.

### Repo hygiene gates

- `.gitignore` covers build output, runs, zips, SQLite/db, env files, editor junk, OS junk.
- docs avoid token-like prefixes (`ghp_`, `sk-`) to prevent false positives.

---

## 8) Release and publishing

### Branch protection

- main requires PRs (no direct pushes). Treat release as a PR + tag.

### Versioning

- Workspace version is shared; crates pin path+version internally.
- Keep CHANGELOG accurate and explicit about contracts (especially schema/serde changes).

### First-time crates.io publish strategy

Reality: the ecosystem has internal dependencies, so you must publish **in dependency order**.

Suggested publish order:

1. **Foundation**
   - `shiplog-ids`
   - `shiplog-schema`
   - `shiplog-ports`
2. **Adapters / modules**
   - `shiplog-coverage`, `shiplog-cache`
   - `shiplog-render-json`
   - `shiplog-workstreams`, `shiplog-redact`, `shiplog-render-md`, `shiplog-bundle`
   - `shiplog-ingest-json`, `shiplog-ingest-manual`, `shiplog-ingest-github`
   - `shiplog-cluster-llm` (optional feature crate)
3. **Orchestration**
   - `shiplog-engine`
4. **CLI**
   - `shiplog`

For the first publish, `cargo publish --dry-run` may fail until foundation crates exist on crates.io. Publish foundation first, then dry-run the next layer.

### Binary releases (near-term)

Goal: reduce onboarding friction:

- build and upload binaries for Linux/macOS/Windows via CI
- attach to GitHub Releases
- keep `cargo install shiplog` as the universal fallback

---

## 9) Roadmap (near-term, confirmed)

**Now (v0.2.x)**

- Binary releases (CI)
- Local git ingest adapter
- GitLab + Jira adapters (at least basic PR/issue ingestion)

**Next (v0.3.x)**

- Multi-source merging with identity resolution
- Templateable packet rendering
- Better non-code work capture (manual lane ergonomics)

**Non-goals**

- Productivity scoring / dashboards
- Telemetry by default
- "AI writes your review" without receipts

---

## Appendix A - Pre-public release checklist

**Repo hygiene**

- [ ] `git status --porcelain` is empty
- [ ] secret scan (`git grep` patterns) returns nothing
- [ ] `.gitignore` covers `out/`, `target/`, `*.zip`, `*.db`, `*.sqlite*`, `.env*`

**Build gates**

- [ ] fmt/clippy/test gates green
- [ ] CI green on ubuntu + windows
- [ ] MSRV pinned (`rust-toolchain.toml`)

**Licensing**

- [ ] `LICENSE-MIT` and `LICENSE-APACHE` are canonical texts
- [ ] Cargo `license = "MIT OR Apache-2.0"` across publishable crates

**Share safety**

- [ ] `redaction.aliases.json` excluded from bundles
- [ ] public profile tests prove "no titles/urls/repo names leak"

---

## Appendix B - Troubleshooting

**"no matching package named shiplog-schema found" during publish/package**

- You are publishing out of order. Publish foundation crates first.

**Refresh does not preserve curation**

- Verify you are targeting the same run directory (`--run-dir out/<run_id>`).

**LLM clustering errors**

- Confirm you built with `--features llm`.
- Expect JSON-only responses; improve parser to strip code fences if needed.

---

*End.*
