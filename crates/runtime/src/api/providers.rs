//! Asynchronous abstraction for sourcing player and NPC intent.
//!
//! This module defines the [`ActionProvider`] trait, which serves as the
//! interface between the runtime and different sources of entity actions.
use async_trait::async_trait;
use game_core::{Action, EntityId, GameEnv, GameState};

use super::errors::Result;

/// Trait for providing actions based on the current game state.
///
/// This is the core abstraction that allows the runtime to obtain actions
/// from different sources without knowing the implementation details.
///
/// # Contract
///
/// Implementations must:
/// - Be thread-safe (`Send + Sync`)
/// - Return actions that match the requested `entity`
/// - Handle state snapshots without mutation
/// - Fail gracefully with appropriate errors
///
/// # Error Handling
///
/// If a provider fails to generate an action, the runtime will fall back
/// to a Wait action and log a warning.
#[async_trait]
pub trait ActionProvider: Send + Sync {
    /// Provide an action for the given entity based on the current game state.
    ///
    /// # Arguments
    ///
    /// * `entity` - The entity that needs to act
    /// * `state` - Read-only snapshot of the current game state
    /// * `env` - Read-only access to all game oracles (map, items, tables, npcs, config)
    ///
    /// # Returns
    ///
    /// The action to execute, or an error if action cannot be determined
    async fn provide_action(
        &self,
        entity: EntityId,
        state: &GameState,
        env: GameEnv<'_>,
    ) -> Result<Action>;
}
