//! Utility scoring functions for AI decision-making.
//!
//! This module provides scoring functions for all three layers of the AI system:
//!
//! 1. **Intent scoring** ([`intents`]): Layer 1 - What to do?
//! 2. **Tactic scoring** ([`tactics`]): Layer 2 - How to do it?
//! 3. **Action scoring** ([`actions`]): Layer 3 - Which action exactly?
//!
//! # Structured Scoring
//!
//! All layers use the same [`Score`] structure to ensure consistency and enable
//! debugging across the entire decision pipeline.
//!
//! ## Score Components
//!
//! Every score consists of four explicit components:
//!
//! - **is_possible**: Binary feasibility (can this be done at all?)
//! - **situation**: Game state favorability (0-100)
//! - **personality**: NPC trait preferences (0-100, from TraitProfile)
//! - **modifier**: Contextual adjustments (0-200, typically 100)
//!
//! ## Score Formula
//!
//! ```text
//! final_score = is_possible × situation × personality × modifier / 10000
//! ```

pub mod actions;
pub mod intents;
pub mod selector;
pub mod tactics;

/// Generic scoring result used across all three AI layers.
///
/// This structure explicitly separates scoring into four components,
/// making AI decisions transparent and debuggable.
///
/// # Fields
///
/// - `is_possible`: Can this option be executed? (bool)
/// - `situation`: How favorable is the current game state? (0-100)
/// - `personality`: How much does this NPC prefer this option? (0-100)
/// - `modifier`: Additional contextual factors (0-200, typically 100)
///
/// # Computing Final Score
///
/// Use the `value()` method to compute the final 0-100 score.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Score {
    /// Is this option feasible at all?
    ///
    /// Examples:
    /// - Combat: Can see enemy AND has attack actions
    /// - Kiting: Has ranged weapon AND can move
    /// - Flee: Has escape route
    pub is_possible: bool,

    /// Game state favorability (0-100).
    ///
    /// How favorable is the current situation for this option?
    ///
    /// Examples:
    /// - Combat: Distance to enemy (closer = higher)
    /// - Survival: HP level (lower = higher)
    /// - Exploration: Safety (no enemies = higher)
    pub situation: u32,

    /// NPC trait preference (0-100).
    ///
    /// How much does this NPC's personality favor this option?
    /// Derived from TraitProfile (Species × Archetype × Faction × Temperament).
    ///
    /// Examples:
    /// - Combat: Aggression + Bravery
    /// - Survival: Inverse of Bravery (cowardice)
    /// - Exploration: Curiosity
    pub personality: u32,

    /// Contextual adjustment factor (0-200, typically 100).
    ///
    /// Additional factors that modify the score beyond situation and personality.
    ///
    /// Examples:
    /// - Combat: HP penalty (low HP = 50, healthy = 100)
    /// - Survival: Enemy visibility (visible = 120, none = 80)
    /// - Exploration: Movement availability (can move = 100, cannot = 30)
    pub modifier: u32,
}

impl Score {
    /// Creates a new score with all components explicitly specified.
    ///
    /// # Arguments
    ///
    /// * `is_possible` - Can this option be executed?
    /// * `situation` - Game state favorability (0-100)
    /// * `personality` - NPC trait preference (0-100)
    /// * `modifier` - Contextual adjustments (0-200, typically 100)
    pub const fn new(is_possible: bool, situation: u32, personality: u32, modifier: u32) -> Self {
        Self {
            is_possible,
            situation,
            personality,
            modifier,
        }
    }

    /// Creates an impossible score (all components zero).
    ///
    /// Use this when an option cannot be executed at all.
    pub const fn impossible() -> Self {
        Self {
            is_possible: false,
            situation: 0,
            personality: 0,
            modifier: 0,
        }
    }

    /// Computes the final score value.
    ///
    /// # Formula
    ///
    /// ```text
    /// value = is_possible × situation × personality × modifier / 10000
    /// ```
    ///
    /// # Returns
    ///
    /// Final score 0-100 (or 0 if impossible).
    pub const fn value(&self) -> u32 {
        if !self.is_possible {
            return 0;
        }

        // situation × personality × modifier / 10000
        // Typical range: 0-100
        // Max theoretical: 100 × 100 × 200 / 10000 = 200
        (self.situation * self.personality * self.modifier) / 10000
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_calculation() {
        let score = Score::new(true, 90, 70, 100);
        assert_eq!(score.value(), 63); // 90 × 70 × 100 / 10000
    }

    #[test]
    fn test_impossible_score() {
        let score = Score::impossible();
        assert_eq!(score.value(), 0);
        assert!(!score.is_possible);
    }

    #[test]
    fn test_modifier_effect() {
        // Low HP modifier reduces score
        let healthy = Score::new(true, 90, 70, 100);
        let wounded = Score::new(true, 90, 70, 50);

        assert_eq!(healthy.value(), 63);
        assert_eq!(wounded.value(), 31); // Half score
    }

    #[test]
    fn test_components_public() {
        let score = Score::new(true, 90, 70, 100);

        // All fields should be public for debugging
        assert!(score.is_possible);
        assert_eq!(score.situation, 90);
        assert_eq!(score.personality, 70);
        assert_eq!(score.modifier, 100);
    }
}
