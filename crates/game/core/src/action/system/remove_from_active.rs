//! Remove entity from active set system action.
//!
//! This action removes an entity from the active actor set, typically when
//! they die or become otherwise unavailable for turn scheduling.

use crate::action::ActionTransition;
use crate::action::error::RemoveFromActiveError;
use crate::env::GameEnv;
use crate::state::{EntityId, GameState, Tick};

/// System action that removes an entity from the active set.
///
/// This action:
/// 1. Sets the entity's `ready_at` to None
/// 2. Removes the entity from `turn.active_actors`
///
/// # Use Cases
///
/// - Entity death
/// - Entity becoming incapacitated
/// - Entity leaving the game area
///
/// # Invariants
///
/// - Entity must exist in the game state
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RemoveFromActiveAction {
    /// The entity to remove from active set
    pub entity: EntityId,
}

impl RemoveFromActiveAction {
    /// Creates a new RemoveFromActive action.
    pub fn new(entity: EntityId) -> Self {
        Self { entity }
    }
}

impl ActionTransition for RemoveFromActiveAction {
    type Error = RemoveFromActiveError;
    type Result = ();

    fn actor(&self) -> EntityId {
        EntityId::SYSTEM
    }

    fn pre_validate(&self, state: &GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        let nonce = state.turn.nonce;

        // Verify this action is executed by the SYSTEM actor
        if self.actor() != EntityId::SYSTEM {
            return Err(RemoveFromActiveError::not_system_actor(nonce));
        }

        // Verify entity exists
        state
            .entities
            .actor(self.entity)
            .ok_or_else(|| RemoveFromActiveError::entity_not_found(self.entity, nonce))?;

        Ok(())
    }

    fn apply(&self, state: &mut GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        // Remove from active set
        state.turn.active_actors.remove(&self.entity);

        // Clear ready_at
        if let Some(actor) = state.entities.actor_mut(self.entity) {
            actor.ready_at = None;
        }

        Ok(())
    }

    fn post_validate(&self, state: &GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        // Verify entity is no longer in active set
        debug_assert!(
            !state.turn.active_actors.contains(&self.entity),
            "entity should not be in active set after removal"
        );

        // Verify entity has no ready_at
        if let Some(actor) = state.entities.actor(self.entity) {
            debug_assert!(
                actor.ready_at.is_none(),
                "entity should have no ready_at after removal"
            );
        }

        Ok(())
    }

    fn cost(&self, _env: &GameEnv<'_>) -> Tick {
        0 // System actions have no time cost
    }
}
