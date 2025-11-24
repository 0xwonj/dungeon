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
/// 1. Capture actor snapshot for cost calculation (before state changes)
/// 2. Load `ActionProfile` from `TablesOracle`
/// 3. Resolve targets using `resolve_targets`
/// 4. For each target:
///    - Sort effects by phase (PreEffect → Primary → PostEffect → Finalize)
///    - Within same phase, sort by priority (higher first)
///    - Create `EffectContext` with mutable state access
///    - Apply each effect via `apply_effect`
///    - Collect `EffectResult` for each effect
/// 5. Apply action cost to actor's ready_at timestamp
/// 6. Build and return `ActionResult` with all effect results
///
/// ## Phase Execution Order
/// - `PreEffect` (0): Setup, positioning, buffs
/// - `Primary` (1): Main damage/healing
/// - `PostEffect` (2): On-hit effects, conditionals
/// - `Finalize` (3): Cleanup, death checks
///
/// ## Action Cost Application
/// The action cost is calculated based on the actor's stats BEFORE the action
/// executes (to prevent self-buffs from affecting the current action's cost).
/// After effects are applied, the cost is added to the actor's ready_at timestamp.
///
/// ## Error Handling
/// Any error during effect application stops execution immediately.
pub(super) fn apply(
    action: &CharacterAction,
    state: &mut GameState,
    env: &GameEnv<'_>,
) -> Result<ActionResult, ActionError> {
    // 1. Capture actor snapshot BEFORE execution for cost calculation
    let actor_snapshot = state
        .entities
        .actor(action.actor)
        .ok_or(ActionError::ActorNotFound)?
        .snapshot();

    // 2. Calculate action cost using pre-execution stats
    let action_wrapper = crate::action::Action::character(action.clone());
    let cost = action_wrapper.cost(&actor_snapshot, env);

    // 3. Load action profile
    let profile = env
        .actions()
        .map_err(|_| ActionError::ProfileNotFound)?
        .action_profile(action.kind);

    // 4. Resolve targets
    let targets = resolve_targets(action, state, env, &profile)?;

    // 5. Collect all effect results
    let mut effect_results = Vec::new();

    // 6. Execute effects for each target
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

    // 7. Apply action cost to actor's ready_at timestamp
    // This happens AFTER all effects to ensure effects don't accidentally modify
    // the ready_at that we're trying to update
    if let Some(actor) = state.entities.actor_mut(action.actor)
        && let Some(ready_at) = actor.ready_at
    {
        actor.ready_at = Some(ready_at + cost);
    }

    // 8. Build ActionResult from collected effect results
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
            if let ActionInput::Target(target) = action.input {
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
