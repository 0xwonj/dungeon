//! Inventory system for actors.
//!
//! Defines inventory storage and item slots for actors.

use arrayvec::ArrayVec;

use crate::config::GameConfig;
use crate::state::types::ItemHandle;

/// Inventory slot containing an item and its quantity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InventorySlot {
    pub handle: ItemHandle,
    pub quantity: u16,
}

impl InventorySlot {
    pub fn new(handle: ItemHandle, quantity: u16) -> Self {
        Self { handle, quantity }
    }
}

/// Simplified inventory snapshot; expand as item systems mature.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InventoryState {
    pub items: ArrayVec<InventorySlot, { GameConfig::MAX_INVENTORY_SLOTS }>,
}

impl InventoryState {
    pub fn new(items: ArrayVec<InventorySlot, { GameConfig::MAX_INVENTORY_SLOTS }>) -> Self {
        Self { items }
    }

    pub fn empty() -> Self {
        Self {
            items: ArrayVec::new(),
        }
    }
}
