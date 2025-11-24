//! Read and inspect action log files from persistence layer
//!
//! Deserializes action log files and displays their contents.

use anyhow::{Context, Result};
use clap::Parser;
use console::style;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;

use game_core::Action;

use crate::dirs;

/// Read and inspect action log files
#[derive(Parser)]
pub struct ReadActions {
    /// Starting nonce of the action batch to read (e.g., 0, 300, 600)
    /// Reads from actions_{nonce:010}.bin file
    #[arg(value_name = "NONCE")]
    nonce: Option<u64>,

    /// Session ID to read actions from (e.g., session_1762685005)
    /// If not provided, uses the most recent session
    #[arg(short, long, value_name = "SESSION")]
    session: Option<String>,

    /// Custom data directory (defaults to platform-specific location)
    #[arg(short, long, value_name = "DIR")]
    data_dir: Option<PathBuf>,

    /// Output format
    #[arg(short, long, value_enum, default_value = "summary")]
    format: OutputFormat,

    /// Limit number of actions to display (0 = unlimited)
    #[arg(short, long, default_value = "100")]
    limit: usize,

    /// Skip first N actions
    #[arg(long, default_value = "0")]
    skip: usize,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum OutputFormat {
    /// Summary view (action types and counts)
    Summary,
    /// List all actions
    List,
    /// Full JSON output
    Json,
    /// Pretty-printed debug format
    Debug,
}

/// Action log entry for deserialization (matches runtime format)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ActionLogEntry {
    nonce: u64,
    action: Action,
}

impl ReadActions {
    pub fn execute(self) -> Result<()> {
        // Determine data directory
        let data_dir = match self.data_dir {
            Some(dir) => dir,
            None => dirs::data_dir()?,
        };

        // Find session directory
        let session_id = match self.session {
            Some(id) => id,
            None => find_latest_session(&data_dir)?,
        };

        let session_dir = data_dir.join(&session_id);
        if !session_dir.exists() {
            anyhow::bail!("Session directory not found: {}", session_dir.display());
        }

        let actions_dir = session_dir.join("actions");
        if !actions_dir.exists() {
            anyhow::bail!("Actions directory not found: {}", actions_dir.display());
        }

        // If nonce is specified, read that specific batch; otherwise read all batches
        let entries = if let Some(nonce) = self.nonce {
            let action_file = actions_dir.join(format!("actions_{:010}.bin", nonce));
            if !action_file.exists() {
                anyhow::bail!("Action file not found: {}", action_file.display());
            }
            read_action_batch(&action_file)?
        } else {
            read_all_action_batches(&actions_dir)?
        };

        // Print header
        println!("{} {}", style("Session:").bold().cyan(), session_id);
        println!(
            "{} {}",
            style("Actions Directory:").bold().cyan(),
            actions_dir.display()
        );
        if let Some(nonce) = self.nonce {
            println!(
                "{} actions_{:010}.bin",
                style("Reading Batch:").bold().cyan(),
                nonce
            );
        } else {
            println!("{} All action batches", style("Reading:").bold().cyan());
        }
        println!(
            "{} {}",
            style("Total Actions:").bold().cyan(),
            entries.len()
        );
        println!();

        // Apply skip/limit
        let entries: Vec<_> = entries
            .into_iter()
            .skip(self.skip)
            .take(if self.limit == 0 {
                usize::MAX
            } else {
                self.limit
            })
            .collect();

        if self.skip > 0 {
            println!("{} {}", style("Skipped:").bold().cyan(), self.skip);
        }
        if self.limit > 0 && entries.len() == self.limit {
            println!("{} {}", style("Showing:").bold().cyan(), entries.len());
            println!();
        }

        // Output based on format
        match self.format {
            OutputFormat::Summary => print_summary(&entries),
            OutputFormat::List => print_list(&entries),
            OutputFormat::Json => print_json(&entries)?,
            OutputFormat::Debug => print_debug(&entries),
        }

        Ok(())
    }
}

fn find_latest_session(data_dir: &PathBuf) -> Result<String> {
    let mut sessions = Vec::new();

    for entry in std::fs::read_dir(data_dir)
        .with_context(|| format!("Failed to read data directory: {}", data_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir()
            && let Some(name) = path.file_name().and_then(|n| n.to_str())
            && name.starts_with("session_")
        {
            let modified = entry.metadata()?.modified()?;
            sessions.push((name.to_string(), modified));
        }
    }

    if sessions.is_empty() {
        anyhow::bail!("No sessions found in data directory");
    }

    // Sort by modification time (newest first)
    sessions.sort_by(|a, b| b.1.cmp(&a.1));

    Ok(sessions[0].0.clone())
}

fn read_action_batch(path: &PathBuf) -> Result<Vec<ActionLogEntry>> {
    let file = File::open(path)
        .with_context(|| format!("Failed to open action batch: {}", path.display()))?;

    let mut reader = BufReader::with_capacity(8 * 1024 * 1024, file);
    let mut entries = Vec::new();

    loop {
        // Read length prefix (4 bytes)
        let mut len_bytes = [0u8; 4];
        match reader.read_exact(&mut len_bytes) {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                // Reached end of file
                break;
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to read length prefix: {}", e));
            }
        }

        let len = u32::from_le_bytes(len_bytes) as usize;

        // Read data
        let mut data = vec![0u8; len];
        reader
            .read_exact(&mut data)
            .with_context(|| "Failed to read entry data")?;

        // Deserialize entry
        let entry: ActionLogEntry = bincode::deserialize(&data)
            .with_context(|| "Failed to deserialize action log entry")?;

        entries.push(entry);
    }

    Ok(entries)
}

fn read_all_action_batches(actions_dir: &PathBuf) -> Result<Vec<ActionLogEntry>> {
    let mut all_entries = Vec::new();
    let mut batch_files = Vec::new();

    // Collect all action batch files
    for entry in std::fs::read_dir(actions_dir).with_context(|| {
        format!(
            "Failed to read actions directory: {}",
            actions_dir.display()
        )
    })? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file()
            && let Some(name) = path.file_name().and_then(|n| n.to_str())
            && name.starts_with("actions_")
            && name.ends_with(".bin")
        {
            batch_files.push(path);
        }
    }

    // Sort batch files by name to ensure correct order
    batch_files.sort();

    // Read all batches
    for batch_file in batch_files {
        let entries = read_action_batch(&batch_file)?;
        all_entries.extend(entries);
    }

    Ok(all_entries)
}

fn print_summary(entries: &[ActionLogEntry]) {
    println!("{}", style("=== Action Summary ===").bold().green());
    println!();

    // Count action types
    let mut counts = std::collections::HashMap::new();
    for entry in entries {
        let action_type = format!("{:?}", entry.action)
            .split('(')
            .next()
            .unwrap()
            .to_string();
        *counts.entry(action_type).or_insert(0) += 1;
    }

    // Sort by count (descending)
    let mut counts: Vec<_> = counts.into_iter().collect();
    counts.sort_by(|a, b| b.1.cmp(&a.1));

    println!("{}", style("Action Type Distribution:").bold().yellow());
    for (action_type, count) in counts {
        println!("  {}: {}", action_type, count);
    }
    println!();

    // Nonce range
    if let (Some(first), Some(last)) = (entries.first(), entries.last()) {
        println!("{}", style("Nonce Range:").bold().yellow());
        println!("  First: {}", first.nonce);
        println!("  Last: {}", last.nonce);
        println!();
    }
}

fn print_list(entries: &[ActionLogEntry]) {
    println!("{}", style("=== Action List ===").bold().green());
    println!();

    for entry in entries {
        println!(
            "{} {}: {:?}",
            style("Nonce").bold(),
            entry.nonce,
            entry.action
        );
    }
}

fn print_json(entries: &[ActionLogEntry]) -> Result<()> {
    let json =
        serde_json::to_string_pretty(entries).context("Failed to serialize actions to JSON")?;
    println!("{}", json);
    Ok(())
}

fn print_debug(entries: &[ActionLogEntry]) {
    for entry in entries {
        println!("{:#?}", entry);
    }
}
