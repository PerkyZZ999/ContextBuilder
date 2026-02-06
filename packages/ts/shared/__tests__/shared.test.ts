/**
 * Tests for @contextbuilder/shared — types, constants, and loaders.
 */
import { describe, expect, test } from "bun:test";
import { resolve } from "node:path";
import {
  KbManifestSchema,
  TocSchema,
  PageMetaSchema,
  CURRENT_SCHEMA_VERSION,
  ARTIFACT_NAMES,
  DEFAULT_CONFIG,
  loadManifest,
  loadToc,
  readPage,
  validateManifestVersion,
} from "../src/index";

const TEST_KB_DIR = resolve(import.meta.dir, "../../../../fixtures/test-kb");

describe("types", () => {
  test("KbManifestSchema validates correctly", () => {
    const valid = {
      schema_version: 1,
      id: "019748d2-b3f0-7000-8000-000000000001",
      name: "test",
      source_url: "https://example.com",
      tool_version: "0.1.0",
      created_at: "2025-01-01T00:00:00Z",
      updated_at: "2025-01-01T00:00:00Z",
      page_count: 0,
    };
    expect(KbManifestSchema.safeParse(valid).success).toBe(true);
  });

  test("TocSchema validates nested entries", () => {
    const toc = {
      sections: [
        {
          title: "Root",
          path: "root",
          children: [{ title: "Child", path: "root/child", children: [] }],
        },
      ],
    };
    expect(TocSchema.safeParse(toc).success).toBe(true);
  });

  test("PageMetaSchema validates correctly", () => {
    const page = {
      id: "page-1",
      kb_id: "kb-1",
      url: "https://example.com/page",
      path: "page",
      title: "Test Page",
      content_hash: "abc123",
      fetched_at: "2025-01-01T00:00:00Z",
      status_code: 200,
      content_len: 1024,
    };
    expect(PageMetaSchema.safeParse(page).success).toBe(true);
  });
});

describe("constants", () => {
  test("CURRENT_SCHEMA_VERSION is 1", () => {
    expect(CURRENT_SCHEMA_VERSION).toBe(1);
  });

  test("ARTIFACT_NAMES has 6 entries", () => {
    expect(ARTIFACT_NAMES).toHaveLength(6);
    expect(ARTIFACT_NAMES).toContain("SKILL.md");
    expect(ARTIFACT_NAMES).toContain("llms.txt");
  });

  test("DEFAULT_CONFIG has expected values", () => {
    expect(DEFAULT_CONFIG.crawl_depth).toBe(3);
    expect(DEFAULT_CONFIG.concurrency).toBe(4);
    expect(DEFAULT_CONFIG.model).toBe("moonshotai/kimi-k2.5");
  });
});

describe("loaders", () => {
  test("loadManifest reads and validates", async () => {
    const manifest = await loadManifest(TEST_KB_DIR);
    expect(manifest.schema_version).toBe(1);
    expect(manifest.name).toBe("example-docs");
    expect(manifest.page_count).toBe(3);
  });

  test("loadToc reads and validates", async () => {
    const toc = await loadToc(TEST_KB_DIR);
    expect(toc.sections).toHaveLength(2);
    expect(toc.sections[0].title).toBe("Getting Started");
  });

  test("readPage reads markdown content", async () => {
    const content = await readPage(TEST_KB_DIR, "getting-started");
    expect(content).toContain("Getting Started");
  });

  test("validateManifestVersion accepts current version", async () => {
    const manifest = await loadManifest(TEST_KB_DIR);
    expect(validateManifestVersion(manifest)).toBe(true);
  });

  test("validateManifestVersion rejects future version", () => {
    const futureManifest = {
      schema_version: 999,
      id: "019748d2-b3f0-7000-8000-000000000001",
      name: "future",
      source_url: "https://example.com",
      tool_version: "99.0.0",
      created_at: "2025-01-01T00:00:00Z",
      updated_at: "2025-01-01T00:00:00Z",
      page_count: 0,
    };
    const parsed = KbManifestSchema.parse(futureManifest);
    expect(validateManifestVersion(parsed)).toBe(false);
  });
});
