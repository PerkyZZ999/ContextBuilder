# ContextBuilder — Product Requirements Document (PRD)

Status: Draft (Feb 2026)

## 1) Problem statement
Developers and teams increasingly want to use LLMs/agents with authoritative tech documentation, but documentation is often fragmented across many pages and not packaged for agent consumption.  
When sites do provide `llms.txt` / `llms-full.txt`, it is not always discovered automatically or turned into consistent agent assets.

## 2) Product vision
ContextBuilder turns "a documentation URL" into a repeatable, portable, AI-ready knowledge bundle: local Markdown KB + TOC, `llms.txt` / `llms-full.txt`, an Agent Skill file, and optional agent Instructions/Rules — with built-in LLM-assisted enrichment woven into the core build pipeline.

## 3) Target users
- Solo developers building agents and devtools who want quick, local ingestion and MCP access.
- Small teams standardizing "agent rules" and best practices from internal/external docs.
- Power users who want automation-first workflows (CLI) with an optional guided TUI.

## 4) Key user outcomes
- "I can paste a docs URL and get a clean local KB I can grep, commit, and share."
- "My AI agent can query the docs through MCP, and it's the same KB every time."
- "I can generate consistent 'rules' and a Skill from the docs so my agents follow the standards."
- "I can update an existing KB when the upstream docs change, without rebuilding from scratch."

## 5) Core use cases (user stories)
- As a user, I enter a docs URL and ContextBuilder checks for `llms.txt` / `llms-full.txt` and uses them when available.
- As a user, if llms files are missing, ContextBuilder crawls the docs, converts each page to Markdown, and generates `llms.txt`/`llms-full.txt`.
- As a user, I export a local knowledge base folder with a TOC file for navigation and automation.
- As a user, I generate an Agent Skill artifact aligned to the Agent Skills standard.
- As a user, I generate Instructions/Rules files (style guides, do/don't lists, operational constraints) from docs content.
- As a user, I start a local MCP server (TypeScript, official SDK, MCP 2025-11-25) so my IDE/agent can access the KB through standard MCP tools and resources.
- As a user, every build automatically uses LLM-assisted enrichment (Vercel AI SDK + OpenRouter) to produce high-quality artifacts with summaries, best-practice rules, and glossary entries.
- As a user, I update an existing KB to fetch newer versions of the upstream docs and incrementally refine the KB (add/update/prune pages).
- As a user, I configure persistent defaults (output paths, API keys, crawl policies) in `~/.contextbuilder/contextbuilder.toml` instead of passing flags every time.

## 6) Functional requirements
Ingestion
- URL input (CLI + TUI).
- Discovery-first: try `llms.txt` and `llms-full.txt` at the site root.
- Crawl fallback with scope controls: include/exclude patterns, depth caps, concurrency limits.
- Platform adapters (Docusaurus, VitePress, GitBook, ReadTheDocs, generic fallback) implemented via a `PlatformAdapter` trait for extensible detection and extraction.

Transformation
- HTML → clean Markdown conversion with high fidelity for code blocks and headings.
- TOC extraction and stable path generation for pages.

Outputs (user-selectable)
- `llms.txt`, `llms-full.txt`.
- Local KB: `docs/**/*.md` + `toc.json`.
- Agent Skill file output (e.g., `SKILL.md`) aligned to the Agent Skills standard.
- Instructions/Rules files output.

Incremental updates
- `contextbuilder update --kb <path>` re-fetches upstream docs, diffs against existing page hashes, and adds/updates/prunes pages without full rebuild.
- Updated pages trigger re-generation of affected artifacts.

LLM-assisted enrichment (always-on)
- Enrichment is a mandatory step of the core build pipeline that runs on every build.
- The build uses the OpenRouter provider for the Vercel AI SDK to produce high-quality TOC summaries, Skill entries, and Instructions/Rules.
- Requires a configured OpenRouter API key (via env var referenced in `~/.contextbuilder/contextbuilder.toml`).
- Enrichment is doc-grounded: prompts quote source sections and provenance (page path + section anchors) is stored alongside generated content.
- Outputs are cached by `(kb_id, artifact_type, prompt_hash, model_id)` to avoid re-spending tokens.

MCP access
- Provide a local MCP server (TypeScript) built with the official MCP TypeScript SDK, targeting MCP protocol revision **2025-11-25**.
- Transport: stdio (default, best for IDE integrations) and Streamable HTTP (optional, for local LAN / multi-client use).
- Expose **tools** (model-controlled): `kb_list`, `kb_get_toc`, `kb_get_page`, `kb_search`, `kb_get_artifact`.
- Expose **resources** (application-controlled) via resource templates: `contextbuilder://kb/{kb_id}/docs/{path}`, `contextbuilder://kb/{kb_id}/artifacts/{name}`.
- Tools declare `inputSchema` and `outputSchema` with JSON Schema for structured content responses.
- Support resource annotations (`audience`, `priority`, `lastModified`) for client display heuristics.
- Support `listChanged` notifications when KBs are added/updated.

Configuration
- Persistent user configuration stored at `~/.contextbuilder/contextbuilder.toml`.
- Covers: default output paths, OpenRouter API key env-var reference, default crawl policies (depth, concurrency, patterns), and KB registry (list of known KBs with paths).
- CLI flags override config-file values; config-file values override built-in defaults.

Storage
- Use Turso Embedded/libSQL in **offline mode** for indexes, crawl metadata, and search indexes while keeping the KB portable on disk.
- The Rust CLI writes the database; the TypeScript MCP server reads it in read-only mode.
- Optional future mode: AgentFS-backed KB packaging for single-file portability + auditing.

Observability
- Rust CLI and libraries: `tracing` crate for structured, leveled logging with optional JSON output.
- TypeScript components (MCP server): `evlog` wide events + structured errors.

## 7) Non-functional requirements
- Local-first by default; requires an OpenRouter API key for LLM enrichment (no other cloud account needed).
- Deterministic builds: same inputs + same tool version should generate stable outputs (except timestamps).
- Safety: protect against SSRF and unsafe URL schemes; never write secrets into artifacts.
- Performance: handle hundreds to thousands of docs pages with resumable crawl jobs.
- Portability: KB output must be readable without ContextBuilder installed.
- KB format versioning: `manifest.json` includes a `schema_version` field; the tool warns or migrates when opening a KB built with an older schema version.

## 8) Out of scope (initial definition)
- Authentication-heavy/private documentation ingestion (unless user supplies cookies/tokens explicitly later).
- Full browser automation by default (headless rendering can be optional/advanced).
- Guaranteeing legal right to scrape any target site (user remains responsible).
- Custom output templates (fixed artifact formats for now; templating may be added later).

## 9) Success metrics (initial)
- Time-to-first-KB: median time from URL input to usable KB output.
- Output quality: user-rated "cleanliness" of Markdown and TOC correctness.
- Adoption: number of KBs created per user and MCP server activations.
- Reliability: crawl success rate and resumability (failures that can resume without restarting).
- Update efficiency: percentage of pages skipped (unchanged) during incremental updates.

## 10) Risks and mitigations
- Docs conversion quality varies: mitigate with platform adapters (`PlatformAdapter` trait) and user-configurable cleanup rules.
- Very large doc sets can produce oversized `llms-full.txt`: mitigate with size caps and chunking plus "index-only" default behavior.
- MCP ecosystem changes: mitigate by targeting a pinned protocol revision (2025-11-25), building on the official TypeScript SDK, and keeping the server surface small and well-versioned.
