//! Platform adapter trait and built-in adapters for content extraction.
//!
//! Adapters detect specific documentation platforms (Docusaurus, VitePress, etc.)
//! and extract content + TOC intelligently for each platform.

mod docusaurus;
mod generic;
mod gitbook;
mod readthedocs;
mod vitepress;

use contextbuilder_shared::TocEntry;
use scraper::Html;
use url::Url;

pub use docusaurus::DocusaurusAdapter;
pub use generic::GenericAdapter;
pub use gitbook::GitBookAdapter;
pub use readthedocs::ReadTheDocsAdapter;
pub use vitepress::VitePressAdapter;

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Metadata extracted by a platform adapter.
#[derive(Debug, Clone, Default)]
pub struct PageMeta {
    /// Page title (extracted from content or <title>).
    pub title: Option<String>,
}

/// Content extraction result from an adapter.
#[derive(Debug, Clone)]
pub struct ExtractedContent {
    /// Clean HTML content (nav/footer/chrome stripped).
    pub html: String,
    /// Page metadata.
    pub meta: PageMeta,
}

/// Trait for platform-specific content extraction.
///
/// Adapters are tried in priority order; `GenericAdapter` is the always-last fallback.
pub trait PlatformAdapter: Send + Sync {
    /// Try to detect this platform in the parsed HTML.
    /// Returns `true` if this adapter should handle the document.
    fn detect(&self, doc: &Html, url: &Url) -> bool;

    /// Extract a TOC from the document's navigation structure.
    fn extract_toc(&self, doc: &Html) -> Vec<TocEntry>;

    /// Extract the main content as clean HTML.
    fn extract_content(&self, doc: &Html) -> ExtractedContent;

    /// Human-readable adapter name for tracing.
    fn name(&self) -> &str;
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// Holds registered adapters in priority order.
pub struct AdapterRegistry {
    adapters: Vec<Box<dyn PlatformAdapter>>,
}

impl AdapterRegistry {
    /// Create a registry with all built-in adapters (platform-specific first, generic last).
    pub fn new() -> Self {
        Self {
            adapters: vec![
                Box::new(DocusaurusAdapter),
                Box::new(VitePressAdapter),
                Box::new(GitBookAdapter),
                Box::new(ReadTheDocsAdapter),
                Box::new(GenericAdapter),
            ],
        }
    }

    /// Detect the best adapter for the given HTML document.
    /// Always returns an adapter (GenericAdapter is the fallback).
    pub fn detect(&self, doc: &Html, url: &Url) -> &dyn PlatformAdapter {
        for adapter in &self.adapters {
            if adapter.detect(doc, url) {
                return adapter.as_ref();
            }
        }
        // Unreachable: GenericAdapter always matches
        unreachable!("GenericAdapter must always match");
    }
}

impl Default for AdapterRegistry {
    fn default() -> Self {
        Self::new()
    }
}
