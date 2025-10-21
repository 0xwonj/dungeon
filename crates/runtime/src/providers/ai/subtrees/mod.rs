//! Reusable behavior tree subtrees.
//!
//! This module provides a hierarchical library of composable behavior patterns
//! for building NPC AI. Subtrees are organized into three layers:
//!
//! - **Layer 1 (Patterns)**: Simple if-then patterns combining conditions with actions
//! - **Layer 2 (Tactics)**: Goal-oriented behaviors combining multiple patterns
//! - **Layer 3 (Strategies)**: Decision-making logic selecting tactics based on state
//!
//! # Architecture
//!
//! ```text
//! Layer 3 (strategies)
//!     ├─ aggressive_melee()
//!     │   ├─ flee_when_low_health()     ← Layer 1
//!     │   └─ melee_engagement()         ← Layer 2
//!     │       ├─ attack_when_adjacent() ← Layer 1
//!     │       └─ chase_player()         ← Layer 1
//! ```
//!
//! # Design Principles
//!
//! 1. **Composability**: Every subtree can be combined with others
//! 2. **Single Responsibility**: Each subtree has one clear purpose
//! 3. **Parameterization**: Similar behaviors are parameterized, not duplicated
//! 4. **Type Safety**: All subtrees return `BehaviorTree` for consistency
//!
//! # Example
//!
//! ```rust,ignore
//! use runtime::providers::ai::subtrees::{patterns, tactics, strategies};
//!
//! // Use pre-built strategy
//! let ai = strategies::aggressive_melee(0.2);
//!
//! // Or compose custom AI
//! let custom_ai = Selector::new(vec![
//!     patterns::flee_when_low_health(0.3),
//!     tactics::melee_engagement(),
//!     patterns::wait_fallback(),
//! ]);
//! ```

pub mod patterns;
pub mod strategies;
pub mod tactics;

use behavior_tree::Behavior;

use super::context::AiContext;

/// Type alias for behavior trees to reduce verbosity.
///
/// All subtree functions return this type for consistency.
pub type BehaviorTree = Box<dyn Behavior<AiContext<'static>>>;
