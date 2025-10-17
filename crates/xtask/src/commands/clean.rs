//! Clean save data and logs command
//!
//! Provides utilities to clean up Dungeon's persistent data:
//! - Logs (cache directory)
//! - Save data (data directory)
//!
//! Safety: Always prompts for confirmation before deletion.

use anyhow::{Context, Result};
use clap::Parser;
use console::style;
use std::io::{self, Write};

use crate::dirs;

/// Clean save data and logs
#[derive(Parser, Debug)]
pub struct Clean {
    /// Clean only logs (cache directory)
    #[arg(long)]
    pub logs: bool,

    /// Clean only save data (data directory)
    #[arg(long)]
    pub data: bool,

    /// Skip confirmation prompt (dangerous!)
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// Specific session to clean (only works with --logs)
    #[arg(long)]
    pub session: Option<String>,
}

impl Clean {
    pub fn execute(self) -> Result<()> {
        // If no flags specified, clean both
        let clean_logs = self.logs || !self.data;
        let clean_data = self.data || !self.logs;

        // Validate session flag
        if self.session.is_some() && !self.logs {
            anyhow::bail!("--session can only be used with --logs");
        }

        // Collect directories to clean
        let mut targets = Vec::new();

        if clean_logs {
            let log_dir = dirs::log_dir()?;
            if let Some(ref session_id) = self.session {
                // Clean specific session
                let session_dir = log_dir.join(session_id);
                if session_dir.exists() {
                    targets.push((format!("Session logs ({})", session_id), session_dir));
                } else {
                    eprintln!(
                        "{} Session not found: {}",
                        style("✗").red().bold(),
                        style(session_id).cyan()
                    );
                    anyhow::bail!("Session directory does not exist");
                }
            } else if log_dir.exists() {
                targets.push(("All logs".to_string(), log_dir));
            }
        }

        if clean_data {
            let data_dir = dirs::data_dir()?;
            if data_dir.exists() {
                targets.push(("Save data".to_string(), data_dir));
            }
        }

        if targets.is_empty() {
            println!(
                "{}",
                style("Nothing to clean - directories don't exist yet").dim()
            );
            return Ok(());
        }

        // Display what will be cleaned
        println!("{}", style("🧹 Clean Dungeon Data").yellow().bold());
        println!();
        println!("The following will be deleted:");
        for (label, path) in &targets {
            println!("  {} {}", style("→").cyan(), style(label).bold());
            println!("    {}", style(path.display()).dim());
        }
        println!();

        // Confirm deletion
        if !self.yes && !self.confirm()? {
            println!("{}", style("Cancelled").dim());
            return Ok(());
        }

        // Perform deletion
        for (label, path) in targets {
            print!("Deleting {}... ", label);
            io::stdout().flush()?;

            std::fs::remove_dir_all(&path)
                .with_context(|| format!("Failed to delete: {}", path.display()))?;

            println!("{}", style("✓").green());
        }

        println!();
        println!("{}", style("✓ Cleanup complete!").green().bold());

        Ok(())
    }

    /// Prompt user for confirmation
    fn confirm(&self) -> Result<bool> {
        print!("{} ", style("Proceed? [y/N]").yellow().bold());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let input = input.trim().to_lowercase();
        Ok(input == "y" || input == "yes")
    }
}
