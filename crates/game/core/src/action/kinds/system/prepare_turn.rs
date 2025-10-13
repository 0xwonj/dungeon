//! Turn preparation system action.
//!
//! Selects the next entity to act based on turn scheduling rules and advances
//! the game clock to that entity's scheduled time.

use crate::action::ActionTransition;
use crate::engine::TurnError;
use crate::env::GameEnv;
use crate::state::{GameState, Tick};

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
pub struct PrepareTurnAction;

impl ActionTransition for PrepareTurnAction {
    type Error = TurnError;

    fn pre_validate(&self, state: &GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
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
        Tick::ZERO
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{ActorState, ActorStats, EntitiesState, EntityId, InventoryState, Position};

    fn create_test_actor(id: EntityId, ready_at: Tick) -> ActorState {
        ActorState::new(id, Position::ORIGIN, ActorStats::default(), InventoryState::default())
            .with_ready_at(ready_at)
    }

    #[test]
    fn selects_entity_with_earliest_ready_at() {
        let mut state = GameState::default();

        // Setup three entities with different ready_at times
        state.entities.player = create_test_actor(EntityId::PLAYER, Tick(100));
        state
            .entities
            .npcs
            .push(create_test_actor(EntityId(1), Tick(50)))
            .unwrap();
        state
            .entities
            .npcs
            .push(create_test_actor(EntityId(2), Tick(150)))
            .unwrap();

        state.turn.active_actors.insert(EntityId::PLAYER);
        state.turn.active_actors.insert(EntityId(1));
        state.turn.active_actors.insert(EntityId(2));

        let action = PrepareTurnAction;
        let env = GameEnv::empty();

        action.pre_validate(&state, &env).unwrap();
        action.apply(&mut state, &env).unwrap();
        action.post_validate(&state, &env).unwrap();

        assert_eq!(state.turn.current_actor, EntityId(1));
        assert_eq!(state.turn.clock, Tick(50));
    }

    #[test]
    fn tie_breaks_by_entity_id() {
        let mut state = GameState::default();

        // Setup entities with same ready_at
        state.entities.player = create_test_actor(EntityId::PLAYER, Tick(100));
        state
            .entities
            .npcs
            .push(create_test_actor(EntityId(2), Tick(100)))
            .unwrap();

        state.turn.active_actors.insert(EntityId::PLAYER);
        state.turn.active_actors.insert(EntityId(2));

        let action = PrepareTurnAction;
        let env = GameEnv::empty();

        action.apply(&mut state, &env).unwrap();

        // Lower entity ID should be selected
        assert_eq!(state.turn.current_actor, EntityId::PLAYER);
    }

    #[test]
    fn fails_when_no_active_entities() {
        let state = GameState::default();
        let action = PrepareTurnAction;
        let env = GameEnv::empty();

        let result = action.pre_validate(&state, &env);
        assert!(matches!(result, Err(TurnError::NoActiveEntities)));
    }
}
