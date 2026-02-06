/**
 * @module @contextbuilder/schemas-mcp
 * Zod schemas for MCP server tool inputs and outputs.
 */
import { z } from "zod";

// ---------------------------------------------------------------------------
// Tool inputs
// ---------------------------------------------------------------------------

export const SearchDocsInputSchema = z
  .object({
    kb_id: z.string(),
    query: z.string().min(1),
    limit: z.number().int().min(1).max(100).default(10),
  })
  .strict();

export type SearchDocsInput = z.infer<typeof SearchDocsInputSchema>;

export const GetPageInputSchema = z
  .object({
    kb_id: z.string(),
    path: z.string(),
  })
  .strict();

export type GetPageInput = z.infer<typeof GetPageInputSchema>;

export const GetTocInputSchema = z
  .object({
    kb_id: z.string(),
  })
  .strict();

export type GetTocInput = z.infer<typeof GetTocInputSchema>;

export const ARTIFACT_NAMES = [
  "llms.txt",
  "llms-full.txt",
  "SKILL.md",
  "rules.md",
  "style.md",
  "do_dont.md",
] as const;

export const GetArtifactInputSchema = z
  .object({
    kb_id: z.string(),
    artifact_name: z.enum(ARTIFACT_NAMES),
  })
  .strict();

export type GetArtifactInput = z.infer<typeof GetArtifactInputSchema>;

export const ListKbsInputSchema = z.object({}).strict();

export type ListKbsInput = z.infer<typeof ListKbsInputSchema>;

// ---------------------------------------------------------------------------
// Tool outputs
// ---------------------------------------------------------------------------

export const SearchResultSchema = z
  .object({
    path: z.string(),
    title: z.string().nullable().optional(),
    score: z.number(),
  })
  .strict();

export type SearchResult = z.infer<typeof SearchResultSchema>;

export const SearchDocsOutputSchema = z
  .object({
    results: z.array(SearchResultSchema),
  })
  .strict();

export type SearchDocsOutput = z.infer<typeof SearchDocsOutputSchema>;

export const PageOutputSchema = z
  .object({
    path: z.string(),
    title: z.string().nullable().optional(),
    content: z.string(),
    source_url: z.string().url().optional(),
  })
  .strict();

export type PageOutput = z.infer<typeof PageOutputSchema>;

export const ArtifactOutputSchema = z
  .object({
    name: z.string(),
    content: z.string(),
  })
  .strict();

export type ArtifactOutput = z.infer<typeof ArtifactOutputSchema>;

export const KbListEntrySchema = z
  .object({
    id: z.string(),
    name: z.string(),
    source_url: z.string(),
  })
  .strict();

export type KbListEntry = z.infer<typeof KbListEntrySchema>;

export const ListKbsOutputSchema = z
  .object({
    kbs: z.array(KbListEntrySchema),
  })
  .strict();

export type ListKbsOutput = z.infer<typeof ListKbsOutputSchema>;
