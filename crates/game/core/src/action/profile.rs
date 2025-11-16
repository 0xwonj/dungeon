//! Action profile definitions - complete specification for each action.
//!
//! ActionProfile defines the behavior, costs, targeting, and effects for each
//! action type. Profiles are loaded from RON data files via TablesOracle.

use crate::action::effect::ActionEffect;
use crate::action::targeting::TargetingMode;
use crate::state::Tick;
use crate::stats::ResourceKind;

/// Types of actions that can be performed.
///
/// Each variant represents a specific action an entity can perform.
/// Action behavior is defined in RON data files via ActionProfile.
///
/// NOTE: Commented variants are not yet implemented (missing RON data files).
/// Uncomment and implement as needed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ActionKind {
    // ========================================================================
    // Basic Actions
    // ========================================================================
    /// Move to an adjacent tile.
    Move,

    /// Wait and do nothing.
    Wait,

    /// Pick up an item from the ground.
    PickupItem,

    /// Use an item from inventory.
    UseItem,

    // /// Interact with props (doors, levers, etc.).
    // Interact,

    // ========================================================================
    // Combat - Melee
    // ========================================================================
    /// Basic melee attack.
    MeleeAttack,
    // /// Powerful melee attack with extra damage.
    // PowerAttack,
    //
    // /// High damage when attacking from behind.
    // Backstab,
    //
    // /// Attack multiple adjacent enemies.
    // Cleave,

    // ========================================================================
    // Combat - Ranged
    // ========================================================================
    // /// Basic ranged attack.
    // RangedAttack,
    //
    // /// Aimed shot with bonus accuracy.
    // AimedShot,

    // ========================================================================
    // Magic - Offensive
    // ========================================================================
    // /// Fire magic attack.
    // Fireball,
    //
    // /// Lightning magic attack.
    // Lightning,

    // ========================================================================
    // Magic - Support
    // ========================================================================
    // /// Heal self or allies.
    // Heal,
    //
    // /// Create protective barrier.
    // Shield,
    //
    // /// Teleport to nearby location.
    // Teleport,

    // ========================================================================
    // Movement
    // ========================================================================
    // /// Dash 2+ tiles in one action.
    // Dash,

    // ========================================================================
    // Stealth
    // ========================================================================
    // /// Become invisible for several turns.
    // Stealth,
    //
    // /// Attack with bonus damage from stealth.
    // SneakAttack,

    // ========================================================================
    // Social
    // ========================================================================
    // /// Call nearby allies.
    // CallAllies,
    //
    // /// Frighten enemies.
    // Intimidate,
    //
    // /// Buff nearby allies.
    // Rally,
}

impl ActionKind {
    /// Returns the snake_case string representation of this action kind.
    ///
    /// This is used for generating file names, logging, and serialization keys.
    pub fn as_snake_case(self) -> &'static str {
        match self {
            // Basic Actions
            ActionKind::Move => "move",
            ActionKind::Wait => "wait",
            ActionKind::PickupItem => "pickup_item",
            ActionKind::UseItem => "use_item",

            // Combat - Melee
            ActionKind::MeleeAttack => "melee_attack",
        }
    }

    /// Returns all ActionKind variants.
    ///
    /// This is used for iterating all possible actions, e.g., when creating snapshots.
    pub fn all_variants() -> &'static [ActionKind] {
        &[
            // Basic Actions
            ActionKind::Move,
            ActionKind::Wait,
            ActionKind::PickupItem,
            ActionKind::UseItem,
            // Combat - Melee
            ActionKind::MeleeAttack,
        ]
    }
}

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
