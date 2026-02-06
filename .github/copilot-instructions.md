# ContextBuilder — AI Agent Instructions

## What This Project Is

ContextBuilder is a **local-first documentation ingestion tool** that converts a docs URL into AI-ready artifacts (`llms.txt`, `llms-full.txt`, `SKILL.md`, `rules.md`, `style.md`, `do_dont.md`) and a portable knowledge base (Markdown pages + TOC + SQLite indexes). It is a **Rust + TypeScript monorepo** — Rust handles the CLI/TUI/core pipeline, TypeScript handles the MCP server and LLM bridge.

## Architecture — The Critical "Why"

```
URL → Discovery (llms.txt?) → Crawl (fallback) → Markdown conversion
  → LLM Enrichment (always-on, via TS subprocess) → Artifact generation → KB on disk
  → MCP server (TS, reads KB read-only)
```

- **Rust CLI is the sole writer** to both the filesystem KB and the Turso Embedded/libSQL database (`kb/<id>/indexes/contextbuilder.db`).
- **TypeScript MCP server is read-only** — it never writes to the DB or KB files.
- **LLM enrichment is always-on** — it is NOT optional. Every `add`/`build`/`update` invokes the OpenRouter bridge. Never add `--llm off` flags or `llm = false` config options.
- **Rust↔TS integration** uses no IPC — they share state via filesystem + DB. The enrichment bridge (`packages/ts/openrouter-provider/`) is a one-shot subprocess (stdin JSON-lines → stdout JSON-lines), not a persistent server.

## Monorepo Layout

| Path | Language | Role |
|------|----------|------|
| `apps/cli/` | Rust | CLI binary (`clap` subcommands) |
| `apps/tui/` | Rust | TUI binary (`ratatui` + `crossterm`) |
| `apps/mcp-server/` | TypeScript | MCP server (protocol 2025-11-25) |
| `packages/rust/{core,shared,discovery,crawler,markdown,artifacts,storage}/` | Rust | Library crates |
| `packages/ts/{shared,kb-reader,openrouter-provider}/` | TypeScript | TS libraries |
| `packages/schemas/{manifest,toc,artifacts,mcp}/` | JSON Schema + zod | Cross-language schemas |
| `fixtures/{html,markdown,llms}/` | Data | Test fixtures (golden files) |

## Build Commands

All orchestration goes through `Makefile`. Use **Bun** (never npm/yarn/npx/node):

```bash
make build    # cargo build --workspace && bun run build (MCP server)
make test     # cargo test --workspace && bun test
make lint     # cargo clippy -D warnings && bunx biome check .
make fmt      # cargo fmt --all && bunx biome format --write .
```

## Language & Tooling Conventions

### Rust (Edition 2024)
- **Error handling:** `thiserror` in library crates, `color-eyre` in app crates (cli/tui). Never `unwrap()` without a justification comment.
- **Logging:** `tracing` + `tracing-subscriber`. One span per major operation (crawl, convert, enrich, build).
- **IDs:** `uuid` v7 (time-sortable) for KBs, pages, crawl jobs.
- **Hashing:** `sha2` (SHA-256) for content-change detection in incremental updates.
- **Async:** `tokio` runtime. Use `tokio::Semaphore` for concurrency caps.
- **Visibility:** default to `pub(crate)`, not `pub`. Re-export public API via `pub use` in `lib.rs`.

### TypeScript (strict mode, Bun runtime)
- **Linting/formatting:** Biome (not ESLint/Prettier). Config in `biome.json`.
- **Logging:** `evlog` (wide-event structured logs), NOT `console.log`.
- **Validation:** `zod` for all runtime schema validation (MCP inputs, config, IPC payloads).
- **MCP SDK:** `@modelcontextprotocol/sdk` — protocol revision 2025-11-25 with stdio + Streamable HTTP.
- **LLM calls:** `ai` (Vercel AI SDK) + `@openrouter/ai-sdk-provider`. Structured output via zod schemas.
- **No `any`:** Use `unknown` + type narrowing. Suppress with `biome-ignore` only when justified.

## Key Patterns

### PlatformAdapter trait (Rust)
Content extraction uses a trait-based adapter registry. Adapters are tried in priority order; `GenericAdapter` is always-last fallback:
```rust
pub trait PlatformAdapter: Send + Sync {
    fn detect(doc: &Html, url: &Url) -> Option<Self> where Self: Sized;
    fn extract_toc(&self, doc: &Html) -> Vec<TocEntry>;
    fn extract_content(&self, doc: &Html) -> String;
    fn extract_metadata(&self, doc: &Html) -> PageMeta;
    fn name(&self) -> &str;
}
```
Built-in: `DocusaurusAdapter`, `VitePressAdapter`, `GitBookAdapter`, `ReadTheDocsAdapter`, `GenericAdapter`.

### Data Formats
- **All structured data is JSON** — `manifest.json`, `toc.json`, schemas. Never TOML for data files.
- **User config is TOML** — `~/.contextbuilder/contextbuilder.toml`. Only place TOML is used.
- **`manifest.json` has `schema_version: 1`** — always include and check this field.
- **Secrets:** API keys read from env vars (referenced by name in config). Never write keys to DB or artifacts.

### Enrichment Cache
Cache is keyed by `(kb_id, artifact_type, prompt_hash, model_id)` in the `enrichment_cache` table. On incremental update, unchanged pages get cache hits — only changed/new pages trigger LLM calls.

### MCP Resources
Custom URI scheme: `contextbuilder://kb/{kb_id}/docs/{path}`, `contextbuilder://kb/{kb_id}/artifacts/{name}`, `contextbuilder://kb/{kb_id}/toc`. Resources carry annotations (`audience`, `priority`, `lastModified`).

## Testing Approach
- **Rust:** `cargo test` with fixtures in `fixtures/html/` and `fixtures/markdown/` (golden file comparisons).
- **TypeScript:** `bun test` with fixture KB directories.
- **E2E:** mock HTTP server + mock OpenRouter responses (no real network in CI).

## Files to Read First
- `docs/technical-specification.md` — full architecture and schemas
- `docs/project-structure.md` — directory layout rationale
- `docs/tech-stack.md` — every dependency and why
- `docs/ImplementationPlan_01.md` through `_04.md` — phased build plan

## Reference Links
- https://llmstxt.org/ — llms.txt specification (output format)
- https://agentskills.io/home — Agent Skills spec (SKILL.md format)
- https://modelcontextprotocol.io/specification/2025-11-25 — MCP protocol spec (target revision)
- https://modelcontextprotocol.io/ — MCP documentation hub
- https://github.com/modelcontextprotocol/typescript-sdk — MCP TypeScript SDK (used by `apps/mcp-server/`)
- https://docs.turso.tech/llms.txt — Turso docs (libSQL/embedded database)
- https://www.evlog.dev/llms.txt — evlog docs (TS structured logging)
