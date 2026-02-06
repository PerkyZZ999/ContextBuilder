/**
 * @module reader
 * KbReader — read-only access to a ContextBuilder knowledge base directory.
 *
 * Opens manifest.json, toc.json, artifact files, page Markdown files,
 * and the Turso Embedded/libSQL database for FTS5 search.
 */
import { createClient } from "@libsql/client";
import type { Client } from "@libsql/client";
import { readFile, readdir, access, stat } from "node:fs/promises";
import { join, resolve } from "node:path";
import {
  type KbManifest,
  type Toc,
  type TocEntry,
  KbManifestSchema,
  TocSchema,
  CURRENT_SCHEMA_VERSION,
} from "@contextbuilder/shared";
import { ARTIFACT_NAMES } from "@contextbuilder/shared";

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/** Content and metadata for a single KB page. */
export interface PageContent {
  path: string;
  title: string | null;
  content: string;
  source_url: string | null;
  last_fetched: string | null;
}

/** A search result from FTS5. */
export interface SearchResult {
  path: string;
  title: string | null;
  snippet: string;
  score: number;
}

/** Summary info for listing KBs. */
export interface KbSummary {
  id: string;
  name: string;
  source_url: string;
  page_count: number;
  updated_at: string;
  kb_path: string;
}

// ---------------------------------------------------------------------------
// Error helpers
// ---------------------------------------------------------------------------

class KbReaderError extends Error {
  constructor(
    message: string,
    public readonly code: string,
  ) {
    super(message);
    this.name = "KbReaderError";
  }
}

function notFound(what: string, detail: string): KbReaderError {
  return new KbReaderError(`${what} not found: ${detail}`, "NOT_FOUND");
}

function schemaError(detail: string): KbReaderError {
  return new KbReaderError(`Schema error: ${detail}`, "SCHEMA_ERROR");
}

// ---------------------------------------------------------------------------
// KbReader class
// ---------------------------------------------------------------------------

/**
 * Read-only handle to a single knowledge base directory.
 *
 * Use `KbReader.open(kbPath)` to create an instance. Call `close()` when done.
 */
export class KbReader {
  private constructor(
    private readonly kbPath: string,
    private readonly manifest: KbManifest,
    private readonly toc: Toc,
    private readonly db: Client,
  ) {}

  /**
   * Open a KB directory for reading.
   *
   * Validates manifest schema version, parses manifest and TOC,
   * and opens the DB in read-only mode.
   *
   * @param kbPath - Absolute path to the KB root directory.
   * @throws {KbReaderError} If files are missing or schema version is incompatible.
   */
  static async open(kbPath: string): Promise<KbReader> {
    const absPath = resolve(kbPath);

    // --- Load manifest ---
    const manifestPath = join(absPath, "manifest.json");
    let manifestRaw: string;
    try {
      manifestRaw = await readFile(manifestPath, "utf-8");
    } catch {
      throw notFound("manifest.json", manifestPath);
    }

    let manifest: KbManifest;
    try {
      const data: unknown = JSON.parse(manifestRaw);
      manifest = KbManifestSchema.parse(data);
    } catch (err) {
      throw schemaError(`invalid manifest.json: ${err instanceof Error ? err.message : err}`);
    }

    if (manifest.schema_version > CURRENT_SCHEMA_VERSION) {
      throw schemaError(
        `manifest schema_version ${manifest.schema_version} is newer than supported version ${CURRENT_SCHEMA_VERSION}. Please upgrade ContextBuilder.`,
      );
    }

    // --- Load TOC ---
    const tocPath = join(absPath, "toc.json");
    let tocRaw: string;
    try {
      tocRaw = await readFile(tocPath, "utf-8");
    } catch {
      throw notFound("toc.json", tocPath);
    }

    let toc: Toc;
    try {
      const data: unknown = JSON.parse(tocRaw);
      toc = TocSchema.parse(data);
    } catch (err) {
      throw schemaError(`invalid toc.json: ${err instanceof Error ? err.message : err}`);
    }

    // --- Open DB ---
    const dbPath = join(absPath, "indexes", "contextbuilder.db");
    let db: Client;
    try {
      db = createClient({ url: `file:${dbPath}` });
    } catch (err) {
      throw new KbReaderError(
        `Failed to open DB: ${err instanceof Error ? err.message : err}`,
        "DB_ERROR",
      );
    }

    return new KbReader(absPath, manifest, toc, db);
  }

  /** Get the parsed and validated manifest. */
  getManifest(): KbManifest {
    return this.manifest;
  }

  /** Get the parsed and validated TOC. */
  getToc(): Toc {
    return this.toc;
  }

  /** Get the KB ID from the manifest. */
  get id(): string {
    return this.manifest.id;
  }

  /** Get the KB path. */
  get path(): string {
    return this.kbPath;
  }

  /**
   * Read a single page by its path.
   *
   * @param pagePath - Relative path within `docs/` (e.g., `getting-started/installation`).
   * @returns Page content and metadata.
   * @throws {KbReaderError} If the page file or DB record is not found.
   */
  async getPage(pagePath: string): Promise<PageContent> {
    // Read markdown content from disk
    const filePath = join(this.kbPath, "docs", `${pagePath}.md`);
    let content: string;
    try {
      content = await readFile(filePath, "utf-8");
    } catch {
      throw notFound("page", pagePath);
    }

    // Get metadata from DB
    const result = await this.db.execute({
      sql: `SELECT title, url, fetched_at FROM pages WHERE kb_id = ? AND path = ?`,
      args: [this.manifest.id, pagePath],
    });

    const row = result.rows[0];
    return {
      path: pagePath,
      title: row ? (row.title as string | null) : null,
      content,
      source_url: row ? (row.url as string | null) : null,
      last_fetched: row ? (row.fetched_at as string | null) : null,
    };
  }

  /**
   * Read an artifact file by name.
   *
   * @param name - Artifact name (e.g., `llms.txt`, `SKILL.md`).
   * @returns The artifact content as a string.
   * @throws {KbReaderError} If the artifact file does not exist or name is invalid.
   */
  async getArtifact(name: string): Promise<string> {
    const validNames = ARTIFACT_NAMES as readonly string[];
    if (!validNames.includes(name)) {
      throw new KbReaderError(
        `Invalid artifact name '${name}'. Valid names: ${validNames.join(", ")}`,
        "INVALID_ARTIFACT",
      );
    }

    const filePath = join(this.kbPath, "artifacts", name);
    try {
      return await readFile(filePath, "utf-8");
    } catch {
      throw notFound("artifact", name);
    }
  }

  /**
   * Full-text search across pages in the KB via FTS5.
   *
   * @param query - FTS5 search query string.
   * @param limit - Maximum results (default: 10, max: 50).
   * @returns Ranked search results.
   */
  async search(query: string, limit = 10): Promise<SearchResult[]> {
    const clampedLimit = Math.min(Math.max(limit, 1), 50);

    const result = await this.db.execute({
      sql: `SELECT p.path, p.title, rank
            FROM pages_fts fts
            JOIN pages p ON p.rowid = fts.rowid
            WHERE pages_fts MATCH ? AND p.kb_id = ?
            ORDER BY rank
            LIMIT ?`,
      args: [query, this.manifest.id, clampedLimit],
    });

    return result.rows.map((row) => ({
      path: row.path as string,
      title: (row.title as string | null) ?? null,
      snippet: "", // FTS5 snippet would require snippet() function
      score: typeof row.rank === "number" ? row.rank : 0,
    }));
  }

  /**
   * List all pages in the KB from the database.
   *
   * @returns Array of page paths and titles.
   */
  async listPages(): Promise<Array<{ path: string; title: string | null }>> {
    const result = await this.db.execute({
      sql: `SELECT path, title FROM pages WHERE kb_id = ? ORDER BY path`,
      args: [this.manifest.id],
    });

    return result.rows.map((row) => ({
      path: row.path as string,
      title: (row.title as string | null) ?? null,
    }));
  }

  /**
   * List all available artifact files.
   *
   * @returns Array of artifact file names that exist on disk.
   */
  async listArtifacts(): Promise<string[]> {
    const artifactsDir = join(this.kbPath, "artifacts");
    try {
      const entries = await readdir(artifactsDir);
      return entries.filter((e) => (ARTIFACT_NAMES as readonly string[]).includes(e));
    } catch {
      return [];
    }
  }

  /**
   * Collect all page paths from the TOC tree.
   */
  getTocPaths(): string[] {
    const paths: string[] = [];
    const walk = (entries: TocEntry[]): void => {
      for (const entry of entries) {
        paths.push(entry.path);
        if (entry.children.length > 0) {
          walk(entry.children);
        }
      }
    };
    walk(this.toc.sections);
    return paths;
  }

  /**
   * Get a summary of this KB for listing.
   */
  getSummary(): KbSummary {
    return {
      id: this.manifest.id,
      name: this.manifest.name,
      source_url: this.manifest.source_url,
      page_count: this.manifest.page_count,
      updated_at: this.manifest.updated_at,
      kb_path: this.kbPath,
    };
  }

  /**
   * Close the database connection. Call when done with this reader.
   */
  close(): void {
    this.db.close();
  }

  // -----------------------------------------------------------------------
  // Static helpers
  // -----------------------------------------------------------------------

  /**
   * Discover KBs from a root directory (e.g., `var/kb/`).
   * Looks for subdirectories containing `manifest.json`.
   *
   * @param rootDir - Root directory to scan.
   * @returns Array of KbReader instances (one per valid KB found).
   */
  static async discoverKbs(rootDir: string): Promise<KbReader[]> {
    const absRoot = resolve(rootDir);
    let entries: string[];
    try {
      entries = await readdir(absRoot);
    } catch {
      return [];
    }

    const readers: KbReader[] = [];
    for (const entry of entries) {
      const kbPath = join(absRoot, entry);
      const manifestPath = join(kbPath, "manifest.json");
      try {
        await access(manifestPath);
        const kbStat = await stat(kbPath);
        if (kbStat.isDirectory()) {
          const reader = await KbReader.open(kbPath);
          readers.push(reader);
        }
      } catch {
        // Skip invalid entries
      }
    }
    return readers;
  }
}
