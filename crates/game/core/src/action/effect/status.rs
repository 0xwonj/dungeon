//! Status effect implementations.

use crate::action::effect::ExecutionPhase;
use crate::action::error::ActionError;
use crate::action::execute::EffectContext;
use crate::action::types::AppliedValue;
use crate::state::Tick;
use crate::state::types::status::StatusEffectKind;

/// Apply status effect to target.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ApplyStatusEffect {
    pub status: StatusEffectKind,
    pub duration: Tick,
}

impl ApplyStatusEffect {
    /// Create a new ApplyStatus effect.
    pub fn new(status: StatusEffectKind, duration: Tick) -> Self {
        Self { status, duration }
    }

    /// Pre-validate: No additional validation needed.
    pub fn pre_validate(&self, _ctx: &EffectContext) -> Result<(), ActionError> {
        Ok(())
    }

    /// Apply status effect.
    pub fn apply(&self, _ctx: &mut EffectContext) -> Result<AppliedValue, ActionError> {
        // TODO: Implement status effects
        Err(ActionError::NotImplemented(
            "Status effects not yet implemented".to_string(),
        ))
    }

    /// Post-validate: No additional validation needed.
    pub fn post_validate(&self, _ctx: &EffectContext) -> Result<(), ActionError> {
        Ok(())
    }

    /// Get default execution phase for ApplyStatus effects.
    pub fn default_phase() -> ExecutionPhase {
        ExecutionPhase::PostEffect
    }
}

/// Remove status effect from target.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RemoveStatusEffect {
    pub status: StatusEffectKind,
}

impl RemoveStatusEffect {
    /// Create a new RemoveStatus effect.
    pub fn new(status: StatusEffectKind) -> Self {
        Self { status }
    }

    /// Pre-validate: No additional validation needed.
    pub fn pre_validate(&self, _ctx: &EffectContext) -> Result<(), ActionError> {
        Ok(())
    }

    /// Apply status removal.
    pub fn apply(&self, _ctx: &mut EffectContext) -> Result<AppliedValue, ActionError> {
        // TODO: Implement status effects
        Err(ActionError::NotImplemented(
            "Status effects not yet implemented".to_string(),
        ))
    }

    /// Post-validate: No additional validation needed.
    pub fn post_validate(&self, _ctx: &EffectContext) -> Result<(), ActionError> {
        Ok(())
    }

    /// Get default execution phase for RemoveStatus effects.
    pub fn default_phase() -> ExecutionPhase {
        ExecutionPhase::Finalize
    }
}

/// Clear all debuffs from target.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ClearDebuffsEffect;

impl ClearDebuffsEffect {
    /// Create a new ClearDebuffs effect.
    pub fn new() -> Self {
        Self
    }

    /// Pre-validate: No additional validation needed.
    pub fn pre_validate(&self, _ctx: &EffectContext) -> Result<(), ActionError> {
        Ok(())
    }

    /// Apply debuff clearing.
    pub fn apply(&self, _ctx: &mut EffectContext) -> Result<AppliedValue, ActionError> {
        // TODO: Implement status effects
        Err(ActionError::NotImplemented(
            "Status effects not yet implemented".to_string(),
        ))
    }

    /// Post-validate: No additional validation needed.
    pub fn post_validate(&self, _ctx: &EffectContext) -> Result<(), ActionError> {
        Ok(())
    }

    /// Get default execution phase for ClearDebuffs effects.
    pub fn default_phase() -> ExecutionPhase {
        ExecutionPhase::PostEffect
    }
}

impl Default for ClearDebuffsEffect {
    fn default() -> Self {
        Self::new()
    }
}

/// Clear all buffs from target.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ClearBuffsEffect;

impl ClearBuffsEffect {
    /// Create a new ClearBuffs effect.
    pub fn new() -> Self {
        Self
    }

    /// Pre-validate: No additional validation needed.
    pub fn pre_validate(&self, _ctx: &EffectContext) -> Result<(), ActionError> {
        Ok(())
    }

    /// Apply buff clearing.
    pub fn apply(&self, _ctx: &mut EffectContext) -> Result<AppliedValue, ActionError> {
        // TODO: Implement status effects
        Err(ActionError::NotImplemented(
            "Status effects not yet implemented".to_string(),
        ))
    }

    /// Post-validate: No additional validation needed.
    pub fn post_validate(&self, _ctx: &EffectContext) -> Result<(), ActionError> {
        Ok(())
    }

    /// Get default execution phase for ClearBuffs effects.
    pub fn default_phase() -> ExecutionPhase {
        ExecutionPhase::PostEffect
    }
}

impl Default for ClearBuffsEffect {
    fn default() -> Self {
        Self::new()
    }
}
