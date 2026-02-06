/**
 * @module @contextbuilder/schemas-toc
 * Zod schemas for KB toc.json — mirrors Rust `Toc` / `TocEntry`.
 */
import { z } from "zod";

/** Recursive schema for a single TOC entry. */
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

export interface TocEntry {
  title: string;
  path: string;
  source_url?: string;
  summary?: string;
  children: TocEntry[];
}

export const TocSchema = z
  .object({
    sections: z.array(TocEntrySchema),
  })
  .strict();

export type Toc = z.infer<typeof TocSchema>;
