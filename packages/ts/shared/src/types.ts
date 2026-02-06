/**
 * @module types
 * Shared zod schemas and TypeScript types mirroring the Rust `shared` crate.
 */
import { z } from "zod";

// ---------------------------------------------------------------------------
// Schema version
// ---------------------------------------------------------------------------

/** Current schema version for the KB manifest format. */
export const CURRENT_SCHEMA_VERSION = 1;

// ---------------------------------------------------------------------------
// KbManifest — mirrors Rust `KbManifest`
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// TocEntry / Toc — mirrors Rust `TocEntry` / `Toc`
// ---------------------------------------------------------------------------

export interface TocEntry {
  title: string;
  path: string;
  source_url?: string;
  summary?: string;
  children: TocEntry[];
}

export const TocEntrySchema: z.ZodType<TocEntry> = z.lazy(() =>
  z
    .object({
      title: z.string().min(1),
      path: z.string(),
      source_url: z.string().url().optional(),
      summary: z.string().optional(),
      children: z.array(TocEntrySchema).default([]),
    })
    .strict(),
);

export const TocSchema = z
  .object({
    sections: z.array(TocEntrySchema),
  })
  .strict();

export type Toc = z.infer<typeof TocSchema>;

// ---------------------------------------------------------------------------
// PageMeta — mirrors Rust `PageMeta`
// ---------------------------------------------------------------------------

export const PageMetaSchema = z
  .object({
    id: z.string(),
    kb_id: z.string(),
    url: z.string().url(),
    path: z.string(),
    title: z.string().nullable().optional(),
    content_hash: z.string(),
    fetched_at: z.string().datetime(),
    status_code: z.number().int().min(100).max(599).nullable().optional(),
    content_len: z.number().int().min(0).nullable().optional(),
  })
  .strict();

export type PageMeta = z.infer<typeof PageMetaSchema>;
