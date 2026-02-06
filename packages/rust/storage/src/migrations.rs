//! SQL migration definitions for the ContextBuilder database.
//!
//! Migrations are applied in order on database open. Each migration has a
//! version number and a set of SQL statements executed within a transaction.

/// A database migration with a version and SQL statements.
pub(crate) struct Migration {
    pub version: u32,
    pub description: &'static str,
    pub sql: &'static str,
}

/// All migrations, in ascending version order.
pub(crate) fn all_migrations() -> Vec<Migration> {
    vec![Migration {
        version: 1,
        description: "Initial schema: kb, pages, links, crawl_jobs, enrichment_cache, FTS5",
        sql: r#"
-- Schema version tracking
CREATE TABLE IF NOT EXISTS schema_migrations (
    version   INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Knowledge base metadata
CREATE TABLE IF NOT EXISTS kb (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    source_url  TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    config_json TEXT
);

-- Individual pages
CREATE TABLE IF NOT EXISTS pages (
    id           TEXT PRIMARY KEY,
    kb_id        TEXT NOT NULL REFERENCES kb(id) ON DELETE CASCADE,
    url          TEXT NOT NULL,
    path         TEXT NOT NULL,
    title        TEXT,
    content_hash TEXT NOT NULL,
    fetched_at   TEXT NOT NULL,
    status_code  INTEGER,
    content_len  INTEGER,
    UNIQUE(kb_id, path)
);

CREATE INDEX IF NOT EXISTS idx_pages_kb_id ON pages(kb_id);
CREATE INDEX IF NOT EXISTS idx_pages_content_hash ON pages(content_hash);

-- Link graph for crawl management
CREATE TABLE IF NOT EXISTS links (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    from_page_id TEXT NOT NULL REFERENCES pages(id) ON DELETE CASCADE,
    to_url       TEXT NOT NULL,
    kind         TEXT
);

CREATE INDEX IF NOT EXISTS idx_links_from ON links(from_page_id);

-- Crawl job history
CREATE TABLE IF NOT EXISTS crawl_jobs (
    id          TEXT PRIMARY KEY,
    kb_id       TEXT NOT NULL REFERENCES kb(id) ON DELETE CASCADE,
    started_at  TEXT NOT NULL,
    finished_at TEXT,
    stats_json  TEXT
);

CREATE INDEX IF NOT EXISTS idx_crawl_jobs_kb_id ON crawl_jobs(kb_id);

-- LLM enrichment cache
CREATE TABLE IF NOT EXISTS enrichment_cache (
    id            TEXT PRIMARY KEY,
    kb_id         TEXT NOT NULL REFERENCES kb(id) ON DELETE CASCADE,
    artifact_type TEXT NOT NULL,
    prompt_hash   TEXT NOT NULL,
    model_id      TEXT NOT NULL,
    result_json   TEXT NOT NULL,
    created_at    TEXT NOT NULL,
    UNIQUE(kb_id, artifact_type, prompt_hash, model_id)
);

CREATE INDEX IF NOT EXISTS idx_enrichment_kb ON enrichment_cache(kb_id);

-- Full-text search on pages
CREATE VIRTUAL TABLE IF NOT EXISTS pages_fts USING fts5(
    title,
    path,
    content=pages,
    content_rowid=rowid
);

-- Triggers to keep FTS in sync with pages table
CREATE TRIGGER IF NOT EXISTS pages_fts_insert AFTER INSERT ON pages BEGIN
    INSERT INTO pages_fts(rowid, title, path)
    VALUES (new.rowid, new.title, new.path);
END;

CREATE TRIGGER IF NOT EXISTS pages_fts_delete AFTER DELETE ON pages BEGIN
    INSERT INTO pages_fts(pages_fts, rowid, title, path)
    VALUES ('delete', old.rowid, old.title, old.path);
END;

CREATE TRIGGER IF NOT EXISTS pages_fts_update AFTER UPDATE ON pages BEGIN
    INSERT INTO pages_fts(pages_fts, rowid, title, path)
    VALUES ('delete', old.rowid, old.title, old.path);
    INSERT INTO pages_fts(rowid, title, path)
    VALUES (new.rowid, new.title, new.path);
END;

INSERT INTO schema_migrations (version) VALUES (1);
"#,
    }]
}
