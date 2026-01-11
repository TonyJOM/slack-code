use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

/// Input events from the terminal
#[derive(Debug, Clone)]
pub enum InputEvent {
    /// Key press
    Key(KeyEvent),
    /// Terminal resize
    Resize(u16, u16),
    /// Tick (for animations/updates)
    Tick,
}

/// Event handler for terminal input
pub struct EventHandler {
    tick_rate: Duration,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        Self { tick_rate }
    }

    /// Poll for the next event
    pub fn next(&self) -> Result<InputEvent> {
        if event::poll(self.tick_rate)? {
            match event::read()? {
                Event::Key(key) => Ok(InputEvent::Key(key)),
                Event::Resize(w, h) => Ok(InputEvent::Resize(w, h)),
                _ => Ok(InputEvent::Tick),
            }
        } else {
            Ok(InputEvent::Tick)
        }
    }
}

/// Message type for the TUI application
#[derive(Debug, Clone)]
pub enum Message {
    // Navigation
    Quit,
    NextItem,
    PrevItem,
    Select,
    Back,
    SwitchMode(crate::app::AppMode),

    // Input
    Escape,

    // Config actions
    TestTokens,
    ManageHooks,

    // Session actions
    RefreshSessions,

    // Help
    ToggleHelp,

    // Daemon events
    DaemonEvent(slack_code_common::ipc::DaemonEvent),
}

impl Message {
    /// Convert a key event to a message based on current mode
    pub fn from_key(key: KeyEvent, mode: &crate::app::AppMode) -> Option<Self> {
        // Handle Ctrl+C globally
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Some(Message::Quit);
        }

        match key.code {
            // Global navigation
            KeyCode::Char('q') => Some(Message::Quit),
            KeyCode::Char('?') => Some(Message::ToggleHelp),
            KeyCode::Char('1') => Some(Message::SwitchMode(crate::app::AppMode::Sessions)),
            KeyCode::Char('2') => Some(Message::SwitchMode(crate::app::AppMode::Config)),
            KeyCode::Char('3') => Some(Message::SwitchMode(crate::app::AppMode::Logs)),

            // List navigation
            KeyCode::Down | KeyCode::Char('j') => Some(Message::NextItem),
            KeyCode::Up | KeyCode::Char('k') => Some(Message::PrevItem),
            KeyCode::Enter => Some(Message::Select),
            KeyCode::Esc => Some(Message::Escape),

            // Mode-specific keys
            KeyCode::Char('t') if matches!(mode, crate::app::AppMode::Config) => {
                Some(Message::TestTokens)
            }
            KeyCode::Char('h') if matches!(mode, crate::app::AppMode::Config) => {
                Some(Message::ManageHooks)
            }
            KeyCode::Char('r') if matches!(mode, crate::app::AppMode::Sessions) => {
                Some(Message::RefreshSessions)
            }

            _ => None,
        }
    }
}
