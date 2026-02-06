//! Reusable TUI widgets.

use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

/// Bottom status bar.
pub(crate) fn status_bar(msg: &str) -> Paragraph<'_> {
    Paragraph::new(format!(" {msg}"))
        .style(
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::White),
        )
}
