//! Web crawler, content extraction, and platform adapters.
//!
//! This crate provides:
//! - [`adapters`] — Platform-specific content extractors (Docusaurus, VitePress, etc.)
//! - [`AdapterRegistry`] — Detects the best adapter for a given HTML document
//! - [`engine`] — Concurrent, scope-aware web crawler

pub mod adapters;
pub mod engine;

pub use adapters::{
    AdapterRegistry, DocusaurusAdapter, ExtractedContent, GenericAdapter, GitBookAdapter,
    PlatformAdapter, ReadTheDocsAdapter, VitePressAdapter,
};
pub use engine::{CrawlResult, Crawler, FetchedPage, url_to_path};

#[cfg(test)]
mod tests {
    use super::*;
    use scraper::Html;
    use url::Url;

    fn load_fixture(name: &str) -> Html {
        let path = format!("../../../fixtures/html/{name}");
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("missing fixture: {path}"));
        Html::parse_document(&content)
    }

    fn dummy_url() -> Url {
        Url::parse("https://docs.example.com/page").unwrap()
    }

    // -----------------------------------------------------------------------
    // Adapter detection tests
    // -----------------------------------------------------------------------

    #[test]
    fn detect_docusaurus() {
        let doc = load_fixture("docusaurus.html");
        let registry = AdapterRegistry::new();
        let adapter = registry.detect(&doc, &dummy_url());
        assert_eq!(adapter.name(), "docusaurus");
    }

    #[test]
    fn detect_vitepress() {
        let doc = load_fixture("vitepress.html");
        let registry = AdapterRegistry::new();
        let adapter = registry.detect(&doc, &dummy_url());
        assert_eq!(adapter.name(), "vitepress");
    }

    #[test]
    fn detect_gitbook() {
        let doc = load_fixture("gitbook.html");
        let registry = AdapterRegistry::new();
        let adapter = registry.detect(&doc, &dummy_url());
        assert_eq!(adapter.name(), "gitbook");
    }

    #[test]
    fn detect_readthedocs() {
        let doc = load_fixture("readthedocs.html");
        let registry = AdapterRegistry::new();
        let adapter = registry.detect(&doc, &dummy_url());
        assert_eq!(adapter.name(), "readthedocs");
    }

    #[test]
    fn detect_generic_fallback() {
        let doc = load_fixture("generic.html");
        let registry = AdapterRegistry::new();
        let adapter = registry.detect(&doc, &dummy_url());
        assert_eq!(adapter.name(), "generic");
    }

    // -----------------------------------------------------------------------
    // Content extraction tests
    // -----------------------------------------------------------------------

    #[test]
    fn docusaurus_extracts_content() {
        let doc = load_fixture("docusaurus.html");
        let adapter = DocusaurusAdapter;
        let content = adapter.extract_content(&doc);

        assert_eq!(content.meta.title, Some("Installation".into()));
        // Should contain the main content
        assert!(content.html.contains("Prerequisites"));
        assert!(content.html.contains("npm install example-tool"));
        // Should NOT contain navbar or footer chrome
        assert!(!content.html.contains("Copyright"));
    }

    #[test]
    fn vitepress_extracts_content() {
        let doc = load_fixture("vitepress.html");
        let adapter = VitePressAdapter;
        let content = adapter.extract_content(&doc);

        assert_eq!(content.meta.title, Some("Getting Started".into()));
        assert!(content.html.contains("bun add -d vitepress"));
        assert!(content.html.contains("File Structure"));
    }

    #[test]
    fn gitbook_extracts_content() {
        let doc = load_fixture("gitbook.html");
        let adapter = GitBookAdapter;
        let content = adapter.extract_content(&doc);

        assert_eq!(content.meta.title, Some("Quick Start".into()));
        assert!(content.html.contains("Create Your First Space"));
    }

    #[test]
    fn readthedocs_extracts_content() {
        let doc = load_fixture("readthedocs.html");
        let adapter = ReadTheDocsAdapter;
        let content = adapter.extract_content(&doc);

        assert_eq!(content.meta.title, Some("API Reference".into()));
        assert!(content.html.contains("Client Class"));
        assert!(content.html.contains("from project import Client"));
    }

    #[test]
    fn generic_extracts_content() {
        let doc = load_fixture("generic.html");
        let adapter = GenericAdapter;
        let content = adapter.extract_content(&doc);

        assert_eq!(content.meta.title, Some("About Our Company".into()));
        assert!(content.html.contains("Our Mission"));
        // Should strip nav/header/footer/script
        assert!(!content.html.contains("analytics"));
    }

    // -----------------------------------------------------------------------
    // TOC extraction tests
    // -----------------------------------------------------------------------

    #[test]
    fn docusaurus_extracts_toc() {
        let doc = load_fixture("docusaurus.html");
        let adapter = DocusaurusAdapter;
        let toc = adapter.extract_toc(&doc);

        assert!(!toc.is_empty());
        // Should find sidebar links
        let titles: Vec<&str> = toc.iter().map(|e| e.title.as_str()).collect();
        assert!(titles.contains(&"Getting Started"));
        assert!(titles.contains(&"Installation"));
    }

    #[test]
    fn vitepress_extracts_toc() {
        let doc = load_fixture("vitepress.html");
        let adapter = VitePressAdapter;
        let toc = adapter.extract_toc(&doc);

        assert!(!toc.is_empty());
        let titles: Vec<&str> = toc.iter().map(|e| e.title.as_str()).collect();
        assert!(titles.contains(&"Getting Started"));
    }

    #[test]
    fn generic_extracts_toc_from_headings() {
        let doc = load_fixture("generic.html");
        let adapter = GenericAdapter;
        let toc = adapter.extract_toc(&doc);

        assert!(!toc.is_empty());
        let titles: Vec<&str> = toc.iter().map(|e| e.title.as_str()).collect();
        assert!(titles.contains(&"About Our Company"));
        assert!(titles.contains(&"Our Mission"));
        assert!(titles.contains(&"History"));
    }
}
