//! Effect execution context and dispatcher.
//!
//! This module provides:
//! - `EffectContext`: Execution context passed to all effects
//! - `apply_effect`: Dispatcher that delegates to EffectKind implementations

use crate::action::effect::ActionEffect;
use crate::action::types::{ActionInput, EffectResult};
use crate::env::GameEnv;
use crate::state::{EntityId, GameState};

use crate::action::error::ActionError;

// ============================================================================
// Effect Context
// ============================================================================

/// Context for effect execution.
///
/// This provides all data needed for effects and tracks accumulated values
/// across multiple effects in a single action.
pub struct EffectContext<'a> {
    /// The entity performing the action.
    pub caster: EntityId,

    /// The current target entity.
    pub target: EntityId,

    /// Mutable game state.
    pub state: &'a mut GameState,

    /// Environment oracles.
    pub env: &'a GameEnv<'a>,

    /// User/AI input for this action (e.g., direction, position).
    pub action_input: &'a ActionInput,

    /// Total damage dealt in this action so far.
    pub accumulated_damage: u32,

    /// Total healing done in this action so far.
    pub accumulated_healing: u32,

    /// Whether any effect was a critical hit.
    pub was_critical: bool,
}

impl<'a> EffectContext<'a> {
    /// Creates a new effect context.
    pub fn new(
        caster: EntityId,
        target: EntityId,
        state: &'a mut GameState,
        env: &'a GameEnv<'a>,
        action_input: &'a ActionInput,
    ) -> Self {
        Self {
            caster,
            target,
            state,
            env,
            action_input,
            accumulated_damage: 0,
            accumulated_healing: 0,
            was_critical: false,
        }
    }
}

// ============================================================================
// Effect Dispatcher
// ============================================================================

/// Apply a single effect to current context and return the result.
///
/// This delegates to EffectKind::apply() which dispatches to individual effect implementations.
pub(super) fn apply_effect(
    effect: &ActionEffect,
    ctx: &mut EffectContext,
) -> Result<EffectResult, ActionError> {
    // Delegate to EffectKind's apply method (defined in effect/kinds.rs)
    let applied_value = effect.kind.apply(ctx)?;
    Ok(EffectResult::new(ctx.target, applied_value))
}
