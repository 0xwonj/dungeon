use crate::state::types::{ActorState, InventoryState, ItemState, PropState};
use crate::state::{EntityId, Position, Tick};
use crate::stats::{ActorBonuses, CoreStats, ResourceCurrent};

/// Represents a field that may or may not have changed.
///
/// This avoids the confusing `Option<Option<T>>` pattern by making
/// the "changed" vs "unchanged" distinction explicit.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum FieldDelta<T> {
    /// Field has not changed
    #[default]
    Unchanged,
    /// Field has changed to the given value
    Changed(T),
}

impl<T> FieldDelta<T> {
    /// Returns true if the field changed
    #[allow(dead_code)]
    pub fn is_changed(&self) -> bool {
        matches!(self, FieldDelta::Changed(_))
    }

    /// Returns the changed value if present
    #[allow(dead_code)]
    pub fn as_changed(&self) -> Option<&T> {
        match self {
            FieldDelta::Changed(value) => Some(value),
            FieldDelta::Unchanged => None,
        }
    }
}

/// Minimal actor update.
///
/// Only includes fields that changed. Used for efficient delta transmission
/// and ZK proof generation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActorPatch {
    pub id: EntityId,
    pub position: FieldDelta<Position>,
    pub core_stats: FieldDelta<CoreStats>,
    pub resources: FieldDelta<ResourceCurrent>,
    pub bonuses: FieldDelta<ActorBonuses>,
    pub inventory: FieldDelta<InventoryState>,
    pub ready_at: FieldDelta<Option<Tick>>,
}

impl ActorPatch {
    pub(super) fn from_states(before: &ActorState, after: &ActorState) -> Option<Self> {
        let position = if before.position != after.position {
            FieldDelta::Changed(after.position)
        } else {
            FieldDelta::Unchanged
        };

        let core_stats = if before.core_stats != after.core_stats {
            FieldDelta::Changed(after.core_stats.clone())
        } else {
            FieldDelta::Unchanged
        };

        let resources = if before.resources != after.resources {
            FieldDelta::Changed(after.resources.clone())
        } else {
            FieldDelta::Unchanged
        };

        let bonuses = if before.bonuses != after.bonuses {
            FieldDelta::Changed(after.bonuses.clone())
        } else {
            FieldDelta::Unchanged
        };

        let inventory = if before.inventory != after.inventory {
            FieldDelta::Changed(after.inventory.clone())
        } else {
            FieldDelta::Unchanged
        };

        let ready_at = if before.ready_at != after.ready_at {
            FieldDelta::Changed(after.ready_at)
        } else {
            FieldDelta::Unchanged
        };

        if matches!(position, FieldDelta::Unchanged)
            && matches!(core_stats, FieldDelta::Unchanged)
            && matches!(resources, FieldDelta::Unchanged)
            && matches!(bonuses, FieldDelta::Unchanged)
            && matches!(inventory, FieldDelta::Unchanged)
            && matches!(ready_at, FieldDelta::Unchanged)
        {
            return None;
        }

        Some(Self {
            id: after.id,
            position,
            core_stats,
            resources,
            bonuses,
            inventory,
            ready_at,
        })
    }
}

/// Minimal prop update.
///
/// Used for efficient delta transmission and ZK proof generation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PropPatch {
    pub id: EntityId,
    pub position: FieldDelta<Position>,
    pub is_active: FieldDelta<bool>,
}

impl PropPatch {
    pub(super) fn from_states(before: &PropState, after: &PropState) -> Option<Self> {
        let position = if before.position != after.position {
            FieldDelta::Changed(after.position)
        } else {
            FieldDelta::Unchanged
        };

        let is_active = if before.is_active != after.is_active {
            FieldDelta::Changed(after.is_active)
        } else {
            FieldDelta::Unchanged
        };

        if matches!(position, FieldDelta::Unchanged) && matches!(is_active, FieldDelta::Unchanged) {
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
///
/// Used for efficient delta transmission and ZK proof generation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemPatch {
    pub id: EntityId,
    pub position: FieldDelta<Position>,
}

impl ItemPatch {
    pub(super) fn from_states(before: &ItemState, after: &ItemState) -> Option<Self> {
        let position = if before.position != after.position {
            FieldDelta::Changed(after.position)
        } else {
            FieldDelta::Unchanged
        };

        if matches!(position, FieldDelta::Unchanged) {
            return None;
        }

        Some(Self {
            id: after.id,
            position,
        })
    }
}

/// Minimal occupancy update for a tile position.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OccupancyPatch {
    pub position: Position,
    pub occupants: Vec<EntityId>,
}
