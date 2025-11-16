//! Development tasks for Dungeon project
//!
//! This binary provides development utilities using the cargo-xtask pattern.
//! Run with: `cargo xtask <command>`

mod commands;
mod dirs;

use anyhow::Result;
use clap::Parser;
use commands::{Clean, ReadActions, ReadState, TailLogs};

/// Development tasks for Dungeon project
#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Development tools for Dungeon", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Parser)]
enum Command {
    /// Monitor client logs in real-time
    TailLogs(TailLogs),

    /// Clean save data and logs
    Clean(Clean),

    /// Read and inspect state files
    ReadState(ReadState),

    /// Read and inspect action log files
    ReadActions(ReadActions),
}

fn main() -> Result<()> {
    // Load .env file if it exists (for SAVE_DATA_DIR and other env vars)
    let _ = dotenvy::dotenv();

    let cli = Cli::parse();

    match cli.command {
        Command::TailLogs(cmd) => cmd.execute(),
        Command::Clean(cmd) => cmd.execute(),
        Command::ReadState(cmd) => cmd.execute(),
        Command::ReadActions(cmd) => cmd.execute(),
    }
}
