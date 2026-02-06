/**
 * Seed the test-kb fixture database for integration testing.
 * Creates the SQLite DB with the same schema as the Rust storage crate.
 */
import { createClient } from "@libsql/client";
import { resolve } from "node:path";
import { mkdirSync } from "node:fs";

const FIXTURE_KB = resolve(import.meta.dir, "test-kb");
const DB_DIR = resolve(FIXTURE_KB, "indexes");
const DB_PATH = resolve(DB_DIR, "contextbuilder.db");

mkdirSync(DB_DIR, { recursive: true });

const db = createClient({ url: `file:${DB_PATH}` });

// Run migrations matching the Rust storage crate
await db.executeMultiple(`
  CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT (datetime('now'))
  );

  CREATE TABLE IF NOT EXISTS kb (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    source_url TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    config_json TEXT
  );

  CREATE TABLE IF NOT EXISTS pages (
    id TEXT PRIMARY KEY,
    kb_id TEXT NOT NULL REFERENCES kb(id),
    url TEXT NOT NULL,
    path TEXT NOT NULL,
    title TEXT,
    content_hash TEXT NOT NULL,
    fetched_at TEXT NOT NULL,
    status_code INTEGER,
    content_len INTEGER,
    UNIQUE(kb_id, path)
  );

  CREATE TABLE IF NOT EXISTS links (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_page_id TEXT NOT NULL REFERENCES pages(id),
    to_url TEXT NOT NULL,
    kind TEXT
  );

  CREATE TABLE IF NOT EXISTS crawl_jobs (
    id TEXT PRIMARY KEY,
    kb_id TEXT NOT NULL REFERENCES kb(id),
    started_at TEXT NOT NULL,
    finished_at TEXT,
    stats_json TEXT
  );

  CREATE TABLE IF NOT EXISTS enrichment_cache (
    id TEXT PRIMARY KEY,
    kb_id TEXT NOT NULL,
    artifact_type TEXT NOT NULL,
    prompt_hash TEXT NOT NULL,
    model_id TEXT NOT NULL,
    result_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(kb_id, artifact_type, prompt_hash, model_id)
  );

  CREATE VIRTUAL TABLE IF NOT EXISTS pages_fts USING fts5(
    title,
    path,
    content = pages,
    content_rowid = rowid
  );

  INSERT OR IGNORE INTO schema_migrations (version) VALUES (1);
`);

// Seed KB
const KB_ID = "019748d2-b3f0-7000-8000-000000000001";
const NOW = "2025-01-02T12:00:00Z";

await db.execute({
  sql: `INSERT OR REPLACE INTO kb (id, name, source_url, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?)`,
  args: [KB_ID, "example-docs", "https://example.com/docs", "2025-01-01T00:00:00Z", NOW],
});

// Seed pages
const pages = [
  {
    id: "019748d2-b3f0-7001-8000-000000000001",
    path: "getting-started",
    url: "https://example.com/docs/getting-started",
    title: "Getting Started",
    hash: "abc123",
  },
  {
    id: "019748d2-b3f0-7001-8000-000000000002",
    path: "getting-started/installation",
    url: "https://example.com/docs/getting-started/installation",
    title: "Installation",
    hash: "def456",
  },
  {
    id: "019748d2-b3f0-7001-8000-000000000003",
    path: "getting-started/configuration",
    url: "https://example.com/docs/getting-started/configuration",
    title: "Configuration",
    hash: "ghi789",
  },
  {
    id: "019748d2-b3f0-7001-8000-000000000004",
    path: "api-reference",
    url: "https://example.com/docs/api-reference",
    title: "API Reference",
    hash: "jkl012",
  },
];

for (const page of pages) {
  await db.execute({
    sql: `INSERT OR REPLACE INTO pages (id, kb_id, url, path, title, content_hash, fetched_at, status_code, content_len)
          VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`,
    args: [page.id, KB_ID, page.url, page.path, page.title, page.hash, NOW, 200, 100],
  });

  // Also insert into FTS
  await db.execute({
    sql: `INSERT OR REPLACE INTO pages_fts (rowid, title, path)
          VALUES ((SELECT rowid FROM pages WHERE id = ?), ?, ?)`,
    args: [page.id, page.title, page.path],
  });
}

db.close();

// biome-ignore lint/nursery/noConsole: Script output
console.log(`Seeded fixture DB at ${DB_PATH}`);
// biome-ignore lint/nursery/noConsole: Script output
console.log(`  KB: ${KB_ID} (example-docs)`);
// biome-ignore lint/nursery/noConsole: Script output
console.log(`  Pages: ${pages.length}`);
