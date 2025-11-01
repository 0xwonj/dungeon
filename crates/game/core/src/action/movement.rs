use crate::action::ActionTransition;
use crate::env::{GameEnv, OracleError};
use crate::error::{ErrorContext, ErrorSeverity, GameError};
use crate::state::{EntityId, GameState, Position, Tick};

/// Errors that can occur during movement actions.
///
/// Movement errors arise from validation failures (actor not found, destination blocked)
/// or internal state inconsistencies (occupancy map desync).
#[derive(Clone, Debug, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MoveError {
    /// Oracle error (map not available, position out of bounds, etc.)
    #[error(transparent)]
    Oracle(#[from] OracleError),

    /// Actor not found in game state.
    #[error("actor {actor:?} not found")]
    ActorNotFound {
        actor: EntityId,
        #[cfg_attr(feature = "serde", serde(skip))]
        context: ErrorContext,
    },

    /// Destination is blocked by impassable terrain.
    #[error("destination {destination:?} is blocked by {terrain:?}")]
    Blocked {
        destination: Position,
        terrain: &'static str,
        #[cfg_attr(feature = "serde", serde(skip))]
        context: ErrorContext,
    },

    /// Destination is occupied by another entity.
    #[error("destination {destination:?} is occupied by entity {occupant:?}")]
    Occupied {
        destination: Position,
        occupant: EntityId,
        #[cfg_attr(feature = "serde", serde(skip))]
        context: ErrorContext,
    },

    /// Internal state desync - occupancy map inconsistent with entity positions.
    #[error("occupancy desync: actor {actor:?} at {position:?} - {expected}")]
    OccupancyDesync {
        actor: EntityId,
        position: Position,
        expected: &'static str,
        #[cfg_attr(feature = "serde", serde(skip))]
        context: ErrorContext,
    },
}

impl MoveError {
    /// Creates an ActorNotFound error with context.
    pub fn actor_not_found(actor: EntityId, nonce: u64) -> Self {
        Self::ActorNotFound {
            actor,
            context: ErrorContext::new(nonce).with_actor(actor),
        }
    }

    /// Creates a Blocked error with context.
    pub fn blocked(
        destination: Position,
        terrain: &'static str,
        actor: EntityId,
        nonce: u64,
    ) -> Self {
        Self::Blocked {
            destination,
            terrain,
            context: ErrorContext::new(nonce)
                .with_actor(actor)
                .with_position(destination),
        }
    }

    /// Creates an Occupied error with context.
    pub fn occupied(
        destination: Position,
        occupant: EntityId,
        actor: EntityId,
        nonce: u64,
    ) -> Self {
        Self::Occupied {
            destination,
            occupant,
            context: ErrorContext::new(nonce)
                .with_actor(actor)
                .with_position(destination),
        }
    }

    /// Creates an OccupancyDesync error with context.
    pub fn occupancy_desync(
        actor: EntityId,
        position: Position,
        expected: &'static str,
        nonce: u64,
    ) -> Self {
        Self::OccupancyDesync {
            actor,
            position,
            expected,
            context: ErrorContext::new(nonce)
                .with_actor(actor)
                .with_position(position)
                .with_message("internal state inconsistency"),
        }
    }
}

impl GameError for MoveError {
    fn severity(&self) -> ErrorSeverity {
        match self {
            Self::Oracle(e) => e.severity(),
            Self::ActorNotFound { .. } => ErrorSeverity::Validation,
            Self::Blocked { .. } => ErrorSeverity::Recoverable,
            Self::Occupied { .. } => ErrorSeverity::Recoverable,
            Self::OccupancyDesync { .. } => ErrorSeverity::Internal,
        }
    }

    fn context(&self) -> Option<&ErrorContext> {
        match self {
            Self::Oracle(_) => None,
            Self::ActorNotFound { context, .. }
            | Self::Blocked { context, .. }
            | Self::Occupied { context, .. }
            | Self::OccupancyDesync { context, .. } => Some(context),
        }
    }

    fn error_code(&self) -> &'static str {
        match self {
            Self::Oracle(_) => "MOVE_ORACLE",
            Self::ActorNotFound { .. } => "MOVE_ACTOR_NOT_FOUND",
            Self::Blocked { .. } => "MOVE_BLOCKED",
            Self::Occupied { .. } => "MOVE_OCCUPIED",
            Self::OccupancyDesync { .. } => "MOVE_DESYNC",
        }
    }
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

    fn cost(&self, env: &GameEnv<'_>) -> Tick {
        // SAFETY: cost() is called before validation, so we use a default if oracle is missing.
        // This is acceptable because the action will fail in pre_validate if oracle is truly needed.
        env.tables()
            .map(|t| t.action_costs().move_action)
            .unwrap_or(100)
    }

    fn pre_validate(&self, state: &GameState, env: &GameEnv<'_>) -> Result<(), Self::Error> {
        let nonce = state.turn.nonce;

        let actor_state = state
            .entities
            .actor(self.actor)
            .ok_or_else(|| MoveError::actor_not_found(self.actor, nonce))?;

        let map = env.map()?;
        let destination = self.destination_from(actor_state.position);

        // Check map bounds
        if !map.contains(destination) {
            return Err(OracleError::PositionOutOfBounds(destination).into());
        }

        let tile_view = state
            .tile_view(map, destination)
            .ok_or(OracleError::TileNotFound(destination))?;

        // Check if destination is passable
        if !tile_view.is_passable() {
            let terrain = match tile_view.static_tile().terrain() {
                crate::env::TerrainKind::Wall => "wall",
                crate::env::TerrainKind::Water => "water",
                crate::env::TerrainKind::Floor => "floor",
                crate::env::TerrainKind::Void => "void",
                crate::env::TerrainKind::Custom(_) => "custom",
            };
            return Err(MoveError::blocked(destination, terrain, self.actor, nonce));
        }

        // Check if destination is occupied
        if tile_view.is_occupied() {
            // Get first occupant for error message
            let occupant = tile_view.occupants().next().unwrap_or(EntityId::SYSTEM);
            return Err(MoveError::occupied(
                destination,
                occupant,
                self.actor,
                nonce,
            ));
        }

        Ok(())
    }

    fn apply(&self, state: &mut GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        let nonce = state.turn.nonce;

        let actor_state = state
            .entities
            .actor(self.actor)
            .ok_or_else(|| MoveError::actor_not_found(self.actor, nonce))?;
        let origin = actor_state.position;
        let destination = self.destination_from(origin);

        // Update occupancy map
        if !state.world.tile_map.remove_occupant(&origin, self.actor) {
            return Err(MoveError::occupancy_desync(
                self.actor,
                origin,
                "actor should be at origin",
                nonce,
            ));
        }

        if !state.world.tile_map.add_occupant(destination, self.actor) {
            // Rollback on failure
            let _ = state.world.tile_map.add_occupant(origin, self.actor);
            return Err(MoveError::occupancy_desync(
                self.actor,
                destination,
                "destination should be empty",
                nonce,
            ));
        }

        // Update actor position
        let actor_state = state
            .entities
            .actor_mut(self.actor)
            .ok_or_else(|| MoveError::actor_not_found(self.actor, nonce))?;
        actor_state.position = destination;

        Ok(())
    }

    fn post_validate(&self, state: &GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        let nonce = state.turn.nonce;

        let actor_state = state
            .entities
            .actor(self.actor)
            .ok_or_else(|| MoveError::actor_not_found(self.actor, nonce))?;

        let is_present = state
            .world
            .tile_map
            .occupants(&actor_state.position)
            .map(|slot| slot.iter().copied().any(|id| id == self.actor))
            .unwrap_or(false);

        if is_present {
            Ok(())
        } else {
            Err(MoveError::occupancy_desync(
                self.actor,
                actor_state.position,
                "actor should be at destination",
                nonce,
            ))
        }
    }
}
