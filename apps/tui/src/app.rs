//! Core TUI application state and event loop.

use std::io;
use std::time::Duration;

use color_eyre::eyre::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Tabs};

use crate::screens::{Screen, ScreenId};
use crate::widgets::status_bar;

/// Application state.
pub(crate) struct App {
    /// Currently active screen tab.
    pub active_tab: usize,
    /// Available screens.
    pub screens: Vec<ScreenId>,
    /// Whether the app should quit.
    pub should_quit: bool,
    /// Status message shown in bottom bar.
    pub status: String,
    /// Whether help overlay is visible.
    pub show_help: bool,
    /// Per-screen state.
    pub screen_states: Vec<Screen>,
}

impl App {
    pub(crate) fn new() -> Self {
        let screens = vec![
            ScreenId::CreateKb,
            ScreenId::BrowseKbs,
            ScreenId::UpdateKb,
            ScreenId::Outputs,
            ScreenId::McpServer,
        ];
        let screen_states = screens.iter().map(|s| Screen::new(*s)).collect();

        Self {
            active_tab: 0,
            screens,
            should_quit: false,
            status: "Ready — press ? for help".to_string(),
            show_help: false,
            screen_states,
        }
    }

    fn current_screen(&self) -> &Screen {
        &self.screen_states[self.active_tab]
    }

    fn current_screen_mut(&mut self) -> &mut Screen {
        &mut self.screen_states[self.active_tab]
    }
}

/// Entry point — sets up terminal, runs event loop, restores terminal.
pub(crate) fn run() -> Result<()> {
    // Setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run app
    let result = run_app(&mut terminal);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    let mut app = App::new();

    loop {
        terminal.draw(|f| draw(f, &app))?;

        // Poll for events with 100ms timeout for responsive UI
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                handle_key(&mut app, key.code, key.modifiers);
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

fn handle_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    // Global keybindings (always active)
    match code {
        KeyCode::Char('q') | KeyCode::Char('c')
            if modifiers.contains(KeyModifiers::CONTROL) =>
        {
            app.should_quit = true;
            return;
        }
        KeyCode::Char('q') if !app.current_screen().is_editing() => {
            app.should_quit = true;
            return;
        }
        KeyCode::Char('?') if !app.current_screen().is_editing() => {
            app.show_help = !app.show_help;
            return;
        }
        KeyCode::Esc if app.show_help => {
            app.show_help = false;
            return;
        }
        // Tab navigation with number keys
        KeyCode::Char(c @ '1'..='5') if !app.current_screen().is_editing() => {
            let idx = (c as usize) - ('1' as usize);
            if idx < app.screens.len() {
                app.active_tab = idx;
                app.status = format!("{}", app.screens[idx]);
            }
            return;
        }
        KeyCode::Tab if !app.current_screen().is_editing() => {
            app.active_tab = (app.active_tab + 1) % app.screens.len();
            app.status = format!("{}", app.screens[app.active_tab]);
            return;
        }
        KeyCode::BackTab if !app.current_screen().is_editing() => {
            app.active_tab = if app.active_tab == 0 {
                app.screens.len() - 1
            } else {
                app.active_tab - 1
            };
            app.status = format!("{}", app.screens[app.active_tab]);
            return;
        }
        _ => {}
    }

    // If help is showing, consume any key to dismiss
    if app.show_help {
        app.show_help = false;
        return;
    }

    // Delegate to current screen
    app.current_screen_mut().handle_key(code, modifiers);
}

fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Tab bar
            Constraint::Min(1),    // Content
            Constraint::Length(1), // Status bar
        ])
        .split(f.area());

    // Tab bar
    let tab_titles: Vec<Line> = app
        .screens
        .iter()
        .map(|s| Line::from(format!("{s}")))
        .collect();

    let tabs = Tabs::new(tab_titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" ContextBuilder "),
        )
        .select(app.active_tab)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .divider(" │ ");

    f.render_widget(tabs, chunks[0]);

    // Content area — delegate to screen
    app.current_screen().draw(f, chunks[1]);

    // Status bar
    let bar = status_bar(&app.status);
    f.render_widget(bar, chunks[2]);

    // Help overlay
    if app.show_help {
        draw_help_overlay(f);
    }
}

fn draw_help_overlay(f: &mut Frame) {
    let area = centered_rect(60, 60, f.area());

    let help_text = vec![
        Line::from("Keybindings").style(Style::default().add_modifier(Modifier::BOLD)),
        Line::from(""),
        Line::from("  1-5          Switch to screen"),
        Line::from("  Tab/S-Tab    Next/previous screen"),
        Line::from("  ?            Toggle this help"),
        Line::from("  q / Ctrl-C   Quit"),
        Line::from(""),
        Line::from("Screen-specific:").style(Style::default().add_modifier(Modifier::BOLD)),
        Line::from("  Enter        Confirm / Start action"),
        Line::from("  Esc          Cancel / Back"),
        Line::from("  ↑/↓          Navigate lists"),
        Line::from("  Tab          Next input field"),
    ];

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Help — press any key to close ")
                .style(Style::default().bg(Color::DarkGray)),
        )
        .style(Style::default().fg(Color::White).bg(Color::DarkGray));

    // Clear background
    f.render_widget(ratatui::widgets::Clear, area);
    f.render_widget(help, area);
}

/// Create a centered rectangle with percentage width and height.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
