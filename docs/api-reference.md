# API Reference

Complete API reference for ContextBuilder's CLI commands, MCP tools and resources, TypeScript KB Reader library, Rust Storage API, and the OpenRouter bridge protocol.

---

## Table of Contents

- [CLI Commands](#cli-commands)
- [MCP Tools](#mcp-tools)
- [MCP Resources](#mcp-resources)
- [KbReader TypeScript API](#kbreader-typescript-api)
- [Storage Rust API](#storage-rust-api)
- [OpenRouter Bridge Protocol](#openrouter-bridge-protocol)
- [Data Schemas](#data-schemas)

---

## CLI Commands

### `contextbuilder add <URL>`

Create a new knowledge base from a documentation URL.

```
USAGE:
    contextbuilder add <URL> [OPTIONS]

ARGUMENTS:
    <URL>    Documentation site URL to ingest

OPTIONS:
    -n, --name <NAME>          Human-readable name for the KB
        --max-pages <N>        Maximum pages to crawl [default: 500]
        --max-depth <N>        Maximum crawl depth [default: 5]
        --delay <MS>           Delay between requests in ms [default: 200]
        --concurrent <N>       Concurrent requests [default: 5]
    -h, --help                 Print help
```

**Example:**
```bash
contextbuilder add https://docs.astro.build --name "Astro Docs" --max-pages 800
```

**Pipeline:** Discovery → Crawl → Markdown → LLM Enrichment → Artifact Generation → Storage

---

### `contextbuilder update`

Update an existing knowledge base with incremental change detection.

```
USAGE:
    contextbuilder update --kb <PATH> [OPTIONS]

OPTIONS:
        --kb <PATH>     Path to the KB directory [required]
        --force         Force re-crawl of all pages (ignore content hashes)
        --prune         Remove pages that no longer exist at the source
    -h, --help          Print help
```

**Example:**
```bash
# Incremental update
contextbuilder update --kb var/kb/019748d2-...

# Full re-crawl with cleanup
contextbuilder update --kb var/kb/019748d2-... --force --prune
```

**Behavior:**
- Without `--force`: Computes SHA-256 hashes, skips unchanged pages
- With `--force`: Re-crawls and re-processes all pages
- With `--prune`: Removes pages from the KB that no longer exist at the source URL

---

### `contextbuilder build`

Regenerate artifacts for an existing KB without re-crawling.

```
USAGE:
    contextbuilder build --kb <PATH> [OPTIONS]

OPTIONS:
        --kb <PATH>       Path to the KB directory [required]
        --model <MODEL>   Override the LLM model for this build
    -h, --help            Print help
```

**Example:**
```bash
contextbuilder build --kb var/kb/019748d2-... --model anthropic/claude-sonnet-4
```

---

### `contextbuilder list`

List all knowledge bases.

```
USAGE:
    contextbuilder list [OPTIONS]

OPTIONS:
        --dir <PATH>       Directory to scan for KBs [default: var/kb/]
        --format <FMT>     Output format: table, json [default: table]
    -h, --help             Print help
```

**Table output:**
```
ID                                    Name            Pages  Updated
────────────────────────────────────  ──────────────  ─────  ──────────
019748d2-abcd-7000-8000-000000000001  Astro Docs      156    2025-07-14
019748d3-efab-7000-8000-000000000002  React Docs      284    2025-07-13
```

**JSON output:**
```json
[
  {
    "id": "019748d2-...",
    "name": "Astro Docs",
    "source_url": "https://docs.astro.build",
    "page_count": 156,
    "updated_at": "2025-07-14T10:00:00Z"
  }
]
```

---

### `contextbuilder tui`

Launch the interactive terminal user interface.

```
USAGE:
    contextbuilder tui
```

Opens a 5-screen TUI for browsing KBs, viewing pages, searching, and managing the MCP server. See the [User Guide](user-guide.md#using-the-tui) for screen descriptions and keybindings.

---

### `contextbuilder mcp serve`

Start the MCP server for AI client integration.

```
USAGE:
    contextbuilder mcp serve --kb <PATH> [OPTIONS]

OPTIONS:
        --kb <PATH>            KB directory path [required]
        --transport <TYPE>     Transport: stdio, http [default: stdio]
        --port <PORT>          HTTP port [default: 3100]
    -h, --help                 Print help
```

---

### `contextbuilder mcp config`

Generate client configuration snippets.

```
USAGE:
    contextbuilder mcp config --target <CLIENT> --kb <PATH>

OPTIONS:
        --target <CLIENT>    Client: vscode, claude, cursor [required]
        --kb <PATH>          KB directory path [required]
    -h, --help               Print help
```

**Example output (VS Code):**
```json
{
  "servers": {
    "contextbuilder": {
      "type": "stdio",
      "command": "bun",
      "args": ["run", "/home/user/contextbuilder/apps/mcp-server/src/index.ts", "--kb", "/home/user/contextbuilder/var/kb/019748d2-..."]
    }
  }
}
```

---

### `contextbuilder config init`

Create the config file with defaults.

```
USAGE:
    contextbuilder config init
```

Creates `~/.contextbuilder/contextbuilder.toml` with all available options and their default values.

---

### `contextbuilder config show`

Display the active configuration.

```
USAGE:
    contextbuilder config show
```

Shows the merged configuration from all sources (config file + defaults).

---

## MCP Tools

The MCP server exposes 5 tools callable by AI clients via the Model Context Protocol.

### `kb_list`

List all loaded knowledge bases.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {},
  "required": []
}
```

**Returns:**
```typescript
{
  content: [{
    type: "text",
    text: JSON.stringify([{
      id: string,           // UUID v7
      name: string,
      source_url: string,
      page_count: number,
      created_at: string,   // ISO 8601
      updated_at: string    // ISO 8601
    }])
  }]
}
```

---

### `kb_get_toc`

Get the table of contents for a knowledge base.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "kb_id": {
      "type": "string",
      "description": "Knowledge base ID"
    }
  },
  "required": ["kb_id"]
}
```

**Returns:**
```typescript
{
  content: [{
    type: "text",
    text: JSON.stringify({
      entries: [{
        title: string,
        path: string,
        children: TocEntry[]  // Recursive
      }]
    })
  }]
}
```

**Error:** Returns `isError: true` if the KB ID is not found.

---

### `kb_get_page`

Read a specific documentation page's content and metadata.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "kb_id": {
      "type": "string",
      "description": "Knowledge base ID"
    },
    "path": {
      "type": "string",
      "description": "Page path within the KB (e.g., 'getting-started/installation')"
    }
  },
  "required": ["kb_id", "path"]
}
```

**Returns:**
```typescript
{
  content: [{
    type: "text",
    text: JSON.stringify({
      title: string,
      path: string,
      content: string,         // Full Markdown content
      url: string,             // Original source URL
      content_hash: string,    // SHA-256
      updated_at: string       // ISO 8601
    })
  }]
}
```

**Error:** Returns `isError: true` if the page is not found.

---

### `kb_search`

Full-text search across a knowledge base's pages using SQLite FTS5.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "kb_id": {
      "type": "string",
      "description": "Knowledge base ID"
    },
    "query": {
      "type": "string",
      "description": "Search query"
    },
    "limit": {
      "type": "number",
      "description": "Maximum results to return (default: 10)"
    }
  },
  "required": ["kb_id", "query"]
}
```

**Returns:**
```typescript
{
  content: [{
    type: "text",
    text: JSON.stringify([{
      path: string,
      title: string,
      snippet: string,    // Highlighted match context
      score: number        // Relevance score (0-1)
    }])
  }]
}
```

**Search syntax:** Supports SQLite FTS5 query syntax:
- Simple terms: `routing middleware`
- Phrases: `"error handling"`
- Boolean: `routing AND NOT legacy`
- Prefix: `config*`

---

### `kb_get_artifact`

Read a generated artifact from a knowledge base.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "kb_id": {
      "type": "string",
      "description": "Knowledge base ID"
    },
    "artifact_name": {
      "type": "string",
      "description": "Artifact name",
      "enum": ["llms.txt", "llms-full.txt", "SKILL.md", "rules.md", "style.md", "do_dont.md"]
    }
  },
  "required": ["kb_id", "artifact_name"]
}
```

**Returns:**
```typescript
{
  content: [{
    type: "text",
    text: string  // Artifact content (Markdown)
  }]
}
```

**Error:** Returns `isError: true` if the artifact is not found.

---

## MCP Resources

Resources provide URI-based access to KB content via the MCP resource protocol.

### Documentation Pages

**URI Template:** `contextbuilder://kb/{kb_id}/docs/{+path}`

| Field | Value |
|-------|-------|
| MIME Type | `text/markdown` |
| Annotations | `audience: "developer"`, `priority`, `lastModified` |

**List handler:** Returns all pages in the KB.

**Read handler:** Returns the Markdown content of the specified page.

---

### Artifacts

**URI Template:** `contextbuilder://kb/{kb_id}/artifacts/{name}`

| Field | Value |
|-------|-------|
| MIME Type | `text/markdown` |
| Annotations | `audience: "developer"`, `priority: "high"` |

**List handler:** Returns all 6 artifacts.

**Read handler:** Returns the artifact content.

---

### Table of Contents

**URI Template:** `contextbuilder://kb/{kb_id}/toc`

| Field | Value |
|-------|-------|
| MIME Type | `application/json` |
| Annotations | `audience: "developer"`, `priority: "high"` |

**Read handler:** Returns the TOC as JSON.

---

## KbReader TypeScript API

The `KbReader` class (`packages/ts/kb-reader/`) provides read-only access to knowledge bases from TypeScript.

### Import

```typescript
import { KbReader } from "@contextbuilder/kb-reader";
```

### Static Methods

#### `KbReader.open(kbPath: string): Promise<KbReader>`

Open a knowledge base by path.

```typescript
const reader = await KbReader.open("/path/to/var/kb/019748d2-...");
```

**Throws:** If the path doesn't contain a valid KB (missing `manifest.json` or database).

---

#### `KbReader.discoverKbs(dir: string): Promise<string[]>`

Discover all KB directories within a parent directory.

```typescript
const kbPaths = await KbReader.discoverKbs("/path/to/var/kb/");
// ["/path/to/var/kb/019748d2-...", "/path/to/var/kb/019748d3-..."]
```

---

### Instance Methods

#### `getManifest(): Promise<KbManifest>`

Get the KB's manifest (metadata).

```typescript
const manifest = await reader.getManifest();
// { schema_version: 1, id: "...", name: "...", source_url: "...", ... }
```

---

#### `getToc(): Promise<TocEntry[]>`

Get the table of contents.

```typescript
const toc = await reader.getToc();
// [{ title: "Getting Started", path: "getting-started", children: [...] }]
```

---

#### `getPage(path: string): Promise<PageContent>`

Read a specific page.

```typescript
const page = await reader.getPage("getting-started/installation");
// { title: "Installation", path: "...", content: "# Installation\n...", url: "...", ... }
```

**Type: `PageContent`**
```typescript
interface PageContent {
  title: string;
  path: string;
  content: string;      // Markdown
  url: string;           // Source URL
  content_hash: string;  // SHA-256
  updated_at: string;    // ISO 8601
}
```

---

#### `searchPages(query: string, limit?: number): Promise<SearchResult[]>`

Full-text search across pages.

```typescript
const results = await reader.searchPages("authentication middleware", 5);
// [{ path: "...", title: "...", snippet: "...", score: 0.95 }]
```

**Type: `SearchResult`**
```typescript
interface SearchResult {
  path: string;
  title: string;
  snippet: string;   // Match context with highlights
  score: number;      // Relevance (0-1)
}
```

---

#### `getArtifact(name: string): Promise<string>`

Read an artifact's content.

```typescript
const rules = await reader.getArtifact("rules.md");
// "# Rules\n\n1. Always use TypeScript strict mode..."
```

Valid names: `llms.txt`, `llms-full.txt`, `SKILL.md`, `rules.md`, `style.md`, `do_dont.md`

---

#### `listArtifacts(): Promise<string[]>`

List available artifact names.

```typescript
const artifacts = await reader.listArtifacts();
// ["llms.txt", "llms-full.txt", "SKILL.md", "rules.md", "style.md", "do_dont.md"]
```

---

#### `getSummary(): Promise<KbSummary>`

Get a summary of the KB.

```typescript
const summary = await reader.getSummary();
// { id: "...", name: "...", page_count: 28, artifact_count: 6, ... }
```

**Type: `KbSummary`**
```typescript
interface KbSummary {
  id: string;
  name: string;
  source_url: string;
  page_count: number;
  artifact_count: number;
  created_at: string;
  updated_at: string;
}
```

---

#### `getPageCount(): Promise<number>`

Get the total number of pages.

```typescript
const count = await reader.getPageCount();
// 28
```

---

#### `getRecentPages(limit?: number): Promise<PageContent[]>`

Get recently updated pages.

```typescript
const recent = await reader.getRecentPages(5);
```

---

#### `close(): Promise<void>`

Close the database connection. Call when done.

```typescript
await reader.close();
```

---

## Storage Rust API

The `Storage` struct (`packages/rust/storage/`) provides the database access layer.

### Key Methods

```rust
use contextbuilder_storage::Storage;

// Open a database
let storage = Storage::open(kb_path).await?;

// Pages
storage.insert_page(&page).await?;
storage.get_page(page_id).await?;
storage.get_page_by_path(kb_id, path).await?;
storage.list_pages(kb_id).await?;
storage.update_page(&page).await?;
storage.delete_page(page_id).await?;

// Search
storage.search_pages(kb_id, query, limit).await?;

// TOC
storage.insert_toc_entries(kb_id, &entries).await?;
storage.get_toc(kb_id).await?;

// Enrichment Cache
storage.get_cached_enrichment(kb_id, artifact_type, content_hash, model_id).await?;
storage.cache_enrichment(&cache_entry).await?;

// Knowledge Bases
storage.insert_kb(&kb).await?;
storage.get_kb(kb_id).await?;
storage.list_kbs().await?;
storage.update_kb(&kb).await?;

// Crawl Jobs
storage.insert_crawl_job(&job).await?;
storage.update_crawl_job(&job).await?;

// Artifacts
storage.insert_artifact(&artifact).await?;
storage.get_artifact(kb_id, name).await?;
storage.list_artifacts(kb_id).await?;

// Lifecycle
storage.close().await?;
```

---

## OpenRouter Bridge Protocol

The enrichment bridge (`packages/ts/openrouter-provider/`) communicates with Rust via **stdin/stdout JSON-lines**.

### Request Format (stdin → bridge)

Each line is a JSON object:

```json
{
  "task_id": "unique-task-id",
  "task_type": "extract_rules",
  "content": "# Page Title\n\nPage markdown content...",
  "model": "moonshotai/kimi-k2.5",
  "kb_name": "Example Docs"
}
```

### Task Types

| Task Type | Output | Used In Artifact |
|-----------|--------|-----------------|
| `extract_rules` | Array of coding rules | `rules.md` |
| `extract_style` | Style patterns | `style.md` |
| `extract_do_dont` | Do/don't pairs | `do_dont.md` |
| `extract_skill` | Skill definition | `SKILL.md` |
| `extract_summary` | Page summary | `llms.txt` |
| `extract_full_content` | Clean content | `llms-full.txt` |
| `extract_metadata` | Structured metadata | Internal use |
| `extract_toc_entry` | TOC metadata | Internal use |

### Response Format (bridge → stdout)

```json
{
  "task_id": "unique-task-id",
  "success": true,
  "result": {
    "rules": ["Always use strict mode", "Prefer const over let"]
  }
}
```

**Error response:**
```json
{
  "task_id": "unique-task-id",
  "success": false,
  "error": "Rate limit exceeded"
}
```

### LLM Client Details

- **SDK:** Vercel AI SDK (`ai`) + `@openrouter/ai-sdk-provider`
- **Method:** `generateObject()` with zod schemas for structured output
- **Retry:** Automatic retry with exponential backoff on transient errors
- **Streaming:** Not used — full responses for structured output reliability

---

## Data Schemas

### manifest.json

```typescript
const KbManifestSchema = z.object({
  schema_version: z.literal(1),
  id: z.string().uuid(),
  name: z.string(),
  source_url: z.string().url(),
  created_at: z.string().datetime(),
  updated_at: z.string().datetime(),
});
```

### toc.json

```typescript
const TocEntrySchema: z.ZodType<TocEntry> = z.object({
  title: z.string(),
  path: z.string(),
  children: z.lazy(() => TocEntrySchema.array()).default([]),
});

const TocSchema = z.object({
  entries: z.array(TocEntrySchema),
});
```

### Page Metadata

```typescript
const PageMetaSchema = z.object({
  title: z.string(),
  path: z.string(),
  url: z.string().url(),
  content_hash: z.string(),
  created_at: z.string().datetime(),
  updated_at: z.string().datetime(),
});
```

### Artifact Names

```typescript
const ARTIFACT_NAMES = [
  "llms.txt",
  "llms-full.txt",
  "SKILL.md",
  "rules.md",
  "style.md",
  "do_dont.md",
] as const;
```

---

## Next Steps

- [User Guide](user-guide.md) — Usage walkthrough
- [MCP Integration Guide](mcp-integration-guide.md) — Connecting to AI clients
- [Architecture Guide](architecture.md) — System design deep dive
- [Developer Guide](developer-guide.md) — Contributing guidelines
