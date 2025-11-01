//! Action selection (Layer 3).
//!
//! This module implements the final layer of the utility AI system:
//! selecting concrete actions from the list of available actions based
//! on the chosen tactic.
//!
//! # Architecture
//!
//! ```text
//! Layer 1: Intent → Combat
//! Layer 2: Tactic → Kiting
//! Layer 3: Action → Move(West)  ← This layer
//! ```
//!
//! # Design
//!
//! - **Utilities**: Common helper functions for action evaluation
//! - **Tactic Modules**: Specific scoring logic for each tactic category
//! - **ActionScorer**: Orchestrator that dispatches to appropriate scorer

pub mod combat;
pub mod idle;
pub mod utilities;

use game_core::CharacterActionKind;

use super::Score;
use crate::providers::ai::AiContext;
use crate::providers::ai::types::Tactic;

/// Action selector for Layer 3 decision-making.
///
/// This struct provides the logic for evaluating all available actions
/// for a specific tactic and selecting the one with the highest utility score.
///
/// # Design
///
/// The selector evaluates each action in `available_actions` using
/// tactic-specific scoring functions from submodules ([`combat`], etc.).
/// Each action is scored based on:
///
/// - Feasibility (is_possible)
/// - Situational favorability (situation)
/// - Tactic alignment (personality)
/// - Contextual modifiers (modifier)
///
/// The action with the highest `score.value()` is selected.
///
/// # Determinism
///
/// All scoring is deterministic and pure (no randomness, no I/O).
/// Given the same tactic, available actions, and game state, the same
/// action will always be selected.
pub struct ActionScorer;

impl ActionScorer {
    /// Selects the best action for the given tactic and context.
    ///
    /// This method evaluates all available actions and returns the one
    /// with the highest utility score for the specified tactic.
    ///
    /// # Arguments
    ///
    /// * `tactic` - The tactical approach to use
    /// * `available_actions` - List of actions the NPC can currently perform
    /// * `ctx` - The AI context containing game state
    ///
    /// # Returns
    ///
    /// - `Some(CharacterActionKind)` - The best action to perform
    /// - `None` - If no actions are available or all score 0
    pub fn select(
        tactic: Tactic,
        available_actions: &[CharacterActionKind],
        ctx: &AiContext,
    ) -> Option<CharacterActionKind> {
        if available_actions.is_empty() {
            tracing::debug!("ActionScorer: No available actions");
            return None;
        }

        let mut best_action: Option<CharacterActionKind> = None;
        let mut best_score = Score::impossible();

        tracing::debug!(
            "ActionScorer: Evaluating {} actions for tactic {:?}",
            available_actions.len(),
            tactic
        );

        for action in available_actions {
            let score = Self::score_action(tactic, action, ctx);

            tracing::debug!(
                "  Action {:?}: score={} (possible={}, sit={}, pers={}, mod={})",
                action,
                score.value(),
                score.is_possible,
                score.situation,
                score.personality,
                score.modifier
            );

            // Select action with highest value
            // In case of tie, first action wins (stable ordering)
            if score.value() > best_score.value() {
                best_action = Some(action.clone());
                best_score = score;
            }
        }

        tracing::debug!(
            "ActionScorer: Best action = {:?} (score={})",
            best_action,
            best_score.value()
        );

        best_action
    }

    /// Scores a specific action for a given tactic.
    ///
    /// This method dispatches to the correct scoring function based on
    /// the tactic type.
    ///
    /// # Arguments
    ///
    /// * `tactic` - The tactic to use for scoring
    /// * `action` - The action to score
    /// * `ctx` - The AI context
    ///
    /// # Returns
    ///
    /// The detailed `Score` for this action under this tactic.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let attack_score = ActionScorer::score_action(
    ///     Tactic::AggressiveMelee,
    ///     &CharacterActionKind::Attack(...),
    ///     &ctx
    /// );
    ///
    /// let move_score = ActionScorer::score_action(
    ///     Tactic::AggressiveMelee,
    ///     &CharacterActionKind::Move(...),
    ///     &ctx
    /// );
    ///
    /// if attack_score.value() > move_score.value() {
    ///     println!("NPC prefers attacking over moving");
    /// }
    /// ```
    pub fn score_action(tactic: Tactic, action: &CharacterActionKind, ctx: &AiContext) -> Score {
        match tactic {
            // Combat tactics (fully implemented)
            Tactic::AggressiveMelee => combat::score_for_aggressive_melee(action, ctx),
            Tactic::DefensiveMelee => combat::score_for_defensive_melee(action, ctx),
            Tactic::Ranged => combat::score_for_ranged(action, ctx),
            Tactic::Kiting => combat::score_for_kiting(action, ctx),
            Tactic::Ambush => combat::score_for_ambush(action, ctx),

            // Survival tactics (placeholders)
            Tactic::Flee => Score::impossible(),
            Tactic::Retreat => Score::impossible(),
            Tactic::SeekCover => Score::impossible(),
            Tactic::UseSurvivalItem => Score::impossible(),

            // Social tactics (placeholders)
            Tactic::HealAlly => Score::impossible(),
            Tactic::BuffAlly => Score::impossible(),
            Tactic::CoordinateAttack => Score::impossible(),

            // Exploration tactics (placeholders)
            Tactic::Patrol => Score::impossible(),
            Tactic::Investigate => Score::impossible(),
            Tactic::Search => Score::impossible(),

            // Resource tactics (placeholders)
            Tactic::Loot => Score::impossible(),
            Tactic::GuardTreasure => Score::impossible(),

            // Idle tactics (fully implemented)
            Tactic::Wait => idle::score_for_wait(action, ctx),
            Tactic::Wander => idle::score_for_wander(action, ctx),
        }
    }

    /// Evaluates all actions and returns their scores for debugging.
    ///
    /// This method is useful for understanding why a particular action
    /// was selected by showing all scores side-by-side.
    ///
    /// # Arguments
    ///
    /// * `tactic` - The tactic to evaluate actions for
    /// * `available_actions` - List of actions to evaluate
    /// * `ctx` - The AI context
    ///
    /// # Returns
    ///
    /// Vector of all actions paired with their scores.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let all_scores = ActionScorer::evaluate_all(
    ///     Tactic::Kiting,
    ///     &available_actions,
    ///     &ctx
    /// );
    ///
    /// println!("Kiting action scores:");
    /// for (action, score) in all_scores {
    ///     println!("  {:?}: {}", action, score.value());
    /// }
    /// ```
    pub fn evaluate_all(
        tactic: Tactic,
        available_actions: &[CharacterActionKind],
        ctx: &AiContext,
    ) -> Vec<(CharacterActionKind, Score)> {
        available_actions
            .iter()
            .map(|action| (action.clone(), Self::score_action(tactic, action, ctx)))
            .collect()
    }
}
