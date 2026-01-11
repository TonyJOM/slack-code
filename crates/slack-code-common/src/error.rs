use thiserror::Error;

#[derive(Error, Debug)]
pub enum SlackCodeError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Configuration not found. Run 'slack-code setup' first.")]
    ConfigNotFound,

    #[error("IPC error: {0}")]
    Ipc(String),

    #[error("Session error: {0}")]
    Session(String),

    #[error("Hook error: {0}")]
    Hook(String),

    #[error("Daemon not running")]
    DaemonNotRunning,

    #[error("Daemon already running")]
    DaemonAlreadyRunning,

    #[error("Slack API error: {0}")]
    SlackApi(String),

    #[error("Invalid token format: {0}")]
    InvalidToken(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("TOML serialization error: {0}")]
    TomlSer(#[from] toml::ser::Error),

    #[error("TOML deserialization error: {0}")]
    TomlDe(#[from] toml::de::Error),
}

pub type Result<T> = std::result::Result<T, SlackCodeError>;
