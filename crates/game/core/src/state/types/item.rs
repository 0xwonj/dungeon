//! Item-related state types.
//!
//! This module contains foundational item types:
//! - ItemHandle: Reference to item definitions
//! - ItemState: Items that exist on the ground (world items)

use super::{EntityId, Position};

/// Reference to an item definition stored outside the core (lookup via Env).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ItemHandle(pub u32);

/// Items that exist on the ground (not inside inventories).
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ItemState {
    pub id: EntityId,
    pub position: Position,
    pub handle: ItemHandle,
    pub quantity: u16,
}

impl ItemState {
    pub fn new(id: EntityId, position: Position, handle: ItemHandle, quantity: u16) -> Self {
        Self {
            id,
            position,
            handle,
            quantity,
        }
    }
}
