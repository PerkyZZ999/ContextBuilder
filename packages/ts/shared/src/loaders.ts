/**
 * @module loaders
 * Utilities for loading KB manifest, TOC, and pages from disk.
 * Used by the MCP server (read-only) to access KB data.
 */
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { CURRENT_SCHEMA_VERSION, KbManifestSchema, TocSchema } from "./types";
import type { KbManifest, Toc } from "./types";

/**
 * Load and validate `manifest.json` from a KB directory.
 *
 * @param kbPath - Absolute path to the KB root directory.
 * @returns Parsed and validated manifest.
 * @throws If the file is missing, malformed, or fails schema validation.
 */
export async function loadManifest(kbPath: string): Promise<KbManifest> {
  const raw = await readFile(join(kbPath, "manifest.json"), "utf-8");
  const data: unknown = JSON.parse(raw);
  return KbManifestSchema.parse(data);
}

/**
 * Load and validate `toc.json` from a KB directory.
 *
 * @param kbPath - Absolute path to the KB root directory.
 * @returns Parsed and validated TOC.
 * @throws If the file is missing, malformed, or fails schema validation.
 */
export async function loadToc(kbPath: string): Promise<Toc> {
  const raw = await readFile(join(kbPath, "toc.json"), "utf-8");
  const data: unknown = JSON.parse(raw);
  return TocSchema.parse(data);
}

/**
 * Read a Markdown page's content from a KB directory.
 *
 * @param kbPath - Absolute path to the KB root directory.
 * @param pagePath - Relative path within `docs/` (e.g., `getting-started/installation`).
 * @returns The raw Markdown content.
 * @throws If the page file does not exist.
 */
export async function readPage(kbPath: string, pagePath: string): Promise<string> {
  const fullPath = join(kbPath, "docs", `${pagePath}.md`);
  return readFile(fullPath, "utf-8");
}

/**
 * Validate that a manifest's schema version is compatible with the current tool version.
 *
 * @param manifest - The manifest to check.
 * @returns `true` if compatible, `false` if the manifest is from a newer schema version.
 */
export function validateManifestVersion(manifest: KbManifest): boolean {
  return manifest.schema_version <= CURRENT_SCHEMA_VERSION;
}
