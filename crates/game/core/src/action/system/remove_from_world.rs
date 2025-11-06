//! Remove entity from world system action.
//!
//! This action removes an entity from their current position in the world,
//! clearing both their position field and the world occupancy map.

use crate::action::ActionTransition;
use crate::action::error::RemoveFromWorldError;
use crate::env::GameEnv;
use crate::state::{EntityId, GameState, Tick};

/// System action that removes an entity from the world.
///
/// This action only:
/// 1. Clears the entity's position from world occupancy
/// 2. Sets the entity's `position` to None
///
/// Does NOT affect:
/// - Turn scheduling (active_actors, ready_at)
/// - Entity stats or inventory
/// - Entity existence (still in entities state)
///
/// # Use Cases
///
/// - Entity death (corpse removed from map)
/// - Entity leaving the game area
/// - Entity being picked up or stored
///
/// # Invariants
///
/// - Entity must exist in the game state
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RemoveFromWorldAction {
    /// The entity to remove from world
    pub entity: EntityId,
}

impl RemoveFromWorldAction {
    /// Creates a new RemoveFromWorld action.
    pub fn new(entity: EntityId) -> Self {
        Self { entity }
    }
}

impl ActionTransition for RemoveFromWorldAction {
    type Error = RemoveFromWorldError;
    type Result = ();

    fn actor(&self) -> EntityId {
        EntityId::SYSTEM
    }

    fn pre_validate(&self, state: &GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        let nonce = state.turn.nonce;

        // Verify this action is executed by the SYSTEM actor
        if self.actor() != EntityId::SYSTEM {
            return Err(RemoveFromWorldError::not_system_actor(nonce));
        }

        // Verify entity exists
        state
            .entities
            .actor(self.entity)
            .ok_or_else(|| RemoveFromWorldError::entity_not_found(self.entity, nonce))?;

        Ok(())
    }

    fn apply(&self, state: &mut GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        // Get the actor and their current position
        if let Some(actor) = state.entities.actor_mut(self.entity) {
            // Remove from world occupancy if they have a position
            if let Some(position) = actor.position {
                state.world.tile_map.remove_occupant(&position, self.entity);
            }

            // Clear position
            actor.position = None;
        }

        Ok(())
    }

    fn post_validate(&self, state: &GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        // Verify entity no longer has a position
        if state.actor_position(self.entity).is_some() {
            return Err(RemoveFromWorldError::StillHasPosition {
                entity: self.entity,
                nonce: state.turn.nonce,
            });
        }

        // Verify entity is not in occupancy map
        for (_pos, occupants) in state.world.tile_map.occupancy().iter() {
            if occupants.contains(&self.entity) {
                return Err(RemoveFromWorldError::StillInOccupancy {
                    entity: self.entity,
                    nonce: state.turn.nonce,
                });
            }
        }

        Ok(())
    }

    fn cost(&self, _env: &GameEnv<'_>) -> Tick {
        0 // System actions have no time cost
    }
}
