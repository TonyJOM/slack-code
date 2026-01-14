use crate::error::{Result, SlackCodeError};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub slack: SlackConfig,
    #[serde(default)]
    pub daemon: DaemonConfig,
    #[serde(default)]
    pub defaults: DefaultsConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            slack: SlackConfig::default(),
            daemon: DaemonConfig::default(),
            defaults: DefaultsConfig::default(),
        }
    }
}

/// Slack API configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SlackConfig {
    /// Bot OAuth Token (xoxb-...)
    /// Can be overridden by SLACK_CODE_BOT_TOKEN env var
    #[serde(default)]
    pub bot_token: String,

    /// App-Level Token for Socket Mode (xapp-...)
    /// Can be overridden by SLACK_CODE_APP_TOKEN env var
    #[serde(default)]
    pub app_token: String,

    /// Your Slack Member ID (required, set during setup)
    pub user_id: String,
}

impl SlackConfig {
    /// Get bot token, checking env var first
    pub fn get_bot_token(&self) -> String {
        std::env::var("SLACK_CODE_BOT_TOKEN").unwrap_or_else(|_| self.bot_token.clone())
    }

    /// Get app token, checking env var first
    pub fn get_app_token(&self) -> String {
        std::env::var("SLACK_CODE_APP_TOKEN").unwrap_or_else(|_| self.app_token.clone())
    }

    /// Mask a token for display (show first 4 and last 4 chars)
    pub fn mask_token(token: &str) -> String {
        if token.len() <= 12 {
            return "****".to_string();
        }
        let prefix = &token[..8];
        let suffix = &token[token.len() - 4..];
        format!("{}****...{}", prefix, suffix)
    }
}

/// Daemon configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    /// Socket path for IPC
    #[serde(default = "default_socket_path")]
    pub socket_path: PathBuf,

    /// PID file path
    #[serde(default = "default_pid_file")]
    pub pid_file: PathBuf,

    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub log_level: String,

    /// Log file path
    #[serde(default = "default_log_file")]
    pub log_file: PathBuf,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            socket_path: default_socket_path(),
            pid_file: default_pid_file(),
            log_level: default_log_level(),
            log_file: default_log_file(),
        }
    }
}

/// Default values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultsConfig {
    /// Timeout for hook operations (seconds)
    #[serde(default = "default_hook_timeout")]
    pub hook_timeout: u64,
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            hook_timeout: default_hook_timeout(),
        }
    }
}

// Default value functions
fn default_socket_path() -> PathBuf {
    get_runtime_dir().join("slack-code/daemon.sock")
}

fn default_pid_file() -> PathBuf {
    get_runtime_dir().join("slack-code/daemon.pid")
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_file() -> PathBuf {
    get_data_dir().join("slack-code/daemon.log")
}

fn default_hook_timeout() -> u64 {
    5
}

// Directory helpers
pub fn get_config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap().join(".config"))
}

pub fn get_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap().join(".local/share"))
}

pub fn get_runtime_dir() -> PathBuf {
    dirs::runtime_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap().join(".local/run"))
}

impl Config {
    /// Get the config file path
    pub fn config_path() -> PathBuf {
        get_config_dir().join("slack-code/config.toml")
    }

    /// Load configuration from file
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path();

        if !config_path.exists() {
            return Err(SlackCodeError::ConfigNotFound);
        }

        let content = std::fs::read_to_string(&config_path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path();

        // Ensure directory exists
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        std::fs::write(&config_path, content)?;
        Ok(())
    }

    /// Check if config file exists
    pub fn exists() -> bool {
        Self::config_path().exists()
    }
}

// Claude Code hooks management
const HOOK_COMMAND: &str = "slack-code-hook";

/// Install hooks into Claude Code settings
pub fn install_hooks() -> Result<()> {
    let claude_settings_path = dirs::home_dir()
        .ok_or_else(|| SlackCodeError::Hook("Cannot determine home directory".into()))?
        .join(".claude/settings.json");

    // Read existing settings or create new
    let mut settings: serde_json::Value = if claude_settings_path.exists() {
        let content = std::fs::read_to_string(&claude_settings_path)?;
        serde_json::from_str(&content)?
    } else {
        serde_json::json!({})
    };

    // Ensure hooks object exists
    if settings.get("hooks").is_none() {
        settings["hooks"] = serde_json::json!({});
    }

    let hooks = settings.get_mut("hooks").unwrap();

    // Add SessionStart hook
    hooks["SessionStart"] = serde_json::json!([{
        "hooks": [{
            "type": "command",
            "command": HOOK_COMMAND,
            "timeout": 5
        }]
    }]);

    // Add SessionEnd hook
    hooks["SessionEnd"] = serde_json::json!([{
        "hooks": [{
            "type": "command",
            "command": HOOK_COMMAND,
            "timeout": 5
        }]
    }]);

    // Add Notification hook
    hooks["Notification"] = serde_json::json!([{
        "matcher": "permission_prompt",
        "hooks": [{
            "type": "command",
            "command": HOOK_COMMAND,
            "timeout": 5
        }]
    }]);

    // Add Stop hook - fires when Claude finishes responding
    hooks["Stop"] = serde_json::json!([{
        "hooks": [{
            "type": "command",
            "command": HOOK_COMMAND,
            "timeout": 5
        }]
    }]);

    // Ensure directory exists
    if let Some(parent) = claude_settings_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Write settings
    let content = serde_json::to_string_pretty(&settings)?;
    std::fs::write(&claude_settings_path, content)?;

    Ok(())
}

/// Uninstall hooks from Claude Code settings
pub fn uninstall_hooks() -> Result<()> {
    let claude_settings_path = dirs::home_dir()
        .ok_or_else(|| SlackCodeError::Hook("Cannot determine home directory".into()))?
        .join(".claude/settings.json");

    if !claude_settings_path.exists() {
        return Ok(());
    }

    let content = std::fs::read_to_string(&claude_settings_path)?;
    let mut settings: serde_json::Value = serde_json::from_str(&content)?;

    if let Some(hooks) = settings.get_mut("hooks") {
        // Remove our hooks
        if let Some(obj) = hooks.as_object_mut() {
            obj.remove("SessionStart");
            obj.remove("SessionEnd");
            obj.remove("Notification");
        }
    }

    let content = serde_json::to_string_pretty(&settings)?;
    std::fs::write(&claude_settings_path, content)?;

    Ok(())
}

/// Check if hooks are installed
pub fn check_hooks_installed() -> Result<bool> {
    let claude_settings_path = dirs::home_dir()
        .ok_or_else(|| SlackCodeError::Hook("Cannot determine home directory".into()))?
        .join(".claude/settings.json");

    if !claude_settings_path.exists() {
        return Ok(false);
    }

    let content = std::fs::read_to_string(&claude_settings_path)?;
    let settings: serde_json::Value = serde_json::from_str(&content)?;

    // Check if our hooks exist
    if let Some(hooks) = settings.get("hooks") {
        let has_session_start = hooks.get("SessionStart").is_some();
        let has_session_end = hooks.get("SessionEnd").is_some();
        let has_notification = hooks.get("Notification").is_some();

        return Ok(has_session_start && has_session_end && has_notification);
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_token() {
        assert_eq!(
            SlackConfig::mask_token("xoxb-1234567890-abcdefghij"),
            "xoxb-123****...ghij"
        );
        assert_eq!(SlackConfig::mask_token("short"), "****");
    }
}
