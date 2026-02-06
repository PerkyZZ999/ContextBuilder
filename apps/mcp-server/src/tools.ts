/**
 * @contextbuilder/mcp-server — tools module
 *
 * Registers the 5 MCP tools: kb_list, kb_get_toc, kb_get_page, kb_search, kb_get_artifact.
 */
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import type { KbReader } from "@contextbuilder/kb-reader";
import { ARTIFACT_NAMES } from "@contextbuilder/shared";

// ---------------------------------------------------------------------------
// Tool registration
// ---------------------------------------------------------------------------

/**
 * Register all ContextBuilder tools on the given MCP server.
 *
 * @param server - The McpServer instance to register tools on.
 * @param getReaders - A function that returns the currently loaded KbReaders.
 */
export function registerTools(
  server: McpServer,
  getReaders: () => Map<string, KbReader>,
): void {
  // --- kb_list ---
  server.registerTool(
    "kb_list",
    {
      description: "List all registered knowledge bases.",
      inputSchema: {},
    },
    async () => {
      const readers = getReaders();
      const kbs = Array.from(readers.values()).map((r) => r.getSummary());

      const structured = { kbs };
      const textFallback = kbs
        .map((kb) => `${kb.name} (${kb.id}): ${kb.source_url} — ${kb.page_count} pages`)
        .join("\n");

      return {
        structuredContent: structured,
        content: [{ type: "text" as const, text: textFallback || "No knowledge bases loaded." }],
      };
    },
  );

  // --- kb_get_toc ---
  server.registerTool(
    "kb_get_toc",
    {
      description: "Get the table of contents for a knowledge base.",
      inputSchema: { kb_id: z.string().describe("Knowledge base ID") },
    },
    async ({ kb_id }) => {
      const reader = getReaders().get(kb_id);
      if (!reader) {
        return {
          isError: true,
          content: [{ type: "text" as const, text: `Knowledge base '${kb_id}' not found.` }],
        };
      }

      const toc = reader.getToc();
      return {
        structuredContent: toc,
        content: [{ type: "text" as const, text: JSON.stringify(toc, null, 2) }],
      };
    },
  );

  // --- kb_get_page ---
  server.registerTool(
    "kb_get_page",
    {
      description: "Get a documentation page from a knowledge base.",
      inputSchema: {
        kb_id: z.string().describe("Knowledge base ID"),
        path: z.string().describe("Page path (e.g., 'getting-started/installation')"),
      },
    },
    async ({ kb_id, path }) => {
      const reader = getReaders().get(kb_id);
      if (!reader) {
        return {
          isError: true,
          content: [{ type: "text" as const, text: `Knowledge base '${kb_id}' not found.` }],
        };
      }

      try {
        const page = await reader.getPage(path);
        return {
          structuredContent: page,
          content: [{ type: "text" as const, text: page.content }],
        };
      } catch {
        return {
          isError: true,
          content: [
            { type: "text" as const, text: `Page '${path}' not found in KB '${kb_id}'.` },
          ],
        };
      }
    },
  );

  // --- kb_search ---
  server.registerTool(
    "kb_search",
    {
      description: "Full-text search across pages in a knowledge base.",
      inputSchema: {
        kb_id: z.string().describe("Knowledge base ID"),
        query: z.string().min(1).describe("Search query string"),
        limit: z
          .number()
          .int()
          .min(1)
          .max(50)
          .default(10)
          .optional()
          .describe("Maximum number of results (default: 10, max: 50)"),
      },
    },
    async ({ kb_id, query, limit }) => {
      const reader = getReaders().get(kb_id);
      if (!reader) {
        return {
          isError: true,
          content: [{ type: "text" as const, text: `Knowledge base '${kb_id}' not found.` }],
        };
      }

      const results = await reader.search(query, limit ?? 10);
      const structured = { results };
      const textFallback = results.length > 0
        ? results
            .map((r, i) => `${i + 1}. ${r.title ?? r.path} (score: ${r.score.toFixed(2)})`)
            .join("\n")
        : "No results found.";

      return {
        structuredContent: structured,
        content: [{ type: "text" as const, text: textFallback }],
      };
    },
  );

  // --- kb_get_artifact ---
  server.registerTool(
    "kb_get_artifact",
    {
      description: "Get a generated artifact from a knowledge base.",
      inputSchema: {
        kb_id: z.string().describe("Knowledge base ID"),
        name: z
          .enum(ARTIFACT_NAMES)
          .describe(
            "Artifact name: llms.txt, llms-full.txt, SKILL.md, rules.md, style.md, do_dont.md",
          ),
      },
    },
    async ({ kb_id, name }) => {
      const reader = getReaders().get(kb_id);
      if (!reader) {
        return {
          isError: true,
          content: [{ type: "text" as const, text: `Knowledge base '${kb_id}' not found.` }],
        };
      }

      try {
        const content = await reader.getArtifact(name);
        return {
          structuredContent: { name, content },
          content: [{ type: "text" as const, text: content }],
        };
      } catch {
        return {
          isError: true,
          content: [
            { type: "text" as const, text: `Artifact '${name}' not found in KB '${kb_id}'.` },
          ],
        };
      }
    },
  );
}
