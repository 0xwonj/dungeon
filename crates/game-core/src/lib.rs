//! Deterministic rules engine for Dungeon.

pub mod action;
pub mod state;

pub use action::{
    Action, ActionKind, AttackAction, AttackStyle, CardinalDirection, InteractAction,
    InventorySlot, ItemTarget, MoveAction, UseItemAction,
};
pub use state::{
    ActorState, ActorStats, EntitiesState, EntityId, GameState, InventoryState, ItemHandle,
    ItemState, Position, PropKind, PropState, ResourceMeter, TileMap, TileOverlay, TileState,
    TurnPhase, TurnState, WorldState,
};
