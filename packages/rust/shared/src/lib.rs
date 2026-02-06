//! Shared types, error model, and configuration for ContextBuilder.
//!
//! This crate is the foundation depended on by all other ContextBuilder crates.
//! It provides:
//! - [`ContextBuilderError`] — the unified error type
//! - Domain types ([`KbManifest`], [`TocEntry`], [`PageMeta`], [`KbId`])
//! - Configuration ([`AppConfig`], [`CrawlConfig`], config loading)

pub mod config;
pub mod error;
pub mod types;

// Re-export public API at crate root for ergonomic imports.
pub use config::{
    AppConfig, CrawlConfig, CrawlPoliciesConfig, DefaultsConfig, KbRegistryEntry,
    OpenRouterConfig, config_dir, config_file_path, init_config, load_config, load_config_from,
    validate_api_key,
};
pub use error::{ContextBuilderError, Result};
pub use types::{CURRENT_SCHEMA_VERSION, KbId, KbManifest, PageMeta, Toc, TocEntry};
