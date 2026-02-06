//! Core pipeline orchestration and domain logic for ContextBuilder.
//!
//! This crate ties together discovery, crawling, markdown conversion, and
//! KB assembly into end-to-end workflows (e.g., `add_kb`).

pub mod assembler;
pub mod enrichment;
pub mod pipeline;
pub mod toc;
pub mod update;
