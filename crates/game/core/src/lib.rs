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
pub use action::{
    Action, ActionCommand, ActionKind, ActionTransition, AttackAction, AttackCommand, AttackStyle,
    CardinalDirection, CommandContext, InteractAction, InteractCommand, InventorySlot, ItemTarget,
    MoveAction, MoveCommand, MoveError, UseItemAction, UseItemCommand,
};
pub use config::GameConfig;
pub use engine::{
    ExecuteError, GameEngine, StateReducer, TransitionPhase, TransitionPhaseError, TurnError,
};
pub use env::{
    AttackProfile, Env, GameEnv, InitialEntityKind, InitialEntitySpec, ItemCategory,
    ItemDefinition, ItemOracle, MapDimensions, MapOracle, MovementRules, NpcOracle, NpcTemplate,
    StaticTile, TablesOracle, TerrainKind,
};
pub use state::{
    ActorPatch, ActorState, ActorStats, CollectionDelta, EntitiesDelta, EntitiesState, EntityId,
    EventId, GameState, HazardOverlay, InitializationError, InventoryState, ItemHandle, ItemPatch,
    ItemState, OccupancyPatch, Overlay, OverlayPatch, OverlaySet, Position, PropKind, PropPatch,
    PropState, ResourceMeter, StateDelta, Tick, TileMap, TileView, TurnDelta, TurnState,
    WorldDelta, WorldState,
};
