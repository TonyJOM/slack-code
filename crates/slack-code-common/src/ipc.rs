use crate::session::Session;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Messages sent from Claude Code hooks to the daemon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HookEvent {
    /// A new session has started
    SessionStart {
        /// Claude's internal session ID
        session_id: String,
        /// Path to transcript file
        transcript_path: Option<String>,
        /// Working directory
        cwd: String,
    },

    /// A session has ended
    SessionEnd {
        /// Claude's internal session ID
        session_id: String,
    },

    /// A notification was triggered
    Notification {
        /// Claude's internal session ID
        session_id: String,
        /// Notification message
        message: String,
        /// Notification type (permission_prompt, idle_prompt, etc.)
        notification_type: Option<String>,
    },

    /// Claude finished responding (Stop hook)
    Stop {
        /// Claude's internal session ID
        session_id: String,
    },
}

/// Messages sent from daemon to TUI clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DaemonEvent {
    /// A session was created or updated
    SessionUpdated(Session),

    /// A session was removed
    SessionRemoved(Uuid),

    /// Slack message was sent
    SlackMessageSent {
        session_id: Uuid,
        thread_ts: String,
    },

    /// An error occurred
    Error(String),

    /// Daemon status update
    Status(DaemonStatus),

    /// List of all sessions (response to GetSessions)
    SessionList(Vec<Session>),

    /// Configuration (response to GetConfig)
    ConfigResponse(crate::Config),
}

/// Daemon connection status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DaemonStatus {
    /// Connected to Slack
    Connected,

    /// Connecting to Slack
    Connecting,

    /// Disconnected from Slack (with reason)
    Disconnected(String),
}

/// Commands sent from TUI to daemon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DaemonCommand {
    /// Subscribe to daemon events
    Subscribe,

    /// Unsubscribe from daemon events
    Unsubscribe,

    /// Get all active sessions
    GetSessions,

    /// Get current configuration
    GetConfig,

    /// Ping to check if daemon is alive
    Ping,
}

/// Response to Ping command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingResponse {
    pub version: String,
    pub uptime_secs: u64,
    pub session_count: usize,
    pub slack_status: DaemonStatus,
}

/// Input structure received by hook from Claude Code (via stdin)
#[derive(Debug, Clone, Deserialize)]
pub struct ClaudeHookInput {
    pub session_id: String,
    #[serde(default)]
    pub transcript_path: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    pub hook_event_name: String,
    /// For Notification events
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub notification_type: Option<String>,
    /// For SessionStart events
    #[serde(default)]
    pub source: Option<String>,
    /// For SessionEnd events
    #[serde(default)]
    pub reason: Option<String>,
}

impl ClaudeHookInput {
    /// Convert to a HookEvent
    pub fn to_hook_event(&self) -> Option<HookEvent> {
        match self.hook_event_name.as_str() {
            "SessionStart" => Some(HookEvent::SessionStart {
                session_id: self.session_id.clone(),
                transcript_path: self.transcript_path.clone(),
                cwd: self.cwd.clone().unwrap_or_default(),
            }),
            "SessionEnd" => Some(HookEvent::SessionEnd {
                session_id: self.session_id.clone(),
            }),
            "Notification" => Some(HookEvent::Notification {
                session_id: self.session_id.clone(),
                message: self.message.clone().unwrap_or_default(),
                notification_type: self.notification_type.clone(),
            }),
            "Stop" => Some(HookEvent::Stop {
                session_id: self.session_id.clone(),
            }),
            _ => None,
        }
    }
}
