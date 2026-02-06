//! TUI screen definitions.
//!
//! Each screen corresponds to a tab in the TUI and encapsulates its
//! own state and rendering logic.

mod create_kb;
mod browse_kbs;
mod update_kb;
mod outputs;
mod mcp_server;

use std::fmt;

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::prelude::*;

/// Screen identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScreenId {
    CreateKb,
    BrowseKbs,
    UpdateKb,
    Outputs,
    McpServer,
}

impl fmt::Display for ScreenId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CreateKb => write!(f, "Create KB"),
            Self::BrowseKbs => write!(f, "Browse KBs"),
            Self::UpdateKb => write!(f, "Update KB"),
            Self::Outputs => write!(f, "Outputs"),
            Self::McpServer => write!(f, "MCP Server"),
        }
    }
}

/// Per-screen state and behaviour.
pub(crate) struct Screen {
    pub id: ScreenId,
    pub create: create_kb::CreateKbScreen,
    pub browse: browse_kbs::BrowseKbsScreen,
    pub update: update_kb::UpdateKbScreen,
    pub outputs: outputs::OutputsScreen,
    pub mcp: mcp_server::McpServerScreen,
}

impl Screen {
    pub(crate) fn new(id: ScreenId) -> Self {
        Self {
            id,
            create: create_kb::CreateKbScreen::new(),
            browse: browse_kbs::BrowseKbsScreen::new(),
            update: update_kb::UpdateKbScreen::new(),
            outputs: outputs::OutputsScreen::new(),
            mcp: mcp_server::McpServerScreen::new(),
        }
    }

    /// Whether the current screen has an active text input field.
    pub(crate) fn is_editing(&self) -> bool {
        match self.id {
            ScreenId::CreateKb => self.create.is_editing(),
            _ => false,
        }
    }

    pub(crate) fn draw(&self, f: &mut Frame, area: Rect) {
        match self.id {
            ScreenId::CreateKb => self.create.draw(f, area),
            ScreenId::BrowseKbs => self.browse.draw(f, area),
            ScreenId::UpdateKb => self.update.draw(f, area),
            ScreenId::Outputs => self.outputs.draw(f, area),
            ScreenId::McpServer => self.mcp.draw(f, area),
        }
    }

    pub(crate) fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        match self.id {
            ScreenId::CreateKb => self.create.handle_key(code, modifiers),
            ScreenId::BrowseKbs => self.browse.handle_key(code, modifiers),
            ScreenId::UpdateKb => self.update.handle_key(code, modifiers),
            ScreenId::Outputs => self.outputs.handle_key(code, modifiers),
            ScreenId::McpServer => self.mcp.handle_key(code, modifiers),
        }
    }
}
