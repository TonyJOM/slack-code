use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use ratatui::widgets::ListState;
use slack_code_common::ipc::DaemonEvent;
use slack_code_common::session::Session;
use slack_code_common::Config;
use slack_code_daemon::ipc::{EventSubscription, IpcClient};
use std::collections::VecDeque;
use std::io;
use std::time::Duration;

use crate::events::{EventHandler, InputEvent, Message};
use crate::ui;
use crate::ui::config::ConfigSection;
use crate::ui::logs::LogEntry;

/// Application mode/screen
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    Sessions,
    Config,
    Logs,
    Help,
}

/// Main TUI application
pub struct App {
    /// Current mode/screen
    mode: AppMode,

    /// Whether to show help overlay
    show_help: bool,

    /// Active sessions
    sessions: Vec<Session>,

    /// Current configuration
    config: Config,

    /// Selected index in current list
    selected_index: usize,

    /// Selected config section
    config_section: ConfigSection,

    /// Log entries
    logs: VecDeque<LogEntry>,

    /// Log scroll offset
    log_scroll: usize,

    /// Whether hooks are installed
    hooks_installed: bool,

    /// Whether daemon is connected
    daemon_connected: bool,

    /// Should quit
    should_quit: bool,

    /// List state for ratatui
    list_state: ListState,
}

impl App {
    /// Create a new app instance
    pub fn new() -> Result<Self> {
        let config = Config::load().unwrap_or_default();
        let hooks_installed =
            slack_code_common::config::check_hooks_installed().unwrap_or(false);

        Ok(Self {
            mode: AppMode::Sessions,
            show_help: false,
            sessions: Vec::new(),
            config,
            selected_index: 0,
            config_section: ConfigSection::SlackTokens,
            logs: VecDeque::with_capacity(1000),
            log_scroll: 0,
            hooks_installed,
            daemon_connected: false,
            should_quit: false,
            list_state: ListState::default(),
        })
    }

    /// Run the TUI application
    pub async fn run() -> Result<()> {
        let mut app = Self::new()?;
        let config = Config::load().unwrap_or_default();

        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Event handler for keyboard/terminal events
        let event_handler = EventHandler::new(Duration::from_millis(250));

        // Connect to daemon and subscribe to events
        let ipc_client = IpcClient::new(config.daemon.socket_path.clone());
        let daemon_subscription = match ipc_client.subscribe() {
            Ok(sub) => {
                app.daemon_connected = true;
                app.add_log(LogEntry::info("Connected to daemon"));
                Some(sub)
            }
            Err(e) => {
                app.add_log(LogEntry::warning(format!("Could not connect to daemon: {}", e)));
                None
            }
        };

        app.add_log(LogEntry::info("slack-code TUI started"));

        // Check if required Slack configuration is set
        let bot_token = app.config.slack.get_bot_token();
        let user_id = &app.config.slack.user_id;
        if bot_token.is_empty() || user_id.is_empty() {
            app.add_log(LogEntry::warning(
                "Slack not configured. Run 'slack-code setup' to configure.".to_string(),
            ));
        }

        // Main loop
        let result = app.run_loop(&mut terminal, &event_handler, daemon_subscription).await;

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        result
    }

    async fn run_loop<B: Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
        event_handler: &EventHandler,
        mut daemon_sub: Option<EventSubscription>,
    ) -> Result<()> {
        loop {
            // Draw UI
            terminal.draw(|frame| self.render(frame))?;

            // Handle events
            match event_handler.next()? {
                InputEvent::Key(key) => {
                    if let Some(msg) = Message::from_key(key, &self.mode) {
                        self.update(msg);
                    }
                }
                InputEvent::Resize(_, _) => {
                    // Terminal will redraw automatically
                }
                InputEvent::Tick => {
                    // Poll daemon for events (non-blocking)
                    if let Some(ref mut sub) = daemon_sub {
                        match sub.try_recv() {
                            Ok(Some(event)) => {
                                self.update(Message::DaemonEvent(event));
                            }
                            Ok(None) => {} // No event ready
                            Err(_) => {
                                // Connection lost
                                self.daemon_connected = false;
                                self.add_log(LogEntry::error("Daemon connection lost".to_string()));
                                daemon_sub = None;
                            }
                        }
                    }
                }
            }

            if self.should_quit {
                break;
            }
        }

        Ok(())
    }

    /// Handle a message and update state
    fn update(&mut self, msg: Message) {
        match msg {
            Message::Quit => {
                self.should_quit = true;
            }
            Message::ToggleHelp => {
                self.show_help = !self.show_help;
            }
            Message::SwitchMode(mode) => {
                self.mode = mode;
                self.selected_index = 0;
            }
            Message::NextItem => {
                let max = self.item_count();
                if max > 0 {
                    self.selected_index = (self.selected_index + 1).min(max - 1);
                }
            }
            Message::PrevItem => {
                self.selected_index = self.selected_index.saturating_sub(1);
            }
            Message::Select => {
                self.handle_select();
            }
            Message::Escape => {
                if self.show_help {
                    self.show_help = false;
                }
            }
            Message::TestTokens => {
                self.add_log(LogEntry::info("Testing Slack tokens..."));
                // TODO: Actually test tokens
            }
            Message::ManageHooks => {
                if self.hooks_installed {
                    match slack_code_common::config::uninstall_hooks() {
                        Ok(()) => {
                            self.hooks_installed = false;
                            self.add_log(LogEntry::info("Hooks uninstalled"));
                        }
                        Err(e) => {
                            self.add_log(LogEntry::error(format!(
                                "Failed to uninstall hooks: {}",
                                e
                            )));
                        }
                    }
                } else {
                    match slack_code_common::config::install_hooks() {
                        Ok(()) => {
                            self.hooks_installed = true;
                            self.add_log(LogEntry::info("Hooks installed"));
                        }
                        Err(e) => {
                            self.add_log(LogEntry::error(format!(
                                "Failed to install hooks: {}",
                                e
                            )));
                        }
                    }
                }
            }
            Message::RefreshSessions => {
                self.add_log(LogEntry::info("Refreshing sessions..."));
                // TODO: Request sessions from daemon
            }
            Message::DaemonEvent(event) => {
                self.handle_daemon_event(event);
            }
            _ => {}
        }
    }

    fn handle_select(&mut self) {
        match &self.mode {
            AppMode::Config => {
                // Cycle through config sections
                self.config_section = match self.config_section {
                    ConfigSection::SlackTokens => ConfigSection::Hooks,
                    ConfigSection::Hooks => ConfigSection::SlackTokens,
                };
                self.selected_index = 0;
            }
            _ => {}
        }
    }

    fn handle_daemon_event(&mut self, event: DaemonEvent) {
        match event {
            DaemonEvent::SessionUpdated(session) => {
                // Update or add session
                if let Some(existing) = self.sessions.iter_mut().find(|s| s.id == session.id) {
                    *existing = session;
                } else {
                    self.sessions.push(session);
                }
            }
            DaemonEvent::SessionRemoved(id) => {
                self.sessions.retain(|s| s.id != id);
            }
            DaemonEvent::Error(msg) => {
                self.add_log(LogEntry::error(msg));
            }
            DaemonEvent::Status(status) => {
                self.daemon_connected =
                    matches!(status, slack_code_common::ipc::DaemonStatus::Connected);
            }
            DaemonEvent::SessionList(sessions) => {
                self.sessions = sessions;
            }
            DaemonEvent::ConfigResponse(config) => {
                self.config = config;
            }
            _ => {}
        }
    }

    fn add_log(&mut self, entry: LogEntry) {
        self.logs.push_back(entry);
        if self.logs.len() > 1000 {
            self.logs.pop_front();
        }
    }

    fn item_count(&self) -> usize {
        match &self.mode {
            AppMode::Sessions => self.sessions.len(),
            AppMode::Logs => self.logs.len(),
            _ => 0,
        }
    }

    /// Render the UI
    fn render(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Header
                Constraint::Min(0),     // Content
                Constraint::Length(3),  // Status bar
            ])
            .split(frame.area());

        // Header
        ui::render_header(frame, chunks[0], self.daemon_connected);

        // Content based on mode
        match &self.mode {
            AppMode::Sessions => {
                ui::sessions::render(
                    frame,
                    chunks[1],
                    &self.sessions,
                    self.selected_index,
                    &mut self.list_state,
                );
            }
            AppMode::Config => {
                ui::config::render(
                    frame,
                    chunks[1],
                    &self.config,
                    self.config_section,
                    self.hooks_installed,
                );
            }
            AppMode::Logs => {
                ui::logs::render(frame, chunks[1], &self.logs, self.log_scroll);
            }
            AppMode::Help => {
                ui::sessions::render(
                    frame,
                    chunks[1],
                    &self.sessions,
                    self.selected_index,
                    &mut self.list_state,
                );
            }
        }

        // Status bar
        ui::render_status_bar(frame, chunks[2], &self.mode);

        // Help overlay
        if self.show_help {
            ui::render_help_overlay(frame, frame.area());
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new().expect("Failed to create app")
    }
}
