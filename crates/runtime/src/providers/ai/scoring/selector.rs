//! Intent and tactic selection logic (Layers 1 and 2).
//!
//! This module implements the decision-making logic for the first two layers
//! of the utility AI system:
//!
//! - **Layer 1 (IntentScorer)**: Selects the best strategic intent
//! - **Layer 2 (TacticScorer)**: Selects the best tactical approach
//!
//! # Intent Selection
//!
//! The [`IntentScorer`] evaluates all intents using scoring functions from
//! [`super::intents`] and selects the highest-scoring intent.
//!
//! # Tactic Selection
//!
//! The [`TacticScorer`] evaluates all tactics for a given intent using
//! scoring functions from [`super::tactics`] and selects the highest-scoring tactic.

use super::Score;
use super::{intents, tactics};
use crate::providers::ai::AiContext;
use crate::providers::ai::types::{Intent, Tactic};

/// Intent selector for Layer 1 decision-making.
///
/// This struct provides the logic for evaluating all available intents
/// and selecting the one with the highest utility score.
///
/// # Design
///
/// The selector evaluates all 6 intents using the scoring functions from
/// [`super::intents`] module. Each intent is scored based on:
///
/// - Feasibility (is_possible)
/// - Situational favorability (situation)
/// - NPC personality traits (personality)
/// - Contextual modifiers (modifier)
///
/// The intent with the highest `score.value()` is selected.
///
/// # Determinism
///
/// All scoring is deterministic and pure (no randomness, no I/O).
/// Given the same game state and NPC traits, the same intent will always
/// be selected.
pub struct IntentScorer;

impl IntentScorer {
    /// Selects the best intent for the given context.
    ///
    /// This method evaluates all 6 intents and returns the one with the
    /// highest utility score. If all intents score 0, defaults to `Intent::Idle`.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The AI context containing game state and available actions
    ///
    /// # Returns
    ///
    /// The selected `Intent` and its detailed `Score`.
    pub fn select(ctx: &AiContext) -> (Intent, Score) {
        let mut best_intent = Intent::Idle;
        let mut best_score = Score::impossible();

        tracing::debug!("IntentScorer: Evaluating all intents");

        for intent in Intent::all() {
            let score = Self::score_intent(intent, ctx);

            tracing::debug!(
                "  Intent {:?}: score={} (possible={}, sit={}, pers={}, mod={})",
                intent,
                score.value(),
                score.is_possible,
                score.situation,
                score.personality,
                score.modifier
            );

            // Select intent with highest value
            // In case of tie, first intent wins (stable ordering)
            if score.value() > best_score.value() {
                best_intent = intent;
                best_score = score;
            }
        }

        tracing::debug!(
            "IntentScorer: Best intent = {:?} (score={})",
            best_intent,
            best_score.value()
        );

        (best_intent, best_score)
    }

    /// Scores a specific intent using the appropriate scoring function.
    ///
    /// This method dispatches to the correct scoring function from
    /// [`super::intents`] based on the intent type.
    ///
    /// # Arguments
    ///
    /// * `intent` - The intent to score
    /// * `ctx` - The AI context
    ///
    /// # Returns
    ///
    /// The detailed `Score` for this intent.
    pub fn score_intent(intent: Intent, ctx: &AiContext) -> Score {
        match intent {
            Intent::Combat => intents::combat(ctx),
            Intent::Survival => intents::survival(ctx),
            Intent::Exploration => intents::exploration(ctx),
            Intent::Social => intents::social(ctx),
            Intent::Resource => intents::resource(ctx),
            Intent::Idle => intents::idle(ctx),
        }
    }

    /// Evaluates all intents and returns their scores for debugging.
    ///
    /// This method is useful for understanding why a particular intent
    /// was selected by showing all scores side-by-side.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The AI context
    ///
    /// # Returns
    ///
    /// Array of all 6 intents paired with their scores, in declaration order.
    pub fn evaluate_all(ctx: &AiContext) -> [(Intent, Score); 6] {
        [
            (Intent::Combat, Self::score_intent(Intent::Combat, ctx)),
            (Intent::Survival, Self::score_intent(Intent::Survival, ctx)),
            (
                Intent::Exploration,
                Self::score_intent(Intent::Exploration, ctx),
            ),
            (Intent::Social, Self::score_intent(Intent::Social, ctx)),
            (Intent::Resource, Self::score_intent(Intent::Resource, ctx)),
            (Intent::Idle, Self::score_intent(Intent::Idle, ctx)),
        ]
    }
}

/// Tactic selector for Layer 2 decision-making.
///
/// This struct provides the logic for evaluating all tactics for a given intent
/// and selecting the one with the highest utility score.
///
/// # Design
///
/// The selector evaluates all tactics available for a specific intent using
/// scoring functions from [`super::tactics`]. Each tactic is scored based on:
///
/// - Feasibility (is_possible)
/// - Situational favorability (situation)
/// - NPC personality traits (personality)
/// - Contextual modifiers (modifier)
///
/// The tactic with the highest `score.value()` is selected.
///
/// # Intent-Tactic Mapping
///
/// Each intent has a specific set of tactics that can achieve it:
///
/// - **Combat**: AggressiveMelee, DefensiveMelee, Ranged, Kiting, Ambush (5)
/// - **Survival**: Flee, Retreat, SeekCover, UseSurvivalItem (4)
/// - **Exploration**: Patrol, Investigate, Search (3)
/// - **Social**: HealAlly, BuffAlly, CoordinateAttack (3)
/// - **Resource**: Loot, GuardTreasure (2)
/// - **Idle**: Wait, Wander (2)
///
/// # Determinism
///
/// All scoring is deterministic and pure (no randomness, no I/O).
/// Given the same game state and NPC traits, the same tactic will always
/// be selected for a given intent.
pub struct TacticScorer;

impl TacticScorer {
    /// Selects the best tactic for the given intent and context.
    ///
    /// This method evaluates all tactics applicable to the specified intent
    /// and returns the one with the highest utility score.
    ///
    /// # Arguments
    ///
    /// * `intent` - The strategic intent to achieve
    /// * `ctx` - The AI context containing game state and available actions
    ///
    /// # Returns
    ///
    /// The selected `Tactic` and its detailed `Score`.
    pub fn select(intent: Intent, ctx: &AiContext) -> (Tactic, Score) {
        let candidates = Tactic::for_intent(intent);

        tracing::debug!(
            "TacticScorer: Evaluating {} tactics for intent {:?}",
            candidates.len(),
            intent
        );

        // Handle empty tactics (should not happen, but defensive)
        if candidates.is_empty() {
            tracing::warn!("TacticScorer: No tactics available for intent {:?}", intent);
            return (Tactic::Wait, Score::impossible());
        }

        let mut best_tactic = candidates[0];
        let mut best_score = Self::score_tactic(best_tactic, ctx);

        tracing::debug!(
            "  Tactic {:?}: score={} (possible={}, sit={}, pers={}, mod={})",
            best_tactic,
            best_score.value(),
            best_score.is_possible,
            best_score.situation,
            best_score.personality,
            best_score.modifier
        );

        for &tactic in &candidates[1..] {
            let score = Self::score_tactic(tactic, ctx);

            tracing::debug!(
                "  Tactic {:?}: score={} (possible={}, sit={}, pers={}, mod={})",
                tactic,
                score.value(),
                score.is_possible,
                score.situation,
                score.personality,
                score.modifier
            );

            // Select tactic with highest value
            // In case of tie, first tactic wins (stable ordering)
            if score.value() > best_score.value() {
                best_tactic = tactic;
                best_score = score;
            }
        }

        tracing::debug!(
            "TacticScorer: Best tactic = {:?} (score={})",
            best_tactic,
            best_score.value()
        );

        (best_tactic, best_score)
    }

    /// Scores a specific tactic using the appropriate scoring function.
    ///
    /// This method dispatches to the correct scoring function from
    /// [`super::tactics`] based on the tactic type.
    ///
    /// # Arguments
    ///
    /// * `tactic` - The tactic to score
    /// * `ctx` - The AI context
    ///
    /// # Returns
    ///
    /// The detailed `Score` for this tactic.
    pub fn score_tactic(tactic: Tactic, ctx: &AiContext) -> Score {
        match tactic {
            // Combat tactics (fully implemented)
            Tactic::AggressiveMelee => tactics::aggressive_melee(ctx),
            Tactic::DefensiveMelee => tactics::defensive_melee(ctx),
            Tactic::Ranged => tactics::ranged(ctx),
            Tactic::Kiting => tactics::kiting(ctx),
            Tactic::Ambush => tactics::ambush(ctx),

            // Survival tactics (placeholders)
            Tactic::Flee => tactics::flee(ctx),
            Tactic::Retreat => tactics::retreat(ctx),
            Tactic::SeekCover => tactics::seek_cover(ctx),
            Tactic::UseSurvivalItem => tactics::use_survival_item(ctx),

            // Social tactics (placeholders)
            Tactic::HealAlly => tactics::heal_ally(ctx),
            Tactic::BuffAlly => tactics::buff_ally(ctx),
            Tactic::CoordinateAttack => tactics::coordinate_attack(ctx),

            // Exploration tactics (placeholders)
            Tactic::Patrol => tactics::patrol(ctx),
            Tactic::Investigate => tactics::investigate(ctx),
            Tactic::Search => tactics::search(ctx),

            // Resource tactics (placeholders)
            Tactic::Loot => tactics::loot(ctx),
            Tactic::GuardTreasure => tactics::guard_treasure(ctx),

            // Idle tactics (placeholders)
            Tactic::Wait => tactics::wait(ctx),
            Tactic::Wander => tactics::wander(ctx),
        }
    }

    /// Evaluates all tactics for the given intent and returns their scores for debugging.
    ///
    /// This method is useful for understanding why a particular tactic
    /// was selected by showing all scores side-by-side.
    ///
    /// # Arguments
    ///
    /// * `intent` - The intent to evaluate tactics for
    /// * `ctx` - The AI context
    ///
    /// # Returns
    ///
    /// Vector of all tactics for this intent paired with their scores.
    pub fn evaluate_all(intent: Intent, ctx: &AiContext) -> Vec<(Tactic, Score)> {
        Tactic::for_intent(intent)
            .iter()
            .map(|&tactic| (tactic, Self::score_tactic(tactic, ctx)))
            .collect()
    }
}
