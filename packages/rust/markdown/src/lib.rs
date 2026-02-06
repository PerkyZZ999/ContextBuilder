//! HTML-to-Markdown conversion and cleanup passes.
//!
//! Converts raw HTML pages to clean Markdown using the `htmd` crate, then applies
//! a series of cleanup passes to normalize headings, whitespace, code blocks, and links.

mod cleanup;

use std::sync::LazyLock;

use regex::Regex;
use scraper::Html;
use tracing::{debug, instrument};
use url::Url;

use contextbuilder_shared::{ContextBuilderError, Result};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Result of converting an HTML page to Markdown.
#[derive(Debug, Clone)]
pub struct ConvertResult {
    /// The final Markdown content (with frontmatter).
    pub markdown: String,
    /// Extracted or inferred page title.
    pub title: String,
    /// Approximate word count of the Markdown body (excluding frontmatter).
    pub word_count: usize,
}

/// Options for the HTML-to-Markdown conversion.
#[derive(Debug, Clone)]
pub struct ConvertOptions {
    /// Source URL used for resolving relative links and frontmatter.
    pub source_url: String,
    /// Override title (if `None`, extracted from first H1).
    pub title: Option<String>,
    /// ISO 8601 timestamp for the `fetched_at` frontmatter field.
    pub fetched_at: Option<String>,
}

// ---------------------------------------------------------------------------
// Converter
// ---------------------------------------------------------------------------

/// Convert HTML to clean Markdown with frontmatter.
///
/// This is the main entry point. It:
/// 1. Extracts the content HTML (via adapter or from raw `<main>`/`<body>`)
/// 2. Pre-processes HTML tables into markdown tables
/// 3. Converts HTML → Markdown via `htmd`
/// 4. Runs the cleanup pipeline
/// 5. Prepends YAML frontmatter
#[instrument(skip(html), fields(url = %opts.source_url))]
pub fn convert(html: &str, opts: &ConvertOptions) -> Result<ConvertResult> {
    // Step 1: Extract content HTML (strip nav/header/footer/aside/script/style)
    let content_html = extract_content_html(html);

    // Step 2: Pre-process tables into markdown
    let content_html = preprocess_tables(&content_html);

    // Step 3: Convert HTML → Markdown using htmd
    let converter = htmd::HtmlToMarkdown::builder()
        .skip_tags(vec!["script", "style", "nav", "iframe", "noscript", "svg"])
        .build();

    let raw_markdown = converter
        .convert(&content_html)
        .map_err(|e| ContextBuilderError::Conversion(format!("htmd conversion failed: {e}")))?;

    debug!(raw_len = raw_markdown.len(), "htmd conversion complete");

    // Step 3: Run cleanup pipeline
    let base_url = Url::parse(&opts.source_url).ok();
    let cleaned = cleanup::run_pipeline(&raw_markdown, base_url.as_ref());

    // Step 4: Extract title
    let title = opts
        .title
        .clone()
        .or_else(|| extract_title_from_markdown(&cleaned))
        .unwrap_or_else(|| "Untitled".to_string());

    // Step 5: Count words (body only)
    let word_count = count_words(&cleaned);

    // Step 6: Build frontmatter
    let frontmatter = build_frontmatter(&opts.source_url, &title, opts.fetched_at.as_deref());
    let markdown = format!("{frontmatter}\n{cleaned}");

    debug!(
        title = %title,
        word_count,
        final_len = markdown.len(),
        "conversion complete"
    );

    Ok(ConvertResult {
        markdown,
        title,
        word_count,
    })
}

/// Convert pre-extracted content HTML (from a platform adapter) to Markdown.
///
/// Use this when you've already extracted the content via a platform adapter
/// and just need the HTML → Markdown + cleanup step.
#[instrument(skip(content_html), fields(url = %opts.source_url))]
pub fn convert_extracted(content_html: &str, opts: &ConvertOptions) -> Result<ConvertResult> {
    let content_html = preprocess_tables(content_html);

    let converter = htmd::HtmlToMarkdown::builder()
        .skip_tags(vec!["script", "style", "nav", "iframe", "noscript", "svg"])
        .build();

    let raw_markdown = converter.convert(&content_html).map_err(|e| {
        ContextBuilderError::Conversion(format!("htmd conversion failed: {e}"))
    })?;

    let base_url = Url::parse(&opts.source_url).ok();
    let cleaned = cleanup::run_pipeline(&raw_markdown, base_url.as_ref());

    let title = opts
        .title
        .clone()
        .or_else(|| extract_title_from_markdown(&cleaned))
        .unwrap_or_else(|| "Untitled".to_string());

    let word_count = count_words(&cleaned);
    let frontmatter = build_frontmatter(&opts.source_url, &title, opts.fetched_at.as_deref());
    let markdown = format!("{frontmatter}\n{cleaned}");

    Ok(ConvertResult {
        markdown,
        title,
        word_count,
    })
}

// ---------------------------------------------------------------------------
// Table pre-processing
// ---------------------------------------------------------------------------

/// Convert HTML `<table>` elements to markdown table syntax before htmd conversion.
///
/// `htmd` 0.1 doesn't support table conversion, so we handle it manually.
fn preprocess_tables(html: &str) -> String {
    let doc = Html::parse_fragment(html);

    let table_sel = scraper::Selector::parse("table").unwrap();

    if doc.select(&table_sel).next().is_none() {
        return html.to_string();
    }

    let mut result = html.to_string();

    // Process each table: convert to markdown, then replace the HTML
    for table_el in doc.select(&table_sel) {
        let table_html = element_outer_html(&table_el);
        let md_table = html_table_to_markdown(&table_el);
        // Replace the HTML table with a placeholder that htmd will pass through
        // We wrap in a <pre> so htmd doesn't mangle it, then unwrap in cleanup
        result = result.replacen(&table_html, &md_table, 1);
    }

    result
}

/// Convert a single HTML table element to a markdown table string.
fn html_table_to_markdown(table: &scraper::ElementRef) -> String {
    let tr_sel = scraper::Selector::parse("tr").unwrap();
    let th_sel = scraper::Selector::parse("th").unwrap();
    let td_sel = scraper::Selector::parse("td").unwrap();

    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut has_header = false;

    for tr in table.select(&tr_sel) {
        let ths: Vec<String> = tr
            .select(&th_sel)
            .map(|cell| cell.text().collect::<String>().trim().to_string())
            .collect();

        if !ths.is_empty() {
            has_header = true;
            rows.push(ths);
            continue;
        }

        let tds: Vec<String> = tr
            .select(&td_sel)
            .map(|cell| cell.text().collect::<String>().trim().to_string())
            .collect();

        if !tds.is_empty() {
            rows.push(tds);
        }
    }

    if rows.is_empty() {
        return String::new();
    }

    // Determine column count from the widest row
    let col_count = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    if col_count == 0 {
        return String::new();
    }

    // Normalize all rows to have the same number of columns
    for row in &mut rows {
        while row.len() < col_count {
            row.push(String::new());
        }
    }

    let mut md = String::from("\n\n");

    // Header row
    let header = &rows[0];
    md.push_str("| ");
    md.push_str(&header.join(" | "));
    md.push_str(" |\n");

    // Separator row
    md.push_str("| ");
    md.push_str(
        &(0..col_count)
            .map(|_| "---")
            .collect::<Vec<_>>()
            .join(" | "),
    );
    md.push_str(" |\n");

    // Data rows (skip the header if it existed)
    let data_start = if has_header { 1 } else { 0 };
    for row in &rows[data_start..] {
        md.push_str("| ");
        md.push_str(&row.join(" | "));
        md.push_str(" |\n");
    }

    md.push('\n');
    md
}

/// Reconstruct the outer HTML of an element (approximate, for matching).
fn element_outer_html(el: &scraper::ElementRef) -> String {
    el.html()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract the main content HTML, stripping chrome (nav, header, footer, etc.).
fn extract_content_html(html: &str) -> String {
    let doc = Html::parse_document(html);

    // Try known content containers in priority order
    let selectors = [
        "article .markdown",     // Docusaurus
        ".vp-doc",               // VitePress
        ".markdown-section",     // GitBook
        "[role=\"main\"]",       // ReadTheDocs / generic
        "article",               // Common
        "main",                  // HTML5 semantic
        ".content",              // Generic
    ];

    for sel_str in &selectors {
        if let Ok(selector) = scraper::Selector::parse(sel_str) {
            if let Some(el) = doc.select(&selector).next() {
                return el.inner_html();
            }
        }
    }

    // Fallback: use <body> content
    if let Ok(body_sel) = scraper::Selector::parse("body") {
        if let Some(body) = doc.select(&body_sel).next() {
            return body.inner_html();
        }
    }

    // Last resort
    html.to_string()
}

/// Extract title from the first H1 in the Markdown text.
fn extract_title_from_markdown(md: &str) -> Option<String> {
    static H1_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?m)^# (.+)$").expect("valid regex")
    });

    H1_RE
        .captures(md)
        .map(|c| c[1].trim().to_string())
}

/// Count words in Markdown body (excluding code blocks and frontmatter).
fn count_words(md: &str) -> usize {
    static CODE_BLOCK_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?s)```.*?```").expect("valid regex")
    });

    let without_code = CODE_BLOCK_RE.replace_all(md, "");
    without_code
        .split_whitespace()
        .filter(|w| !w.starts_with('#') || w.len() > 2)
        .count()
}

/// Build a YAML frontmatter block.
fn build_frontmatter(source_url: &str, title: &str, fetched_at: Option<&str>) -> String {
    let mut fm = String::from("---\n");
    fm.push_str(&format!("source_url: \"{source_url}\"\n"));
    fm.push_str(&format!("title: \"{}\"\n", escape_yaml_string(title)));
    if let Some(ts) = fetched_at {
        fm.push_str(&format!("fetched_at: \"{ts}\"\n"));
    }
    fm.push_str("---\n");
    fm
}

/// Escape special characters in a YAML string value.
fn escape_yaml_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fixture_path(name: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures")
            .join(name)
    }

    fn load_fixture(name: &str) -> String {
        fs::read_to_string(fixture_path(name))
            .unwrap_or_else(|e| panic!("failed to read fixture {name}: {e}"))
    }

    fn make_opts(url: &str) -> ConvertOptions {
        ConvertOptions {
            source_url: url.to_string(),
            title: None,
            fetched_at: None,
        }
    }

    // --- Core conversion tests ---

    #[test]
    fn convert_simple_html() {
        let html = "<html><body><main><h1>Hello World</h1><p>Some text.</p></main></body></html>";
        let result = convert(html, &make_opts("https://example.com/page")).unwrap();

        assert!(result.markdown.contains("# Hello World"));
        assert!(result.markdown.contains("Some text."));
        assert_eq!(result.title, "Hello World");
        assert!(result.word_count > 0);
    }

    #[test]
    fn convert_includes_frontmatter() {
        let html = "<html><body><main><h1>Test</h1><p>Body</p></main></body></html>";
        let result = convert(
            html,
            &ConvertOptions {
                source_url: "https://example.com/test".into(),
                title: None,
                fetched_at: Some("2024-01-15T10:30:00Z".into()),
            },
        )
        .unwrap();

        assert!(result.markdown.starts_with("---\n"));
        assert!(result.markdown.contains("source_url: \"https://example.com/test\""));
        assert!(result.markdown.contains("title: \"Test\""));
        assert!(result.markdown.contains("fetched_at: \"2024-01-15T10:30:00Z\""));
    }

    #[test]
    fn convert_strips_nav_and_footer() {
        let html = r#"<html><body>
            <nav><a href="/">Home</a></nav>
            <main><h1>Content</h1><p>Important text.</p></main>
            <footer><p>Copyright 2024</p></footer>
        </body></html>"#;

        let result = convert(html, &make_opts("https://example.com/")).unwrap();
        assert!(result.markdown.contains("Important text."));
        assert!(!result.markdown.contains("Copyright 2024"));
    }

    #[test]
    fn convert_preserves_code_blocks() {
        let html = r#"<html><body><main>
            <h1>Code Example</h1>
            <pre><code class="language-rust">fn main() {
    println!("hello");
}</code></pre>
        </main></body></html>"#;

        let result = convert(html, &make_opts("https://example.com/code")).unwrap();
        assert!(result.markdown.contains("```rust"));
        assert!(result.markdown.contains("println!"));
    }

    #[test]
    fn convert_preserves_tables() {
        let html = r#"<html><body><main>
            <h1>Data</h1>
            <table>
                <thead><tr><th>Name</th><th>Value</th></tr></thead>
                <tbody>
                    <tr><td>foo</td><td>bar</td></tr>
                    <tr><td>baz</td><td>qux</td></tr>
                </tbody>
            </table>
        </main></body></html>"#;

        let result = convert(html, &make_opts("https://example.com/data")).unwrap();
        assert!(result.markdown.contains("| Name | Value |"));
        assert!(result.markdown.contains("| foo | bar |"));
    }

    #[test]
    fn convert_handles_lists() {
        let html = r#"<html><body><main>
            <h1>Lists</h1>
            <ul>
                <li>Item one</li>
                <li>Item two</li>
                <li>Item three</li>
            </ul>
            <ol>
                <li>First</li>
                <li>Second</li>
            </ol>
        </main></body></html>"#;

        let result = convert(html, &make_opts("https://example.com/lists")).unwrap();
        assert!(result.markdown.contains("Item one"));
        assert!(result.markdown.contains("First"));
    }

    #[test]
    fn convert_no_html_tags_in_output() {
        let html = r#"<html><body><main>
            <h1>Clean Output</h1>
            <p>This should be <strong>clean</strong> markdown.</p>
            <div class="note"><p>A note.</p></div>
        </main></body></html>"#;

        let result = convert(html, &make_opts("https://example.com/clean")).unwrap();
        // Should not contain HTML tags (except possibly inside code blocks)
        let body = result.markdown.split("---").nth(2).unwrap_or(&result.markdown);
        assert!(!body.contains("<p>"), "output contains <p> tags");
        assert!(!body.contains("<h1>"), "output contains <h1> tags");
    }

    #[test]
    fn convert_with_title_override() {
        let html = "<html><body><main><h1>Original</h1><p>Text</p></main></body></html>";
        let result = convert(
            html,
            &ConvertOptions {
                source_url: "https://example.com/".into(),
                title: Some("Custom Title".into()),
                fetched_at: None,
            },
        )
        .unwrap();

        assert_eq!(result.title, "Custom Title");
        assert!(result.markdown.contains("title: \"Custom Title\""));
    }

    // --- Fixture-based tests ---

    #[test]
    fn convert_docusaurus_fixture() {
        let html = load_fixture("html/docusaurus.html");
        let result = convert(&html, &make_opts("https://example.com/docs/installation")).unwrap();

        assert_eq!(result.title, "Installation");
        assert!(result.markdown.contains("Prerequisites"));
        assert!(result.markdown.contains("npm install example-tool"));
        // Should have code blocks
        assert!(result.markdown.contains("```"));
        // Should have table content
        assert!(result.markdown.contains("verbose"));
        // Should not have edit link footer
        assert!(!result.markdown.contains("Edit this page"));
    }

    #[test]
    fn convert_vitepress_fixture() {
        let html = load_fixture("html/vitepress.html");
        let result = convert(&html, &make_opts("https://example.com/guide/getting-started")).unwrap();

        assert_eq!(result.title, "Getting Started");
        assert!(result.markdown.contains("Installation"));
        assert!(result.markdown.contains("bun add -d vitepress"));
        assert!(result.markdown.contains("```"));
    }

    #[test]
    fn convert_gitbook_fixture() {
        let html = load_fixture("html/gitbook.html");
        let result = convert(&html, &make_opts("https://example.com/quick-start")).unwrap();

        assert_eq!(result.title, "Quick Start");
        assert!(result.markdown.contains("Create Your First Space"));
        assert!(result.markdown.contains("Write Your Content"));
    }

    #[test]
    fn convert_readthedocs_fixture() {
        let html = load_fixture("html/readthedocs.html");
        let result = convert(&html, &make_opts("https://example.com/api")).unwrap();

        assert_eq!(result.title, "API Reference");
        assert!(result.markdown.contains("Client Class"));
        assert!(result.markdown.contains("client.query"));
    }

    #[test]
    fn convert_generic_fixture() {
        let html = load_fixture("html/generic.html");
        let result = convert(&html, &make_opts("https://example.com/about")).unwrap();

        assert_eq!(result.title, "About Our Company");
        assert!(result.markdown.contains("Our Mission"));
        assert!(result.markdown.contains("Simplicity"));
    }

    // --- Edge cases ---

    #[test]
    fn convert_empty_html() {
        let html = "<html><body></body></html>";
        let result = convert(html, &make_opts("https://example.com/empty")).unwrap();
        assert_eq!(result.title, "Untitled");
    }

    #[test]
    fn convert_no_main_element() {
        let html = "<html><body><h1>Direct Body</h1><p>Content in body.</p></body></html>";
        let result = convert(html, &make_opts("https://example.com/plain")).unwrap();
        assert!(result.markdown.contains("Direct Body"));
        assert!(result.markdown.contains("Content in body."));
    }

    #[test]
    fn convert_deeply_nested() {
        let html = r#"<html><body><main>
            <div><div><div>
                <h1>Deep</h1>
                <p>Nested content.</p>
            </div></div></div>
        </main></body></html>"#;

        let result = convert(html, &make_opts("https://example.com/deep")).unwrap();
        assert!(result.markdown.contains("# Deep"));
        assert!(result.markdown.contains("Nested content."));
    }

    #[test]
    fn word_count_excludes_code_blocks() {
        let html = r#"<html><body><main>
            <h1>Title</h1>
            <p>One two three.</p>
            <pre><code>lots of code words that should not be counted</code></pre>
        </main></body></html>"#;

        let result = convert(html, &make_opts("https://example.com/wc")).unwrap();
        // Word count should be small (just "One two three." + "Title")
        assert!(result.word_count < 10, "word_count={} should exclude code", result.word_count);
    }
}
