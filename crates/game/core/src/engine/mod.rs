//! Turn scheduling and action execution pipeline.
//!
//! The [`GameEngine`] is the authoritative reducer for [`GameState`]. It
//! orchestrates the transition phases, applies costs, and surfaces rich error
//! information for the runtime.
mod errors;
mod hook;
mod turns;

use std::sync::Arc;

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
/// Combines action execution with turn scheduling in a unified API.
/// Turn scheduling uses simple linear search over active actors for simplicity and correctness.
pub struct GameEngine<'a> {
    state: &'a mut GameState,
    hooks: Arc<[Arc<dyn PostExecutionHook>]>,
}

impl<'a> GameEngine<'a> {
    /// Creates a new game engine with the given state and configuration.
    pub fn new(state: &'a mut GameState) -> Self {
        Self {
            state,
            hooks: hook::default_hooks(),
        }
    }

    /// Executes an action by routing it through the appropriate transition pipeline.
    /// After successful execution, applies post-execution hooks and returns the resulting [`StateDelta`].
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
        })?;

        // Generate initial delta to check what changed
        let initial_delta = StateDelta::from_states(action.clone(), &before, self.state);

        // Apply post-execution hooks (already sorted by priority)
        for hook in self.hooks.iter() {
            if hook.should_trigger(&initial_delta) {
                hook.apply(self.state, &initial_delta, &env);
            }
        }

        // Generate final delta that includes hook effects
        let final_delta = StateDelta::from_states(action.clone(), &before, self.state);
        Ok(final_delta)
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

    #[test]
    fn activate_and_deactivate_entities() {
        use crate::state::{ActorState, ActorStats, InventoryState};

        let mut state = GameState::default();

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

        let mut engine = GameEngine::new(&mut state);

        // Test activation
        engine.activate(EntityId(1));
        assert!(engine.is_entity_active(EntityId(1)));

        // Test deactivation
        assert!(engine.deactivate(EntityId(1)));
        assert!(!engine.is_entity_active(EntityId(1)));
    }

    #[test]
    fn clock_management_through_prepare_next_turn() {
        use crate::state::{ActorState, ActorStats, InventoryState};

        let mut state = GameState::default();

        state.entities.player = ActorState::new(
            EntityId::PLAYER,
            Position::ORIGIN,
            ActorStats::default(),
            InventoryState::default(),
        );

        let mut engine = GameEngine::new(&mut state);

        // Initial clock
        assert_eq!(engine.clock(), Tick(0));

        // Activate player
        engine.activate(EntityId::PLAYER);

        // Prepare next turn should update clock and current_actor
        engine.prepare_next_turn().unwrap();
        assert_eq!(engine.current_actor(), EntityId::PLAYER);
        // Clock should now be at player's ready_at (which is 100 ticks from activation with default speed)
        assert_eq!(engine.clock(), Tick(100));
    }

    #[test]
    fn execute_action_and_update_ready_at() {
        use crate::state::{ActorState, ActorStats, InventoryState};

        let mut state = GameState::default();

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

        // Clock is already at 0 from default state
        static MAP: StubMap = StubMap;
        static ITEMS: StubItems = StubItems;
        static TABLES: StubTables = StubTables;
        static NPCS: StubNpcs = StubNpcs;
        static CONFIG: StubConfig = StubConfig;
        let env = Env::with_all(&MAP, &ITEMS, &TABLES, &NPCS, &CONFIG).into_game_env();

        // Execute action
        let move_action = MoveAction::new(EntityId::PLAYER, CardinalDirection::North, 1);
        let action = Action::new(EntityId::PLAYER, ActionKind::Move(move_action));

        let mut engine = GameEngine::new(&mut state);
        let _ = engine
            .execute(env, &action)
            .expect("should execute successfully");

        // Verify ready_at was updated (speed-scaled)
        assert!(state.entities.player.ready_at.is_some());
        assert!(state.entities.player.ready_at.unwrap().0 > 0);
    }
}
