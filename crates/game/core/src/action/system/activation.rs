//! Entity activation system action.
//!
//! Manages NPC activation and deactivation based on proximity to the player,
//! implementing the activation radius game mechanic.

use crate::action::ActionTransition;
use crate::action::error::ActivationError;
use crate::env::GameEnv;
use crate::state::{EntityId, GameState, Tick};

/// Distance threshold for NPC activation (Chebyshev distance).
///
/// NPCs within this distance of the player are activated (added to the active set
/// and assigned ready_at timestamps). NPCs beyond this distance are deactivated.
const ACTIVATION_RADIUS: u32 = 10;

/// System action that updates NPC activation based on player position.
///
/// This action:
/// 1. Gets player position
/// 2. For all NPCs:
///    - If within activation radius and inactive: activate (set ready_at)
///    - If beyond activation radius and active: deactivate (clear ready_at, remove from active set)
///
/// # Invariants
///
/// - Player must exist in the game state
/// - Activation distance uses Chebyshev metric (chessboard distance)
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ActivationAction;

impl ActionTransition for ActivationAction {
    type Error = ActivationError;
    type Result = ();

    fn actor(&self) -> EntityId {
        EntityId::SYSTEM
    }

    fn pre_validate(&self, state: &GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        let nonce = state.turn.nonce;

        // Verify this action is executed by the SYSTEM actor
        if self.actor() != EntityId::SYSTEM {
            return Err(ActivationError::not_system_actor(nonce));
        }

        // Verify player exists
        state
            .entities
            .actor(EntityId::PLAYER)
            .ok_or_else(|| ActivationError::player_not_found(nonce))?;

        Ok(())
    }

    fn apply(&self, state: &mut GameState, env: &GameEnv<'_>) -> Result<(), Self::Error> {
        let nonce = state.turn.nonce;

        // Get player position
        let player_pos = state
            .actor_position(EntityId::PLAYER)
            .ok_or_else(|| ActivationError::player_not_found(nonce))?;

        // Get current clock time for activation
        let current_clock = state.turn.clock;

        // Phase 1: Iterate tiles within activation radius to activate nearby NPCs
        // This is O(radius²)
        let radius = ACTIVATION_RADIUS as i32;
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let tile_pos = crate::state::Position {
                    x: player_pos.x + dx,
                    y: player_pos.y + dy,
                };

                // Get occupants at this tile
                let Some(occupants) = state.world.tile_map.occupants(&tile_pos) else {
                    continue;
                };

                // Check each occupant (typically 0-2 entities per tile)
                for &entity_id in occupants.iter() {
                    if entity_id == EntityId::PLAYER {
                        continue;
                    }

                    // Only activate if not already active
                    if let Some(actor) = state.entities.actor_mut(entity_id)
                        && actor.ready_at.is_none()
                    {
                        actor.ready_at = Some(current_clock);
                        state.turn.active_actors.insert(entity_id);
                    }
                }
            }
        }

        // Phase 2: Deactivate actors that moved outside radius
        // Iterate active_actors set (typically small) and check distance
        let active_actors: Vec<EntityId> = state.turn.active_actors.iter().copied().collect();
        for entity_id in active_actors {
            if entity_id == EntityId::PLAYER {
                continue;
            }

            if let Some(actor) = state.entities.actor_mut(entity_id) {
                // If actor has no position (dead, in inventory, etc.), deactivate them
                let Some(actor_pos) = actor.position else {
                    actor.ready_at = None;
                    state.turn.active_actors.remove(&entity_id);
                    continue;
                };

                let distance = calculate_distance(player_pos, actor_pos, env);
                if distance > ACTIVATION_RADIUS {
                    // Deactivate: NPC moved beyond radius
                    actor.ready_at = None;
                    state.turn.active_actors.remove(&entity_id);
                }
            }
        }

        Ok(())
    }

    fn post_validate(&self, state: &GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        // Verify player is still active (should always be true)
        debug_assert!(
            state.turn.active_actors.contains(&EntityId::PLAYER),
            "player must be in active set"
        );

        // Verify active_actors set matches actors with ready_at
        #[cfg(debug_assertions)]
        {
            for actor in state.entities.all_actors() {
                let has_ready_at = actor.ready_at.is_some();
                let in_active_set = state.turn.active_actors.contains(&actor.id);
                debug_assert_eq!(
                    has_ready_at, in_active_set,
                    "actor {} has ready_at={:?} but in_active_set={}",
                    actor.id, actor.ready_at, in_active_set
                );
            }
        }

        Ok(())
    }

    fn cost(&self, _env: &GameEnv<'_>) -> Tick {
        // System actions have no time cost
        0
    }
}

/// Calculate Chebyshev distance (chessboard distance) between two positions.
///
/// This is `max(|dx|, |dy|)`, which treats diagonal movement as having the same
/// cost as orthogonal movement (like a chess king).
fn calculate_distance(
    from: crate::state::Position,
    to: crate::state::Position,
    _env: &GameEnv<'_>,
) -> u32 {
    let dx = (from.x - to.x).abs();
    let dy = (from.y - to.y).abs();
    dx.max(dy) as u32
}
