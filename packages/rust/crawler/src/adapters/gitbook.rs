//! GitBook platform adapter.

use super::{ExtractedContent, PageMeta, PlatformAdapter};
use super::docusaurus::extract_h1;
use contextbuilder_shared::TocEntry;
use scraper::{Html, Selector};
use url::Url;

/// Detects and extracts content from GitBook-powered documentation sites.
pub struct GitBookAdapter;

impl PlatformAdapter for GitBookAdapter {
    fn detect(&self, doc: &Html, _url: &Url) -> bool {
        // Check for <meta name="gitbook" ...>
        let meta_sel = Selector::parse(r#"meta[name="gitbook"]"#).unwrap();
        if doc.select(&meta_sel).next().is_some() {
            return true;
        }

        // Check for .gitbook-root class
        let root_sel = Selector::parse(".gitbook-root").unwrap();
        if doc.select(&root_sel).next().is_some() {
            return true;
        }

        false
    }

    fn extract_toc(&self, doc: &Html) -> Vec<TocEntry> {
        let mut entries = Vec::new();

        // GitBook sidebar links
        let link_sel = Selector::parse("aside nav a, .sidebar nav a").unwrap();
        for el in doc.select(&link_sel) {
            let title = el.text().collect::<String>().trim().to_string();
            let path = el.value().attr("href").unwrap_or("").to_string();

            if !title.is_empty() && !path.is_empty() {
                entries.push(TocEntry {
                    title,
                    path: path.trim_start_matches('/').to_string(),
                    source_url: Some(path),
                    summary: None,
                    children: Vec::new(),
                });
            }
        }

        entries
    }

    fn extract_content(&self, doc: &Html) -> ExtractedContent {
        let selectors = [
            ".markdown-section",
            ".page-inner section",
            "main section",
            "main",
        ];

        for sel_str in selectors {
            let sel = Selector::parse(sel_str).unwrap();
            if let Some(el) = doc.select(&sel).next() {
                return ExtractedContent {
                    html: el.inner_html(),
                    meta: PageMeta {
                        title: extract_h1(doc),
                    },
                };
            }
        }

        ExtractedContent {
            html: String::new(),
            meta: PageMeta { title: None },
        }
    }

    fn name(&self) -> &str {
        "gitbook"
    }
}
