//! Repository contracts for saving and loading mutable runtime state.
use game_core::GameState;

use crate::api::Result;

/// Repository for game state persistence and loading
///
/// This is for DYNAMIC data that changes during gameplay:
/// - Save/Load game
/// - Checkpoints for replay
/// - State snapshots for rollback
///
/// Note: Sync for MVP (in-memory). Can be made async later for DB.
pub trait StateRepository: Send + Sync {
    /// Load the current game state
    fn load(&self) -> Result<GameState>;

    /// Save the game state
    fn save(&self, state: &GameState) -> Result<()>;

    /// Save a checkpoint (optional)
    fn save_checkpoint(&self, _turn: u64, _state: &GameState) -> Result<()> {
        Ok(())
    }

    /// Load a checkpoint (optional)
    fn load_checkpoint(&self, _turn: u64) -> Result<Option<GameState>> {
        Ok(None)
    }
}
