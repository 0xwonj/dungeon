//! Action execution pipeline with effect-based composition.
//!
//! This module implements the three-phase action execution system:
//! 1. **Pre-validation**: Check requirements before executing (actor alive, range, resources)
//! 2. **Apply**: Execute effects in phase/priority order with accumulated context
//! 3. **Post-validation**: Check invariants after execution (state consistency)
//!
//! ## Architecture
//!
//! - **Public API**: `pre_validate`, `apply`, `post_validate` (called by game engine)
//! - **Validation**: Pre/post checks isolated in `validation` module
//! - **Pipeline**: Orchestration logic (target resolution, effect sorting) in `pipeline` module
//! - **Context**: EffectContext and effect dispatcher
//!
//! ## Effect Context
//!
//! Effects execute within an `EffectContext` that provides:
//! - Mutable game state access
//! - Environment oracles (map, tables, etc.)
//! - Accumulated results (damage, healing, critical hits)
//! - Caster and target entity IDs
//!
//! ## Error Handling
//!
//! All execution steps return `Result<_, ActionError>` with detailed error variants:
//! - Actor/target not found or dead
//! - Out of range, bounds, or blocked by terrain
//! - Insufficient resources or cooldowns
//! - Formula evaluation failures
//!
//! ## Future Extensions
//!
//! - Line of sight checking
//! - Critical hit calculation
//! - Resistance/armor calculations
//! - Status effect application
//! - Cooldown management
//! - Resource cost validation

mod context;
mod pipeline;
mod validation;

use crate::action::error::ActionError;
use crate::action::types::{ActionResult, CharacterAction};
use crate::env::GameEnv;
use crate::state::GameState;

// ============================================================================
// Public Exports
// ============================================================================

pub use context::EffectContext;

// ============================================================================
// Public API
// ============================================================================

/// Pre-validation: Check requirements before executing.
///
/// This validates:
/// - Actor exists and is alive
/// - Action profile exists in oracle
/// - Target is valid for the targeting mode
/// - Target is in range
///
/// ## Future Validations
/// - Resource costs (lucidity, mana)
/// - Cooldowns
/// - Requirements (items, status, etc.)
/// - Line of sight
///
/// ## Errors
/// - `ActionError::ActorNotFound` - Actor doesn't exist
/// - `ActionError::ActorDead` - Actor has 0 HP
/// - `ActionError::ProfileNotFound` - Action not in oracle
/// - `ActionError::InvalidTarget` - Wrong target type for action
/// - `ActionError::TargetNotFound` - Target doesn't exist
/// - `ActionError::OutOfRange` - Target too far away
pub fn pre_validate(
    action: &CharacterAction,
    state: &GameState,
    env: &GameEnv<'_>,
) -> Result<(), ActionError> {
    validation::pre_validate(action, state, env)
}

/// Apply action: Execute effects and return result.
///
/// This:
/// 1. Loads action profile from TablesOracle
/// 2. Resolves targets based on targeting mode
/// 3. Sorts effects by phase and priority
/// 4. Creates EffectContext for each target
/// 5. Applies effects in order
/// 6. Returns accumulated ActionResult
///
/// ## Effect Phases
/// - `PreEffect` (0): Setup effects (buffs, positioning)
/// - `Primary` (1): Main action effects (damage, healing)
/// - `PostEffect` (2): Conditional effects (on-hit, on-crit)
/// - `Finalize` (3): Cleanup effects (expire buffs, check death)
///
/// ## Errors
/// - `ActionError::ProfileNotFound` - Action not in oracle
/// - `ActionError::InvalidTarget` - Target resolution failed
/// - `ActionError::NotImplemented` - Unsupported effect or targeting mode
/// - Any effect-specific errors from effect application
pub fn apply(
    action: &CharacterAction,
    state: &mut GameState,
    env: &GameEnv<'_>,
) -> Result<ActionResult, ActionError> {
    pipeline::apply(action, state, env)
}

/// Post-validation: Check invariants after execution.
///
/// This validates:
/// - State consistency
/// - Resources don't exceed maximums (HP, MP, Lucidity)
/// - No invalid entity states
///
/// ## Future Validations
/// - Death state propagation
/// - Status effect consistency
/// - Position validity
///
/// Currently unimplemented (returns Ok).
pub fn post_validate(
    action: &CharacterAction,
    state: &GameState,
    env: &GameEnv<'_>,
) -> Result<(), ActionError> {
    validation::post_validate(action, state, env)
}
