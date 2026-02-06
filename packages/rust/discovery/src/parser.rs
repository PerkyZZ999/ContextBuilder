//! llms.txt format parser.
//!
//! Parses the llms.txt format as specified by <https://llmstxt.org/>:
//! - Line 1: `# Title`
//! - Optional: `> Summary blockquote`
//! - Sections: `## Section Name` followed by Markdown link lists
//! - Links: `- [Link Name](url): Optional description`

use contextbuilder_shared::{ContextBuilderError, Result};
use regex::Regex;
use std::sync::LazyLock;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Parsed representation of an llms.txt file.
#[derive(Debug, Clone)]
pub struct LlmsParsed {
    /// The H1 title.
    pub title: String,
    /// The blockquote summary (if present).
    pub summary: Option<String>,
    /// Named sections containing entries.
    pub sections: Vec<LlmsSection>,
    /// All entries across all sections (flat list for convenience).
    pub entries: Vec<LlmsEntry>,
}

/// A named section within the llms.txt (## heading).
#[derive(Debug, Clone)]
pub struct LlmsSection {
    /// Section title (from ## heading).
    pub title: String,
    /// Entries within this section.
    pub entries: Vec<LlmsEntry>,
}

/// A single linked entry in the llms.txt.
#[derive(Debug, Clone)]
pub struct LlmsEntry {
    /// Display name of the link.
    pub name: String,
    /// Target URL.
    pub url: String,
    /// Optional description/notes after the `:`.
    pub notes: Option<String>,
}

// ---------------------------------------------------------------------------
// Regex patterns (compiled once)
// ---------------------------------------------------------------------------

/// Matches `# Title` at the start of a line.
static H1_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^#\s+(.+)$").expect("H1 regex")
});

/// Matches `## Section Title`.
static H2_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^##\s+(.+)$").expect("H2 regex")
});

/// Matches `> Blockquote text`.
static BLOCKQUOTE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^>\s*(.+)$").expect("blockquote regex")
});

/// Matches `- [Name](url)` or `- [Name](url): Notes`.
static LINK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[-*]\s+\[([^\]]+)\]\(([^)]+)\)(?::\s*(.+))?$").expect("link regex")
});

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

/// Parse an llms.txt string into structured data.
pub(crate) fn parse_llms_txt(content: &str) -> Result<LlmsParsed> {
    let mut lines = content.lines().peekable();

    // --- Extract H1 title ---
    let title = loop {
        match lines.next() {
            Some(line) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if let Some(caps) = H1_RE.captures(trimmed) {
                    break caps[1].trim().to_string();
                }
                return Err(ContextBuilderError::parse(
                    "llms.txt must start with an H1 heading (# Title)",
                ));
            }
            None => {
                return Err(ContextBuilderError::parse("llms.txt is empty"));
            }
        }
    };

    // --- Extract optional blockquote summary ---
    let mut summary_parts: Vec<String> = Vec::new();
    let mut sections: Vec<LlmsSection> = Vec::new();
    let mut all_entries: Vec<LlmsEntry> = Vec::new();

    // Collect blockquote lines (may span multiple lines)
    while let Some(&line) = lines.peek() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            lines.next();
            continue;
        }
        if let Some(caps) = BLOCKQUOTE_RE.captures(trimmed) {
            summary_parts.push(caps[1].trim().to_string());
            lines.next();
        } else {
            break;
        }
    }

    let summary = if summary_parts.is_empty() {
        None
    } else {
        Some(summary_parts.join(" "))
    };

    // --- Parse sections and entries ---
    let mut current_section: Option<LlmsSection> = None;

    for line in lines {
        let trimmed = line.trim();

        // Skip blank lines
        if trimmed.is_empty() {
            continue;
        }

        // New section heading?
        if let Some(caps) = H2_RE.captures(trimmed) {
            // Save previous section
            if let Some(section) = current_section.take() {
                sections.push(section);
            }
            current_section = Some(LlmsSection {
                title: caps[1].trim().to_string(),
                entries: Vec::new(),
            });
            continue;
        }

        // Link entry?
        if let Some(caps) = LINK_RE.captures(trimmed) {
            let entry = LlmsEntry {
                name: caps[1].trim().to_string(),
                url: caps[2].trim().to_string(),
                notes: caps.get(3).map(|m| m.as_str().trim().to_string()),
            };
            all_entries.push(entry.clone());
            if let Some(ref mut section) = current_section {
                section.entries.push(entry);
            }
            continue;
        }

        // Other lines (descriptive text) — skip but don't error
    }

    // Save final section
    if let Some(section) = current_section.take() {
        sections.push(section);
    }

    Ok(LlmsParsed {
        title,
        summary,
        sections,
        entries: all_entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_fixture() {
        let content = std::fs::read_to_string("../../../fixtures/llms/valid-llms.txt")
            .expect("read fixture");
        let parsed = parse_llms_txt(&content).unwrap();

        assert_eq!(parsed.title, "Example Docs");
        assert_eq!(
            parsed.summary,
            Some("Example documentation for testing the ContextBuilder discovery module.".into())
        );
        assert_eq!(parsed.sections.len(), 3);
        assert_eq!(parsed.sections[0].title, "Getting Started");
        assert_eq!(parsed.sections[0].entries.len(), 2);
        assert_eq!(parsed.sections[1].title, "API Reference");
        assert_eq!(parsed.sections[1].entries.len(), 2);
        assert_eq!(parsed.sections[2].title, "Guides");
        assert_eq!(parsed.sections[2].entries.len(), 2);

        // Total entries
        assert_eq!(parsed.entries.len(), 6);

        // Check first entry
        let first = &parsed.entries[0];
        assert_eq!(first.name, "Installation");
        assert_eq!(first.url, "https://docs.example.com/getting-started/installation");
        assert_eq!(first.notes, Some("How to install".into()));
    }

    #[test]
    fn parse_minimal_fixture() {
        let content = std::fs::read_to_string("../../../fixtures/llms/minimal-llms.txt")
            .expect("read fixture");
        let parsed = parse_llms_txt(&content).unwrap();

        assert_eq!(parsed.title, "Minimal Docs");
        assert_eq!(
            parsed.summary,
            Some("A minimal llms.txt for testing.".into())
        );
        // No sections (entries without ## heading)
        assert_eq!(parsed.sections.len(), 0);
        // But we still have entries (they're outside sections)
        assert_eq!(parsed.entries.len(), 1);
    }

    #[test]
    fn parse_empty_fails() {
        let result = parse_llms_txt("");
        assert!(result.is_err());
    }

    #[test]
    fn parse_no_h1_fails() {
        let result = parse_llms_txt("This has no heading\nJust text.");
        assert!(result.is_err());
    }

    #[test]
    fn parse_entry_without_notes() {
        let content = "# Test\n\n## Section\n\n- [Link](https://example.com)\n";
        let parsed = parse_llms_txt(content).unwrap();
        assert_eq!(parsed.entries.len(), 1);
        assert_eq!(parsed.entries[0].name, "Link");
        assert_eq!(parsed.entries[0].url, "https://example.com");
        assert!(parsed.entries[0].notes.is_none());
    }

    #[test]
    fn parse_multiline_blockquote() {
        let content = "# Title\n\n> Line one\n> Line two\n\n## Sec\n- [A](https://a.com)\n";
        let parsed = parse_llms_txt(content).unwrap();
        assert_eq!(parsed.summary, Some("Line one Line two".into()));
    }
}
