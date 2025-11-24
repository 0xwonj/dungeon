//! Session management utilities for resuming games.

use anyhow::{Context, Result, anyhow};
use std::path::Path;

use game_core::GameState;
use runtime::StateRepository;

/// Information about a saved session.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionInfo {
    /// Session directory name (e.g., "session_1234567890")
    pub session_id: String,

    /// Session timestamp extracted from directory name
    pub timestamp: u64,

    /// Latest state nonce available in this session
    pub latest_nonce: u64,
}

/// List all session directories in the save data directory.
///
/// Sessions are identified by directories matching "session_<timestamp>" format.
pub fn list_sessions(base_dir: &Path) -> Result<Vec<SessionInfo>> {
    if !base_dir.exists() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();

    for entry in std::fs::read_dir(base_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let dir_name = entry.file_name();
        let name = dir_name
            .to_str()
            .context("Invalid UTF-8 in directory name")?;

        // Extract timestamp from session_<timestamp> format
        let timestamp = if let Some(stripped) = name.strip_prefix("session_") {
            stripped.parse::<u64>().ok()
        } else {
            // Also support plain numeric session IDs for backwards compatibility
            name.parse::<u64>().ok()
        };

        if let Some(timestamp) = timestamp {
            // Find latest nonce in this session
            let session_dir = entry.path();
            let states_dir = session_dir.join("states");

            let latest_nonce = find_highest_state_nonce(&states_dir).unwrap_or(0);

            sessions.push(SessionInfo {
                session_id: name.to_string(),
                timestamp,
                latest_nonce,
            });
        }
    }

    // Sort by timestamp (descending) - most recent first
    sessions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    Ok(sessions)
}

/// Find the most recent session by timestamp.
pub fn find_latest_session(base_dir: &Path) -> Result<Option<SessionInfo>> {
    let sessions = list_sessions(base_dir)?;
    Ok(sessions.into_iter().next())
}

/// Find the highest nonce state file in a states directory.
fn find_highest_state_nonce(states_dir: &Path) -> Option<u64> {
    if !states_dir.exists() {
        return None;
    }

    let mut max_nonce = None;

    for entry in std::fs::read_dir(states_dir).ok()? {
        let entry = entry.ok()?;
        let filename = entry.file_name();
        let name = filename.to_str()?;

        // Files are named: state_<nonce>.bin
        if let Some(stripped) = name
            .strip_prefix("state_")
            .and_then(|s| s.strip_suffix(".bin"))
            && let Ok(nonce) = stripped.parse::<u64>()
        {
            max_nonce = Some(max_nonce.map_or(nonce, |n: u64| n.max(nonce)));
        }
    }

    max_nonce
}

/// Load the latest state from a session directory.
///
/// Returns the highest nonce state available, or None if no states exist.
pub fn load_latest_state(base_dir: &Path, session_id: &str) -> Result<Option<(u64, GameState)>> {
    let states_dir = base_dir.join(session_id).join("states");

    let nonce = match find_highest_state_nonce(&states_dir) {
        Some(n) => n,
        None => return Ok(None),
    };

    // Use FileStateRepository to load the state
    let state_repo = runtime::FileStateRepository::new(states_dir)?;
    let state = state_repo
        .load(nonce)?
        .ok_or_else(|| anyhow!("State file exists but failed to load: state_{}.bin", nonce))?;

    Ok(Some((nonce, state)))
}
