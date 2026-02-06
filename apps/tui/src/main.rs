//! ContextBuilder TUI — interactive terminal interface for KB management.
//!
//! Provides screens for creating, browsing, updating KBs and managing
//! the MCP server, built with `ratatui` + `crossterm`.

mod app;
mod screens;
mod widgets;

use color_eyre::eyre::Result;

fn main() -> Result<()> {
    color_eyre::install()?;
    app::run()
}
