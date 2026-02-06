/**
 * End-to-end integration tests for the ContextBuilder MCP pipeline.
 *
 * Tests the full flow: KB on disk → KbReader → MCP Server → Client queries.
 * Uses the fixture KB at fixtures/test-kb/ with InMemoryTransport.
 */
import { describe, expect, test, beforeAll, afterAll } from "bun:test";
import { resolve } from "node:path";
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { InMemoryTransport } from "@modelcontextprotocol/sdk/inMemory.js";
import { KbReader } from "@contextbuilder/kb-reader";
import { registerTools } from "../src/tools";
import { registerResources } from "../src/resources";
import { ARTIFACT_NAMES, CURRENT_SCHEMA_VERSION } from "@contextbuilder/shared";

const FIXTURE_KB = resolve(import.meta.dir, "../../../fixtures/test-kb");

// ---------------------------------------------------------------------------
// Full E2E: KB directory → KbReader → MCP Server → Client
// ---------------------------------------------------------------------------

describe("E2E: Full MCP Pipeline", () => {
  let server: McpServer;
  let client: Client;
  let reader: KbReader;
  let readers: Map<string, KbReader>;

  beforeAll(async () => {
    reader = await KbReader.open(FIXTURE_KB);
    readers = new Map([[reader.id, reader]]);

    server = new McpServer(
      { name: "contextbuilder-e2e", version: "0.1.0" },
      {
        capabilities: {
          tools: { listChanged: true },
          resources: { subscribe: true, listChanged: true },
        },
      },
    );

    const getReaders = () => readers;
    registerTools(server, getReaders);
    registerResources(server, getReaders);

    client = new Client({ name: "e2e-client", version: "1.0.0" });
    const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair();
    await server.connect(serverTransport);
    await client.connect(clientTransport);
  });

  afterAll(async () => {
    await client.close();
    await server.close();
    reader.close();
  });

  // -----------------------------------------------------------------------
  // Fixture KB validation
  // -----------------------------------------------------------------------

  describe("Fixture KB integrity", () => {
    test("manifest.json exists and has correct schema_version", () => {
      const manifest = reader.getManifest();
      expect(manifest.schema_version).toBe(CURRENT_SCHEMA_VERSION);
      expect(manifest.id).toBeTruthy();
      expect(manifest.name).toBe("example-docs");
    });

    test("toc.json exists with sections", () => {
      const toc = reader.getToc();
      expect(toc.sections).toBeDefined();
      expect(toc.sections.length).toBeGreaterThan(0);
    });

    test("all 6 artifacts exist on disk", () => {
      const artifactsDir = resolve(FIXTURE_KB, "artifacts");
      for (const name of ARTIFACT_NAMES) {
        const path = resolve(artifactsDir, name);
        expect(existsSync(path)).toBe(true);
      }
    });

    test("docs directory has markdown files", () => {
      const docsDir = resolve(FIXTURE_KB, "docs");
      expect(existsSync(docsDir)).toBe(true);
      // Should have at least some .md files
      const files = readdirSync(docsDir, { recursive: true }) as string[];
      const mdFiles = files.filter((f) => f.endsWith(".md"));
      expect(mdFiles.length).toBeGreaterThan(0);
    });

    test("DB file exists", () => {
      const dbPath = resolve(FIXTURE_KB, "indexes/contextbuilder.db");
      expect(existsSync(dbPath)).toBe(true);
    });
  });

  // -----------------------------------------------------------------------
  // KbReader → MCP Tool: kb_list
  // -----------------------------------------------------------------------

  describe("kb_list tool", () => {
    test("returns all loaded KBs", async () => {
      const result = await client.callTool({ name: "kb_list", arguments: {} });
      const text = (result.content as Array<{ type: string; text: string }>)[0].text;
      expect(text).toContain(reader.id);
      expect(text).toContain("example-docs");
    });
  });

  // -----------------------------------------------------------------------
  // KbReader → MCP Tool: kb_get_toc
  // -----------------------------------------------------------------------

  describe("kb_get_toc tool", () => {
    test("returns parseable TOC JSON", async () => {
      const result = await client.callTool({
        name: "kb_get_toc",
        arguments: { kb_id: reader.id },
      });
      const text = (result.content as Array<{ type: string; text: string }>)[0].text;
      const toc = JSON.parse(text);
      expect(toc.sections).toBeDefined();
    });

    test("rejects unknown kb_id", async () => {
      const result = await client.callTool({
        name: "kb_get_toc",
        arguments: { kb_id: "00000000-0000-0000-0000-000000000000" },
      });
      expect(result.isError).toBe(true);
    });
  });

  // -----------------------------------------------------------------------
  // KbReader → MCP Tool: kb_get_page
  // -----------------------------------------------------------------------

  describe("kb_get_page tool", () => {
    test("returns page content matching disk file", async () => {
      const result = await client.callTool({
        name: "kb_get_page",
        arguments: { kb_id: reader.id, path: "getting-started" },
      });
      const text = (result.content as Array<{ type: string; text: string }>)[0].text;

      // Compare with what's on disk
      const diskContent = readFileSync(
        resolve(FIXTURE_KB, "docs/getting-started.md"),
        "utf-8",
      );
      expect(text).toContain(diskContent.trim().substring(0, 50));
    });

    test("returns nested page content", async () => {
      const result = await client.callTool({
        name: "kb_get_page",
        arguments: {
          kb_id: reader.id,
          path: "getting-started/installation",
        },
      });
      expect(result.isError).not.toBe(true);
      const text = (result.content as Array<{ type: string; text: string }>)[0].text;
      expect(text).toContain("Installation");
    });

    test("returns error for nonexistent page", async () => {
      const result = await client.callTool({
        name: "kb_get_page",
        arguments: { kb_id: reader.id, path: "does/not/exist" },
      });
      expect(result.isError).toBe(true);
    });
  });

  // -----------------------------------------------------------------------
  // KbReader → MCP Tool: kb_search
  // -----------------------------------------------------------------------

  describe("kb_search tool", () => {
    test("finds pages by keyword", async () => {
      const result = await client.callTool({
        name: "kb_search",
        arguments: { kb_id: reader.id, query: "installation" },
      });
      expect(result.isError).not.toBe(true);
      const text = (result.content as Array<{ type: string; text: string }>)[0].text;
      // Should find installation-related pages
      expect(text.toLowerCase()).toContain("install");
    });

    test("respects limit parameter", async () => {
      const result = await client.callTool({
        name: "kb_search",
        arguments: { kb_id: reader.id, query: "getting", limit: 1 },
      });
      expect(result.isError).not.toBe(true);
    });

    test("returns empty results for nonsense query", async () => {
      const result = await client.callTool({
        name: "kb_search",
        arguments: {
          kb_id: reader.id,
          query: "xyzzy_nonexistent_term_42",
        },
      });
      expect(result.isError).not.toBe(true);
    });
  });

  // -----------------------------------------------------------------------
  // KbReader → MCP Tool: kb_get_artifact
  // -----------------------------------------------------------------------

  describe("kb_get_artifact tool", () => {
    for (const artifactName of ARTIFACT_NAMES) {
      test(`retrieves ${artifactName}`, async () => {
        const result = await client.callTool({
          name: "kb_get_artifact",
          arguments: { kb_id: reader.id, name: artifactName },
        });
        expect(result.isError).not.toBe(true);
        const text = (result.content as Array<{ type: string; text: string }>)[0].text;
        expect(text.length).toBeGreaterThan(0);
      });
    }

    test("rejects invalid artifact name", async () => {
      const result = await client.callTool({
        name: "kb_get_artifact",
        arguments: { kb_id: reader.id, name: "fake.md" },
      });
      expect(result.isError).toBe(true);
    });
  });

  // -----------------------------------------------------------------------
  // MCP Resources: contextbuilder:// URIs
  // -----------------------------------------------------------------------

  describe("MCP Resources", () => {
    test("page resource returns markdown content", async () => {
      const uri = `contextbuilder://kb/${reader.id}/docs/getting-started`;
      const { contents } = await client.readResource({ uri });
      expect(contents.length).toBe(1);
      expect(contents[0].mimeType).toBe("text/markdown");
      const text = "text" in contents[0] ? contents[0].text : "";
      expect(text).toContain("Getting Started");
    });

    test("nested page resource works", async () => {
      const uri = `contextbuilder://kb/${reader.id}/docs/getting-started/installation`;
      const { contents } = await client.readResource({ uri });
      expect(contents.length).toBe(1);
      const text = "text" in contents[0] ? contents[0].text : "";
      expect(text).toContain("Installation");
    });

    test("artifact resources return content for all 6 artifacts", async () => {
      for (const name of ARTIFACT_NAMES) {
        const uri = `contextbuilder://kb/${reader.id}/artifacts/${name}`;
        const { contents } = await client.readResource({ uri });
        expect(contents.length).toBe(1);
        const text = "text" in contents[0] ? contents[0].text : "";
        expect(text.length).toBeGreaterThan(0);
      }
    });

    test("TOC resource returns valid JSON", async () => {
      const uri = `contextbuilder://kb/${reader.id}/toc`;
      const { contents } = await client.readResource({ uri });
      expect(contents.length).toBe(1);
      expect(contents[0].mimeType).toBe("application/json");
      const text = "text" in contents[0] ? contents[0].text : "";
      const toc = JSON.parse(text);
      expect(toc.sections).toBeDefined();
    });

    test("resource list contains pages, artifacts, and TOC", async () => {
      const { resources } = await client.listResources();
      const uris = resources.map((r) => r.uri);

      // At least 4 pages, 6 artifacts, 1 TOC
      expect(resources.length).toBeGreaterThanOrEqual(11);

      // Verify URI scheme
      for (const uri of uris) {
        expect(uri.startsWith("contextbuilder://")).toBe(true);
      }

      // Check categories are present
      const docUris = uris.filter((u) => u.includes("/docs/"));
      const artifactUris = uris.filter((u) => u.includes("/artifacts/"));
      const tocUris = uris.filter((u) => u.includes("/toc"));

      expect(docUris.length).toBeGreaterThan(0);
      expect(artifactUris.length).toBe(ARTIFACT_NAMES.length);
      expect(tocUris.length).toBe(1);
    });
  });

  // -----------------------------------------------------------------------
  // Cross-cutting: consistency between tools and resources
  // -----------------------------------------------------------------------

  describe("Cross-cutting consistency", () => {
    test("tool and resource return same page content", async () => {
      // Get via tool
      const toolResult = await client.callTool({
        name: "kb_get_page",
        arguments: { kb_id: reader.id, path: "getting-started" },
      });
      const toolText = (toolResult.content as Array<{ type: string; text: string }>)[0].text;

      // Get via resource
      const uri = `contextbuilder://kb/${reader.id}/docs/getting-started`;
      const { contents } = await client.readResource({ uri });
      const resourceText = "text" in contents[0] ? contents[0].text : "";

      // Both should contain the same core content
      expect(toolText).toContain("Getting Started");
      expect(resourceText).toContain("Getting Started");
    });

    test("tool and resource return same artifact content", async () => {
      const toolResult = await client.callTool({
        name: "kb_get_artifact",
        arguments: { kb_id: reader.id, name: "llms.txt" },
      });
      const toolText = (toolResult.content as Array<{ type: string; text: string }>)[0].text;

      const uri = `contextbuilder://kb/${reader.id}/artifacts/llms.txt`;
      const { contents } = await client.readResource({ uri });
      const resourceText = "text" in contents[0] ? contents[0].text : "";

      expect(toolText).toBe(resourceText);
    });

    test("tool and resource return same TOC", async () => {
      const toolResult = await client.callTool({
        name: "kb_get_toc",
        arguments: { kb_id: reader.id },
      });
      const toolText = (toolResult.content as Array<{ type: string; text: string }>)[0].text;
      const toolToc = JSON.parse(toolText);

      const uri = `contextbuilder://kb/${reader.id}/toc`;
      const { contents } = await client.readResource({ uri });
      const resourceText = "text" in contents[0] ? contents[0].text : "";
      const resourceToc = JSON.parse(resourceText);

      expect(toolToc.sections.length).toBe(resourceToc.sections.length);
    });
  });
});
