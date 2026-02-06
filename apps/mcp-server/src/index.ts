/**
 * @contextbuilder/mcp-server
 * MCP server exposing ContextBuilder knowledge bases via MCP protocol 2025-11-25.
 *
 * Supports stdio and Streamable HTTP transports.
 *
 * Usage:
 *   bun run apps/mcp-server/src/index.ts --kb <path>
 *   bun run apps/mcp-server/src/index.ts --kb-root <dir> [--transport http --port 3100]
 */
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { KbReader } from "@contextbuilder/kb-reader";
import { registerTools } from "./tools";
import { registerResources } from "./resources";

// ---------------------------------------------------------------------------
// CLI argument parsing
// ---------------------------------------------------------------------------

interface ServerArgs {
  kbPaths: string[];
  kbRoot: string | null;
  transport: "stdio" | "http";
  port: number;
}

function parseArgs(): ServerArgs {
  const args = process.argv.slice(2);
  const result: ServerArgs = {
    kbPaths: [],
    kbRoot: null,
    transport: "stdio",
    port: 3100,
  };

  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    switch (arg) {
      case "--kb":
        if (args[i + 1]) {
          result.kbPaths.push(args[++i]);
        }
        break;
      case "--kb-root":
        result.kbRoot = args[++i] ?? null;
        break;
      case "--transport":
        result.transport = (args[++i] as "stdio" | "http") ?? "stdio";
        break;
      case "--port":
        result.port = Number.parseInt(args[++i] ?? "3100", 10);
        break;
      default:
        break;
    }
  }

  return result;
}

// ---------------------------------------------------------------------------
// Server initialization
// ---------------------------------------------------------------------------

function log(event: string, data: Record<string, unknown> = {}): void {
  const entry = { ts: new Date().toISOString(), level: "info", event, ...data };
  process.stderr.write(`${JSON.stringify(entry)}\n`);
}

async function loadReaders(args: ServerArgs): Promise<Map<string, KbReader>> {
  const readers = new Map<string, KbReader>();

  // Load explicitly specified KBs
  for (const kbPath of args.kbPaths) {
    try {
      const reader = await KbReader.open(kbPath);
      readers.set(reader.id, reader);
      log("kb_loaded", { id: reader.id, name: reader.getManifest().name, path: kbPath });
    } catch (err) {
      log("kb_load_failed", {
        path: kbPath,
        error: err instanceof Error ? err.message : String(err),
      });
    }
  }

  // Discover KBs from root directory
  if (args.kbRoot) {
    const discovered = await KbReader.discoverKbs(args.kbRoot);
    for (const reader of discovered) {
      if (!readers.has(reader.id)) {
        readers.set(reader.id, reader);
        log("kb_discovered", { id: reader.id, name: reader.getManifest().name });
      } else {
        reader.close();
      }
    }
  }

  return readers;
}

async function main(): Promise<void> {
  const args = parseArgs();

  // Load knowledge bases
  const readers = await loadReaders(args);
  log("server_starting", {
    kb_count: readers.size,
    transport: args.transport,
  });

  if (readers.size === 0) {
    log("warning", { message: "No knowledge bases loaded. Tools will return empty results." });
  }

  // Create MCP server
  const server = new McpServer(
    {
      name: "contextbuilder",
      version: "0.1.0",
    },
    {
      capabilities: {
        tools: { listChanged: true },
        resources: { subscribe: true, listChanged: true },
        logging: {},
      },
    },
  );

  // Register tools and resources
  const getReaders = () => readers;
  registerTools(server, getReaders);
  registerResources(server, getReaders);

  // Connect transport
  if (args.transport === "stdio") {
    const transport = new StdioServerTransport();
    await server.connect(transport);
    log("server_started", { transport: "stdio" });

    // Handle graceful shutdown
    const shutdown = async () => {
      log("server_shutting_down");
      for (const reader of readers.values()) {
        reader.close();
      }
      await server.close();
      process.exit(0);
    };

    process.on("SIGINT", shutdown);
    process.on("SIGTERM", shutdown);
  } else {
    // HTTP transport — use Node.js http server with Streamable HTTP
    const { createServer } = await import("node:http");

    // Dynamic import for the streamable HTTP transport
    const { StreamableHTTPServerTransport } = await import(
      "@modelcontextprotocol/sdk/server/streamableHttp.js"
    );

    const httpServer = createServer(async (req, res) => {
      if (req.url === "/mcp" && req.method === "POST") {
        const transport = new StreamableHTTPServerTransport({
          sessionIdGenerator: undefined,
        });
        await server.connect(transport);
        await transport.handleRequest(req, res);
      } else if (req.url === "/health") {
        res.writeHead(200, { "Content-Type": "application/json" });
        res.end(JSON.stringify({ status: "ok", kbs: readers.size }));
      } else {
        res.writeHead(404);
        res.end("Not found");
      }
    });

    httpServer.listen(args.port, "127.0.0.1", () => {
      log("server_started", {
        transport: "http",
        url: `http://127.0.0.1:${args.port}/mcp`,
      });
    });

    const shutdown = () => {
      log("server_shutting_down");
      for (const reader of readers.values()) {
        reader.close();
      }
      httpServer.close();
      process.exit(0);
    };

    process.on("SIGINT", shutdown);
    process.on("SIGTERM", shutdown);
  }
}

main().catch((err) => {
  process.stderr.write(`Fatal: ${err instanceof Error ? err.message : err}\n`);
  process.exit(1);
});
