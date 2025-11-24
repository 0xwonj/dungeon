//! Equipment system for actors.
//!
//! Defines what items an actor has equipped, which determines their
//! available combat actions and provides stat bonuses.

use crate::state::types::ItemHandle;

/// Equipment state for an actor.
///
/// Represents what items are currently equipped. Equipment stores handles
/// to items in the actor's inventory that are currently equipped.
///
/// Equipment provides:
/// - Stat bonuses (via `compute_actor_bonuses`)
/// - Available actions (melee, ranged, magic attacks)
///
/// # Design
///
/// Equipment slots reference items by `ItemHandle`. The actual item data
/// (including its kind, stats, etc.) is stored in the inventory.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Equipment {
    /// Currently equipped weapon (determines attack type).
    pub weapon: Option<ItemHandle>,

    /// Currently equipped armor (provides defense and may restrict actions).
    pub armor: Option<ItemHandle>,
}

impl Equipment {
    /// Creates empty equipment (no weapon or armor).
    pub fn empty() -> Self {
        Self::default()
    }

    /// Creates a builder for constructing equipment.
    pub fn builder() -> EquipmentBuilder {
        EquipmentBuilder::default()
    }

    /// Equips a weapon, returning the previously equipped weapon handle if any.
    pub fn equip_weapon(&mut self, handle: ItemHandle) -> Option<ItemHandle> {
        self.weapon.replace(handle)
    }

    /// Unequips the current weapon, returning its handle if any was equipped.
    pub fn unequip_weapon(&mut self) -> Option<ItemHandle> {
        self.weapon.take()
    }

    /// Equips armor, returning the previously equipped armor handle if any.
    pub fn equip_armor(&mut self, handle: ItemHandle) -> Option<ItemHandle> {
        self.armor.replace(handle)
    }

    /// Unequips the current armor, returning its handle if any was equipped.
    pub fn unequip_armor(&mut self) -> Option<ItemHandle> {
        self.armor.take()
    }
}

/// Builder for constructing equipment.
#[derive(Default)]
pub struct EquipmentBuilder {
    weapon: Option<ItemHandle>,
    armor: Option<ItemHandle>,
}

impl EquipmentBuilder {
    /// Sets the weapon by item handle.
    pub fn weapon(mut self, handle: ItemHandle) -> Self {
        self.weapon = Some(handle);
        self
    }

    /// Sets the armor by item handle.
    pub fn armor(mut self, handle: ItemHandle) -> Self {
        self.armor = Some(handle);
        self
    }

    /// Builds the equipment.
    pub fn build(self) -> Equipment {
        Equipment {
            weapon: self.weapon,
            armor: self.armor,
        }
    }
}
