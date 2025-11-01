use crate::action::ActionTransition;
use crate::env::GameEnv;
use crate::state::{EntityId, GameState, Position, Tick};

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MoveError {
    #[error("map oracle not available")]
    MissingMap,

    #[error("actor {0:?} not found")]
    ActorNotFound(EntityId),

    #[error("destination {destination:?} is out of bounds")]
    OutOfBounds { destination: Position },

    #[error("tile at {destination:?} not found")]
    TileNotFound { destination: Position },

    #[error("destination {destination:?} is blocked")]
    Blocked { destination: Position },

    #[error("destination {destination:?} is occupied")]
    Occupied { destination: Position },

    #[error("occupancy desync for actor {actor:?} at {position:?}")]
    OccupancyDesync { actor: EntityId, position: Position },

    #[error("actor {actor:?} missing from occupants at {position:?}")]
    MissingOccupant { actor: EntityId, position: Position },
}

/// High-level movement intent materialised into a canonical action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MoveAction {
    pub actor: EntityId,
    pub direction: CardinalDirection,
}

impl MoveAction {
    pub fn new(actor: EntityId, direction: CardinalDirection) -> Self {
        Self { actor, direction }
    }

    fn destination_from(&self, origin: Position) -> Position {
        let (dx, dy) = self.direction.delta();
        Position::new(origin.x + dx, origin.y + dy)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CardinalDirection {
    North,
    South,
    East,
    West,
}

impl CardinalDirection {
    pub const ALL: [CardinalDirection; 4] = [
        CardinalDirection::North,
        CardinalDirection::South,
        CardinalDirection::East,
        CardinalDirection::West,
    ];

    pub fn delta(self) -> (i32, i32) {
        match self {
            CardinalDirection::North => (0, 1),
            CardinalDirection::South => (0, -1),
            CardinalDirection::East => (1, 0),
            CardinalDirection::West => (-1, 0),
        }
    }
}

impl ActionTransition for MoveAction {
    type Error = MoveError;
    type Result = ();

    fn actor(&self) -> EntityId {
        self.actor
    }

    fn cost(&self) -> Tick {
        10
    }

    fn pre_validate(&self, state: &GameState, env: &GameEnv<'_>) -> Result<(), Self::Error> {
        let actor_state = state
            .entities
            .actor(self.actor)
            .ok_or(MoveError::ActorNotFound(self.actor))?;

        let map = env.map().ok_or(MoveError::MissingMap)?;
        let destination = self.destination_from(actor_state.position);
        if !map.contains(destination) {
            return Err(MoveError::OutOfBounds { destination });
        }

        let tile_view = state
            .tile_view(map, destination)
            .ok_or(MoveError::TileNotFound { destination })?;

        if !tile_view.is_passable() {
            return Err(MoveError::Blocked { destination });
        }

        if tile_view.is_occupied() {
            return Err(MoveError::Occupied { destination });
        }

        Ok(())
    }

    fn apply(&self, state: &mut GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        let actor_state = state
            .entities
            .actor(self.actor)
            .ok_or(MoveError::ActorNotFound(self.actor))?;
        let origin = actor_state.position;
        let destination = self.destination_from(origin);

        // Update occupancy map
        if !state.world.tile_map.remove_occupant(&origin, self.actor) {
            return Err(MoveError::OccupancyDesync {
                actor: self.actor,
                position: origin,
            });
        }
        if !state.world.tile_map.add_occupant(destination, self.actor) {
            // Rollback on failure
            let _ = state.world.tile_map.add_occupant(origin, self.actor);
            return Err(MoveError::OccupancyDesync {
                actor: self.actor,
                position: destination,
            });
        }

        // Update actor position
        let actor_state = state
            .entities
            .actor_mut(self.actor)
            .ok_or(MoveError::ActorNotFound(self.actor))?;
        actor_state.position = destination;

        Ok(())
    }

    fn post_validate(&self, state: &GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        let actor_state = state
            .entities
            .actor(self.actor)
            .ok_or(MoveError::ActorNotFound(self.actor))?;
        let is_present = state
            .world
            .tile_map
            .occupants(&actor_state.position)
            .map(|slot| slot.iter().copied().any(|id| id == self.actor))
            .unwrap_or(false);

        if is_present {
            Ok(())
        } else {
            Err(MoveError::MissingOccupant {
                actor: self.actor,
                position: actor_state.position,
            })
        }
    }
}
