//! Action kind enumeration - all possible action types.

/// Types of actions that can be performed.
///
/// Each variant represents a specific action an entity can perform.
/// Action behavior is defined in RON data files via ActionProfile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ActionKind {
    // ========================================================================
    // Basic Actions
    // ========================================================================
    /// Move to an adjacent tile.
    Move,

    /// Wait and do nothing.
    Wait,

    /// Use an item from inventory.
    UseItem,

    /// Interact with props (doors, levers, etc.).
    Interact,

    // ========================================================================
    // Combat - Melee
    // ========================================================================
    /// Basic melee attack.
    MeleeAttack,

    /// Powerful melee attack with extra damage.
    PowerAttack,

    /// High damage when attacking from behind.
    Backstab,

    /// Attack multiple adjacent enemies.
    Cleave,

    // ========================================================================
    // Combat - Ranged
    // ========================================================================
    /// Basic ranged attack.
    RangedAttack,

    /// Aimed shot with bonus accuracy.
    AimedShot,

    // ========================================================================
    // Magic - Offensive
    // ========================================================================
    /// Fire magic attack.
    Fireball,

    /// Lightning magic attack.
    Lightning,

    // ========================================================================
    // Magic - Support
    // ========================================================================
    /// Heal self or allies.
    Heal,

    /// Create protective barrier.
    Shield,

    /// Teleport to nearby location.
    Teleport,

    // ========================================================================
    // Movement
    // ========================================================================
    /// Dash 2+ tiles in one action.
    Dash,

    // ========================================================================
    // Stealth
    // ========================================================================
    /// Become invisible for several turns.
    Stealth,

    /// Attack with bonus damage from stealth.
    SneakAttack,

    // ========================================================================
    // Social
    // ========================================================================
    /// Call nearby allies.
    CallAllies,

    /// Frighten enemies.
    Intimidate,

    /// Buff nearby allies.
    Rally,
}
