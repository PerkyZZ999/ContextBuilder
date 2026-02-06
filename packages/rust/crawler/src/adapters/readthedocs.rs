//! Read the Docs platform adapter.

use super::{ExtractedContent, PageMeta, PlatformAdapter};
use super::docusaurus::extract_h1;
use contextbuilder_shared::TocEntry;
use scraper::{Html, Selector};
use url::Url;

/// Detects and extracts content from Read the Docs (Sphinx) documentation sites.
pub struct ReadTheDocsAdapter;

impl PlatformAdapter for ReadTheDocsAdapter {
    fn detect(&self, doc: &Html, _url: &Url) -> bool {
        // Check for readthedocs meta
        let meta_sel = Selector::parse(r#"meta[name="readthedocs"]"#).unwrap();
        if doc.select(&meta_sel).next().is_some() {
            return true;
        }

        // Check for .wy-nav-side or wy-body-for-nav
        let wy_sel = Selector::parse(".wy-nav-side").unwrap();
        if doc.select(&wy_sel).next().is_some() {
            return true;
        }

        let body_sel = Selector::parse(".wy-body-for-nav").unwrap();
        if doc.select(&body_sel).next().is_some() {
            return true;
        }

        // Check for _static/ asset paths (common Sphinx marker)
        let link_sel = Selector::parse(r#"link[href*="_static"]"#).unwrap();
        doc.select(&link_sel).next().is_some()
    }

    fn extract_toc(&self, doc: &Html) -> Vec<TocEntry> {
        let mut entries = Vec::new();

        // Try .wy-menu links, or .toctree links
        let selectors = [
            ".wy-menu a",
            ".toctree-wrapper a",
            "nav.wy-nav-side a",
        ];

        for sel_str in selectors {
            let sel = Selector::parse(sel_str).unwrap();
            for el in doc.select(&sel) {
                let title = el.text().collect::<String>().trim().to_string();
                let path = el.value().attr("href").unwrap_or("").to_string();

                if !title.is_empty() && !path.is_empty() && !path.starts_with('#') {
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
            if !entries.is_empty() {
                break; // Use the first selector that yields results
            }
        }

        entries
    }

    fn extract_content(&self, doc: &Html) -> ExtractedContent {
        // ReadTheDocs uses [role="main"] or .document
        let selectors = [r#"[role="main"]"#, ".document", ".rst-content .section", "main"];

        for sel_str in selectors {
            let sel = Selector::parse(sel_str).unwrap();
            if let Some(el) = doc.select(&sel).next() {
                let html = el.inner_html();
                return ExtractedContent {
                    html: strip_rtd_footer(&html),
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
        "readthedocs"
    }
}

/// Strip RTD-specific footer/navigation elements from extracted HTML.
fn strip_rtd_footer(html: &str) -> String {
    let doc = Html::parse_fragment(html);
    let footer_sel = Selector::parse(r#"footer, [role="navigation"]"#).unwrap();

    let mut result = html.to_string();
    for el in doc.select(&footer_sel) {
        let outer = el.html();
        result = result.replace(&outer, "");
    }
    result
}
