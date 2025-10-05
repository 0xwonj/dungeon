use arrayvec::ArrayVec;
use bounded_vector::BoundedVec;

use super::{EntityId, Position, ResourceMeter, Tick};
use crate::config::GameConfig;

/// Aggregate state for every entity in the map.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct EntitiesState {
    pub player: ActorState,
    pub npcs: BoundedVec<ActorState, 0, { GameConfig::MAX_NPCS }>,
    pub props: BoundedVec<PropState, 0, { GameConfig::MAX_PROPS }>,
    pub items: BoundedVec<ItemState, 0, { GameConfig::MAX_WORLD_ITEMS }>,
}

impl EntitiesState {
    pub fn new(
        player: ActorState,
        npcs: BoundedVec<ActorState, 0, { GameConfig::MAX_NPCS }>,
        props: BoundedVec<PropState, 0, { GameConfig::MAX_PROPS }>,
        items: BoundedVec<ItemState, 0, { GameConfig::MAX_WORLD_ITEMS }>,
    ) -> Self {
        Self {
            player,
            npcs,
            props,
            items,
        }
    }

    /// Returns a reference to an actor by ID (player or NPC).
    pub fn actor(&self, id: EntityId) -> Option<&ActorState> {
        if self.player.id == id {
            return Some(&self.player);
        }
        self.npcs.iter().find(|actor| actor.id == id)
    }

    /// Returns a mutable reference to an actor by ID (player or NPC).
    pub fn actor_mut(&mut self, id: EntityId) -> Option<&mut ActorState> {
        if self.player.id == id {
            return Some(&mut self.player);
        }
        self.npcs.iter_mut().find(|actor| actor.id == id)
    }

    /// Returns an iterator over all actors (player + NPCs).
    pub fn all_actors(&self) -> impl Iterator<Item = &ActorState> {
        std::iter::once(&self.player).chain(self.npcs.iter())
    }

    /// Returns a mutable iterator over all actors (player + NPCs).
    pub fn all_actors_mut(&mut self) -> impl Iterator<Item = &mut ActorState> {
        std::iter::once(&mut self.player).chain(self.npcs.iter_mut())
    }
}

/// Minimal representation of any active actor (player or NPC).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ActorState {
    pub id: EntityId,
    pub position: Position,
    pub stats: ActorStats,
    pub inventory: InventoryState,
    /// When this actor is scheduled to act next. None means not currently scheduled.
    pub ready_at: Option<Tick>,
}

impl ActorState {
    pub fn new(
        id: EntityId,
        position: Position,
        stats: ActorStats,
        inventory: InventoryState,
    ) -> Self {
        Self {
            id,
            position,
            stats,
            inventory,
            ready_at: None,
        }
    }

    pub fn with_ready_at(mut self, ready_at: Tick) -> Self {
        self.ready_at = Some(ready_at);
        self
    }
}

/// Simple combat/resource stats for an actor.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ActorStats {
    pub health: ResourceMeter,
    pub energy: ResourceMeter,
}

impl ActorStats {
    pub fn new(health: ResourceMeter, energy: ResourceMeter) -> Self {
        Self { health, energy }
    }
}

/// Simplified inventory snapshot; expand as item systems mature.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct InventoryState {
    pub items: ArrayVec<ItemHandle, { GameConfig::MAX_INVENTORY_SLOTS }>,
}

impl InventoryState {
    pub fn new(items: ArrayVec<ItemHandle, { GameConfig::MAX_INVENTORY_SLOTS }>) -> Self {
        Self { items }
    }
}

/// Non-actor entities such as doors, switches, or hazards.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PropState {
    pub id: EntityId,
    pub position: Position,
    pub kind: PropKind,
    pub is_active: bool,
}

impl PropState {
    pub fn new(id: EntityId, position: Position, kind: PropKind, is_active: bool) -> Self {
        Self {
            id,
            position,
            kind,
            is_active,
        }
    }
}

/// Enumerates the basic prop categories. Extend as needed by gameplay.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PropKind {
    Door,
    Switch,
    Hazard,
    Other,
}

/// Items that exist on the ground (not inside inventories).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemState {
    pub id: EntityId,
    pub position: Position,
    pub handle: ItemHandle,
}

impl ItemState {
    pub fn new(id: EntityId, position: Position, handle: ItemHandle) -> Self {
        Self {
            id,
            position,
            handle,
        }
    }
}

/// Reference to an item definition stored outside the core (lookup via Env).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemHandle(pub u32);
