# ContextBuilder — Implementation Plan: Phase 4

## MCP Server, TUI & End-to-End Polish

**Goal:** Ship the MCP server (TypeScript, MCP 2025-11-25) exposing the KB via tools and resources, the TUI for interactive workflows, and perform end-to-end integration testing across the full pipeline.

**Estimated effort:** ~3 weeks

**Depends on:** Phase 3 (complete KB with artifacts available on disk + in DB)

---

### Task 4.1 — KB reader package (`packages/ts/kb-reader`)

**Description:** TypeScript library that reads a KB directory and its Turso Embedded DB (read-only) to provide typed access for the MCP server.

**Deliverables:**
- [x] `KbReader` class:
  - `static async open(kbPath: string): Promise<KbReader>` — opens manifest, validates `schema_version`, opens DB read-only
  - `getManifest(): KbManifest` — returns parsed `manifest.json`
  - `getToc(): Toc` — returns parsed `toc.json`
  - `getPage(path: string): Promise<PageContent>` — reads Markdown file from `docs/` + metadata from DB
  - `getArtifact(name: string): Promise<string>` — reads artifact file from `artifacts/`
  - `search(query: string, limit?: number): Promise<SearchResult[]>` — FTS5 search via DB
  - `listKbs(): Promise<KbSummary[]>` — reads all KBs from `~/.contextbuilder/contextbuilder.toml`
  - `close(): Promise<void>` — close DB connection
- [x] Use `@libsql/client` for read-only DB access
- [x] Use zod schemas from `packages/schemas/` for validation
- [x] All results typed with TS interfaces matching the JSON schemas
- [x] Error handling: clear errors for missing files, schema version mismatch, DB corruption
- [x] evlog: structured log events for each read operation

**Acceptance criteria:**
- Unit tests with fixture KB directory (valid manifest, toc, pages, artifacts, DB)
- `search()` returns ranked results from FTS5
- Schema version mismatch throws descriptive error
- Read-only: no writes to DB or filesystem
- All return types match zod schemas

---

### Task 4.2 — MCP server core (`apps/mcp-server`)

**Description:** Implement the MCP server using `@modelcontextprotocol/sdk`, targeting protocol revision 2025-11-25 with stdio and Streamable HTTP transports.

**Deliverables:**
- [x] Server setup:
  - `name: "contextbuilder"`, `version` from `package.json`
  - Capabilities: `tools` (with `listChanged: true`), `resources` (with `subscribe: true`, `listChanged: true`)
  - Use `@modelcontextprotocol/sdk` `Server` class
- [x] **stdio transport** (default):
  - Read JSON-RPC from stdin, write to stdout
  - Launched via `bun run apps/mcp-server/src/index.ts --kb <path>` or `contextbuilder mcp serve`
- [x] **Streamable HTTP transport** (optional):
  - Single endpoint (e.g., `http://localhost:3100/mcp`)
  - POST for client→server, GET for SSE server→client
  - `MCP-Session-Id` header for session management
  - Bind to localhost by default, `Origin` header validation
- [x] Server lifecycle:
  - Parse `--kb <path>` arg or read all KBs from config
  - Initialize `KbReader` for each KB
  - Register tools and resource templates
  - Handle graceful shutdown (SIGINT/SIGTERM)
- [x] evlog logging: wide events for each RPC call (method, KB id, duration, result size)

**Acceptance criteria:**
- Server starts, completes capability negotiation, and responds to `initialize` → `initialized`
- stdio transport works with `@modelcontextprotocol/sdk` client in tests
- Streamable HTTP transport accepts connections on configured port
- Graceful shutdown closes DB connections and exits cleanly
- Rejects unsupported protocol versions with proper error

---

### Task 4.3 — MCP tools implementation (`apps/mcp-server`)

**Description:** Implement all 5 MCP tools as specified in tech spec §10.

**Deliverables:**
- [x] `kb_list` tool:
  - No required inputs
  - Returns array of `{id, name, source_url, page_count, updated_at}` for all registered KBs
  - Both `structuredContent` and `TextContent` fallback
- [x] `kb_get_toc` tool:
  - Input: `{ kb_id: string }`
  - Returns TOC JSON structure (sections → pages)
  - `inputSchema` and `outputSchema` declared
- [x] `kb_get_page` tool:
  - Input: `{ kb_id: string, path: string }`
  - Returns `{path, title, content, source_url, last_fetched}`
  - Error handling: `-32002` if page not found
- [x] `kb_search` tool:
  - Input: `{ kb_id: string, query: string, limit?: number }`
  - Returns array of `{path, title, snippet, score}`
  - Uses FTS5 via `KbReader.search()`
  - Default limit: 10, max limit: 50
- [x] `kb_get_artifact` tool:
  - Input: `{ kb_id: string, name: string }`
  - Returns artifact content as text
  - Valid names: `llms.txt`, `llms-full.txt`, `skill.md`, `rules.md`, `style.md`, `do_dont.md`
  - Error handling: `-32002` if artifact not found
- [x] Common patterns:
  - All tools validate inputs against `inputSchema` (via zod)
  - All tools return `structuredContent` + `TextContent` fallback
  - Execution errors use `isError: true` with actionable messages
  - Tool execution tracing via evlog

**Acceptance criteria:**
- Each tool has integration tests using the MCP SDK client
- Invalid `kb_id` returns proper error code
- `kb_search` returns relevant results ranked by score
- `kb_get_artifact` rejects unknown artifact names
- All tools return both `structuredContent` and `TextContent`

---

### Task 4.4 — MCP resources implementation (`apps/mcp-server`)

**Description:** Implement resource templates, annotations, subscriptions, and change notifications as specified in tech spec §10.

**Deliverables:**
- [x] Resource templates (exposed via `resources/templates/list`):
  - `contextbuilder://kb/{kb_id}/docs/{path}` → single page (MIME: `text/markdown`)
  - `contextbuilder://kb/{kb_id}/artifacts/{name}` → artifact file (MIME: `text/markdown` or `text/plain`)
  - `contextbuilder://kb/{kb_id}/toc` → TOC (MIME: `application/json`)
- [x] Resource annotations on every resource:
  - `audience`: `["assistant"]` for pages/artifacts, `["user", "assistant"]` for TOC
  - `priority`: TOC → `1.0`, artifacts → `0.8`, pages → `0.5`
  - `lastModified`: ISO 8601 timestamp from page fetch or artifact generation
- [x] `resources/read` handler:
  - Parse URI scheme, extract KB ID + path/name
  - Return content with correct MIME type
  - Error `-32002` for unknown resources
- [x] `resources/list` handler:
  - List all pages + artifacts + TOC for all loaded KBs
  - Return with annotations
- [x] Subscriptions:
  - Track subscribed resource URIs per client session
  - `resources/subscribe` and `resources/unsubscribe` handlers
  - Emit `notifications/resources/updated` when content changes (webhook from file watcher or manual trigger)
- [x] `notifications/resources/list_changed`:
  - Emit when KBs are added, updated, or removed
  - File watcher on KB root directories (optional, or triggered by CLI commands)

**Acceptance criteria:**
- Resource templates are discoverable via `resources/templates/list`
- `resources/read` returns correct content for each URI pattern
- Annotations are present on every resource
- Subscription lifecycle works: subscribe → update → notification → unsubscribe
- Invalid URI patterns return proper error codes

---

### Task 4.5 — CLI `mcp serve` command (`apps/cli`)

**Description:** Wire the Rust CLI to spawn the MCP server as a managed subprocess.

**Deliverables:**
- [x] `contextbuilder mcp serve` subcommand:
  - Options: `--transport stdio|http` (default: `stdio`), `--port <n>` (default: 3100 for HTTP), `--kb <path>` (optional, all KBs if omitted)
  - Spawns `bun run apps/mcp-server/src/index.ts` with correct args
  - Forwards stdin/stdout in stdio mode
  - Prints server URL in HTTP mode
  - Handles SIGINT: forwards signal to child, waits for clean exit
- [x] `contextbuilder mcp config` subcommand:
  - Prints MCP client configuration JSON snippets for popular tools:
    - VS Code (Copilot) `mcp.json` format
    - Claude Desktop `claude_desktop_config.json` format
    - Cursor settings format
  - Includes the correct command and args for the user's installation path
- [x] Validation: check that Bun is available, MCP server package is built, KB paths exist

**Acceptance criteria:**
- `contextbuilder mcp serve` starts the MCP server and handles lifecycle
- `contextbuilder mcp config` outputs valid, copy-pasteable configuration
- SIGINT cleanly stops both the CLI and the MCP server subprocess
- Missing Bun or unbuilt server produces clear error messages

---

### Task 4.6 — TUI interactive interface (`apps/tui`)

**Description:** Implement the ratatui-based TUI for interactive KB management.

**Deliverables:**
- [x] TUI framework setup:
  - `ratatui` + `crossterm` backend
  - Event loop with key input handling
  - Tab/screen navigation
- [x] **"Create KB" screen:**
  - URL input field
  - Scope rules editor (include/exclude patterns)
  - Crawl depth slider
  - Name input with auto-suggestion from URL
  - "Start" button → runs full `add_kb` pipeline with live progress
- [x] **"Crawl Preview" screen:**
  - Shows detected platform adapter
  - TOC preview (tree view)
  - Estimated page count
  - Confirm/cancel before full crawl
- [x] **"Outputs" screen:**
  - Toggle display for generated artifacts
  - Preview artifact content (scrollable)
  - KB stats: page count, total size, enrichment token usage
  - Enrichment runs automatically (no toggle — always-on)
- [x] **"Update KB" screen:**
  - Select from existing KBs (list)
  - Preview diff: new/changed/deleted pages
  - Confirm update → runs `update_kb` pipeline with progress
- [x] **"Run MCP" screen:**
  - Start/stop MCP server
  - Show transport type + port/status
  - Copy config snippet button (copies to clipboard)
- [x] Common TUI components:
  - Status bar with KB count, current operation, errors
  - Help overlay (keybindings)
  - Log viewer panel (tracing output)
  - Confirmation dialogs

**Acceptance criteria:**
- TUI launches, navigates between screens, and exits cleanly
- "Create KB" successfully runs the full pipeline with visible progress
- "Update KB" shows accurate diff preview
- "Run MCP" starts and stops the server
- Keyboard navigation is consistent and discoverable
- All screens render correctly in 80x24 minimum terminal size

---

### Task 4.7 — End-to-end integration tests

**Description:** Build comprehensive integration tests that exercise the entire pipeline from URL to MCP-served KB.

**Deliverables:**
- [x] **Fixture doc site**: a small static site (5-10 pages) served by a local HTTP server during tests
  - Includes valid `llms.txt` for discovery path testing
  - Covers at least 2 platform layouts (Docusaurus + generic)
- [x] **E2E test: add with discovery**
  1. Start fixture server with `llms.txt`
  2. `contextbuilder add <fixture-url> --name test-kb`
  3. Verify KB directory structure (manifest, toc, docs, artifacts)
  4. Verify all 6 artifacts are present
  5. Start MCP server, query tools, verify responses
- [x] **E2E test: add with crawl fallback**
  1. Start fixture server without `llms.txt`
  2. `contextbuilder add <fixture-url> --name test-crawl`
  3. Verify crawler found all pages
  4. Verify KB completeness
- [x] **E2E test: update flow**
  1. Add a KB
  2. Modify the fixture server (change 1 page, add 1 page)
  3. `contextbuilder update test-kb`
  4. Verify diff is correct (1 changed, 1 new, 0 removed)
  5. Verify enrichment cache was used for unchanged pages
- [x] **E2E test: MCP tools + resources**
  1. Start MCP server over stdio
  2. Initialize MCP client
  3. Call all 5 tools, verify responses
  4. Read all resource templates, verify content
  5. Subscribe to a resource, trigger update, verify notification
- [x] **CI integration**: all E2E tests run in `make test` (behind a feature flag or separate test target `make test-e2e`)
- [x] Mock OpenRouter responses for enrichment (no real API calls in CI)

**Acceptance criteria:**
- All E2E tests pass in CI without real network access (mock HTTP + mock OpenRouter)
- Discovery path and crawl path both produce valid KBs
- Update path correctly handles additions, changes, and removals
- MCP server responds correctly to all tool and resource queries
- Tests complete in < 60 seconds

---

### Task 4.8 — Documentation & release prep

**Description:** Final documentation, README, and release preparation.

**Deliverables:**
- [x] **README.md** (repo root):
  - Project overview and motivation
  - Installation instructions (from releases, from source via `cargo build`)
  - Quick start: `contextbuilder add <url>`, `contextbuilder mcp serve`
  - Configuration guide (`~/.contextbuilder/contextbuilder.toml`)
  - MCP integration setup (VS Code, Claude Desktop, Cursor snippets)
  - Architecture diagram (Mermaid)
  - Contributing section
- [x] **CHANGELOG.md**: initial v0.1.0 entry
- [x] **LICENSE**: confirm license file exists
- [x] **Release artifacts**:
  - GitHub Actions workflow for building release binaries (Linux amd64/arm64, macOS amd64/arm64, Windows amd64)
  - MCP server published as npm package (or bundled in release archive)
  - Release tarball structure: binary + MCP server + README
- [x] **MCP server package.json**: correct `name`, `version`, `bin`, `main` fields for distribution
- [x] Update `package.json` scripts to match final build commands

**Acceptance criteria:**
- README is clear enough for a new user to install and use ContextBuilder in < 5 minutes
- `make release` produces distributable binaries
- GitHub Actions CI pipeline passes on push and PR
- MCP client config snippets in README work when copy-pasted

---

### Phase 4 completion criteria

All of the following must be true:
1. MCP server starts via `contextbuilder mcp serve` and responds correctly to all 5 tools
2. MCP resources are accessible via `contextbuilder://` URIs with annotations
3. MCP server works over both stdio and Streamable HTTP transports
4. TUI launches and all 5 screens are functional
5. `contextbuilder mcp config` outputs valid config snippets for VS Code, Claude Desktop, and Cursor
6. All E2E integration tests pass (discovery, crawl, update, MCP)
7. README and documentation are complete
8. CI/CD pipeline builds and tests on all target platforms
9. `make build`, `make test`, and `make lint` all pass
10. The full pipeline works: URL → KB → MCP → AI client can query docs
