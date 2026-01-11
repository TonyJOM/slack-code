use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use std::collections::VecDeque;

/// Log entry with level and message
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub level: LogLevel,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
}

impl LogEntry {
    pub fn info(msg: impl Into<String>) -> Self {
        Self {
            level: LogLevel::Info,
            message: msg.into(),
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn warning(msg: impl Into<String>) -> Self {
        Self {
            level: LogLevel::Warning,
            message: msg.into(),
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            level: LogLevel::Error,
            message: msg.into(),
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Render the logs view
pub fn render(frame: &mut Frame, area: Rect, logs: &VecDeque<LogEntry>, scroll_offset: usize) {
    let block = Block::default()
        .title(" Logs ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));

    if logs.is_empty() {
        let empty_msg = Paragraph::new("\n  No logs yet.")
            .style(Style::default().fg(Color::DarkGray))
            .block(block);
        frame.render_widget(empty_msg, area);
        return;
    }

    // Format log entries
    let log_text: String = logs
        .iter()
        .skip(scroll_offset)
        .map(|entry| {
            let level_str = match entry.level {
                LogLevel::Info => "[INFO]",
                LogLevel::Warning => "[WARN]",
                LogLevel::Error => "[ERROR]",
            };
            let time = entry.timestamp.format("%H:%M:%S");
            format!("{} {} {}\n", time, level_str, entry.message)
        })
        .collect();

    let paragraph = Paragraph::new(log_text)
        .block(block)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}
