//! Layer 1: Basic if-then patterns.
//!
//! This module provides simple building blocks that combine one condition
//! with one action. These are the smallest reusable units in the behavior
//! tree library.
//!
//! # Pattern Types
//!
//! - **Combat**: Attack patterns based on distance/adjacency
//! - **Survival**: Flee/defensive behaviors based on health
//! - **Movement**: Basic movement toward/away from player
//! - **Fallback**: Safe default behaviors
//!
//! # Naming Convention
//!
//! Patterns follow the `action_when_condition()` convention to clearly
//! express the if-then relationship.

use behavior_tree::Sequence;

use crate::providers::ai::nodes::{
    AttackPlayer, IsAdjacentToPlayer, IsHealthLow, MoveTowardPlayer, Wait,
};

use super::BehaviorTree;

// ============================================================================
// Combat Patterns
// ============================================================================

/// Attack player when adjacent (melee range).
///
/// This is the fundamental melee combat pattern. Returns Success if the
/// attack action is generated, Failure if not adjacent.
///
/// # Example
///
/// ```rust,ignore
/// Selector::new(vec![
///     patterns::attack_when_adjacent(),
///     patterns::chase_player(),
/// ])
/// ```
pub fn attack_when_adjacent() -> BehaviorTree {
    Box::new(Sequence::new(vec![
        Box::new(IsAdjacentToPlayer),
        Box::new(AttackPlayer),
    ]))
}

// ============================================================================
// Survival Patterns
// ============================================================================

/// Flee (wait in place) when health is below threshold.
///
/// This is a simplified flee behavior. The entity stops pursuing and waits
/// when critically wounded. In the future, this could be enhanced to
/// actually move away from the player.
///
/// # Arguments
///
/// * `threshold` - Health ratio (0.0-1.0) below which to flee
///
/// # Example
///
/// ```rust,ignore
/// // Aggressive: only flee at 20% health
/// patterns::flee_when_low_health(0.2)
///
/// // Cautious: flee at 40% health
/// patterns::flee_when_low_health(0.4)
/// ```
pub fn flee_when_low_health(threshold: f32) -> BehaviorTree {
    Box::new(Sequence::new(vec![
        Box::new(IsHealthLow { threshold }),
        Box::new(Wait),
    ]))
}

// ============================================================================
// Movement Patterns
// ============================================================================

/// Move toward the player.
///
/// This pattern wraps the MoveTowardPlayer action node for consistency
/// with other patterns. It calculates the best direction to approach
/// the player and generates a move action.
///
/// # Example
///
/// ```rust,ignore
/// Selector::new(vec![
///     patterns::attack_when_adjacent(),
///     patterns::chase_player(),  // If not adjacent, chase
/// ])
/// ```
pub fn chase_player() -> BehaviorTree {
    Box::new(MoveTowardPlayer)
}

// ============================================================================
// Fallback Patterns
// ============================================================================

/// Wait (do nothing) as a safe fallback.
///
/// This pattern always succeeds by generating a Wait action. It ensures
/// that behavior trees never fail completely by providing a guaranteed
/// success path.
///
/// Every complete AI should include this as the final selector option.
///
/// # Example
///
/// ```rust,ignore
/// Selector::new(vec![
///     patterns::attack_when_adjacent(),
///     patterns::chase_player(),
///     patterns::wait_fallback(),  // Always succeeds
/// ])
/// ```
pub fn wait_fallback() -> BehaviorTree {
    Box::new(Wait)
}
