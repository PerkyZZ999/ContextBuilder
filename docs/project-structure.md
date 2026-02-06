# ContextBuilder — Project Structure

This monorepo follows a common workspace layout where `apps/` contains runnable deliverables and `packages/` contains shared libraries and schemas.

```text
contextbuilder/                         - Repo root (monorepo workspace)
├─ .github/                             - GitHub configuration (automation, templates)
│  └─ workflows/                        - CI/CD pipelines (build, test, release)
├─ docs/                                - Project documentation (PRD, technical spec, guides)
├─ examples/                            - Runnable/config examples and demonstrations
│  ├─ demo-configs/                     - Example config files users can copy/adapt
│  └─ sample-outputs/                   - Example generated KB/artifacts for reference
├─ fixtures/                            - Test fixtures used for deterministic testing
│  ├─ html/                             - Saved HTML inputs for crawler/converter tests
│  ├─ markdown/                         - Golden markdown outputs for regression tests
│  └─ llms/                             - Sample llms.txt / llms-full.txt inputs/fixtures
├─ scripts/                             - Dev/CI helper scripts (smoke tests, local setup)
├─ tooling/                             - Shared tooling configs and helpers for the repo
│  ├─ lint/                             - Linting/formatting configurations
│  ├─ release/                          - Release automation tooling and conventions
│  └─ ci/                               - CI utilities (caching, environment setup, helpers)
├─ apps/                                - Executable applications (end-user deliverables)
│  ├─ cli/                              - Rust CLI app (command-driven interface)
│  ├─ tui/                              - Rust TUI app (interactive terminal UI)
│  └─ mcp-server/                       - TypeScript MCP server app (exposes KB via MCP 2025-11-25)
├─ packages/                            - Shared libraries and cross-app modules
│  ├─ rust/                             - Rust crates used by CLI/TUI (library modules)
│  │  ├─ core/                          - Core pipeline orchestration + domain logic
│  │  ├─ shared/                        - Shared Rust types, error model, config structs
│  │  ├─ discovery/                     - llms.txt / llms-full.txt detection + heuristics
│  │  ├─ crawler/                       - Crawling, fetching, link graph, dedupe
│  │  ├─ markdown/                      - HTML-to-Markdown conversion + cleanup passes
│  │  ├─ artifacts/                     - Emitters for llms/skill/rules artifacts
│  │  └─ storage/                       - Turso Embedded/libSQL integration (offline) + indexing
│  ├─ ts/                               - TypeScript libraries shared by TS apps
│  │  ├─ shared/                        - Shared TS utilities/types (non-schema)
│  │  ├─ kb-reader/                     - KB reading/index access helpers for MCP server
│  │  └─ openrouter-provider/           - OpenRouter + AI SDK wiring (provider wrapper + CLI bridge)
│  └─ schemas/                          - Cross-language schema definitions (JSON)
│     ├─ toc/                           - TOC schema (toc.json structure/navigation model)
│     ├─ manifest/                      - Manifest schema (provenance, versions, schema_version)
│     ├─ artifacts/                     - Artifact schemas (skill/rules/llms metadata)
│     └─ mcp/                           - MCP-related schemas (tool input/output shapes)
└─ var/                                 - Local-only runtime data (not committed)
   ├─ cache/                            - Local caches (crawl cache, build cache)
   ├─ logs/                             - Local logs (CLI/TUI/MCP runs)
   └─ kb/                               - Default local knowledge base output directory
```

## Key changes from initial layout

1. **`apps/llm-enrich/` removed.** LLM enrichment is integrated directly into the core build pipeline (`packages/rust/core/` orchestrates it, `packages/ts/openrouter-provider/` provides the LLM bridge). There is no separate enrichment application.

2. **All structured data files use JSON.** `toc.json` (not TOML) for consistency with `manifest.json` and cross-language consumption by both Rust and TypeScript.

3. **`manifest.json` includes `schema_version`.** Enables forward-compatible reads and automatic migrations for older KB formats.

4. **User configuration lives at `~/.contextbuilder/contextbuilder.toml`.** This is outside the repo tree and not shown above. It stores persistent defaults (output paths, API key env-var references, crawl policies, KB registry).

5. **Turso Embedded/libSQL in offline mode.** The database at `kb/<kb-id>/indexes/contextbuilder.db` is written by Rust (sole writer) and read by the TypeScript MCP server (read-only).

6. **MCP server targets protocol revision 2025-11-25.** Supports stdio and Streamable HTTP transports, resource templates with annotations, tools with structured content and output schemas.
