use crate::state::ItemHandle;
use crate::state::types::{ArmorKind, WeaponKind};

pub trait ItemOracle: Send + Sync {
    fn definition(&self, handle: ItemHandle) -> Option<ItemDefinition>;

    /// Returns all item definitions available in this oracle.
    /// Used for creating complete ItemsSnapshot for zkVM execution.
    #[cfg(feature = "std")]
    fn all_definitions(&self) -> Vec<ItemDefinition>;
}

/// Item definition with common fields and type-specific data.
///
/// # Design: Base + Kind Pattern
///
/// - Base struct holds common fields (handle, max_stack)
/// - `kind` enum holds type-specific data (weapon stats, consumable effects, etc.)
/// - Display data (name, description) provided by oracle separately if needed
///
/// # Stacking
///
/// All items have a `max_stack` value:
/// - Weapons/Armor: max_stack=1 (cannot stack)
/// - Consumables: max_stack=99 (stackable)
/// - Keys: max_stack=1 (unique keys don't stack)
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ItemDefinition {
    pub handle: ItemHandle,
    pub kind: ItemKind,
    pub max_stack: u16,
}

impl ItemDefinition {
    pub fn new(handle: ItemHandle, kind: ItemKind, max_stack: u16) -> Self {
        Self {
            handle,
            kind,
            max_stack,
        }
    }
}

/// Item type with type-specific data.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ItemKind {
    /// Equippable weapon.
    Weapon(WeaponData),

    /// Equippable armor.
    Armor(ArmorData),

    /// Consumable item (potions, scrolls, food).
    Consumable(ConsumableData),

    /// Key for unlocking doors/chests.
    Key { door_id: u16 },

    /// Utility item.
    Utility,

    /// Custom item type.
    Custom(u16),
}

/// Weapon-specific data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WeaponData {
    pub kind: WeaponKind,
    pub damage: u16,
}

/// Armor-specific data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ArmorData {
    pub kind: ArmorKind,
    pub defense: u16,
}

/// Consumable-specific data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ConsumableData {
    pub effect: ConsumableEffect,
    pub use_cost: u32,
}

/// Consumable effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ConsumableEffect {
    /// Restore health.
    HealHealth(u16),

    /// Restore mana.
    RestoreMana(u16),

    /// Teleport to specific location.
    Teleport,

    /// Custom effect.
    Custom(u16),
}
