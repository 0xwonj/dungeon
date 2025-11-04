//! Action validation - pre and post execution checks.
//!
//! This module contains all validation logic for the action execution pipeline.
//!
//! ## Pre-validation
//!
//! Checks performed before action execution:
//! - Actor exists and is alive
//! - Action profile exists
//! - Target validity and range
//!
//! ## Post-validation
//!
//! Checks performed after action execution:
//! - State consistency
//! - Resource bounds
//!
//! ## Design Notes
//!
//! Validation is separated from execution to:
//! - Enable early rejection (fail fast)
//! - Support future optimizations (batch validation)
//! - Keep execution logic clean

use crate::action::TargetingMode;
use crate::action::types::{ActionInput, CharacterAction};
use crate::env::GameEnv;
use crate::state::{GameState, Position};

// ============================================================================
// Action Errors
// ============================================================================

/// Errors that can occur during action execution.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ActionError {
    /// Actor not found in game state.
    ActorNotFound,

    /// Actor is dead (HP = 0).
    ActorDead,

    /// Target not found in game state.
    TargetNotFound,

    /// Action profile not found in oracle.
    ProfileNotFound,

    /// Invalid target for this action.
    InvalidTarget,

    /// Out of range.
    OutOfRange,

    /// Position is out of map bounds.
    OutOfBounds,

    /// Position is invalid (e.g., wall, impassable terrain).
    InvalidPosition,

    /// Position is blocked by terrain.
    Blocked,

    /// Position is occupied by another entity.
    Occupied,

    /// Map oracle not available.
    MapNotAvailable,

    /// Insufficient resources (lucidity, mana).
    InsufficientResources,

    /// Action is on cooldown.
    OnCooldown,

    /// Requirements not met.
    RequirementsNotMet(String),

    /// Effect application failed.
    EffectFailed(String),

    /// Formula evaluation failed.
    FormulaEvaluationFailed(String),

    /// Not yet implemented.
    NotImplemented(String),
}

impl core::fmt::Display for ActionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ActionError::ActorNotFound => write!(f, "Actor not found"),
            ActionError::ActorDead => write!(f, "Actor is dead"),
            ActionError::TargetNotFound => write!(f, "Target not found"),
            ActionError::ProfileNotFound => write!(f, "Action profile not found"),
            ActionError::InvalidTarget => write!(f, "Invalid target"),
            ActionError::OutOfRange => write!(f, "Out of range"),
            ActionError::OutOfBounds => write!(f, "Position out of bounds"),
            ActionError::InvalidPosition => write!(f, "Invalid position"),
            ActionError::Blocked => write!(f, "Position blocked by terrain"),
            ActionError::Occupied => write!(f, "Position occupied by entity"),
            ActionError::MapNotAvailable => write!(f, "Map not available"),
            ActionError::InsufficientResources => write!(f, "Insufficient resources"),
            ActionError::OnCooldown => write!(f, "Action is on cooldown"),
            ActionError::RequirementsNotMet(msg) => write!(f, "Requirements not met: {}", msg),
            ActionError::EffectFailed(msg) => write!(f, "Effect failed: {}", msg),
            ActionError::FormulaEvaluationFailed(msg) => {
                write!(f, "Formula evaluation failed: {}", msg)
            }
            ActionError::NotImplemented(msg) => write!(f, "Not implemented: {}", msg),
        }
    }
}

// ============================================================================
// Pre-validation
// ============================================================================

/// Pre-validation: Check requirements before executing.
///
/// ## Validation Steps
/// 1. Actor existence and liveness
/// 2. Action profile lookup
/// 3. Target validation based on targeting mode
/// 4. Range checking (Chebyshev distance)
///
/// ## Future Extensions
/// - Resource cost validation
/// - Cooldown checking
/// - Requirement checking (items, buffs, etc.)
/// - Line of sight validation
pub(super) fn pre_validate(
    action: &CharacterAction,
    state: &GameState,
    env: &GameEnv<'_>,
) -> Result<(), ActionError> {
    // 1. Check actor exists and is alive
    let actor = state
        .entities
        .actor(action.actor)
        .ok_or(ActionError::ActorNotFound)?;

    if actor.resources.hp == 0 {
        return Err(ActionError::ActorDead);
    }

    // 2. Load action profile
    let profile = env
        .tables()
        .map_err(|_| ActionError::ProfileNotFound)?
        .action_profile(action.kind);

    // 3. Validate target based on targeting mode
    validate_targeting(action, state, &profile.targeting)?;

    // TODO: Future validations
    // - Check resources (lucidity, mana)
    // - Check cooldowns
    // - Check requirements
    // - Check line of sight

    Ok(())
}

/// Validate targeting based on action's targeting mode.
fn validate_targeting(
    action: &CharacterAction,
    state: &GameState,
    targeting: &TargetingMode,
) -> Result<(), ActionError> {
    match targeting {
        TargetingMode::None | TargetingMode::SelfOnly => {
            // No target validation needed
            Ok(())
        }

        TargetingMode::SingleTarget {
            range,
            requires_los: _,
        } => {
            // Must have a single entity input
            let target_id = match action.input {
                ActionInput::Entity(id) => id,
                _ => return Err(ActionError::InvalidTarget),
            };

            // Check target exists
            let actor = state
                .entities
                .actor(action.actor)
                .ok_or(ActionError::ActorNotFound)?;
            let target = state
                .entities
                .actor(target_id)
                .ok_or(ActionError::TargetNotFound)?;

            // Check range (Chebyshev distance)
            let distance = calculate_distance(actor.position, target.position);
            if distance > *range {
                return Err(ActionError::OutOfRange);
            }

            Ok(())
        }

        TargetingMode::Directional { range: _, width: _ } => {
            // Must have a direction input
            match action.input {
                ActionInput::Direction(_) => Ok(()),
                _ => Err(ActionError::InvalidTarget),
            }
        }
    }
}

/// Calculate Chebyshev distance (chessboard distance) between two positions.
///
/// This is `max(|dx|, |dy|)`, which treats diagonal movement as having the same
/// cost as orthogonal movement (like a chess king).
fn calculate_distance(from: Position, to: Position) -> u32 {
    let dx = (from.x - to.x).abs();
    let dy = (from.y - to.y).abs();
    dx.max(dy) as u32
}

// ============================================================================
// Post-validation
// ============================================================================

/// Post-validation: Check invariants after execution.
///
/// ## Future Validations
/// - HP/MP/Lucidity don't exceed maximums
/// - No entities with invalid states
/// - Death state propagation
/// - Status effect consistency
///
/// Currently unimplemented (always returns Ok).
pub(super) fn post_validate(
    _action: &CharacterAction,
    _state: &GameState,
    _env: &GameEnv<'_>,
) -> Result<(), ActionError> {
    // TODO: Implement post-validation
    Ok(())
}
