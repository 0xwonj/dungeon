//! Repository contracts for saving and loading mutable runtime state.

use game_core::GameState;

use crate::events::Event;

use super::error::RepositoryError;

type Result<T> = std::result::Result<T, RepositoryError>;

// Re-export shared types
pub use super::types::ActionLogEntry;

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
/// - `FileActionLogReader`: Buffered file reader for sequential reads
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

/// Repository for action batch tracking and management.
///
/// Action batches represent logical groups of actions bounded by checkpoints.
/// This repository tracks the lifecycle of each batch through various stages:
/// - InProgress → Complete (PersistenceWorker)
/// - Complete → Proving → Proven (ProverWorker)
/// - Proven → OnChain (OnchainWorker)
///
/// Batches are identified by their start_nonce, which is known when the batch is created
/// and remains constant throughout its lifecycle.
pub trait ActionBatchRepository: Send + Sync {
    /// Save or update an action batch.
    fn save(&self, batch: &super::types::ActionBatch) -> Result<()>;

    /// Load an action batch by its start nonce.
    ///
    /// The start nonce uniquely identifies a batch within a session.
    fn load(&self, session_id: &str, start_nonce: u64)
    -> Result<Option<super::types::ActionBatch>>;

    /// List all batches for a session.
    fn list(&self, session_id: &str) -> Result<Vec<super::types::ActionBatch>>;

    /// List batches by status.
    fn list_by_status(
        &self,
        session_id: &str,
        status: super::types::ActionBatchStatus,
    ) -> Result<Vec<super::types::ActionBatch>>;

    /// Delete a batch.
    fn delete(&self, session_id: &str, start_nonce: u64) -> Result<()>;

    /// Get the current in-progress batch (if any).
    fn get_current_batch(&self, session_id: &str) -> Result<Option<super::types::ActionBatch>> {
        let batches =
            self.list_by_status(session_id, super::types::ActionBatchStatus::InProgress)?;
        Ok(batches.into_iter().next())
    }
}
