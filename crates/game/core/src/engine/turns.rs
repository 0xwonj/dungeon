use crate::state::{EntityId, Tick};

use super::GameEngine;

/// Errors that can occur during turn operations
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum TurnError {
    #[error("no entities are currently active")]
    NoActiveEntities,
}

/// Internal representation of a scheduled turn
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ScheduledTurn {
    entity: EntityId,
    ready_at: Tick,
}

/// Turn scheduling methods for GameEngine.
impl<'a> GameEngine<'a> {
    /// Returns the current timeline clock value.
    pub fn clock(&self) -> Tick {
        self.state.turn.clock
    }

    /// Checks if an entity is currently scheduled.
    pub fn is_entity_active(&self, entity: EntityId) -> bool {
        self.state.turn.active_actors.contains(&entity)
    }

    /// Activates an entity for scheduling, adding it to the active set.
    /// The initial ready_at is calculated based on the actor's speed stat using Wait action cost.
    pub fn activate(&mut self, entity: EntityId) {
        let current_clock = self.state.turn.clock;

        if let Some(actor) = self.state.entities.actor_mut(entity) {
            // Use the same calculation as Wait action for consistency
            let delay = crate::action::Action::calculate_delay(
                &crate::action::ActionKind::Wait,
                &actor.stats,
            );

            actor.ready_at = Some(Tick(current_clock.0 + delay.0));
        }

        self.state.turn.active_actors.insert(entity);
    }

    /// Deactivates an entity, removing it from scheduling and the active set.
    /// Returns true if the entity was active.
    pub fn deactivate(&mut self, entity: EntityId) -> bool {
        let was_active = self.state.turn.active_actors.remove(&entity);

        if let Some(actor) = self.state.entities.actor_mut(entity) {
            actor.ready_at = None;
        }

        was_active
    }

    /// Selects the next entity to act by finding the one with the smallest ready_at tick.
    /// Returns None if no entities are active.
    fn select_next_turn(&self) -> Option<ScheduledTurn> {
        self.state
            .turn
            .active_actors
            .iter()
            .filter_map(|&id| {
                let actor = self.state.entities.actor(id)?;
                actor.ready_at.map(|tick| (tick, id))
            })
            .min_by_key(|(tick, entity)| (*tick, *entity))
            .map(|(ready_at, entity)| ScheduledTurn { entity, ready_at })
    }

    /// Prepares for the next turn by selecting the next entity and updating the clock.
    /// After calling this, use current_actor() to get which entity should act.
    /// The caller must then provide an action via execute().
    pub fn prepare_next_turn(&mut self) -> Result<(), TurnError> {
        let scheduled = self.select_next_turn().ok_or(TurnError::NoActiveEntities)?;

        // Update clock to the scheduled time
        self.state.turn.clock = scheduled.ready_at;

        // Set current actor
        self.state.turn.current_actor = scheduled.entity;

        Ok(())
    }

    /// Returns the entity currently taking their turn.
    pub fn current_actor(&self) -> EntityId {
        self.state.turn.current_actor
    }
}
