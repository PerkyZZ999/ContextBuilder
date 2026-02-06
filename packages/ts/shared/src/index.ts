/**
 * @contextbuilder/shared
 * Shared types, constants, and utilities for ContextBuilder TypeScript packages.
 */

export { type KbManifest, KbManifestSchema, CURRENT_SCHEMA_VERSION } from "./types";
export { type Toc, type TocEntry, TocSchema, TocEntrySchema } from "./types";
export { type PageMeta, PageMetaSchema } from "./types";
export { ARTIFACT_NAMES, DEFAULT_CONFIG } from "./constants";
export { loadManifest, loadToc, readPage, validateManifestVersion } from "./loaders";
