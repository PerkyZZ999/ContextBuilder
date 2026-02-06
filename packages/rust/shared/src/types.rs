//! Core domain types for ContextBuilder knowledge bases.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Current schema version for the KB manifest format.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

// ---------------------------------------------------------------------------
// KbId
// ---------------------------------------------------------------------------

/// A UUID v7 wrapper for knowledge base identifiers (time-sortable).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct KbId(pub Uuid);

impl KbId {
    /// Generate a new time-sortable KB identifier.
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for KbId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for KbId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for KbId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

// ---------------------------------------------------------------------------
// KbManifest
// ---------------------------------------------------------------------------

/// The `manifest.json` structure stored at the root of each KB directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KbManifest {
    /// Schema version for forward compatibility.
    pub schema_version: u32,
    /// Unique identifier for this KB.
    pub id: KbId,
    /// Human-readable name.
    pub name: String,
    /// The original documentation URL.
    pub source_url: String,
    /// Tool version that created this KB.
    pub tool_version: String,
    /// When the KB was first created.
    pub created_at: DateTime<Utc>,
    /// When the KB was last updated.
    pub updated_at: DateTime<Utc>,
    /// Total number of pages in the KB.
    pub page_count: usize,
    /// Build/crawl configuration used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,
    /// Artifact metadata (populated after enrichment).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifacts: Option<serde_json::Value>,
    /// Enrichment metadata (model, tokens, timestamp).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enrichment: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// TocEntry
// ---------------------------------------------------------------------------

/// A single entry in the table of contents (`toc.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocEntry {
    /// Display title.
    pub title: String,
    /// Stable local path within `docs/` (e.g., `getting-started/installation`).
    pub path: String,
    /// Original source URL for traceability.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    /// LLM-generated or extracted summary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Nested child entries (for hierarchical TOCs).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<TocEntry>,
}

/// Root structure for `toc.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Toc {
    /// Top-level sections.
    pub sections: Vec<TocEntry>,
}

// ---------------------------------------------------------------------------
// PageMeta
// ---------------------------------------------------------------------------

/// Metadata for a single ingested page, stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageMeta {
    /// Unique page identifier (UUID v7).
    pub id: String,
    /// Owning knowledge base.
    pub kb_id: String,
    /// Original page URL.
    pub url: String,
    /// Stable local path within the KB directory.
    pub path: String,
    /// Page title (extracted or generated).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// SHA-256 hash of the Markdown content.
    pub content_hash: String,
    /// When the page was last fetched.
    pub fetched_at: DateTime<Utc>,
    /// HTTP status code from fetch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
    /// Content length in bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_len: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kb_id_roundtrip() {
        let id = KbId::new();
        let s = id.to_string();
        let parsed: KbId = s.parse().expect("parse KbId");
        assert_eq!(id, parsed);
    }

    #[test]
    fn manifest_serialization() {
        let manifest = KbManifest {
            schema_version: CURRENT_SCHEMA_VERSION,
            id: KbId::new(),
            name: "test-kb".into(),
            source_url: "https://example.com/docs".into(),
            tool_version: "0.1.0".into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            page_count: 0,
            config: None,
            artifacts: None,
            enrichment: None,
        };

        let json = serde_json::to_string_pretty(&manifest).expect("serialize");
        let parsed: KbManifest = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(parsed.name, "test-kb");
    }

    #[test]
    fn toc_entry_serialization() {
        let toc = Toc {
            sections: vec![TocEntry {
                title: "Getting Started".into(),
                path: "getting-started".into(),
                source_url: Some("https://example.com/docs/getting-started".into()),
                summary: None,
                children: vec![TocEntry {
                    title: "Installation".into(),
                    path: "getting-started/installation".into(),
                    source_url: None,
                    summary: Some("How to install the tool".into()),
                    children: vec![],
                }],
            }],
        };

        let json = serde_json::to_string(&toc).expect("serialize");
        let parsed: Toc = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.sections.len(), 1);
        assert_eq!(parsed.sections[0].children.len(), 1);
    }

    #[test]
    fn manifest_fixture_validates() {
        let fixture =
            std::fs::read_to_string("../../../fixtures/json/manifest.fixture.json")
                .expect("read fixture");
        let parsed: KbManifest =
            serde_json::from_str(&fixture).expect("deserialize fixture manifest");
        assert_eq!(parsed.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(parsed.name, "example-docs");
        assert_eq!(parsed.page_count, 3);
    }

    #[test]
    fn toc_fixture_validates() {
        let fixture =
            std::fs::read_to_string("../../../fixtures/json/toc.fixture.json")
                .expect("read fixture");
        let parsed: Toc = serde_json::from_str(&fixture).expect("deserialize fixture toc");
        assert_eq!(parsed.sections.len(), 2);
        assert_eq!(parsed.sections[0].children.len(), 2);
        assert_eq!(parsed.sections[0].title, "Getting Started");
    }
}
