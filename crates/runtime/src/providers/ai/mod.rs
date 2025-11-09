//! Goal-based AI system for NPCs.
//!
//! This module provides a simple, natural approach to NPC decision-making:
//!
//! 1. **Goal Selection**: Pick a concrete goal based on situation and traits
//!    - Examples: "Attack Player", "Flee from Player", "Heal Self", "Idle"
//!
//! 2. **Candidate Generation**: Generate all possible action+input combinations
//!    - For each available ActionKind, generate valid ActionInput variants
//!    - Example: Move action → 8 directional candidates
//!
//! 3. **Evaluation**: Score each candidate by how well it serves the goal
//!    - Each goal has custom scoring logic (0-100 points)
//!    - Example: "Flee from Player" → Moving away scores 100, towards scores 0
//!
//! 4. **Selection**: Execute the highest-scoring candidate
//!
//! # Core Components
//!
//! - [`GoalBasedAiProvider`]: Main AI provider implementing [`crate::ActionProvider`]
//! - [`Goal`]: Goal enum with evaluation logic for each goal type
//! - [`GoalSelector`]: Selects goal based on HP, distance, traits, etc.
//! - [`ActionCandidateGenerator`]: Generates all valid action+input pairs
//! - [`AiContext`]: Shared context providing game state and helper methods

pub mod context;
pub mod generator;
pub mod goal;
pub mod provider;
pub mod scoring;

// Re-export public API
pub use context::AiContext;
pub use generator::ActionCandidateGenerator;
pub use goal::{Goal, GoalSelector};
pub use provider::GoalBasedAiProvider;
