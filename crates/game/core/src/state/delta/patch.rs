use crate::state::types::{ActorState, InventoryState, ItemState, PropState};
use crate::state::{EntityId, Position, Tick};
use crate::stats::ActorStats;

/// Minimal actor update.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActorPatch {
    pub id: EntityId,
    pub position: Option<Position>,
    pub stats: Option<ActorStats>,
    pub inventory: Option<InventoryState>,
    pub ready_at: Option<Option<Tick>>,
}

impl ActorPatch {
    pub(super) fn from_states(before: &ActorState, after: &ActorState) -> Option<Self> {
        let mut position = None;
        if before.position != after.position {
            position = Some(after.position);
        }

        let mut stats = None;
        if before.stats != after.stats {
            stats = Some(after.stats.clone());
        }

        let mut inventory = None;
        if before.inventory != after.inventory {
            inventory = Some(after.inventory.clone());
        }

        let mut ready_at = None;
        if before.ready_at != after.ready_at {
            ready_at = Some(after.ready_at);
        }

        if position.is_none() && stats.is_none() && inventory.is_none() && ready_at.is_none() {
            return None;
        }

        Some(Self {
            id: after.id,
            position,
            stats,
            inventory,
            ready_at,
        })
    }
}

/// Minimal prop update.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PropPatch {
    pub id: EntityId,
    pub position: Option<Position>,
    pub is_active: Option<bool>,
}

impl PropPatch {
    pub(super) fn from_states(before: &PropState, after: &PropState) -> Option<Self> {
        let mut position = None;
        if before.position != after.position {
            position = Some(after.position);
        }

        let mut is_active = None;
        if before.is_active != after.is_active {
            is_active = Some(after.is_active);
        }

        if position.is_none() && is_active.is_none() {
            return None;
        }

        Some(Self {
            id: after.id,
            position,
            is_active,
        })
    }
}

/// Minimal item update.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemPatch {
    pub id: EntityId,
    pub position: Option<Position>,
}

impl ItemPatch {
    pub(super) fn from_states(before: &ItemState, after: &ItemState) -> Option<Self> {
        if before.position == after.position {
            return None;
        }

        Some(Self {
            id: after.id,
            position: Some(after.position),
        })
    }
}
