# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2025-07-14

### Added

- **CLI** (`contextbuilder`): Full command-line interface with subcommands:
  - `add <url>` — Ingest documentation from a URL into a knowledge base
  - `update --kb <path>` — Incremental update with content-hash change detection
  - `mcp serve` — Start the MCP server (stdio or HTTP transport)
  - `mcp config` — Print client configuration snippets (VS Code, Claude Desktop, Cursor)
  - `config init / show` — Configuration management
  - `list` — List registered knowledge bases
  - `tui` — Launch interactive terminal UI

- **TUI** (`contextbuilder-tui`): Interactive ratatui-based terminal interface with 5 screens:
  - Create KB, Browse KBs, Update KB, Outputs, MCP Server

- **MCP Server**: Model Context Protocol server (protocol revision 2025-11-25)
  - 5 tools: `kb_list`, `kb_get_toc`, `kb_get_page`, `kb_search`, `kb_get_artifact`
  - 3 resource templates: `contextbuilder://kb/{id}/docs/{path}`, `.../artifacts/{name}`, `.../toc`
  - stdio and Streamable HTTP transports
  - Full-text search via FTS5

- **Discovery**: Automatic `llms.txt` detection with crawl fallback
- **Crawler**: Concurrent web crawler with platform-aware adapters
  - Built-in adapters: Docusaurus, VitePress, GitBook, ReadTheDocs, Generic
  - Configurable depth, concurrency, rate limiting, robots.txt compliance

- **Markdown conversion**: HTML → clean Markdown with navigation chrome stripping
- **LLM enrichment**: Always-on AI enrichment via OpenRouter bridge subprocess
  - Generates 6 artifacts: `llms.txt`, `llms-full.txt`, `SKILL.md`, `rules.md`, `style.md`, `do_dont.md`
  - Enrichment cache for incremental efficiency

- **Storage**: SQLite/libSQL with FTS5 full-text search, schema migrations
- **KB reader**: TypeScript read-only access to KB directories for MCP server
- **Cross-language schemas**: JSON Schema + zod for manifest, TOC, artifacts, MCP types
- **CI**: GitHub Actions pipeline for Rust + TypeScript (build, test, lint)
- **227 tests** across Rust and TypeScript suites
