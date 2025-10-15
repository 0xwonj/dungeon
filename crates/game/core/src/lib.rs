//! Deterministic game logic and data types shared across clients.
//!
//! `game-core` defines the canonical rules (actions, engine, world state) and
//! exposes pure APIs that can be reused by both the runtime and offline tools.
//! All state mutation flows through [`engine::GameEngine`], and supporting
//! crates depend on the types re-exported here.
pub mod action;
pub mod config;
pub mod engine;
pub mod env;
pub mod state;
pub mod stats;
pub use action::{
    Action, ActionCostAction, ActionKind, ActionTransition, ActivationAction, AttackAction,
    AttackStyle, CardinalDirection, InteractAction, InventorySlot, ItemTarget, MoveAction,
    MoveError, PrepareTurnAction, TurnError, UseItemAction,
};
pub use config::GameConfig;
pub use engine::{ExecuteError, GameEngine, TransitionPhase, TransitionPhaseError};
pub use env::{
    AttackProfile, ConfigOracle, Env, GameEnv, InitialEntityKind, InitialEntitySpec, ItemCategory,
    ItemDefinition, ItemOracle, MapDimensions, MapOracle, MovementRules, NpcOracle, NpcTemplate,
    StaticTile, TablesOracle, TerrainKind,
};
pub use state::{
    ActorChanges, ActorFields, ActorState, CollectionChanges, EntitiesChanges, EntitiesState,
    EntityId, GameState, InitializationError, InventoryState, ItemChanges, ItemFields, ItemHandle,
    ItemState, OccupancyChanges, Position, PropChanges, PropFields, PropKind, PropState,
    StateDelta, Tick, TileMap, TileView, TurnChanges, TurnFields, TurnState, WorldChanges,
    WorldState,
};
pub use stats::{
    ActorBonuses, Bonus, BonusStack, CoreEffective, CoreStatBonuses, CoreStats, DerivedBonuses,
    DerivedStats, ModifierBonuses, ResourceBonuses, ResourceCurrent, ResourceMaximums,
    SpeedBonuses, SpeedKind, SpeedStats, StatBounds, StatLayer, StatModifiers, StatsSnapshot,
    StatsSnapshotBuilder, compute_actor_bonuses,
};
