# ContextBuilder — Tech Stack

Status: Draft (Feb 2026)

## Rust Ecosystem

| Category | Technology | Purpose |
|----------|-----------|---------|
| Language | **Rust** (Edition 2024) | CLI, TUI, and core pipeline |
| Async runtime | **tokio** | Async I/O for crawling, file operations, subprocess management |
| CLI framework | **clap** (derive) | Subcommand/flag parsing with auto-generated help |
| TUI framework | **ratatui** + **crossterm** | Interactive terminal UI (Create KB, Crawl Preview, Update, etc.) |
| Progress | **indicatif** | Progress bars and spinners for crawl/build operations |
| Logging | **tracing** + **tracing-subscriber** | Structured, leveled diagnostics with optional JSON output |
| HTTP client | **reqwest** | Async HTTP fetching with redirect/timeout/TLS support |
| HTML parsing | **scraper** (html5ever + selectors) | DOM traversal, CSS selectors, platform adapter detection |
| HTML → Markdown | **htmd** | High-fidelity HTML-to-Markdown conversion |
| Serialization | **serde** + **serde_json** | JSON (de)serialization for manifest, TOC, schemas, IPC |
| Config (TOML) | **toml** | Parse `~/.contextbuilder/contextbuilder.toml` |
| Database | **libsql** | Turso Embedded/libSQL client (offline, single-file SQLite) |
| URL handling | **url** | URL parsing, canonicalization, join, origin checks |
| Hashing | **sha2** | SHA-256 content hashing for incremental update diffs |
| UUID | **uuid** (v7) | Sortable unique IDs for KBs, pages, crawl jobs |
| Error handling | **thiserror** (libraries) / **color-eyre** (apps) | Typed errors in crates; rich diagnostics in CLI/TUI |
| Regex | **regex** | URL pattern matching (include/exclude), content cleanup |
| Testing | **cargo test** (built-in) | Unit + integration tests |
| Linting | **clippy** | Rust linter |
| Formatting | **rustfmt** | Rust formatter |

## TypeScript Ecosystem

| Category | Technology | Purpose |
|----------|-----------|---------|
| Language | **TypeScript** (strict) | MCP server, LLM enrichment bridge, shared libraries |
| Runtime | **Bun** | JavaScript/TypeScript runtime (replaces Node.js for execution) |
| Package manager | **Bun** | Dependency management, workspace resolution, lockfile |
| MCP SDK | **@modelcontextprotocol/sdk** | Official MCP TypeScript SDK — tools, resources, transports (stdio + Streamable HTTP) |
| AI SDK | **ai** (Vercel AI SDK) | Unified LLM interface for enrichment pipeline |
| OpenRouter provider | **@openrouter/ai-sdk-provider** | Vercel AI SDK provider for OpenRouter model access |
| Database | **@libsql/client** | Turso Embedded/libSQL client for TS (read-only access to KB indexes) |
| Logging | **evlog** | Wide-event structured logging with structured errors |
| Schema validation | **zod** | Runtime schema validation for MCP tool inputs, config, IPC payloads |
| Testing | **bun test** (built-in) | Unit + integration tests |
| Linting & formatting | **Biome** | All-in-one linter + formatter for TS/JS/JSON/CSS |

## Cross-Language Schemas

| Category | Technology | Purpose |
|----------|-----------|---------|
| Data format | **JSON** | Unified structured data format (manifest, TOC, MCP schemas, IPC) |
| Schema spec | **JSON Schema** (2020-12) | MCP tool `inputSchema` / `outputSchema` definitions |
| Config format | **TOML** | User configuration file (`contextbuilder.toml`) |

## Database

| Category | Technology | Purpose |
|----------|-----------|---------|
| Engine | **Turso Embedded / libSQL** | Offline-mode embedded SQLite-compatible database |
| Access pattern | Single-writer (Rust CLI) / multi-reader (TS MCP server) | No cloud sync; filesystem-level database file |

## Build System & Tooling

| Category | Technology | Purpose |
|----------|-----------|---------|
| Rust workspace | **Cargo** (workspace) | Multi-crate build, test, and dependency management |
| TS workspace | **Bun** (workspaces) | Multi-package build, test, and dependency management |
| Orchestration | **Makefile** | Cross-language build/test/lint/release commands |
| CI/CD | **GitHub Actions** | Automated build, test, lint, release pipelines |

## MCP Protocol

| Category | Detail |
|----------|--------|
| Protocol revision | **2025-11-25** |
| Transports | **stdio** (default) + **Streamable HTTP** (optional) |
| Server capabilities | `tools` (listChanged) + `resources` (subscribe, listChanged) |
| URI scheme | `contextbuilder://` (custom, per RFC 3986) |

## Distribution

| Category | Technology | Purpose |
|----------|-----------|---------|
| Rust binary | **cargo build --release** | Statically linked CLI binary (Linux, macOS, Windows) |
| MCP server | **npm** / bundled release | TypeScript MCP server package |
| Binary compilation | **bun build --compile** (future) | Optional single-binary MCP server distribution |
