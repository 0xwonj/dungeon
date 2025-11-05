//! Resource manipulation effect implementations.

use crate::action::effect::ExecutionPhase;
use crate::action::error::ActionError;
use crate::action::execute::EffectContext;
use crate::action::formula::{Formula, evaluate};
use crate::action::types::AppliedValue;
use crate::stats::ResourceKind;

/// Restore resource to target (healing, mana restore, etc.).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RestoreResourceEffect {
    pub resource: ResourceKind,
    pub formula: Formula,
    pub overfill_allowed: bool,
}

impl RestoreResourceEffect {
    /// Create a new RestoreResource effect.
    pub fn new(resource: ResourceKind, formula: Formula) -> Self {
        Self {
            resource,
            formula,
            overfill_allowed: false,
        }
    }

    /// Allow overfilling (healing above max).
    pub fn with_overfill(mut self) -> Self {
        self.overfill_allowed = true;
        self
    }

    /// Pre-validate: No additional validation needed.
    pub fn pre_validate(&self, _ctx: &EffectContext) -> Result<(), ActionError> {
        Ok(())
    }

    /// Apply resource restoration.
    pub fn apply(&self, ctx: &mut EffectContext) -> Result<AppliedValue, ActionError> {
        // 1. Evaluate formula
        let planned = evaluate(&self.formula, ctx)?;

        // 2. Restore resource
        let actual = restore_resource_to(ctx, ctx.target, self.resource, planned)?;

        // 3. Update accumulated healing if HP
        if self.resource == ResourceKind::Hp {
            ctx.accumulated_healing += actual;
            Ok(AppliedValue::Healing { planned, actual })
        } else {
            Ok(AppliedValue::ResourceChange {
                resource: self.resource,
                delta: actual as i32,
            })
        }
    }

    /// Post-validate: No additional validation needed.
    pub fn post_validate(&self, _ctx: &EffectContext) -> Result<(), ActionError> {
        Ok(())
    }

    /// Get default execution phase for RestoreResource effects.
    pub fn default_phase() -> ExecutionPhase {
        ExecutionPhase::Primary
    }
}

/// Set resource to specific value.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SetResourceEffect {
    pub resource: ResourceKind,
    pub formula: Formula,
}

impl SetResourceEffect {
    /// Create a new SetResource effect.
    pub fn new(resource: ResourceKind, formula: Formula) -> Self {
        Self { resource, formula }
    }

    /// Pre-validate: No additional validation needed.
    pub fn pre_validate(&self, _ctx: &EffectContext) -> Result<(), ActionError> {
        Ok(())
    }

    /// Apply resource set.
    pub fn apply(&self, _ctx: &mut EffectContext) -> Result<AppliedValue, ActionError> {
        // TODO: Implement SetResource
        Err(ActionError::NotImplemented(
            "SetResource effect not yet implemented".to_string(),
        ))
    }

    /// Post-validate: No additional validation needed.
    pub fn post_validate(&self, _ctx: &EffectContext) -> Result<(), ActionError> {
        Ok(())
    }

    /// Get default execution phase for SetResource effects.
    pub fn default_phase() -> ExecutionPhase {
        ExecutionPhase::Primary
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Restore resource to specific entity.
fn restore_resource_to(
    ctx: &mut EffectContext,
    entity: crate::state::EntityId,
    resource: ResourceKind,
    amount: u32,
) -> Result<u32, ActionError> {
    let actor = ctx
        .state
        .entities
        .actor_mut(entity)
        .ok_or(ActionError::TargetNotFound)?;

    let max = actor.snapshot().resource_max.get(resource);
    let current = match resource {
        ResourceKind::Hp => actor.resources.hp,
        ResourceKind::Mp => actor.resources.mp,
        ResourceKind::Lucidity => actor.resources.lucidity,
    };

    let missing = max.saturating_sub(current);
    let actual = amount.min(missing);

    // Apply restoration
    match resource {
        ResourceKind::Hp => actor.resources.hp += actual,
        ResourceKind::Mp => actor.resources.mp += actual,
        ResourceKind::Lucidity => actor.resources.lucidity += actual,
    }

    Ok(actual)
}
