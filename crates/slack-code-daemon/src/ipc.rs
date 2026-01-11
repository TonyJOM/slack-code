use anyhow::Result;
use slack_code_common::ipc::{DaemonCommand, DaemonEvent, HookEvent};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use tokio::net::UnixListener;
use tokio::sync::{broadcast, mpsc};

/// IPC Server for handling connections from hooks and TUI clients
pub struct IpcServer {
    socket_path: PathBuf,
    /// Channel for sending events from hooks to session manager
    hook_tx: mpsc::Sender<HookEvent>,
    /// Channel for sending commands from TUI to daemon
    command_tx: mpsc::Sender<DaemonCommand>,
    /// Broadcast channel for sending events to all TUI subscribers
    event_tx: broadcast::Sender<DaemonEvent>,
}

impl IpcServer {
    pub fn new(
        socket_path: PathBuf,
        hook_tx: mpsc::Sender<HookEvent>,
        command_tx: mpsc::Sender<DaemonCommand>,
        event_tx: broadcast::Sender<DaemonEvent>,
    ) -> Self {
        Self {
            socket_path,
            hook_tx,
            command_tx,
            event_tx,
        }
    }

    /// Start the IPC server
    pub async fn run(self) -> Result<()> {
        // Ensure socket directory exists
        if let Some(parent) = self.socket_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Remove existing socket file
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path)?;
        }

        let listener = UnixListener::bind(&self.socket_path)?;

        tracing::info!("IPC server listening on {:?}", self.socket_path);

        // Accept connections in a loop using native async
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    // Convert tokio UnixStream to std UnixStream for handle_connection
                    let std_stream = match stream.into_std() {
                        Ok(s) => s,
                        Err(e) => {
                            tracing::warn!("Failed to convert stream: {}", e);
                            continue;
                        }
                    };

                    let hook_tx = self.hook_tx.clone();
                    let command_tx = self.command_tx.clone();
                    let event_tx = self.event_tx.clone();

                    // Handle connection in a separate task
                    tokio::spawn(async move {
                        if let Err(e) =
                            Self::handle_connection(std_stream, hook_tx, command_tx, event_tx).await
                        {
                            tracing::warn!("Connection handler error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    tracing::warn!("Accept error: {}", e);
                }
            }
        }
    }

    async fn handle_connection(
        mut stream: UnixStream,
        hook_tx: mpsc::Sender<HookEvent>,
        command_tx: mpsc::Sender<DaemonCommand>,
        event_tx: broadcast::Sender<DaemonEvent>,
    ) -> Result<()> {
        // Set read timeout, but continue without it if it fails (can happen on some platforms)
        if let Err(e) = stream.set_read_timeout(Some(std::time::Duration::from_secs(5))) {
            tracing::debug!("Could not set read timeout: {}, continuing without timeout", e);
        }

        // Read message
        let msg = read_message(&mut stream)?;

        // Try to parse as HookEvent first (from hook binary)
        if let Ok(event) = serde_json::from_str::<HookEvent>(&msg) {
            tracing::debug!("Received hook event: {:?}", event);
            hook_tx.send(event).await?;
            return Ok(());
        }

        // Try to parse as DaemonCommand (from TUI)
        if let Ok(cmd) = serde_json::from_str::<DaemonCommand>(&msg) {
            tracing::debug!("Received command: {:?}", cmd);

            match cmd {
                DaemonCommand::Subscribe => {
                    // Subscribe this connection to events
                    let mut rx = event_tx.subscribe();

                    // Keep connection open and forward events
                    if let Err(e) = stream.set_write_timeout(Some(std::time::Duration::from_secs(5))) {
                        tracing::debug!("Could not set write timeout: {}, continuing without timeout", e);
                    }

                    loop {
                        match rx.recv().await {
                            Ok(event) => {
                                if let Err(e) = write_message(&mut stream, &event) {
                                    tracing::debug!("Subscriber disconnected: {}", e);
                                    break;
                                }
                            }
                            Err(broadcast::error::RecvError::Closed) => break,
                            Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        }
                    }
                }
                _ => {
                    // Forward command to daemon
                    command_tx.send(cmd).await?;
                }
            }
        }

        Ok(())
    }
}

/// Read a length-prefixed message from a stream
fn read_message(stream: &mut UnixStream) -> Result<String> {
    // Read 4-byte length prefix
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf)?;
    let len = u32::from_be_bytes(len_buf) as usize;

    // Read message body
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf)?;

    Ok(String::from_utf8(buf)?)
}

/// Write a length-prefixed message to a stream
fn write_message<T: serde::Serialize>(stream: &mut UnixStream, msg: &T) -> Result<()> {
    let json = serde_json::to_string(msg)?;
    let len = (json.len() as u32).to_be_bytes();

    stream.write_all(&len)?;
    stream.write_all(json.as_bytes())?;
    stream.flush()?;

    Ok(())
}

/// IPC Client for connecting to the daemon
pub struct IpcClient {
    socket_path: PathBuf,
}

impl IpcClient {
    pub fn new(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }

    /// Connect to the daemon
    pub fn connect(&self) -> Result<UnixStream> {
        let stream = UnixStream::connect(&self.socket_path)?;
        stream.set_read_timeout(Some(std::time::Duration::from_secs(30)))?;
        stream.set_write_timeout(Some(std::time::Duration::from_secs(5)))?;
        Ok(stream)
    }

    /// Send a command to the daemon
    pub fn send_command(&self, cmd: &DaemonCommand) -> Result<()> {
        let mut stream = self.connect()?;
        write_message(&mut stream, cmd)?;
        Ok(())
    }

    /// Subscribe to daemon events
    pub fn subscribe(&self) -> Result<EventSubscription> {
        let mut stream = self.connect()?;
        write_message(&mut stream, &DaemonCommand::Subscribe)?;
        Ok(EventSubscription { stream })
    }

    /// Check if daemon is running
    pub fn is_running(&self) -> bool {
        self.connect().is_ok()
    }
}

/// Subscription to daemon events
pub struct EventSubscription {
    stream: UnixStream,
}

impl EventSubscription {
    /// Receive the next event (blocking)
    pub fn recv(&mut self) -> Result<DaemonEvent> {
        let msg = read_message(&mut self.stream)?;
        let event = serde_json::from_str(&msg)?;
        Ok(event)
    }

    /// Non-blocking receive - returns Ok(None) if no event ready
    pub fn try_recv(&mut self) -> Result<Option<DaemonEvent>> {
        // Set socket to non-blocking temporarily
        self.stream.set_nonblocking(true)?;

        let result = match read_message(&mut self.stream) {
            Ok(json) => {
                let event: DaemonEvent = serde_json::from_str(&json)?;
                Ok(Some(event))
            }
            Err(e) => {
                // Check if it's a WouldBlock error (no data available)
                if let Some(io_err) = e.downcast_ref::<std::io::Error>() {
                    if io_err.kind() == std::io::ErrorKind::WouldBlock {
                        return Ok(None);
                    }
                }
                Err(e)
            }
        };

        // Restore blocking mode
        let _ = self.stream.set_nonblocking(false);
        result
    }
}
