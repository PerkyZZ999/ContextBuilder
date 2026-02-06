/**
 * @module schemas
 * Zod schemas for the enrichment bridge JSON-lines protocol (stdin/stdout).
 */
import { z } from "zod";

// ---------------------------------------------------------------------------
// Enrichment task types
// ---------------------------------------------------------------------------

/** Supported enrichment task types. */
export const TASK_TYPES = [
  "summarize_page",
  "generate_description",
  "generate_skill_md",
  "generate_rules",
  "generate_style",
  "generate_do_dont",
  "generate_llms_txt",
  "generate_llms_full_txt",
] as const;

export type TaskType = (typeof TASK_TYPES)[number];

// ---------------------------------------------------------------------------
// Request schemas
// ---------------------------------------------------------------------------

/** An enrichment task sent from Rust to the bridge. */
export const EnrichmentTaskSchema = z.object({
  /** One-line description target. */
  task_type: z.enum(TASK_TYPES),
  /** Page content (markdown) for page-level tasks. */
  content: z.string().optional(),
  /** Page title. */
  title: z.string().optional(),
  /** Source URL. */
  source_url: z.string().optional(),
  /** TOC JSON for KB-level tasks. */
  toc_json: z.string().optional(),
  /** Summaries JSON for llms.txt generation. */
  summaries_json: z.string().optional(),
  /** All pages content JSON for llms-full.txt generation. */
  pages_json: z.string().optional(),
  /** KB name for context. */
  kb_name: z.string().optional(),
  /** KB source URL for context. */
  kb_source_url: z.string().optional(),
});

export type EnrichmentTask = z.infer<typeof EnrichmentTaskSchema>;

/** A protocol message from Rust to the bridge. */
export const RequestMessageSchema = z.discriminatedUnion("type", [
  z.object({
    type: z.literal("enrich"),
    id: z.string(),
    task: EnrichmentTaskSchema,
  }),
  z.object({
    type: z.literal("shutdown"),
  }),
]);

export type RequestMessage = z.infer<typeof RequestMessageSchema>;

// ---------------------------------------------------------------------------
// Response schemas
// ---------------------------------------------------------------------------

/** Successful enrichment result. */
export const EnrichmentResultSchema = z.object({
  /** Generated text output. */
  text: z.string(),
  /** Input tokens used. */
  tokens_in: z.number().int().min(0),
  /** Output tokens used. */
  tokens_out: z.number().int().min(0),
  /** Model that produced this result. */
  model: z.string(),
  /** Latency in milliseconds. */
  latency_ms: z.number().int().min(0),
});

export type EnrichmentResult = z.infer<typeof EnrichmentResultSchema>;

/** A protocol message from the bridge back to Rust. */
export const ResponseMessageSchema = z.discriminatedUnion("type", [
  z.object({
    type: z.literal("result"),
    id: z.string(),
    result: EnrichmentResultSchema,
  }),
  z.object({
    type: z.literal("error"),
    id: z.string(),
    error: z.string(),
  }),
  z.object({
    type: z.literal("ready"),
  }),
]);

export type ResponseMessage = z.infer<typeof ResponseMessageSchema>;

// ---------------------------------------------------------------------------
// Structured output schemas (for LLM responses)
// ---------------------------------------------------------------------------

/** Schema for page summary output. */
export const PageSummaryOutputSchema = z.object({
  summary: z.string().describe("A concise 1-3 sentence summary of the page content."),
});

/** Schema for page description output. */
export const PageDescriptionOutputSchema = z.object({
  description: z
    .string()
    .describe("A single-line description suitable for an llms.txt entry."),
});

/** Schema for SKILL.md output. */
export const SkillMdOutputSchema = z.object({
  content: z.string().describe("Full SKILL.md content following Agent Skills specification."),
});

/** Schema for rules output. */
export const RulesOutputSchema = z.object({
  content: z
    .string()
    .describe("Coding rules and conventions extracted as actionable directives."),
});

/** Schema for style output. */
export const StyleOutputSchema = z.object({
  content: z
    .string()
    .describe("API style, naming conventions, and formatting preferences."),
});

/** Schema for do/don't output. */
export const DoDontOutputSchema = z.object({
  content: z
    .string()
    .describe("Do/Don't pairs covering best practices and common pitfalls."),
});

/** Schema for llms.txt output. */
export const LlmsTxtOutputSchema = z.object({
  content: z
    .string()
    .describe("Complete llms.txt file content following the llmstxt.org format."),
});

/** Schema for llms-full.txt output. */
export const LlmsFullTxtOutputSchema = z.object({
  content: z
    .string()
    .describe("Complete llms-full.txt with all page content concatenated."),
});
