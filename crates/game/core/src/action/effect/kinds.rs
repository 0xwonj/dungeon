//! Effect kind enum and dispatcher.
//!
//! This module defines the EffectKind enum which wraps all concrete effect types.
//! It provides serialization support and dispatches to individual effect implementations.

use crate::action::formula::Formula;
use crate::action::types::DamageType;
use crate::state::Tick;
use crate::state::types::StatusEffectKind;
use crate::stats::ResourceKind;

use super::condition::Condition;
use super::damage::DamageEffect;
use super::displacement::Displacement;
use super::interaction::InteractionType;
use super::movement::{MoveSelfEffect, MoveTargetEffect, SwapEffect};
use super::resource::{RestoreResourceEffect, SetResourceEffect};
use super::status::{ApplyStatusEffect, ClearBuffsEffect, ClearDebuffsEffect, RemoveStatusEffect};

/// The actual effect to apply.
///
/// This enum wraps all concrete effect types and provides serialization support.
/// Each variant delegates to its corresponding struct implementation.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EffectKind {
    // ========================================================================
    // Damage
    // ========================================================================
    Damage(DamageEffect),

    // ========================================================================
    // Resource Manipulation
    // ========================================================================
    RestoreResource(RestoreResourceEffect),
    SetResource(SetResourceEffect),

    // ========================================================================
    // Status Effects
    // ========================================================================
    ApplyStatus(ApplyStatusEffect),
    RemoveStatus(RemoveStatusEffect),
    ClearDebuffs(ClearDebuffsEffect),
    ClearBuffs(ClearBuffsEffect),

    // ========================================================================
    // Movement
    // ========================================================================
    MoveSelf(MoveSelfEffect),
    MoveTarget(MoveTargetEffect),
    Swap(SwapEffect),

    // ========================================================================
    // Complex/Unimplemented (keeping as enum variants for now)
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

    /// Interact with world object.
    Interact {
        interaction_type: InteractionType,
    },

    /// Conditional effect (if-then-else).
    Conditional {
        condition: Condition,
        then_effects: Vec<super::ActionEffect>,
        else_effects: Vec<super::ActionEffect>,
    },

    /// Repeat effect N times.
    Repeat {
        effect: Box<super::ActionEffect>,
        count: u32,
    },
}

// Backward compatibility: constructors matching old EffectKind variants
impl EffectKind {
    /// Create a Damage effect (backward compatibility).
    pub fn damage(formula: Formula, damage_type: DamageType, can_crit: bool) -> Self {
        Self::Damage(DamageEffect {
            formula,
            damage_type,
            can_crit,
        })
    }

    /// Create a RestoreResource effect (backward compatibility).
    pub fn restore_resource(
        resource: ResourceKind,
        formula: Formula,
        overfill_allowed: bool,
    ) -> Self {
        Self::RestoreResource(RestoreResourceEffect {
            resource,
            formula,
            overfill_allowed,
        })
    }

    /// Create a MoveSelf effect (backward compatibility).
    pub fn move_self(displacement: Displacement) -> Self {
        Self::MoveSelf(MoveSelfEffect { displacement })
    }

    /// Create a MoveTarget effect (backward compatibility).
    pub fn move_target(displacement: Displacement) -> Self {
        Self::MoveTarget(MoveTargetEffect { displacement })
    }

    /// Create an ApplyStatus effect (backward compatibility).
    pub fn apply_status(status: StatusEffectKind, duration: Tick) -> Self {
        Self::ApplyStatus(ApplyStatusEffect { status, duration })
    }
}

// ============================================================================
// Three-Phase Execution (Dispatcher)
// ============================================================================

impl EffectKind {
    /// Pre-validate: Check requirements before applying.
    ///
    /// Dispatches to individual effect's pre_validate method.
    pub fn pre_validate(
        &self,
        ctx: &crate::action::execute::EffectContext,
    ) -> Result<(), crate::action::error::ActionError> {
        match self {
            Self::Damage(e) => e.pre_validate(ctx),
            Self::RestoreResource(e) => e.pre_validate(ctx),
            Self::SetResource(e) => e.pre_validate(ctx),
            Self::ApplyStatus(e) => e.pre_validate(ctx),
            Self::RemoveStatus(e) => e.pre_validate(ctx),
            Self::ClearDebuffs(e) => e.pre_validate(ctx),
            Self::ClearBuffs(e) => e.pre_validate(ctx),
            Self::MoveSelf(e) => e.pre_validate(ctx),
            Self::MoveTarget(e) => e.pre_validate(ctx),
            Self::Swap(e) => e.pre_validate(ctx),

            // Unimplemented effects - no validation yet
            Self::Summon { .. }
            | Self::Transform { .. }
            | Self::Interact { .. }
            | Self::Conditional { .. }
            | Self::Repeat { .. } => Ok(()),
        }
    }

    /// Apply: Execute the effect and return result.
    ///
    /// Dispatches to individual effect's apply method.
    pub fn apply(
        &self,
        ctx: &mut crate::action::execute::EffectContext,
    ) -> Result<crate::action::types::AppliedValue, crate::action::error::ActionError> {
        match self {
            Self::Damage(e) => e.apply(ctx),
            Self::RestoreResource(e) => e.apply(ctx),
            Self::SetResource(e) => e.apply(ctx),
            Self::ApplyStatus(e) => e.apply(ctx),
            Self::RemoveStatus(e) => e.apply(ctx),
            Self::ClearDebuffs(e) => e.apply(ctx),
            Self::ClearBuffs(e) => e.apply(ctx),
            Self::MoveSelf(e) => e.apply(ctx),
            Self::MoveTarget(e) => e.apply(ctx),
            Self::Swap(e) => e.apply(ctx),

            // Unimplemented effects
            Self::Summon { .. } => Err(crate::action::error::ActionError::NotImplemented(
                "Summon effect not yet implemented".to_string(),
            )),
            Self::Transform { .. } => Err(crate::action::error::ActionError::NotImplemented(
                "Transform effect not yet implemented".to_string(),
            )),
            Self::Interact { .. } => Err(crate::action::error::ActionError::NotImplemented(
                "Interact effect not yet implemented".to_string(),
            )),
            Self::Conditional { .. } => Err(crate::action::error::ActionError::NotImplemented(
                "Conditional effect not yet implemented".to_string(),
            )),
            Self::Repeat { .. } => Err(crate::action::error::ActionError::NotImplemented(
                "Repeat effect not yet implemented".to_string(),
            )),
        }
    }

    /// Post-validate: Check invariants after applying.
    ///
    /// Dispatches to individual effect's post_validate method.
    pub fn post_validate(
        &self,
        ctx: &crate::action::execute::EffectContext,
    ) -> Result<(), crate::action::error::ActionError> {
        match self {
            Self::Damage(e) => e.post_validate(ctx),
            Self::RestoreResource(e) => e.post_validate(ctx),
            Self::SetResource(e) => e.post_validate(ctx),
            Self::ApplyStatus(e) => e.post_validate(ctx),
            Self::RemoveStatus(e) => e.post_validate(ctx),
            Self::ClearDebuffs(e) => e.post_validate(ctx),
            Self::ClearBuffs(e) => e.post_validate(ctx),
            Self::MoveSelf(e) => e.post_validate(ctx),
            Self::MoveTarget(e) => e.post_validate(ctx),
            Self::Swap(e) => e.post_validate(ctx),

            // Unimplemented effects - no validation yet
            Self::Summon { .. }
            | Self::Transform { .. }
            | Self::Interact { .. }
            | Self::Conditional { .. }
            | Self::Repeat { .. } => Ok(()),
        }
    }
}
