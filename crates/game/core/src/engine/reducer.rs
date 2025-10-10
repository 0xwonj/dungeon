use crate::state::{
    ActorState, EntitiesState, EntityId, GameState, Overlay, Position, Tick, TurnState, WorldState,
};

/// Wraps mutable access to [`GameState`] with structured sub-reducers.
pub struct StateReducer<'a> {
    state: &'a mut GameState,
}

impl<'a> StateReducer<'a> {
    pub fn new(state: &'a mut GameState) -> Self {
        Self { state }
    }

    pub fn state(&self) -> &GameState {
        self.state
    }

    pub fn turn(&mut self) -> TurnReducer<'_> {
        TurnReducer {
            turn: &mut self.state.turn,
        }
    }

    pub fn entities(&mut self) -> EntitiesReducer<'_> {
        EntitiesReducer {
            entities: &mut self.state.entities,
        }
    }

    pub fn world(&mut self) -> WorldReducer<'_> {
        WorldReducer {
            world: &mut self.state.world,
        }
    }
}

pub struct TurnReducer<'a> {
    turn: &'a mut TurnState,
}

impl<'a> TurnReducer<'a> {
    pub fn advance_clock(&mut self, next_tick: Tick) {
        debug_assert!(next_tick >= self.turn.clock);
        self.turn.clock = next_tick;
    }

    pub fn set_current_actor(&mut self, actor: EntityId) {
        self.turn.current_actor = actor;
    }

    pub fn activate(&mut self, actor: EntityId) -> bool {
        self.turn.active_actors.insert(actor)
    }

    pub fn deactivate(&mut self, actor: EntityId) -> bool {
        self.turn.active_actors.remove(&actor)
    }
}

pub struct EntitiesReducer<'a> {
    entities: &'a mut EntitiesState,
}

impl<'a> EntitiesReducer<'a> {
    pub fn actor(&self, id: EntityId) -> Option<&ActorState> {
        self.entities.actor(id)
    }

    pub fn actor_mut(&mut self, id: EntityId) -> Option<&mut ActorState> {
        self.entities.actor_mut(id)
    }

    pub fn set_actor_position(&mut self, actor: EntityId, position: Position) -> Option<Position> {
        let actor_state = self.entities.actor_mut(actor)?;
        let previous = actor_state.position;
        actor_state.position = position;
        Some(previous)
    }

    pub fn set_actor_ready_at(
        &mut self,
        actor: EntityId,
        ready_at: Option<Tick>,
    ) -> Option<Option<Tick>> {
        let actor_state = self.entities.actor_mut(actor)?;
        let previous = actor_state.ready_at;
        actor_state.ready_at = ready_at;
        Some(previous)
    }
}

pub struct WorldReducer<'a> {
    world: &'a mut WorldState,
}

impl<'a> WorldReducer<'a> {
    pub fn add_occupant(&mut self, position: Position, entity: EntityId) -> bool {
        self.world.tile_map.add_occupant(position, entity)
    }

    pub fn remove_occupant(&mut self, position: &Position, entity: EntityId) -> bool {
        self.world.tile_map.remove_occupant(position, entity)
    }

    pub fn clear_occupants(&mut self, position: &Position) {
        self.world.tile_map.clear_occupants(position);
    }

    pub fn add_overlay(&mut self, position: Position, overlay: Overlay) -> Result<(), Overlay> {
        let mut result = Ok(());
        self.world.tile_map.with_overlay(position, |set| {
            if let Err(e) = set.push_overlay(overlay) {
                result = Err(e);
            }
        });
        result
    }

    pub fn remove_overlay<F>(&mut self, position: Position, predicate: F)
    where
        F: FnMut(&Overlay) -> bool,
    {
        self.world
            .tile_map
            .with_overlay(position, |set| set.retain_overlays(predicate));
    }

    pub fn clear_overlays(&mut self, position: Position) {
        self.world.tile_map.with_overlay(position, |set| set.clear());
    }
}
