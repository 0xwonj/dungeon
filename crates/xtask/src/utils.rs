//! Utility functions for xtask commands

use anyhow::{Context, Result, anyhow};
use std::path::PathBuf;

/// Get the data directory for save files
pub fn data_dir() -> Result<PathBuf> {
    crate::dirs::data_dir().context("Failed to determine data directory")
}

/// Find the latest session directory
pub fn find_latest_session() -> Result<String> {
    let data_dir = data_dir()?;

    if !data_dir.exists() {
        return Err(anyhow!(
            "Data directory does not exist: {:?}\n\
             No sessions found. Run the client first to create a session.",
            data_dir
        ));
    }

    // List all session directories and find the latest one
    let mut sessions: Vec<_> = std::fs::read_dir(&data_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .file_type()
                .ok()
                .map(|ft| ft.is_dir())
                .unwrap_or(false)
        })
        .filter_map(|entry| {
            let name = entry.file_name().to_str()?.to_string();
            // Extract timestamp from session_<timestamp> format
            let timestamp = if let Some(stripped) = name.strip_prefix("session_") {
                stripped.parse::<u64>().ok()?
            } else {
                // Also support plain numeric session IDs for backwards compatibility
                name.parse::<u64>().ok()?
            };
            Some((timestamp, name))
        })
        .collect();

    if sessions.is_empty() {
        return Err(anyhow!(
            "No session directories found in: {:?}\n\
             Run the client first to create a session.",
            data_dir
        ));
    }

    // Sort by timestamp (descending) to get the latest
    sessions.sort_by(|a, b| b.0.cmp(&a.0));

    let (_, session_name) = &sessions[0];
    Ok(session_name.clone())
}
