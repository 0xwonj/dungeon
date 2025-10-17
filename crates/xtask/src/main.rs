//! Development tasks for Dungeon project
//!
//! This binary provides development utilities using the cargo-xtask pattern.
//! Run with: `cargo xtask <command>`

mod commands;
mod dirs;

use anyhow::Result;
use clap::Parser;
use commands::{Clean, TailLogs};

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
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::TailLogs(cmd) => cmd.execute(),
        Command::Clean(cmd) => cmd.execute(),
    }
}
