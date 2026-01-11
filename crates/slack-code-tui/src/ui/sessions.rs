use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use slack_code_common::session::{Session, SessionStatus};

/// Render the sessions view
pub fn render(
    frame: &mut Frame,
    area: Rect,
    sessions: &[Session],
    selected_index: usize,
    list_state: &mut ListState,
) {
    let block = Block::default()
        .title(" Active Sessions ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));

    if sessions.is_empty() {
        let empty_msg = Paragraph::new(
            "\n  No active sessions.\n\n  Sessions will appear here when Claude Code is running.",
        )
        .style(Style::default().fg(Color::DarkGray))
        .block(block);
        frame.render_widget(empty_msg, area);
        return;
    }

    // Create list items
    let items: Vec<ListItem> = sessions
        .iter()
        .enumerate()
        .map(|(i, session)| {
            let (status_icon, status_color) = status_display(&session.status);
            let is_selected = i == selected_index;

            let name = session.display_name();
            let prompt = truncate_string(&session.prompt, 50);
            let duration = session.duration_string();

            let content = format!(
                " {} {:<20} {} {}\n   {} \n   {}",
                if is_selected { ">" } else { " " },
                name,
                status_icon,
                session.status.short_string(),
                prompt,
                duration
            );

            let style = if is_selected {
                Style::default().fg(status_color).bold()
            } else {
                Style::default().fg(status_color)
            };

            ListItem::new(content).style(style)
        })
        .collect();

    list_state.select(Some(selected_index));

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, area, list_state);
}

/// Get status icon and color
fn status_display(status: &SessionStatus) -> (&str, Color) {
    match status {
        SessionStatus::Starting => ("...", Color::Yellow),
        SessionStatus::Running => (">>>", Color::Green),
        SessionStatus::WaitingForInput(_) => ("???", Color::Magenta),
        SessionStatus::Completed => ("[x]", Color::Cyan),
        SessionStatus::Failed(_) => ("[!]", Color::Red),
    }
}

/// Truncate a string to max length with ellipsis
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
