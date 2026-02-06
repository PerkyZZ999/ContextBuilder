# ContextBuilder — Implementation Plan: Phase 2

## Ingestion Engine: Discovery, Crawling & Markdown Conversion

**Goal:** Build the complete ingestion pipeline — from URL input to clean Markdown pages on disk with TOC and manifest — so that `contextbuilder add <url>` produces a usable (pre-enrichment) KB.

**Estimated effort:** ~3 weeks

**Depends on:** Phase 1 (shared types, storage, CLI skeleton)

---

### Task 2.1 — Discovery module (`packages/rust/discovery`)

**Description:** Implement the llms.txt / llms-full.txt discovery logic that checks a site for existing LLM-friendly files before falling back to crawling.

**Deliverables:**
- [x] `discover(url: &Url) -> DiscoveryResult` — main entry point
- [x] `DiscoveryResult` enum: `Found { llms_txt: String, llms_full_txt: Option<String> }` | `NotFound`
- [x] HTTP fetch for `<origin>/llms.txt` and `<origin>/llms-full.txt`
  - Follows redirects (max 3)
  - Respects timeouts (configurable, default 10s)
  - User-Agent: `ContextBuilder/<version>`
- [x] Validation: check that response is valid Markdown, starts with `# ` (H1), has reasonable size (< 10MB)
- [x] Parse `llms.txt` format: extract H1 title, blockquote summary, sections with file lists
- [x] `LlmsTxtParsed` struct: `title`, `summary`, `sections: Vec<LlmsSection>`, `entries: Vec<LlmsEntry>` where each entry has `name`, `url`, `notes`
- [x] If `llms.txt` is found: extract all linked URLs as potential page sources
- [x] Tracing: spans for discovery attempt, timing, result

**Acceptance criteria:**
- Unit tests with fixture `llms.txt` files (from `fixtures/llms/`)
- Integration test against a mock HTTP server returning valid/invalid `llms.txt`
- Correctly parses the llms.txt format (H1, blockquote, sections, links)
- Returns `NotFound` gracefully on 404, timeout, or invalid content

---

### Task 2.2 — Platform adapters (`packages/rust/crawler` — adapter submodule)

**Description:** Implement the `PlatformAdapter` trait and the 5 built-in adapters for content extraction and TOC detection.

**Deliverables:**
- [x] `PlatformAdapter` trait definition (as specified in tech spec §11)
- [x] `AdapterRegistry` — holds registered adapters, iterates in priority order, returns first match
- [x] `DocusaurusAdapter`:
  - Detects by `<meta name="generator" content="Docusaurus">` or `data-docusaurus` attributes
  - Extracts TOC from sidebar JSON or `<nav>` with `.menu__list` classes
  - Extracts content from `<article>` or `.markdown` container
  - Strips footer, navbar, edit links
- [x] `VitePressAdapter`:
  - Detects by `<div id="VPContent">` or `.VPDoc` class
  - Extracts TOC from `.VPSidebar` navigation
  - Extracts content from `.vp-doc` container
- [x] `GitBookAdapter`:
  - Detects by GitBook-specific meta tags or API-based markers
  - Extracts TOC from left sidebar navigation
  - Extracts content from main content area
- [x] `ReadTheDocsAdapter`:
  - Detects by `readthedocs` class/meta markers or `_static/` asset paths
  - Extracts TOC from `<nav>` with `.wy-nav-side` or `.toctree` classes
  - Extracts content from `.document` or `[role="main"]`
- [x] `GenericAdapter`:
  - Always matches (lowest priority)
  - Uses readability heuristics to find main content
  - Extracts TOC from heading structure (H1→H6)
  - Strips nav, header, footer, sidebar, script, style elements

**Acceptance criteria:**
- Each adapter has fixture HTML files in `fixtures/html/` for its target platform
- Unit tests: `detect()` correctly identifies the right adapter for each fixture
- Unit tests: `extract_content()` produces clean text without nav/chrome junk
- Unit tests: `extract_toc()` returns valid `TocEntry` trees
- `GenericAdapter` succeeds on arbitrary HTML pages as fallback

---

### Task 2.3 — Crawler engine (`packages/rust/crawler`)

**Description:** Implement the concurrent, scope-aware web crawler that fetches docs pages, respects politeness rules, and stores results.

**Deliverables:**
- [x] `Crawler` struct with builder pattern for config (depth, concurrency, include/exclude, rate limit)
- [x] `Crawler::crawl(start_url, storage) -> CrawlResult`:
  - BFS/DFS traversal starting from `start_url`
  - Concurrent fetching with `tokio::Semaphore` for concurrency cap
  - Rate limiting per host (configurable `rate_limit_ms`)
  - Respects `robots.txt` (fetch and parse using `texting_robots` or manual parser)
  - URL deduplication (normalized URLs, fragment stripping)
  - Scope filtering: only follow links within the docs scope (same path prefix or matching include patterns)
  - Exclude pattern support (skip URLs matching exclude globs)
  - Depth cap enforcement
- [x] For each fetched page:
  - Run through `AdapterRegistry` to select the best platform adapter
  - Extract content, metadata, and TOC entries
  - Store raw HTML in memory (not persisted) for adapter processing
  - Compute SHA-256 content hash of the extracted Markdown
  - Write page record to storage (`upsert_page`)
  - Write link records to storage (`insert_link`)
- [x] `CrawlResult` struct: pages fetched, pages skipped, errors, duration, adapter used
- [x] Crawl job tracking: create `crawl_jobs` record on start, update on finish
- [x] Resumable crawls: if a crawl is interrupted, re-running skips already-fetched pages (by URL dedup in DB)
- [x] SSRF protection: block `file://`, `data:`, link-local IPs, and private IP ranges
- [x] Tracing: span per crawl job, span per page fetch, structured error events

**Acceptance criteria:**
- Integration test with mock HTTP server serving a small doc site (5-10 pages)
- Respects concurrency limits (verify with timing)
- Respects depth cap (doesn't crawl beyond configured depth)
- Correctly deduplicates URLs
- Stores all pages and links in the database
- SSRF protection rejects `file://` and private IPs
- Crawl job record is created and updated

---

### Task 2.4 — HTML-to-Markdown conversion (`packages/rust/markdown`)

**Description:** Implement high-fidelity HTML-to-Markdown conversion with cleanup passes.

**Deliverables:**
- [x] `convert(html: &str, adapter: &dyn PlatformAdapter) -> ConvertResult`:
  - Use adapter's `extract_content()` to get clean HTML
  - Convert HTML to Markdown using `htmd`
  - Post-processing cleanup pipeline
- [x] Cleanup passes (applied in order):
  1. Normalize heading levels (ensure single H1, adjust hierarchy)
  2. Clean up excessive blank lines (max 2 consecutive)
  3. Fix code block language hints (detect from class names)
  4. Remove leftover HTML tags/attributes
  5. Normalize link URLs (resolve relative to source URL)
  6. Strip inline styles and class attributes
  7. Preserve and clean up tables
- [x] `ConvertResult` struct: `markdown: String`, `title: String`, `word_count: usize`
- [x] Frontmatter generation: add YAML frontmatter with `source_url`, `title`, `fetched_at`

**Acceptance criteria:**
- Golden file tests: convert `fixtures/html/*.html` → compare against `fixtures/markdown/*.md`
- Code blocks preserve language tags and content
- Tables render correctly in Markdown
- No leftover HTML junk in output
- Links are properly resolved (no broken relative URLs)
- Headings are properly hierarchical

---

### Task 2.5 — TOC generation & KB assembly (`packages/rust/core` — partial)

**Description:** Implement the TOC builder and the KB directory assembly logic that writes the final KB structure to disk.

**Deliverables:**
- [x] `TocBuilder::build(pages: &[PageMeta], adapter_toc: &[TocEntry]) -> Toc`:
  - Merge adapter-extracted TOC with discovered pages
  - Generate stable paths for each page (URL → slug conversion)
  - Build hierarchical section structure
  - Order sections/pages by adapter-provided order (or alphabetical fallback)
- [x] `Toc` struct serializable to `toc.json`
- [x] `KbAssembler::assemble(config, pages, toc) -> KbPath`:
  - Create `kb/<kb-id>/` directory structure
  - Write `manifest.json` (with `schema_version: 1`, source URL, timestamps, config)
  - Write `toc.json`
  - Write `docs/**/*.md` (one file per page, using stable paths from TOC)
  - Create `artifacts/` directory (empty, populated in Phase 3)
  - Create `indexes/` directory with `contextbuilder.db` (moved/copied from temp location)
- [x] `manifest.json` generation: tool version, source URL, crawl timestamps, page count, `schema_version`
- [x] Slug generation: URL path → kebab-case file path, handle collisions
- [x] KB registration: add entry to `~/.contextbuilder/contextbuilder.toml` `[[kbs]]` array

**Acceptance criteria:**
- Integration test: given mock pages and TOC entries, assembles a valid KB directory
- `manifest.json` is valid per schema
- `toc.json` is valid per schema and contains all pages
- All Markdown files are in correct locations matching `toc.json` paths
- KB appears in config file's `[[kbs]]` registry after assembly
- Idempotent: assembling the same data twice produces identical output (except timestamps)

---

### Task 2.6 — Wire `contextbuilder add` end-to-end (`apps/cli` + `packages/rust/core`)

**Description:** Connect all Phase 2 modules into the `contextbuilder add` CLI command for the complete ingestion flow (without enrichment — that's wired in Phase 3).

**Deliverables:**
- [x] `core::add_kb(url, name, output_path, config) -> Result<KbPath>`:
  1. Run discovery (Task 2.1)
  2. If found: parse llms.txt, extract URLs, fetch each linked page
  3. If not found: run crawler (Task 2.3)
  4. Convert all pages to Markdown (Task 2.4)
  5. Build TOC (Task 2.5)
  6. Assemble KB directory (Task 2.5)
  7. Register KB in config
  8. Return KB path
- [x] CLI `add` subcommand calls `core::add_kb` with resolved config
- [x] Progress reporting with `indicatif`: spinner during discovery, progress bar during crawl, summary on completion
- [x] Structured output: print KB path, page count, source URL, time elapsed
- [x] Error handling: clear messages for network failures, empty results, scope mismatches

**Acceptance criteria:**
- `contextbuilder add https://docs.example.com --name example-docs` produces a complete KB directory
- KB directory contains `manifest.json`, `toc.json`, and populated `docs/` folder
- Discovery-first: when `llms.txt` exists, it's used without crawling
- Crawl fallback: when `llms.txt` is missing, crawler runs and produces the KB
- Progress bar shows during crawl operations
- Final summary shows page count and elapsed time

---

### Phase 2 completion criteria

All of the following must be true:
1. `contextbuilder add <url>` works end-to-end for a real documentation site
2. Discovery correctly detects and uses `llms.txt` when available
3. Crawler correctly fetches and converts pages when discovery fails
4. Platform adapters produce clean content for at least Docusaurus and VitePress sites
5. KB directory is well-formed: valid manifest, valid TOC, all pages present
6. Storage contains accurate page metadata, content hashes, and link graph
7. All tests pass including golden file tests for Markdown conversion
8. `make test` and `make lint` pass
