//! Action effect system - atomic, composable effects that actions apply.
//!
//! Effects are the fundamental building blocks of actions. An action is simply
//! a sequence of effects applied to resolved targets.
//!
//! # Design Principles
//!
//! 1. **Generalization**: Effects operate on resources, not specific types
//!    - `RestoreResource(Hp, Formula::Constant(50))` instead of `Heal(50)`
//!    - Enables arbitrary resource manipulation
//!
//! 2. **Composition**: Complex effects via sequencing
//!    - DrainLife = `Damage + RestoreResource(Hp, FromPreviousDamage(50%))`
//!    - No special-case effects
//!
//! 3. **Phases**: Explicit execution ordering
//!    - PreEffect → Primary → PostEffect → Finalize
//!    - Clear, predictable behavior

use crate::combat::DamageType;
use crate::state::Tick;
use crate::state::types::status::StatusEffectKind;
use crate::stats::{CoreStatKind, ResourceKind};

// ============================================================================
// Execution Phases
// ============================================================================

/// Execution phase for effect ordering.
///
/// Effects within the same action are executed in phase order.
/// Within the same phase, effects execute in definition order (or by priority).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ExecutionPhase {
    /// Before main effects (buffs, debuffs setup).
    PreEffect = 0,

    /// Main damage/healing phase.
    Primary = 1,

    /// After main effects (lifesteal, on-hit effects).
    PostEffect = 2,

    /// Final effects (stacks, cooldowns, cleanup).
    Finalize = 3,
}

impl Default for ExecutionPhase {
    fn default() -> Self {
        Self::Primary
    }
}

// ============================================================================
// Formula System (Unified Value Calculation)
// ============================================================================

/// Formula for calculating numeric values.
///
/// Replaces separate DamageFormula/HealFormula with a unified system.
/// Formulas can reference stats, resources, weapon damage, or previous effects.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Formula {
    /// Fixed constant value.
    Constant(u32),

    /// Percentage of caster's stat.
    CasterStat {
        stat: CoreStatKind,
        percent: u32, // 100 = 1.0x
    },

    /// Percentage of target's stat.
    TargetStat { stat: CoreStatKind, percent: u32 },

    /// Percentage of caster's current resource.
    CasterResource {
        resource: ResourceKind,
        percent: u32,
    },

    /// Percentage of target's current resource.
    TargetResource {
        resource: ResourceKind,
        percent: u32,
    },

    /// Percentage of target's missing resource (max - current).
    TargetMissingResource {
        resource: ResourceKind,
        percent: u32,
    },

    /// Percentage of target's max resource.
    TargetMaxResource {
        resource: ResourceKind,
        percent: u32,
    },

    /// Percentage of weapon damage.
    WeaponDamage { percent: u32 },

    /// Percentage of damage dealt in previous effects (this action).
    FromPreviousDamage { percent: u32 },

    /// Percentage of healing done in previous effects (this action).
    FromPreviousHealing { percent: u32 },

    /// Sum of multiple formulas.
    Sum(Vec<Formula>),

    /// Product of multiple formulas (result = f1 * f2 * ... / 100^(n-1)).
    Product(Vec<Formula>),

    /// Minimum of multiple formulas.
    Min(Vec<Formula>),

    /// Maximum of multiple formulas.
    Max(Vec<Formula>),
}

// ============================================================================
// Movement & Displacement
// ============================================================================

/// How to determine displacement for movement effects.
///
/// Unified type for all movement: Move, Teleport, Knockback, Pull, Dash, Charge, etc.
/// Distance of 0 means "teleport directly to target/position".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Displacement {
    /// Move in direction specified by ActionInput::Direction.
    ///
    /// Effect reads direction from context.action_input at runtime.
    /// Used for player-controlled movement, charges, etc.
    FromInput { distance: u32 },

    /// Move toward target entity (charge, dash, approach).
    TowardTarget { distance: u32 },

    /// Move away from target entity (retreat, flee).
    AwayFromTarget { distance: u32 },

    /// Move away from caster (knockback, repel).
    AwayFromCaster { distance: u32 },

    /// Teleport to position specified by ActionInput::Position.
    ///
    /// Effect reads position from context.action_input at runtime.
    ToInputPosition,

    /// Teleport to random valid position within range.
    RandomInRange { range: u32 },
}

// ============================================================================
// Interaction
// ============================================================================

/// Type of interaction with world objects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum InteractionType {
    Open,
    Close,
    PickUp,
    Use,
    Talk,
}

// ============================================================================
// Conditions
// ============================================================================

/// Condition for conditional effects.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Condition {
    /// Target's resource is below threshold (percentage).
    TargetResourceBelow {
        resource: ResourceKind,
        percent: u32,
    },

    /// Target's resource is above threshold (percentage).
    TargetResourceAbove {
        resource: ResourceKind,
        percent: u32,
    },

    /// Caster's resource is below threshold (percentage).
    CasterResourceBelow {
        resource: ResourceKind,
        percent: u32,
    },

    /// Caster's resource is above threshold (percentage).
    CasterResourceAbove {
        resource: ResourceKind,
        percent: u32,
    },

    /// Target has status effect.
    TargetHasStatus(StatusEffectKind),

    /// Caster has status effect.
    CasterHasStatus(StatusEffectKind),

    /// Target is behind caster.
    TargetBehind,

    /// Random chance (percentage, 0-100).
    RandomChance(u32),

    /// Previous effect was critical.
    WasCritical,

    /// All conditions must be true.
    And(Vec<Condition>),

    /// Any condition must be true.
    Or(Vec<Condition>),

    /// Condition must be false.
    Not(Box<Condition>),
}

// ============================================================================
// Effect Kinds
// ============================================================================

/// The actual effect to apply.
///
/// This is wrapped in `ActionEffect` with phase and priority.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EffectKind {
    // ========================================================================
    // Damage
    // ========================================================================
    /// Deal damage to target.
    Damage {
        formula: Formula,
        damage_type: DamageType,
        can_crit: bool,
    },

    // ========================================================================
    // Resource Manipulation (generalized)
    // ========================================================================
    /// Restore resource to target (healing, mana restore, etc.).
    RestoreResource {
        resource: ResourceKind,
        formula: Formula,
        overfill_allowed: bool,
    },

    /// Drain resource from target.
    DrainResource {
        resource: ResourceKind,
        formula: Formula,
        transfer_to_caster: bool,
    },

    /// Set resource to specific value.
    SetResource {
        resource: ResourceKind,
        formula: Formula,
    },

    // ========================================================================
    // Status Effects
    // ========================================================================
    /// Apply status effect to target.
    ApplyStatus {
        status: StatusEffectKind,
        duration: Tick,
    },

    /// Remove status effect from target.
    RemoveStatus { status: StatusEffectKind },

    /// Clear all debuffs.
    ClearDebuffs,

    /// Clear all buffs.
    ClearBuffs,

    // ========================================================================
    // Movement
    // ========================================================================
    /// Move the caster.
    MoveSelf { displacement: Displacement },

    /// Move the target.
    MoveTarget { displacement: Displacement },

    /// Swap positions with target.
    Swap,

    // ========================================================================
    // Summon & Transform
    // ========================================================================
    /// Summon entity.
    Summon {
        template_id: String,
        count: u32,
        duration: Option<Tick>,
    },

    /// Transform caster into different form.
    Transform {
        into_template: String,
        duration: Option<Tick>,
    },

    // ========================================================================
    // Utility
    // ========================================================================
    /// Interact with world object.
    Interact { interaction_type: InteractionType },

    /// Conditional effect (if-then-else).
    Conditional {
        condition: Condition,
        then_effects: Vec<ActionEffect>,
        else_effects: Vec<ActionEffect>,
    },

    /// Repeat effect N times.
    Repeat {
        effect: Box<ActionEffect>,
        count: u32,
    },
}

// ============================================================================
// Action Effect (with phase and priority)
// ============================================================================

/// Complete effect specification with execution ordering.
///
/// Actions contain a list of these, executed in phase/priority order.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ActionEffect {
    /// The actual effect to apply.
    pub kind: EffectKind,

    /// Execution phase (determines ordering).
    #[cfg_attr(feature = "serde", serde(default))]
    pub phase: ExecutionPhase,

    /// Priority within phase (higher = earlier, default = 0).
    #[cfg_attr(feature = "serde", serde(default))]
    pub priority: i32,
}

impl ActionEffect {
    /// Creates an effect with default phase (Primary) and priority (0).
    pub fn new(kind: EffectKind) -> Self {
        Self {
            kind,
            phase: ExecutionPhase::Primary,
            priority: 0,
        }
    }

    /// Creates an effect with specified phase.
    pub fn with_phase(kind: EffectKind, phase: ExecutionPhase) -> Self {
        Self {
            kind,
            phase,
            priority: 0,
        }
    }

    /// Creates an effect with phase and priority.
    pub fn with_priority(kind: EffectKind, phase: ExecutionPhase, priority: i32) -> Self {
        Self {
            kind,
            phase,
            priority,
        }
    }

    /// Builder: set phase.
    pub fn phase(mut self, phase: ExecutionPhase) -> Self {
        self.phase = phase;
        self
    }

    /// Builder: set priority.
    pub fn priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
}
