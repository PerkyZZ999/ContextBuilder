//! End-to-end `add` pipeline: URL → discovery → crawl → convert → assemble → KB.

use std::path::PathBuf;
use std::time::Instant;

use tracing::{info, instrument, warn};
use url::Url;

use contextbuilder_crawler::{CrawlResult, Crawler, FetchedPage};
use contextbuilder_discovery::{DiscoveryOptions, DiscoveryResult};
use contextbuilder_markdown::ConvertOptions;
use contextbuilder_shared::{
    CrawlConfig, ContextBuilderError, KbId, Result,
};
use contextbuilder_storage::Storage;

use crate::assembler::{AssembleConfig, AssemblePage, EnrichmentMeta};
use crate::enrichment::{self, EnrichmentConfig, EnrichmentProgress};
use crate::toc;

/// Configuration for the `add_kb` pipeline.
#[derive(Debug, Clone)]
pub struct AddKbConfig {
    /// URL to ingest.
    pub url: Url,
    /// Human-readable name (defaults to hostname).
    pub name: String,
    /// Output root directory for KB storage.
    pub output_root: PathBuf,
    /// Discovery mode: "auto", "llms-txt", or "crawl".
    pub mode: String,
    /// Crawl configuration.
    pub crawl: CrawlConfig,
    /// Tool version string.
    pub tool_version: String,
    /// OpenRouter model ID for enrichment.
    pub model_id: String,
    /// Bridge command (e.g., "bun").
    pub bridge_cmd: String,
    /// Bridge script path.
    pub bridge_script: String,
    /// Working directory for the bridge subprocess.
    pub bridge_working_dir: String,
}

/// Result of the `add_kb` pipeline.
#[derive(Debug)]
pub struct AddKbResult {
    /// Path to the assembled KB directory.
    pub kb_path: PathBuf,
    /// KB identifier.
    pub kb_id: KbId,
    /// Number of pages ingested.
    pub page_count: usize,
    /// Discovery method used.
    pub method: String,
    /// Total elapsed time.
    pub elapsed: std::time::Duration,
}

/// Progress callback for reporting pipeline status.
pub trait ProgressReporter: Send + Sync {
    /// Called when entering a new phase.
    fn phase(&self, name: &str);
    /// Called when a page is fetched during crawl.
    fn page_fetched(&self, url: &str, current: usize, total_estimate: usize);
    /// Called when a page is converted.
    fn page_converted(&self, path: &str, current: usize, total: usize);
    /// Called when the pipeline completes.
    fn done(&self, result: &AddKbResult);
}

/// No-op progress reporter for headless/test usage.
pub struct SilentProgress;

impl ProgressReporter for SilentProgress {
    fn phase(&self, _name: &str) {}
    fn page_fetched(&self, _url: &str, _current: usize, _total_estimate: usize) {}
    fn page_converted(&self, _path: &str, _current: usize, _total: usize) {}
    fn done(&self, _result: &AddKbResult) {}
}

/// Run the full `add` pipeline.
///
/// 1. Discovery: check for llms.txt
/// 2. Crawl (if needed)
/// 3. Convert HTML → Markdown
/// 4. Build TOC
/// 5. Assemble KB directory
#[instrument(skip_all, fields(url = %config.url, name = %config.name))]
pub async fn add_kb(
    config: &AddKbConfig,
    progress: &dyn ProgressReporter,
) -> Result<AddKbResult> {
    let start = Instant::now();
    let kb_id = KbId::new();

    info!(%kb_id, url = %config.url, "starting add pipeline");

    // --- Phase 1: Storage ---
    progress.phase("Initializing storage");
    let db_path = config
        .output_root
        .join(kb_id.to_string())
        .join("indexes")
        .join("contextbuilder.db");

    // Ensure parent dirs exist
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| ContextBuilderError::io(parent, e))?;
    }

    let storage = Storage::open(&db_path).await?;
    storage
        .insert_kb(
            &kb_id.to_string(),
            &config.name,
            config.url.as_str(),
            None,
        )
        .await?;

    // --- Phase 2: Discovery / Crawl ---
    let (fetched_pages, method) = match config.mode.as_str() {
        "llms-txt" => {
            progress.phase("Discovering llms.txt");
            discover_and_fetch(&config.url, &storage, &kb_id, progress).await?
        }
        "crawl" => {
            progress.phase("Crawling documentation");
            let (_result, pages) =
                crawl_pages(&config.url, &config.crawl, &kb_id, &storage, progress).await?;
            (pages, "crawl".to_string())
        }
        _ => {
            // Auto mode: try discovery first, fall back to crawl
            progress.phase("Discovering llms.txt");
            match discover_and_fetch(&config.url, &storage, &kb_id, progress).await {
                Ok((pages, method)) if !pages.is_empty() => (pages, method),
                _ => {
                    progress.phase("Crawling documentation");
                    let (_result, pages) = crawl_pages(
                        &config.url,
                        &config.crawl,
                        &kb_id,
                        &storage,
                        progress,
                    )
                    .await?;
                    (pages, "crawl".to_string())
                }
            }
        }
    };

    if fetched_pages.is_empty() {
        return Err(ContextBuilderError::validation(
            "no pages were fetched from the documentation source",
        ));
    }

    // --- Phase 3: Convert HTML → Markdown ---
    progress.phase("Converting to Markdown");
    let mut assembled_pages: Vec<AssemblePage> = Vec::new();
    let total = fetched_pages.len();

    for (i, page) in fetched_pages.iter().enumerate() {
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
                warn!(url = %page.meta.url, error = %e, "conversion failed, skipping page");
            }
        }
    }

    // --- Phase 4: Build TOC ---
    progress.phase("Building table of contents");
    let page_metas: Vec<_> = fetched_pages.iter().map(|p| p.meta.clone()).collect();
    let toc = toc::build_toc(&page_metas, &[]);

    // --- Phase 5: Assemble KB ---
    progress.phase("Assembling knowledge base");
    let assemble_config = AssembleConfig {
        kb_id: kb_id.clone(),
        name: config.name.clone(),
        source_url: config.url.to_string(),
        output_root: config.output_root.clone(),
        tool_version: config.tool_version.clone(),
    };

    let assemble_result =
        crate::assembler::assemble(&assemble_config, &assembled_pages, &toc)?;

    // --- Phase 6: Enrichment ---
    progress.phase("Running LLM enrichment");

    let enrich_config = EnrichmentConfig {
        bridge_cmd: config.bridge_cmd.clone(),
        bridge_script: config.bridge_script.clone(),
        working_dir: config.bridge_working_dir.clone(),
        model_id: config.model_id.clone(),
        kb_name: config.name.clone(),
        kb_source_url: config.url.to_string(),
    };

    // Collect pages with their markdown content for enrichment
    let pages_with_content: Vec<(contextbuilder_shared::PageMeta, String)> = fetched_pages
        .iter()
        .zip(assembled_pages.iter())
        .map(|(fp, ap)| (fp.meta.clone(), ap.markdown.clone()))
        .collect();

    let enrich_progress = PipelineEnrichmentProgress { inner: progress };
    let enrich_results = enrichment::run_enrichment(
        &enrich_config,
        &pages_with_content,
        &toc,
        &storage,
        &enrich_progress,
    )
    .await?;

    // --- Phase 7: Generate & write artifacts ---
    progress.phase("Generating artifacts");

    let summary_text = enrich_results
        .summaries
        .values()
        .next()
        .cloned()
        .unwrap_or_else(|| format!("Documentation for {}", config.name));

    let llms_txt = contextbuilder_artifacts::generate_llms_txt(
        &config.name,
        &summary_text,
        &toc,
        &enrich_results.descriptions,
        config.url.as_str(),
        &config.tool_version,
    );

    let full_pages: Vec<contextbuilder_artifacts::FullPage> = assembled_pages
        .iter()
        .zip(fetched_pages.iter())
        .map(|(ap, fp)| contextbuilder_artifacts::FullPage {
            title: ap.title.clone(),
            url: fp.meta.url.clone(),
            content: ap.markdown.clone(),
        })
        .collect();

    let llms_full_txt = contextbuilder_artifacts::generate_llms_full_txt(
        &config.name,
        &full_pages,
        config.url.as_str(),
        &config.tool_version,
    );

    let skill_md = contextbuilder_artifacts::generate_skill_md(
        &config.name,
        config.url.as_str(),
        &summary_text,
        enrich_results.skill_md.as_deref(),
        &config.tool_version,
    );

    let rules = contextbuilder_artifacts::generate_rules(
        &config.name,
        config.url.as_str(),
        enrich_results.rules.as_deref(),
        &config.tool_version,
    );

    let style = contextbuilder_artifacts::generate_style(
        &config.name,
        config.url.as_str(),
        enrich_results.style.as_deref(),
        &config.tool_version,
    );

    let do_dont = contextbuilder_artifacts::generate_do_dont(
        &config.name,
        config.url.as_str(),
        enrich_results.do_dont.as_deref(),
        &config.tool_version,
    );

    let artifacts: Vec<(&str, &str)> = vec![
        ("llms.txt", &llms_txt),
        ("llms-full.txt", &llms_full_txt),
        ("SKILL.md", &skill_md),
        ("rules.md", &rules),
        ("style.md", &style),
        ("do_dont.md", &do_dont),
    ];

    let now = chrono::Utc::now();
    let enrichment_meta = EnrichmentMeta {
        model: enrich_results.model.clone(),
        total_tokens_in: enrich_results.total_tokens_in,
        total_tokens_out: enrich_results.total_tokens_out,
        cache_hits: enrich_results.cache_hits,
        cache_misses: enrich_results.cache_misses,
        completed_at: now.to_rfc3339(),
    };

    crate::assembler::assemble_artifacts(&assemble_result.kb_path, &artifacts, &enrichment_meta)?;

    let result = AddKbResult {
        kb_path: assemble_result.kb_path,
        kb_id,
        page_count: assembled_pages.len(),
        method,
        elapsed: start.elapsed(),
    };

    progress.done(&result);

    info!(
        kb_id = %result.kb_id,
        page_count = result.page_count,
        method = %result.method,
        elapsed_ms = result.elapsed.as_millis(),
        "add pipeline complete"
    );

    Ok(result)
}

// ---------------------------------------------------------------------------
// Enrichment progress adapter
// ---------------------------------------------------------------------------

/// Adapts a `ProgressReporter` to the `EnrichmentProgress` interface.
struct PipelineEnrichmentProgress<'a> {
    inner: &'a dyn ProgressReporter,
}

impl EnrichmentProgress for PipelineEnrichmentProgress<'_> {
    fn phase(&self, name: &str) {
        self.inner.phase(name);
    }

    fn task_progress(&self, current: usize, total: usize, detail: &str) {
        self.inner.phase(&format!("[{current}/{total}] {detail}"));
    }
}

// ---------------------------------------------------------------------------
// Discovery path
// ---------------------------------------------------------------------------

/// Try llms.txt discovery and fetch linked pages.
async fn discover_and_fetch(
    url: &Url,
    storage: &Storage,
    kb_id: &KbId,
    progress: &dyn ProgressReporter,
) -> Result<(Vec<FetchedPage>, String)> {
    let opts = DiscoveryOptions { timeout_secs: 10 };
    let discovery = contextbuilder_discovery::discover(url, &opts).await?;

    match discovery {
        DiscoveryResult::Found {
            parsed,
            llms_txt: _,
            llms_full_txt: _,
        } => {
            info!(
                title = %parsed.title,
                entries = parsed.entries.len(),
                "llms.txt discovered"
            );

            // Extract URLs from the parsed llms.txt
            let urls: Vec<Url> = parsed
                .entries
                .iter()
                .filter_map(|e| Url::parse(&e.url).ok())
                .collect();

            if urls.is_empty() {
                return Ok((vec![], "llms-txt".to_string()));
            }

            // Fetch each linked page
            let client = reqwest::Client::builder()
                .user_agent(concat!("ContextBuilder/", env!("CARGO_PKG_VERSION")))
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .map_err(|e| ContextBuilderError::Network(format!("client build: {e}")))?;

            let mut pages = Vec::new();
            let total = urls.len();

            for (i, page_url) in urls.iter().enumerate() {
                progress.page_fetched(page_url.as_str(), i + 1, total);

                match fetch_single_page(&client, page_url, &kb_id.to_string()).await {
                    Ok(page) => {
                        let _ = storage.upsert_page(&page.meta).await;
                        pages.push(page);
                    }
                    Err(e) => {
                        warn!(url = %page_url, error = %e, "failed to fetch llms.txt link");
                    }
                }
            }

            Ok((pages, "llms-txt".to_string()))
        }
        DiscoveryResult::NotFound => Ok((vec![], "none".to_string())),
    }
}

/// Fetch a single page via HTTP.
async fn fetch_single_page(
    client: &reqwest::Client,
    url: &Url,
    kb_id: &str,
) -> Result<FetchedPage> {
    let response = client
        .get(url.as_str())
        .send()
        .await
        .map_err(|e| ContextBuilderError::Network(format!("{url}: {e}")))?;

    let status = response.status();
    let status_code = status.as_u16();

    if !status.is_success() {
        return Err(ContextBuilderError::Network(format!(
            "{url}: HTTP {status}"
        )));
    }

    let body = response
        .text()
        .await
        .map_err(|e| ContextBuilderError::Network(format!("{url}: {e}")))?;

    let content_hash = {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(body.as_bytes());
        format!("{:x}", hasher.finalize())
    };

    let page_path = contextbuilder_crawler::url_to_path(url);

    let title = {
        let doc = scraper::Html::parse_document(&body);
        let h1_sel = scraper::Selector::parse("h1").unwrap();
        doc.select(&h1_sel)
            .next()
            .map(|el| el.text().collect::<String>().trim().to_string())
    };

    let meta = contextbuilder_shared::PageMeta {
        id: uuid::Uuid::now_v7().to_string(),
        kb_id: kb_id.to_string(),
        url: url.to_string(),
        path: page_path,
        title,
        content_hash,
        fetched_at: chrono::Utc::now(),
        status_code: Some(status_code),
        content_len: Some(body.len()),
    };

    let content = contextbuilder_crawler::ExtractedContent {
        html: body.clone(),
        meta: contextbuilder_crawler::adapters::PageMeta {
            title: meta.title.clone(),
        },
    };

    Ok(FetchedPage {
        meta,
        content,
        html: body,
        links: vec![],
    })
}

// ---------------------------------------------------------------------------
// Crawl path
// ---------------------------------------------------------------------------

/// Run the crawler to fetch pages.
async fn crawl_pages(
    url: &Url,
    crawl_config: &CrawlConfig,
    kb_id: &KbId,
    storage: &Storage,
    _progress: &dyn ProgressReporter,
) -> Result<(CrawlResult, Vec<FetchedPage>)> {
    let crawler = Crawler::new(crawl_config.clone())?;
    let (result, pages) = crawler
        .crawl(url, &kb_id.to_string(), storage)
        .await?;

    info!(
        pages_fetched = result.pages_fetched,
        pages_skipped = result.pages_skipped,
        errors = result.errors.len(),
        "crawl complete"
    );

    Ok((result, pages))
}
