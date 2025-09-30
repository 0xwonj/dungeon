use super::{EntityId, Position, ResourceMeter};

/// Aggregate state for every entity in the map.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct EntitiesState {
    pub player: ActorState,
    pub npcs: Vec<ActorState>,
    pub props: Vec<PropState>,
    pub items: Vec<ItemState>,
}

impl EntitiesState {
    pub fn new(
        player: ActorState,
        npcs: Vec<ActorState>,
        props: Vec<PropState>,
        items: Vec<ItemState>,
    ) -> Self {
        Self {
            player,
            npcs,
            props,
            items,
        }
    }
}

/// Minimal representation of any active actor (player or NPC).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ActorState {
    pub id: EntityId,
    pub position: Position,
    pub stats: ActorStats,
    pub inventory: InventoryState,
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
        }
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
    pub items: Vec<ItemHandle>,
}

impl InventoryState {
    pub fn new(items: Vec<ItemHandle>) -> Self {
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
