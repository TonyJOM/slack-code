use anyhow::Result;
use daemonize::Daemonize;
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use slack_code_common::ipc::{DaemonCommand, DaemonEvent, DaemonStatus, HookEvent};
use slack_code_common::Config;
use std::fs::File;
use std::io::Read as _;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};

use crate::ipc::{IpcClient, IpcServer};
use crate::session::SessionManager;
use crate::slack::SlackService;

/// Main daemon process
pub struct Daemon {
    config: Config,
    session_manager: Arc<RwLock<SessionManager>>,
}

impl Daemon {
    /// Create a new daemon instance
    pub fn new(config: Config) -> Result<Self> {
        let session_manager = Arc::new(RwLock::new(SessionManager::new()));

        Ok(Self {
            config,
            session_manager,
        })
    }

    /// Run daemon - daemonizes when called via `slack-code daemon start`
    /// This is sync because we need to fork before any tokio runtime exists
    pub fn run_foreground() -> Result<()> {
        let config = Config::load()?;

        // Ensure directories exist
        if let Some(parent) = config.daemon.pid_file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if let Some(parent) = config.daemon.log_file.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Check if already running
        if Self::is_running_sync() {
            anyhow::bail!("Daemon is already running");
        }

        // Open log file
        let stdout = File::create(&config.daemon.log_file)?;
        let stderr = stdout.try_clone()?;

        // Daemonize (detach from terminal) - this forks and parent exits
        let daemonize = Daemonize::new()
            .pid_file(&config.daemon.pid_file)
            .chown_pid_file(true)
            .working_directory("/")
            .stdout(stdout)
            .stderr(stderr);

        daemonize.start().map_err(|e| anyhow::anyhow!("Failed to daemonize: {}", e))?;

        // Now running as daemon child process - create new tokio runtime
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let daemon = Self::new(config)?;
            daemon.run().await
        })
    }

    /// Start the daemon in the background as a subprocess
    pub fn start_background() -> Result<()> {
        // Check if already running
        if Self::is_running_sync() {
            return Ok(());
        }

        let config = Config::load()?;

        // Ensure directories exist
        if let Some(parent) = config.daemon.pid_file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if let Some(parent) = config.daemon.log_file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if let Some(parent) = config.daemon.socket_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Spawn `slack-code daemon start` as detached process
        let exe = std::env::current_exe()?;
        std::process::Command::new(exe)
            .arg("daemon")
            .arg("start")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .stdin(std::process::Stdio::null())
            .spawn()?;

        // Brief wait for daemon to initialize and create socket
        std::thread::sleep(std::time::Duration::from_millis(500));

        Ok(())
    }

    /// Stop the running daemon
    pub fn stop() -> Result<()> {
        let config = Config::load().unwrap_or_default();

        // Read PID from file
        let pid = Self::read_pid_file(&config.daemon.pid_file)?;

        if let Some(pid) = pid {
            tracing::info!("Sending SIGTERM to daemon (PID: {})", pid);

            match kill(Pid::from_raw(pid), Signal::SIGTERM) {
                Ok(()) => {
                    // Wait for process to exit (with timeout)
                    for _ in 0..50 {
                        // 5 seconds timeout
                        if !Self::is_process_running(pid) {
                            tracing::info!("Daemon stopped successfully");
                            // Clean up PID file
                            let _ = std::fs::remove_file(&config.daemon.pid_file);
                            // Clean up socket
                            let _ = std::fs::remove_file(&config.daemon.socket_path);
                            return Ok(());
                        }
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }

                    // Force kill if still running
                    tracing::warn!("Daemon did not stop gracefully, sending SIGKILL");
                    let _ = kill(Pid::from_raw(pid), Signal::SIGKILL);
                    let _ = std::fs::remove_file(&config.daemon.pid_file);
                    let _ = std::fs::remove_file(&config.daemon.socket_path);
                }
                Err(nix::errno::Errno::ESRCH) => {
                    tracing::info!("Daemon not running (stale PID file)");
                    let _ = std::fs::remove_file(&config.daemon.pid_file);
                }
                Err(e) => {
                    anyhow::bail!("Failed to send signal: {}", e);
                }
            }
        } else {
            tracing::info!("No PID file found, daemon may not be running");
        }

        // Also remove socket file for cleanup
        let _ = std::fs::remove_file(&config.daemon.socket_path);

        Ok(())
    }

    /// Check if daemon is running (async version)
    pub async fn is_running() -> bool {
        Self::is_running_sync()
    }

    /// Check if daemon is running (sync version)
    pub fn is_running_sync() -> bool {
        let config = Config::load().unwrap_or_default();

        // First check PID file
        if let Ok(Some(pid)) = Self::read_pid_file(&config.daemon.pid_file) {
            if Self::is_process_running(pid) {
                // Process exists, also verify socket is reachable
                let client = IpcClient::new(config.daemon.socket_path);
                return client.is_running();
            }
        }

        false
    }

    /// Read PID from file
    fn read_pid_file(path: &std::path::PathBuf) -> Result<Option<i32>> {
        if !path.exists() {
            return Ok(None);
        }

        let mut file = File::open(path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        let pid: i32 = content.trim().parse()?;
        Ok(Some(pid))
    }

    /// Check if a process is running
    fn is_process_running(pid: i32) -> bool {
        // Sending signal 0 checks if process exists without actually sending a signal
        kill(Pid::from_raw(pid), None).is_ok()
    }

    /// Main daemon event loop
    async fn run(self) -> Result<()> {
        tracing::info!("Starting slack-code daemon");

        // Create channels
        let (hook_tx, mut hook_rx) = mpsc::channel::<HookEvent>(100);
        let (command_tx, mut command_rx) = mpsc::channel::<DaemonCommand>(100);
        let (event_tx, _) = broadcast::channel::<DaemonEvent>(100);

        // Initialize Slack service
        let bot_token = self.config.slack.get_bot_token();
        let user_id = self.config.slack.user_id.clone();

        let slack_service = if !bot_token.is_empty() {
            match SlackService::new(&bot_token, user_id) {
                Ok(service) => {
                    tracing::info!("Slack service initialized");
                    Some(Arc::new(RwLock::new(service)))
                }
                Err(e) => {
                    tracing::warn!("Failed to initialize Slack service: {}", e);
                    None
                }
            }
        } else {
            tracing::warn!("Slack bot token not configured");
            None
        };

        // Start IPC server
        let ipc_server = IpcServer::new(
            self.config.daemon.socket_path.clone(),
            hook_tx,
            command_tx,
            event_tx.clone(),
        );

        let _ipc_handle = tokio::spawn(async move {
            if let Err(e) = ipc_server.run().await {
                tracing::error!("IPC server error: {}", e);
            }
        });

        // Set up signal handlers for graceful shutdown
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to create SIGTERM handler");
        let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
            .expect("Failed to create SIGINT handler");

        // Main event loop
        let session_manager = self.session_manager.clone();
        let event_tx_clone = event_tx.clone();

        loop {
            tokio::select! {
                // Handle hook events from Claude Code
                Some(hook_event) = hook_rx.recv() => {
                    tracing::debug!("Received hook event: {:?}", hook_event);

                    let mut manager = session_manager.write().await;
                    if let Some((mut session, status_changed)) = manager.handle_hook_event(hook_event) {
                        // If session has no Slack thread, create one (for external sessions)
                        if session.slack_thread.is_none() {
                            if let Some(ref slack) = slack_service {
                                let mut slack = slack.write().await;
                                match slack.post_session_start(&session).await {
                                    Ok(thread) => {
                                        manager.set_slack_thread(session.id, thread.clone());
                                        session.slack_thread = Some(thread);
                                        tracing::info!("Created Slack thread for session: {}", session.id);
                                    }
                                    Err(e) => {
                                        tracing::warn!("Failed to create Slack thread: {}", e);
                                    }
                                }
                            }
                        } else if status_changed {
                            // Only post status update, if status actually changed.
                            if let Some(ref slack) = slack_service {
                                if let Some(ref thread) = session.slack_thread {
                                    let slack = slack.read().await;
                                    if let Err(e) = slack.post_thread_reply(thread, &session).await {
                                        tracing::warn!("Failed to post to Slack: {}", e);
                                    }
                                }
                            }
                        }

                        // Broadcast session update
                        let _ = event_tx_clone.send(DaemonEvent::SessionUpdated(session.clone()));
                    }
                }

                // Handle TUI commands
                Some(cmd) = command_rx.recv() => {
                    tracing::debug!("Received command: {:?}", cmd);

                    match cmd {
                        DaemonCommand::GetSessions => {
                            let manager = session_manager.read().await;
                            let sessions = manager.get_sessions();
                            let _ = event_tx_clone.send(DaemonEvent::SessionList(sessions));
                        }
                        DaemonCommand::GetConfig => {
                            let _ = event_tx_clone.send(DaemonEvent::ConfigResponse(self.config.clone()));
                        }
                        DaemonCommand::Ping => {
                            let _ = event_tx_clone.send(DaemonEvent::Status(DaemonStatus::Connected));
                        }
                        _ => {}
                    }
                }

                // Handle SIGTERM for graceful shutdown
                _ = sigterm.recv() => {
                    tracing::info!("Received SIGTERM, shutting down gracefully...");
                    break;
                }

                // Handle SIGINT (Ctrl+C) for graceful shutdown
                _ = sigint.recv() => {
                    tracing::info!("Received SIGINT, shutting down gracefully...");
                    break;
                }

                else => {
                    break;
                }
            }
        }

        // Cleanup
        tracing::info!("Daemon shutdown complete");
        Ok(())
    }
}
