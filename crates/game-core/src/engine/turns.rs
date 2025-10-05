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

    /// Finds the next entity to act by scanning all active actors.
    /// Returns the entity with the smallest ready_at tick.
    /// Advances the clock to the returned entity's ready tick.
    pub fn pop_next_turn(&mut self) -> Option<ScheduledTurn> {
        let (next_tick, next_entity) = self
            .state
            .turn
            .active_actors
            .iter()
            .filter_map(|&id| {
                let actor = self.state.entities.actor(id)?;
                actor.ready_at.map(|tick| (tick, id))
            })
            .min_by_key(|(tick, _)| *tick)?;

        self.state.turn.clock = next_tick;

        Some(ScheduledTurn {
            entity: next_entity,
            ready_at: next_tick,
        })
    }

    /// Maintains the active entity set based on proximity to the player.
    /// Entities within the activation radius are activated (if not already active).
    /// Entities outside the radius are deactivated.
    /// The on_activate callback provides the initial ready tick for newly activated entities.
    pub fn maintain_active_set<I, F>(&mut self, entities: I, mut on_activate: F)
    where
        I: IntoIterator<Item = (EntityId, Position)>,
        F: FnMut(EntityId) -> Tick,
    {
        let player_position = self.state.entities.player.position;
        let mut newly_active = std::collections::HashSet::new();

        for (entity, position) in entities {
            if !self.is_within_activation_region(player_position, position) {
                continue;
            }

            newly_active.insert(entity);

            if self.state.turn.active_actors.contains(&entity) {
                // Already active, just update position
                if let Some(actor) = self.state.entities.actor_mut(entity) {
                    actor.position = position;
                }
            } else {
                // Newly activated
                let ready_at = on_activate(entity).max(self.state.turn.clock);
                if let Some(actor) = self.state.entities.actor_mut(entity) {
                    actor.ready_at = Some(ready_at);
                    actor.position = position;
                }
                self.state.turn.active_actors.insert(entity);
            }
        }

        // Deactivate entities outside the activation radius
        let to_deactivate: Vec<_> = self
            .state
            .turn
            .active_actors
            .iter()
            .copied()
            .filter(|id| !newly_active.contains(id))
            .collect();

        for entity in to_deactivate {
            self.deactivate(entity);
        }
    }

    fn is_within_activation_region(
        &self,
        player_position: Position,
        entity_position: Position,
    ) -> bool {
        let dx = (entity_position.x - player_position.x).abs() as u32;
        let dy = (entity_position.y - player_position.y).abs() as u32;
        dx <= self.config.activation_radius && dy <= self.config.activation_radius
    }
}
