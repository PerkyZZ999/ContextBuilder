/**
 * @module @contextbuilder/schemas-manifest
 * Zod schemas for KB manifest.json — mirrors Rust `KbManifest`.
 */
import { z } from "zod";

export const KbManifestSchema = z
  .object({
    schema_version: z.number().int().min(1),
    id: z.string().uuid(),
    name: z.string().min(1),
    source_url: z.string().url(),
    tool_version: z.string(),
    created_at: z.string().datetime(),
    updated_at: z.string().datetime(),
    page_count: z.number().int().min(0),
    config: z.record(z.unknown()).nullish(),
    artifacts: z.record(z.unknown()).nullish(),
    enrichment: z.record(z.unknown()).nullish(),
  })
  .strict();

export type KbManifest = z.infer<typeof KbManifestSchema>;

/** Current schema version for the KB manifest format. */
export const CURRENT_SCHEMA_VERSION = 1;
