pub mod actor;
pub mod common;
pub mod entities;
pub mod item;
pub mod turn;
pub mod world;

// Re-export all actor-related types
pub use actor::{
    // Abilities
    ActionAbilities,
    ActionAbility,
    // Main actor state
    ActorState,
    // Equipment
    Equipment,
    EquipmentBuilder,
    // Inventory
    InventorySlot,
    InventoryState,
    PassiveAbilities,
    PassiveAbility,
    PassiveKind,
    // Status effects
    StatusEffect,
    StatusEffectKind,
    StatusEffects,
};

// Re-export common types
pub use common::{EntityId, Position, Tick};

// Re-export entity collection and prop types
pub use entities::{EntitiesState, PropKind, PropState};

// Re-export item types
pub use item::{ItemHandle, ItemState};

// Re-export turn state
pub use turn::TurnState;

// Re-export world types
pub use world::{TileMap, TileView, WorldState};
