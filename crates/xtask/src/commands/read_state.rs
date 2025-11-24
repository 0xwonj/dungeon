//! Read and inspect state files from persistence layer
//!
//! Deserializes state_{nonce}.bin files and displays their contents.

use anyhow::{Context, Result};
use clap::Parser;
use console::style;
use std::path::PathBuf;

use game_core::GameState;

use crate::dirs;

/// Read and inspect state files
#[derive(Parser)]
pub struct ReadState {
    /// Nonce of the state file to read (e.g., 0, 299, 599)
    #[arg(value_name = "NONCE")]
    nonce: u64,

    /// Session ID to read from (e.g., session_1763197900)
    /// If not provided, uses the most recent session
    #[arg(short, long, value_name = "SESSION")]
    session: Option<String>,

    /// Custom data directory (defaults to platform-specific location)
    #[arg(short, long, value_name = "DIR")]
    data_dir: Option<PathBuf>,

    /// Output format
    #[arg(short, long, value_enum, default_value = "summary")]
    format: OutputFormat,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum OutputFormat {
    /// Summary view (entities count, turn info, basic stats)
    Summary,
    /// Full JSON output
    Json,
    /// Pretty-printed debug format
    Debug,
}

impl ReadState {
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

        let state_file = session_dir
            .join("states")
            .join(format!("state_{}.bin", self.nonce));

        if !state_file.exists() {
            anyhow::bail!(
                "State file not found: {}\n\nHint: Check available states in {}",
                state_file.display(),
                session_dir.join("states").display()
            );
        }

        // Read and deserialize state file
        let bytes = std::fs::read(&state_file)
            .with_context(|| format!("Failed to read state file: {}", state_file.display()))?;

        let state: GameState = bincode::deserialize(&bytes).with_context(|| {
            format!("Failed to deserialize state file: {}", state_file.display())
        })?;

        // Print header
        println!(
            "{} {}",
            style("State File:").bold().cyan(),
            state_file.display()
        );
        println!(
            "{} {}",
            style("File Size:").bold().cyan(),
            format_bytes(bytes.len())
        );
        println!("{} {}", style("Nonce:").bold().cyan(), state.nonce());
        println!();

        // Output based on format
        match self.format {
            OutputFormat::Summary => print_summary(&state),
            OutputFormat::Json => print_json(&state)?,
            OutputFormat::Debug => print_debug(&state),
        }

        Ok(())
    }
}

fn print_summary(state: &GameState) {
    println!("{}", style("=== Game State Summary ===").bold().green());
    println!();

    // Turn info
    println!("{}", style("Turn Information:").bold().yellow());
    println!("  Nonce: {}", state.turn.nonce);
    println!("  Clock: {}", state.turn.clock);
    println!("  Current Actor: {:?}", state.turn.current_actor);
    println!("  Active Actors: {}", state.turn.active_actors.len());
    println!();

    // Entities
    println!("{}", style("Entities:").bold().yellow());
    println!("  Total Actors: {}", state.entities.actors.len());
    println!("  Total Props: {}", state.entities.props.len());
    println!("  Total Items: {}", state.entities.items.len());
    println!();

    // Actor details
    if !state.entities.actors.is_empty() {
        println!("{}", style("Actors:").bold().yellow());
        for actor in state.entities.all_actors() {
            let snapshot = actor.snapshot();
            println!(
                "  Actor (ID: {}) - HP: {}/{}, Position: {:?}",
                actor.id.0, actor.resources.hp, snapshot.resource_max.hp_max, actor.position
            );
        }
        println!();
    }

    // World info
    println!("{}", style("World:").bold().yellow());
    let total_occupants = state.world.tile_map.occupancy().len();
    println!("  Total Occupied Tiles: {}", total_occupants);
    println!();
}

fn print_json(state: &GameState) -> Result<()> {
    let json = serde_json::to_string_pretty(state).context("Failed to serialize state to JSON")?;
    println!("{}", json);
    Ok(())
}

fn print_debug(state: &GameState) {
    println!("{:#?}", state);
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

fn format_bytes(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = KB * 1024;

    if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}
