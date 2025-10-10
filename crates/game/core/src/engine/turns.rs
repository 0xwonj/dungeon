use crate::state::{EntityId, Tick};

use super::GameEngine;

/// Errors that can occur during turn operations
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum TurnError {
    #[error("no entities are currently active")]
    NoActiveEntities,
}

/// Turn scheduling methods for GameEngine.
impl<'a> GameEngine<'a> {
    /// Returns the current timeline clock value.
    pub fn clock(&self) -> Tick {
        self.state.turn.clock
    }

    /// Prepares for the next turn by selecting the next entity and updating the clock.
    /// After calling this, use current_actor() to get which entity should act.
    /// The caller must then provide an action via execute().
    pub fn prepare_next_turn(&mut self) -> Result<(), TurnError> {
        // Find the entity with the smallest ready_at tick
        let (entity, ready_at) = self
            .state
            .turn
            .active_actors
            .iter()
            .filter_map(|&id| {
                let actor = self.state.entities.actor(id)?;
                actor.ready_at.map(|tick| (id, tick))
            })
            .min_by_key(|(entity, tick)| (*tick, *entity))
            .ok_or(TurnError::NoActiveEntities)?;

        // Update clock to the scheduled time
        self.state.turn.clock = ready_at;

        // Set current actor
        self.state.turn.current_actor = entity;

        Ok(())
    }

    /// Returns the entity currently taking their turn.
    pub fn current_actor(&self) -> EntityId {
        self.state.turn.current_actor
    }
}
