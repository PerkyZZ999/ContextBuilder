//! ContextBuilder CLI — local-first documentation ingestion tool.
//!
//! Converts documentation URLs into AI-ready artifacts and a portable
//! knowledge base with LLM enrichment.

mod commands;

use clap::Parser;
use color_eyre::eyre::Result;

use commands::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();
    commands::init_tracing(&cli);
    commands::run(cli).await
}