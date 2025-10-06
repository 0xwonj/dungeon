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
    /// After successful execution, updates the actor's ready_at by the action's cost.
    pub fn execute(&mut self, env: GameEnv<'_>, action: &Action) -> Result<(), ExecuteError> {
        // Execute the action
        dispatch_transition!(action, self.state, env, {
            Move => Move,
            Attack => Attack,
            UseItem => UseItem,
            Interact => Interact,
        })?;

        // Update actor's ready_at by action cost
        let cost = action.cost();
        if let Some(actor) = self.state.entities.actor_mut(action.actor) {
            if let Some(current_ready_at) = actor.ready_at {
                actor.ready_at = Some(crate::state::Tick(current_ready_at.0 + cost.0));
            }
        }

        Ok(())
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

    #[derive(Debug, Default)]
    struct StubNpcs;

    impl crate::env::NpcOracle for StubNpcs {
        fn template(&self, _template_id: u16) -> Option<crate::env::NpcTemplate> {
            Some(crate::env::NpcTemplate::simple(100, 50))
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
        static NPCS: StubNpcs = StubNpcs;
        let env = Env::with_all(&MAP, &ITEMS, &TABLES, &NPCS).into_game_env();
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
    fn activate_and_deactivate_entities() {
        use crate::state::{ActorState, ActorStats, InventoryState};

        let mut state = GameState::default();
        let config = GameConfig::default();

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

        let mut engine = GameEngine::new(&mut state, &config);

        // Test activation
        engine.activate(EntityId(1), Position::new(1, 0), Tick(0));
        assert!(engine.is_entity_active(EntityId(1)));

        // Test deactivation
        assert!(engine.deactivate(EntityId(1)));
        assert!(!engine.is_entity_active(EntityId(1)));
    }

    #[test]
    fn clock_management() {
        use crate::state::{ActorState, ActorStats, InventoryState};

        let mut state = GameState::default();
        let config = GameConfig::default();

        state.entities.player = ActorState::new(
            EntityId::PLAYER,
            Position::ORIGIN,
            ActorStats::default(),
            InventoryState::default(),
        );

        let mut engine = GameEngine::new(&mut state, &config);

        assert_eq!(engine.clock(), Tick(0));

        engine.set_clock(Tick(100));
        assert_eq!(engine.clock(), Tick(100));
    }

    #[test]
    fn execute_action_and_update_ready_at() {
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
        state.entities.player.ready_at = Some(Tick(0));

        // Add player to tile occupants
        let mut occupants =
            arrayvec::ArrayVec::<EntityId, { GameConfig::MAX_OCCUPANTS_PER_TILE }>::new();
        occupants.push(EntityId::PLAYER);
        state
            .world
            .tile_map
            .replace_occupants(Position::ORIGIN, occupants);

        let mut engine = GameEngine::new(&mut state, &config);
        engine.set_clock(Tick(0));

        static MAP: StubMap = StubMap;
        static ITEMS: StubItems = StubItems;
        static TABLES: StubTables = StubTables;
        static NPCS: StubNpcs = StubNpcs;
        let env = Env::with_all(&MAP, &ITEMS, &TABLES, &NPCS).into_game_env();

        // Execute action
        let move_action = MoveAction::new(EntityId::PLAYER, CardinalDirection::North, 1);
        let action = Action::new(EntityId::PLAYER, ActionKind::Move(move_action));
        let cost = action.cost();

        assert!(engine.execute(env, &action).is_ok());

        // Verify ready_at was updated by action cost
        assert_eq!(state.entities.player.ready_at, Some(Tick(cost.0)));
    }
}
