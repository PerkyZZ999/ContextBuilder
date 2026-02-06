/**
 * @module @contextbuilder/schemas-artifacts
 * Zod schemas for artifact metadata (SKILL.md, llms.txt).
 */
import { z } from "zod";

// ---------------------------------------------------------------------------
// SKILL.md metadata
// ---------------------------------------------------------------------------

export const SkillMetaSchema = z
  .object({
    name: z.string().min(1),
    description: z.string(),
    version: z.string(),
    source_url: z.string().url(),
    kb_id: z.string().uuid().optional(),
    generated_at: z.string().datetime().optional(),
    model_id: z.string().optional(),
    topics: z.array(z.string()).optional(),
  })
  .strict();

export type SkillMeta = z.infer<typeof SkillMetaSchema>;

// ---------------------------------------------------------------------------
// llms.txt / llms-full.txt generation metadata
// ---------------------------------------------------------------------------

export const LlmsMetaSchema = z
  .object({
    source_url: z.string().url(),
    kb_id: z.string().uuid().optional(),
    generated_at: z.string().datetime(),
    page_count: z.number().int().min(0),
    model_id: z.string().optional(),
    token_count: z.number().int().min(0).optional(),
    variant: z.enum(["llms.txt", "llms-full.txt"]).optional(),
  })
  .strict();

export type LlmsMeta = z.infer<typeof LlmsMetaSchema>;
