/**
 * Tests for the enrichment bridge schemas and protocol.
 */
import { describe, expect, test } from "bun:test";
import {
  RequestMessageSchema,
  ResponseMessageSchema,
  EnrichmentTaskSchema,
  EnrichmentResultSchema,
  TASK_TYPES,
} from "../src/schemas";
import { buildPrompt, SYSTEM_PROMPT } from "../src/prompts";

// ---------------------------------------------------------------------------
// Schema validation tests
// ---------------------------------------------------------------------------

describe("RequestMessageSchema", () => {
  test("validates enrich message", () => {
    const msg = {
      type: "enrich",
      id: "req-001",
      task: {
        task_type: "summarize_page",
        content: "# Hello\n\nThis is a page.",
        title: "Hello",
        source_url: "https://example.com/hello",
      },
    };
    const result = RequestMessageSchema.safeParse(msg);
    expect(result.success).toBe(true);
  });

  test("validates shutdown message", () => {
    const msg = { type: "shutdown" };
    const result = RequestMessageSchema.safeParse(msg);
    expect(result.success).toBe(true);
  });

  test("rejects unknown type", () => {
    const msg = { type: "unknown", id: "req-001" };
    const result = RequestMessageSchema.safeParse(msg);
    expect(result.success).toBe(false);
  });

  test("rejects enrich without id", () => {
    const msg = {
      type: "enrich",
      task: { task_type: "summarize_page", content: "test" },
    };
    const result = RequestMessageSchema.safeParse(msg);
    expect(result.success).toBe(false);
  });

  test("rejects enrich with invalid task_type", () => {
    const msg = {
      type: "enrich",
      id: "req-001",
      task: { task_type: "invalid_task", content: "test" },
    };
    const result = RequestMessageSchema.safeParse(msg);
    expect(result.success).toBe(false);
  });
});

describe("ResponseMessageSchema", () => {
  test("validates result message", () => {
    const msg = {
      type: "result",
      id: "req-001",
      result: {
        text: "This page explains...",
        tokens_in: 100,
        tokens_out: 50,
        model: "moonshotai/kimi-k2.5",
        latency_ms: 1200,
      },
    };
    const result = ResponseMessageSchema.safeParse(msg);
    expect(result.success).toBe(true);
  });

  test("validates error message", () => {
    const msg = {
      type: "error",
      id: "req-001",
      error: "API rate limited",
    };
    const result = ResponseMessageSchema.safeParse(msg);
    expect(result.success).toBe(true);
  });

  test("validates ready message", () => {
    const msg = { type: "ready" };
    const result = ResponseMessageSchema.safeParse(msg);
    expect(result.success).toBe(true);
  });
});

describe("EnrichmentTaskSchema", () => {
  test("validates all task types", () => {
    for (const taskType of TASK_TYPES) {
      const task = { task_type: taskType };
      const result = EnrichmentTaskSchema.safeParse(task);
      expect(result.success).toBe(true);
    }
  });

  test("validates task with all optional fields", () => {
    const task = {
      task_type: "summarize_page" as const,
      content: "# Page content",
      title: "Test Page",
      source_url: "https://example.com",
      toc_json: "[]",
      summaries_json: "{}",
      pages_json: "[]",
      kb_name: "test-kb",
      kb_source_url: "https://example.com",
    };
    const result = EnrichmentTaskSchema.safeParse(task);
    expect(result.success).toBe(true);
  });
});

describe("EnrichmentResultSchema", () => {
  test("validates complete result", () => {
    const result = {
      text: "Summary of the page",
      tokens_in: 500,
      tokens_out: 100,
      model: "moonshotai/kimi-k2.5",
      latency_ms: 2000,
    };
    expect(EnrichmentResultSchema.safeParse(result).success).toBe(true);
  });

  test("rejects negative tokens", () => {
    const result = {
      text: "Summary",
      tokens_in: -1,
      tokens_out: 100,
      model: "test",
      latency_ms: 100,
    };
    expect(EnrichmentResultSchema.safeParse(result).success).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Prompt builder tests
// ---------------------------------------------------------------------------

describe("buildPrompt", () => {
  test("system prompt is non-empty", () => {
    expect(SYSTEM_PROMPT.length).toBeGreaterThan(0);
  });

  test("summarize_page includes content", () => {
    const prompt = buildPrompt({
      task_type: "summarize_page",
      content: "# Getting Started\n\nInstall with npm.",
      title: "Getting Started",
      source_url: "https://docs.example.com/getting-started",
    });
    expect(prompt).toContain("Getting Started");
    expect(prompt).toContain("Install with npm");
    expect(prompt).toContain("summary");
  });

  test("generate_description produces short description prompt", () => {
    const prompt = buildPrompt({
      task_type: "generate_description",
      content: "# API Reference\n\nComplete API docs.",
      title: "API Reference",
    });
    expect(prompt).toContain("single-line description");
    expect(prompt).toContain("120 characters");
  });

  test("generate_skill_md includes KB context", () => {
    const prompt = buildPrompt({
      task_type: "generate_skill_md",
      kb_name: "My Docs",
      kb_source_url: "https://docs.example.com",
      toc_json: '[{"title":"Intro","path":"intro"}]',
      summaries_json: '{"intro":"Introduction page"}',
    });
    expect(prompt).toContain("My Docs");
    expect(prompt).toContain("SKILL.md");
    expect(prompt).toContain("Agent Skills");
  });

  test("generate_llms_txt includes TOC and summaries", () => {
    const prompt = buildPrompt({
      task_type: "generate_llms_txt",
      kb_name: "Test KB",
      toc_json: "[]",
      summaries_json: "{}",
    });
    expect(prompt).toContain("llms.txt");
    expect(prompt).toContain("llmstxt.org");
    expect(prompt).toContain("Test KB");
  });

  test("all task types produce non-empty prompts", () => {
    for (const taskType of TASK_TYPES) {
      const prompt = buildPrompt({
        task_type: taskType,
        content: "Test content",
        kb_name: "Test",
      });
      expect(prompt.length).toBeGreaterThan(10);
    }
  });
});
