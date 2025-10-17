//! Turn preparation system action.
//!
//! Selects the next entity to act based on turn scheduling rules and advances
//! the game clock to that entity's scheduled time.

use crate::action::ActionTransition;
use crate::env::GameEnv;
use crate::state::{EntityId, GameState, Tick};

/// System action that prepares the next turn by selecting which entity acts next.
///
/// This action:
/// 1. Finds the active entity with the smallest `ready_at` timestamp
/// 2. Advances the game clock to that timestamp
/// 3. Sets the entity as the current actor
///
/// # Invariants
///
/// - At least one entity must be active (have a `ready_at` value)
/// - The selected entity's `ready_at` must not be before the current clock
/// - Tie-breaking uses entity ID (lower ID acts first)
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PrepareTurnAction;

impl ActionTransition for PrepareTurnAction {
    type Error = TurnError;

    fn actor(&self) -> EntityId {
        EntityId::SYSTEM
    }

    fn pre_validate(&self, state: &GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        // Verify this action is executed by the SYSTEM actor
        if self.actor() != EntityId::SYSTEM {
            return Err(TurnError::NotSystemActor);
        }

        // Verify at least one entity is active and ready
        let has_ready_entity = state
            .turn
            .active_actors
            .iter()
            .any(|&id| state.entities.actor(id).and_then(|a| a.ready_at).is_some());

        if !has_ready_entity {
            return Err(TurnError::NoActiveEntities);
        }

        Ok(())
    }

    fn apply(&self, state: &mut GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        // Find the entity with the earliest ready_at timestamp
        // Tie-breaking: if multiple entities have the same timestamp, choose by entity ID
        let (entity, ready_at) = state
            .turn
            .active_actors
            .iter()
            .filter_map(|&id| {
                let actor = state.entities.actor(id)?;
                actor.ready_at.map(|tick| (id, tick))
            })
            .min_by_key(|&(entity_id, tick)| (tick, entity_id))
            .ok_or(TurnError::NoActiveEntities)?;

        // Advance clock to the scheduled time
        state.turn.clock = ready_at;

        // Set current actor
        state.turn.current_actor = entity;

        Ok(())
    }

    fn post_validate(&self, state: &GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        // Verify the selected actor is actually in the active set
        debug_assert!(
            state.turn.active_actors.contains(&state.turn.current_actor),
            "current_actor must be in active_actors set"
        );

        // Verify the current actor has a ready_at timestamp
        if let Some(actor) = state.entities.actor(state.turn.current_actor) {
            debug_assert!(
                actor.ready_at.is_some(),
                "current_actor must have a ready_at timestamp"
            );

            // Verify clock matches the actor's ready_at
            if let Some(ready_at) = actor.ready_at {
                debug_assert_eq!(
                    state.turn.clock, ready_at,
                    "clock must match current_actor's ready_at"
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

/// Errors that can occur during turn operations
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TurnError {
    #[error("prepare turn action must be executed by SYSTEM actor")]
    NotSystemActor,

    #[error("no entities are currently active")]
    NoActiveEntities,
}
