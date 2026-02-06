//! "Browse KBs" screen — lists existing knowledge bases.

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

pub(crate) struct BrowseKbsScreen {
    /// Discovered KB entries (id, name, path).
    entries: Vec<(String, String, String)>,
    selected: usize,
    status: String,
}

impl BrowseKbsScreen {
    pub(crate) fn new() -> Self {
        Self {
            entries: Vec::new(),
            selected: 0,
            status: "Press 'r' to refresh the KB list.".to_string(),
        }
    }

    pub(crate) fn draw(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Min(1),   // List
                Constraint::Length(3), // Status
            ])
            .split(area);

        if self.entries.is_empty() {
            let empty = Paragraph::new(
                "No knowledge bases found.\n\nUse the 'Create KB' tab to add one, \
                 or press 'r' to scan var/kb/.",
            )
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Knowledge Bases "),
            );
            f.render_widget(empty, chunks[0]);
        } else {
            let items: Vec<ListItem> = self
                .entries
                .iter()
                .enumerate()
                .map(|(i, (id, name, path))| {
                    let style = if i == self.selected {
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    let prefix = if i == self.selected { "▸ " } else { "  " };
                    ListItem::new(format!(
                        "{prefix}{name}  ({id})  [{path}]"
                    ))
                    .style(style)
                })
                .collect();

            let list = List::new(items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" Knowledge Bases ({}) ", self.entries.len())),
            );
            f.render_widget(list, chunks[0]);
        }

        let status = Paragraph::new(self.status.as_str())
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(status, chunks[1]);
    }

    pub(crate) fn handle_key(&mut self, code: KeyCode, _modifiers: KeyModifiers) {
        match code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected + 1 < self.entries.len() {
                    self.selected += 1;
                }
            }
            KeyCode::Char('r') => {
                self.status = "Scanning for KBs...".to_string();
                // In a full implementation, this would spawn a task to scan
                // the KB root directory and populate self.entries.
                self.scan_kbs();
            }
            _ => {}
        }
    }

    fn scan_kbs(&mut self) {
        // Attempt to discover KBs from var/kb/ directory (synchronous scan).
        let kb_root = std::path::PathBuf::from("var/kb");
        if !kb_root.is_dir() {
            self.status = "No var/kb/ directory found.".to_string();
            return;
        }

        self.entries.clear();
        if let Ok(entries) = std::fs::read_dir(&kb_root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.join("manifest.json").exists() {
                    if let Ok(data) = std::fs::read_to_string(path.join("manifest.json")) {
                        if let Ok(manifest) =
                            serde_json::from_str::<serde_json::Value>(&data)
                        {
                            let id = manifest["id"]
                                .as_str()
                                .unwrap_or("?")
                                .to_string();
                            let name = manifest["name"]
                                .as_str()
                                .unwrap_or("unnamed")
                                .to_string();
                            self.entries.push((
                                id,
                                name,
                                path.to_string_lossy().to_string(),
                            ));
                        }
                    }
                }
            }
        }

        self.selected = 0;
        self.status = format!("Found {} knowledge base(s).", self.entries.len());
    }
}
