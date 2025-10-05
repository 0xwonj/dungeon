mod errors;
mod turns;

use crate::action::{Action, ActionKind, ActionTransition};
use crate::config::GameConfig;
use crate::env::GameEnv;
use crate::state::GameState;

pub use errors::{ExecuteError, TransitionPhase, TransitionPhaseError};
pub use turns::ScheduledTurn;

type TransitionResult<E> = Result<(), TransitionPhaseError<E>>;

macro_rules! dispatch_transition {
    ($action:expr, $state:expr, $env:expr, { $($variant:ident => $err:ident),+ $(,)? }) => {{
        match &$action.kind {
            $(
                ActionKind::$variant(transition) => {
                    drive_transition(transition, $state, $env).map_err(ExecuteError::$err)
                }
            )+
            ActionKind::Wait => Ok(()),
        }
    }};
}

/// Game engine that manages action execution, turn scheduling, and game logic.
///
/// Combines action execution with turn scheduling in a unified API.
/// Turn scheduling uses simple linear search over active actors for simplicity and correctness.
pub struct GameEngine<'a> {
    state: &'a mut GameState,
    config: &'a GameConfig,
}

impl<'a> GameEngine<'a> {
    /// Creates a new game engine with the given state and configuration.
    pub fn new(state: &'a mut GameState, config: &'a GameConfig) -> Self {
        Self { state, config }
    }

    /// Executes an action by routing it through the appropriate transition pipeline.
    pub fn execute(&mut self, env: GameEnv<'_>, action: &Action) -> Result<(), ExecuteError> {
        dispatch_transition!(action, self.state, env, {
            Move => Move,
            Attack => Attack,
            UseItem => UseItem,
            Interact => Interact,
        })
    }
}

#[inline]
fn drive_transition<T>(
    transition: &T,
    state: &mut GameState,
    env: GameEnv<'_>,
) -> TransitionResult<T::Error>
where
    T: ActionTransition,
{
    transition
        .pre_validate(&*state, &env)
        .map_err(|error| TransitionPhaseError::new(TransitionPhase::PreValidate, error))?;

    transition
        .apply(state, &env)
        .map_err(|error| TransitionPhaseError::new(TransitionPhase::Apply, error))?;

    transition
        .post_validate(&*state, &env)
        .map_err(|error| TransitionPhaseError::new(TransitionPhase::PostValidate, error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{Action, ActionKind, CardinalDirection, MoveAction};
    use crate::config::GameConfig;
    use crate::env::{
        AttackProfile, Env, ItemCategory, ItemDefinition, ItemOracle, MapDimensions, MapOracle,
        MovementRules, StaticTile, TablesOracle, TerrainKind,
    };
    use crate::state::{EntityId, GameState, ItemHandle, Position, Tick};

    #[derive(Debug, Default)]
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

    #[derive(Debug, Default)]
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

    #[derive(Debug, Default)]
    struct StubTables;

    impl TablesOracle for StubTables {
        fn movement_rules(&self) -> MovementRules {
            MovementRules::new(1, 1)
        }

        fn attack_profile(&self, _style: crate::action::AttackStyle) -> Option<AttackProfile> {
            Some(AttackProfile::new(1, 0))
        }
    }

    #[test]
    fn engine_executes_actions() {
        let mut state = GameState::default();
        let mut occupants =
            arrayvec::ArrayVec::<EntityId, { GameConfig::MAX_OCCUPANTS_PER_TILE }>::new();
        occupants
            .try_push(EntityId::PLAYER)
            .expect("occupancy capacity");
        state
            .world
            .tile_map
            .replace_occupants(Position::ORIGIN, occupants);
        static MAP: StubMap = StubMap;
        static ITEMS: StubItems = StubItems;
        static TABLES: StubTables = StubTables;
        let env = Env::with_all(&MAP, &ITEMS, &TABLES).into_game_env();
        let actor = EntityId::PLAYER;
        let move_action = MoveAction::new(actor, CardinalDirection::North, 1);
        let action = Action::new(actor, ActionKind::Move(move_action));

        let config = GameConfig::default();
        let mut engine = GameEngine::new(&mut state, &config);
        engine
            .execute(env, &action)
            .expect("action execution should succeed");
    }

    #[test]
    fn maintain_active_set_handles_activation_changes() {
        use crate::state::{ActorState, ActorStats, InventoryState};

        let mut state = GameState::default();
        let config = GameConfig::with_activation_radius(1);

        state.entities.player = ActorState::new(
            EntityId::PLAYER,
            Position::ORIGIN,
            ActorStats::default(),
            InventoryState::default(),
        );
        state
            .entities
            .npcs
            .push(ActorState::new(
                EntityId(1),
                Position::new(1, 0),
                ActorStats::default(),
                InventoryState::default(),
            ))
            .unwrap();
        state
            .entities
            .npcs
            .push(ActorState::new(
                EntityId(2),
                Position::new(3, 0),
                ActorStats::default(),
                InventoryState::default(),
            ))
            .unwrap();

        state.entities.player.position = Position::ORIGIN;
        let mut engine = GameEngine::new(&mut state, &config);

        engine.maintain_active_set(
            [
                (EntityId(1), Position::new(1, 0)),
                (EntityId(2), Position::new(3, 0)),
            ],
            |_| Tick(0),
        );

        assert!(engine.is_entity_active(EntityId(1)));
        assert!(!engine.is_entity_active(EntityId(2)));

        // Move player far away
        engine.state.entities.player.position = Position::new(10, 10);
        engine.maintain_active_set(std::iter::empty::<(EntityId, Position)>(), |_| Tick(0));
        assert!(!engine.is_entity_active(EntityId(1)));
    }

    #[test]
    fn pop_next_turn_and_execute_workflow() {
        use crate::state::{ActorState, ActorStats, InventoryState};

        let mut state = GameState::default();
        let config = GameConfig::default();

        // Setup player
        state.entities.player = ActorState::new(
            EntityId::PLAYER,
            Position::ORIGIN,
            ActorStats::default(),
            InventoryState::default(),
        );

        // Add player to tile occupants
        let mut occupants =
            arrayvec::ArrayVec::<EntityId, { GameConfig::MAX_OCCUPANTS_PER_TILE }>::new();
        occupants.push(EntityId::PLAYER);
        state
            .world
            .tile_map
            .replace_occupants(Position::ORIGIN, occupants);

        let mut engine = GameEngine::new(&mut state, &config);

        // Activate player
        engine.activate(EntityId::PLAYER, Position::ORIGIN, Tick(0));

        static MAP: StubMap = StubMap;
        static ITEMS: StubItems = StubItems;
        static TABLES: StubTables = StubTables;
        let env = Env::with_all(&MAP, &ITEMS, &TABLES).into_game_env();

        // Pop next turn
        let scheduled = engine.pop_next_turn();
        assert!(scheduled.is_some());
        let scheduled = scheduled.unwrap();
        assert_eq!(scheduled.entity, EntityId::PLAYER);
        assert_eq!(scheduled.ready_at, Tick(0));

        // Execute action for that entity
        let move_action = MoveAction::new(EntityId::PLAYER, CardinalDirection::North, 1);
        let action = Action::new(EntityId::PLAYER, ActionKind::Move(move_action));
        assert!(engine.execute(env, &action).is_ok());
    }
}
