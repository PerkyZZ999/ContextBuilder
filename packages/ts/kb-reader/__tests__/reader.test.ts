import { describe, expect, test, beforeAll, afterAll } from "bun:test";
import { resolve } from "node:path";
import { KbReader } from "../src/reader";

const FIXTURE_KB = resolve(import.meta.dir, "../../../../fixtures/test-kb");

describe("KbReader", () => {
  let reader: KbReader;

  beforeAll(async () => {
    reader = await KbReader.open(FIXTURE_KB);
  });

  afterAll(() => {
    reader.close();
  });

  // -----------------------------------------------------------------------
  // Opening & validation
  // -----------------------------------------------------------------------

  test("opens fixture KB successfully", () => {
    expect(reader).toBeDefined();
    expect(reader.id).toBe("019748d2-b3f0-7000-8000-000000000001");
  });

  test("rejects missing KB directory", async () => {
    await expect(KbReader.open("/tmp/nonexistent-kb-path")).rejects.toThrow("not found");
  });

  // -----------------------------------------------------------------------
  // Manifest
  // -----------------------------------------------------------------------

  test("getManifest returns valid manifest", () => {
    const manifest = reader.getManifest();
    expect(manifest.schema_version).toBe(1);
    expect(manifest.name).toBe("example-docs");
    expect(manifest.source_url).toBe("https://example.com/docs");
    expect(manifest.page_count).toBe(3);
    expect(manifest.id).toBe("019748d2-b3f0-7000-8000-000000000001");
  });

  // -----------------------------------------------------------------------
  // TOC
  // -----------------------------------------------------------------------

  test("getToc returns valid TOC", () => {
    const toc = reader.getToc();
    expect(toc.sections.length).toBeGreaterThan(0);
    const gettingStarted = toc.sections.find((s) => s.path === "getting-started");
    expect(gettingStarted).toBeDefined();
    expect(gettingStarted?.children.length).toBeGreaterThan(0);
  });

  test("getTocPaths returns all page paths", () => {
    const paths = reader.getTocPaths();
    expect(paths).toContain("getting-started");
    expect(paths).toContain("getting-started/installation");
    expect(paths).toContain("api-reference");
  });

  // -----------------------------------------------------------------------
  // Pages
  // -----------------------------------------------------------------------

  test("getPage reads markdown and metadata", async () => {
    const page = await reader.getPage("getting-started");
    expect(page.path).toBe("getting-started");
    expect(page.content).toContain("Getting Started");
    expect(page.title).toBe("Getting Started");
    expect(page.source_url).toBe("https://example.com/docs/getting-started");
  });

  test("getPage reads nested page", async () => {
    const page = await reader.getPage("getting-started/installation");
    expect(page.content).toContain("Installation");
    expect(page.title).toBe("Installation");
  });

  test("getPage throws for missing page", async () => {
    await expect(reader.getPage("nonexistent-page")).rejects.toThrow("not found");
  });

  test("listPages returns all pages from DB", async () => {
    const pages = await reader.listPages();
    expect(pages.length).toBe(4);
    const paths = pages.map((p) => p.path);
    expect(paths).toContain("getting-started");
    expect(paths).toContain("api-reference");
  });

  // -----------------------------------------------------------------------
  // Artifacts
  // -----------------------------------------------------------------------

  test("getArtifact reads llms.txt", async () => {
    const content = await reader.getArtifact("llms.txt");
    expect(content).toContain("Example Docs");
  });

  test("getArtifact reads SKILL.md", async () => {
    const content = await reader.getArtifact("SKILL.md");
    expect(content).toContain("example-docs");
  });

  test("getArtifact throws for invalid name", async () => {
    await expect(reader.getArtifact("invalid.txt")).rejects.toThrow("Invalid artifact name");
  });

  test("listArtifacts finds all fixture artifacts", async () => {
    const artifacts = await reader.listArtifacts();
    expect(artifacts).toContain("llms.txt");
    expect(artifacts).toContain("SKILL.md");
    expect(artifacts).toContain("rules.md");
  });

  // -----------------------------------------------------------------------
  // Search
  // -----------------------------------------------------------------------

  test("search finds pages by title", async () => {
    const results = await reader.search("installation");
    expect(results.length).toBeGreaterThan(0);
    expect(results[0].path).toBe("getting-started/installation");
  });

  test("search respects limit", async () => {
    const results = await reader.search("getting", 1);
    expect(results.length).toBeLessThanOrEqual(1);
  });

  test("search clamps limit to 50", async () => {
    // Should not throw even with large limit
    const results = await reader.search("docs", 999);
    expect(results.length).toBeLessThanOrEqual(50);
  });

  // -----------------------------------------------------------------------
  // Summary
  // -----------------------------------------------------------------------

  test("getSummary returns correct info", () => {
    const summary = reader.getSummary();
    expect(summary.id).toBe("019748d2-b3f0-7000-8000-000000000001");
    expect(summary.name).toBe("example-docs");
    expect(summary.source_url).toBe("https://example.com/docs");
    expect(summary.page_count).toBe(3);
  });

  // -----------------------------------------------------------------------
  // Discovery
  // -----------------------------------------------------------------------

  test("discoverKbs finds KBs in a directory", async () => {
    const fixturesRoot = resolve(FIXTURE_KB, "..");
    const readers = await KbReader.discoverKbs(fixturesRoot);
    expect(readers.length).toBeGreaterThanOrEqual(1);
    const names = readers.map((r) => r.getManifest().name);
    expect(names).toContain("example-docs");
    for (const r of readers) r.close();
  });

  test("discoverKbs returns empty for nonexistent dir", async () => {
    const readers = await KbReader.discoverKbs("/tmp/nonexistent-kbs");
    expect(readers).toEqual([]);
  });
});
