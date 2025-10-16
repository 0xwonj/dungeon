use crate::state::types::{ActorState, ItemState, PropState, TurnState};
use crate::state::{EntityId, Position};

use super::bitmask::{ActorFields, ItemFields, PropFields, TurnFields};

/// Metadata describing which fields of an actor changed.
///
/// This structure stores only the entity ID and a bitmask indicating which fields
/// were modified. Actual values are retrieved from the before/after [`GameState`]
/// when needed (e.g., during ZK witness generation).
///
/// # Design Rationale
///
/// - **Memory efficient**: ~10 bytes per changed entity vs. ~100 bytes with value storage
/// - **ZK-friendly**: Bitmask directly indicates which fields need proof constraints
/// - **Separation of concerns**: Metadata (what changed) vs. data (actual values)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ActorChanges {
    pub id: EntityId,
    pub fields: ActorFields,
}

impl ActorChanges {
    /// Creates actor changes by comparing two actor states.
    ///
    /// Returns `None` if no fields changed (optimization to avoid storing no-ops).
    pub(super) fn from_states(before: &ActorState, after: &ActorState) -> Option<Self> {
        debug_assert_eq!(
            before.id, after.id,
            "Cannot compare actors with different IDs"
        );

        let mut fields = ActorFields::empty();

        if before.position != after.position {
            fields |= ActorFields::POSITION;
        }
        if before.core_stats != after.core_stats {
            fields |= ActorFields::CORE_STATS;
        }
        if before.resources != after.resources {
            fields |= ActorFields::RESOURCES;
        }
        if before.bonuses != after.bonuses {
            fields |= ActorFields::BONUSES;
        }
        if before.inventory != after.inventory {
            fields |= ActorFields::INVENTORY;
        }
        if before.ready_at != after.ready_at {
            fields |= ActorFields::READY_AT;
        }

        if fields.is_empty() {
            None
        } else {
            Some(Self {
                id: after.id,
                fields,
            })
        }
    }
}

/// Metadata describing which fields of a prop changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PropChanges {
    pub id: EntityId,
    pub fields: PropFields,
}

impl PropChanges {
    /// Creates prop changes by comparing two prop states.
    ///
    /// Returns `None` if no fields changed.
    pub(super) fn from_states(before: &PropState, after: &PropState) -> Option<Self> {
        debug_assert_eq!(
            before.id, after.id,
            "Cannot compare props with different IDs"
        );

        let mut fields = PropFields::empty();

        if before.position != after.position {
            fields |= PropFields::POSITION;
        }
        if before.is_active != after.is_active {
            fields |= PropFields::IS_ACTIVE;
        }

        if fields.is_empty() {
            None
        } else {
            Some(Self {
                id: after.id,
                fields,
            })
        }
    }
}

/// Metadata describing which fields of an item changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ItemChanges {
    pub id: EntityId,
    pub fields: ItemFields,
}

impl ItemChanges {
    /// Creates item changes by comparing two item states.
    ///
    /// Returns `None` if no fields changed.
    pub(super) fn from_states(before: &ItemState, after: &ItemState) -> Option<Self> {
        debug_assert_eq!(
            before.id, after.id,
            "Cannot compare items with different IDs"
        );

        let mut fields = ItemFields::empty();

        if before.position != after.position {
            fields |= ItemFields::POSITION;
        }

        if fields.is_empty() {
            None
        } else {
            Some(Self {
                id: after.id,
                fields,
            })
        }
    }
}

/// Metadata describing which fields of turn state changed.
///
/// Unlike entity changes, turn changes also include activation lists since
/// these are essential for turn scheduling and cannot be efficiently reconstructed
/// from bitmasks alone.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TurnChanges {
    pub fields: TurnFields,
    pub activated: Vec<EntityId>,
    pub deactivated: Vec<EntityId>,
}

impl TurnChanges {
    /// Creates turn changes by comparing two turn states.
    pub(super) fn from_states(before: &TurnState, after: &TurnState) -> Self {
        let mut fields = TurnFields::empty();

        if before.clock != after.clock {
            fields |= TurnFields::CLOCK;
        }
        if before.current_actor != after.current_actor {
            fields |= TurnFields::CURRENT_ACTOR;
        }
        if before.action_nonce != after.action_nonce {
            fields |= TurnFields::ACTION_NONCE;
        }

        let activated = after
            .active_actors
            .difference(&before.active_actors)
            .copied()
            .collect();

        let deactivated = before
            .active_actors
            .difference(&after.active_actors)
            .copied()
            .collect();

        Self {
            fields,
            activated,
            deactivated,
        }
    }

    /// Returns true if turn state is completely unchanged.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty() && self.activated.is_empty() && self.deactivated.is_empty()
    }
}

/// Metadata describing changes to world occupancy.
///
/// Instead of storing full occupancy data, we only track which tile positions
/// had their occupant list modified. Actual occupant lists are retrieved from
/// the before/after [`WorldState`] when needed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OccupancyChanges {
    pub position: Position,
}
