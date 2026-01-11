use anyhow::Result;
use slack_code_common::ipc::{ClaudeHookInput, HookEvent};
use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

fn main() {
    if let Err(e) = run() {
        // Log errors to stderr but don't fail loudly
        // (hooks shouldn't interrupt Claude Code)
        eprintln!("slack-code-hook error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    // Read JSON from stdin
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    // Parse the hook input
    let hook_input: ClaudeHookInput = serde_json::from_str(&input)?;

    // Convert to our event type
    let Some(event) = hook_input.to_hook_event() else {
        // Unknown event type, silently ignore
        return Ok(());
    };

    // Try to send to daemon
    if let Err(e) = send_to_daemon(&event) {
        // Daemon might not be running - that's okay
        eprintln!("Could not notify daemon: {}", e);
    }

    Ok(())
}

fn send_to_daemon(event: &HookEvent) -> Result<()> {
    let socket_path = get_socket_path();

    // Connect with timeout
    let stream = UnixStream::connect(&socket_path)?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;

    send_message(&stream, event)?;

    Ok(())
}

fn send_message<T: serde::Serialize>(mut stream: &UnixStream, msg: &T) -> Result<()> {
    let json = serde_json::to_string(msg)?;
    let len = (json.len() as u32).to_be_bytes();

    stream.write_all(&len)?;
    stream.write_all(json.as_bytes())?;
    stream.flush()?;

    Ok(())
}

fn get_socket_path() -> PathBuf {
    // Try to load from config, fallback to default
    if let Ok(config) = slack_code_common::Config::load() {
        return config.daemon.socket_path;
    }

    // Default path
    let runtime_dir = dirs::runtime_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap().join(".local/run"));

    runtime_dir.join("slack-code/daemon.sock")
}
