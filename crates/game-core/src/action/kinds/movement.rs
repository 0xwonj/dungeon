use crate::action::ActionTransition;
use crate::env::GameEnv;
use crate::state::{EntityId, GameState, Position};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveError {
    MissingMap,
    MissingTables,
    ActorNotFound(EntityId),
    DistanceExceeded { distance: u32, max_distance: u8 },
    OutOfBounds { destination: Position },
    TileNotFound { destination: Position },
    Blocked { destination: Position },
    Occupied { destination: Position },
    OccupancyDesync { actor: EntityId, position: Position },
    MissingOccupant { actor: EntityId, position: Position },
}

/// High-level movement intent materialised into a canonical action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoveAction {
    pub actor: EntityId,
    pub direction: CardinalDirection,
    pub distance: u32,
}

impl MoveAction {
    pub fn new(actor: EntityId, direction: CardinalDirection, distance: u32) -> Self {
        Self {
            actor,
            direction,
            distance,
        }
    }

    fn destination_from(&self, origin: Position) -> Position {
        let (dx, dy) = self.direction.delta();
        Position::new(
            origin.x + dx * self.distance as i32,
            origin.y + dy * self.distance as i32,
        )
    }

    fn step_distance(&self) -> u32 {
        self.distance
    }
}

/// Cardinal grid movement command. Distance defaults to a single tile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoveCommand {
    pub direction: CardinalDirection,
    pub distance: u32,
}

impl MoveCommand {
    pub fn new(direction: CardinalDirection, distance: u32) -> Self {
        Self {
            direction,
            distance,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardinalDirection {
    North,
    South,
    East,
    West,
}

impl CardinalDirection {
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

    fn pre_validate(&self, state: &GameState, env: &GameEnv<'_>) -> Result<(), Self::Error> {
        let actor_state = state
            .entities
            .actor(self.actor)
            .ok_or(MoveError::ActorNotFound(self.actor))?;

        let tables = env.tables().ok_or(MoveError::MissingTables)?;
        let movement_rules = tables.movement_rules();
        let step_distance = self.step_distance();
        if step_distance > movement_rules.max_step_distance as u32 {
            return Err(MoveError::DistanceExceeded {
                distance: step_distance,
                max_distance: movement_rules.max_step_distance,
            });
        }

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
            .actor_mut(self.actor)
            .ok_or(MoveError::ActorNotFound(self.actor))?;
        let origin = actor_state.position;
        let destination = self.destination_from(origin);

        let occupancy = state.world.tile_map.occupancy_mut();
        if !occupancy.remove(&origin, self.actor) {
            return Err(MoveError::OccupancyDesync {
                actor: self.actor,
                position: origin,
            });
        }
        occupancy.add(destination, self.actor);

        actor_state.position = destination;
        Ok(())
    }

    fn post_validate(&self, state: &GameState, _env: &GameEnv<'_>) -> Result<(), Self::Error> {
        let actor_state = state
            .entities
            .actor(self.actor)
            .ok_or(MoveError::ActorNotFound(self.actor))?;
        let occupants = state
            .world
            .tile_map
            .occupants_slice(&actor_state.position)
            .unwrap_or(&[]);

        if occupants.iter().any(|&id| id == self.actor) {
            Ok(())
        } else {
            Err(MoveError::MissingOccupant {
                actor: self.actor,
                position: actor_state.position,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::{
        AttackProfile, Env, GameEnv, ItemCategory, ItemDefinition, ItemOracle, MapDimensions,
        MapOracle, MovementRules, StaticTile, TablesOracle, TerrainKind,
    };
    use crate::state::{EntityId, GameState, ItemHandle, Position};

    #[derive(Debug)]
    struct StubMap;

    impl MapOracle for StubMap {
        fn dimensions(&self) -> MapDimensions {
            MapDimensions::new(4, 4)
        }

        fn tile(&self, position: Position) -> Option<StaticTile> {
            if self.dimensions().contains(position) {
                Some(StaticTile::new(TerrainKind::Floor))
            } else {
                None
            }
        }
    }

    #[derive(Debug)]
    struct StubItems;

    impl ItemOracle for StubItems {
        fn definition(&self, handle: ItemHandle) -> Option<ItemDefinition> {
            Some(ItemDefinition::new(
                handle,
                ItemCategory::Utility,
                None,
                None,
            ))
        }
    }

    #[derive(Debug)]
    struct StubTables;

    impl TablesOracle for StubTables {
        fn movement_rules(&self) -> MovementRules {
            MovementRules::new(1, 1)
        }

        fn attack_profile(&self, _style: crate::action::AttackStyle) -> Option<AttackProfile> {
            Some(AttackProfile::new(1, 0))
        }
    }

    fn test_env() -> GameEnv<'static> {
        static MAP: StubMap = StubMap;
        static ITEMS: StubItems = StubItems;
        static TABLES: StubTables = StubTables;
        Env::with_all(&MAP, &ITEMS, &TABLES).into_game_env()
    }

    #[test]
    fn move_action_updates_state_and_occupancy() {
        let mut state = GameState::default();
        state
            .world
            .tile_map
            .replace_occupants(Position::ORIGIN, vec![EntityId::PLAYER]);
        let env = test_env();
        let action = MoveAction::new(EntityId::PLAYER, CardinalDirection::North, 1);

        action
            .pre_validate(&state, &env)
            .expect("move should be allowed");
        action
            .apply(&mut state, &env)
            .expect("apply should succeed");
        action
            .post_validate(&state, &env)
            .expect("post should accept new state");

        let expected = Position::new(0, 1);
        assert_eq!(state.entities.player.position, expected);
        let occupants = state
            .world
            .tile_map
            .occupants_slice(&expected)
            .expect("player should occupy destination");
        assert_eq!(occupants, &[EntityId::PLAYER]);
        assert!(
            state
                .world
                .tile_map
                .occupants_slice(&Position::ORIGIN)
                .is_none()
        );
    }

    #[test]
    fn move_action_rejects_occupied_destination() {
        let state = {
            let mut state = GameState::default();
            state
                .world
                .tile_map
                .replace_occupants(Position::ORIGIN, vec![EntityId::PLAYER]);
            let destination = Position::new(0, 1);
            state
                .world
                .tile_map
                .replace_occupants(destination, vec![EntityId(7)]);
            state
        };
        let env = test_env();
        let action = MoveAction::new(EntityId::PLAYER, CardinalDirection::North, 1);

        let err = action
            .pre_validate(&state, &env)
            .expect_err("tile should be occupied");
        assert_eq!(
            err,
            MoveError::Occupied {
                destination: Position::new(0, 1),
            }
        );
    }

    #[test]
    fn move_action_rejects_out_of_bounds_step() {
        let state = {
            let mut state = GameState::default();
            state
                .world
                .tile_map
                .replace_occupants(Position::new(0, 3), vec![EntityId::PLAYER]);
            state.entities.player.position = Position::new(0, 3);
            state
        };
        let env = test_env();
        let action = MoveAction::new(EntityId::PLAYER, CardinalDirection::North, 1);

        let err = action
            .pre_validate(&state, &env)
            .expect_err("step should exceed map bounds");
        assert_eq!(
            err,
            MoveError::OutOfBounds {
                destination: Position::new(0, 4),
            }
        );
    }
}
