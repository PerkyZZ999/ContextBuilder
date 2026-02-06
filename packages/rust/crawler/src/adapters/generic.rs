//! Generic (fallback) platform adapter.
//!
//! Always matches — used when no platform-specific adapter is detected.
//! Uses readability heuristics to find the main content area and
//! extracts TOC from the heading structure.

use super::{ExtractedContent, PageMeta, PlatformAdapter};
use super::docusaurus::extract_h1;
use contextbuilder_shared::TocEntry;
use scraper::{Html, Selector};
use url::Url;

/// Generic adapter that works on arbitrary HTML pages.
/// Always matches as the lowest-priority fallback.
pub struct GenericAdapter;

impl PlatformAdapter for GenericAdapter {
    fn detect(&self, _doc: &Html, _url: &Url) -> bool {
        // Generic adapter always matches
        true
    }

    fn extract_toc(&self, doc: &Html) -> Vec<TocEntry> {
        // Build TOC from heading structure (H1–H6)
        let mut entries = Vec::new();

        let heading_sel = Selector::parse("h1, h2, h3, h4, h5, h6").unwrap();
        for el in doc.select(&heading_sel) {
            let tag = el.value().name();
            let level: u8 = tag[1..].parse().unwrap_or(1);
            let title = el.text().collect::<String>().trim().to_string();

            if title.is_empty() {
                continue;
            }

            // Generate a slug from the title
            let slug = slugify(&title);

            // For H1, use as top-level; for H2+, add as flat entries
            // (hierarchical nesting is done in the TocBuilder later)
            if level <= 2 {
                entries.push(TocEntry {
                    title,
                    path: slug,
                    source_url: None,
                    summary: None,
                    children: Vec::new(),
                });
            }
        }

        entries
    }

    fn extract_content(&self, doc: &Html) -> ExtractedContent {
        // Readability heuristics: try <main>, <article>, then largest content block
        let selectors = ["main", "article", r#"[role="main"]"#, ".content"];

        for sel_str in selectors {
            let sel = Selector::parse(sel_str).unwrap();
            if let Some(el) = doc.select(&sel).next() {
                let html = el.inner_html();
                return ExtractedContent {
                    html: strip_chrome(&html),
                    meta: PageMeta {
                        title: extract_h1(doc),
                    },
                };
            }
        }

        // Last resort: use the body, stripping nav/header/footer/script/style/aside
        let body_sel = Selector::parse("body").unwrap();
        if let Some(body) = doc.select(&body_sel).next() {
            let html = body.inner_html();
            return ExtractedContent {
                html: strip_chrome(&html),
                meta: PageMeta {
                    title: extract_h1(doc),
                },
            };
        }

        ExtractedContent {
            html: String::new(),
            meta: PageMeta { title: None },
        }
    }

    fn name(&self) -> &str {
        "generic"
    }
}

/// Strip common navigation/chrome elements from HTML content.
fn strip_chrome(html: &str) -> String {
    let doc = Html::parse_fragment(html);
    let chrome_sel =
        Selector::parse("nav, header, footer, aside, script, style, .sidebar, .nav").unwrap();

    let mut result = html.to_string();
    for el in doc.select(&chrome_sel) {
        let outer = el.html();
        result = result.replace(&outer, "");
    }
    result
}

/// Generate a URL-safe slug from a title.
pub(crate) fn slugify(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
