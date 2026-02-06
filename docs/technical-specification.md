# ContextBuilder — Technical Specification

Status: Draft (Feb 2026)

## 1) Overview
ContextBuilder is a local-first documentation ingestion tool that turns a documentation URL into AI-ready artifacts: `llms.txt`, `llms-full.txt`, an Agent Skill file, and optional "Instructions/Rules" files, plus a portable local knowledge base (Markdown pages + TOC).  
It ships primarily as a cross-platform Rust CLI with an optional TUI, and it can optionally spawn a TypeScript MCP server that exposes the generated knowledge base to MCP-compatible clients. This project is a monorepo.

## 2) Goals and non-goals
Goals:
- Accept a docs URL and produce a portable knowledge base (Markdown + TOC + metadata).
- Prefer authoritative site-provided `llms.txt` / `llms-full.txt` when present and fall back to crawling otherwise.
- Generate agent-facing artifacts: Agent Skill file + Instructions/Rules outputs.
- Offer an MCP server (TypeScript) that exposes the KB via tools/resources using the official MCP TypeScript SDK, targeting **MCP protocol revision 2025-11-25**.
- Integrate LLM-assisted enrichment (Vercel AI SDK + OpenRouter) as a mandatory step in the core build pipeline.
- Support incremental KB updates: re-fetch upstream docs, diff, and add/update/prune pages.
- Provide persistent user configuration via `~/.contextbuilder/contextbuilder.toml`.
- Require an OpenRouter API key for all artifact generation (enrichment is always-on).

Non-goals:
- Being a general-purpose web crawler for the whole internet (ContextBuilder is docs-focused).
- Circumventing robots.txt, authentication gates, or paywalls.
- Acting as a hosted SaaS crawler by default (the design is local-first; a web UI can be added later).
- Custom output templates (fixed artifact formats for now).

## 3) Primary components
A. Rust CLI + optional TUI
- CLI argument parsing: `clap` for subcommands/flags.
- TUI framework: `ratatui` for interactive terminal UI flows.
- Progress reporting: `indicatif` progress bars/spinners.
- Logging: `tracing` crate for structured, leveled diagnostics with optional JSON output.

B. Ingestion + transformation engine (Rust)
- URL canonicalization, fetch, HTML parsing, readability extraction, and Markdown conversion.
- Platform adapters via a `PlatformAdapter` trait (Docusaurus, VitePress, GitBook, ReadTheDocs, generic fallback) to improve TOC extraction and reduce junk navigation.

C. Artifact store (filesystem)
- Primary truth is a directory on disk containing Markdown pages and generated artifacts.

D. Metadata + index store (Turso Embedded / libSQL — offline mode)
- Store crawl jobs, page metadata, hashes, dedupe indexes, link graph, and search indexes using Turso's embedded database approach in **offline mode** (no cloud sync required).
- The Rust CLI is the sole writer; the TypeScript MCP server opens the database in **read-only mode**.
- Optional future backend: AgentFS for bundling files + audit trails in a single portable SQLite file.

E. LLM-assist module (always-on, integrated into core pipeline)
- A mandatory stage of every build pipeline invocation — not optional or toggleable.
- Uses the OpenRouter provider for the Vercel AI SDK to generate summaries, best-practice rules, glossary entries, and instruction templates.
- Enrichment runs during artifact generation (TOC summarization, Skill/Rules synthesis).
- All enrichment is doc-grounded: prompts quote source sections and provenance is preserved.
- Requires a configured OpenRouter API key (env var referenced in config).

F. MCP server (TypeScript)
- A separate Node.js process, implemented using the official MCP TypeScript SDK.
- Targets **MCP protocol revision 2025-11-25**.
- Reads the generated KB directory and the Turso Embedded DB (read-only) for fast indexes.
- Logging: `evlog` for wide-event, structured errors.

## 4) Rust ↔ TypeScript integration contract
The Rust CLI and TypeScript MCP server are **independent processes** with no direct IPC. They share state exclusively through the filesystem and the Turso Embedded database:

- **Rust CLI (writer):** Produces the KB directory on disk (Markdown pages, `toc.json`, `manifest.json`, artifacts). Writes the Turso Embedded/libSQL database (`contextbuilder.db`) with crawl metadata, page indexes, and link graphs.
- **TypeScript MCP server (reader):** Reads the KB directory and opens `contextbuilder.db` in **read-only mode**. Never writes to the database or modifies KB files.
- **LLM enrichment bridge:** The Rust CLI shells out to a bundled TypeScript script (`packages/ts/openrouter-provider/`) that performs LLM calls and returns structured JSON to stdout. The Rust side parses the response and writes it into the appropriate artifacts. This is a one-shot subprocess call, not a persistent IPC channel. This bridge is invoked on every build — enrichment is not optional.
- **No process spawning between CLI and MCP server.** The user starts them independently (e.g., `contextbuilder add ...` then `contextbuilder mcp serve ...`).

## 5) Core workflow
1. **Input**
   - User provides a documentation root URL (CLI flag or TUI form).

2. **Discovery-first**
   - ContextBuilder checks for `/<site>/llms.txt` and `/<site>/llms-full.txt` before crawling, because the llms.txt standard is specifically intended to provide LLM-friendly context and pointers.

3. **Crawl + convert (fallback)**
   - If no llms files exist (or user forces rebuild), ContextBuilder crawls the docs scope, extracts content via the matched `PlatformAdapter`, and converts each page to clean Markdown.
   - It produces a TOC/index file and stores pages under a stable path scheme.

4. **Generate AI artifacts**
   - Emit one or more of:
     - `llms.txt` (index-oriented).
     - `llms-full.txt` (expanded, content-heavy).
     - `SKILL.md` (Agent Skill artifact, format aligned to the Agent Skills standard).
     - `rules.md`, `style.md`, `do_dont.md` (Instructions/Rules artifacts derived from docs sections and LLM-enriched).
   - Enrichment runs inline on every build: the pipeline invokes the OpenRouter provider to improve summaries, generate best-practice rules, and refine instruction outputs.

5. **Optional: start MCP server**
   - `contextbuilder mcp serve --kb <path>` spawns the TypeScript MCP server that exposes the KB via tools and resources.

6. **Incremental update**
   - `contextbuilder update --kb <path>` re-fetches upstream docs.
   - Compares page content hashes (stored in the `pages` table) to detect changed, new, and deleted pages.
   - Updates only changed/new pages; prunes pages that no longer exist upstream.
   - Re-generates affected artifacts (TOC, llms files, skill/rules if impacted pages changed).

## 6) Artifact formats and folder schema
KB root (example):
```
kb/<kb-id>/
  manifest.json           # tool version, source URL, timestamps, crawl policy, hashing, schema_version
  toc.json                # ordered structure: sections → pages; title, path, source URL
  docs/
    .../page-slug.md      # clean markdown
  artifacts/
    llms.txt
    llms-full.txt
    SKILL.md
    rules.md
    style.md
    do_dont.md
  indexes/
    contextbuilder.db     # Turso Embedded/libSQL (offline)
```

Notes:
- All structured data files use **JSON** format for consistency across Rust and TypeScript consumers.
- `manifest.json` includes a `schema_version` field (integer, starting at `1`). The tool checks this on load and warns/refuses if the KB was built with a newer schema version than the current tool supports. Migrations are applied automatically for older versions when possible.
- `llms.txt` and `llms-full.txt` are Markdown documents under the llms.txt convention (project title + short summary, then structured sections/links/content).
- ContextBuilder always preserves source URLs in frontmatter or adjacent metadata for traceability.

## 7) Configuration file
Location: `~/.contextbuilder/contextbuilder.toml`

```toml
# Global defaults
[defaults]
output_dir = "~/projects/kbs"          # Default KB output directory
crawl_depth = 3                        # Default max crawl depth
crawl_concurrency = 4                  # Default concurrent requests
mode = "auto"                          # auto | prefer-llms | crawl-only

[openrouter]
api_key_env = "OPENROUTER_API_KEY"     # Env var name (required; never store key directly)
default_model = "moonshotai/kimi-k2.5"

[crawl_policies]
include_patterns = []                  # Global include URL patterns
exclude_patterns = []                  # Global exclude URL patterns
respect_robots_txt = true
rate_limit_ms = 200                    # Minimum ms between requests to same host

# Known KBs registry
[[kbs]]
name = "nextjs-docs"
path = "~/projects/kbs/nextjs-docs"
source_url = "https://nextjs.org/docs"

[[kbs]]
name = "mcp-spec"
path = "~/projects/kbs/mcp-spec"
source_url = "https://modelcontextprotocol.io"
```

Precedence: CLI flags > config file > built-in defaults.

## 8) CLI interface (proposed)
- `contextbuilder add <url> --name <kb-name> --out <path> [--mode auto|prefer-llms|crawl-only]`
- `contextbuilder build --kb <path> --emit llms,llms-full,skill,rules`
- `contextbuilder update --kb <path> [--prune] [--force]`
- `contextbuilder list` (list known KBs from config)
- `contextbuilder tui` (interactive wizard)
- `contextbuilder mcp serve --kb <path> [--transport stdio|streamable-http] [--port 3100]`
- `contextbuilder config init` (create default `~/.contextbuilder/contextbuilder.toml`)
- `contextbuilder config show` (print resolved config)

## 9) TUI UX (proposed screens)
- "Create KB": URL input, scope rules, crawl depth, include/exclude patterns.
- "Crawl Preview": detected platform adapter + TOC preview + estimated page count.
- "Outputs": toggles for llms/skill/rules + local KB (enrichment runs automatically).
- "Update KB": select existing KB, preview diff (new/changed/deleted pages), confirm update.
- "Run MCP": show command and current status; copy config snippets.

## 10) MCP server surface (MCP 2025-11-25)

### Protocol alignment
- The server implements **MCP protocol revision 2025-11-25**.
- Capability negotiation declares `tools` (with `listChanged: true`) and `resources` (with `subscribe: true`, `listChanged: true`).
- Server info includes `name: "contextbuilder"` and `version` matching the package version.

### Transports
- **stdio** (default): The server reads JSON-RPC from stdin, writes to stdout. Best for IDE integrations (VS Code, Cursor, etc.). The Rust CLI can launch it as a subprocess.
- **Streamable HTTP** (optional): A single HTTP endpoint (e.g., `http://localhost:3100/mcp`) supporting POST for client→server messages and GET for server→client SSE streams. Supports session management via `MCP-Session-Id` header. Suitable for local LAN or multi-client scenarios.

### Tools (model-controlled)
Tools follow MCP tool naming conventions (`[a-zA-Z0-9_.-]`, max 128 chars) and declare both `inputSchema` and `outputSchema` for structured content.

| Tool name | Description | Key input params | Returns |
|-----------|-------------|------------------|---------|
| `kb_list` | List all available knowledge bases | — | Array of `{id, name, source_url, page_count, updated_at}` |
| `kb_get_toc` | Get the table of contents for a KB | `kb_id` | TOC JSON structure (sections → pages) |
| `kb_get_page` | Fetch a single page's Markdown content | `kb_id`, `path` | `{path, title, content, source_url, last_fetched}` |
| `kb_search` | Keyword search across a KB's pages | `kb_id`, `query`, `limit?` | Array of `{path, title, snippet, score}` |
| `kb_get_artifact` | Fetch a generated artifact | `kb_id`, `name` | Artifact content as text |

All tools return structured content in `structuredContent` and a serialized JSON fallback in a `TextContent` block for backwards compatibility.  
Tool execution errors use `isError: true` with actionable messages.

### Resources (application-controlled)
Resources use a custom URI scheme `contextbuilder://` and are exposed via resource templates:

| Resource template | Description | MIME type |
|-------------------|-------------|-----------|
| `contextbuilder://kb/{kb_id}/docs/{path}` | A single KB page | `text/markdown` |
| `contextbuilder://kb/{kb_id}/artifacts/{name}` | A generated artifact file | `text/markdown` or `text/plain` |
| `contextbuilder://kb/{kb_id}/toc` | The KB's table of contents | `application/json` |

Resources include annotations:
- `audience`: `["assistant"]` for pages/artifacts, `["user", "assistant"]` for TOC.
- `priority`: Configurable; TOC defaults to `1.0`, artifacts to `0.8`, pages to `0.5`.
- `lastModified`: ISO 8601 timestamp from page fetch or artifact generation time.

The server emits `notifications/resources/list_changed` when KBs are added, updated, or removed.  
Clients may `resources/subscribe` to individual resources and receive `notifications/resources/updated` when content changes.

### Error handling
- Standard JSON-RPC error codes: `-32002` (resource not found), `-32602` (invalid params), `-32603` (internal error).
- Tool execution errors return `isError: true` with descriptive messages for LLM self-correction.

## 11) Platform adapter architecture (Rust)
Platform detection and content extraction are handled by a `PlatformAdapter` trait, enabling clean extensibility:

```rust
pub trait PlatformAdapter: Send + Sync {
    /// Detect whether this adapter matches the given HTML document.
    fn detect(doc: &Html, url: &Url) -> Option<Self> where Self: Sized;

    /// Extract the table of contents / navigation structure.
    fn extract_toc(&self, doc: &Html) -> Vec<TocEntry>;

    /// Clean and extract the main content from an HTML document.
    fn extract_content(&self, doc: &Html) -> String;

    /// Extract page metadata (title, description, etc.).
    fn extract_metadata(&self, doc: &Html) -> PageMeta;

    /// Return the adapter's display name (e.g., "Docusaurus v3").
    fn name(&self) -> &str;
}
```

Built-in adapters (shipped with v1):
- `DocusaurusAdapter` — detects Docusaurus v2/v3 by meta tags and DOM structure.
- `VitePressAdapter` — detects VitePress by `<div id="VPContent">` and sidebar patterns.
- `GitBookAdapter` — detects GitBook by API markers and layout classes.
- `ReadTheDocsAdapter` — detects RTD by `readthedocs` meta/class markers.
- `GenericAdapter` — fallback using readability heuristics (always matches, lowest priority).

Adapter selection: iterate registered adapters in priority order; first one where `detect()` returns `Some` is used. `GenericAdapter` is always last.

## 12) Storage model (Turso Embedded — offline mode)
Turso Embedded / libSQL is used in **offline mode only** (no Turso cloud sync). The database file lives at `kb/<kb-id>/indexes/contextbuilder.db`.

Minimum tables (illustrative):

```sql
-- Knowledge base metadata
CREATE TABLE kb (
  id          TEXT PRIMARY KEY,
  name        TEXT NOT NULL,
  source_url  TEXT NOT NULL,
  created_at  TEXT NOT NULL,   -- ISO 8601
  updated_at  TEXT NOT NULL,   -- ISO 8601
  config_json TEXT             -- Serialized crawl/build config
);

-- Individual pages
CREATE TABLE pages (
  id           TEXT PRIMARY KEY,
  kb_id        TEXT NOT NULL REFERENCES kb(id),
  url          TEXT NOT NULL,
  path         TEXT NOT NULL,      -- Stable local path
  title        TEXT,
  content_hash TEXT NOT NULL,      -- SHA-256 of Markdown content
  fetched_at   TEXT NOT NULL,      -- ISO 8601
  status_code  INTEGER,
  content_len  INTEGER,
  UNIQUE(kb_id, path)
);

-- Link graph for crawl management
CREATE TABLE links (
  from_page_id TEXT NOT NULL REFERENCES pages(id),
  to_url       TEXT NOT NULL,
  kind         TEXT              -- "internal", "external", "anchor"
);

-- Crawl job history
CREATE TABLE crawl_jobs (
  id           TEXT PRIMARY KEY,
  kb_id        TEXT NOT NULL REFERENCES kb(id),
  started_at   TEXT NOT NULL,
  finished_at  TEXT,
  stats_json   TEXT              -- Pages fetched, errors, duration, etc.
);

-- LLM enrichment cache
CREATE TABLE enrichment_cache (
  id            TEXT PRIMARY KEY,
  kb_id         TEXT NOT NULL REFERENCES kb(id),
  artifact_type TEXT NOT NULL,     -- "skill", "rules", "summary", etc.
  prompt_hash   TEXT NOT NULL,
  model_id      TEXT NOT NULL,
  result_json   TEXT NOT NULL,
  created_at    TEXT NOT NULL,
  UNIQUE(kb_id, artifact_type, prompt_hash, model_id)
);
```

Access rules:
- Rust CLI: read-write (sole writer).
- TypeScript MCP server: read-only.
- Concurrent access: libSQL supports multiple readers + single writer; the MCP server never writes.

Optional future backend:
- AgentFS could serve as an alternate storage mode, storing everything in one portable SQLite file with copy-on-write isolation and built-in auditing.

## 13) LLM-assisted enrichment (always-on)
Enrichment is a mandatory stage of every `contextbuilder build`, `contextbuilder add`, and `contextbuilder update` pipeline. It is not toggleable.

Provider:
- OpenRouter via its Vercel AI SDK provider package (`packages/ts/openrouter-provider/`).
- The Rust CLI invokes a bundled TypeScript script as a subprocess, passing a JSON payload on stdin and reading structured JSON from stdout.

When enrichment runs:
- During TOC generation: LLM summarizes each section for richer `llms.txt` descriptions.
- During Skill generation: LLM synthesizes best-practice rules and skill entries from doc sections.
- During Instructions/Rules generation: LLM extracts do/don't patterns, style guides, and constraints.

Prerequisite:
- A valid OpenRouter API key must be configured (env var referenced in `~/.contextbuilder/contextbuilder.toml`). The CLI validates this on startup and exits with a clear error if missing.

Policies:
- "Doc-grounded" generation: enrichment prompts must quote or reference source sections, and the tool stores provenance (page path + section anchors) alongside generated entries.

Caching:
- Cache model outputs in the `enrichment_cache` table keyed by `(kb_id, artifact_type, prompt_hash, model_id)` to avoid re-spending tokens.
- Cache is invalidated when source page `content_hash` changes.

## 14) Logging and error handling
- **Rust CLI + libraries:** `tracing` crate with `tracing-subscriber` for structured output.
  - Default: human-friendly, colored terminal output.
  - Optional: JSON output via `--log-format json` for machine consumption.
  - Spans: one span per major operation (crawl, convert, build, update) with timing and context.
- **TypeScript MCP server:** `evlog` wide events with structured errors to produce one comprehensive event per operation (serve request, search, fetch).
- Both sides emit structured error types with error codes, context, and recovery hints.

## 15) Security considerations
- SSRF and local network protections: restrict fetch targets unless explicitly allowed (e.g., block `file://`, link-local IPs, and private ranges by default).
- Respect crawl politeness: rate limits, concurrency caps, and an explicit "I confirm I'm allowed to ingest these docs" toggle.
- Secrets: OpenRouter keys only read from env (referenced by var name in config); never written to artifacts or the database.
- Sanitization: strip scripts, avoid executing JS during parsing unless headless mode is explicitly enabled.
- MCP server security (per MCP 2025-11-25):
  - Streamable HTTP transport: validate `Origin` header, bind to localhost by default, implement session management via `MCP-Session-Id`.
  - Validate all tool inputs against declared `inputSchema`.
  - Rate-limit tool invocations.
  - Sanitize tool outputs before returning to clients.

## 16) Build system and workspace orchestration

### Rust workspace (Cargo)
A Cargo workspace defined at the repo root manages all Rust crates:

```toml
# Cargo.toml (root)
[workspace]
members = [
  "apps/cli",
  "apps/tui",
  "packages/rust/core",
  "packages/rust/shared",
  "packages/rust/discovery",
  "packages/rust/crawler",
  "packages/rust/markdown",
  "packages/rust/artifacts",
  "packages/rust/storage",
]
resolver = "3"
```

### TypeScript workspace (Bun)
A Bun workspace defined in the root `package.json` manages all TypeScript packages:

```json
{
  "name": "contextbuilder",
  "private": true,
  "workspaces": [
    "apps/mcp-server",
    "packages/ts/shared",
    "packages/ts/kb-reader",
    "packages/ts/openrouter-provider",
    "packages/schemas/*"
  ]
}
```

### Cross-language orchestration (Makefile)
A top-level `Makefile` provides unified commands across both ecosystems:

```makefile
.PHONY: build test lint fmt clean release

build:           ## Build all (Rust + TS)
	 cargo build --workspace
	 cd apps/mcp-server && bun install && bun run build

test:            ## Run all tests
	 cargo test --workspace
	 bun test

lint:            ## Lint all
	 cargo clippy --workspace -- -D warnings
	 bunx biome check .

fmt:             ## Format all
	 cargo fmt --all
	 bunx biome format --write .

clean:           ## Clean build artifacts
	 cargo clean
	 rm -rf node_modules apps/mcp-server/node_modules

release:         ## Build release binaries
	 cargo build --workspace --release
	 cd apps/mcp-server && bun run build
```

## 17) Compatibility and distribution
- Primary distribution is a single Rust binary + optional Node-based MCP server package.
- The MCP server is installed via npm (or bundled in releases) and invoked by the user or the CLI when requested.
- The Rust binary is statically linked where possible for maximum portability (Linux, macOS, Windows).
- KB format is versioned (`schema_version` in `manifest.json`); the tool supports forward-compatible reads and automatic migrations for older KB versions.
