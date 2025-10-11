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
            Wait => Wait,
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

    #[test]
    fn execute_wait_action_updates_ready_at() {
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

        static MAP: StubMap = StubMap;
        static ITEMS: StubItems = StubItems;
        static TABLES: StubTables = StubTables;
        static NPCS: StubNpcs = StubNpcs;
        static CONFIG: StubConfig = StubConfig;
        let env = Env::with_all(&MAP, &ITEMS, &TABLES, &NPCS, &CONFIG).into_game_env();

        // Execute Wait action
        let wait_action = crate::action::WaitAction::new(EntityId::PLAYER);
        let action = Action::new(EntityId::PLAYER, ActionKind::Wait(wait_action));

        let mut engine = GameEngine::new(&mut state);
        let delta = engine
            .execute(env, &action)
            .expect("wait action should execute successfully");

        // Verify ready_at was updated with Wait cost (100 ticks, speed-scaled)
        assert!(state.entities.player.ready_at.is_some());
        let new_ready_at = state.entities.player.ready_at.unwrap();
        assert_eq!(new_ready_at.0, 100); // Default speed is 100, so cost is 100

        // Verify delta reports the action
        assert!(matches!(delta.action.kind, ActionKind::Wait(_)));
    }

    #[test]
    fn npc_wait_doesnt_block_player_turn() {
        use crate::state::{ActorState, ActorStats, InventoryState};

        let mut state = GameState::default();

        // Setup player at (0, 0)
        state.entities.player = ActorState::new(
            EntityId::PLAYER,
            Position::ORIGIN,
            ActorStats::default(),
            InventoryState::default(),
        );
        state.entities.player.ready_at = Some(Tick(0));
        state.turn.active_actors.insert(EntityId::PLAYER);

        // Setup NPC at (1, 0) - adjacent to player
        let npc_id = EntityId(1);
        state.entities.npcs.push(ActorState::new(
            npc_id,
            Position::new(1, 0),
            ActorStats::default(),
            InventoryState::default(),
        ));
        state.entities.npcs[0].ready_at = Some(Tick(0));
        state.turn.active_actors.insert(npc_id);

        // Add occupants
        let mut player_occupants =
            arrayvec::ArrayVec::<EntityId, { GameConfig::MAX_OCCUPANTS_PER_TILE }>::new();
        player_occupants.push(EntityId::PLAYER);
        state
            .world
            .tile_map
            .replace_occupants(Position::ORIGIN, player_occupants);

        let mut npc_occupants =
            arrayvec::ArrayVec::<EntityId, { GameConfig::MAX_OCCUPANTS_PER_TILE }>::new();
        npc_occupants.push(npc_id);
        state
            .world
            .tile_map
            .replace_occupants(Position::new(1, 0), npc_occupants);

        static MAP: StubMap = StubMap;
        static ITEMS: StubItems = StubItems;
        static TABLES: StubTables = StubTables;
        static NPCS: StubNpcs = StubNpcs;
        static CONFIG: StubConfig = StubConfig;

        // Turn 1: Player acts (moves or waits)
        let mut engine = GameEngine::new(&mut state);
        engine.prepare_next_turn().expect("should prepare turn");
        let actor1 = engine.current_actor();
        assert_eq!(actor1, EntityId::PLAYER); // Player should go first (lower ID)

        let wait_action = crate::action::WaitAction::new(EntityId::PLAYER);
        let action = Action::new(EntityId::PLAYER, ActionKind::Wait(wait_action));
        let env1 = Env::with_all(&MAP, &ITEMS, &TABLES, &NPCS, &CONFIG).into_game_env();
        let _ = engine.execute(env1, &action).expect("should execute");

        // Player's ready_at should now be 100
        assert_eq!(state.entities.player.ready_at, Some(Tick(100)));

        // Turn 2: NPC acts (should wait because adjacent)
        let mut engine = GameEngine::new(&mut state);
        engine.prepare_next_turn().expect("should prepare turn");
        let actor2 = engine.current_actor();
        assert_eq!(actor2, npc_id); // NPC should go next (ready_at = 0 < 100)

        let npc_wait = crate::action::WaitAction::new(npc_id);
        let npc_action = Action::new(npc_id, ActionKind::Wait(npc_wait));
        let env2 = Env::with_all(&MAP, &ITEMS, &TABLES, &NPCS, &CONFIG).into_game_env();
        let _ = engine.execute(env2, &npc_action).expect("should execute");

        // NPC's ready_at should now be 100
        assert_eq!(state.entities.npcs[0].ready_at, Some(Tick(100)));

        // Turn 3: Should be player's turn again (both at 100, player has lower ID)
        let mut engine = GameEngine::new(&mut state);
        engine.prepare_next_turn().expect("should prepare turn");
        let actor3 = engine.current_actor();
        assert_eq!(actor3, EntityId::PLAYER); // Player should go (tie-breaker by ID)

        // Clock should be 100
        assert_eq!(state.turn.clock, Tick(100));
    }
}
