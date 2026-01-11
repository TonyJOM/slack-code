use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// A Claude Code session tracked by slack-code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique identifier for this session
    pub id: Uuid,

    /// Claude Code's internal session ID (from transcript path)
    #[serde(default)]
    pub claude_session_id: Option<String>,

    /// Full path to the repository
    pub repo_path: PathBuf,

    /// Alias used to start the session (if any)
    #[serde(default)]
    pub repo_alias: Option<String>,

    /// The prompt/task given to Claude Code
    pub prompt: String,

    /// Current status of the session
    pub status: SessionStatus,

    /// When the session was started
    pub started_at: DateTime<Utc>,

    /// When the session ended (if completed/failed)
    #[serde(default)]
    pub ended_at: Option<DateTime<Utc>>,

    /// Slack thread information
    #[serde(default)]
    pub slack_thread: Option<SlackThread>,

    /// Path to Claude Code's transcript file
    #[serde(default)]
    pub transcript_path: Option<PathBuf>,
}

impl Session {
    /// Create a new session
    pub fn new(repo_path: PathBuf, repo_alias: Option<String>, prompt: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            claude_session_id: None,
            repo_path,
            repo_alias,
            prompt,
            status: SessionStatus::Starting,
            started_at: Utc::now(),
            ended_at: None,
            slack_thread: None,
            transcript_path: None,
        }
    }

    /// Get the display name for this session (alias or path)
    pub fn display_name(&self) -> String {
        self.repo_alias
            .clone()
            .unwrap_or_else(|| self.repo_path.display().to_string())
    }

    /// Calculate session duration
    pub fn duration(&self) -> chrono::Duration {
        let end = self.ended_at.unwrap_or_else(Utc::now);
        end - self.started_at
    }

    /// Format duration as human-readable string
    pub fn duration_string(&self) -> String {
        let duration = self.duration();
        let secs = duration.num_seconds();

        if secs < 60 {
            format!("{}s", secs)
        } else if secs < 3600 {
            format!("{}m {}s", secs / 60, secs % 60)
        } else {
            format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
        }
    }

    /// Check if session is still active
    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            SessionStatus::Starting | SessionStatus::Running | SessionStatus::WaitingForInput(_)
        )
    }
}

/// Current status of a session
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SessionStatus {
    /// Session is starting up
    Starting,

    /// Claude is actively working
    Running,

    /// Waiting for user input
    WaitingForInput(WaitReason),

    /// Session completed successfully
    Completed,

    /// Session failed with error
    Failed(String),
}

impl SessionStatus {
    /// Get a short status string for display
    pub fn short_string(&self) -> &str {
        match self {
            SessionStatus::Starting => "Starting",
            SessionStatus::Running => "Running",
            SessionStatus::WaitingForInput(WaitReason::PermissionPrompt) => "Needs Permission",
            SessionStatus::WaitingForInput(WaitReason::IdlePrompt) => "Waiting",
            SessionStatus::WaitingForInput(WaitReason::PlanApproval) => "Plan Review",
            SessionStatus::Completed => "Completed",
            SessionStatus::Failed(_) => "Failed",
        }
    }

    /// Get a status icon for display
    pub fn icon(&self) -> &str {
        match self {
            SessionStatus::Starting => "...",
            SessionStatus::Running => ">>>",
            SessionStatus::WaitingForInput(_) => "???",
            SessionStatus::Completed => "[x]",
            SessionStatus::Failed(_) => "[!]",
        }
    }
}

/// Reason for waiting on user input
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WaitReason {
    /// Permission dialog shown
    PermissionPrompt,

    /// Idle/waiting for input (generic)
    IdlePrompt,

    /// Plan approval needed
    PlanApproval,
}

impl WaitReason {
    /// Convert from notification type string
    pub fn from_notification_type(s: &str) -> Self {
        match s {
            "permission_prompt" => WaitReason::PermissionPrompt,
            "idle_prompt" => WaitReason::IdlePrompt,
            _ => WaitReason::IdlePrompt,
        }
    }
}

/// Slack thread information for a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackThread {
    /// Slack channel ID (DM channel with user)
    pub channel_id: String,

    /// Parent message timestamp (thread root)
    pub parent_ts: String,
}
