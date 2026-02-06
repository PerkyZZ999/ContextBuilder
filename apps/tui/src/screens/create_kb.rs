//! "Create KB" screen — URL input, name, crawl depth, and start action.

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

/// Which input field is focused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    Url,
    Name,
    Mode,
}

pub(crate) struct CreateKbScreen {
    url: String,
    name: String,
    mode: String,
    focused: Field,
    editing: bool,
    status: String,
}

impl CreateKbScreen {
    pub(crate) fn new() -> Self {
        Self {
            url: String::new(),
            name: String::new(),
            mode: "auto".to_string(),
            focused: Field::Url,
            editing: false,
            status: "Enter a documentation URL and press Enter to start.".to_string(),
        }
    }

    pub(crate) fn is_editing(&self) -> bool {
        self.editing
    }

    pub(crate) fn draw(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3), // URL
                Constraint::Length(3), // Name
                Constraint::Length(3), // Mode
                Constraint::Length(3), // Action hint
                Constraint::Min(1),   // Status / progress
            ])
            .split(area);

        // URL field
        let url_style = if self.focused == Field::Url && self.editing {
            Style::default().fg(Color::Yellow)
        } else if self.focused == Field::Url {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };
        let url_block = Block::default()
            .borders(Borders::ALL)
            .title(" URL ")
            .border_style(url_style);
        let url_text = Paragraph::new(self.url.as_str()).block(url_block);
        f.render_widget(url_text, chunks[0]);

        // Name field
        let name_style = if self.focused == Field::Name && self.editing {
            Style::default().fg(Color::Yellow)
        } else if self.focused == Field::Name {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };
        let name_block = Block::default()
            .borders(Borders::ALL)
            .title(" Name (optional) ")
            .border_style(name_style);
        let name_text = Paragraph::new(self.name.as_str()).block(name_block);
        f.render_widget(name_text, chunks[1]);

        // Mode selector
        let mode_style = if self.focused == Field::Mode {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };
        let mode_block = Block::default()
            .borders(Borders::ALL)
            .title(" Discovery mode ")
            .border_style(mode_style);
        let mode_text = Paragraph::new(format!(
            "< {} >  (← → to change)",
            self.mode
        ))
        .block(mode_block);
        f.render_widget(mode_text, chunks[2]);

        // Action hint
        let hint = if self.editing {
            "Type to edit · Esc to stop editing · Tab to next field"
        } else {
            "Enter to edit · Tab to next field · Ctrl-Enter to start"
        };
        let hint_p = Paragraph::new(hint)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(hint_p, chunks[3]);

        // Status area
        let status_block = Block::default()
            .borders(Borders::ALL)
            .title(" Status ");
        let status_text = Paragraph::new(self.status.as_str()).block(status_block);
        f.render_widget(status_text, chunks[4]);
    }

    pub(crate) fn handle_key(&mut self, code: KeyCode, _modifiers: KeyModifiers) {
        if self.editing {
            match code {
                KeyCode::Esc => {
                    self.editing = false;
                }
                KeyCode::Tab => {
                    self.editing = false;
                    self.next_field();
                }
                KeyCode::Backspace => {
                    let field = self.current_field_mut();
                    field.pop();
                }
                KeyCode::Char(c) => {
                    self.current_field_mut().push(c);
                }
                _ => {}
            }
        } else {
            match code {
                KeyCode::Enter => {
                    if self.focused == Field::Mode {
                        self.cycle_mode();
                    } else {
                        self.editing = true;
                    }
                }
                KeyCode::Tab => self.next_field(),
                KeyCode::BackTab => self.prev_field(),
                KeyCode::Left if self.focused == Field::Mode => self.cycle_mode_back(),
                KeyCode::Right if self.focused == Field::Mode => self.cycle_mode(),
                KeyCode::Up => self.prev_field(),
                KeyCode::Down => self.next_field(),
                _ => {}
            }
        }
    }

    fn current_field_mut(&mut self) -> &mut String {
        match self.focused {
            Field::Url => &mut self.url,
            Field::Name => &mut self.name,
            Field::Mode => &mut self.mode,
        }
    }

    fn next_field(&mut self) {
        self.focused = match self.focused {
            Field::Url => Field::Name,
            Field::Name => Field::Mode,
            Field::Mode => Field::Url,
        };
    }

    fn prev_field(&mut self) {
        self.focused = match self.focused {
            Field::Url => Field::Mode,
            Field::Name => Field::Url,
            Field::Mode => Field::Name,
        };
    }

    fn cycle_mode(&mut self) {
        self.mode = match self.mode.as_str() {
            "auto" => "llms-txt".to_string(),
            "llms-txt" => "crawl".to_string(),
            "crawl" => "auto".to_string(),
            _ => "auto".to_string(),
        };
    }

    fn cycle_mode_back(&mut self) {
        self.mode = match self.mode.as_str() {
            "auto" => "crawl".to_string(),
            "llms-txt" => "auto".to_string(),
            "crawl" => "llms-txt".to_string(),
            _ => "auto".to_string(),
        };
    }
}
