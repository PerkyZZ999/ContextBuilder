/**
 * @contextbuilder/kb-reader
 * Read-only access to ContextBuilder knowledge bases for the MCP server.
 *
 * Provides typed access to KB manifest, TOC, pages, artifacts,
 * and FTS5 search via the Turso Embedded/libSQL database.
 */

export { KbReader } from "./reader";
export type { PageContent, SearchResult, KbSummary } from "./reader";
