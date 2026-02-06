//! Docusaurus platform adapter.

use super::{ExtractedContent, PageMeta, PlatformAdapter};
use contextbuilder_shared::TocEntry;
use scraper::{Html, Selector};
use url::Url;

/// Detects and extracts content from Docusaurus-powered documentation sites.
pub struct DocusaurusAdapter;

impl PlatformAdapter for DocusaurusAdapter {
    fn detect(&self, doc: &Html, _url: &Url) -> bool {
        // Check for <meta name="generator" content="Docusaurus ...">
        let meta_sel = Selector::parse(r#"meta[name="generator"]"#).unwrap();
        for el in doc.select(&meta_sel) {
            if let Some(content) = el.value().attr("content") {
                if content.to_lowercase().contains("docusaurus") {
                    return true;
                }
            }
        }

        // Check for data-docusaurus attributes
        let html_sel = Selector::parse("[data-docusaurus-version]").unwrap();
        if doc.select(&html_sel).next().is_some() {
            return true;
        }

        false
    }

    fn extract_toc(&self, doc: &Html) -> Vec<TocEntry> {
        let mut entries = Vec::new();

        // Try sidebar with .menu__list structure
        let link_sel = Selector::parse(".menu__list .menu__link").unwrap();
        for el in doc.select(&link_sel) {
            let title = el.text().collect::<String>().trim().to_string();
            let path = el.value().attr("href").unwrap_or("").to_string();

            if !title.is_empty() && !path.is_empty() {
                entries.push(TocEntry {
                    title,
                    path: normalize_doc_path(&path),
                    source_url: Some(path),
                    summary: None,
                    children: Vec::new(),
                });
            }
        }

        entries
    }

    fn extract_content(&self, doc: &Html) -> ExtractedContent {
        // Try <article> first, then .markdown container
        let selectors = ["article .markdown", "article", ".markdown", "main"];

        for sel_str in selectors {
            let sel = Selector::parse(sel_str).unwrap();
            if let Some(el) = doc.select(&sel).next() {
                let html = el.inner_html();
                let title = extract_h1(doc);

                return ExtractedContent {
                    html: strip_edit_links(&html),
                    meta: PageMeta { title },
                };
            }
        }

        // Fallback to body
        let body_sel = Selector::parse("body").unwrap();
        let html = doc
            .select(&body_sel)
            .next()
            .map(|el| el.inner_html())
            .unwrap_or_default();

        ExtractedContent {
            html,
            meta: PageMeta {
                title: extract_h1(doc),
            },
        }
    }

    fn name(&self) -> &str {
        "docusaurus"
    }
}

/// Strip edit-this-page links and Docusaurus footer elements from HTML.
fn strip_edit_links(html: &str) -> String {
    // Simple approach: remove common Docusaurus footer patterns
    let doc = Html::parse_fragment(html);
    let footer_sel = Selector::parse(".theme-doc-footer, .pagination-nav").unwrap();

    let mut result = html.to_string();
    for el in doc.select(&footer_sel) {
        let outer = el.html();
        result = result.replace(&outer, "");
    }
    result
}

/// Extract the H1 title from any document.
pub(crate) fn extract_h1(doc: &Html) -> Option<String> {
    let h1_sel = Selector::parse("h1").unwrap();
    doc.select(&h1_sel)
        .next()
        .map(|el| el.text().collect::<String>().trim().to_string())
}

/// Normalize a doc path (strip leading /docs/ prefix, strip .html suffix).
pub(crate) fn normalize_doc_path(path: &str) -> String {
    let p = path
        .trim_start_matches('/')
        .trim_start_matches("docs/")
        .trim_end_matches(".html")
        .trim_end_matches('/');
    p.to_string()
}
