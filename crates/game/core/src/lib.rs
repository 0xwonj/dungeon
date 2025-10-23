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
    Action, ActionCostAction, ActionTransition, ActivationAction, AttackAction, AttackStyle,
    CardinalDirection, CharacterActionKind, InteractAction, InventoryIndex, ItemTarget, MoveAction,
    MoveError, PrepareTurnAction, SystemActionKind, TurnError, UseItemAction,
    get_available_actions,
};
pub use config::GameConfig;
pub use engine::{ExecuteError, GameEngine, TransitionPhase, TransitionPhaseError};
pub use env::{
    ActorOracle, ActorTemplate, ActorTemplateBuilder, ActorsSnapshot, ArmorData, ConfigOracle,
    ConfigSnapshot, ConsumableData, ConsumableEffect, Env, GameEnv, ItemDefinition, ItemKind,
    ItemOracle, ItemsSnapshot, MapDimensions, MapOracle, MapSnapshot, OracleSnapshot,
    SnapshotActorOracle, SnapshotConfigOracle, SnapshotItemOracle, SnapshotMapOracle,
    SnapshotOracleBundle, SnapshotTablesOracle, StaticTile, TablesOracle, TablesSnapshot,
    TerrainKind, WeaponData,
};
pub use state::{
    ActionAbilities, ActionAbility, ActionKind, ActorChanges, ActorFields, ActorState, ArmorKind,
    CollectionChanges, EntitiesChanges, EntitiesState, EntityId, Equipment, EquipmentBuilder,
    GameState, InventorySlot, InventoryState, ItemChanges, ItemFields, ItemHandle, ItemState,
    OccupancyChanges, PassiveAbilities, PassiveAbility, PassiveKind, Position, PropChanges,
    PropFields, PropKind, PropState, StateDelta, StatusEffect, StatusEffectKind, StatusEffects,
    Tick, TileMap, TileView, TurnChanges, TurnFields, TurnState, WeaponKind, WorldChanges,
    WorldState,
};
pub use stats::{
    ActorBonuses, Bonus, BonusStack, CoreEffective, CoreStatBonuses, CoreStats, DerivedBonuses,
    DerivedStats, ModifierBonuses, ResourceBonuses, ResourceCurrent, ResourceMaximums,
    SpeedBonuses, SpeedKind, SpeedStats, StatBounds, StatLayer, StatModifiers, StatsSnapshot,
    StatsSnapshotBuilder, compute_actor_bonuses,
};
