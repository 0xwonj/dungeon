//! Action log entry type for proof generation.
//!
//! This module defines the `ActionLogEntry` type, which is the dedicated format
//! for storing executed actions in the action log files. This is separate from
//! the general event log to optimize proof generation.
//!
//! # Design: Action-Only Format
//!
//! Each entry stores only the nonce and action. No state is stored in action log entries.
//! State is managed separately:
//! - ActionBatch references start_nonce and end_nonce
//! - StateRepository stores GameState at those nonces
//! - ProverWorker loads start state and replays actions to generate proof
//!
//! This reduces action log file size dramatically (from ~5MB per action to ~100 bytes).
//!
//! # Format
//!
//! Each entry is serialized using bincode and stored with a length prefix:
//! ```text
//! [u32 length][bincode serialized ActionLogEntry]
//! ```

use serde::{Deserialize, Serialize};

use game_core::Action;

/// Action log entry for proof generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionLogEntry {
    /// Sequential action nonce
    pub nonce: u64,

    /// The action that was executed
    pub action: Action,
}

impl ActionLogEntry {
    /// Create a new action log entry.
    pub fn new(nonce: u64, action: Action) -> Self {
        Self { nonce, action }
    }
}
