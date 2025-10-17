//! Platform-specific directory utilities
//!
//! Provides consistent directory paths across different operating systems,
//! following platform conventions for cache and data directories.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Get the platform-specific log directory for Dungeon
///
/// Follows platform conventions:
/// - macOS: `~/Library/Caches/dungeon/logs`
/// - Linux: `~/.cache/dungeon/logs` (or `$XDG_CACHE_HOME/dungeon/logs`)
/// - Windows: `%LOCALAPPDATA%\dungeon\logs`
/// - Fallback: `/tmp/dungeon/logs`
pub fn log_dir() -> Result<PathBuf> {
    let base_dir = directories::ProjectDirs::from("", "", "dungeon")
        .map(|dirs| dirs.cache_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("/tmp/dungeon"));

    Ok(base_dir.join("logs"))
}

/// Get the platform-specific data directory for Dungeon
///
/// Follows platform conventions:
/// - macOS: `~/Library/Application Support/dungeon`
/// - Linux: `~/.local/share/dungeon` (or `$XDG_DATA_HOME/dungeon`)
/// - Windows: `%APPDATA%\dungeon`
/// - Fallback: `./save_data`
pub fn data_dir() -> Result<PathBuf> {
    let dir = directories::ProjectDirs::from("", "", "dungeon")
        .map(|dirs| dirs.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("./save_data"));

    Ok(dir)
}

/// List all session directories in the log directory
///
/// Returns a vector of (session_id, path) tuples, sorted by modification time (newest first)
pub fn list_sessions(log_dir: &Path) -> Result<Vec<(String, PathBuf)>> {
    if !log_dir.exists() {
        return Ok(Vec::new());
    }

    let mut sessions: Vec<(String, PathBuf, std::time::SystemTime)> = Vec::new();

    for entry in std::fs::read_dir(log_dir)
        .with_context(|| format!("Failed to read log directory: {}", log_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir()
            && let Some(session_id) = path.file_name().and_then(|n| n.to_str())
        {
            // Get modification time for sorting
            let modified = entry.metadata()?.modified()?;
            sessions.push((session_id.to_string(), path.clone(), modified));
        }
    }

    // Sort by modification time (newest first)
    sessions.sort_by(|a, b| b.2.cmp(&a.2));

    // Return without modification time
    Ok(sessions
        .into_iter()
        .map(|(id, path, _)| (id, path))
        .collect())
}

/// Find the log file for a specific session
pub fn find_session_log(log_dir: &Path, session_id: &str) -> Result<PathBuf> {
    let log_path = log_dir.join(session_id).join("client.log");

    if !log_path.exists() {
        anyhow::bail!("Log file not found: {}", log_path.display());
    }

    Ok(log_path)
}

/// Find the most recent session's log file
pub fn find_latest_log(log_dir: &Path) -> Result<(String, PathBuf)> {
    let sessions = list_sessions(log_dir)?;

    if sessions.is_empty() {
        anyhow::bail!("No sessions found in log directory");
    }

    let (session_id, session_path) = &sessions[0];
    let log_path = session_path.join("client.log");

    if !log_path.exists() {
        anyhow::bail!(
            "Log file not found for latest session: {}",
            log_path.display()
        );
    }

    Ok((session_id.clone(), log_path))
}
