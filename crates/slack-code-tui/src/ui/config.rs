use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};
use slack_code_common::config::SlackConfig;
use slack_code_common::Config;

/// Configuration section
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigSection {
    SlackTokens,
    Hooks,
}

/// Render the config view
pub fn render(
    frame: &mut Frame,
    area: Rect,
    config: &Config,
    selected_section: ConfigSection,
    hooks_installed: bool,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),  // Slack tokens
            Constraint::Min(4),     // Hooks status
        ])
        .split(area);

    // Slack Tokens section
    render_tokens_section(
        frame,
        chunks[0],
        &config.slack,
        selected_section == ConfigSection::SlackTokens,
    );

    // Hooks section
    render_hooks_section(
        frame,
        chunks[1],
        hooks_installed,
        selected_section == ConfigSection::Hooks,
    );
}

fn render_tokens_section(frame: &mut Frame, area: Rect, slack: &SlackConfig, selected: bool) {
    let border_style = if selected {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Blue)
    };

    let bot_token = slack.get_bot_token();

    // Need to format these properly due to lifetime issues
    let bot_display = if bot_token.is_empty() {
        "Not configured".to_string()
    } else {
        SlackConfig::mask_token(&bot_token)
    };

    let content = format!(
        "\n  Bot Token: {}",
        bot_display,
    );

    let block = Block::default()
        .title(" Slack Integration ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let paragraph = Paragraph::new(content).block(block);
    frame.render_widget(paragraph, area);
}

fn render_hooks_section(frame: &mut Frame, area: Rect, installed: bool, selected: bool) {
    let border_style = if selected {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Blue)
    };

    let (status, color) = if installed {
        ("Installed", Color::Green)
    } else {
        ("Not installed", Color::Red)
    };

    let content = format!("\n  Claude Code Hooks: {}\n  Press 'h' to {}", status, if installed { "uninstall" } else { "install" });

    let block = Block::default()
        .title(" Hook Status ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let paragraph = Paragraph::new(content)
        .style(Style::default().fg(color))
        .block(block);

    frame.render_widget(paragraph, area);
}
