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
pub mod error;
pub mod state;
pub mod stats;
pub use action::{
    Action, ActionEffect, ActionError, ActionInput, ActionKind, ActionProfile, ActionResult,
    ActionTag, ActionTransition, ActivationAction, ActivationError, CardinalDirection,
    CharacterAction, DamageType, DeactivateAction, EffectContext, EffectKind, ExecutionPhase,
    Formula, PrepareTurnAction, RemoveFromWorldAction, RemoveFromWorldError, ResourceCost,
    SystemActionKind, TargetingMode, TurnError, compute_actions_root, get_available_actions,
};
pub use config::GameConfig;
pub use engine::{
    ExecuteError, ExecutionOutcome, GameEngine, TransitionPhase, TransitionPhaseError,
};
pub use env::{
    ActionCosts, ActorOracle, ActorTemplate, ActorTemplateBuilder, ActorsSnapshot, ArmorData,
    CombatParams, ConfigOracle, ConfigSnapshot, ConsumableData, ConsumableEffect, DamageParams,
    Env, GameEnv, HitChanceParams, ItemDefinition, ItemKind, ItemOracle, ItemsSnapshot,
    MapDimensions, MapOracle, MapSnapshot, OracleError, OracleSnapshot, PcgRng, RngOracle,
    SnapshotActorOracle, SnapshotConfigOracle, SnapshotItemOracle, SnapshotMapOracle,
    SnapshotOracleBundle, SnapshotTablesOracle, SpeedParams, StaticTile, TablesOracle,
    TablesSnapshot, TerrainKind, WeaponData, compute_seed,
};
pub use error::{ErrorContext, ErrorSeverity, GameError, NeverError};
pub use state::{
    ActionAbilities, ActionAbility, ActorChanges, ActorFields, ActorState, ArmorKind, AttackType,
    CollectionChanges, EntitiesChanges, EntitiesState, EntityId, Equipment, EquipmentBuilder,
    GameState, InventorySlot, InventoryState, ItemChanges, ItemFields, ItemHandle, ItemState,
    OccupancyChanges, PassiveAbilities, PassiveAbility, PassiveKind, Position, PropChanges,
    PropFields, PropKind, PropState, StateDelta, StateError, StatusEffect, StatusEffectKind,
    StatusEffects, Tick, TileMap, TileView, TurnChanges, TurnFields, TurnState, WeaponKind,
    WorldChanges, WorldState,
};
pub use stats::{
    ActorBonuses, Bonus, BonusStack, CoreEffective, CoreStatBonuses, CoreStatKind, CoreStats,
    DerivedBonuses, DerivedStats, ModifierBonuses, ResourceBonuses, ResourceCurrent, ResourceKind,
    ResourceMaximums, SpeedBonuses, SpeedKind, SpeedStats, StatBounds, StatLayer, StatModifiers,
    StatsSnapshot, StatsSnapshotBuilder, compute_actor_bonuses,
};
