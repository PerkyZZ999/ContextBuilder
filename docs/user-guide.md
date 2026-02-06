# ContextBuilder User Guide

This guide walks you through everything you need to use ContextBuilder — from installing it to creating knowledge bases, browsing them in the TUI, and connecting them to your AI coding assistant.

---

## Table of Contents

- [Installation](#installation)
- [Your First Knowledge Base](#your-first-knowledge-base)
- [Understanding the Output](#understanding-the-output)
- [Updating a Knowledge Base](#updating-a-knowledge-base)
- [Listing Knowledge Bases](#listing-knowledge-bases)
- [Building Artifacts](#building-artifacts)
- [Using the TUI](#using-the-tui)
- [Connecting to AI Clients](#connecting-to-ai-clients)
- [Configuration](#configuration)
- [Workflows & Recipes](#workflows--recipes)
- [Troubleshooting](#troubleshooting)

---

## Installation

### Prerequisites

| Dependency | Version | Purpose |
|-----------|---------|---------|
| [Rust](https://rustup.rs/) | 1.85+ | CLI and core pipeline |
| [Bun](https://bun.sh/) | 1.3+ | MCP server and LLM bridge |
| [OpenRouter API key](https://openrouter.ai/) | — | LLM enrichment (required) |

### Building from Source

```bash
# Clone the repository
git clone https://github.com/PerkyZZ999/ContextBuilder.git
cd ContextBuilder

# Build everything (Rust + TypeScript)
make build

# Verify the build
./target/debug/contextbuilder --version
```

This produces two binaries:

- `./target/debug/contextbuilder` — The main CLI
- `./target/debug/contextbuilder-tui` — The standalone TUI

### Setting Up Your API Key

ContextBuilder requires an OpenRouter API key for LLM enrichment. There are two ways to provide it:

**Option 1: Environment variable (quick start)**
```bash
export OPENROUTER_API_KEY=sk-or-v1-your-key-here
```

**Option 2: Config file (persistent)**
```bash
# Create config with defaults
./target/debug/contextbuilder config init

# The config file at ~/.contextbuilder/contextbuilder.toml will reference
# the OPENROUTER_API_KEY env var by name (it never stores the key itself)
```

> **Security note:** The config file stores the *name* of the environment variable (`api_key_env = "OPENROUTER_API_KEY"`), never the actual key. Always set the key as an environment variable.

---

## Your First Knowledge Base

### The `add` Command

The `add` command creates a new knowledge base from a documentation URL:

```bash
./target/debug/contextbuilder add <URL> [OPTIONS]
```

**Example:**
```bash
./target/debug/contextbuilder add https://docs.example.com --name "Example Docs"
```

**Output:**
```
Discovering content at https://docs.example.com...
  ✓ Found llms.txt with 28 entries
Converting 28 pages to Markdown...
  ✓ 28/28 pages converted
Running LLM enrichment...
  ✓ 6 artifacts generated
Building knowledge base...
  ✓ Knowledge base created successfully!

  ID:     019748d2-abcd-7000-8000-000000000001
  Name:   Example Docs
  Pages:  28
  Path:   var/kb/019748d2-abcd-7000-8000-000000000001
```

### Options

| Flag | Short | Description | Default |
|------|-------|-------------|---------|
| `--name <NAME>` | `-n` | Human-readable name for the KB | Derived from URL |
| `--max-pages <N>` | — | Maximum pages to crawl | 500 |
| `--max-depth <N>` | — | Maximum crawl depth | 5 |
| `--delay <MS>` | — | Delay between requests (ms) | 200 |
| `--concurrent <N>` | — | Concurrent requests | 5 |

### How Discovery Works

When you run `add`, ContextBuilder tries two strategies in order:

1. **llms.txt detection** — Checks if the site publishes an `llms.txt` file (per the [llms.txt spec](https://llmstxt.org/)). If found, it parses the listed URLs directly — fast and accurate.
2. **Crawl fallback** — If no `llms.txt` is found, ContextBuilder crawls the site starting from the given URL, respecting `robots.txt`, depth limits, and page caps.

### Platform Adapters

During content extraction, ContextBuilder auto-detects the documentation framework and uses a specialized adapter:

| Platform | Detection | What It Does |
|----------|-----------|--------------|
| **Docusaurus** | `<meta name="generator" content="Docusaurus">` | Extracts from `<article>`, handles sidebar nav |
| **VitePress** | `.vp-doc` class, VitePress meta | Extracts from `.vp-doc` container |
| **GitBook** | GitBook-specific elements | Handles GitBook's page structure |
| **ReadTheDocs** | Sphinx/RTD class names | Extracts from `.rst-content` |
| **Generic** | Always matches (fallback) | Best-effort `<main>` / `<article>` / `<body>` extraction |

Adapters are tried in priority order; the first one that matches wins.

---

## Understanding the Output

After running `add`, your knowledge base lives in `var/kb/<kb-id>/`. Here's what's inside:

```
var/kb/019748d2-.../
├── manifest.json          # KB metadata (name, source URL, timestamps)
├── toc.json               # Table of contents with page hierarchy
├── docs/                  # Converted Markdown pages
│   ├── getting-started.md
│   ├── api/
│   │   ├── overview.md
│   │   └── endpoints.md
│   └── ...
├── artifacts/             # LLM-generated AI artifacts
│   ├── llms.txt           # Compact overview (llms.txt spec)
│   ├── llms-full.txt      # Complete docs in one file
│   ├── SKILL.md           # Agent Skills definition
│   ├── rules.md           # Coding rules & conventions
│   ├── style.md           # Style patterns & preferences
│   └── do_dont.md         # Do/don't guidelines
└── indexes/
    └── contextbuilder.db  # SQLite database (FTS5 indexes)
```

### Artifacts Explained

| Artifact | Format | Best For |
|----------|--------|----------|
| **llms.txt** | Structured overview | Quick context injection — compact enough for any model |
| **llms-full.txt** | Complete concatenation | Large-context models (100k+ tokens) that can ingest everything |
| **SKILL.md** | Agent Skills format | Teaching an AI agent what this library/tool can do |
| **rules.md** | Rules list | Telling an AI "when using this library, follow these rules" |
| **style.md** | Style guide | Code style preferences (naming, patterns, idioms) |
| **do_dont.md** | Do/Don't pairs | Explicit "do this, don't do that" for AI assistants |

### The SQLite Database

The `indexes/contextbuilder.db` file contains:

- **Pages table** — All page metadata (title, path, URL, content hash, timestamps)
- **FTS5 index** — Full-text search across all page content
- **Enrichment cache** — Cached LLM results keyed by content hash + model
- **TOC entries** — Hierarchical table of contents

This database powers the MCP server's search and retrieval capabilities.

---

## Updating a Knowledge Base

### The `update` Command

Refresh an existing KB to pick up documentation changes:

```bash
./target/debug/contextbuilder update --kb var/kb/<kb-id> [OPTIONS]
```

**How incremental updates work:**

1. Re-crawl the source URL (or re-fetch `llms.txt` entries)
2. Compute SHA-256 hashes of new content
3. Compare against stored hashes — skip unchanged pages
4. Re-convert and re-enrich only changed/new pages
5. Optionally prune pages that no longer exist

### Options

| Flag | Description | Default |
|------|-------------|---------|
| `--force` | Force re-crawl of all pages (ignore hashes) | `false` |
| `--prune` | Remove pages that no longer exist at the source | `false` |

**Examples:**
```bash
# Incremental update (fast — only changed pages)
./target/debug/contextbuilder update --kb var/kb/<kb-id>

# Full re-crawl + cleanup
./target/debug/contextbuilder update --kb var/kb/<kb-id> --force --prune
```

### Enrichment Caching

LLM enrichment results are cached by `(kb_id, artifact_type, content_hash, model_id)`. This means:

- **Unchanged pages** → instant cache hit, no LLM call
- **Changed pages** → new LLM call, result cached for next time
- **Model change** → all pages re-enriched (different cache key)

---

## Listing Knowledge Bases

### The `list` Command

See all knowledge bases:

```bash
./target/debug/contextbuilder list [OPTIONS]
```

| Flag | Description | Default |
|------|-------------|---------|
| `--dir <PATH>` | Directory to scan for KBs | `var/kb/` |
| `--format <FMT>` | Output format: `table`, `json` | `table` |

**Example output:**
```
ID                                    Name            Pages  Updated
────────────────────────────────────  ──────────────  ─────  ──────────
019748d2-abcd-7000-8000-000000000001  Example Docs    28     2025-07-14
019748d3-efab-7000-8000-000000000002  React Docs      156    2025-07-13
```

---

## Building Artifacts

### The `build` Command

Regenerate artifacts for an existing KB without re-crawling:

```bash
./target/debug/contextbuilder build --kb var/kb/<kb-id> [OPTIONS]
```

This is useful when you want to:
- Regenerate artifacts with a different LLM model
- Update artifacts after manual page edits
- Rebuild specific artifact types

| Flag | Description |
|------|-------------|
| `--model <MODEL>` | Override the LLM model for this build |

---

## Using the TUI

The Terminal User Interface provides a visual way to manage your knowledge bases.

### Launching

```bash
# Via the main CLI
./target/debug/contextbuilder tui

# Or the standalone binary
./target/debug/contextbuilder-tui
```

### Screens

The TUI has 5 screens you can navigate between:

| Screen | Description |
|--------|-------------|
| **KB List** | Browse all knowledge bases, see page counts and dates |
| **KB Detail** | View a KB's metadata, artifacts, and table of contents |
| **Page Viewer** | Read a specific page's Markdown content |
| **Search** | Full-text search across a KB's pages |
| **MCP Status** | Start/stop the MCP server, view connection info |

### Keybindings

| Key | Action |
|-----|--------|
| `↑` / `↓` or `j` / `k` | Navigate lists |
| `Enter` | Select / Open |
| `Esc` | Go back |
| `q` | Quit |
| `/` | Open search |
| `r` | Refresh current view |
| `Tab` | Switch between panes |
| `?` | Show help |

---

## Connecting to AI Clients

### Quick Setup

The fastest way to get config for your AI client:

```bash
# VS Code / GitHub Copilot
./target/debug/contextbuilder mcp config --target vscode --kb var/kb/<kb-id>

# Claude Desktop
./target/debug/contextbuilder mcp config --target claude --kb var/kb/<kb-id>

# Cursor
./target/debug/contextbuilder mcp config --target cursor --kb var/kb/<kb-id>
```

This outputs a JSON snippet you can paste into your client's configuration file.

### Manual MCP Server

You can also start the MCP server manually:

```bash
# stdio transport (default — for AI clients that manage the process)
./target/debug/contextbuilder mcp serve --kb var/kb/<kb-id>

# HTTP transport (for remote or multi-client access)
./target/debug/contextbuilder mcp serve --kb var/kb/<kb-id> --transport http --port 3100
```

> **See the [MCP Integration Guide](mcp-integration-guide.md)** for detailed setup instructions for each client, transport options, and troubleshooting.

---

## Configuration

### Initializing

```bash
# Create ~/.contextbuilder/contextbuilder.toml with defaults
./target/debug/contextbuilder config init

# View current configuration
./target/debug/contextbuilder config show
```

### Config File

```toml
[openrouter]
api_key_env = "OPENROUTER_API_KEY"        # env var name holding the key
default_model = "moonshotai/kimi-k2.5"    # default LLM model

[defaults]
max_pages = 500                # max pages per crawl
max_depth = 5                  # max crawl depth
request_delay_ms = 200         # delay between HTTP requests
concurrent_requests = 5        # parallel crawl requests
respect_robots_txt = true      # honor robots.txt
user_agent = "ContextBuilder/0.1"

[crawl_policies]
# Per-domain overrides:
# [crawl_policies."docs.example.com"]
# max_pages = 1000
# max_depth = 10
```

### Precedence

Settings are resolved in this order (highest priority first):

1. **CLI flags** — `--max-pages 100` overrides everything
2. **Config file** — `~/.contextbuilder/contextbuilder.toml`
3. **Built-in defaults** — Hardcoded fallbacks

> **See the [Configuration Reference](configuration-reference.md)** for a complete field-by-field reference.

---

## Workflows & Recipes

### Recipe: Ingest Library Docs for a Project

```bash
# 1. Add the library's documentation
contextbuilder add https://docs.astro.build --name "Astro Docs"

# 2. Generate a VS Code MCP config snippet
contextbuilder mcp config --target vscode --kb var/kb/<kb-id>

# 3. Paste the snippet into .vscode/mcp.json
# 4. Restart VS Code — Copilot now has Astro knowledge!
```

### Recipe: Keep Multiple KBs Updated

```bash
# Update all KBs in the default directory
for dir in var/kb/*/; do
  echo "Updating $dir..."
  contextbuilder update --kb "$dir"
done
```

### Recipe: Use Artifacts Directly

You don't need the MCP server to use artifacts. They're plain Markdown files:

```bash
# Copy rules.md into your project for AI assistants
cp var/kb/<kb-id>/artifacts/rules.md .github/copilot-instructions.md

# Or feed llms.txt directly to an AI prompt
cat var/kb/<kb-id>/artifacts/llms.txt | pbcopy
```

### Recipe: Different Models for Different Docs

```bash
# Use a fast model for simple API docs
contextbuilder add https://api.example.com/docs \
  --name "API Docs"

# Rebuild with a more capable model for complex framework docs
contextbuilder build --kb var/kb/<kb-id> \
  --model "anthropic/claude-sonnet-4"
```

---

## Troubleshooting

### Common Issues

<details>
<summary><strong>"OPENROUTER_API_KEY not set"</strong></summary>

LLM enrichment requires an OpenRouter API key. Set it:
```bash
export OPENROUTER_API_KEY=sk-or-v1-your-key-here
```
Or add to your shell profile (`~/.bashrc`, `~/.zshrc`, etc.).
</details>

<details>
<summary><strong>"Bun not found" or MCP server fails to start</strong></summary>

The MCP server and LLM bridge run on Bun. Install it:
```bash
curl -fsSL https://bun.sh/install | bash
```
Then rebuild:
```bash
make build
```
</details>

<details>
<summary><strong>Crawl returns 0 pages</strong></summary>

Possible causes:
1. **robots.txt blocking** — Try with `respect_robots_txt = false` in config
2. **JavaScript-rendered site** — ContextBuilder crawls server-rendered HTML. SPAs that render client-side may return empty content
3. **Rate limiting** — Increase `request_delay_ms` in config
4. **Wrong URL** — Ensure the URL points to the docs root, not a specific page
</details>

<details>
<summary><strong>LLM enrichment is slow</strong></summary>

Enrichment calls an LLM for each page. To speed things up:
- Use a faster/cheaper model in config
- Reduce page count with `--max-pages`
- Subsequent updates use the enrichment cache — only changed pages trigger LLM calls
</details>

<details>
<summary><strong>MCP server not connecting</strong></summary>

1. Verify the server starts: `contextbuilder mcp serve --kb var/kb/<kb-id>`
2. Check that the KB path exists and contains `manifest.json`
3. For VS Code, ensure `.vscode/mcp.json` has the correct absolute paths
4. Check Bun version: `bun --version` (needs 1.3+)
</details>

---

## Next Steps

- [MCP Integration Guide](mcp-integration-guide.md) — Detailed AI client setup
- [Configuration Reference](configuration-reference.md) — All config fields
- [Architecture Guide](architecture.md) — How ContextBuilder works under the hood
- [API Reference](api-reference.md) — CLI commands and MCP tools reference
