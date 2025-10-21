//! Layer 3: Strategic decision-making behaviors.
//!
//! Strategies combine tactics with survival logic to create adaptive
//! behaviors. Each strategy answers: "When should I use which tactic?"
//!
//! # Strategic Categories
//!
//! - **Aggressive**: Prioritize offense over defense
//! - **Defensive**: Prioritize survival over aggression
//! - **Balanced**: Mix offense and defense based on conditions
//!
//! # Design Pattern
//!
//! Strategies implement the **Template Method Pattern**: they define the
//! skeleton of decision-making (survival check → tactic selection) while
//! allowing subclasses (different strategies) to vary specific steps.
//!
//! # Naming Convention
//!
//! Strategies are named as `adjective_noun()` to describe their characteristics:
//! - `aggressive_melee()` - aggressive approach to melee combat
//! - `defensive_melee()` - defensive approach to melee combat
//! - `balanced_melee()` - balanced approach to melee combat

use behavior_tree::Selector;

use super::{patterns, tactics, BehaviorTree};

// ============================================================================
// Melee Strategies
// ============================================================================

/// Aggressive melee strategy: fight until critical health.
///
/// This strategy prioritizes offense with minimal self-preservation:
/// 1. If health is critically low → Flee
/// 2. Otherwise → Engage in melee
///
/// # Arguments
///
/// * `flee_threshold` - Health ratio (0.0-1.0) below which to flee
///
/// # Decision Logic
///
/// The flee threshold determines how "aggressive" this strategy is:
/// - Low threshold (0.1-0.2): Very aggressive, fights until nearly dead
/// - Medium threshold (0.3-0.4): Moderately aggressive
/// - High threshold (0.5+): Cautious, flees early
///
/// # Example
///
/// ```rust,ignore
/// // Goblin: fights until 20% health
/// strategies::aggressive_melee(0.2)
///
/// // Orc: more cautious, flees at 40%
/// strategies::aggressive_melee(0.4)
/// ```
pub fn aggressive_melee(flee_threshold: f32) -> BehaviorTree {
    Box::new(Selector::new(vec![
        patterns::flee_when_low_health(flee_threshold),
        tactics::melee_engagement(),
    ]))
}
