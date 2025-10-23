//! Action scoring for idle tactics.
//!
//! This module implements action selection for idle behaviors:
//! - **Wait**: Stand still and do nothing
//! - **Wander**: Move randomly in available directions
//!
//! # Design
//!
//! Idle tactics have simple scoring:
//! - Wait always prefers the Wait action (100% match)
//! - Wander prefers random movement in any direction (equal scoring)

use game_core::CharacterActionKind;

use super::Score;
use crate::providers::ai::AiContext;

/// Score actions for the Wait tactic.
///
/// Prefers the explicit Wait action above all else.
/// If no wait action exists (shouldn't happen), scores all actions as impossible.
///
/// # Scoring
///
/// - Wait action: score=100 (perfect match)
/// - All other actions: impossible
pub fn score_for_wait(action: &CharacterActionKind, _ctx: &AiContext) -> Score {
    match action {
        CharacterActionKind::Wait => Score::new(true, 100, 100, 100),
        _ => Score::impossible(),
    }
}

/// Score actions for the Wander tactic.
///
/// Prefers movement in any direction equally. All directions are treated
/// as equally good for wandering (uniform random selection).
///
/// # Scoring
///
/// - Movement actions: score=50 (all equal, uniform distribution)
/// - All other actions: impossible
///
/// # Future Enhancement
///
/// Could add directional preferences based on:
/// - Unexplored areas (situation modifier)
/// - Recent movement history (avoid backtracking)
/// - NPC personality (some prefer straight lines, others zigzag)
pub fn score_for_wander(action: &CharacterActionKind, _ctx: &AiContext) -> Score {
    match action {
        CharacterActionKind::Move(_) => {
            // All directions equally valid for wandering
            // Use 50 as base score to indicate "neutral preference"
            Score::new(true, 50, 50, 100)
        }
        _ => Score::impossible(),
    }
}
