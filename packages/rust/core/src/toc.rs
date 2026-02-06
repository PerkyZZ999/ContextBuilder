//! TOC (Table of Contents) builder.
//!
//! Merges crawler-extracted page metadata with adapter-detected navigation
//! into a hierarchical `Toc` structure that maps to `toc.json`.

use std::collections::HashMap;

use tracing::{debug, instrument};

use contextbuilder_shared::{PageMeta, Toc, TocEntry};

/// Build a TOC from crawled pages and optional adapter-extracted navigation.
///
/// The builder:
/// 1. Creates entries from page metadata (path, title, source URL)
/// 2. Merges adapter TOC info if available
/// 3. Builds a hierarchical structure from path segments
/// 4. Orders entries alphabetically (with index pages first)
#[instrument(skip_all, fields(page_count = pages.len()))]
pub fn build_toc(pages: &[PageMeta], adapter_toc: &[TocEntry]) -> Toc {
    if !adapter_toc.is_empty() && adapter_toc.len() >= pages.len() / 2 {
        // Use adapter TOC as the primary structure when it covers most pages
        debug!(
            adapter_entries = adapter_toc.len(),
            "using adapter-provided TOC structure"
        );
        return Toc {
            sections: adapter_toc.to_vec(),
        };
    }

    // Build from page paths
    let mut root_entries: Vec<TocEntry> = Vec::new();
    let mut section_map: HashMap<String, Vec<TocEntry>> = HashMap::new();

    for page in pages {
        let entry = TocEntry {
            title: page
                .title
                .clone()
                .unwrap_or_else(|| title_from_path(&page.path)),
            path: page.path.clone(),
            source_url: Some(page.url.clone()),
            summary: None,
            children: vec![],
        };

        // Determine parent section from path segments
        if let Some(parent) = parent_path(&page.path) {
            section_map.entry(parent).or_default().push(entry);
        } else {
            root_entries.push(entry);
        }
    }

    // Merge section children into root entries or create section entries
    let mut sections = build_hierarchy(root_entries, &mut section_map);

    // Sort sections: index first, then alphabetically
    sort_entries(&mut sections);

    debug!(sections = sections.len(), "TOC built from page paths");

    Toc { sections }
}

/// Generate a slug-safe path from a URL path.
pub fn slugify_path(url_path: &str) -> String {
    let cleaned = url_path
        .trim_start_matches('/')
        .trim_end_matches('/')
        .trim_end_matches(".html")
        .trim_end_matches(".htm")
        .trim_end_matches(".md");

    if cleaned.is_empty() {
        return "index".to_string();
    }

    // Convert to kebab-case (lowercase, dashes instead of spaces/underscores)
    cleaned
        .split('/')
        .map(|segment| {
            segment
                .to_lowercase()
                .replace(' ', "-")
                .replace('_', "-")
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '/')
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("/")
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract a human-readable title from a path slug.
fn title_from_path(path: &str) -> String {
    let segment = path.rsplit('/').next().unwrap_or(path);

    if segment == "index" {
        return "Overview".to_string();
    }

    segment
        .replace('-', " ")
        .replace('_', " ")
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(c) => {
                    let upper: String = c.to_uppercase().collect();
                    format!("{upper}{}", chars.collect::<String>())
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Get the parent path (all but the last segment).
fn parent_path(path: &str) -> Option<String> {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() <= 1 {
        return None;
    }
    Some(parts[..parts.len() - 1].join("/"))
}

/// Build a hierarchical TOC from flat entries and a section map.
fn build_hierarchy(
    mut root_entries: Vec<TocEntry>,
    section_map: &mut HashMap<String, Vec<TocEntry>>,
) -> Vec<TocEntry> {
    // Check if any root entry matches a section key
    for entry in &mut root_entries {
        if let Some(mut children) = section_map.remove(&entry.path) {
            sort_entries(&mut children);
            entry.children = children;
        }
    }

    // Any remaining sections without a root entry become new section entries
    let mut remaining: Vec<(String, Vec<TocEntry>)> = section_map.drain().collect();
    remaining.sort_by(|a, b| a.0.cmp(&b.0));

    for (section_path, mut children) in remaining {
        sort_entries(&mut children);
        root_entries.push(TocEntry {
            title: title_from_path(&section_path),
            path: section_path,
            source_url: None,
            summary: None,
            children,
        });
    }

    root_entries
}

/// Sort entries: "index" first, then alphabetically by title.
fn sort_entries(entries: &mut [TocEntry]) {
    entries.sort_by(|a, b| {
        let a_is_index = a.path.ends_with("index") || a.path == "index";
        let b_is_index = b.path.ends_with("index") || b.path == "index";

        match (a_is_index, b_is_index) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
        }
    });

    // Recursively sort children
    for entry in entries.iter_mut() {
        if !entry.children.is_empty() {
            sort_entries(&mut entry.children);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_page(path: &str, title: &str, url: &str) -> PageMeta {
        PageMeta {
            id: uuid::Uuid::now_v7().to_string(),
            kb_id: "test-kb".into(),
            url: url.into(),
            path: path.into(),
            title: Some(title.into()),
            content_hash: "abc123".into(),
            fetched_at: Utc::now(),
            status_code: Some(200),
            content_len: Some(1000),
        }
    }

    #[test]
    fn build_toc_from_flat_pages() {
        let pages = vec![
            make_page("index", "Home", "https://docs.example.com/"),
            make_page("getting-started", "Getting Started", "https://docs.example.com/getting-started"),
            make_page("api-reference", "API Reference", "https://docs.example.com/api-reference"),
        ];

        let toc = build_toc(&pages, &[]);
        assert_eq!(toc.sections.len(), 3);
        // Index should be first
        assert_eq!(toc.sections[0].path, "index");
    }

    #[test]
    fn build_toc_hierarchical() {
        let pages = vec![
            make_page("guide", "Guide", "https://docs.example.com/guide"),
            make_page("guide/installation", "Installation", "https://docs.example.com/guide/installation"),
            make_page("guide/quick-start", "Quick Start", "https://docs.example.com/guide/quick-start"),
            make_page("api", "API", "https://docs.example.com/api"),
        ];

        let toc = build_toc(&pages, &[]);
        assert_eq!(toc.sections.len(), 2); // guide (with children) + api

        let guide = toc.sections.iter().find(|s| s.path == "guide").unwrap();
        assert_eq!(guide.children.len(), 2);
    }

    #[test]
    fn build_toc_uses_adapter_when_sufficient() {
        let pages = vec![
            make_page("a", "A", "https://example.com/a"),
            make_page("b", "B", "https://example.com/b"),
        ];

        let adapter_toc = vec![
            TocEntry {
                title: "Alpha".into(),
                path: "a".into(),
                source_url: None,
                summary: None,
                children: vec![],
            },
            TocEntry {
                title: "Beta".into(),
                path: "b".into(),
                source_url: None,
                summary: None,
                children: vec![],
            },
        ];

        let toc = build_toc(&pages, &adapter_toc);
        assert_eq!(toc.sections[0].title, "Alpha");
    }

    #[test]
    fn slugify_path_handles_common_patterns() {
        assert_eq!(slugify_path("/guide/getting-started.html"), "guide/getting-started");
        assert_eq!(slugify_path("/"), "index");
        assert_eq!(slugify_path("/docs/API_Reference.html"), "docs/api-reference");
        assert_eq!(slugify_path("/path/with spaces/page"), "path/with-spaces/page");
    }

    #[test]
    fn title_from_path_converts_slugs() {
        assert_eq!(title_from_path("getting-started"), "Getting Started");
        assert_eq!(title_from_path("api_reference"), "Api Reference");
        assert_eq!(title_from_path("index"), "Overview");
        assert_eq!(title_from_path("guide/installation"), "Installation");
    }

    #[test]
    fn sort_entries_index_first() {
        let mut entries = vec![
            TocEntry {
                title: "Zebra".into(),
                path: "zebra".into(),
                source_url: None,
                summary: None,
                children: vec![],
            },
            TocEntry {
                title: "Overview".into(),
                path: "index".into(),
                source_url: None,
                summary: None,
                children: vec![],
            },
            TocEntry {
                title: "Alpha".into(),
                path: "alpha".into(),
                source_url: None,
                summary: None,
                children: vec![],
            },
        ];

        sort_entries(&mut entries);
        assert_eq!(entries[0].path, "index");
        assert_eq!(entries[1].path, "alpha");
        assert_eq!(entries[2].path, "zebra");
    }
}
