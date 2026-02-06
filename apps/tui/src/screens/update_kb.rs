//! "Update KB" screen — select a KB and trigger incremental update.

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

pub(crate) struct UpdateKbScreen {
    /// KB path to update (user-entered).
    kb_path: String,
    editing: bool,
    prune: bool,
    force: bool,
    status: String,
}

impl UpdateKbScreen {
    pub(crate) fn new() -> Self {
        Self {
            kb_path: String::new(),
            editing: false,
            prune: false,
            force: false,
            status: "Enter a KB path and press Ctrl-Enter to update.".to_string(),
        }
    }

    pub(crate) fn draw(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3), // KB path
                Constraint::Length(3), // Options
                Constraint::Length(3), // Action hint
                Constraint::Min(1),   // Status
            ])
            .split(area);

        // KB path input
        let path_style = if self.editing {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Cyan)
        };
        let path_block = Block::default()
            .borders(Borders::ALL)
            .title(" KB Path ")
            .border_style(path_style);
        let path_text = Paragraph::new(self.kb_path.as_str()).block(path_block);
        f.render_widget(path_text, chunks[0]);

        // Options
        let prune_marker = if self.prune { "✓" } else { " " };
        let force_marker = if self.force { "✓" } else { " " };
        let opts = Paragraph::new(format!(
            "[{prune_marker}] Prune removed pages (p)    [{force_marker}] Force re-crawl (f)"
        ))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Options "),
        );
        f.render_widget(opts, chunks[1]);

        // Hint
        let hint = if self.editing {
            "Type KB path · Esc to stop editing"
        } else {
            "Enter to edit path · p/f toggle options · Ctrl-Enter to start"
        };
        let hint_p = Paragraph::new(hint)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(hint_p, chunks[2]);

        // Status
        let status_block = Block::default()
            .borders(Borders::ALL)
            .title(" Status ");
        let status_text = Paragraph::new(self.status.as_str()).block(status_block);
        f.render_widget(status_text, chunks[3]);
    }

    pub(crate) fn handle_key(&mut self, code: KeyCode, _modifiers: KeyModifiers) {
        if self.editing {
            match code {
                KeyCode::Esc => self.editing = false,
                KeyCode::Backspace => {
                    self.kb_path.pop();
                }
                KeyCode::Char(c) => self.kb_path.push(c),
                _ => {}
            }
        } else {
            match code {
                KeyCode::Enter => self.editing = true,
                KeyCode::Char('p') => self.prune = !self.prune,
                KeyCode::Char('f') => self.force = !self.force,
                _ => {}
            }
        }
    }
}
