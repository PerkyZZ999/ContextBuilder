# MCP Integration Guide

This guide covers everything you need to connect ContextBuilder's MCP server to your AI coding assistant — VS Code / GitHub Copilot, Claude Desktop, Cursor, Windsurf, or any MCP-compatible client.

---

## Table of Contents

- [Overview](#overview)
- [Starting the MCP Server](#starting-the-mcp-server)
- [Transport Options](#transport-options)
- [Client Configuration](#client-configuration)
  - [VS Code / GitHub Copilot](#vs-code--github-copilot)
  - [Claude Desktop](#claude-desktop)
  - [Cursor](#cursor)
  - [Windsurf](#windsurf)
  - [Generic MCP Client](#generic-mcp-client)
- [Using the Config Generator](#using-the-config-generator)
- [MCP Tools Reference](#mcp-tools-reference)
- [MCP Resources Reference](#mcp-resources-reference)
- [Multi-KB Setup](#multi-kb-setup)
- [HTTP Transport & Remote Access](#http-transport--remote-access)
- [Troubleshooting](#troubleshooting)

---

## Overview

ContextBuilder implements the [Model Context Protocol](https://modelcontextprotocol.io/) (revision **2025-11-25**), exposing your knowledge bases through:

- **5 tools** — Functions the AI can call to list, search, and read from your KBs
- **3 resource templates** — URI-based access to documentation pages, artifacts, and tables of contents

The MCP server is a TypeScript process (run via Bun) that reads your KB's files and SQLite database **read-only**. It never modifies your knowledge base.

---

## Starting the MCP Server

### Via the CLI

```bash
# Start with stdio transport (default)
./target/debug/contextbuilder mcp serve --kb var/kb/<kb-id>

# Start with HTTP transport
./target/debug/contextbuilder mcp serve --kb var/kb/<kb-id> --transport http --port 3100
```

### Options

| Flag | Description | Default |
|------|-------------|---------|
| `--kb <PATH>` | Path to the knowledge base directory | Required |
| `--transport <TYPE>` | Transport: `stdio` or `http` | `stdio` |
| `--port <PORT>` | HTTP port (only with `--transport http`) | `3100` |

### Direct with Bun

You can also start the MCP server directly without the Rust CLI:

```bash
bun run apps/mcp-server/src/index.ts --kb /path/to/kb
```

This is what most client configurations use, since the AI client manages the process lifecycle.

---

## Transport Options

### stdio (Default)

The client starts the MCP server as a child process and communicates via stdin/stdout. This is the standard for local AI clients.

**Best for:** VS Code, Claude Desktop, Cursor — any client that manages the server process.

```bash
contextbuilder mcp serve --kb var/kb/<kb-id>
# or
contextbuilder mcp serve --kb var/kb/<kb-id> --transport stdio
```

### Streamable HTTP

The server runs as an HTTP endpoint. Clients connect over the network.

**Best for:** Remote access, shared team servers, or clients that connect to URLs.

```bash
contextbuilder mcp serve --kb var/kb/<kb-id> --transport http --port 3100
```

The server will be available at `http://localhost:3100/mcp`.

---

## Client Configuration

### VS Code / GitHub Copilot

1. Create or edit `.vscode/mcp.json` in your project:

```json
{
  "servers": {
    "contextbuilder": {
      "type": "stdio",
      "command": "bun",
      "args": [
        "run",
        "/absolute/path/to/contextbuilder/apps/mcp-server/src/index.ts",
        "--kb",
        "/absolute/path/to/var/kb/<kb-id>"
      ]
    }
  }
}
```

2. Reload VS Code (`Ctrl+Shift+P` → "Developer: Reload Window")
3. The MCP server appears in the Copilot panel's tool list

> **Tip:** Use `contextbuilder mcp config --target vscode --kb var/kb/<kb-id>` to auto-generate this.

### Claude Desktop

1. Open your Claude Desktop config file:
   - **macOS:** `~/Library/Application Support/Claude/claude_desktop_config.json`
   - **Linux:** `~/.config/Claude/claude_desktop_config.json`
   - **Windows:** `%APPDATA%\Claude\claude_desktop_config.json`

2. Add the server configuration:

```json
{
  "mcpServers": {
    "contextbuilder": {
      "command": "bun",
      "args": [
        "run",
        "/absolute/path/to/contextbuilder/apps/mcp-server/src/index.ts",
        "--kb",
        "/absolute/path/to/var/kb/<kb-id>"
      ]
    }
  }
}
```

3. Restart Claude Desktop

### Cursor

1. Open Cursor Settings → MCP Servers
2. Add a new server:

```json
{
  "mcpServers": {
    "contextbuilder": {
      "command": "bun",
      "args": [
        "run",
        "/absolute/path/to/contextbuilder/apps/mcp-server/src/index.ts",
        "--kb",
        "/absolute/path/to/var/kb/<kb-id>"
      ]
    }
  }
}
```

3. Save and restart Cursor

### Windsurf

1. Open Windsurf MCP configuration
2. Add:

```json
{
  "mcpServers": {
    "contextbuilder": {
      "command": "bun",
      "args": [
        "run",
        "/absolute/path/to/contextbuilder/apps/mcp-server/src/index.ts",
        "--kb",
        "/absolute/path/to/var/kb/<kb-id>"
      ]
    }
  }
}
```

### Generic MCP Client

For any MCP-compatible client, you need:

| Setting | Value |
|---------|-------|
| **Command** | `bun` |
| **Arguments** | `run /path/to/apps/mcp-server/src/index.ts --kb /path/to/kb` |
| **Transport** | `stdio` |
| **Protocol** | MCP 2025-11-25 |

Or for HTTP transport, point the client to `http://localhost:3100/mcp` after starting the server.

---

## Using the Config Generator

The CLI can generate client configuration snippets:

```bash
# Generate VS Code config
contextbuilder mcp config --target vscode --kb var/kb/<kb-id>

# Generate Claude Desktop config
contextbuilder mcp config --target claude --kb var/kb/<kb-id>

# Generate Cursor config
contextbuilder mcp config --target cursor --kb var/kb/<kb-id>
```

This outputs ready-to-paste JSON with absolute paths filled in.

---

## MCP Tools Reference

The MCP server exposes 5 tools that AI clients can call:

### `kb_list`

List all loaded knowledge bases.

**Parameters:** None

**Returns:** Array of KB summaries:
```json
[
  {
    "id": "019748d2-...",
    "name": "Example Docs",
    "source_url": "https://docs.example.com",
    "page_count": 28,
    "created_at": "2025-07-14T10:00:00Z",
    "updated_at": "2025-07-14T10:05:00Z"
  }
]
```

### `kb_get_toc`

Get the table of contents for a knowledge base.

**Parameters:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `kb_id` | string | Yes | Knowledge base ID |

**Returns:** Hierarchical TOC with nested entries:
```json
{
  "entries": [
    {
      "title": "Getting Started",
      "path": "getting-started",
      "children": [
        { "title": "Installation", "path": "getting-started/installation" }
      ]
    }
  ]
}
```

### `kb_get_page`

Read a specific documentation page.

**Parameters:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `kb_id` | string | Yes | Knowledge base ID |
| `path` | string | Yes | Page path within the KB |

**Returns:** Page content and metadata:
```json
{
  "title": "Installation",
  "path": "getting-started/installation",
  "content": "# Installation\n\nTo install the library...",
  "url": "https://docs.example.com/getting-started/installation",
  "content_hash": "sha256:abc123...",
  "updated_at": "2025-07-14T10:00:00Z"
}
```

### `kb_search`

Full-text search across a KB's pages using SQLite FTS5.

**Parameters:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `kb_id` | string | Yes | Knowledge base ID |
| `query` | string | Yes | Search query |
| `limit` | number | No | Max results (default: 10) |

**Returns:** Array of search results with relevance scores:
```json
[
  {
    "path": "api/endpoints",
    "title": "API Endpoints",
    "snippet": "...the **search** endpoint accepts query parameters...",
    "score": 0.95
  }
]
```

### `kb_get_artifact`

Read a generated artifact.

**Parameters:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `kb_id` | string | Yes | Knowledge base ID |
| `artifact_name` | string | Yes | One of: `llms.txt`, `llms-full.txt`, `SKILL.md`, `rules.md`, `style.md`, `do_dont.md` |

**Returns:** The artifact content as text:
```json
{
  "name": "rules.md",
  "content": "# Rules\n\n1. Always use TypeScript strict mode..."
}
```

---

## MCP Resources Reference

Resources provide URI-based access to KB content. AI clients can browse and read these.

### Documentation Pages

**URI template:** `contextbuilder://kb/{kb_id}/docs/{+path}`

**MIME type:** `text/markdown`

**Example:** `contextbuilder://kb/019748d2-.../docs/getting-started/installation`

Returns the Markdown content of the specified documentation page.

### Artifacts

**URI template:** `contextbuilder://kb/{kb_id}/artifacts/{name}`

**MIME type:** `text/markdown`

**Example:** `contextbuilder://kb/019748d2-.../artifacts/rules.md`

Returns the content of the specified artifact.

### Table of Contents

**URI template:** `contextbuilder://kb/{kb_id}/toc`

**MIME type:** `application/json`

**Example:** `contextbuilder://kb/019748d2-.../toc`

Returns the full hierarchical table of contents as JSON.

### Resource Annotations

Resources include MCP annotations for AI client prioritization:

| Annotation | Description |
|-----------|-------------|
| `audience` | Target audience (e.g., `developer`) |
| `priority` | Relative importance (`high`, `medium`, `low`) |
| `lastModified` | ISO 8601 timestamp of last update |

---

## Multi-KB Setup

You can serve multiple knowledge bases from a single MCP server by pointing to a parent directory:

```bash
# Serve all KBs in the default directory
contextbuilder mcp serve --kb var/kb/
```

Or configure multiple MCP servers in your client for different KBs:

```json
{
  "servers": {
    "react-docs": {
      "type": "stdio",
      "command": "bun",
      "args": ["run", "/path/to/apps/mcp-server/src/index.ts", "--kb", "/path/to/var/kb/react-kb-id"]
    },
    "astro-docs": {
      "type": "stdio",
      "command": "bun",
      "args": ["run", "/path/to/apps/mcp-server/src/index.ts", "--kb", "/path/to/var/kb/astro-kb-id"]
    }
  }
}
```

---

## HTTP Transport & Remote Access

The HTTP transport is useful for shared team setups or remote access:

```bash
# Start HTTP server
contextbuilder mcp serve --kb var/kb/<kb-id> --transport http --port 3100
```

### Connecting via HTTP

For clients that support HTTP MCP connections, point to:

```
http://localhost:3100/mcp
```

### Security Considerations

The HTTP transport currently has **no authentication**. For production use:

- Run behind a reverse proxy (Nginx, Caddy) with authentication
- Use SSH tunneling for remote access
- Restrict to `localhost` for local-only access

---

## Troubleshooting

### Server won't start

**Check 1: KB path is valid**
```bash
ls var/kb/<kb-id>/manifest.json
# Should exist
```

**Check 2: Bun is installed and accessible**
```bash
bun --version
# Should print 1.3+
```

**Check 3: Dependencies are built**
```bash
make build
# Rebuilds everything
```

### Client doesn't see the server

1. **VS Code:** Check the MCP panel (Output → MCP) for error logs
2. **Claude Desktop:** Check the application logs for connection errors
3. **All clients:** Verify paths in config are **absolute**, not relative
4. **Path separators:** Use forward slashes on all platforms

### Tools return empty results

1. Verify the KB has pages: `contextbuilder list`
2. Check the database exists: `ls var/kb/<kb-id>/indexes/contextbuilder.db`
3. Try searching: `contextbuilder mcp serve` → call `kb_list` tool

### HTTP transport connection issues

1. Check port isn't in use: `lsof -i :3100`
2. Verify firewall allows the port
3. Test with curl: `curl http://localhost:3100/mcp`

---

## Next Steps

- [User Guide](user-guide.md) — Complete CLI usage walkthrough
- [Configuration Reference](configuration-reference.md) — All settings and defaults
- [API Reference](api-reference.md) — Full API documentation
- [Architecture Guide](architecture.md) — How the MCP server works internally
