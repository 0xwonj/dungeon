//! Action profile definitions - complete specification for each action.
//!
//! ActionProfile defines the behavior, costs, targeting, and effects for each
//! action type. Profiles are loaded from RON data files via TablesOracle.

use crate::action::ActionKind;
use crate::action::effect::ActionEffect;
use crate::action::targeting::TargetingMode;
use crate::state::Tick;
use crate::stats::ResourceKind;

/// Tags for gameplay logic (AI, rules, synergies).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ActionTag {
    // Damage Types
    Physical,
    Fire,
    Cold,
    Lightning,
    Poison,
    Arcane,

    // Action Types
    Attack,
    Spell,
    Movement,

    // Delivery Methods
    Melee,
    Ranged,
    Projectile,
    Aoe,

    // Schools
    Offensive,
    Defensive,
    Utility,

    // Special Flags
    Channeled,
    Instant,
    Interruptible,
}

/// Resource cost for an action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ResourceCost {
    pub resource: ResourceKind,
    pub amount: u32,
}

impl ResourceCost {
    pub fn hp(amount: u32) -> Self {
        Self {
            resource: ResourceKind::Hp,
            amount,
        }
    }

    pub fn mp(amount: u32) -> Self {
        Self {
            resource: ResourceKind::Mp,
            amount,
        }
    }

    pub fn lucidity(amount: u32) -> Self {
        Self {
            resource: ResourceKind::Lucidity,
            amount,
        }
    }
}

/// Requirement for using an action.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Requirement {
    /// Requires a weapon equipped.
    WeaponEquipped,

    /// Requires attacking from behind.
    TargetBehind,

    /// Requires minimum HP percentage.
    MinHpPercent(u32),

    /// Requires maximum HP percentage.
    MaxHpPercent(u32),
}

/// Complete specification for an action.
///
/// Profiles are loaded from RON data files via TablesOracle.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ActionProfile {
    /// The action kind this profile describes.
    pub kind: ActionKind,

    /// Tags for gameplay logic (AI, rules, synergies).
    pub tags: Vec<ActionTag>,

    /// How this action selects targets.
    pub targeting: TargetingMode,

    /// Base time cost in ticks (before speed scaling).
    pub base_cost: Tick,

    /// Resource costs.
    pub resource_costs: Vec<ResourceCost>,

    /// Effects to apply (in sequence) to each target.
    pub effects: Vec<ActionEffect>,

    /// Requirements to use this action.
    pub requirements: Vec<Requirement>,

    /// Cooldown duration (if any).
    pub cooldown: Option<Tick>,
}
