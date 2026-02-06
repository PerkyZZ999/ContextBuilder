//! llms.txt / llms-full.txt discovery and detection logic.
//!
//! Before crawling a site, ContextBuilder first checks whether the site publishes
//! an `llms.txt` file (per <https://llmstxt.org/>). If found, we parse it to
//! extract page URLs instead of crawling, which is faster and more respectful.

mod parser;

use contextbuilder_shared::{ContextBuilderError, Result};
use reqwest::Client;
use tracing::{debug, info, instrument};
use url::Url;

pub use parser::{LlmsEntry, LlmsParsed, LlmsSection};

/// Maximum number of redirects to follow when fetching llms.txt.
const MAX_REDIRECTS: usize = 3;

/// Default timeout in seconds for fetching llms.txt.
const DEFAULT_TIMEOUT_SECS: u64 = 10;

/// Maximum response size we consider valid (10 MB).
const MAX_RESPONSE_SIZE: u64 = 10 * 1024 * 1024;

/// User-Agent string for discovery requests.
const USER_AGENT: &str = concat!("ContextBuilder/", env!("CARGO_PKG_VERSION"));

// ---------------------------------------------------------------------------
// DiscoveryResult
// ---------------------------------------------------------------------------

/// Outcome of the llms.txt discovery process.
#[derive(Debug, Clone)]
pub enum DiscoveryResult {
    /// An llms.txt (and optionally llms-full.txt) was found at the origin.
    Found {
        /// The parsed llms.txt content.
        parsed: LlmsParsed,
        /// Raw content of llms.txt.
        llms_txt: String,
        /// Raw content of llms-full.txt, if also present.
        llms_full_txt: Option<String>,
    },
    /// No valid llms.txt was found; caller should fall back to crawling.
    NotFound,
}

// ---------------------------------------------------------------------------
// Discovery options
// ---------------------------------------------------------------------------

/// Configuration for the discovery process.
#[derive(Debug, Clone)]
pub struct DiscoveryOptions {
    /// Timeout for HTTP requests in seconds.
    pub timeout_secs: u64,
}

impl Default for DiscoveryOptions {
    fn default() -> Self {
        Self {
            timeout_secs: DEFAULT_TIMEOUT_SECS,
        }
    }
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/// Discover llms.txt / llms-full.txt at the given URL's origin.
///
/// Checks `<origin>/llms.txt` and `<origin>/llms-full.txt` (in parallel),
/// validates the content is well-formed Markdown starting with an H1,
/// and parses it into structured sections with linked URLs.
#[instrument(skip_all, fields(url = %url))]
pub async fn discover(url: &Url, opts: &DiscoveryOptions) -> Result<DiscoveryResult> {
    let origin = origin_url(url)?;
    let llms_url = format!("{origin}/llms.txt");
    let llms_full_url = format!("{origin}/llms-full.txt");

    info!(%llms_url, "checking for llms.txt");

    let client = build_client(opts)?;

    // Fetch llms.txt and llms-full.txt concurrently
    let (llms_result, llms_full_result) = tokio::join!(
        fetch_and_validate(&client, &llms_url),
        fetch_and_validate(&client, &llms_full_url),
    );

    let llms_txt = match llms_result {
        Ok(content) => content,
        Err(e) => {
            debug!(error = %e, "llms.txt not found or invalid");
            return Ok(DiscoveryResult::NotFound);
        }
    };

    let llms_full_txt = match llms_full_result {
        Ok(content) => {
            info!("llms-full.txt also found");
            Some(content)
        }
        Err(e) => {
            debug!(error = %e, "llms-full.txt not found (optional)");
            None
        }
    };

    // Parse the llms.txt content into structured data
    let parsed = parser::parse_llms_txt(&llms_txt)?;

    info!(
        title = %parsed.title,
        sections = parsed.sections.len(),
        entries = parsed.entries.len(),
        "llms.txt discovered and parsed"
    );

    Ok(DiscoveryResult::Found {
        parsed,
        llms_txt,
        llms_full_txt,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract the origin (scheme + host + port) from a URL.
fn origin_url(url: &Url) -> Result<String> {
    let scheme = url.scheme();
    let host = url.host_str().ok_or_else(|| {
        ContextBuilderError::validation(format!("URL has no host: {url}"))
    })?;

    match url.port() {
        Some(port) => Ok(format!("{scheme}://{host}:{port}")),
        None => Ok(format!("{scheme}://{host}")),
    }
}

/// Build a reqwest client with appropriate settings.
fn build_client(opts: &DiscoveryOptions) -> Result<Client> {
    Client::builder()
        .user_agent(USER_AGENT)
        .redirect(reqwest::redirect::Policy::limited(MAX_REDIRECTS))
        .timeout(std::time::Duration::from_secs(opts.timeout_secs))
        .build()
        .map_err(|e| ContextBuilderError::Network(format!("failed to build HTTP client: {e}")))
}

/// Fetch a URL and validate the response is valid Markdown content.
async fn fetch_and_validate(client: &Client, url: &str) -> Result<String> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| ContextBuilderError::Network(format!("{url}: {e}")))?;

    let status = response.status();
    if !status.is_success() {
        return Err(ContextBuilderError::Network(format!(
            "{url}: HTTP {status}"
        )));
    }

    // Check content-length if available
    if let Some(len) = response.content_length() {
        if len > MAX_RESPONSE_SIZE {
            return Err(ContextBuilderError::validation(format!(
                "{url}: response too large ({len} bytes, max {MAX_RESPONSE_SIZE})"
            )));
        }
    }

    let body = response
        .text()
        .await
        .map_err(|e| ContextBuilderError::Network(format!("{url}: failed to read body: {e}")))?;

    // Validate that the content starts with an H1 (Markdown heading)
    let trimmed = body.trim_start();
    if !trimmed.starts_with("# ") {
        return Err(ContextBuilderError::validation(format!(
            "{url}: content does not start with an H1 heading"
        )));
    }

    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_origin_url_simple() {
        let url = Url::parse("https://docs.example.com/foo/bar").unwrap();
        assert_eq!(origin_url(&url).unwrap(), "https://docs.example.com");
    }

    #[test]
    fn test_origin_url_with_port() {
        let url = Url::parse("http://localhost:3000/docs").unwrap();
        assert_eq!(origin_url(&url).unwrap(), "http://localhost:3000");
    }

    #[tokio::test]
    async fn test_discover_with_mock_server() {
        let server = wiremock::MockServer::start().await;

        let llms_content = std::fs::read_to_string("../../../fixtures/llms/valid-llms.txt")
            .expect("read llms fixture");

        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/llms.txt"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(&llms_content))
            .mount(&server)
            .await;

        // llms-full.txt returns 404
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/llms-full.txt"))
            .respond_with(wiremock::ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let url = Url::parse(&server.uri()).unwrap();
        let opts = DiscoveryOptions::default();
        let result = discover(&url, &opts).await.unwrap();

        match result {
            DiscoveryResult::Found { parsed, llms_full_txt, .. } => {
                assert_eq!(parsed.title, "Example Docs");
                assert_eq!(parsed.summary, Some("Example documentation for testing the ContextBuilder discovery module.".into()));
                assert!(!parsed.sections.is_empty());
                assert!(!parsed.entries.is_empty());
                assert!(llms_full_txt.is_none());
            }
            DiscoveryResult::NotFound => panic!("expected Found, got NotFound"),
        }
    }

    #[tokio::test]
    async fn test_discover_with_full_txt() {
        let server = wiremock::MockServer::start().await;

        let llms_content = std::fs::read_to_string("../../../fixtures/llms/valid-llms.txt")
            .expect("read llms fixture");
        let full_content = std::fs::read_to_string("../../../fixtures/llms/valid-llms-full.txt")
            .expect("read llms-full fixture");

        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/llms.txt"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(&llms_content))
            .mount(&server)
            .await;

        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/llms-full.txt"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(&full_content))
            .mount(&server)
            .await;

        let url = Url::parse(&server.uri()).unwrap();
        let opts = DiscoveryOptions::default();
        let result = discover(&url, &opts).await.unwrap();

        match result {
            DiscoveryResult::Found { llms_full_txt, .. } => {
                assert!(llms_full_txt.is_some());
            }
            DiscoveryResult::NotFound => panic!("expected Found"),
        }
    }

    #[tokio::test]
    async fn test_discover_not_found() {
        let server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/llms.txt"))
            .respond_with(wiremock::ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let url = Url::parse(&server.uri()).unwrap();
        let opts = DiscoveryOptions::default();
        let result = discover(&url, &opts).await.unwrap();

        assert!(matches!(result, DiscoveryResult::NotFound));
    }

    #[tokio::test]
    async fn test_discover_invalid_content() {
        let server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/llms.txt"))
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .set_body_string("This is not valid llms.txt\nNo H1 heading"),
            )
            .mount(&server)
            .await;

        let url = Url::parse(&server.uri()).unwrap();
        let opts = DiscoveryOptions::default();
        let result = discover(&url, &opts).await.unwrap();

        // Invalid content → NotFound (graceful fallback)
        assert!(matches!(result, DiscoveryResult::NotFound));
    }
}
