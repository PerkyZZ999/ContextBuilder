# ContextBuilder — Implementation Plan: Phase 1

## Foundation & Core Infrastructure

**Goal:** Establish the monorepo skeleton, build system, shared types, configuration, and storage layer so all subsequent phases build on solid ground.

**Estimated effort:** ~2 weeks

**Status: COMPLETE**

---

### Task 1.1 — Monorepo scaffold & build system ✅

**Description:** Initialize the repo with Cargo workspace, Bun workspace, Makefile, and all placeholder crate/package directories.

**Deliverables:**
- [x] Root `Cargo.toml` workspace with all Rust members listed
- [x] Root `package.json` with Bun workspaces configured
- [x] Root `Makefile` with targets: `build`, `test`, `lint`, `fmt`, `clean`, `release`
- [x] `.gitignore` covering Rust (`target/`), Bun (`node_modules/`, `bun.lockb`), and `var/`
- [x] Each `apps/*` and `packages/*` directory has its own `Cargo.toml` or `package.json` with correct name/dependencies
- [x] `biome.json` at root for TS/JS linting and formatting
- [x] `rustfmt.toml` and `clippy.toml` at root for Rust formatting and linting
- [x] CI: GitHub Actions workflow for `make build`, `make test`, `make lint` on push/PR

**Acceptance criteria:**
- [x] `make build` compiles all Rust crates and `bun install` succeeds
- [x] `make lint` and `make fmt` run without errors
- [x] CI pipeline defined (`.github/workflows/ci.yml`)

---

### Task 1.2 — Shared Rust types & error model (`packages/rust/shared`) ✅

**Description:** Define the foundational types used across all Rust crates: error types, config structs, KB metadata types, and common utilities.

**Deliverables:**
- [x] `ContextBuilderError` enum with `thiserror` — variants for: `Config`, `Network`, `Parse`, `Storage`, `Enrichment`, `Io`, `Validation`
- [x] `KbId` newtype (UUID v7 wrapper)
- [x] `KbManifest` struct matching `manifest.json` schema (including `schema_version: u32`)
- [x] `TocEntry` struct matching `toc.json` schema (title, path, source_url, children, summary)
- [x] `PageMeta` struct (id, kb_id, url, path, title, content_hash, fetched_at, status_code, content_len)
- [x] `CrawlConfig` struct (depth, concurrency, include/exclude patterns, rate_limit_ms, mode)
- [x] `AppConfig` struct for deserialized `contextbuilder.toml` with `toml` crate
- [x] Config loading logic: read `~/.contextbuilder/contextbuilder.toml`, merge with CLI flags
- [x] Re-export everything from `packages/rust/shared/src/lib.rs`

**Acceptance criteria:**
- [x] All types derive `Debug, Clone, Serialize, Deserialize`
- [x] Config loader finds/creates `~/.contextbuilder/` directory
- [x] Config loader returns correct defaults when no file exists
- [x] 11 unit tests passing (error display, KbId roundtrip, manifest/toc serialization, config roundtrip, fixture validation)

---

### Task 1.3 — Storage layer (`packages/rust/storage`) ✅

**Description:** Implement the Turso Embedded/libSQL storage layer with schema creation, migrations, and CRUD operations.

**Deliverables:**
- [x] `Storage` struct wrapping a `libsql::Database` connection (offline embedded mode)
- [x] `Storage::open(path)` — opens or creates DB at the given path
- [x] `Storage::open_readonly(path)` — opens in read-only mode (for TS MCP server parity testing)
- [x] Schema migration system: versioned SQL migrations applied on open
- [x] Initial migration (v1): create tables `kb`, `pages`, `links`, `crawl_jobs`, `enrichment_cache`
- [x] CRUD operations:
  - `insert_kb`, `get_kb`, `list_kbs`, `update_kb`
  - `upsert_page`, `get_page`, `list_pages_by_kb`, `delete_page`
  - `insert_link`, `get_links_for_page`
  - `insert_crawl_job`, `update_crawl_job`
  - `get_enrichment_cache`, `set_enrichment_cache`, `invalidate_enrichment_cache`
- [x] Full-text search setup: FTS5 virtual table on `pages(title, path)` with sync triggers
- [x] `Storage::search(kb_id, query, limit)` using FTS5

**Acceptance criteria:**
- [x] 9 integration tests passing: open/migrate, idempotent migration, KB CRUD, page upsert/query, links, crawl jobs, enrichment cache, FTS search, read-only rejection

---

### Task 1.4 — JSON schemas (`packages/schemas/*`) ✅

**Description:** Define the canonical JSON schemas for all structured data formats, usable by both Rust (via serde) and TypeScript (via zod).

**Deliverables:**
- [x] `packages/schemas/manifest/manifest.schema.json` — manifest.json schema
- [x] `packages/schemas/toc/toc.schema.json` — toc.json schema
- [x] `packages/schemas/artifacts/skill.schema.json` — SKILL.md frontmatter metadata schema
- [x] `packages/schemas/artifacts/llms-meta.schema.json` — llms.txt generation metadata
- [x] `packages/schemas/mcp/tool-inputs.schema.json` — MCP tool input schemas
- [x] `packages/schemas/mcp/tool-outputs.schema.json` — MCP tool output schemas
- [x] TypeScript: zod schemas written to match each JSON schema (4 packages)
- [x] Validation: fixture files validated by both Rust serde and TS zod (13 TS tests + 2 Rust tests)

**Acceptance criteria:**
- [x] Schemas validate example fixture files without errors
- [x] Rust `serde` structs serialize/deserialize to schema-compliant JSON
- [x] TS zod schemas parse the same fixture files successfully

---

### Task 1.5 — Shared TypeScript utilities (`packages/ts/shared`) ✅

**Description:** Set up the shared TS package with common types, constants, and utilities.

**Deliverables:**
- [x] TypeScript project with `tsconfig.json` (strict mode)
- [x] Shared types mirroring Rust types: `KbManifest`, `TocEntry`, `PageMeta` (zod schemas)
- [x] Constants: `CURRENT_SCHEMA_VERSION`, `DEFAULT_CONFIG`, `ARTIFACT_NAMES`
- [x] Utility: `loadManifest(kbPath)`, `loadToc(kbPath)`, `readPage(kbPath, pagePath)`
- [x] Utility: `validateManifestVersion(manifest)` — checks `schema_version` compatibility

**Acceptance criteria:**
- [x] 11 unit tests passing (type validation, constants, loaders, version checks)
- [x] All types match the Rust `shared` crate's serialization format

---

### Task 1.6 — CLI skeleton (`apps/cli`) ✅

**Description:** Set up the Rust CLI binary with `clap` subcommands, config loading, tracing initialization, and placeholder handlers.

**Deliverables:**
- [x] Binary crate with `clap` derive-based CLI definition
- [x] Subcommands (all with placeholder implementations):
  - `add <url> --name --out --mode`
  - `build --kb --emit`
  - `update --kb --prune --force`
  - `list`
  - `tui`
  - `mcp serve --kb --transport --port`
  - `config init`
  - `config show`
- [x] `tracing-subscriber` initialization with text/JSON mode (`--log-format`)
- [x] Config loading on startup (from shared config loader)
- [x] OpenRouter API key validation on `add` command
- [x] Version flag (`--version`) displaying crate version
- [x] `config init` — creates `~/.contextbuilder/contextbuilder.toml` with default values
- [x] `config show` — prints resolved config as TOML

**Acceptance criteria:**
- [x] `cargo run --bin contextbuilder -- --help` shows all subcommands with descriptions
- [x] `cargo run --bin contextbuilder -- config init` creates the config file
- [x] `cargo run --bin contextbuilder -- config show` prints valid TOML
- [x] `cargo run --bin contextbuilder -- add https://example.com` exits with API key error
- [x] `cargo run --bin contextbuilder -- --version` prints `contextbuilder 0.1.0`

---

### Phase 1 completion criteria

All of the following must be true:
1. [x] `cargo build --workspace` succeeds — **20 Rust tests pass, 0 warnings**
2. [x] `bun test` passes — **24 TS tests pass**
3. [x] `cargo clippy --workspace -- -D warnings` passes with zero warnings
4. [x] CI pipeline defined (`.github/workflows/ci.yml`)
5. [x] CLI binary runs, loads config, validates API key, responds to all subcommands
6. [x] Storage layer creates DB, runs migrations, performs CRUD + FTS operations (9 tests)
7. [x] JSON schemas validated by both Rust serde and TypeScript zod
