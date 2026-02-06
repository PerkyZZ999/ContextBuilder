//! "MCP Server" screen — start/stop the MCP server and view config.

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Transport {
    Stdio,
    Http,
}

impl std::fmt::Display for Transport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stdio => write!(f, "stdio"),
            Self::Http => write!(f, "http"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ServerState {
    Stopped,
    Running,
}

pub(crate) struct McpServerScreen {
    transport: Transport,
    port: u16,
    state: ServerState,
    config_target: usize,
    status: String,
}

const CONFIG_TARGETS: &[&str] = &["vscode", "claude-desktop", "cursor"];

impl McpServerScreen {
    pub(crate) fn new() -> Self {
        Self {
            transport: Transport::Stdio,
            port: 3100,
            state: ServerState::Stopped,
            config_target: 0,
            status: "Press Enter to start the MCP server.".to_string(),
        }
    }

    pub(crate) fn draw(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(5),  // Server status
                Constraint::Length(5),  // Config selector
                Constraint::Min(1),    // Config preview
                Constraint::Length(1), // Controls
            ])
            .split(area);

        // Server status
        let state_color = match self.state {
            ServerState::Stopped => Color::Red,
            ServerState::Running => Color::Green,
        };
        let state_label = match self.state {
            ServerState::Stopped => "● Stopped",
            ServerState::Running => "● Running",
        };

        let server_info = Paragraph::new(vec![
            Line::from(vec![
                Span::raw("Status: "),
                Span::styled(state_label, Style::default().fg(state_color)),
            ]),
            Line::from(format!("Transport: {}", self.transport)),
            Line::from(format!("Port: {}", self.port)),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" MCP Server "),
        );
        f.render_widget(server_info, chunks[0]);

        // Config selector
        let targets: Vec<Span> = CONFIG_TARGETS
            .iter()
            .enumerate()
            .flat_map(|(i, name)| {
                let style = if i == self.config_target {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                vec![Span::styled(format!(" {name} "), style), Span::raw(" │ ")]
            })
            .collect();

        let config_header = Paragraph::new(vec![
            Line::from("Client configuration snippet:"),
            Line::from(""),
            Line::from(targets),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Config (← → to switch) "),
        );
        f.render_widget(config_header, chunks[1]);

        // Config preview
        let config_text = self.generate_config_snippet();
        let config_preview = Paragraph::new(config_text)
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(
                        " {} config ",
                        CONFIG_TARGETS[self.config_target]
                    )),
            );
        f.render_widget(config_preview, chunks[2]);

        // Controls
        let controls = match self.state {
            ServerState::Stopped => {
                "Enter: Start server · t: Toggle transport · ← →: Switch config target"
            }
            ServerState::Running => "Enter: Stop server · ← →: Switch config target",
        };
        let ctrl = Paragraph::new(controls)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(ctrl, chunks[3]);
    }

    pub(crate) fn handle_key(&mut self, code: KeyCode, _modifiers: KeyModifiers) {
        match code {
            KeyCode::Enter => {
                self.state = match self.state {
                    ServerState::Stopped => {
                        self.status = "MCP server started.".to_string();
                        ServerState::Running
                    }
                    ServerState::Running => {
                        self.status = "MCP server stopped.".to_string();
                        ServerState::Stopped
                    }
                };
            }
            KeyCode::Char('t') if self.state == ServerState::Stopped => {
                self.transport = match self.transport {
                    Transport::Stdio => Transport::Http,
                    Transport::Http => Transport::Stdio,
                };
            }
            KeyCode::Left => {
                if self.config_target > 0 {
                    self.config_target -= 1;
                }
            }
            KeyCode::Right => {
                if self.config_target + 1 < CONFIG_TARGETS.len() {
                    self.config_target += 1;
                }
            }
            _ => {}
        }
    }

    fn generate_config_snippet(&self) -> String {
        let target = CONFIG_TARGETS[self.config_target];
        match target {
            "vscode" => {
                r#"{
  "servers": {
    "contextbuilder": {
      "type": "stdio",
      "command": "contextbuilder",
      "args": ["mcp", "serve", "--kb", "<path-to-kb>"]
    }
  }
}"#
                .to_string()
            }
            "claude-desktop" => {
                r#"{
  "mcpServers": {
    "contextbuilder": {
      "command": "contextbuilder",
      "args": ["mcp", "serve", "--kb", "<path-to-kb>"]
    }
  }
}"#
                .to_string()
            }
            "cursor" => {
                r#"{
  "mcpServers": {
    "contextbuilder": {
      "command": "contextbuilder",
      "args": ["mcp", "serve", "--kb", "<path-to-kb>"]
    }
  }
}"#
                .to_string()
            }
            _ => "Unknown target".to_string(),
        }
    }
}
