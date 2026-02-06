//! Post-conversion cleanup pipeline for Markdown output.
//!
//! Each cleanup pass is a function `&str -> String` applied in sequence.
//! The pipeline normalizes headings, whitespace, code blocks, and links.

use std::sync::LazyLock;

use regex::Regex;
use url::Url;

/// Run the full cleanup pipeline on raw Markdown text.
pub(crate) fn run_pipeline(md: &str, base_url: Option<&Url>) -> String {
    let mut result = md.to_string();

    result = normalize_headings(&result);
    result = clean_blank_lines(&result);
    result = fix_code_block_languages(&result);
    result = strip_leftover_html(&result);
    result = resolve_links(&result, base_url);
    result = normalize_whitespace(&result);
    result = ensure_trailing_newline(&result);

    result
}

// ---------------------------------------------------------------------------
// Pass 1: Normalize heading levels
// ---------------------------------------------------------------------------

/// Ensure there's at most one H1, and heading hierarchy is proper.
fn normalize_headings(md: &str) -> String {
    static H_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?m)^(#{1,6})\s+(.+)$").expect("valid regex")
    });

    let mut h1_count = 0;
    let mut lines: Vec<String> = Vec::new();

    for line in md.lines() {
        if let Some(caps) = H_RE.captures(line) {
            let hashes = &caps[1];
            let text = &caps[2];

            if hashes == "#" {
                h1_count += 1;
                if h1_count > 1 {
                    // Demote duplicate H1s to H2
                    lines.push(format!("## {text}"));
                    continue;
                }
            }
            lines.push(line.to_string());
        } else {
            lines.push(line.to_string());
        }
    }

    lines.join("\n")
}

// ---------------------------------------------------------------------------
// Pass 2: Clean up excessive blank lines
// ---------------------------------------------------------------------------

/// Collapse runs of 3+ blank lines into exactly 2.
fn clean_blank_lines(md: &str) -> String {
    static MULTI_BLANK_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\n{4,}").expect("valid regex")
    });

    MULTI_BLANK_RE.replace_all(md, "\n\n\n").to_string()
}

// ---------------------------------------------------------------------------
// Pass 3: Fix code block language hints
// ---------------------------------------------------------------------------

/// Detect and fix code block language hints from class names.
///
/// Handles patterns like `language-js`, `lang-python`, `highlight-rust`.
fn fix_code_block_languages(md: &str) -> String {
    static LANG_PREFIX_RE: LazyLock<Regex> = LazyLock::new(|| {
        // Matches code fence with a class-like language prefix
        Regex::new(r"(?m)^```(?:language-|lang-|highlight-)(\w+)").expect("valid regex")
    });

    LANG_PREFIX_RE.replace_all(md, "```$1").to_string()
}

// ---------------------------------------------------------------------------
// Pass 4: Strip leftover HTML tags and attributes
// ---------------------------------------------------------------------------

/// Remove stray HTML tags that survived the conversion.
///
/// We keep `<br>` since some Markdown renderers support it,
/// and we preserve content inside tags (just remove the tags themselves).
fn strip_leftover_html(md: &str) -> String {
    // Don't strip inside code blocks
    let mut result = String::new();
    let mut in_code_block = false;

    for line in md.lines() {
        if line.trim_start().starts_with("```") {
            in_code_block = !in_code_block;
            result.push_str(line);
            result.push('\n');
            continue;
        }

        if in_code_block {
            result.push_str(line);
            result.push('\n');
            continue;
        }

        // Strip HTML tags outside code blocks (preserve content)
        let cleaned = strip_html_tags(line);
        result.push_str(&cleaned);
        result.push('\n');
    }

    // Remove trailing newline added by our loop (will be handled by trailing newline pass)
    if result.ends_with('\n') {
        result.pop();
    }

    result
}

/// Strip HTML tags from a single line, preserving inner text.
fn strip_html_tags(line: &str) -> String {
    static HTML_TAG_RE: LazyLock<Regex> = LazyLock::new(|| {
        // Match opening/closing/self-closing HTML tags
        // Exclude markdown-compatible tags or common ones we want to keep
        Regex::new(r"</?(?:div|span|section|article|aside|header|footer|figure|figcaption|details|summary)(?:\s[^>]*)?>").expect("valid regex")
    });

    HTML_TAG_RE.replace_all(line, "").to_string()
}

// ---------------------------------------------------------------------------
// Pass 5: Resolve relative links
// ---------------------------------------------------------------------------

/// Resolve relative URLs in Markdown links against a base URL.
fn resolve_links(md: &str, base_url: Option<&Url>) -> String {
    let Some(base) = base_url else {
        return md.to_string();
    };

    static LINK_RE: LazyLock<Regex> = LazyLock::new(|| {
        // Match [text](url) — we'll filter out image links (![...]) in the replacement
        Regex::new(r"\[([^\]]*)\]\(([^)]+)\)").expect("valid regex")
    });

    LINK_RE
        .replace_all(md, |caps: &regex::Captures| {
            let full_match = caps.get(0).unwrap();
            let text = &caps[1];
            let href = &caps[2];

            // Check if this is an image link by looking at the char before the match
            let start = full_match.start();
            if start > 0 && md.as_bytes()[start - 1] == b'!' {
                // This is an image ![alt](url), leave as-is
                return caps[0].to_string();
            }

            // Skip absolute URLs and anchors
            if href.starts_with("http://")
                || href.starts_with("https://")
                || href.starts_with('#')
                || href.starts_with("mailto:")
            {
                return format!("[{text}]({href})");
            }

            // Resolve relative URL
            match base.join(href) {
                Ok(resolved) => format!("[{text}]({})", resolved),
                Err(_) => format!("[{text}]({href})"),
            }
        })
        .to_string()
}

// ---------------------------------------------------------------------------
// Pass 6: Normalize whitespace
// ---------------------------------------------------------------------------

/// Clean up trailing whitespace on lines and normalize line endings.
fn normalize_whitespace(md: &str) -> String {
    md.lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
}

// ---------------------------------------------------------------------------
// Pass 7: Ensure trailing newline
// ---------------------------------------------------------------------------

/// Ensure the file ends with exactly one newline.
fn ensure_trailing_newline(md: &str) -> String {
    let trimmed = md.trim_end_matches('\n');
    format!("{trimmed}\n")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_headings_demotes_duplicate_h1() {
        let input = "# Title\n\nSome text\n\n# Another Title\n\nMore text";
        let result = normalize_headings(input);
        assert_eq!(
            result,
            "# Title\n\nSome text\n\n## Another Title\n\nMore text"
        );
    }

    #[test]
    fn normalize_headings_keeps_single_h1() {
        let input = "# Only One\n\n## Sub\n\n### Deep";
        let result = normalize_headings(input);
        assert_eq!(result, input);
    }

    #[test]
    fn clean_blank_lines_collapses_excess() {
        let input = "Line 1\n\n\n\n\nLine 2";
        let result = clean_blank_lines(input);
        assert_eq!(result, "Line 1\n\n\nLine 2");
    }

    #[test]
    fn clean_blank_lines_keeps_double() {
        let input = "Line 1\n\nLine 2";
        let result = clean_blank_lines(input);
        assert_eq!(result, input);
    }

    #[test]
    fn fix_code_block_languages_strips_prefix() {
        let input = "```language-javascript\nconsole.log('hi');\n```";
        let result = fix_code_block_languages(input);
        assert!(result.starts_with("```javascript"));
    }

    #[test]
    fn fix_code_block_languages_keeps_plain() {
        let input = "```rust\nfn main() {}\n```";
        let result = fix_code_block_languages(input);
        assert_eq!(result, input);
    }

    #[test]
    fn strip_leftover_html_removes_div_tags() {
        let input = "# Title\n\n<div class=\"note\">Important info</div>\n\nMore text";
        let result = strip_leftover_html(input);
        assert!(result.contains("Important info"));
        assert!(!result.contains("<div"));
        assert!(!result.contains("</div>"));
    }

    #[test]
    fn strip_leftover_html_preserves_code_blocks() {
        let input = "# Title\n\n```html\n<div>Preserved</div>\n```\n\nText";
        let result = strip_leftover_html(input);
        assert!(result.contains("<div>Preserved</div>"));
    }

    #[test]
    fn resolve_links_absolute_untouched() {
        let base = Url::parse("https://docs.example.com/guide/").unwrap();
        let input = "[Link](https://other.com/page)";
        let result = resolve_links(input, Some(&base));
        assert_eq!(result, "[Link](https://other.com/page)");
    }

    #[test]
    fn resolve_links_relative_resolved() {
        let base = Url::parse("https://docs.example.com/guide/intro").unwrap();
        let input = "[Next](/api/reference)";
        let result = resolve_links(input, Some(&base));
        assert_eq!(result, "[Next](https://docs.example.com/api/reference)");
    }

    #[test]
    fn resolve_links_anchor_untouched() {
        let base = Url::parse("https://docs.example.com/page").unwrap();
        let input = "[Section](#section-1)";
        let result = resolve_links(input, Some(&base));
        assert_eq!(result, "[Section](#section-1)");
    }

    #[test]
    fn normalize_whitespace_trims_trailing() {
        let input = "Line 1   \nLine 2\t\nLine 3";
        let result = normalize_whitespace(input);
        assert_eq!(result, "Line 1\nLine 2\nLine 3");
    }

    #[test]
    fn ensure_trailing_newline_adds_if_missing() {
        let input = "Content";
        let result = ensure_trailing_newline(input);
        assert_eq!(result, "Content\n");
    }

    #[test]
    fn ensure_trailing_newline_normalizes_multiple() {
        let input = "Content\n\n\n";
        let result = ensure_trailing_newline(input);
        assert_eq!(result, "Content\n");
    }

    #[test]
    fn full_pipeline_cleans_markdown() {
        let input = "# Title\n\n\n\n\n\n## Section\n\n<div>Some content</div>\n\n```language-python\nprint('hi')\n```\n\nEnd";
        let base = Url::parse("https://example.com/page").unwrap();
        let result = run_pipeline(input, Some(&base));

        // Excessive blank lines collapsed
        assert!(!result.contains("\n\n\n\n"));
        // Language prefix stripped
        assert!(result.contains("```python"));
        // HTML tags stripped
        assert!(!result.contains("<div>"));
        // Content preserved
        assert!(result.contains("Some content"));
        // Ends with newline
        assert!(result.ends_with('\n'));
    }
}
