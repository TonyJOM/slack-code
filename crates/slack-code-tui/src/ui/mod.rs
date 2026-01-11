pub mod config;
pub mod logs;
pub mod sessions;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

/// Render the header bar
pub fn render_header(frame: &mut Frame, area: Rect, daemon_connected: bool) {
    let status = if daemon_connected {
        "Daemon: [Connected]"
    } else {
        "Daemon: [Disconnected]"
    };

    let header_text = format!("  slack-code v{}                           {}",
        env!("CARGO_PKG_VERSION"),
        status
    );

    let header = Paragraph::new(header_text)
        .style(Style::default().fg(Color::Cyan).bold())
        .block(Block::default().borders(Borders::BOTTOM));

    frame.render_widget(header, area);
}

/// Render the status bar with keybindings
pub fn render_status_bar(frame: &mut Frame, area: Rect, mode: &crate::app::AppMode) {
    let keybindings = match mode {
        crate::app::AppMode::Sessions => {
            "[1] Sessions  [2] Config  [3] Logs  [r] Refresh  [?] Help  [q] Quit"
        }
        crate::app::AppMode::Config => {
            "[t] Test tokens  [h] Manage hooks  [?] Help  [q] Quit"
        }
        crate::app::AppMode::Logs => {
            "[1] Sessions  [2] Config  [3] Logs  [?] Help  [q] Quit"
        }
        crate::app::AppMode::Help => {
            "Press any key to close help"
        }
    };

    let status = Paragraph::new(format!("  {}", keybindings))
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::TOP));

    frame.render_widget(status, area);
}

/// Render the help overlay
pub fn render_help_overlay(frame: &mut Frame, area: Rect) {
    let help_text = r#"
    KEYBOARD SHORTCUTS

    Global:
      1         Sessions view
      2         Config view
      3         Logs view
      ?         Toggle help
      q         Quit

    Navigation:
      j / Down  Next item
      k / Up    Previous item
      Enter     Select
      Esc       Cancel / Back

    Sessions:
      r         Refresh sessions

    Config:
      t         Test Slack tokens
      h         Manage hooks
    "#;

    let help = Paragraph::new(help_text)
        .style(Style::default())
        .block(
            Block::default()
                .title(" Help ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        );

    // Center the help overlay
    let popup_area = centered_rect(60, 70, area);
    frame.render_widget(ratatui::widgets::Clear, popup_area);
    frame.render_widget(help, popup_area);
}

/// Create a centered rectangle
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
