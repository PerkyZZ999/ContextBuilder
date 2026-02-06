# Configuration Reference

Complete reference for all ContextBuilder configuration options, environment variables, and precedence rules.

---

## Table of Contents

- [Configuration Reference](#configuration-reference)
  - [Table of Contents](#table-of-contents)
  - [Overview](#overview)
  - [Config File Location](#config-file-location)
  - [Managing Configuration](#managing-configuration)
    - [Initialize Config](#initialize-config)
    - [View Current Config](#view-current-config)
  - [Configuration Sections](#configuration-sections)
    - [`[openrouter]`](#openrouter)
    - [`[defaults]`](#defaults)
      - [Tuning Guidelines](#tuning-guidelines)
    - [`[crawl_policies]`](#crawl_policies)
    - [`[[kbs]]`](#kbs)
  - [Precedence Rules](#precedence-rules)
    - [Example Resolution](#example-resolution)
  - [Environment Variables](#environment-variables)
    - [Setting Environment Variables](#setting-environment-variables)
  - [CLI Flag Reference](#cli-flag-reference)
    - [`contextbuilder add`](#contextbuilder-add)
    - [`contextbuilder update`](#contextbuilder-update)
    - [`contextbuilder build`](#contextbuilder-build)
    - [`contextbuilder list`](#contextbuilder-list)
    - [`contextbuilder mcp serve`](#contextbuilder-mcp-serve)
    - [`contextbuilder mcp config`](#contextbuilder-mcp-config)
    - [`contextbuilder config init`](#contextbuilder-config-init)
    - [`contextbuilder config show`](#contextbuilder-config-show)
  - [Complete Example](#complete-example)
  - [Next Steps](#next-steps)

---

## Overview

ContextBuilder uses **TOML** for its configuration file — the only place TOML is used in the project. All other structured data (manifests, TOCs, schemas) uses JSON.

Configuration is resolved with the following precedence (highest first):

1. **CLI flags** — `--max-pages 100`
2. **Config file** — `~/.contextbuilder/contextbuilder.toml`
3. **Built-in defaults** — Hardcoded fallbacks

---

## Config File Location

| Platform | Path |
|----------|------|
| Linux | `~/.contextbuilder/contextbuilder.toml` |
| macOS | `~/.contextbuilder/contextbuilder.toml` |
| Windows | `%USERPROFILE%\.contextbuilder\contextbuilder.toml` |

---

## Managing Configuration

### Initialize Config

Create the config file with all defaults:

```bash
contextbuilder config init
```

This creates the file with commented defaults so you can see all available options.

### View Current Config

Display the active configuration (merged from all sources):

```bash
contextbuilder config show
```

---

## Configuration Sections

### `[openrouter]`

Settings for the OpenRouter LLM API connection.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `api_key_env` | string | `"OPENROUTER_API_KEY"` | Name of the environment variable holding the API key. **Not the key itself.** |
| `default_model` | string | `"moonshotai/kimi-k2.5"` | Default LLM model for enrichment tasks |

```toml
[openrouter]
api_key_env = "OPENROUTER_API_KEY"
default_model = "moonshotai/kimi-k2.5"
```

**Security:** The config file stores the *name* of the environment variable, never the API key directly. This prevents accidental key exposure in version-controlled config files.

**Available models:** Any model available on [OpenRouter](https://openrouter.ai/models) can be used. Examples:
- `moonshotai/kimi-k2.5` (default — fast and cost-effective)
- `anthropic/claude-sonnet-4` (high quality)
- `google/gemini-2.5-flash` (very fast)
- `openai/gpt-4o` (strong general purpose)

### `[defaults]`

Default values for crawl behavior. These apply to all `add`/`update` commands unless overridden by CLI flags or per-domain policies.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_pages` | integer | `500` | Maximum number of pages to crawl per KB |
| `max_depth` | integer | `5` | Maximum crawl depth from the seed URL |
| `request_delay_ms` | integer | `200` | Delay between HTTP requests (milliseconds) |
| `concurrent_requests` | integer | `5` | Maximum concurrent crawl requests |
| `respect_robots_txt` | boolean | `true` | Whether to honor `robots.txt` directives |
| `user_agent` | string | `"ContextBuilder/0.1"` | User-Agent string for HTTP requests |

```toml
[defaults]
max_pages = 500
max_depth = 5
request_delay_ms = 200
concurrent_requests = 5
respect_robots_txt = true
user_agent = "ContextBuilder/0.1"
```

#### Tuning Guidelines

| Scenario | Recommendation |
|----------|---------------|
| Small docs site (<50 pages) | Defaults work well |
| Large docs site (500+ pages) | Increase `max_pages`, maybe `max_depth` |
| Rate-limited site | Increase `request_delay_ms` to 500-1000 |
| Fast site, want quick crawl | Increase `concurrent_requests` to 10-20 |
| Site blocks crawlers | Try `respect_robots_txt = false` |

### `[crawl_policies]`

Per-domain overrides for crawl settings. These let you customize behavior for specific documentation sites.

**Syntax:** `[crawl_policies."domain.com"]`

| Field | Type | Description |
|-------|------|-------------|
| `max_pages` | integer | Override `defaults.max_pages` for this domain |
| `max_depth` | integer | Override `defaults.max_depth` for this domain |
| `request_delay_ms` | integer | Override `defaults.request_delay_ms` for this domain |
| `concurrent_requests` | integer | Override `defaults.concurrent_requests` for this domain |
| `respect_robots_txt` | boolean | Override `defaults.respect_robots_txt` for this domain |

```toml
# Large docs site — allow more pages
[crawl_policies."react.dev"]
max_pages = 1000
max_depth = 8

# Rate-limited site — be gentle
[crawl_policies."api.slow-service.com"]
request_delay_ms = 1000
concurrent_requests = 2

# Internal docs — no robots.txt
[crawl_policies."internal-docs.company.com"]
respect_robots_txt = false
max_pages = 2000
```

### `[[kbs]]`

Pre-configured knowledge base definitions. These let you define KBs in config that can be referenced by name.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Human-readable KB name |
| `url` | string | Yes | Documentation source URL |
| `max_pages` | integer | No | Override max pages for this KB |
| `max_depth` | integer | No | Override max depth for this KB |

```toml
[[kbs]]
name = "React Docs"
url = "https://react.dev"
max_pages = 800

[[kbs]]
name = "Astro Docs"
url = "https://docs.astro.build"

[[kbs]]
name = "Internal API"
url = "https://internal-docs.company.com/api"
max_pages = 200
max_depth = 3
```

---

## Precedence Rules

When the same setting is defined in multiple places, the highest-precedence source wins:

```
CLI flags  >  crawl_policies (per-domain)  >  config file [defaults]  >  built-in defaults
```

### Example Resolution

Given this config:

```toml
[defaults]
max_pages = 500

[crawl_policies."react.dev"]
max_pages = 1000
```

| Command | Effective `max_pages` | Why |
|---------|----------------------|-----|
| `add https://react.dev` | 1000 | Domain policy overrides default |
| `add https://other.com` | 500 | Config default |
| `add https://react.dev --max-pages 50` | 50 | CLI flag overrides everything |
| `add https://other.com --max-pages 50` | 50 | CLI flag overrides everything |

---

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `OPENROUTER_API_KEY` | Yes | OpenRouter API key for LLM enrichment |
| `CONTEXTBUILDER_CONFIG` | No | Override config file path (default: `~/.contextbuilder/contextbuilder.toml`) |
| `CONTEXTBUILDER_LOG` | No | Log level: `error`, `warn`, `info`, `debug`, `trace` (default: `info`) |
| `RUST_LOG` | No | Rust-specific log filter (e.g., `contextbuilder_crawler=debug`) |

### Setting Environment Variables

**Temporary (current session):**
```bash
export OPENROUTER_API_KEY=sk-or-v1-your-key-here
```

**Persistent (add to shell profile):**
```bash
# ~/.bashrc or ~/.zshrc
export OPENROUTER_API_KEY=sk-or-v1-your-key-here
```

**Per-command:**
```bash
OPENROUTER_API_KEY=sk-or-v1-... contextbuilder add https://docs.example.com
```

---

## CLI Flag Reference

### `contextbuilder add`

| Flag | Short | Type | Default | Description |
|------|-------|------|---------|-------------|
| `--name` | `-n` | string | From URL | KB display name |
| `--max-pages` | — | integer | 500 | Max pages to crawl |
| `--max-depth` | — | integer | 5 | Max crawl depth |
| `--delay` | — | integer | 200 | Request delay (ms) |
| `--concurrent` | — | integer | 5 | Concurrent requests |

### `contextbuilder update`

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--kb` | string | Required | Path to KB directory |
| `--force` | boolean | `false` | Force re-crawl (ignore hashes) |
| `--prune` | boolean | `false` | Remove pages that no longer exist |

### `contextbuilder build`

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--kb` | string | Required | Path to KB directory |
| `--model` | string | From config | Override LLM model |

### `contextbuilder list`

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--dir` | string | `var/kb/` | Directory to scan for KBs |
| `--format` | string | `table` | Output format: `table`, `json` |

### `contextbuilder mcp serve`

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--kb` | string | Required | KB directory path |
| `--transport` | string | `stdio` | Transport: `stdio`, `http` |
| `--port` | integer | `3100` | HTTP port (with `--transport http`) |

### `contextbuilder mcp config`

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--target` | string | Required | Client: `vscode`, `claude`, `cursor` |
| `--kb` | string | Required | KB directory path |

### `contextbuilder config init`

No flags. Creates the config file with defaults.

### `contextbuilder config show`

No flags. Displays the active configuration.

---

## Complete Example

A fully configured `~/.contextbuilder/contextbuilder.toml`:

```toml
# ContextBuilder Configuration
# See: docs/configuration-reference.md

# ─── LLM Settings ───────────────────────────────────────────
[openrouter]
api_key_env = "OPENROUTER_API_KEY"
default_model = "moonshotai/kimi-k2.5"

# ─── Default Crawl Settings ─────────────────────────────────
[defaults]
max_pages = 500
max_depth = 5
request_delay_ms = 200
concurrent_requests = 5
respect_robots_txt = true
user_agent = "ContextBuilder/0.1"

# ─── Per-Domain Overrides ───────────────────────────────────
[crawl_policies."react.dev"]
max_pages = 1000
max_depth = 8

[crawl_policies."docs.astro.build"]
max_pages = 800

[crawl_policies."internal-docs.company.com"]
respect_robots_txt = false
max_pages = 2000
request_delay_ms = 100
concurrent_requests = 10

# ─── Pre-Configured Knowledge Bases ─────────────────────────
[[kbs]]
name = "React Docs"
url = "https://react.dev"

[[kbs]]
name = "Astro Docs"
url = "https://docs.astro.build"

[[kbs]]
name = "Internal API"
url = "https://internal-docs.company.com/api"
max_pages = 200
max_depth = 3
```

---

## Next Steps

- [User Guide](user-guide.md) — How to use ContextBuilder
- [API Reference](api-reference.md) — CLI and MCP API details
- [Developer Guide](developer-guide.md) — Contributing to ContextBuilder
