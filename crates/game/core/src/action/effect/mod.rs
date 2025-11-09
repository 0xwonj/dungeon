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
//!
//! 4. **Three-Phase Execution**: Each effect implements:
//!    - `pre_validate`: Check requirements before any state changes
//!    - `apply`: Mutate state and return result
//!    - `post_validate`: Verify invariants after changes
//!
//! # Architecture
//!
//! This module uses an **Enum + Struct hybrid** approach:
//! - Individual effect types are structs (DamageEffect, MoveSelfEffect, etc.)
//! - EffectKind enum wraps them for serialization and dispatching
//! - Each struct implements pre_validate/apply/post_validate methods
//!
//! This provides:
//! - ✅ Serialization support (via enum)
//! - ✅ ZK compatibility (static dispatch in enum match)
//! - ✅ Code organization (each effect in separate file)
//! - ✅ Explicit validation phases

mod condition;
mod damage;
mod displacement;
mod interaction;
mod kinds;
mod movement;
mod phase;
mod resource;
mod status;

// Re-export core types
pub use condition::Condition;
pub use damage::DamageEffect;
pub use displacement::Displacement;
pub use interaction::InteractionType;
pub use kinds::EffectKind;
pub use movement::{MoveSelfEffect, MoveTargetEffect, SwapEffect};
pub use phase::ExecutionPhase;
pub use resource::{RestoreResourceEffect, SetResourceEffect};
pub use status::{ApplyStatusEffect, ClearBuffsEffect, ClearDebuffsEffect, RemoveStatusEffect};

// ============================================================================
// Action Effect (with phase and priority)
// ============================================================================

/// Complete effect specification with execution ordering.
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
