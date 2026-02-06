/**
 * Schema validation tests — validates fixture files against zod schemas.
 */
import { describe, expect, test } from "bun:test";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { KbManifestSchema, CURRENT_SCHEMA_VERSION } from "../manifest/src/index";
import { TocSchema } from "../toc/src/index";
import { SkillMetaSchema, LlmsMetaSchema } from "../artifacts/src/index";
import {
  SearchDocsInputSchema,
  GetPageInputSchema,
  GetArtifactInputSchema,
  ListKbsOutputSchema,
  SearchDocsOutputSchema,
} from "../mcp/src/index";

const FIXTURES_DIR = resolve(import.meta.dir, "../../../fixtures/json");

async function loadFixture(name: string): Promise<unknown> {
  const raw = await readFile(resolve(FIXTURES_DIR, name), "utf-8");
  return JSON.parse(raw);
}

describe("manifest schema", () => {
  test("validates fixture manifest.json", async () => {
    const data = await loadFixture("manifest.fixture.json");
    const result = KbManifestSchema.safeParse(data);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.schema_version).toBe(CURRENT_SCHEMA_VERSION);
      expect(result.data.name).toBe("example-docs");
      expect(result.data.page_count).toBe(3);
    }
  });

  test("rejects manifest with missing required fields", () => {
    const result = KbManifestSchema.safeParse({ name: "test" });
    expect(result.success).toBe(false);
  });

  test("rejects manifest with invalid schema_version", () => {
    const result = KbManifestSchema.safeParse({
      schema_version: 0,
      id: "019748d2-b3f0-7000-8000-000000000001",
      name: "test",
      source_url: "https://example.com",
      tool_version: "0.1.0",
      created_at: "2025-01-01T00:00:00Z",
      updated_at: "2025-01-01T00:00:00Z",
      page_count: 0,
    });
    expect(result.success).toBe(false);
  });
});

describe("toc schema", () => {
  test("validates fixture toc.json", async () => {
    const data = await loadFixture("toc.fixture.json");
    const result = TocSchema.safeParse(data);
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.sections).toHaveLength(2);
      expect(result.data.sections[0].children).toHaveLength(2);
    }
  });

  test("rejects toc without sections", () => {
    const result = TocSchema.safeParse({});
    expect(result.success).toBe(false);
  });
});

describe("artifact schemas", () => {
  test("validates skill metadata", () => {
    const data = {
      name: "react-hooks",
      description: "React Hooks patterns and best practices.",
      version: "0.1.0",
      source_url: "https://react.dev/reference/react",
      kb_id: "019748d2-b3f0-7000-8000-000000000001",
      generated_at: "2025-01-01T00:00:00Z",
      model_id: "moonshotai/kimi-k2.5",
      topics: ["hooks", "state", "effects"],
    };
    const result = SkillMetaSchema.safeParse(data);
    expect(result.success).toBe(true);
  });

  test("validates llms.txt metadata", () => {
    const data = {
      source_url: "https://example.com/docs",
      generated_at: "2025-01-01T00:00:00Z",
      page_count: 42,
      variant: "llms.txt",
    };
    const result = LlmsMetaSchema.safeParse(data);
    expect(result.success).toBe(true);
  });
});

describe("MCP tool schemas", () => {
  test("validates search_docs input", () => {
    const result = SearchDocsInputSchema.safeParse({
      kb_id: "kb-123",
      query: "installation",
    });
    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.data.limit).toBe(10); // default
    }
  });

  test("validates get_page input", () => {
    const result = GetPageInputSchema.safeParse({
      kb_id: "kb-123",
      path: "getting-started/installation",
    });
    expect(result.success).toBe(true);
  });

  test("validates get_artifact input", () => {
    const result = GetArtifactInputSchema.safeParse({
      kb_id: "kb-123",
      artifact_name: "SKILL.md",
    });
    expect(result.success).toBe(true);
  });

  test("rejects invalid artifact name", () => {
    const result = GetArtifactInputSchema.safeParse({
      kb_id: "kb-123",
      artifact_name: "invalid.txt",
    });
    expect(result.success).toBe(false);
  });

  test("validates list_kbs output", () => {
    const result = ListKbsOutputSchema.safeParse({
      kbs: [
        { id: "kb-1", name: "React Docs", source_url: "https://react.dev" },
        { id: "kb-2", name: "Vue Docs", source_url: "https://vuejs.org" },
      ],
    });
    expect(result.success).toBe(true);
  });

  test("validates search_docs output", () => {
    const result = SearchDocsOutputSchema.safeParse({
      results: [
        { path: "installation", title: "Installation Guide", score: -1.5 },
        { path: "quickstart", title: null, score: -0.8 },
      ],
    });
    expect(result.success).toBe(true);
  });
});
