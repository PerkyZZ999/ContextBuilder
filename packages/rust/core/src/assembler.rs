//! KB directory assembler.
//!
//! Takes crawled pages, converted markdown, TOC, and metadata,
//! then writes the final KB directory structure to disk.

use std::path::{Path, PathBuf};

use chrono::Utc;
use sha2::{Digest, Sha256};
use tracing::{debug, info, instrument};

use contextbuilder_shared::{
    ContextBuilderError, KbId, KbManifest, Result, Toc, TocEntry, CURRENT_SCHEMA_VERSION,
};

/// Output from a successful KB assembly.
#[derive(Debug, Clone)]
pub struct AssembleResult {
    /// Absolute path to the assembled KB directory.
    pub kb_path: PathBuf,
    /// Number of pages written.
    pub page_count: usize,
    /// The KB manifest that was written.
    pub manifest: KbManifest,
}

/// A page ready for assembly (markdown content + metadata).
#[derive(Debug, Clone)]
pub struct AssemblePage {
    /// Stable path within `docs/` (e.g., `getting-started/installation`).
    pub path: String,
    /// The converted Markdown content (with frontmatter).
    pub markdown: String,
    /// Page title.
    pub title: String,
}

/// Configuration for KB assembly.
#[derive(Debug, Clone)]
pub struct AssembleConfig {
    /// Knowledge base ID.
    pub kb_id: KbId,
    /// Human-readable name.
    pub name: String,
    /// Original documentation URL.
    pub source_url: String,
    /// Root directory for KB output (e.g., `var/kb/`).
    pub output_root: PathBuf,
    /// Tool version string.
    pub tool_version: String,
}

/// Assemble a complete KB directory structure.
///
/// Creates the following layout:
/// ```text
/// <output_root>/<kb_id>/
/// ├── manifest.json
/// ├── toc.json
/// ├── docs/
/// │   ├── index.md
/// │   ├── getting-started/
/// │   │   └── installation.md
/// │   └── ...
/// ├── artifacts/       (empty, populated in Phase 3)
/// └── indexes/         (for DB file)
/// ```
#[instrument(skip_all, fields(kb_id = %config.kb_id, name = %config.name, pages = pages.len()))]
pub fn assemble(
    config: &AssembleConfig,
    pages: &[AssemblePage],
    toc: &Toc,
) -> Result<AssembleResult> {
    let kb_dir = config.output_root.join(config.kb_id.to_string());

    info!(path = %kb_dir.display(), "assembling KB directory");

    // Create directory structure
    create_dirs(&kb_dir)?;

    // Write manifest.json
    let manifest = build_manifest(config, pages.len());
    write_json(&kb_dir.join("manifest.json"), &manifest)?;

    // Write toc.json
    write_json(&kb_dir.join("toc.json"), toc)?;

    // Write docs/**/*.md
    let docs_dir = kb_dir.join("docs");
    for page in pages {
        write_page(&docs_dir, page)?;
    }

    info!(
        page_count = pages.len(),
        path = %kb_dir.display(),
        "KB assembly complete"
    );

    Ok(AssembleResult {
        kb_path: kb_dir,
        page_count: pages.len(),
        manifest,
    })
}

/// Verify that a KB directory is well-formed.
pub fn validate_kb(kb_path: &Path) -> Result<()> {
    // Check required files exist
    let manifest_path = kb_path.join("manifest.json");
    let toc_path = kb_path.join("toc.json");
    let docs_dir = kb_path.join("docs");

    if !manifest_path.exists() {
        return Err(ContextBuilderError::validation("missing manifest.json"));
    }
    if !toc_path.exists() {
        return Err(ContextBuilderError::validation("missing toc.json"));
    }
    if !docs_dir.exists() {
        return Err(ContextBuilderError::validation("missing docs/ directory"));
    }

    // Validate manifest
    let manifest_content = std::fs::read_to_string(&manifest_path)
        .map_err(|e| ContextBuilderError::io(&manifest_path, e))?;
    let manifest: KbManifest = serde_json::from_str(&manifest_content).map_err(|e| {
        ContextBuilderError::validation(format!("invalid manifest.json: {e}"))
    })?;

    if manifest.schema_version != CURRENT_SCHEMA_VERSION {
        return Err(ContextBuilderError::validation(format!(
            "unsupported schema_version: {} (expected {})",
            manifest.schema_version, CURRENT_SCHEMA_VERSION
        )));
    }

    // Validate TOC
    let toc_content = std::fs::read_to_string(&toc_path)
        .map_err(|e| ContextBuilderError::io(&toc_path, e))?;
    let toc: Toc = serde_json::from_str(&toc_content).map_err(|e| {
        ContextBuilderError::validation(format!("invalid toc.json: {e}"))
    })?;

    // Check that TOC paths have corresponding files
    validate_toc_paths(&docs_dir, &toc.sections)?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Artifact assembly
// ---------------------------------------------------------------------------

/// Metadata for a single artifact file.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ArtifactMeta {
    pub filename: String,
    pub sha256: String,
    pub size_bytes: usize,
}

/// Metadata about the enrichment run.
#[derive(Debug, Clone, serde::Serialize)]
pub struct EnrichmentMeta {
    pub model: String,
    pub total_tokens_in: u64,
    pub total_tokens_out: u64,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub completed_at: String,
}

/// Write artifact files to the KB artifacts directory and update the manifest.
///
/// Each entry in `artifacts` is a `(filename, content)` pair.
/// This function:
/// 1. Writes each artifact file atomically (write to temp, then rename)
/// 2. Updates `manifest.json` with artifact checksums and enrichment metadata
#[instrument(skip_all, fields(kb_path = %kb_path.display(), artifact_count = artifacts.len()))]
pub fn assemble_artifacts(
    kb_path: &Path,
    artifacts: &[(&str, &str)],
    enrichment_meta: &EnrichmentMeta,
) -> Result<Vec<ArtifactMeta>> {
    let artifacts_dir = kb_path.join("artifacts");
    std::fs::create_dir_all(&artifacts_dir)
        .map_err(|e| ContextBuilderError::io(&artifacts_dir, e))?;

    let mut metas = Vec::with_capacity(artifacts.len());

    for (filename, content) in artifacts {
        let target = artifacts_dir.join(filename);
        let temp = artifacts_dir.join(format!(".{filename}.tmp"));

        // Write to temp file first
        std::fs::write(&temp, content).map_err(|e| ContextBuilderError::io(&temp, e))?;

        // Atomic rename
        std::fs::rename(&temp, &target).map_err(|e| ContextBuilderError::io(&target, e))?;

        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let hash = format!("{:x}", hasher.finalize());

        debug!(file = %filename, size = content.len(), "wrote artifact");

        metas.push(ArtifactMeta {
            filename: (*filename).to_string(),
            sha256: hash,
            size_bytes: content.len(),
        });
    }

    // Update manifest
    update_manifest(kb_path, &metas, enrichment_meta)?;

    info!(
        count = metas.len(),
        "artifact assembly complete"
    );

    Ok(metas)
}

/// Update `manifest.json` with artifact and enrichment metadata.
fn update_manifest(
    kb_path: &Path,
    artifacts: &[ArtifactMeta],
    enrichment_meta: &EnrichmentMeta,
) -> Result<()> {
    let manifest_path = kb_path.join("manifest.json");

    let content = std::fs::read_to_string(&manifest_path)
        .map_err(|e| ContextBuilderError::io(&manifest_path, e))?;

    let mut manifest: KbManifest = serde_json::from_str(&content).map_err(|e| {
        ContextBuilderError::validation(format!("invalid manifest.json: {e}"))
    })?;

    manifest.artifacts = Some(serde_json::to_value(artifacts).unwrap_or_default());
    manifest.enrichment = Some(serde_json::to_value(enrichment_meta).unwrap_or_default());
    manifest.updated_at = Utc::now();

    write_json(&manifest_path, &manifest)?;
    debug!("manifest updated with artifact metadata");

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create the KB directory structure.
fn create_dirs(kb_dir: &Path) -> Result<()> {
    let dirs = [
        kb_dir.to_path_buf(),
        kb_dir.join("docs"),
        kb_dir.join("artifacts"),
        kb_dir.join("indexes"),
    ];

    for dir in &dirs {
        std::fs::create_dir_all(dir).map_err(|e| ContextBuilderError::io(dir, e))?;
    }

    debug!(path = %kb_dir.display(), "directory structure created");
    Ok(())
}

/// Build the KB manifest.
fn build_manifest(config: &AssembleConfig, page_count: usize) -> KbManifest {
    let now = Utc::now();
    KbManifest {
        schema_version: CURRENT_SCHEMA_VERSION,
        id: config.kb_id.clone(),
        name: config.name.clone(),
        source_url: config.source_url.clone(),
        tool_version: config.tool_version.clone(),
        created_at: now,
        updated_at: now,
        page_count,
        config: None,
        artifacts: None,
        enrichment: None,
    }
}

/// Write a JSON file (pretty-printed).
fn write_json<T: serde::Serialize>(path: &Path, data: &T) -> Result<()> {
    let json = serde_json::to_string_pretty(data).map_err(|e| {
        ContextBuilderError::validation(format!("JSON serialization failed: {e}"))
    })?;
    std::fs::write(path, json).map_err(|e| ContextBuilderError::io(path, e))?;
    debug!(path = %path.display(), "wrote JSON file");
    Ok(())
}

/// Write a single page's Markdown file to the docs directory.
fn write_page(docs_dir: &Path, page: &AssemblePage) -> Result<()> {
    let file_path = docs_dir.join(format!("{}.md", page.path));

    // Create parent directories if needed
    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| ContextBuilderError::io(parent, e))?;
    }

    std::fs::write(&file_path, &page.markdown)
        .map_err(|e| ContextBuilderError::io(&file_path, e))?;

    debug!(path = %file_path.display(), title = %page.title, "wrote page");
    Ok(())
}

/// Recursively check that TOC entry paths have corresponding .md files.
fn validate_toc_paths(docs_dir: &Path, entries: &[TocEntry]) -> Result<()> {
    for entry in entries {
        let file_path = docs_dir.join(format!("{}.md", entry.path));
        if !file_path.exists() {
            debug!(
                path = %entry.path,
                expected = %file_path.display(),
                "TOC entry missing corresponding file (non-fatal)"
            );
        }

        if !entry.children.is_empty() {
            validate_toc_paths(docs_dir, &entry.children)?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "cb-assembler-test-{}",
            uuid::Uuid::now_v7()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn make_config(output_root: &Path) -> AssembleConfig {
        AssembleConfig {
            kb_id: KbId::new(),
            name: "Test KB".into(),
            source_url: "https://docs.example.com".into(),
            output_root: output_root.into(),
            tool_version: "0.1.0-test".into(),
        }
    }

    fn make_pages() -> Vec<AssemblePage> {
        vec![
            AssemblePage {
                path: "index".into(),
                markdown: "---\ntitle: \"Home\"\n---\n\n# Home\n\nWelcome.\n".into(),
                title: "Home".into(),
            },
            AssemblePage {
                path: "getting-started".into(),
                markdown: "---\ntitle: \"Getting Started\"\n---\n\n# Getting Started\n\nBegin here.\n".into(),
                title: "Getting Started".into(),
            },
            AssemblePage {
                path: "guide/installation".into(),
                markdown: "---\ntitle: \"Installation\"\n---\n\n# Installation\n\nInstall it.\n".into(),
                title: "Installation".into(),
            },
        ]
    }

    fn make_toc() -> Toc {
        Toc {
            sections: vec![
                TocEntry {
                    title: "Home".into(),
                    path: "index".into(),
                    source_url: Some("https://docs.example.com/".into()),
                    summary: None,
                    children: vec![],
                },
                TocEntry {
                    title: "Getting Started".into(),
                    path: "getting-started".into(),
                    source_url: Some("https://docs.example.com/getting-started".into()),
                    summary: None,
                    children: vec![],
                },
                TocEntry {
                    title: "Guide".into(),
                    path: "guide".into(),
                    source_url: None,
                    summary: None,
                    children: vec![TocEntry {
                        title: "Installation".into(),
                        path: "guide/installation".into(),
                        source_url: Some("https://docs.example.com/guide/installation".into()),
                        summary: None,
                        children: vec![],
                    }],
                },
            ],
        }
    }

    #[test]
    fn assemble_creates_directory_structure() {
        let tmp = temp_dir();
        let config = make_config(&tmp);
        let pages = make_pages();
        let toc = make_toc();

        let result = assemble(&config, &pages, &toc).unwrap();

        // Check directory exists
        assert!(result.kb_path.exists());
        assert!(result.kb_path.join("docs").exists());
        assert!(result.kb_path.join("artifacts").exists());
        assert!(result.kb_path.join("indexes").exists());

        // Cleanup
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn assemble_writes_manifest() {
        let tmp = temp_dir();
        let config = make_config(&tmp);
        let pages = make_pages();
        let toc = make_toc();

        let result = assemble(&config, &pages, &toc).unwrap();

        let manifest_path = result.kb_path.join("manifest.json");
        assert!(manifest_path.exists());

        let manifest: KbManifest =
            serde_json::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();
        assert_eq!(manifest.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(manifest.name, "Test KB");
        assert_eq!(manifest.page_count, 3);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn assemble_writes_toc() {
        let tmp = temp_dir();
        let config = make_config(&tmp);
        let pages = make_pages();
        let toc = make_toc();

        let result = assemble(&config, &pages, &toc).unwrap();

        let toc_path = result.kb_path.join("toc.json");
        assert!(toc_path.exists());

        let read_toc: Toc =
            serde_json::from_str(&std::fs::read_to_string(&toc_path).unwrap()).unwrap();
        assert_eq!(read_toc.sections.len(), 3);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn assemble_writes_all_pages() {
        let tmp = temp_dir();
        let config = make_config(&tmp);
        let pages = make_pages();
        let toc = make_toc();

        let result = assemble(&config, &pages, &toc).unwrap();
        let docs_dir = result.kb_path.join("docs");

        assert!(docs_dir.join("index.md").exists());
        assert!(docs_dir.join("getting-started.md").exists());
        assert!(docs_dir.join("guide/installation.md").exists());

        // Verify content
        let content = std::fs::read_to_string(docs_dir.join("index.md")).unwrap();
        assert!(content.contains("# Home"));

        assert_eq!(result.page_count, 3);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn assemble_idempotent() {
        let tmp = temp_dir();
        let config = make_config(&tmp);
        let pages = make_pages();
        let toc = make_toc();

        // Assemble twice
        let _result1 = assemble(&config, &pages, &toc).unwrap();
        let result2 = assemble(&config, &pages, &toc).unwrap();

        // Second assembly should succeed (overwrites)
        assert!(result2.kb_path.exists());
        assert_eq!(result2.page_count, 3);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn validate_kb_valid() {
        let tmp = temp_dir();
        let config = make_config(&tmp);
        let pages = make_pages();
        let toc = make_toc();

        let result = assemble(&config, &pages, &toc).unwrap();
        assert!(validate_kb(&result.kb_path).is_ok());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn validate_kb_missing_manifest() {
        let tmp = temp_dir();
        std::fs::create_dir_all(tmp.join("docs")).unwrap();
        std::fs::write(tmp.join("toc.json"), "{}").unwrap();

        let err = validate_kb(&tmp).unwrap_err();
        assert!(err.to_string().contains("missing manifest.json"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    // Artifact assembly tests -------------------------------------------------

    #[test]
    fn assemble_artifacts_writes_files() {
        let tmp = temp_dir();
        let config = make_config(&tmp);
        let pages = make_pages();
        let toc = make_toc();

        let result = assemble(&config, &pages, &toc).unwrap();

        let enrichment_meta = EnrichmentMeta {
            model: "test-model".into(),
            total_tokens_in: 1000,
            total_tokens_out: 500,
            cache_hits: 2,
            cache_misses: 3,
            completed_at: "2025-01-01T00:00:00Z".into(),
        };

        let artifacts = vec![
            ("llms.txt", "# Test\n\n> Summary\n"),
            ("rules.md", "# Rules\n\nBe nice.\n"),
        ];

        let metas = assemble_artifacts(&result.kb_path, &artifacts, &enrichment_meta).unwrap();

        assert_eq!(metas.len(), 2);
        assert_eq!(metas[0].filename, "llms.txt");
        assert!(metas[0].sha256.len() == 64);
        assert!(metas[0].size_bytes > 0);

        // Verify files exist
        assert!(result.kb_path.join("artifacts/llms.txt").exists());
        assert!(result.kb_path.join("artifacts/rules.md").exists());

        // Verify content
        let content = std::fs::read_to_string(result.kb_path.join("artifacts/llms.txt")).unwrap();
        assert!(content.contains("# Test"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn assemble_artifacts_updates_manifest() {
        let tmp = temp_dir();
        let config = make_config(&tmp);
        let pages = make_pages();
        let toc = make_toc();

        let result = assemble(&config, &pages, &toc).unwrap();

        let enrichment_meta = EnrichmentMeta {
            model: "test-model".into(),
            total_tokens_in: 1000,
            total_tokens_out: 500,
            cache_hits: 2,
            cache_misses: 3,
            completed_at: "2025-01-01T00:00:00Z".into(),
        };

        let artifacts = vec![("llms.txt", "content")];
        assemble_artifacts(&result.kb_path, &artifacts, &enrichment_meta).unwrap();

        // Re-read manifest
        let manifest_json = std::fs::read_to_string(result.kb_path.join("manifest.json")).unwrap();
        let manifest: KbManifest = serde_json::from_str(&manifest_json).unwrap();

        // Artifacts should be set
        assert!(manifest.artifacts.is_some());
        let arr = manifest.artifacts.unwrap();
        assert!(arr.is_array());
        assert_eq!(arr.as_array().unwrap().len(), 1);

        // Enrichment should be set
        assert!(manifest.enrichment.is_some());
        let enrich = manifest.enrichment.unwrap();
        assert_eq!(enrich["model"], "test-model");
        assert_eq!(enrich["total_tokens_in"], 1000);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn assemble_artifacts_atomic_no_temp_files() {
        let tmp = temp_dir();
        let config = make_config(&tmp);
        let pages = make_pages();
        let toc = make_toc();

        let result = assemble(&config, &pages, &toc).unwrap();

        let enrichment_meta = EnrichmentMeta {
            model: "m".into(),
            total_tokens_in: 0,
            total_tokens_out: 0,
            cache_hits: 0,
            cache_misses: 0,
            completed_at: "now".into(),
        };

        let artifacts = vec![("test.md", "hello")];
        assemble_artifacts(&result.kb_path, &artifacts, &enrichment_meta).unwrap();

        // No temp files should remain
        let artifacts_dir = result.kb_path.join("artifacts");
        for entry in std::fs::read_dir(&artifacts_dir).unwrap() {
            let name = entry.unwrap().file_name().to_string_lossy().to_string();
            assert!(!name.starts_with('.'), "temp file left behind: {name}");
        }

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
