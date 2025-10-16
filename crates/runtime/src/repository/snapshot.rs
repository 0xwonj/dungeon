//! Snapshot service for coordinated state persistence.
//!
//! Provides high-level operations that coordinate between repositories
//! using the 2-phase commit pattern.

use std::path::Path;

use game_core::GameState;

use crate::api::Result;
use crate::events::Topic;
use crate::repository::{
    Checkpoint, CheckpointRepository, FileCheckpointRepository, FileStateRepository,
    StateRepository,
};

/// Service for managing game state snapshots with 2-phase commit.
///
/// # Design Pattern: Facade + 2-Phase Commit
///
/// Coordinates StateRepository and CheckpointRepository to ensure consistency:
/// 1. Save GameState to StateRepository (data layer)
/// 2. Save Checkpoint to CheckpointRepository (commit point)
///
/// If checkpoint exists, the referenced state is guaranteed to exist.
pub struct SnapshotService {
    state_repo: Box<dyn StateRepository>,
    checkpoint_repo: Box<dyn CheckpointRepository>,
}

impl SnapshotService {
    /// Create a new snapshot service with custom repositories.
    pub fn new(
        state_repo: Box<dyn StateRepository>,
        checkpoint_repo: Box<dyn CheckpointRepository>,
    ) -> Self {
        Self {
            state_repo,
            checkpoint_repo,
        }
    }

    /// Create a file-based snapshot service.
    pub fn new_file_based(base_dir: impl AsRef<Path>) -> Result<Self> {
        let base_path = base_dir.as_ref();

        let state_repo = Box::new(FileStateRepository::new(base_path.join("states"))?);
        let checkpoint_repo = Box::new(FileCheckpointRepository::new(
            base_path.join("checkpoints"),
        )?);

        Ok(Self {
            state_repo,
            checkpoint_repo,
        })
    }

    /// Save a complete snapshot with 2-phase commit.
    ///
    /// # Arguments
    ///
    /// * `session_id` - Session identifier
    /// * `state` - Current game state to save
    /// * `action_count` - Total number of actions executed
    /// * `topic_offsets` - Event log offsets (optional)
    ///
    /// # Returns
    ///
    /// The nonce of the saved state.
    pub fn save_snapshot(
        &self,
        session_id: String,
        state: &GameState,
        action_count: u64,
        topic_offsets: Option<std::collections::HashMap<Topic, u64>>,
    ) -> Result<u64> {
        let nonce = action_count; // Use action_count as nonce
        let state_hash = self.compute_state_hash(state);

        // Phase 1: Save state data
        self.state_repo.save(nonce, state)?;

        // Phase 2: Save checkpoint (commit point)
        let mut checkpoint =
            Checkpoint::with_state(session_id, nonce, state_hash, true, action_count);

        if let Some(offsets) = topic_offsets {
            checkpoint.event_ref.topic_offsets = offsets;
        }

        self.checkpoint_repo.save(&checkpoint)?;

        tracing::info!(
            "Saved snapshot: session={}, nonce={}",
            checkpoint.session_id,
            nonce
        );

        Ok(nonce)
    }

    /// Load a game state from a checkpoint.
    ///
    /// Returns `None` if checkpoint or state doesn't exist.
    pub fn load_snapshot(&self, session_id: &str) -> Result<Option<(GameState, Checkpoint)>> {
        // Load checkpoint first
        let Some(checkpoint) = self.checkpoint_repo.load(session_id)? else {
            return Ok(None);
        };

        // Check if state exists
        if !checkpoint.has_full_state() {
            tracing::warn!(
                "Checkpoint exists but no full state: session={}, nonce={}",
                session_id,
                checkpoint.nonce
            );
            return Ok(None);
        }

        // Load state
        let Some(state) = self.state_repo.load(checkpoint.nonce)? else {
            tracing::error!(
                "Checkpoint references missing state: session={}, nonce={}",
                session_id,
                checkpoint.nonce
            );
            return Ok(None);
        };

        Ok(Some((state, checkpoint)))
    }

    /// Get checkpoint without loading full state (fast).
    pub fn get_checkpoint(&self, session_id: &str) -> Result<Option<Checkpoint>> {
        self.checkpoint_repo.load(session_id)
    }

    /// List all available sessions.
    pub fn list_sessions(&self) -> Result<Vec<String>> {
        self.checkpoint_repo.list_sessions()
    }

    /// Delete a snapshot (checkpoint + state).
    pub fn delete_snapshot(&self, session_id: &str) -> Result<()> {
        // Load checkpoint to get nonce
        if let Some(checkpoint) = self.checkpoint_repo.load(session_id)? {
            // Delete state if it exists
            if checkpoint.has_full_state() {
                self.state_repo.delete(checkpoint.nonce)?;
            }

            // Delete checkpoint
            self.checkpoint_repo.delete(session_id)?;

            tracing::info!(
                "Deleted snapshot: session={}, nonce={}",
                session_id,
                checkpoint.nonce
            );
        }

        Ok(())
    }

    /// Compute a hash of the game state for verification.
    ///
    /// Currently uses a placeholder. In production, use a proper hash function.
    fn compute_state_hash(&self, state: &GameState) -> String {
        // Placeholder: In production, use blake3 or similar
        format!("state_{}", state.turn.clock)
    }

    /// Access the underlying state repository (for advanced operations).
    pub fn state_repo(&self) -> &dyn StateRepository {
        &*self.state_repo
    }

    /// Access the underlying checkpoint repository (for advanced operations).
    pub fn checkpoint_repo(&self) -> &dyn CheckpointRepository {
        &*self.checkpoint_repo
    }
}
