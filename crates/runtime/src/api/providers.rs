//! Asynchronous abstraction for sourcing player and NPC intent.
//!
//! Runtime users plug in [`ActionProvider`] implementations so the simulation
//! can run with human input, scripted fixtures, or AI policies.
use async_trait::async_trait;
use game_core::{Action, ActionKind, EntityId, GameState};

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
        use game_core::WaitAction;
        Ok(Action::new(
            entity,
            ActionKind::Wait(WaitAction::new(entity)),
        ))
    }
}

/// Simple NPC AI that moves toward the player.
///
/// Moves one step closer to the player using Manhattan distance pathfinding.
/// Prioritizes the axis with greater distance. Waits when very close to avoid
/// moving into occupied tiles.
pub struct SimpleNpcProvider;

#[async_trait]
impl ActionProvider for SimpleNpcProvider {
    async fn provide_action(&self, entity: EntityId, state: &GameState) -> Result<Action> {
        use game_core::{CardinalDirection, MoveAction};

        // Get NPC position
        let npc_pos = state
            .entities
            .actor(entity)
            .map(|actor| actor.position)
            .ok_or_else(|| super::errors::RuntimeError::InvalidEntity { entity })?;

        // Get player position
        let player_pos = state.entities.player.position;

        // Calculate distance
        let dx = player_pos.x - npc_pos.x;
        let dy = player_pos.y - npc_pos.y;

        // Manhattan distance
        let distance = dx.abs() + dy.abs();

        // If very close to player, wait to avoid moving into occupied tile
        if distance <= 1 {
            use game_core::WaitAction;
            return Ok(Action::new(
                entity,
                ActionKind::Wait(WaitAction::new(entity)),
            ));
        }

        // Move toward player using Manhattan distance
        // Prioritize the axis with greater distance
        // Note: North = +y, South = -y, East = +x, West = -x
        let direction = if dx.abs() >= dy.abs() {
            if dx > 0 {
                CardinalDirection::East
            } else if dx < 0 {
                CardinalDirection::West
            } else if dy > 0 {
                CardinalDirection::North
            } else {
                CardinalDirection::South
            }
        } else if dy > 0 {
            CardinalDirection::North
        } else {
            CardinalDirection::South
        };

        let move_action = MoveAction::new(entity, direction, 1);
        Ok(Action::new(entity, ActionKind::Move(move_action)))
    }
}
