/**
 * MCP server integration tests.
 *
 * Tests tool and resource registration using the MCP SDK Client + InMemoryTransport.
 */
import { describe, expect, test, beforeAll, afterAll } from "bun:test";
import { resolve } from "node:path";
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { InMemoryTransport } from "@modelcontextprotocol/sdk/inMemory.js";
import { KbReader } from "@contextbuilder/kb-reader";
import { registerTools } from "../src/tools";
import { registerResources } from "../src/resources";

const FIXTURE_KB = resolve(import.meta.dir, "../../../fixtures/test-kb");

describe("MCP Server", () => {
  let server: McpServer;
  let client: Client;
  let reader: KbReader;
  let readers: Map<string, KbReader>;

  beforeAll(async () => {
    // Open fixture KB
    reader = await KbReader.open(FIXTURE_KB);
    readers = new Map([[reader.id, reader]]);

    // Create server with tools and resources
    server = new McpServer(
      { name: "contextbuilder-test", version: "0.1.0" },
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

    // Connect via in-memory transport
    client = new Client({ name: "test-client", version: "1.0.0" });
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
  // Tool: kb_list
  // -----------------------------------------------------------------------

  test("kb_list returns loaded KBs", async () => {
    const result = await client.callTool({ name: "kb_list", arguments: {} });
    expect(result.content).toBeDefined();
    const textContent = result.content as Array<{ type: string; text: string }>;
    expect(textContent.length).toBeGreaterThan(0);
    expect(textContent[0].text).toContain("example-docs");
  });

  // -----------------------------------------------------------------------
  // Tool: kb_get_toc
  // -----------------------------------------------------------------------

  test("kb_get_toc returns TOC for valid KB", async () => {
    const result = await client.callTool({
      name: "kb_get_toc",
      arguments: { kb_id: reader.id },
    });
    const textContent = result.content as Array<{ type: string; text: string }>;
    const toc = JSON.parse(textContent[0].text);
    expect(toc.sections).toBeDefined();
    expect(toc.sections.length).toBeGreaterThan(0);
  });

  test("kb_get_toc returns error for invalid KB", async () => {
    const result = await client.callTool({
      name: "kb_get_toc",
      arguments: { kb_id: "nonexistent-id" },
    });
    expect(result.isError).toBe(true);
  });

  // -----------------------------------------------------------------------
  // Tool: kb_get_page
  // -----------------------------------------------------------------------

  test("kb_get_page returns page content", async () => {
    const result = await client.callTool({
      name: "kb_get_page",
      arguments: { kb_id: reader.id, path: "getting-started" },
    });
    const textContent = result.content as Array<{ type: string; text: string }>;
    expect(textContent[0].text).toContain("Getting Started");
  });

  test("kb_get_page returns error for missing page", async () => {
    const result = await client.callTool({
      name: "kb_get_page",
      arguments: { kb_id: reader.id, path: "nonexistent" },
    });
    expect(result.isError).toBe(true);
  });

  // -----------------------------------------------------------------------
  // Tool: kb_search
  // -----------------------------------------------------------------------

  test("kb_search returns results", async () => {
    const result = await client.callTool({
      name: "kb_search",
      arguments: { kb_id: reader.id, query: "installation" },
    });
    const textContent = result.content as Array<{ type: string; text: string }>;
    expect(textContent[0].text).toContain("Installation");
  });

  test("kb_search returns error for invalid KB", async () => {
    const result = await client.callTool({
      name: "kb_search",
      arguments: { kb_id: "bad-id", query: "test" },
    });
    expect(result.isError).toBe(true);
  });

  // -----------------------------------------------------------------------
  // Tool: kb_get_artifact
  // -----------------------------------------------------------------------

  test("kb_get_artifact returns artifact content", async () => {
    const result = await client.callTool({
      name: "kb_get_artifact",
      arguments: { kb_id: reader.id, name: "llms.txt" },
    });
    const textContent = result.content as Array<{ type: string; text: string }>;
    expect(textContent[0].text).toContain("Example Docs");
  });

  test("kb_get_artifact returns error for missing artifact", async () => {
    const result = await client.callTool({
      name: "kb_get_artifact",
      arguments: { kb_id: reader.id, name: "rules.md" },
    });
    // rules.md should exist in the fixture
    expect(result.isError).not.toBe(true);
  });

  // -----------------------------------------------------------------------
  // Tools listing
  // -----------------------------------------------------------------------

  test("listTools returns all 5 tools", async () => {
    const { tools } = await client.listTools();
    const names = tools.map((t) => t.name);
    expect(names).toContain("kb_list");
    expect(names).toContain("kb_get_toc");
    expect(names).toContain("kb_get_page");
    expect(names).toContain("kb_search");
    expect(names).toContain("kb_get_artifact");
    expect(tools.length).toBe(5);
  });

  // -----------------------------------------------------------------------
  // Resources
  // -----------------------------------------------------------------------

  test("listResourceTemplates returns templates", async () => {
    const { resourceTemplates } = await client.listResourceTemplates();
    expect(resourceTemplates.length).toBeGreaterThanOrEqual(3);
    const uris = resourceTemplates.map((t) => t.uriTemplate);
    expect(uris.some((u) => u.includes("docs"))).toBe(true);
    expect(uris.some((u) => u.includes("artifacts"))).toBe(true);
    expect(uris.some((u) => u.includes("toc"))).toBe(true);
  });

  test("listResources returns resources from all templates", async () => {
    const { resources } = await client.listResources();
    expect(resources.length).toBeGreaterThan(0);
    // Should have pages + artifacts + TOC
    const uris = resources.map((r) => r.uri);
    expect(uris.some((u) => u.includes("/docs/"))).toBe(true);
    expect(uris.some((u) => u.includes("/artifacts/"))).toBe(true);
    expect(uris.some((u) => u.includes("/toc"))).toBe(true);
  });

  test("readResource returns page content", async () => {
    const uri = `contextbuilder://kb/${reader.id}/docs/getting-started`;
    const { contents } = await client.readResource({ uri });
    expect(contents.length).toBe(1);
    const content = contents[0];
    expect("text" in content ? content.text : "").toContain("Getting Started");
  });

  test("readResource returns artifact content", async () => {
    const uri = `contextbuilder://kb/${reader.id}/artifacts/llms.txt`;
    const { contents } = await client.readResource({ uri });
    expect(contents.length).toBe(1);
    const content = contents[0];
    expect("text" in content ? content.text : "").toContain("Example Docs");
  });

  test("readResource returns TOC", async () => {
    const uri = `contextbuilder://kb/${reader.id}/toc`;
    const { contents } = await client.readResource({ uri });
    expect(contents.length).toBe(1);
    const content = contents[0];
    const text = "text" in content ? content.text : "";
    const toc = JSON.parse(text);
    expect(toc.sections).toBeDefined();
  });
});
