//! "Outputs" screen — view generated artifacts for a KB.

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

/// Valid artifact names.
const ARTIFACT_NAMES: &[&str] = &[
    "llms.txt",
    "llms-full.txt",
    "SKILL.md",
    "rules.md",
    "style.md",
    "do_dont.md",
];

pub(crate) struct OutputsScreen {
    selected: usize,
    content: String,
    kb_path: String,
}

impl OutputsScreen {
    pub(crate) fn new() -> Self {
        Self {
            selected: 0,
            content: "Select an artifact from the list to preview.".to_string(),
            kb_path: String::new(),
        }
    }

    pub(crate) fn draw(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .margin(1)
            .constraints([
                Constraint::Length(22), // Artifact list
                Constraint::Min(1),    // Preview
            ])
            .split(area);

        // Artifact list
        let items: Vec<ListItem> = ARTIFACT_NAMES
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let style = if i == self.selected {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                let prefix = if i == self.selected { "▸ " } else { "  " };
                ListItem::new(format!("{prefix}{name}")).style(style)
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Artifacts "),
        );
        f.render_widget(list, chunks[0]);

        // Preview panel
        let preview = Paragraph::new(self.content.as_str())
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(
                        " {} ",
                        ARTIFACT_NAMES[self.selected]
                    )),
            );
        f.render_widget(preview, chunks[1]);
    }

    pub(crate) fn handle_key(&mut self, code: KeyCode, _modifiers: KeyModifiers) {
        match code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected > 0 {
                    self.selected -= 1;
                    self.load_artifact();
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected + 1 < ARTIFACT_NAMES.len() {
                    self.selected += 1;
                    self.load_artifact();
                }
            }
            KeyCode::Enter => {
                self.load_artifact();
            }
            _ => {}
        }
    }

    fn load_artifact(&mut self) {
        if self.kb_path.is_empty() {
            self.content =
                "No KB loaded. Use 'Browse KBs' to select one first.".to_string();
            return;
        }

        let artifact = ARTIFACT_NAMES[self.selected];
        let path = std::path::PathBuf::from(&self.kb_path)
            .join("artifacts")
            .join(artifact);

        match std::fs::read_to_string(&path) {
            Ok(data) => self.content = data,
            Err(e) => {
                self.content = format!("Failed to read {artifact}: {e}");
            }
        }
    }
}
