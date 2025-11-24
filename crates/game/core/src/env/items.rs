use crate::state::ItemHandle;

pub trait ItemOracle: Send + Sync {
    fn definition(&self, handle: ItemHandle) -> Option<ItemDefinition>;

    /// Returns all item definitions available in this oracle.
    /// Used for creating complete ItemsSnapshot for zkVM execution.
    #[cfg(feature = "std")]
    fn all_definitions(&self) -> Vec<ItemDefinition>;
}

// ============================================================================
// Item Type Definitions
// ============================================================================

/// Weapon types that determine attack capabilities.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum WeaponKind {
    // Melee weapons
    Sword,
    Dagger,
    Axe,
    Spear,

    // Ranged weapons
    Bow,
    Crossbow,

    // Magic weapons
    Staff,
    Wand,

    // Unarmed (default for NPCs without weapons)
    Unarmed,
}

/// Attack type determined by weapon.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AttackType {
    /// Melee attack (adjacent range).
    Melee,
    /// Ranged attack (long distance).
    Ranged,
    /// Magic attack (varies by spell).
    Magic,
}

impl WeaponKind {
    /// Get the attack type for this weapon.
    pub fn attack_type(&self) -> AttackType {
        match self {
            WeaponKind::Sword
            | WeaponKind::Dagger
            | WeaponKind::Axe
            | WeaponKind::Spear
            | WeaponKind::Unarmed => AttackType::Melee,

            WeaponKind::Bow | WeaponKind::Crossbow => AttackType::Ranged,

            WeaponKind::Staff | WeaponKind::Wand => AttackType::Magic,
        }
    }

    /// Get the melee range for this weapon (in tiles).
    ///
    /// Most melee weapons have range 1 (adjacent only).
    /// Spears have extended range of 2.
    pub fn melee_range(&self) -> u32 {
        match self {
            WeaponKind::Spear => 2,
            _ => 1,
        }
    }
}

/// Armor types that provide defense and may restrict certain actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ArmorKind {
    /// Light armor (leather) - allows stealth, minimal defense.
    Light,

    /// Medium armor (chainmail) - balanced defense and mobility.
    Medium,

    /// Heavy armor (plate) - maximum defense, restricts stealth and some movement.
    Heavy,
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
#[derive(Clone, Debug, PartialEq)]
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
#[derive(Clone, Debug, PartialEq)]
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
///
/// Consumables use the same ActionEffect system as actions.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ConsumableData {
    /// Effects applied when this consumable is used.
    pub effects: Vec<crate::action::ActionEffect>,

    /// Action cost to use this consumable (0 = free action).
    pub use_cost: u32,
}
