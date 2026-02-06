//! VitePress platform adapter.

use super::{ExtractedContent, PageMeta, PlatformAdapter};
use super::docusaurus::extract_h1;
use contextbuilder_shared::TocEntry;
use scraper::{Html, Selector};
use url::Url;

/// Detects and extracts content from VitePress-powered documentation sites.
pub struct VitePressAdapter;

impl PlatformAdapter for VitePressAdapter {
    fn detect(&self, doc: &Html, _url: &Url) -> bool {
        // Check for #VPContent or .VPDoc class
        let vp_content = Selector::parse("#VPContent").unwrap();
        if doc.select(&vp_content).next().is_some() {
            return true;
        }

        let vp_doc = Selector::parse(".VPDoc").unwrap();
        if doc.select(&vp_doc).next().is_some() {
            return true;
        }

        false
    }

    fn extract_toc(&self, doc: &Html) -> Vec<TocEntry> {
        let mut entries = Vec::new();

        // Try .VPSidebar links
        let link_sel = Selector::parse(".VPSidebar a").unwrap();
        for el in doc.select(&link_sel) {
            let title = el.text().collect::<String>().trim().to_string();
            let path = el.value().attr("href").unwrap_or("").to_string();

            if !title.is_empty() && !path.is_empty() {
                entries.push(TocEntry {
                    title,
                    path: path
                        .trim_start_matches('/')
                        .trim_end_matches(".html")
                        .to_string(),
                    source_url: Some(path),
                    summary: None,
                    children: Vec::new(),
                });
            }
        }

        entries
    }

    fn extract_content(&self, doc: &Html) -> ExtractedContent {
        // VitePress uses .vp-doc for content
        let selectors = [".vp-doc", ".VPDoc", "#VPContent main", "main"];

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
        "vitepress"
    }
}
