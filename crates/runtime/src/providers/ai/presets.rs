//! Layer 4: Complete AI definitions for NPC types.
//!
//! This module provides ready-to-use behavior trees for different NPC archetypes.
//! Each AI is a complete, production-ready behavior that handles all situations
//! with appropriate fallbacks.
//!
//! # Architecture
//!
//! AI definitions are built by composing subtrees from layers 1-3:
//!
//! ```text
//! goblin()
//!   └─ Selector
//!       ├─ strategies::aggressive_melee(0.2)  ← Layer 3
//!       │   ├─ patterns::flee_when_low_health(0.2)  ← Layer 1
//!       │   └─ tactics::melee_engagement()          ← Layer 2
//!       │       ├─ patterns::attack_when_adjacent() ← Layer 1
//!       │       └─ patterns::chase_player()         ← Layer 1
//!       └─ patterns::wait_fallback()  ← Layer 1
//! ```
//!
//! # Organization
//!
//! - **Basic NPCs**: Common enemy types (goblin, skeleton, orc)
//! - **Special NPCs**: Utility/testing behaviors (passive, training_dummy)
//!
//! # Usage
//!
//! ```rust,ignore
//! use runtime::providers::ai::{BehaviorTreeProvider, presets};
//!
//! // Use pre-built AI
//! runtime_builder.provider(
//!     ProviderKind::Ai(AiKind::Aggressive),
//!     BehaviorTreeProvider::new(presets::goblin())
//! );
//! ```

use behavior_tree::{Behavior, Selector};

use super::{context::AiContext, subtrees};

/// Type alias for behavior trees to match subtrees.
pub type BehaviorTree = Box<dyn Behavior<AiContext<'static>>>;

// ============================================================================
// Basic Melee NPCs
// ============================================================================

/// Goblin: Aggressive melee fighter.
///
/// # Arguments
///
/// * `flee_threshold` - Health ratio (0.0-1.0) below which to flee.
///   - Lower values = more aggressive (fights longer)
///   - Higher values = more cautious (flees earlier)
///   - Typical range: 0.1 - 0.3
///   - Default: 0.2
///
/// # Behavior
///
/// 1. Flee when health < threshold
/// 2. Attack if adjacent, otherwise chase
/// 3. Wait if no action possible
///
/// # Examples
///
/// ```rust,ignore
/// // Standard goblin (flees at 20%)
/// presets::goblin(0.2)
///
/// // Weak goblin (flees at 30%)
/// presets::goblin(0.3)
///
/// // Elite goblin (flees at 10%)
/// presets::goblin(0.1)
/// ```
pub fn goblin(flee_threshold: f32) -> BehaviorTree {
    Box::new(Selector::new(vec![
        subtrees::strategies::aggressive_melee(flee_threshold),
        subtrees::patterns::wait_fallback(),
    ]))
}

/// Skeleton: Fearless undead melee fighter.
///
/// # Characteristics
///
/// - **Aggression**: Maximum - never flees
/// - **Tactics**: Pure melee engagement
/// - **Difficulty**: Easy - no self-preservation
///
/// # Behavior
///
/// 1. Attack if adjacent, otherwise chase
/// 2. Wait if no action possible
///
/// # Note
///
/// Skeletons do not flee, so they have no configuration options.
/// They are mindless undead that fight to destruction.
pub fn skeleton() -> BehaviorTree {
    Box::new(Selector::new(vec![
        subtrees::tactics::melee_engagement(),
        subtrees::patterns::wait_fallback(),
    ]))
}

/// Orc: Cautious melee fighter.
///
/// # Arguments
///
/// * `flee_threshold` - Health ratio (0.0-1.0) below which to flee.
///   - Lower values = more aggressive (fights longer)
///   - Higher values = more cautious (flees earlier)
///   - Typical range: 0.3 - 0.5
///   - Default: 0.4
///
/// # Behavior
///
/// 1. Flee when health < threshold
/// 2. Attack if adjacent, otherwise chase
/// 3. Wait if no action possible
///
/// # Examples
///
/// ```rust,ignore
/// // Standard orc (flees at 40%)
/// presets::orc(0.4)
///
/// // Reckless orc (flees at 20%)
/// presets::orc(0.2)
///
/// // Coward orc (flees at 50%)
/// presets::orc(0.5)
/// ```
pub fn orc(flee_threshold: f32) -> BehaviorTree {
    Box::new(Selector::new(vec![
        subtrees::strategies::aggressive_melee(flee_threshold),
        subtrees::patterns::wait_fallback(),
    ]))
}

// ============================================================================
// Special/Utility NPCs
// ============================================================================

/// Training dummy: Stationary target that never acts.
///
/// # Characteristics
///
/// - **Aggression**: None
/// - **Tactics**: No-op
/// - **Difficulty**: None - completely harmless
///
/// # Behavior
///
/// Always waits, never takes any action. This entity exists purely as
/// a static presence in the world.
///
/// # Use Cases
///
/// - **Training targets**: Practice dummies for testing player abilities
/// - **Stationary NPCs**: Non-hostile NPCs that don't move (guards, statues, etc.)
/// - **Default fallback**: Safe fallback for entities without assigned providers
/// - **Testing**: Predictable no-op behavior for automated tests
/// - **Disabled entities**: Temporarily disable an entity without removing it
///
/// # Example
///
/// ```rust,ignore
/// // Create a training dummy NPC
/// runtime_builder.provider(
///     ProviderKind::Ai(AiKind::Dummy),
///     BehaviorTreeProvider::new(presets::dummy())
/// );
/// ```
pub fn dummy() -> BehaviorTree {
    subtrees::patterns::wait_fallback()
}
