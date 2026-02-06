//! Incremental update flow for existing knowledge bases.
//!
//! Re-crawls the documentation source, diffs against stored content hashes,
//! and re-assembles only changed/new pages. Enrichment cache hits for unchanged
//! pages ensure minimal LLM calls on update.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

use tracing::{info, instrument, warn};
use url::Url;

use contextbuilder_crawler::FetchedPage;
use contextbuilder_markdown::ConvertOptions;
use contextbuilder_shared::{
    ContextBuilderError, CrawlConfig, KbId, KbManifest, PageMeta, Result,
};
use contextbuilder_storage::Storage;

use crate::assembler::{AssembleConfig, AssemblePage};
use crate::pipeline::ProgressReporter;
use crate::toc;

// ---------------------------------------------------------------------------
// Update config & result
// ---------------------------------------------------------------------------

/// Configuration for the `update_kb` pipeline.
#[derive(Debug, Clone)]
pub struct UpdateKbConfig {
    /// Path to the existing KB directory (contains manifest.json).
    pub kb_path: PathBuf,
    /// Crawl configuration.
    pub crawl: CrawlConfig,
    /// Tool version string.
    pub tool_version: String,
    /// Whether to remove pages no longer present upstream.
    pub prune: bool,
    /// Whether to force re-crawl even if hashes match.
    pub force: bool,
}

/// Result of the `update_kb` pipeline.
#[derive(Debug)]
pub struct UpdateKbResult {
    /// KB identifier.
    pub kb_id: KbId,
    /// Pages added (new URLs discovered).
    pub pages_added: usize,
    /// Pages removed (no longer upstream).
    pub pages_removed: usize,
    /// Pages whose content changed.
    pub pages_changed: usize,
    /// Pages unchanged (hash match).
    pub pages_unchanged: usize,
    /// Total page count after update.
    pub page_count: usize,
    /// Total elapsed time.
    pub elapsed: std::time::Duration,
}

// ---------------------------------------------------------------------------
// Diff helpers
// ---------------------------------------------------------------------------

/// Diff result categorizing pages by their change status.
#[derive(Debug, Default)]
pub(crate) struct PageDiff {
    /// New pages not previously in the KB.
    pub new_pages: Vec<String>,
    /// Pages whose content hash changed.
    pub changed_pages: Vec<String>,
    /// Pages unchanged.
    pub unchanged_pages: Vec<String>,
    /// Pages in the old KB but not in the new crawl.
    pub removed_pages: Vec<String>,
}

/// Compute the diff between existing pages and newly fetched pages.
pub(crate) fn diff_pages(
    existing: &[PageMeta],
    fetched: &[FetchedPage],
    force: bool,
) -> PageDiff {
    let existing_by_path: HashMap<&str, &PageMeta> =
        existing.iter().map(|p| (p.path.as_str(), p)).collect();

    let fetched_paths: HashSet<&str> = fetched.iter().map(|p| p.meta.path.as_str()).collect();

    let mut diff = PageDiff::default();

    for page in fetched {
        let path = &page.meta.path;
        match existing_by_path.get(path.as_str()) {
            Some(old) if !force && old.content_hash == page.meta.content_hash => {
                diff.unchanged_pages.push(path.clone());
            }
            Some(_) => {
                diff.changed_pages.push(path.clone());
            }
            None => {
                diff.new_pages.push(path.clone());
            }
        }
    }

    for old in existing {
        if !fetched_paths.contains(old.path.as_str()) {
            diff.removed_pages.push(old.path.clone());
        }
    }

    diff
}

// ---------------------------------------------------------------------------
// Update pipeline
// ---------------------------------------------------------------------------

/// Run the update pipeline for an existing KB.
///
/// 1. Load manifest and existing page metadata from storage
/// 2. Re-crawl using the original source URL
/// 3. Diff new pages against stored content hashes
/// 4. Re-convert changed/new pages
/// 5. Re-build TOC and re-assemble the KB directory
#[instrument(skip_all, fields(kb_path = %config.kb_path.display()))]
pub async fn update_kb(
    config: &UpdateKbConfig,
    progress: &dyn ProgressReporter,
) -> Result<UpdateKbResult> {
    let start = Instant::now();

    // --- Load manifest ---
    progress.phase("Loading existing KB");
    let manifest = load_manifest(&config.kb_path)?;
    let kb_id = manifest.id.clone();
    let source_url = Url::parse(&manifest.source_url).map_err(|e| {
        ContextBuilderError::validation(format!("invalid source_url in manifest: {e}"))
    })?;

    info!(%kb_id, source = %source_url, "updating KB");

    // --- Open storage ---
    let db_path = config
        .kb_path
        .join("indexes")
        .join("contextbuilder.db");
    let storage = Storage::open(&db_path).await?;

    // --- Get existing pages ---
    let existing_pages = storage.list_pages_by_kb(&kb_id.to_string()).await?;
    let _existing_count = existing_pages.len();

    // --- Re-crawl ---
    progress.phase("Re-crawling documentation");
    let crawler = contextbuilder_crawler::Crawler::new(config.crawl.clone())?;
    let (_crawl_result, fetched_pages) = crawler
        .crawl(&source_url, &kb_id.to_string(), &storage)
        .await?;

    if fetched_pages.is_empty() {
        return Err(ContextBuilderError::validation(
            "re-crawl returned no pages",
        ));
    }

    // --- Diff ---
    progress.phase("Comparing content");
    let diff = diff_pages(&existing_pages, &fetched_pages, config.force);

    info!(
        new = diff.new_pages.len(),
        changed = diff.changed_pages.len(),
        unchanged = diff.unchanged_pages.len(),
        removed = diff.removed_pages.len(),
        "page diff computed"
    );

    // --- Handle removals ---
    if config.prune {
        for path in &diff.removed_pages {
            if let Some(old) = existing_pages.iter().find(|p| &p.path == path) {
                let _ = storage.delete_page(&old.id).await;
                // Remove the markdown file
                let md_path = config.kb_path.join("docs").join(format!("{path}.md"));
                let _ = std::fs::remove_file(&md_path);
            }
        }
    }

    // --- Convert changed/new pages ---
    progress.phase("Converting updated pages");
    let needs_convert: HashSet<&str> = diff
        .new_pages
        .iter()
        .chain(diff.changed_pages.iter())
        .map(String::as_str)
        .collect();

    let mut assembled_pages: Vec<AssemblePage> = Vec::new();
    let total = fetched_pages.len();

    for (i, page) in fetched_pages.iter().enumerate() {
        if needs_convert.contains(page.meta.path.as_str()) || config.force {
            // Convert HTML → Markdown
            let opts = ConvertOptions {
                source_url: page.meta.url.clone(),
                title: page.meta.title.clone(),
                fetched_at: Some(page.meta.fetched_at.to_rfc3339()),
            };

            match contextbuilder_markdown::convert(&page.html, &opts) {
                Ok(result) => {
                    progress.page_converted(&page.meta.path, i + 1, total);
                    assembled_pages.push(AssemblePage {
                        path: page.meta.path.clone(),
                        markdown: result.markdown,
                        title: result.title,
                    });
                }
                Err(e) => {
                    warn!(path = %page.meta.path, error = %e, "conversion failed, skipping");
                }
            }
        } else {
            // Unchanged — read existing markdown from disk
            let md_path = config.kb_path.join("docs").join(format!("{}.md", page.meta.path));
            match std::fs::read_to_string(&md_path) {
                Ok(content) => {
                    let title = page
                        .meta
                        .title
                        .clone()
                        .unwrap_or_else(|| page.meta.path.clone());
                    assembled_pages.push(AssemblePage {
                        path: page.meta.path.clone(),
                        markdown: content,
                        title,
                    });
                }
                Err(e) => {
                    warn!(path = %page.meta.path, error = %e, "cannot read existing page, re-converting");
                    let opts = ConvertOptions {
                        source_url: page.meta.url.clone(),
                        title: page.meta.title.clone(),
                        fetched_at: Some(page.meta.fetched_at.to_rfc3339()),
                    };
                    if let Ok(result) = contextbuilder_markdown::convert(&page.html, &opts) {
                        assembled_pages.push(AssemblePage {
                            path: page.meta.path.clone(),
                            markdown: result.markdown,
                            title: result.title,
                        });
                    }
                }
            }
        }
    }

    // Update storage for changed/new pages
    for page in &fetched_pages {
        if needs_convert.contains(page.meta.path.as_str()) {
            let _ = storage.upsert_page(&page.meta).await;
        }
    }

    // --- Rebuild TOC ---
    progress.phase("Rebuilding table of contents");
    let all_metas: Vec<_> = fetched_pages.iter().map(|p| p.meta.clone()).collect();
    let toc = toc::build_toc(&all_metas, &[]);

    // --- Re-assemble ---
    progress.phase("Re-assembling knowledge base");
    let output_root = config
        .kb_path
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();

    let assemble_config = AssembleConfig {
        kb_id: kb_id.clone(),
        name: manifest.name.clone(),
        source_url: manifest.source_url.clone(),
        output_root,
        tool_version: config.tool_version.clone(),
    };

    let _assemble_result =
        crate::assembler::assemble(&assemble_config, &assembled_pages, &toc)?;

    let removed_count = if config.prune {
        diff.removed_pages.len()
    } else {
        0
    };

    let result = UpdateKbResult {
        kb_id,
        pages_added: diff.new_pages.len(),
        pages_removed: removed_count,
        pages_changed: diff.changed_pages.len(),
        pages_unchanged: diff.unchanged_pages.len(),
        page_count: assembled_pages.len(),
        elapsed: start.elapsed(),
    };

    info!(
        pages_added = result.pages_added,
        pages_changed = result.pages_changed,
        pages_unchanged = result.pages_unchanged,
        pages_removed = result.pages_removed,
        elapsed_ms = result.elapsed.as_millis(),
        "update complete"
    );

    Ok(result)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Load and parse manifest.json from a KB directory.
fn load_manifest(kb_path: &Path) -> Result<KbManifest> {
    let manifest_path = kb_path.join("manifest.json");
    let content = std::fs::read_to_string(&manifest_path)
        .map_err(|e| ContextBuilderError::io(&manifest_path, e))?;
    let manifest: KbManifest = serde_json::from_str(&content).map_err(|e| {
        ContextBuilderError::validation(format!("invalid manifest.json: {e}"))
    })?;
    Ok(manifest)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use contextbuilder_crawler::FetchedPage;
    use contextbuilder_shared::PageMeta;

    fn make_page_meta(path: &str, hash: &str) -> PageMeta {
        PageMeta {
            id: uuid::Uuid::now_v7().to_string(),
            kb_id: "test-kb".into(),
            url: format!("https://example.com/{path}"),
            path: path.into(),
            title: Some(path.into()),
            content_hash: hash.into(),
            fetched_at: Utc::now(),
            status_code: Some(200),
            content_len: Some(100),
        }
    }

    fn make_fetched_page(path: &str, hash: &str) -> FetchedPage {
        FetchedPage {
            meta: make_page_meta(path, hash),
            content: contextbuilder_crawler::ExtractedContent {
                html: "<p>test</p>".into(),
                meta: contextbuilder_crawler::adapters::PageMeta { title: Some(path.into()) },
            },
            html: "<html><body><p>test</p></body></html>".into(),
            links: vec![],
        }
    }

    #[test]
    fn diff_detects_new_pages() {
        let existing = vec![make_page_meta("index", "hash1")];
        let fetched = vec![
            make_fetched_page("index", "hash1"),
            make_fetched_page("new-page", "hash2"),
        ];

        let diff = diff_pages(&existing, &fetched, false);
        assert_eq!(diff.new_pages, vec!["new-page"]);
        assert_eq!(diff.unchanged_pages, vec!["index"]);
        assert!(diff.changed_pages.is_empty());
        assert!(diff.removed_pages.is_empty());
    }

    #[test]
    fn diff_detects_changed_pages() {
        let existing = vec![make_page_meta("index", "old-hash")];
        let fetched = vec![make_fetched_page("index", "new-hash")];

        let diff = diff_pages(&existing, &fetched, false);
        assert_eq!(diff.changed_pages, vec!["index"]);
        assert!(diff.new_pages.is_empty());
        assert!(diff.unchanged_pages.is_empty());
    }

    #[test]
    fn diff_detects_removed_pages() {
        let existing = vec![
            make_page_meta("index", "h1"),
            make_page_meta("removed", "h2"),
        ];
        let fetched = vec![make_fetched_page("index", "h1")];

        let diff = diff_pages(&existing, &fetched, false);
        assert_eq!(diff.removed_pages, vec!["removed"]);
        assert_eq!(diff.unchanged_pages, vec!["index"]);
    }

    #[test]
    fn diff_force_marks_all_changed() {
        let existing = vec![make_page_meta("index", "same-hash")];
        let fetched = vec![make_fetched_page("index", "same-hash")];

        let diff = diff_pages(&existing, &fetched, true);
        // Force mode: even same hash → changed
        assert_eq!(diff.changed_pages, vec!["index"]);
        assert!(diff.unchanged_pages.is_empty());
    }

    #[test]
    fn diff_empty_existing() {
        let fetched = vec![
            make_fetched_page("a", "h1"),
            make_fetched_page("b", "h2"),
        ];

        let diff = diff_pages(&[], &fetched, false);
        assert_eq!(diff.new_pages.len(), 2);
        assert!(diff.changed_pages.is_empty());
        assert!(diff.removed_pages.is_empty());
    }

    #[test]
    fn diff_empty_fetched() {
        let existing = vec![make_page_meta("index", "h1")];
        let diff = diff_pages(&existing, &[], false);
        assert_eq!(diff.removed_pages, vec!["index"]);
        assert!(diff.new_pages.is_empty());
    }

    #[test]
    fn diff_mixed_scenario() {
        let existing = vec![
            make_page_meta("page-a", "ha"),
            make_page_meta("page-b", "hb"),
            make_page_meta("page-c", "hc"),
        ];
        let fetched = vec![
            make_fetched_page("page-a", "ha"),         // unchanged
            make_fetched_page("page-b", "hb-changed"), // changed
            make_fetched_page("page-d", "hd"),         // new
            // page-c removed
        ];

        let diff = diff_pages(&existing, &fetched, false);
        assert_eq!(diff.unchanged_pages, vec!["page-a"]);
        assert_eq!(diff.changed_pages, vec!["page-b"]);
        assert_eq!(diff.new_pages, vec!["page-d"]);
        assert_eq!(diff.removed_pages, vec!["page-c"]);
    }

    #[test]
    fn update_result_fields() {
        let result = UpdateKbResult {
            kb_id: KbId::new(),
            pages_added: 2,
            pages_removed: 1,
            pages_changed: 3,
            pages_unchanged: 10,
            page_count: 15,
            elapsed: std::time::Duration::from_secs(5),
        };
        assert_eq!(result.pages_added, 2);
        assert_eq!(result.pages_removed, 1);
        assert_eq!(result.pages_changed, 3);
        assert_eq!(result.pages_unchanged, 10);
        assert_eq!(result.page_count, 15);
    }
}
