//! Turso Embedded / libSQL storage layer (offline mode).
//!
//! The [`Storage`] struct wraps a libSQL database for KB metadata, page indexes,
//! link graphs, crawl jobs, enrichment cache, and full-text search.
//!
//! **Access rules:**
//! - Rust CLI: read-write (sole writer) via [`Storage::open`]
//! - TypeScript MCP server: read-only via [`Storage::open_readonly`]

mod migrations;

use std::path::Path;

use chrono::Utc;
use contextbuilder_shared::{ContextBuilderError, PageMeta, Result};
use libsql::{Connection, Database, params};
use uuid::Uuid;

/// Primary storage handle wrapping a libSQL database.
pub struct Storage {
    #[allow(dead_code)]
    db: Database,
    conn: Connection,
    readonly: bool,
}

impl Storage {
    /// Open or create a database at `path` in read-write mode.
    pub async fn open(path: &Path) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ContextBuilderError::io(parent, e))?;
        }

        let db = libsql::Builder::new_local(path)
            .build()
            .await
            .map_err(|e| ContextBuilderError::Storage(e.to_string()))?;

        let conn = db
            .connect()
            .map_err(|e| ContextBuilderError::Storage(e.to_string()))?;

        let storage = Self {
            db,
            conn,
            readonly: false,
        };
        storage.run_migrations().await?;
        Ok(storage)
    }

    /// Open a database at `path` in read-only mode (for MCP server parity).
    pub async fn open_readonly(path: &Path) -> Result<Self> {
        let db = libsql::Builder::new_local(path)
            .build()
            .await
            .map_err(|e| ContextBuilderError::Storage(e.to_string()))?;

        let conn = db
            .connect()
            .map_err(|e| ContextBuilderError::Storage(e.to_string()))?;

        Ok(Self {
            db,
            conn,
            readonly: true,
        })
    }

    /// Run pending schema migrations.
    async fn run_migrations(&self) -> Result<()> {
        let current_version = self.get_schema_version().await;

        for migration in migrations::all_migrations() {
            if migration.version > current_version {
                tracing::info!(
                    version = migration.version,
                    description = migration.description,
                    "applying migration"
                );
                self.conn
                    .execute_batch(migration.sql)
                    .await
                    .map_err(|e| {
                        ContextBuilderError::Storage(format!(
                            "migration v{} failed: {e}",
                            migration.version
                        ))
                    })?;
            }
        }
        Ok(())
    }

    /// Get the current schema version, or 0 if no migrations have been applied.
    async fn get_schema_version(&self) -> u32 {
        let result = self
            .conn
            .query("SELECT MAX(version) FROM schema_migrations", params![])
            .await;

        match result {
            Ok(mut rows) => {
                if let Ok(Some(row)) = rows.next().await {
                    row.get::<u32>(0).unwrap_or(0)
                } else {
                    0
                }
            }
            Err(_) => 0, // Table doesn't exist yet
        }
    }

    /// Ensure we're in read-write mode before writing.
    fn check_writable(&self) -> Result<()> {
        if self.readonly {
            return Err(ContextBuilderError::Storage(
                "database is opened in read-only mode".into(),
            ));
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // KB operations
    // -----------------------------------------------------------------------

    /// Insert a new knowledge base record.
    pub async fn insert_kb(
        &self,
        id: &str,
        name: &str,
        source_url: &str,
        config_json: Option<&str>,
    ) -> Result<()> {
        self.check_writable()?;
        let now = Utc::now().to_rfc3339();
        self.conn
            .execute(
                "INSERT INTO kb (id, name, source_url, created_at, updated_at, config_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![id, name, source_url, now.as_str(), now.as_str(), config_json],
            )
            .await
            .map_err(|e| ContextBuilderError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Get a KB by ID. Returns `(id, name, source_url, created_at, updated_at)`.
    pub async fn get_kb(
        &self,
        id: &str,
    ) -> Result<Option<(String, String, String, String, String)>> {
        let mut rows = self
            .conn
            .query(
                "SELECT id, name, source_url, created_at, updated_at FROM kb WHERE id = ?1",
                params![id],
            )
            .await
            .map_err(|e| ContextBuilderError::Storage(e.to_string()))?;

        match rows.next().await {
            Ok(Some(row)) => Ok(Some((
                row.get::<String>(0)
                    .map_err(|e| ContextBuilderError::Storage(e.to_string()))?,
                row.get::<String>(1)
                    .map_err(|e| ContextBuilderError::Storage(e.to_string()))?,
                row.get::<String>(2)
                    .map_err(|e| ContextBuilderError::Storage(e.to_string()))?,
                row.get::<String>(3)
                    .map_err(|e| ContextBuilderError::Storage(e.to_string()))?,
                row.get::<String>(4)
                    .map_err(|e| ContextBuilderError::Storage(e.to_string()))?,
            ))),
            Ok(None) => Ok(None),
            Err(e) => Err(ContextBuilderError::Storage(e.to_string())),
        }
    }

    /// List all KBs. Returns `Vec<(id, name, source_url)>`.
    pub async fn list_kbs(&self) -> Result<Vec<(String, String, String)>> {
        let mut rows = self
            .conn
            .query(
                "SELECT id, name, source_url FROM kb ORDER BY name",
                params![],
            )
            .await
            .map_err(|e| ContextBuilderError::Storage(e.to_string()))?;

        let mut results = Vec::new();
        while let Ok(Some(row)) = rows.next().await {
            results.push((
                row.get::<String>(0)
                    .map_err(|e| ContextBuilderError::Storage(e.to_string()))?,
                row.get::<String>(1)
                    .map_err(|e| ContextBuilderError::Storage(e.to_string()))?,
                row.get::<String>(2)
                    .map_err(|e| ContextBuilderError::Storage(e.to_string()))?,
            ));
        }
        Ok(results)
    }

    /// Update a KB's `updated_at` timestamp.
    pub async fn update_kb(&self, id: &str) -> Result<()> {
        self.check_writable()?;
        let now = Utc::now().to_rfc3339();
        self.conn
            .execute(
                "UPDATE kb SET updated_at = ?1 WHERE id = ?2",
                params![now.as_str(), id],
            )
            .await
            .map_err(|e| ContextBuilderError::Storage(e.to_string()))?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Page operations
    // -----------------------------------------------------------------------

    /// Upsert a page (insert or update on conflict by `kb_id + path`).
    pub async fn upsert_page(&self, page: &PageMeta) -> Result<()> {
        self.check_writable()?;
        self.conn
            .execute(
                "INSERT INTO pages (id, kb_id, url, path, title, content_hash, fetched_at, status_code, content_len)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(kb_id, path) DO UPDATE SET
                   url = excluded.url,
                   title = excluded.title,
                   content_hash = excluded.content_hash,
                   fetched_at = excluded.fetched_at,
                   status_code = excluded.status_code,
                   content_len = excluded.content_len",
                params![
                    page.id.as_str(),
                    page.kb_id.as_str(),
                    page.url.as_str(),
                    page.path.as_str(),
                    page.title.as_deref(),
                    page.content_hash.as_str(),
                    page.fetched_at.to_rfc3339(),
                    page.status_code.map(i64::from),
                    page.content_len.map(|l| l as i64),
                ],
            )
            .await
            .map_err(|e| ContextBuilderError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Get a page by KB ID and path.
    pub async fn get_page(&self, kb_id: &str, path: &str) -> Result<Option<PageMeta>> {
        let mut rows = self
            .conn
            .query(
                "SELECT id, kb_id, url, path, title, content_hash, fetched_at, status_code, content_len
                 FROM pages WHERE kb_id = ?1 AND path = ?2",
                params![kb_id, path],
            )
            .await
            .map_err(|e| ContextBuilderError::Storage(e.to_string()))?;

        match rows.next().await {
            Ok(Some(row)) => Ok(Some(row_to_page_meta(&row)?)),
            Ok(None) => Ok(None),
            Err(e) => Err(ContextBuilderError::Storage(e.to_string())),
        }
    }

    /// List all pages for a KB.
    pub async fn list_pages_by_kb(&self, kb_id: &str) -> Result<Vec<PageMeta>> {
        let mut rows = self
            .conn
            .query(
                "SELECT id, kb_id, url, path, title, content_hash, fetched_at, status_code, content_len
                 FROM pages WHERE kb_id = ?1 ORDER BY path",
                params![kb_id],
            )
            .await
            .map_err(|e| ContextBuilderError::Storage(e.to_string()))?;

        let mut results = Vec::new();
        while let Ok(Some(row)) = rows.next().await {
            results.push(row_to_page_meta(&row)?);
        }
        Ok(results)
    }

    /// Delete a page by ID.
    pub async fn delete_page(&self, page_id: &str) -> Result<()> {
        self.check_writable()?;
        self.conn
            .execute("DELETE FROM pages WHERE id = ?1", params![page_id])
            .await
            .map_err(|e| ContextBuilderError::Storage(e.to_string()))?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Link operations
    // -----------------------------------------------------------------------

    /// Insert a link record.
    pub async fn insert_link(
        &self,
        from_page_id: &str,
        to_url: &str,
        kind: Option<&str>,
    ) -> Result<()> {
        self.check_writable()?;
        self.conn
            .execute(
                "INSERT INTO links (from_page_id, to_url, kind) VALUES (?1, ?2, ?3)",
                params![from_page_id, to_url, kind],
            )
            .await
            .map_err(|e| ContextBuilderError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Get links originating from a page. Returns `Vec<(to_url, kind)>`.
    pub async fn get_links_for_page(
        &self,
        page_id: &str,
    ) -> Result<Vec<(String, Option<String>)>> {
        let mut rows = self
            .conn
            .query(
                "SELECT to_url, kind FROM links WHERE from_page_id = ?1",
                params![page_id],
            )
            .await
            .map_err(|e| ContextBuilderError::Storage(e.to_string()))?;

        let mut results = Vec::new();
        while let Ok(Some(row)) = rows.next().await {
            let to_url: String = row
                .get(0)
                .map_err(|e| ContextBuilderError::Storage(e.to_string()))?;
            let kind: Option<String> = row.get(1).ok();
            results.push((to_url, kind));
        }
        Ok(results)
    }

    // -----------------------------------------------------------------------
    // Crawl job operations
    // -----------------------------------------------------------------------

    /// Insert a new crawl job. Returns the generated job ID.
    pub async fn insert_crawl_job(&self, kb_id: &str) -> Result<String> {
        self.check_writable()?;
        let id = Uuid::now_v7().to_string();
        let now = Utc::now().to_rfc3339();
        self.conn
            .execute(
                "INSERT INTO crawl_jobs (id, kb_id, started_at) VALUES (?1, ?2, ?3)",
                params![id.as_str(), kb_id, now.as_str()],
            )
            .await
            .map_err(|e| ContextBuilderError::Storage(e.to_string()))?;
        Ok(id)
    }

    /// Update a crawl job with completion data.
    pub async fn update_crawl_job(&self, job_id: &str, stats_json: &str) -> Result<()> {
        self.check_writable()?;
        let now = Utc::now().to_rfc3339();
        self.conn
            .execute(
                "UPDATE crawl_jobs SET finished_at = ?1, stats_json = ?2 WHERE id = ?3",
                params![now.as_str(), stats_json, job_id],
            )
            .await
            .map_err(|e| ContextBuilderError::Storage(e.to_string()))?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Enrichment cache operations
    // -----------------------------------------------------------------------

    /// Get a cached enrichment result.
    pub async fn get_enrichment_cache(
        &self,
        kb_id: &str,
        artifact_type: &str,
        prompt_hash: &str,
        model_id: &str,
    ) -> Result<Option<String>> {
        let mut rows = self
            .conn
            .query(
                "SELECT result_json FROM enrichment_cache
                 WHERE kb_id = ?1 AND artifact_type = ?2 AND prompt_hash = ?3 AND model_id = ?4",
                params![kb_id, artifact_type, prompt_hash, model_id],
            )
            .await
            .map_err(|e| ContextBuilderError::Storage(e.to_string()))?;

        match rows.next().await {
            Ok(Some(row)) => {
                let result: String = row
                    .get(0)
                    .map_err(|e| ContextBuilderError::Storage(e.to_string()))?;
                Ok(Some(result))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(ContextBuilderError::Storage(e.to_string())),
        }
    }

    /// Store an enrichment result in the cache (upserts).
    pub async fn set_enrichment_cache(
        &self,
        kb_id: &str,
        artifact_type: &str,
        prompt_hash: &str,
        model_id: &str,
        result_json: &str,
    ) -> Result<()> {
        self.check_writable()?;
        let id = Uuid::now_v7().to_string();
        let now = Utc::now().to_rfc3339();
        self.conn
            .execute(
                "INSERT INTO enrichment_cache (id, kb_id, artifact_type, prompt_hash, model_id, result_json, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(kb_id, artifact_type, prompt_hash, model_id) DO UPDATE SET
                   result_json = excluded.result_json,
                   created_at = excluded.created_at",
                params![id.as_str(), kb_id, artifact_type, prompt_hash, model_id, result_json, now.as_str()],
            )
            .await
            .map_err(|e| ContextBuilderError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Invalidate all enrichment cache entries for a KB.
    pub async fn invalidate_enrichment_cache(&self, kb_id: &str) -> Result<()> {
        self.check_writable()?;
        self.conn
            .execute(
                "DELETE FROM enrichment_cache WHERE kb_id = ?1",
                params![kb_id],
            )
            .await
            .map_err(|e| ContextBuilderError::Storage(e.to_string()))?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // FTS search
    // -----------------------------------------------------------------------

    /// Full-text search across pages in a KB.
    pub async fn search(
        &self,
        kb_id: &str,
        query: &str,
        limit: u32,
    ) -> Result<Vec<SearchResult>> {
        let mut rows = self
            .conn
            .query(
                "SELECT p.path, p.title, rank
                 FROM pages_fts fts
                 JOIN pages p ON p.rowid = fts.rowid
                 WHERE pages_fts MATCH ?1 AND p.kb_id = ?2
                 ORDER BY rank
                 LIMIT ?3",
                params![query, kb_id, limit],
            )
            .await
            .map_err(|e| ContextBuilderError::Storage(e.to_string()))?;

        let mut results = Vec::new();
        while let Ok(Some(row)) = rows.next().await {
            let path: String = row
                .get(0)
                .map_err(|e| ContextBuilderError::Storage(e.to_string()))?;
            let title: Option<String> = row.get(1).ok();
            let score: f64 = row.get(2).unwrap_or(0.0);
            results.push(SearchResult {
                path,
                title,
                score,
            });
        }
        Ok(results)
    }
}

/// A search result from FTS5.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Page path within the KB.
    pub path: String,
    /// Page title.
    pub title: Option<String>,
    /// FTS5 rank score (lower is better).
    pub score: f64,
}

/// Convert a database row to a [`PageMeta`].
fn row_to_page_meta(row: &libsql::Row) -> Result<PageMeta> {
    Ok(PageMeta {
        id: row
            .get::<String>(0)
            .map_err(|e| ContextBuilderError::Storage(e.to_string()))?,
        kb_id: row
            .get::<String>(1)
            .map_err(|e| ContextBuilderError::Storage(e.to_string()))?,
        url: row
            .get::<String>(2)
            .map_err(|e| ContextBuilderError::Storage(e.to_string()))?,
        path: row
            .get::<String>(3)
            .map_err(|e| ContextBuilderError::Storage(e.to_string()))?,
        title: row.get::<String>(4).ok(),
        content_hash: row
            .get::<String>(5)
            .map_err(|e| ContextBuilderError::Storage(e.to_string()))?,
        fetched_at: {
            let s: String = row
                .get(6)
                .map_err(|e| ContextBuilderError::Storage(e.to_string()))?;
            chrono::DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .map_err(|e| ContextBuilderError::Storage(format!("invalid date: {e}")))?
        },
        status_code: row.get::<i64>(7).ok().map(|v| v as u16),
        content_len: row.get::<i64>(8).ok().map(|v| v as usize),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    /// Create a temp file storage for testing.
    async fn test_storage() -> Storage {
        let tmp = std::env::temp_dir().join(format!("cb_test_{}.db", Uuid::now_v7()));
        Storage::open(&tmp).await.expect("open test db")
    }

    #[tokio::test]
    async fn open_and_migrate() {
        let storage = test_storage().await;
        let version = storage.get_schema_version().await;
        assert_eq!(version, 1);
    }

    #[tokio::test]
    async fn idempotent_migration() {
        let tmp = std::env::temp_dir().join(format!("cb_test_{}.db", Uuid::now_v7()));
        let _s1 = Storage::open(&tmp).await.expect("first open");
        drop(_s1);
        let s2 = Storage::open(&tmp).await.expect("second open");
        assert_eq!(s2.get_schema_version().await, 1);
    }

    #[tokio::test]
    async fn kb_crud() {
        let storage = test_storage().await;
        let kb_id = Uuid::now_v7().to_string();

        storage
            .insert_kb(&kb_id, "test-kb", "https://example.com/docs", None)
            .await
            .expect("insert kb");

        let kb = storage.get_kb(&kb_id).await.expect("get kb");
        assert!(kb.is_some());
        let (id, name, url, _, _) = kb.unwrap();
        assert_eq!(id, kb_id);
        assert_eq!(name, "test-kb");
        assert_eq!(url, "https://example.com/docs");

        let kbs = storage.list_kbs().await.expect("list kbs");
        assert_eq!(kbs.len(), 1);

        storage.update_kb(&kb_id).await.expect("update kb");
    }

    #[tokio::test]
    async fn page_upsert_and_query() {
        let storage = test_storage().await;
        let kb_id = Uuid::now_v7().to_string();
        storage
            .insert_kb(&kb_id, "test-kb", "https://example.com", None)
            .await
            .unwrap();

        let page = PageMeta {
            id: Uuid::now_v7().to_string(),
            kb_id: kb_id.clone(),
            url: "https://example.com/intro".into(),
            path: "intro".into(),
            title: Some("Introduction".into()),
            content_hash: "abc123".into(),
            fetched_at: Utc::now(),
            status_code: Some(200),
            content_len: Some(1024),
        };

        storage.upsert_page(&page).await.expect("upsert page");

        let found = storage.get_page(&kb_id, "intro").await.expect("get page");
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.title.as_deref(), Some("Introduction"));
        assert_eq!(found.content_hash, "abc123");

        // Upsert (update) with new hash
        let updated = PageMeta {
            content_hash: "def456".into(),
            ..page
        };
        storage.upsert_page(&updated).await.expect("upsert again");
        let found = storage.get_page(&kb_id, "intro").await.unwrap().unwrap();
        assert_eq!(found.content_hash, "def456");

        let pages = storage
            .list_pages_by_kb(&kb_id)
            .await
            .expect("list pages");
        assert_eq!(pages.len(), 1);
    }

    #[tokio::test]
    async fn link_operations() {
        let storage = test_storage().await;
        let kb_id = Uuid::now_v7().to_string();
        storage
            .insert_kb(&kb_id, "test-kb", "https://example.com", None)
            .await
            .unwrap();

        let page_id = Uuid::now_v7().to_string();
        let page = PageMeta {
            id: page_id.clone(),
            kb_id: kb_id.clone(),
            url: "https://example.com/a".into(),
            path: "a".into(),
            title: None,
            content_hash: "hash".into(),
            fetched_at: Utc::now(),
            status_code: None,
            content_len: None,
        };
        storage.upsert_page(&page).await.unwrap();

        storage
            .insert_link(&page_id, "https://example.com/b", Some("internal"))
            .await
            .expect("insert link");

        let links = storage
            .get_links_for_page(&page_id)
            .await
            .expect("get links");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].0, "https://example.com/b");
    }

    #[tokio::test]
    async fn crawl_job_lifecycle() {
        let storage = test_storage().await;
        let kb_id = Uuid::now_v7().to_string();
        storage
            .insert_kb(&kb_id, "test-kb", "https://example.com", None)
            .await
            .unwrap();

        let job_id = storage
            .insert_crawl_job(&kb_id)
            .await
            .expect("insert crawl job");
        assert!(!job_id.is_empty());

        storage
            .update_crawl_job(&job_id, r#"{"pages": 10}"#)
            .await
            .expect("update crawl job");
    }

    #[tokio::test]
    async fn enrichment_cache() {
        let storage = test_storage().await;
        let kb_id = Uuid::now_v7().to_string();
        storage
            .insert_kb(&kb_id, "test-kb", "https://example.com", None)
            .await
            .unwrap();

        // Miss
        let cached = storage
            .get_enrichment_cache(&kb_id, "skill", "hash1", "gpt-4o")
            .await
            .expect("get cache miss");
        assert!(cached.is_none());

        // Set
        storage
            .set_enrichment_cache(&kb_id, "skill", "hash1", "gpt-4o", r#"{"result": "test"}"#)
            .await
            .expect("set cache");

        // Hit
        let cached = storage
            .get_enrichment_cache(&kb_id, "skill", "hash1", "gpt-4o")
            .await
            .expect("get cache hit");
        assert!(cached.is_some());
        assert!(cached.unwrap().contains("test"));

        // Invalidate
        storage
            .invalidate_enrichment_cache(&kb_id)
            .await
            .expect("invalidate");
        let cached = storage
            .get_enrichment_cache(&kb_id, "skill", "hash1", "gpt-4o")
            .await
            .expect("get after invalidate");
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn fts_search() {
        let storage = test_storage().await;
        let kb_id = Uuid::now_v7().to_string();
        storage
            .insert_kb(&kb_id, "test-kb", "https://example.com", None)
            .await
            .unwrap();

        for (path, title) in [
            ("getting-started", "Getting Started Guide"),
            ("api-reference", "API Reference Documentation"),
            ("installation", "Installation Instructions"),
        ] {
            let page = PageMeta {
                id: Uuid::now_v7().to_string(),
                kb_id: kb_id.clone(),
                url: format!("https://example.com/{path}"),
                path: path.into(),
                title: Some(title.into()),
                content_hash: "hash".into(),
                fetched_at: Utc::now(),
                status_code: Some(200),
                content_len: None,
            };
            storage.upsert_page(&page).await.unwrap();
        }

        let results = storage
            .search(&kb_id, "installation", 10)
            .await
            .expect("search");
        assert!(!results.is_empty());
        assert_eq!(results[0].path, "installation");
    }

    #[tokio::test]
    async fn readonly_rejects_writes() {
        let tmp = std::env::temp_dir().join(format!("cb_test_{}.db", Uuid::now_v7()));
        let rw = Storage::open(&tmp).await.unwrap();
        rw.insert_kb("kb1", "test", "https://example.com", None)
            .await
            .unwrap();
        drop(rw);

        let ro = Storage::open_readonly(&tmp).await.unwrap();
        let result = ro
            .insert_kb("kb2", "test2", "https://example.com", None)
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("read-only"));
    }
}
