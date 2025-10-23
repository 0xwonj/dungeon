//! Utility-based AI system for NPCs.
//!
//! This module provides a three-layer utility scoring system for NPC decision-making:
//!
//! 1. **Intent Selection** (Layer 1): What does the NPC want to do?
//!    - Combat, Survival, Exploration, Social, Resource, Idle
//!
//! 2. **Tactic Selection** (Layer 2): How should they achieve that intent?
//!    - AggressiveMelee, Kiting, Flee, HealAlly, etc.
//!
//! 3. **Action Selection** (Layer 3): Which specific action to execute?
//!    - Selects from available_actions based on tactic-specific scoring
//!
//! # Core Components
//!
//! - [`AiContext`]: The blackboard for AI decision-making (game state + available actions)
//! - [`UtilityAiProvider`]: Main AI provider implementing [`crate::ActionProvider`]
//! - [`Intent`] / [`Tactic`]: Strategic and tactical decision types
//! - [`scoring`]: Utility scoring functions for all three layers
//!
//! # Design Principles
//!
//! 1. **Utility-first**: All decisions use deterministic scoring functions
//! 2. **Action reuse**: All tactics share the same pool of available actions
//! 3. **Automatic composition**: NPC behavior emerges from TraitProfile (Species × Archetype × Faction × Temperament)
//! 4. **Determinism**: All decisions are purely deterministic (required for ZK proofs)
//!
//! # Example
//!
//! ```rust,ignore
//! use runtime::providers::ai::UtilityAiProvider;
//!
//! // Create AI provider (automatically uses TraitProfile)
//! let ai = UtilityAiProvider::new();
//!
//! // NPC behavior is determined by their TraitProfile
//! // Goblin + Archer + Bandit → prefers kiting, flees easily, ignores allies
//! // Orc + Warrior + Guard → prefers aggressive melee, territorial, protects allies
//! ```

pub mod context;
pub mod provider;
pub mod scoring;
pub mod types;

// Re-export core types
pub use context::AiContext;
pub use provider::UtilityAiProvider;
pub use scoring::actions::ActionScorer;
pub use scoring::selector::{IntentScorer, TacticScorer};
pub use types::{Intent, Tactic};
