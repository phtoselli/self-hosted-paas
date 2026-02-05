mod cli;
mod config;
mod daemon;
mod docker;
mod error;
mod ipc;
mod models;
mod utils;

use clap::Parser;
use cli::commands::{Cli, Commands};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Daemon) => {
            tracing_subscriber::fmt()
                .with_env_filter(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| "info".into()),
                )
                .init();
            daemon::run().await?;
        }
        Some(cmd) => {
            cli::handle_command(cmd).await?;
        }
        None => {
            cli::interactive_menu().await?;
        }
    }

    Ok(())
}
