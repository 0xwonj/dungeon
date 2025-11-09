//! Damage effect implementation.

use crate::action::effect::ExecutionPhase;
use crate::action::error::ActionError;
use crate::action::execute::EffectContext;
use crate::action::formula::{Formula, evaluate};
use crate::action::types::{AppliedValue, DamageType};

/// Deal damage to target.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DamageEffect {
    pub formula: Formula,
    pub damage_type: DamageType,
    pub can_crit: bool,
}

impl DamageEffect {
    /// Create a new damage effect.
    pub fn new(formula: Formula, damage_type: DamageType) -> Self {
        Self {
            formula,
            damage_type,
            can_crit: false,
        }
    }

    /// Enable critical hits for this damage effect.
    pub fn with_crit(mut self) -> Self {
        self.can_crit = true;
        self
    }

    /// Pre-validate: No additional validation needed.
    /// Target existence is checked at action-level pre_validate.
    pub fn pre_validate(&self, _ctx: &EffectContext) -> Result<(), ActionError> {
        Ok(())
    }

    /// Apply damage to target.
    pub fn apply(&self, ctx: &mut EffectContext) -> Result<AppliedValue, ActionError> {
        // 1. Evaluate formula
        let planned = evaluate(&self.formula, ctx)?;

        // 2. Get target actor
        let actor = ctx
            .state
            .entities
            .actor_mut(ctx.target)
            .ok_or(ActionError::TargetNotFound)?;

        // 3. Calculate actual damage (capped at current HP)
        // TODO: Apply resistance/armor based on damage_type
        // TODO: Check for critical hit based on can_crit flag
        let actual_damage = planned.min(actor.resources.hp);

        // 4. Apply damage
        actor.resources.hp = actor.resources.hp.saturating_sub(actual_damage);

        // 5. Update accumulated damage in context
        ctx.accumulated_damage += actual_damage;

        Ok(AppliedValue::Damage {
            planned,
            actual: actual_damage,
        })
    }

    /// Post-validate: No additional validation needed.
    pub fn post_validate(&self, _ctx: &EffectContext) -> Result<(), ActionError> {
        Ok(())
    }

    /// Get default execution phase for damage effects.
    pub fn default_phase() -> ExecutionPhase {
        ExecutionPhase::Primary
    }
}
