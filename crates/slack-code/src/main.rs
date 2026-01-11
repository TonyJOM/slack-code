mod cli;
mod setup;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands, DaemonAction};

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    // Handle daemon start specially - before tokio runtime is created
    // This is because daemonize forks the process, and we need to create
    // a new tokio runtime in the child process
    if let Some(Commands::Daemon { action: DaemonAction::Start }) = &cli.command {
        tracing::info!("Starting daemon...");
        return slack_code_daemon::Daemon::run_foreground();
    }

    // All other commands use the tokio runtime
    tokio::runtime::Runtime::new()?.block_on(async_main(cli))
}

async fn async_main(cli: Cli) -> Result<()> {
    match cli.command {
        Some(Commands::Setup) => {
            setup::run_setup_wizard().await?;
        }
        Some(Commands::Daemon { action }) => {
            cli::handle_daemon_command(action).await?;
        }
        Some(Commands::Hooks { action }) => {
            cli::handle_hooks_command(action).await?;
        }
        None => {
            // Default: start TUI (which also starts daemon if needed)
            cli::start_tui().await?;
        }
    }

    Ok(())
}
