use crate::state::{EntityId, Position, Tick};

use super::GameEngine;

/// Entity scheduled for a turn along with the computed ready tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScheduledTurn {
    pub entity: EntityId,
    pub ready_at: Tick,
}

/// Turn scheduling methods for GameEngine.
impl<'a> GameEngine<'a> {
    /// Returns the current timeline clock value.
    pub fn clock(&self) -> Tick {
        self.state.turn.clock
    }

    /// Sets the current timeline clock value.
    /// This should be called by runtime when starting a turn.
    pub fn set_clock(&mut self, tick: Tick) {
        self.state.turn.clock = tick;
    }

    /// Checks if an entity is currently scheduled.
    pub fn is_entity_active(&self, entity: EntityId) -> bool {
        self.state.turn.active_actors.contains(&entity)
    }

    /// Activates an entity for scheduling, adding it to the active set.
    /// The ready_at tick will be clamped to the current clock if it's in the past.
    pub fn activate(&mut self, entity: EntityId, position: Position, ready_at: Tick) {
        let ready_at = ready_at.max(self.state.turn.clock);

        if let Some(actor) = self.state.entities.actor_mut(entity) {
            actor.ready_at = Some(ready_at);
            actor.position = position;
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
}
