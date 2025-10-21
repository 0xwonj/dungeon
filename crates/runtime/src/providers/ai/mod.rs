//! Behavior tree-based AI system for NPCs.
//!
//! This module provides a complete, layered AI architecture using behavior trees.
//! The system is organized into four layers of abstraction:
//!
//! # Architecture Layers
//!
//! - **Layer 0 ([`nodes`])**: Atomic conditions and actions (building blocks)
//! - **Layer 1 ([`subtrees::patterns`])**: Simple if-then patterns
//! - **Layer 2 ([`subtrees::tactics`])**: Goal-oriented tactical behaviors
//! - **Layer 3 ([`subtrees::strategies`])**: Decision-making strategies
//! - **Layer 4 ([`presets`])**: Complete NPC AI definitions
//!
//! # Core Components
//!
//! - [`AiContext`]: The blackboard for AI decision-making (read game state, write actions)
//! - [`BehaviorTreeProvider`]: Adapter that implements [`crate::ActionProvider`] using BTs
//! - [`nodes`]: Atomic behavior tree nodes (conditions like `IsHealthLow`, actions like `AttackPlayer`)
//! - [`subtrees`]: Reusable behavior tree patterns organized by complexity
//! - [`presets`]: Ready-to-use AI for different NPC types
//!
//! # Usage Examples
//!
//! ## Using Pre-built AI
//!
//! ```rust,ignore
//! use runtime::providers::ai::{BehaviorTreeProvider, presets};
//!
//! // Simple: Use a pre-built NPC AI
//! let goblin_ai = BehaviorTreeProvider::new(presets::goblin());
//! runtime_builder.provider(ProviderKind::Ai(AiKind::Goblin), goblin_ai);
//! ```
//!
//! ## Composing Custom AI from Strategies
//!
//! ```rust,ignore
//! use runtime::providers::ai::{BehaviorTreeProvider, subtrees};
//! use behavior_tree::Selector;
//!
//! // Intermediate: Compose from strategies
//! let custom_ai = BehaviorTreeProvider::new(
//!     Box::new(Selector::new(vec![
//!         subtrees::strategies::aggressive_melee(0.3),
//!         subtrees::patterns::wait_fallback(),
//!     ]))
//! );
//! ```
//!
//! ## Building from Scratch with Patterns
//!
//! ```rust,ignore
//! use runtime::providers::ai::{BehaviorTreeProvider, subtrees};
//! use behavior_tree::Selector;
//!
//! // Advanced: Full control with patterns
//! let custom_ai = BehaviorTreeProvider::new(
//!     Box::new(Selector::new(vec![
//!         subtrees::patterns::flee_when_low_health(0.25),
//!         subtrees::tactics::melee_engagement(),
//!         subtrees::patterns::wait_fallback(),
//!     ]))
//! );
//! ```
//!
//! # Design Principles
//!
//! 1. **Composability**: Every layer can be freely combined with others
//! 2. **Reusability**: Common patterns are abstracted into reusable subtrees
//! 3. **Type Safety**: All subtrees return the same `BehaviorTree` type
//! 4. **Determinism**: AI decisions are purely deterministic (required for ZK proofs)
//! 5. **Zero Runtime Cost**: Abstraction layers compile to direct function calls

pub mod context;
pub mod nodes;
pub mod presets;
pub mod provider;
pub mod subtrees;

pub use context::AiContext;
pub use provider::BehaviorTreeProvider;
