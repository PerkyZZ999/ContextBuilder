# ContextBuilder — Implementation Plan: Phase 3

## Artifact Generation & LLM Enrichment

**Goal:** Generate all output artifacts (`llms.txt`, `llms-full.txt`, `SKILL.md`, rules/style/do-dont files) using always-on LLM enrichment, and implement the `contextbuilder update` incremental refresh flow.

**Estimated effort:** ~3 weeks

**Depends on:** Phase 2 (complete ingestion pipeline, KB on disk with pages + TOC)

---

### Task 3.1 — OpenRouter provider bridge (`packages/ts/openrouter-provider`)

**Description:** Build the TypeScript subprocess that handles all LLM interactions via OpenRouter, invoked by the Rust core on every build.

**Deliverables:**
- [x] `enrichment-bridge` — standalone Bun entrypoint (`packages/ts/openrouter-provider/src/bridge.ts`)
  - Receives enrichment requests via stdin (JSON-lines protocol)
  - Responds via stdout (JSON-lines protocol)
  - Protocol messages:
    - Request: `{ "type": "enrich", "id": string, "task": EnrichmentTask }`
    - Response: `{ "type": "result", "id": string, "result": EnrichmentResult }` | `{ "type": "error", "id": string, "error": string }`
    - Control: `{ "type": "shutdown" }` → graceful exit
- [x] `EnrichmentTask` variants:
  - `summarize_page` — generate concise summary for a page
  - `generate_description` — one-line description for llms.txt entry
  - `generate_skill_md` — produce SKILL.md content (Agent Skills spec format)
  - `generate_rules` — produce rules.md content
  - `generate_style` — produce style.md content
  - `generate_do_dont` — produce do_dont.md content
  - `generate_llms_txt` — produce llms.txt from TOC + summaries
  - `generate_llms_full_txt` — produce llms-full.txt from all page content
- [x] Use `@openrouter/ai-sdk-provider` + Vercel AI SDK (`ai` package)
- [x] Model selection: configurable via `contextbuilder.toml` `[enrichment]` section (default: cost-efficient model)
- [x] Retry logic: exponential backoff on 429/5xx (max 3 retries per request)
- [x] Token counting: track input/output tokens per request, report totals
- [x] Structured output: use `zod` schemas for each enrichment result to ensure well-formed responses
- [x] evlog logging: structured events for each enrichment call (model, tokens, latency, task type)

**Acceptance criteria:**
- Bridge can be spawned, receives requests, returns results, shuts down cleanly
- Unit tests with mocked OpenRouter responses
- Retry logic handles 429 responses correctly
- Token usage is tracked and reported
- evlog produces valid structured log entries

---

### Task 3.2 — Enrichment orchestrator (`packages/rust/core` — enrichment module)

**Description:** Implement the Rust-side orchestrator that spawns the TS bridge, sends enrichment tasks, and caches results.

**Deliverables:**
- [x] `EnrichmentOrchestrator::new(config, storage) -> Self`
- [x] `EnrichmentOrchestrator::run(pages: &[PageMeta], toc: &Toc) -> EnrichmentResults`:
  1. Spawn the TS bridge subprocess (`bun run packages/ts/openrouter-provider/src/bridge.ts`)
  2. Validate API key availability (fail with clear error if missing)
  3. Check `enrichment_cache` table for each page's content_hash
  4. For cache misses: send enrichment tasks to bridge
  5. Store results in `enrichment_cache` (keyed by content_hash + task_type)
  6. Collect all results, send `shutdown` to bridge
  7. Return aggregated `EnrichmentResults`
- [x] Cache semantics:
  - Cache key: `(content_hash, task_type)` — if page content hasn't changed, reuse cached enrichment
  - Cache invalidation: on content_hash change (page was re-fetched with different content)
  - `enrichment_cache` table fields: `content_hash`, `task_type`, `result_json`, `model_used`, `tokens_in`, `tokens_out`, `created_at`
- [x] Progress reporting: `indicatif` progress bar for enrichment tasks (N of M pages)
- [x] Error handling: if bridge crashes, collect partial results and report which pages failed
- [x] Tracing: span for enrichment run, per-page timing

**Acceptance criteria:**
- Spawns bridge, sends tasks, receives results, stops bridge
- Cache hits skip the bridge call (verify with test)
- Cache misses trigger bridge calls and populate cache
- Partial failure: if 2/10 pages fail, the other 8 results are still usable
- API key validation produces a clear, actionable error message
- Progress bar updates during enrichment

---

### Task 3.3 — Artifact generators (`packages/rust/core` — artifacts module)

**Description:** Implement generators for each artifact type, using enrichment results to produce the final output files.

**Deliverables:**
- [x] `LlmsTxtGenerator::generate(toc: &Toc, summaries: &HashMap<PageId, Summary>) -> String`:
  - Follow [llmstxt.org](https://llmstxt.org) format: H1 title, blockquote summary, sections with `- [name](url): description` entries
  - Use enrichment-provided descriptions for each entry
  - Sections derived from TOC hierarchy
- [x] `LlmsFullTxtGenerator::generate(toc: &Toc, pages: &[PageContent]) -> String`:
  - Single concatenated Markdown file
  - Each page separated by H2 heading + source URL comment
  - Full page content included (no truncation)
  - Table of contents at the top linking to sections
- [x] `SkillMdGenerator::generate(enrichment: &EnrichmentResults, kb: &KbManifest) -> String`:
  - Follow [Agent Skills specification](https://agentskills.io/specification) format
  - YAML frontmatter: `name`, `version`, `description`, `author`, `license`
  - Sections: Overview, Capabilities, Usage, Configuration, Examples
  - Content sourced from enrichment's `generate_skill_md` task
- [x] `RulesGenerator::generate(enrichment: &EnrichmentResults) -> String`:
  - Coding rules and conventions extracted from the documentation
  - Structured as actionable directives
- [x] `StyleGenerator::generate(enrichment: &EnrichmentResults) -> String`:
  - API style and naming conventions
  - Code formatting preferences documented in the source
- [x] `DoDontGenerator::generate(enrichment: &EnrichmentResults) -> String`:
  - Do/Don't pairs extracted from docs
  - Common pitfalls and best practices
- [x] All generators:
  - Accept enrichment results (never call LLM directly)
  - Produce deterministic output for the same inputs
  - Include provenance comments (`<!-- Generated by ContextBuilder vX.Y.Z from <source_url> -->`)

**Acceptance criteria:**
- `llms.txt` output validates against the llms.txt format spec
- `llms-full.txt` contains all page content in order
- `SKILL.md` follows Agent Skills spec with valid frontmatter
- All 6 artifact files are generated from enrichment results
- Provenance comments are present in all outputs
- Golden file tests for each generator against fixture inputs

---

### Task 3.4 — KB assembly with artifacts (`packages/rust/core` — assemble)

**Description:** Extend the Phase 2 KB assembler to include the artifacts directory population.

**Deliverables:**
- [x] After enrichment and artifact generation, write all files to `kb/<kb-id>/artifacts/`:
  - `llms.txt`
  - `llms-full.txt`
  - `skill.md`
  - `rules.md`
  - `style.md`
  - `do_dont.md`
- [x] Update `manifest.json`:
  - `artifacts` section listing generated files with checksums
  - `enrichment` section with model used, total tokens, timestamp
- [x] Atomic writes: write to temp location, then rename (avoid partial artifacts on crash)
- [x] Post-assembly validation: check all expected files exist, manifest is consistent

**Acceptance criteria:**
- KB directory contains `artifacts/` with all 6 files
- `manifest.json` accurately lists all artifacts with correct checksums
- Atomic writes: interrupted build doesn't leave partial artifacts
- Re-running assembly overwrites artifacts cleanly

---

### Task 3.5 — Incremental update flow (`contextbuilder update`)

**Description:** Implement the `contextbuilder update` command for refreshing an existing KB.

**Deliverables:**
- [x] `core::update_kb(kb_path, config) -> UpdateResult`:
  1. Load existing `manifest.json` and `toc.json`
  2. Re-run discovery / crawl with same config as original build
  3. Compare new page content hashes against stored hashes
  4. Identify: new pages, removed pages, changed pages, unchanged pages
  5. For changed/new pages only: update storage, re-convert Markdown
  6. Re-run enrichment (cache hits for unchanged pages, new calls for changed/new)
  7. Re-generate all artifacts (since even one page change can affect llms.txt/llms-full.txt)
  8. Re-assemble KB directory, update manifest timestamps
- [x] `UpdateResult` struct: pages_added, pages_removed, pages_changed, pages_unchanged, enrichment_cache_hits, enrichment_cache_misses, duration
- [x] CLI `update` subcommand:
  - `contextbuilder update <kb-name-or-path>`
  - Progress output: "Checking 47 pages... 3 changed, 1 new, 0 removed"
  - Summary on completion
- [x] Efficient diffing: only re-fetch pages whose `Last-Modified` or `ETag` headers indicate changes (where available), otherwise compare content hashes

**Acceptance criteria:**
- Updating an unchanged KB: no re-enrichment (all cache hits), fast completion
- Updating with 1 changed page: only that page is re-enriched, artifacts are regenerated
- Adding new pages: discovered new pages are added to TOC and artifacts
- Removing pages: pages no longer in source are marked as removed in TOC
- `manifest.json` timestamps are updated correctly

---

### Task 3.6 — Wire `contextbuilder add` with enrichment (full pipeline)

**Description:** Integrate the enrichment orchestrator into the `add` pipeline so every new KB build includes always-on LLM enrichment.

**Deliverables:**
- [x] Update `core::add_kb` (from Phase 2 Task 2.6) to include:
  1. After crawl + markdown conversion...
  2. Run `EnrichmentOrchestrator` for all pages
  3. Run artifact generators with enrichment results
  4. Assemble KB with artifacts
- [x] API key check at CLI startup: if `OPENROUTER_API_KEY` is not set and not in config, print actionable error and exit before crawling
- [x] Cost estimation (optional UX): before enrichment, print estimated token count and model, allow user to confirm
- [x] Full pipeline timing: report crawl time, enrichment time, assembly time separately

**Acceptance criteria:**
- `contextbuilder add <url>` produces a KB with both docs and artifacts
- Missing API key → clear error message before any work is done
- Enrichment progress is visible (progress bar)
- End-to-end test: add a small fixture site, verify all 6 artifacts are generated
- `make test` passes with all new tests

---

### Phase 3 completion criteria

All of the following must be true:
1. `contextbuilder add <url>` produces a complete KB with all 6 artifacts
2. `contextbuilder update <kb>` incrementally refreshes a KB efficiently
3. Enrichment cache correctly avoids redundant LLM calls
4. OpenRouter bridge handles errors gracefully (retries, partial failure reporting)
5. All artifact formats match their respective specifications
6. API key validation occurs before any network activity
7. Token usage is tracked and reported in manifest and logs
8. All tests pass including golden file tests for artifact generation
9. `make test` and `make lint` pass
