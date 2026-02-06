/**
 * @contextbuilder/mcp-server — resources module
 *
 * Registers MCP resource templates and static resources exposing KB data
 * via the `contextbuilder://` URI scheme.
 */
import { McpServer, ResourceTemplate } from "@modelcontextprotocol/sdk/server/mcp.js";
import type { KbReader } from "@contextbuilder/kb-reader";

// ---------------------------------------------------------------------------
// Resource registration
// ---------------------------------------------------------------------------

/**
 * Register all ContextBuilder resources and resource templates on the MCP server.
 *
 * URI scheme:
 * - `contextbuilder://kb/{kb_id}/docs/{path}` → single page (text/markdown)
 * - `contextbuilder://kb/{kb_id}/artifacts/{name}` → artifact (text/markdown or text/plain)
 * - `contextbuilder://kb/{kb_id}/toc` → TOC (application/json)
 *
 * @param server - McpServer instance.
 * @param getReaders - Returns the current KbReader map.
 */
export function registerResources(
  server: McpServer,
  getReaders: () => Map<string, KbReader>,
): void {
  // --- Page resource template ---
  server.registerResource(
    "kb-page",
    new ResourceTemplate("contextbuilder://kb/{kb_id}/docs/{+path}", {
      list: async () => {
        const resources: Array<{
          uri: string;
          name: string;
          mimeType: string;
          description?: string;
        }> = [];

        for (const [kbId, reader] of getReaders()) {
          const pages = await reader.listPages();
          for (const page of pages) {
            resources.push({
              uri: `contextbuilder://kb/${kbId}/docs/${page.path}`,
              name: page.title ?? page.path,
              mimeType: "text/markdown",
              description: `Page from ${reader.getManifest().name}`,
            });
          }
        }
        return { resources };
      },
    }),
    {
      description: "Documentation page from a ContextBuilder knowledge base",
      mimeType: "text/markdown",
    },
    async (uri, variables) => {
      const kbId = variables.kb_id as string;
      const pagePath = (variables.path ?? "") as string;
      const reader = getReaders().get(kbId);
      if (!reader) {
        throw new Error(`Knowledge base '${kbId}' not found`);
      }

      const page = await reader.getPage(pagePath);
      return {
        contents: [
          {
            uri: uri.href,
            mimeType: "text/markdown",
            text: page.content,
          },
        ],
      };
    },
  );

  // --- Artifact resource template ---
  server.registerResource(
    "kb-artifact",
    new ResourceTemplate("contextbuilder://kb/{kb_id}/artifacts/{name}", {
      list: async () => {
        const resources: Array<{
          uri: string;
          name: string;
          mimeType: string;
          description?: string;
        }> = [];

        for (const [kbId, reader] of getReaders()) {
          const artifacts = await reader.listArtifacts();
          for (const artifact of artifacts) {
            resources.push({
              uri: `contextbuilder://kb/${kbId}/artifacts/${artifact}`,
              name: artifact,
              mimeType: artifact.endsWith(".md") ? "text/markdown" : "text/plain",
              description: `Artifact from ${reader.getManifest().name}`,
            });
          }
        }
        return { resources };
      },
    }),
    {
      description: "Generated artifact from a ContextBuilder knowledge base",
      mimeType: "text/plain",
    },
    async (uri, variables) => {
      const kbId = variables.kb_id as string;
      const artifactName = variables.name as string;
      const reader = getReaders().get(kbId);
      if (!reader) {
        throw new Error(`Knowledge base '${kbId}' not found`);
      }

      const content = await reader.getArtifact(artifactName);
      const mimeType = artifactName.endsWith(".md") ? "text/markdown" : "text/plain";
      return {
        contents: [
          {
            uri: uri.href,
            mimeType,
            text: content,
          },
        ],
      };
    },
  );

  // --- TOC resource template ---
  server.registerResource(
    "kb-toc",
    new ResourceTemplate("contextbuilder://kb/{kb_id}/toc", {
      list: async () => {
        const resources: Array<{
          uri: string;
          name: string;
          mimeType: string;
          description?: string;
        }> = [];

        for (const [kbId, reader] of getReaders()) {
          resources.push({
            uri: `contextbuilder://kb/${kbId}/toc`,
            name: `${reader.getManifest().name} — Table of Contents`,
            mimeType: "application/json",
            description: `TOC for ${reader.getManifest().name}`,
          });
        }
        return { resources };
      },
    }),
    {
      description: "Table of contents for a ContextBuilder knowledge base",
      mimeType: "application/json",
    },
    async (uri, variables) => {
      const kbId = variables.kb_id as string;
      const reader = getReaders().get(kbId);
      if (!reader) {
        throw new Error(`Knowledge base '${kbId}' not found`);
      }

      const toc = reader.getToc();
      return {
        contents: [
          {
            uri: uri.href,
            mimeType: "application/json",
            text: JSON.stringify(toc, null, 2),
          },
        ],
      };
    },
  );
}
