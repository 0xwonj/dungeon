//! Layer 2: Goal-oriented tactical behaviors.
//!
//! Tactics combine multiple patterns to achieve specific combat goals.
//! Each tactic answers the question: "How do I accomplish X?"
//!
//! # Tactical Categories
//!
//! - **Engagement**: How to close distance and attack
//! - **Disengagement**: How to create distance (future)
//! - **Positioning**: How to maintain optimal range (future)
//!
//! # Design Pattern
//!
//! Tactics use the **Strategy Pattern**: they define a family of algorithms
//! (how to achieve a goal) that can be selected at runtime by higher-level
//! strategies.
//!
//! # Naming Convention
//!
//! Tactics are named as `noun_verb()` to clearly express the goal:
//! - `melee_engagement()` - "engage in melee combat"
//! - `ranged_attack()` - "attack from range"
//! - `kiting()` - "maintain distance while attacking"

use behavior_tree::Selector;

use super::{patterns, BehaviorTree};

// ============================================================================
// Engagement Tactics
// ============================================================================

/// Melee engagement: close distance and attack.
///
/// This tactic combines two patterns to achieve the goal of engaging in
/// melee combat:
/// 1. If adjacent to player → Attack
/// 2. If not adjacent → Chase (close distance)
///
/// This is the fundamental aggressive melee behavior used by most
/// close-combat NPCs.
///
/// # Example
///
/// ```rust,ignore
/// // Simple aggressive AI
/// Selector::new(vec![
///     tactics::melee_engagement(),
///     patterns::wait_fallback(),
/// ])
/// ```
pub fn melee_engagement() -> BehaviorTree {
    Box::new(Selector::new(vec![
        patterns::attack_when_adjacent(),
        patterns::chase_player(),
    ]))
}
