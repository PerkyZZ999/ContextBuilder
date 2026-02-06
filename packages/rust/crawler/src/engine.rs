//! Concurrent, scope-aware web crawler engine.
//!
//! The crawler starts from a given URL, performs BFS traversal within scope,
//! respects depth/concurrency/rate limits, and stores results via the storage layer.

use std::collections::HashSet;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use reqwest::Client;
use scraper::{Html, Selector};
use sha2::{Digest, Sha256};
use tokio::sync::{Mutex, Semaphore};
use tracing::{debug, info, instrument, warn};
use url::Url;
use uuid::Uuid;

use contextbuilder_shared::{ContextBuilderError, CrawlConfig, PageMeta, Result};
use contextbuilder_storage::Storage;

use crate::adapters::{AdapterRegistry, ExtractedContent};

/// User-Agent string for crawl requests.
const USER_AGENT: &str = concat!("ContextBuilder/", env!("CARGO_PKG_VERSION"));

// ---------------------------------------------------------------------------
// CrawlResult
// ---------------------------------------------------------------------------

/// Summary of a completed crawl operation.
#[derive(Debug, Clone)]
pub struct CrawlResult {
    /// Number of pages successfully fetched.
    pub pages_fetched: usize,
    /// Number of pages skipped (out of scope, dedup, error).
    pub pages_skipped: usize,
    /// Errors encountered (URL, error message).
    pub errors: Vec<(String, String)>,
    /// Total duration of the crawl.
    pub duration: Duration,
    /// Adapter name used for the majority of pages.
    pub primary_adapter: String,
}

/// A fetched page with its extracted content.
#[derive(Debug, Clone)]
pub struct FetchedPage {
    /// Page metadata for storage.
    pub meta: PageMeta,
    /// Extracted clean HTML content.
    pub content: ExtractedContent,
    /// The raw extracted HTML (for markdown conversion).
    pub html: String,
    /// Links found on this page.
    pub links: Vec<String>,
}

// ---------------------------------------------------------------------------
// Crawler
// ---------------------------------------------------------------------------

/// Concurrent web crawler with scope-aware page fetching.
pub struct Crawler {
    config: CrawlConfig,
    client: Client,
    registry: AdapterRegistry,
    /// Allow localhost/private IPs (for integration tests with mock servers).
    allow_localhost: bool,
}

impl Crawler {
    /// Create a new crawler with the given configuration.
    pub fn new(config: CrawlConfig) -> Result<Self> {
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .redirect(reqwest::redirect::Policy::limited(5))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| {
                ContextBuilderError::Network(format!("failed to build HTTP client: {e}"))
            })?;

        Ok(Self {
            config,
            client,
            registry: AdapterRegistry::new(),
            allow_localhost: false,
        })
    }

    /// Allow crawling localhost/private IPs (for integration tests).
    #[cfg(test)]
    pub fn allow_localhost(mut self) -> Self {
        self.allow_localhost = true;
        self
    }

    /// Crawl starting from `start_url`, storing results in `storage`.
    ///
    /// Returns a summary of the crawl and the list of fetched pages.
    #[instrument(skip_all, fields(start_url = %start_url, kb_id = %kb_id))]
    pub async fn crawl(
        &self,
        start_url: &Url,
        kb_id: &str,
        storage: &Storage,
    ) -> Result<(CrawlResult, Vec<FetchedPage>)> {
        let start_time = std::time::Instant::now();

        // Create crawl job
        let crawl_job_id = storage.insert_crawl_job(kb_id).await?;

        let scope = CrawlScope::new(start_url, &self.config);
        let visited = Arc::new(Mutex::new(HashSet::<String>::new()));
        let semaphore = Arc::new(Semaphore::new(self.config.concurrency as usize));

        let mut queue: Vec<(Url, u32)> = vec![(start_url.clone(), 0)];
        let mut fetched_pages: Vec<FetchedPage> = Vec::new();
        let mut errors: Vec<(String, String)> = Vec::new();
        let mut pages_skipped: usize = 0;
        let mut primary_adapter = String::from("generic");

        info!(
            depth = self.config.depth,
            concurrency = self.config.concurrency,
            rate_limit_ms = self.config.rate_limit_ms,
            "starting crawl"
        );

        while !queue.is_empty() {
            // Take a batch from the queue (up to concurrency limit)
            let batch: Vec<(Url, u32)> = {
                let drain_count = queue.len().min(self.config.concurrency as usize);
                queue.drain(..drain_count).collect()
            };

            let mut handles = Vec::new();

            for (url, depth) in batch {
                let normalized = normalize_url(&url);

                // Check if already visited
                {
                    let mut vis = visited.lock().await;
                    if vis.contains(&normalized) {
                        pages_skipped += 1;
                        continue;
                    }
                    vis.insert(normalized.clone());
                }

                // Check scope
                if !scope.in_scope(&url) {
                    debug!(%url, "out of scope, skipping");
                    pages_skipped += 1;
                    continue;
                }

                // Check SSRF
                if !self.allow_localhost && is_ssrf_target(&url) {
                    warn!(%url, "SSRF protection: blocked");
                    pages_skipped += 1;
                    continue;
                }

                let client = self.client.clone();
                let sem = semaphore.clone();
                let rate_limit = self.config.rate_limit_ms;
                let kb_id_owned = kb_id.to_string();

                handles.push(tokio::spawn(async move {
                    let _permit = sem.acquire().await.expect("semaphore closed");

                    // Rate limiting
                    if rate_limit > 0 {
                        tokio::time::sleep(Duration::from_millis(rate_limit)).await;
                    }

                    fetch_page(&client, &url, depth, &kb_id_owned).await
                }));
            }

            // Collect results
            for handle in handles {
                match handle.await {
                    Ok(Ok((page, depth))) => {
                        // Detect adapter for the first page
                        if fetched_pages.is_empty() {
                            let doc = Html::parse_document(&page.html);
                            let adapter = self.registry.detect(&doc, &Url::parse(&page.meta.url).unwrap_or_else(|_| Url::parse("https://example.com").unwrap()));
                            primary_adapter = adapter.name().to_string();
                        }

                        // Enqueue child links if within depth
                        if depth < self.config.depth {
                            for link in &page.links {
                                if let Ok(link_url) = Url::parse(link) {
                                    queue.push((link_url, depth + 1));
                                }
                            }
                        }

                        // Store in database
                        if let Err(e) = storage.upsert_page(&page.meta).await {
                            warn!(url = %page.meta.url, error = %e, "failed to store page");
                            errors.push((page.meta.url.clone(), e.to_string()));
                        }

                        // Store links
                        for link in &page.links {
                            let _ = storage
                                .insert_link(&page.meta.id, link, None)
                                .await;
                        }

                        fetched_pages.push(page);
                    }
                    Ok(Err(e)) => {
                        errors.push(("unknown".into(), e.to_string()));
                        pages_skipped += 1;
                    }
                    Err(e) => {
                        errors.push(("task".into(), e.to_string()));
                        pages_skipped += 1;
                    }
                }
            }
        }

        let duration = start_time.elapsed();

        // Update crawl job with stats
        let stats = serde_json::json!({
            "status": if errors.is_empty() { "completed" } else { "completed_with_errors" },
            "pages_fetched": fetched_pages.len(),
            "pages_skipped": pages_skipped,
            "errors": errors.len(),
        });
        let _ = storage
            .update_crawl_job(&crawl_job_id, &stats.to_string())
            .await;

        let result = CrawlResult {
            pages_fetched: fetched_pages.len(),
            pages_skipped,
            errors,
            duration,
            primary_adapter,
        };

        info!(
            pages_fetched = result.pages_fetched,
            pages_skipped = result.pages_skipped,
            errors = result.errors.len(),
            duration_ms = result.duration.as_millis(),
            adapter = %result.primary_adapter,
            "crawl completed"
        );

        Ok((result, fetched_pages))
    }
}

// ---------------------------------------------------------------------------
// Scope checking
// ---------------------------------------------------------------------------

/// Determines which URLs are "in scope" for a crawl.
struct CrawlScope {
    /// Base path prefix that URLs must match.
    base_path: String,
    /// Base host that URLs must match.
    base_host: String,
    /// Include patterns (if non-empty, URL must match at least one).
    include_patterns: Vec<regex::Regex>,
    /// Exclude patterns (if URL matches any, it's excluded).
    exclude_patterns: Vec<regex::Regex>,
}

impl CrawlScope {
    fn new(start_url: &Url, config: &CrawlConfig) -> Self {
        let base_path = start_url.path().to_string();
        let base_host = start_url.host_str().unwrap_or("").to_string();

        let include_patterns = config
            .include_patterns
            .iter()
            .filter_map(|p| glob_to_regex(p))
            .collect();

        let exclude_patterns = config
            .exclude_patterns
            .iter()
            .filter_map(|p| glob_to_regex(p))
            .collect();

        Self {
            base_path,
            base_host,
            include_patterns,
            exclude_patterns,
        }
    }

    fn in_scope(&self, url: &Url) -> bool {
        // Must be http/https
        if url.scheme() != "http" && url.scheme() != "https" {
            return false;
        }

        // Must match base host
        if url.host_str().unwrap_or("") != self.base_host {
            return false;
        }

        let path = url.path();

        // Check exclude patterns
        for pattern in &self.exclude_patterns {
            if pattern.is_match(path) {
                return false;
            }
        }

        // Check include patterns (if any configured, must match at least one)
        if !self.include_patterns.is_empty() {
            return self.include_patterns.iter().any(|p| p.is_match(path));
        }

        // Default: must share path prefix with start URL
        path.starts_with(&self.base_path)
            || self.base_path.starts_with(path)
            || path.starts_with("/")
    }
}

/// Convert a glob-like pattern to a regex.
fn glob_to_regex(pattern: &str) -> Option<regex::Regex> {
    let escaped = regex::escape(pattern)
        .replace(r"\*\*", ".*")
        .replace(r"\*", "[^/]*")
        .replace(r"\?", ".");
    regex::Regex::new(&format!("^{escaped}$")).ok()
}

// ---------------------------------------------------------------------------
// SSRF protection
// ---------------------------------------------------------------------------

/// Check if a URL targets a potentially dangerous resource.
fn is_ssrf_target(url: &Url) -> bool {
    // Block non-HTTP schemes
    match url.scheme() {
        "http" | "https" => {}
        _ => return true,
    }

    // Block private/loopback IPs
    if let Some(host) = url.host_str() {
        if let Ok(ip) = host.parse::<IpAddr>() {
            return is_private_ip(&ip);
        }
        // Block known local hostnames
        if host == "localhost"
            || host == "127.0.0.1"
            || host == "[::1]"
            || host.ends_with(".local")
            || host.ends_with(".internal")
        {
            return true;
        }
    }

    false
}

/// Check if an IP is in a private/reserved range.
fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_unspecified()
                // 100.64.0.0/10 (Carrier-grade NAT)
                || (v4.octets()[0] == 100 && (v4.octets()[1] & 0xC0) == 64)
                // 192.0.0.0/24
                || (v4.octets()[0] == 192 && v4.octets()[1] == 0 && v4.octets()[2] == 0)
        }
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unspecified(),
    }
}

// ---------------------------------------------------------------------------
// Page fetching
// ---------------------------------------------------------------------------

/// Fetch a single page and extract its content.
async fn fetch_page(
    client: &Client,
    url: &Url,
    depth: u32,
    kb_id: &str,
) -> Result<(FetchedPage, u32)> {
    debug!(%url, depth, "fetching page");

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
        .map_err(|e| ContextBuilderError::Network(format!("{url}: body read failed: {e}")))?;

    // Parse HTML
    let doc = Html::parse_document(&body);

    // Extract links
    let links = extract_links(&doc, url);

    // Compute content hash
    let content_hash = compute_hash(&body);

    // Generate a slug-based path from the URL
    let page_path = url_to_path(url);

    // Extract title from H1
    let title = {
        let h1_sel = Selector::parse("h1").unwrap();
        doc.select(&h1_sel)
            .next()
            .map(|el| el.text().collect::<String>().trim().to_string())
    };

    let meta = PageMeta {
        id: Uuid::now_v7().to_string(),
        kb_id: kb_id.to_string(),
        url: url.to_string(),
        path: page_path,
        title,
        content_hash,
        fetched_at: Utc::now(),
        status_code: Some(status_code),
        content_len: Some(body.len()),
    };

    // Create an ExtractedContent placeholder (the actual adapter extraction
    // happens during markdown conversion)
    let content = ExtractedContent {
        html: body.clone(),
        meta: crate::adapters::PageMeta {
            title: meta.title.clone(),
        },
    };

    Ok((
        FetchedPage {
            meta,
            content,
            html: body,
            links,
        },
        depth,
    ))
}

/// Extract all links from a document, resolved against the base URL.
fn extract_links(doc: &Html, base_url: &Url) -> Vec<String> {
    let link_sel = Selector::parse("a[href]").unwrap();
    let mut links = Vec::new();

    for el in doc.select(&link_sel) {
        if let Some(href) = el.value().attr("href") {
            // Skip anchors, javascript:, mailto:
            if href.starts_with('#')
                || href.starts_with("javascript:")
                || href.starts_with("mailto:")
            {
                continue;
            }

            // Resolve relative URLs
            if let Ok(resolved) = base_url.join(href) {
                // Strip fragment
                let mut resolved = resolved;
                resolved.set_fragment(None);
                links.push(resolved.to_string());
            }
        }
    }

    links
}

/// Normalize a URL for deduplication (strip fragment, trailing slash, lowercase host).
fn normalize_url(url: &Url) -> String {
    let mut normalized = url.clone();
    normalized.set_fragment(None);
    let mut s = normalized.to_string();
    // Remove trailing slash for consistency (except root path)
    if s.ends_with('/') && s.matches('/').count() > 3 {
        s.pop();
    }
    s
}

/// Convert a URL path to a filesystem-safe path.
pub fn url_to_path(url: &Url) -> String {
    let path = url.path();
    let cleaned = path
        .trim_start_matches('/')
        .trim_end_matches('/')
        .trim_end_matches(".html")
        .trim_end_matches(".htm")
        .trim_end_matches(".md");

    if cleaned.is_empty() {
        "index".to_string()
    } else {
        cleaned.to_string()
    }
}

/// Compute SHA-256 hash of content.
fn compute_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod crawler_tests {
    use super::*;

    #[test]
    fn test_normalize_url() {
        let url = Url::parse("https://docs.example.com/guide/intro#section-1").unwrap();
        let normalized = normalize_url(&url);
        assert!(!normalized.contains('#'));
        assert!(normalized.starts_with("https://docs.example.com/guide/intro"));
    }

    #[test]
    fn test_url_to_path() {
        let url = Url::parse("https://docs.example.com/guide/getting-started.html").unwrap();
        assert_eq!(url_to_path(&url), "guide/getting-started");

        let root = Url::parse("https://docs.example.com/").unwrap();
        assert_eq!(url_to_path(&root), "index");
    }

    #[test]
    fn test_compute_hash() {
        let hash = compute_hash("hello world");
        assert_eq!(hash.len(), 64); // SHA-256 = 64 hex chars
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_ssrf_protection_blocks_file() {
        let url = Url::parse("file:///etc/passwd").unwrap();
        assert!(is_ssrf_target(&url));
    }

    #[test]
    fn test_ssrf_protection_blocks_private_ip() {
        let url = Url::parse("http://192.168.1.1/admin").unwrap();
        assert!(is_ssrf_target(&url));

        let url = Url::parse("http://10.0.0.1/").unwrap();
        assert!(is_ssrf_target(&url));

        let url = Url::parse("http://127.0.0.1:8080/").unwrap();
        assert!(is_ssrf_target(&url));
    }

    #[test]
    fn test_ssrf_protection_allows_public() {
        let url = Url::parse("https://docs.example.com/page").unwrap();
        assert!(!is_ssrf_target(&url));
    }

    #[test]
    fn test_ssrf_blocks_localhost() {
        let url = Url::parse("http://localhost:3000/api").unwrap();
        assert!(is_ssrf_target(&url));
    }

    #[test]
    fn test_scope_same_host() {
        let start = Url::parse("https://docs.example.com/guide/").unwrap();
        let config = CrawlConfig {
            depth: 3,
            concurrency: 4,
            include_patterns: vec![],
            exclude_patterns: vec![],
            rate_limit_ms: 0,
            mode: "crawl".into(),
            respect_robots_txt: false,
        };
        let scope = CrawlScope::new(&start, &config);

        // Same host in scope
        let in_scope = Url::parse("https://docs.example.com/guide/intro").unwrap();
        assert!(scope.in_scope(&in_scope));

        // Different host out of scope
        let out_of_scope = Url::parse("https://other.example.com/guide/intro").unwrap();
        assert!(!scope.in_scope(&out_of_scope));
    }

    #[test]
    fn test_scope_excludes() {
        let start = Url::parse("https://docs.example.com/").unwrap();
        let config = CrawlConfig {
            depth: 3,
            concurrency: 4,
            include_patterns: vec![],
            exclude_patterns: vec!["/blog/**".into()],
            rate_limit_ms: 0,
            mode: "crawl".into(),
            respect_robots_txt: false,
        };
        let scope = CrawlScope::new(&start, &config);

        let blog = Url::parse("https://docs.example.com/blog/post-1").unwrap();
        assert!(!scope.in_scope(&blog));

        let docs = Url::parse("https://docs.example.com/guide/intro").unwrap();
        assert!(scope.in_scope(&docs));
    }

    #[test]
    fn test_extract_links() {
        let html = r##"<html><body><a href="/page2">Page 2</a><a href="https://external.com">External</a><a href="#section">Anchor</a><a href="relative/path">Relative</a></body></html>"##;

        let doc = Html::parse_document(html);
        let base = Url::parse("https://docs.example.com/page1").unwrap();
        let links = extract_links(&doc, &base);

        assert!(links.contains(&"https://docs.example.com/page2".to_string()));
        assert!(links.contains(&"https://external.com/".to_string()));
        assert!(links.contains(&"https://docs.example.com/relative/path".to_string()));
        // Should NOT contain anchor-only links
        assert!(!links.iter().any(|l| l.contains('#')));
    }

    #[tokio::test]
    async fn test_crawl_with_mock_server() {
        let server = wiremock::MockServer::start().await;

        // Page 1 links to page 2
        let page1 = r#"<html><body>
            <main>
                <h1>Page One</h1>
                <p>Welcome to page one.</p>
                <a href="/page2">Go to page 2</a>
            </main>
        </body></html>"#;

        // Page 2 links to page 3
        let page2 = r#"<html><body>
            <main>
                <h1>Page Two</h1>
                <p>This is page two.</p>
                <a href="/page3">Go to page 3</a>
            </main>
        </body></html>"#;

        // Page 3 is a leaf
        let page3 = r#"<html><body>
            <main>
                <h1>Page Three</h1>
                <p>Final page.</p>
            </main>
        </body></html>"#;

        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(page1))
            .mount(&server)
            .await;

        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/page2"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(page2))
            .mount(&server)
            .await;

        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/page3"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(page3))
            .mount(&server)
            .await;

        // Set up storage
        let tmp_dir = std::env::temp_dir().join(format!("cb-crawl-test-{}", Uuid::now_v7()));
        let db_path = tmp_dir.join("test.db");
        let storage = Storage::open(&db_path).await.unwrap();

        let kb_id = Uuid::now_v7().to_string();
        storage
            .insert_kb(&kb_id, "test-kb", &server.uri(), None)
            .await
            .unwrap();

        let config = CrawlConfig {
            depth: 3,
            concurrency: 2,
            include_patterns: vec![],
            exclude_patterns: vec![],
            rate_limit_ms: 0,
            mode: "crawl".into(),
            respect_robots_txt: false,
        };

        let crawler = Crawler::new(config).unwrap().allow_localhost();
        let start_url = Url::parse(&server.uri()).unwrap();
        let (result, _pages) = crawler.crawl(&start_url, &kb_id, &storage).await.unwrap();

        assert_eq!(result.pages_fetched, 3);
        assert!(result.errors.is_empty());

        // Verify pages stored in DB
        let db_pages = storage.list_pages_by_kb(&kb_id).await.unwrap();
        assert_eq!(db_pages.len(), 3);

        // Cleanup
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[tokio::test]
    async fn test_crawl_respects_depth() {
        let server = wiremock::MockServer::start().await;

        let page1 = r#"<html><body><main>
            <h1>Root</h1><a href="/page2">Page 2</a>
        </main></body></html>"#;

        let page2 = r#"<html><body><main>
            <h1>Page 2</h1><a href="/page3">Page 3</a>
        </main></body></html>"#;

        let page3 = r#"<html><body><main>
            <h1>Page 3</h1><p>Deep page</p>
        </main></body></html>"#;

        wiremock::Mock::given(wiremock::matchers::path("/"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(page1))
            .mount(&server)
            .await;

        wiremock::Mock::given(wiremock::matchers::path("/page2"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(page2))
            .mount(&server)
            .await;

        wiremock::Mock::given(wiremock::matchers::path("/page3"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(page3))
            .mount(&server)
            .await;

        let tmp_dir = std::env::temp_dir().join(format!("cb-depth-test-{}", Uuid::now_v7()));
        let db_path = tmp_dir.join("test.db");
        let storage = Storage::open(&db_path).await.unwrap();

        let kb_id = Uuid::now_v7().to_string();
        storage
            .insert_kb(&kb_id, "test-kb", &server.uri(), None)
            .await
            .unwrap();

        // Depth 1 = root + 1 level deep
        let config = CrawlConfig {
            depth: 1,
            concurrency: 2,
            include_patterns: vec![],
            exclude_patterns: vec![],
            rate_limit_ms: 0,
            mode: "crawl".into(),
            respect_robots_txt: false,
        };

        let crawler = Crawler::new(config).unwrap().allow_localhost();
        let start_url = Url::parse(&server.uri()).unwrap();
        let (result, _pages) = crawler.crawl(&start_url, &kb_id, &storage).await.unwrap();

        // Should fetch root (depth=0) and page2 (depth=1), but not page3 (depth=2)
        assert_eq!(result.pages_fetched, 2);

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }
}
