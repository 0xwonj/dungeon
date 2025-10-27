//! Repository contracts for saving and loading mutable runtime state.

use game_core::GameState;

use crate::api::Result;
use crate::events::Event;

// Re-export shared types
pub use super::types::{ActionLogEntry, Checkpoint, ProofIndex};

/// Repository for game state persistence and loading
///
/// This is for DYNAMIC data that changes during gameplay:
/// - Save/Load game state indexed by nonce
/// - State snapshots for rollback
pub trait StateRepository: Send + Sync {
    /// Save a game state indexed by nonce
    fn save(&self, nonce: u64, state: &GameState) -> Result<()>;

    /// Load a game state by nonce
    fn load(&self, nonce: u64) -> Result<Option<GameState>>;

    /// Check if a state exists
    fn exists(&self, nonce: u64) -> bool;

    /// Delete a state
    fn delete(&self, nonce: u64) -> Result<()>;

    /// List all available state nonces
    fn list_nonces(&self) -> Result<Vec<u64>> {
        Ok(vec![])
    }

    /// Delete all states in a range [start, end]
    fn delete_range(&self, start: u64, end: u64) -> Result<usize> {
        let mut deleted = 0;
        for nonce in start..=end {
            if self.exists(nonce) {
                self.delete(nonce)?;
                deleted += 1;
            }
        }
        Ok(deleted)
    }
}

/// Repository for checkpoint persistence
///
/// Checkpoints store lightweight metadata + indices to external data:
/// - State references (state hash, persisted status)
/// - Action log offsets (for replay/recovery)
/// - Proof references (optional, verified status)
///
/// # Storage Model
///
/// Each checkpoint is stored as an individual file indexed by (session_id, nonce):
/// ```text
/// checkpoint_{session}_nonce_{nonce:010}.json
/// ```
///
/// This enables:
/// - Multiple checkpoints per session (save history)
/// - Easy CRUD operations (load specific nonce or latest)
/// - Selective cleanup (delete old checkpoints)
pub trait CheckpointRepository: Send + Sync {
    /// Save a checkpoint (indexed by session_id + nonce)
    fn save(&self, checkpoint: &Checkpoint) -> Result<()>;

    /// Load the latest checkpoint for a session
    fn load_latest(&self, session_id: &str) -> Result<Option<Checkpoint>>;

    /// Load a checkpoint at a specific nonce
    fn load_at_nonce(&self, session_id: &str, nonce: u64) -> Result<Option<Checkpoint>>;

    /// List all checkpoint nonces for a session (sorted ascending)
    fn list_checkpoints(&self, session_id: &str) -> Result<Vec<u64>>;

    /// Delete a specific checkpoint
    fn delete(&self, session_id: &str, nonce: u64) -> Result<()>;

    /// Delete all checkpoints for a session
    fn delete_all(&self, session_id: &str) -> Result<usize>;

    /// Delete checkpoints before a specific nonce (cleanup old saves)
    fn delete_before(&self, session_id: &str, before_nonce: u64) -> Result<usize>;

    /// List all checkpoint sessions
    fn list_sessions(&self) -> Result<Vec<String>>;
}

/// Repository for event log persistence
///
/// Provides append-only event logging for the complete event timeline.
/// This includes ActionRef entries (references to actions.log) as well as
/// other events like Turn, Proof, etc.
pub trait EventRepository: Send + Sync {
    /// Append an event to the log
    ///
    /// Returns the byte offset where the event was written.
    fn append(&mut self, event: &Event) -> Result<u64>;

    /// Read an event at a specific byte offset
    ///
    /// Returns `None` if the offset is beyond the end of the log.
    /// Returns `Some((event, next_offset))` where next_offset is the byte position after this entry.
    fn read_at_offset(&self, byte_offset: u64) -> Result<Option<(Event, u64)>>;

    /// Flush buffered writes to disk
    fn flush(&mut self) -> Result<()>;

    /// Get the current size of the log in bytes
    fn size(&self) -> Result<u64>;

    /// Get the session ID associated with this log
    fn session_id(&self) -> &str;
}

/// Write-only repository for action log persistence.
///
/// Provides append-only logging specifically for executed actions.
/// This stores the full ActionLogEntry data needed for proof generation,
/// separate from the general event timeline.
///
/// # Purpose
///
/// The action log writer is optimized for proof generation:
/// - Sequential write-only access by SimulationWorker
/// - Contains all data needed for zkVM (before_state, after_state, action)
/// - No filtering required (only ActionExecuted entries)
/// - Reading is handled by ActionLogReader trait (separate responsibility)
///
/// # File Format
///
/// Each entry is stored as:
/// ```text
/// [u32 length][bincode serialized ActionLogEntry]
/// ```
///
/// # Naming
///
/// This trait is named `ActionLogWriter` for symmetry with `ActionLogReader`.
/// Together they provide complete read/write access to action logs while
/// maintaining clear separation of concerns.
pub trait ActionLogWriter: Send + Sync {
    /// Append an action log entry
    ///
    /// Returns the byte offset where the entry was written.
    fn append(&mut self, entry: &ActionLogEntry) -> Result<u64>;

    /// Flush buffered writes to disk
    fn flush(&mut self) -> Result<()>;

    /// Get the current size of the log in bytes
    fn size(&self) -> Result<u64>;

    /// Get the session ID associated with this log
    fn session_id(&self) -> &str;
}

/// Repository for proof index persistence
///
/// Stores metadata about generated ZK proofs, indexed by session ID.
/// This allows efficient lookup of proof status and metadata without
/// loading the full proof files.
pub trait ProofIndexRepository: Send + Sync {
    /// Save the proof index
    fn save(&self, index: &ProofIndex) -> Result<()>;

    /// Load the proof index by session ID
    fn load(&self, session_id: &str) -> Result<Option<ProofIndex>>;

    /// Delete a proof index
    fn delete(&self, session_id: &str) -> Result<()>;

    /// List all proof index sessions
    fn list_sessions(&self) -> Result<Vec<String>>;
}

/// Sequential action log reader for proof generation.
///
/// This trait abstracts the sequential reading of action log entries,
/// allowing different implementations (memory-mapped files, in-memory, etc.)
/// while providing a consistent interface for ProverWorker.
///
/// # Design
///
/// - Read-only sequential access (no random access needed)
/// - Optimized for streaming consumption by ProverWorker
/// - Supports file growth detection for continuous operation
/// - Supports checkpoint/resume via seek()
///
/// # Implementations
///
/// - `MmapActionLogReader`: Zero-copy memory-mapped file reader (production)
/// - `InMemoryActionLogReader`: In-memory reader for testing
pub trait ActionLogReader: Send + Sync {
    /// Read the next action log entry from the current position.
    ///
    /// Returns:
    /// - `Ok(Some(entry))` - Successfully read next entry
    /// - `Ok(None)` - Reached end of log (caught up with writer)
    /// - `Err(_)` - Read error occurred
    ///
    /// This method advances the internal read position on success.
    fn read_next(&self) -> Result<Option<ActionLogEntry>>;

    /// Refresh the reader and check if new data is available.
    ///
    /// For file-based implementations, this checks if the file has grown
    /// and updates internal state (e.g., remapping) if necessary.
    ///
    /// Returns:
    /// - `Ok(true)` - New data is available
    /// - `Ok(false)` - No new data
    /// - `Err(_)` - Refresh failed
    fn refresh(&self) -> Result<bool>;

    /// Get the current read offset in bytes.
    ///
    /// Useful for checkpointing and resuming proof generation.
    fn current_offset(&self) -> u64;

    /// Get the session ID associated with this log.
    fn session_id(&self) -> &str;

    /// Check if there's more data available to read without actually reading.
    ///
    /// Returns `true` if `read_next()` would likely return `Some(_)`.
    fn has_more(&self) -> bool {
        // Default implementation - can be overridden for optimization
        true
    }

    /// Seek to a specific byte offset in the log.
    ///
    /// This is useful for resuming proof generation from a checkpoint.
    /// The offset should be a valid entry boundary (as returned by `current_offset()`).
    ///
    /// # Arguments
    ///
    /// * `offset` - The byte offset to seek to (must be <= log size)
    ///
    /// # Errors
    ///
    /// Returns error if the offset is invalid (beyond end of log).
    ///
    /// # Notes
    ///
    /// - Does not validate that offset points to an entry boundary
    /// - Caller must ensure offset is from a valid checkpoint
    fn seek(&self, offset: u64) -> Result<()>;
}
