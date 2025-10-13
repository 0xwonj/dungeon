//! Turn scheduling and action execution pipeline.
//!
//! The [`GameEngine`] is the authoritative reducer for [`GameState`]. It
//! orchestrates the transition phases and surfaces rich error information
//! for the runtime. All state mutations, including system actions for turn
//! scheduling and cost application, flow through the same execute() pipeline.
mod errors;
mod hook;
mod turns;

use crate::action::{Action, ActionKind, ActionTransition};
use crate::env::GameEnv;
use crate::state::{GameState, StateDelta};

pub use errors::{ExecuteError, TransitionPhase, TransitionPhaseError};
pub use hook::{ActionCostHook, ActivationHook, PostExecutionHook};
pub use turns::TurnError;

type TransitionResult<E> = Result<(), TransitionPhaseError<E>>;

macro_rules! dispatch_transition {
    ($action:expr, $reducer:expr, $env:expr, { $($variant:ident => $err:ident),+ $(,)? }) => {{
        match &$action.kind {
            $(
                ActionKind::$variant(transition) => {
                    drive_transition(transition, $reducer, $env).map_err(ExecuteError::$err)
                }
            )+
            ActionKind::Wait => Ok(()),
        }
    }};
}

/// Game engine that manages action execution, turn scheduling, and game logic.
///
/// All state mutations flow through the three-phase action pipeline:
/// pre_validate → apply → post_validate
///
/// Both player/NPC actions and system actions (turn scheduling, cost application,
/// entity activation) use the same execution path, ensuring complete auditability
/// and proof generation for all state changes.
pub struct GameEngine<'a> {
    state: &'a mut GameState,
}

impl<'a> GameEngine<'a> {
    /// Creates a new game engine with the given state.
    pub fn new(state: &'a mut GameState) -> Self {
        Self { state }
    }

    /// Executes an action by routing it through the appropriate transition pipeline.
    ///
    /// Returns a [`StateDelta`] capturing all state changes made by the action.
    /// Both player/NPC actions and system actions go through the same pipeline.
    pub fn execute(
        &mut self,
        env: GameEnv<'_>,
        action: &Action,
    ) -> Result<StateDelta, ExecuteError> {
        let before = self.state.clone();

        dispatch_transition!(action, self.state, &env, {
            Move => Move,
            Attack => Attack,
            UseItem => UseItem,
            Interact => Interact,
            PrepareTurn => PrepareTurn,
            ActionCost => ActionCost,
            Activation => Activation,
        })?;

        // Generate delta capturing all state changes
        let delta = StateDelta::from_states(action.clone(), &before, self.state);
        Ok(delta)
    }
}

#[inline]
fn drive_transition<T>(
    transition: &T,
    state: &mut GameState,
    env: &GameEnv<'_>,
) -> TransitionResult<T::Error>
where
    T: ActionTransition,
{
    transition
        .pre_validate(state, env)
        .map_err(|error| TransitionPhaseError::new(TransitionPhase::PreValidate, error))?;

    transition
        .apply(state, env)
        .map_err(|error| TransitionPhaseError::new(TransitionPhase::Apply, error))?;

    transition
        .post_validate(state, env)
        .map_err(|error| TransitionPhaseError::new(TransitionPhase::PostValidate, error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{Action, ActionKind, CardinalDirection, MoveAction};
    use crate::config::GameConfig;
    use crate::env::{
        AttackProfile, ConfigOracle, Env, ItemCategory, ItemDefinition, ItemOracle, MapDimensions,
        MapOracle, MovementRules, StaticTile, TablesOracle, TerrainKind,
    };
    use crate::state::{EntityId, GameState, ItemHandle, Position};

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
            Some(crate::env::NpcTemplate::test_npc())
        }
    }

    #[derive(Debug, Default)]
    struct StubConfig;

    impl ConfigOracle for StubConfig {
        fn activation_radius(&self) -> u32 {
            5
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
        static CONFIG: StubConfig = StubConfig;
        let env = Env::with_all(&MAP, &ITEMS, &TABLES, &NPCS, &CONFIG).into_game_env();
        let actor = EntityId::PLAYER;
        let move_action = MoveAction::new(actor, CardinalDirection::North, 1);
        let action = Action::new(actor, ActionKind::Move(move_action));

        let mut engine = GameEngine::new(&mut state);
        let _ = engine
            .execute(env, &action)
            .expect("action execution should succeed");
    }

    #[test]
    fn execute_reports_movement() {
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
        static CONFIG: StubConfig = StubConfig;
        let env = Env::with_all(&MAP, &ITEMS, &TABLES, &NPCS, &CONFIG).into_game_env();
        let actor = EntityId::PLAYER;
        let move_action = MoveAction::new(actor, CardinalDirection::North, 1);
        let action = Action::new(actor, ActionKind::Move(move_action));

        let mut engine = GameEngine::new(&mut state);
        let delta = engine
            .execute(env, &action)
            .expect("action execution should succeed");

        assert_eq!(delta.action, action);
        assert!(delta.turn.clock.is_none());
        assert!(delta.turn.current_actor.is_none());
        assert!(delta.turn.activated.is_empty());
        assert!(delta.turn.deactivated.is_empty());

        let player_patch = delta
            .entities
            .player
            .expect("player delta should be present");
        assert_eq!(player_patch.position, Some(Position::new(0, 1)));

        let origin_patch = delta
            .world
            .occupancy
            .iter()
            .find(|patch| patch.position == Position::ORIGIN)
            .expect("origin occupancy patch");
        assert!(origin_patch.occupants.is_empty());

        let new_tile_patch = delta
            .world
            .occupancy
            .iter()
            .find(|patch| patch.position == Position::new(0, 1))
            .expect("destination occupancy patch");
        assert_eq!(new_tile_patch.occupants, vec![EntityId::PLAYER]);
    }

    // Note: ready_at updates are now handled via ActionCostAction system action
    // at the SimulationWorker level, not within GameEngine::execute().
    // See SimulationWorker tests for integration testing of cost application.
}
