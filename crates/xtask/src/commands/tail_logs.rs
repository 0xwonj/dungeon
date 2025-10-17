//! Tail client logs command
//!
//! Monitors Dungeon client logs in real-time, similar to `tail -f`.
//! Automatically finds the latest session or monitors a specific session.

use anyhow::{Context, Result};
use clap::Parser;
use console::style;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;
use std::time::Duration;

use crate::dirs;

/// Monitor client logs in real-time
#[derive(Parser, Debug)]
pub struct TailLogs {
    /// Specific session ID to monitor (defaults to latest)
    pub session: Option<String>,

    /// Number of lines to show from history before tailing
    #[arg(short = 'n', long, default_value = "10")]
    pub lines: usize,

    /// Poll interval in milliseconds
    #[arg(long, default_value = "100")]
    pub poll_interval: u64,
}

impl TailLogs {
    pub fn execute(self) -> Result<()> {
        let log_dir = dirs::log_dir()?;

        // Check if log directory exists
        if !log_dir.exists() {
            eprintln!("{}", style("✗ Log directory not found").red().bold());
            eprintln!("  Path: {}", style(log_dir.display()).dim());
            eprintln!();
            eprintln!("  Run the client first to generate logs:");
            eprintln!("    {}", style("cargo run -p cli-client").cyan());
            anyhow::bail!("Log directory does not exist");
        }

        // Find the log file to tail
        let (session_id, log_path) = if let Some(ref session) = self.session {
            let path = dirs::find_session_log(&log_dir, session)?;
            (session.clone(), path)
        } else {
            dirs::find_latest_log(&log_dir)
                .context("Failed to find latest log file")?
        };

        // Display header
        println!("{}", style("📝 Monitoring Dungeon Logs").green().bold());
        println!("  Session:  {}", style(&session_id).cyan());
        println!("  Log file: {}", style(log_path.display()).dim());
        println!();

        if self.session.is_none() {
            println!(
                "  {}",
                style("Tip: Specify a session with `--session <id>` to monitor a specific session")
                    .dim()
            );
            println!();
        }

        // Tail the log file
        self.tail_file(&log_path)?;

        Ok(())
    }

    /// Tail a log file, printing the last N lines and then following new content
    fn tail_file(&self, path: &PathBuf) -> Result<()> {
        let mut file = File::open(path)
            .with_context(|| format!("Failed to open log file: {}", path.display()))?;

        // Read last N lines
        let last_lines = self.read_last_n_lines(&mut file, self.lines)?;
        for line in last_lines {
            println!("{}", line);
        }

        // Follow the file for new content
        let mut reader = BufReader::new(file);
        let poll_interval = Duration::from_millis(self.poll_interval);

        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    // No new data, sleep and retry
                    std::thread::sleep(poll_interval);
                }
                Ok(_) => {
                    // New line available, print it
                    print!("{}", line);
                }
                Err(e) => {
                    eprintln!("{}", style(format!("Error reading log file: {}", e)).red());
                    anyhow::bail!("Failed to read log file");
                }
            }
        }
    }

    /// Read the last N lines from a file
    ///
    /// Uses a simple approach: read entire file into memory and take last N lines.
    /// For production use with very large files, consider a more efficient approach.
    fn read_last_n_lines(&self, file: &mut File, n: usize) -> Result<Vec<String>> {
        // Reset to beginning
        file.seek(SeekFrom::Start(0))?;

        let reader = BufReader::new(file);
        let lines: Vec<String> = reader
            .lines()
            .collect::<std::io::Result<Vec<_>>>()
            .context("Failed to read lines from log file")?;

        // Take last N lines
        let start = lines.len().saturating_sub(n);
        Ok(lines[start..].to_vec())
    }
}
