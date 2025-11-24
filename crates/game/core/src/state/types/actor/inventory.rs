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

    /// Check if inventory is full.
    pub fn is_full(&self) -> bool {
        self.items.is_full()
    }

    /// Get item at slot index.
    pub fn get_slot(&self, slot: usize) -> Option<&InventorySlot> {
        self.items.get(slot)
    }

    /// Get mutable item at slot index.
    pub fn get_slot_mut(&mut self, slot: usize) -> Option<&mut InventorySlot> {
        self.items.get_mut(slot)
    }

    /// Add item to inventory. If same item exists, stack it. Otherwise create new slot.
    ///
    /// Returns error if inventory is full and item cannot be stacked.
    pub fn add_item(&mut self, handle: ItemHandle, quantity: u16) -> Result<(), &'static str> {
        // Try to find existing slot with same handle
        for slot in &mut self.items {
            if slot.handle == handle {
                // Stack with existing item
                slot.quantity = slot.quantity.saturating_add(quantity);
                return Ok(());
            }
        }

        // No existing slot, create new one
        if self.items.is_full() {
            return Err("Inventory is full");
        }

        self.items
            .try_push(InventorySlot::new(handle, quantity))
            .map_err(|_| "Inventory is full")?;

        Ok(())
    }

    /// Decrease quantity of item in slot. Remove slot if quantity reaches 0.
    ///
    /// Returns error if slot is empty or quantity is insufficient.
    pub fn decrease_quantity(&mut self, slot: usize, amount: u16) -> Result<(), &'static str> {
        let slot_item = self.items.get_mut(slot).ok_or("Inventory slot is empty")?;

        if slot_item.quantity < amount {
            return Err("Insufficient quantity");
        }

        slot_item.quantity -= amount;

        // Remove slot if quantity reaches 0
        if slot_item.quantity == 0 {
            self.items.remove(slot);
        }

        Ok(())
    }

    /// Remove item slot entirely.
    pub fn remove_slot(&mut self, slot: usize) -> Option<InventorySlot> {
        if slot < self.items.len() {
            Some(self.items.remove(slot))
        } else {
            None
        }
    }
}
