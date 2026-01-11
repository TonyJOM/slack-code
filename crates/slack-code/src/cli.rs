use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "slack-code")]
#[command(about = "Manage Claude Code sessions via Slack", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run the interactive setup wizard
    Setup,

    /// Manage the background daemon
    Daemon {
        #[command(subcommand)]
        action: DaemonAction,
    },

    /// Manage Claude Code hooks
    Hooks {
        #[command(subcommand)]
        action: HooksAction,
    },
}

#[derive(Subcommand)]
pub enum DaemonAction {
    /// Start the daemon in the background
    Start,
    /// Stop the running daemon
    Stop,
    /// Check daemon status
    Status,
}

#[derive(Subcommand)]
pub enum HooksAction {
    /// Install Claude Code hooks
    Install,
    /// Uninstall Claude Code hooks
    Uninstall,
    /// Check hook installation status
    Status,
}

pub async fn handle_daemon_command(action: DaemonAction) -> Result<()> {
    match action {
        DaemonAction::Start => {
            tracing::info!("Starting daemon...");
            start_daemon().await?;
        }
        DaemonAction::Stop => {
            tracing::info!("Stopping daemon...");
            stop_daemon().await?;
        }
        DaemonAction::Status => {
            check_daemon_status().await?;
        }
    }
    Ok(())
}

pub async fn handle_hooks_command(action: HooksAction) -> Result<()> {
    match action {
        HooksAction::Install => {
            slack_code_common::config::install_hooks()?;
            println!("Hooks installed successfully.");
        }
        HooksAction::Uninstall => {
            slack_code_common::config::uninstall_hooks()?;
            println!("Hooks uninstalled successfully.");
        }
        HooksAction::Status => {
            let installed = slack_code_common::config::check_hooks_installed()?;
            if installed {
                println!("Hooks are installed.");
            } else {
                println!("Hooks are NOT installed.");
            }
        }
    }
    Ok(())
}

pub async fn start_tui() -> Result<()> {
    // Ensure daemon is running
    if !is_daemon_running().await {
        tracing::info!("Starting daemon...");
        if let Err(e) = start_daemon_background().await {
            tracing::warn!("Failed to start daemon: {}", e);
            // Continue anyway - TUI can show daemon disconnected status
        }
    }

    // Run TUI
    let result = slack_code_tui::App::run().await;

    // Always stop daemon when TUI exits
    tracing::info!("Stopping daemon...");
    if let Err(e) = stop_daemon().await {
        tracing::warn!("Failed to stop daemon: {}", e);
    }

    result
}

async fn start_daemon() -> Result<()> {
    // run_foreground is sync - it daemonizes, then creates its own tokio runtime
    slack_code_daemon::Daemon::run_foreground()
}

async fn start_daemon_background() -> Result<()> {
    // start_background is sync (uses daemonize to fork)
    slack_code_daemon::Daemon::start_background()
}

async fn stop_daemon() -> Result<()> {
    // stop is sync (uses nix signals)
    slack_code_daemon::Daemon::stop()
}

async fn check_daemon_status() -> Result<()> {
    if is_daemon_running().await {
        println!("Daemon is running.");
    } else {
        println!("Daemon is NOT running.");
    }
    Ok(())
}

async fn is_daemon_running() -> bool {
    slack_code_daemon::Daemon::is_running().await
}
