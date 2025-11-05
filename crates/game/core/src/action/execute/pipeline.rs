//! Action execution orchestration pipeline.
//!
//! This module coordinates the execution flow:
//! 1. Load action profile from oracle
//! 2. Resolve targets based on targeting mode
//! 3. Sort effects by phase and priority
//! 4. Create effect context for each target
//! 5. Apply effects in order
//! 6. Return accumulated result
//!
//! ## Design Principles
//!
//! - **Stateless**: All state passed explicitly (no hidden globals)
//! - **Deterministic**: Same inputs always produce same outputs
//! - **Composable**: Effects execute independently with shared context
//! - **Fail-fast**: Any error stops execution and propagates up

use crate::action::TargetingMode;
use crate::action::types::{ActionInput, ActionResult, CharacterAction};
use crate::env::GameEnv;
use crate::state::{EntityId, GameState};

use super::context::{EffectContext, apply_effect};
use crate::action::error::ActionError;

// ============================================================================
// Pipeline Orchestration
// ============================================================================

/// Apply action: Execute effects and return result.
///
/// ## Execution Flow
/// 1. Load `ActionProfile` from `TablesOracle`
/// 2. Resolve targets using `resolve_targets`
/// 3. For each target:
///    - Sort effects by phase (PreEffect → Primary → PostEffect → Finalize)
///    - Within same phase, sort by priority (higher first)
///    - Create `EffectContext` with mutable state access
///    - Apply each effect via `apply_effect`
///    - Collect `EffectResult` for each effect
/// 4. Build and return `ActionResult` with all effect results
///
/// ## Phase Execution Order
/// - `PreEffect` (0): Setup, positioning, buffs
/// - `Primary` (1): Main damage/healing
/// - `PostEffect` (2): On-hit effects, conditionals
/// - `Finalize` (3): Cleanup, death checks
///
/// ## Error Handling
/// Any error during effect application stops execution immediately.
pub(super) fn apply(
    action: &CharacterAction,
    state: &mut GameState,
    env: &GameEnv<'_>,
) -> Result<ActionResult, ActionError> {
    // 1. Load action profile
    let profile = env
        .tables()
        .map_err(|_| ActionError::ProfileNotFound)?
        .action_profile(action.kind);

    // 2. Resolve targets
    let targets = resolve_targets(action, state, env, &profile)?;

    // 3. Collect all effect results
    let mut effect_results = Vec::new();

    // 4. Execute effects for each target
    for target in targets {
        // Sort effects by phase and priority
        let mut effects = profile.effects.clone();
        effects.sort_by(|a, b| {
            a.phase
                .cmp(&b.phase)
                .then_with(|| b.priority.cmp(&a.priority)) // Higher priority first
        });

        // Create effect context
        let mut ctx = EffectContext::new(action.actor, target, state, env, &action.input);

        // Apply effects in order with three-phase execution
        for effect in &effects {
            // Phase 1: Pre-validate (check requirements before state changes)
            effect.kind.pre_validate(&ctx)?;

            // Phase 2: Apply (mutate state and get result)
            let effect_result = apply_effect(effect, &mut ctx)?;
            effect_results.push(effect_result);

            // Phase 3: Post-validate (check invariants after state changes)
            effect.kind.post_validate(&ctx)?;
        }
    }

    // 5. Build ActionResult from collected effect results
    Ok(ActionResult::from_effects(effect_results))
}

// ============================================================================
// Target Resolution
// ============================================================================

/// Resolve targets based on action targeting mode.
///
/// ## Targeting Modes
/// - `None`: No targets (empty vec)
/// - `SelfOnly`: Actor as target
/// - `SingleTarget`: Single entity from action.targets
/// - `Directional`: Actor as target (for movement actions)
fn resolve_targets(
    action: &CharacterAction,
    _state: &GameState,
    _env: &GameEnv<'_>,
    profile: &crate::action::ActionProfile,
) -> Result<Vec<EntityId>, ActionError> {
    match &profile.targeting {
        TargetingMode::None => Ok(vec![]),

        TargetingMode::SelfOnly => Ok(vec![action.actor]),

        TargetingMode::SingleTarget { .. } => {
            if let ActionInput::Entity(target) = action.input {
                Ok(vec![target])
            } else {
                Err(ActionError::InvalidTarget)
            }
        }

        TargetingMode::Directional { .. } => {
            // For movement actions, return actor as target
            Ok(vec![action.actor])
        }
    }
}
