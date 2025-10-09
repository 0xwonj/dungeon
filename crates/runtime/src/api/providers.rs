//! Asynchronous abstraction for sourcing player and NPC intent.
//!
//! Runtime users plug in [`ActionProvider`] implementations so the simulation
//! can run with human input, scripted fixtures, or AI policies.
use async_trait::async_trait;
use game_core::{Action, EntityId, GameState};

use super::errors::Result;

/// Trait for providing actions based on the current game state.
///
/// Different implementations can handle:
/// - Player input (from UI/CLI)
/// - NPC AI decisions
/// - Scripted/replayed actions
/// - Testing fixtures
#[async_trait]
pub trait ActionProvider: Send + Sync {
    /// Provide an action for the given entity based on the current game state.
    ///
    /// # Arguments
    /// * `entity` - The entity that needs to act
    /// * `state` - Read-only snapshot of the current game state
    ///
    /// # Returns
    /// The action to execute, or an error if action cannot be determined
    async fn provide_action(&self, entity: EntityId, state: &GameState) -> Result<Action>;
}

/// A simple action provider that always returns Wait action.
/// Useful for testing or as a fallback.
pub struct WaitActionProvider;

#[async_trait]
impl ActionProvider for WaitActionProvider {
    async fn provide_action(&self, entity: EntityId, _state: &GameState) -> Result<Action> {
        Ok(Action::new(entity, game_core::ActionKind::Wait))
    }
}
