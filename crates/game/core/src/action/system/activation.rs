//! Entity activation system action.
//!
//! Manages NPC activation and deactivation based on proximity to the player,
//! implementing the activation radius game mechanic.

use crate::action::ActionTransition;
use crate::env::GameEnv;
use crate::state::{EntityId, GameState, Position, Tick};

/// System action that updates entity activation status based on player proximity.
///
/// NPCs within the activation radius are added to the active set and scheduled
/// to act. NPCs outside the radius are deactivated and removed from scheduling.
///
/// This action is typically triggered after the player moves, ensuring that only
/// nearby entities consume processing time and maintain responsiveness in large maps.
///
/// # Invariants
///
/// - Player must exist at the specified position
/// - Activated entities receive an initial `ready_at` based on Wait action cost
/// - Deactivated entities have their `ready_at` cleared
/// - Active set and `ready_at` timestamps remain synchronized
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ActivationAction {
    /// Player's current position (center of activation radius)
    pub player_position: Position,
}

impl ActivationAction {
    /// Creates a new activation update for the given player position.
    pub fn new(player_position: Position) -> Self {
        Self { player_position }
    }

    /// Calculates the grid distance between two positions (Chebyshev distance).
    fn grid_distance(a: Position, b: Position) -> u32 {
        let dx = (a.x - b.x).unsigned_abs();
        let dy = (a.y - b.y).unsigned_abs();
        dx.max(dy)
    }
}

impl ActionTransition for ActivationAction {
    type Error = ActivationError;

    fn actor(&self) -> EntityId {
        EntityId::SYSTEM
    }

    fn pre_validate(&self, _state: &GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        // Verify this action is executed by the SYSTEM actor
        if self.actor() != EntityId::SYSTEM {
            return Err(ActivationError::NotSystemActor);
        }

        Ok(())
    }

    fn apply(&self, state: &mut GameState, env: &GameEnv<'_>) -> Result<(), Self::Error> {
        let activation_radius = env.activation_radius();
        let clock = state.turn.clock;

        // Collect NPC data to avoid borrow checker issues (skip player at index 0)
        let npc_data: Vec<_> = state
            .entities
            .all_actors()
            .filter(|actor| actor.id != EntityId::PLAYER)
            .map(|npc| {
                let is_active = state.turn.active_actors.contains(&npc.id);
                (npc.id, npc.position, is_active, npc.is_alive())
            })
            .collect();

        // Process each NPC's activation status
        for (entity_id, npc_position, is_active, is_alive) in npc_data {
            let distance = Self::grid_distance(self.player_position, npc_position);

            if distance <= activation_radius {
                // Within activation radius - activate if not already active and alive
                if !is_active && is_alive {
                    state.turn.active_actors.insert(entity_id);

                    // Set initial ready_at using Wait action cost (100 ticks scaled by speed)
                    // This gives the NPC time to "wake up" before acting
                    if let Some(actor) = state.entities.actor_mut(entity_id) {
                        let snapshot = actor.snapshot();
                        let speed = snapshot.speed.physical.max(1) as u64;
                        let delay = 100 * 100 / speed;
                        actor.ready_at = Some(clock + delay);
                    }
                }
            } else if is_active {
                // Outside activation radius - deactivate if currently active
                state.turn.active_actors.remove(&entity_id);

                if let Some(actor) = state.entities.actor_mut(entity_id) {
                    actor.ready_at = None;
                }
            }
        }

        Ok(())
    }

    fn post_validate(&self, state: &GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        // Verify invariant: all entities in active_actors must have ready_at
        for &entity_id in &state.turn.active_actors {
            if let Some(actor) = state.entities.actor(entity_id) {
                debug_assert!(
                    actor.ready_at.is_some(),
                    "active actor {:?} must have ready_at timestamp",
                    entity_id
                );
            }
        }

        Ok(())
    }

    fn cost(&self) -> Tick {
        // System actions have no time cost
        0
    }
}

/// Errors that can occur during activation updates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ActivationError {
    #[error("activation action must be executed by SYSTEM actor")]
    NotSystemActor,

    #[error("player not found in game state")]
    PlayerNotFound,
}
